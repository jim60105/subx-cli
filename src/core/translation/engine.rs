//! High-level translation engine that orchestrates parsing, AI calls, and
//! reapplication of translated text.
//!
//! See [`crate::core::translation`] for the broader design rationale.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::json;

use crate::Result;
use crate::core::formats::Subtitle;
use crate::core::formats::manager::FormatManager;
use crate::core::translation::CueIdGenerator;
use crate::core::translation::request::{
    GlossaryEntry, TerminologyMap, TranslationOutcome, TranslationRequest, TranslationResult,
    merge_terminology,
};
use crate::error::SubXError;
use crate::services::ai::AIProvider;
use crate::services::ai::translation_prompts::{
    TERMINOLOGY_SYSTEM_MESSAGE, TRANSLATION_SYSTEM_MESSAGE, build_terminology_prompt,
    build_translation_prompt, is_unknown_cue_id_error, parse_terminology_response,
    parse_translation_response_partial,
};

/// Orchestrates subtitle translation through an [`AIProvider`].
///
/// The engine is deliberately thin: it is composed of an AI provider, a
/// [`FormatManager`] for parsing/serialization, and a configurable batch
/// size. Constructors are provided so [`crate::core::factory::ComponentFactory`]
/// can wire the engine from runtime configuration without leaking provider
/// details.
pub struct TranslationEngine {
    ai_provider: Arc<dyn AIProvider>,
    format_manager: FormatManager,
    batch_size: usize,
}

impl std::fmt::Debug for TranslationEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranslationEngine")
            .field("batch_size", &self.batch_size)
            .finish()
    }
}

impl TranslationEngine {
    /// Create an engine with a custom batch size.
    ///
    /// # Errors
    ///
    /// Returns [`SubXError::config`] when `batch_size` is zero.
    pub fn new(ai_provider: Arc<dyn AIProvider>, batch_size: usize) -> Result<Self> {
        if batch_size == 0 {
            return Err(SubXError::config(
                "Translation batch size must be greater than 0",
            ));
        }
        Ok(Self {
            ai_provider,
            format_manager: FormatManager::new(),
            batch_size,
        })
    }

    /// Get the configured AI batch size.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Borrow the underlying [`FormatManager`].
    pub fn format_manager(&self) -> &FormatManager {
        &self.format_manager
    }

    /// Translate a subtitle that has already been parsed into the shared
    /// [`Subtitle`] data model.
    ///
    /// This is the primary entry point used by the translate command and
    /// integration tests. It does not perform any filesystem I/O.
    ///
    /// The order of operations matches the OpenSpec design:
    ///
    /// 1. Generate UUIDv7 cue IDs with strict 1ms spacing.
    /// 2. Run a terminology extraction pass (single AI request) and merge
    ///    the result with the user glossary.
    /// 3. Translate cues in configurable batches, validating each response
    ///    against the requested cue IDs before applying any text changes.
    /// 4. Reapply translated text to the parsed subtitle while preserving
    ///    timing, ordering, cue counts, and styling metadata.
    pub async fn translate_subtitle(
        &self,
        subtitle: Subtitle,
        request: &TranslationRequest,
    ) -> Result<TranslationResult> {
        if request.target_language.trim().is_empty() {
            return Err(SubXError::config(
                "Translation target language must be provided",
            ));
        }

        let mut subtitle = subtitle;
        if subtitle.entries.is_empty() {
            return Ok(TranslationResult {
                subtitle,
                outcome: TranslationOutcome::default(),
            });
        }

        // 1. Cue ID assignment in subtitle order.
        let mut id_gen = CueIdGenerator::new();
        let cue_ids: Vec<String> = subtitle
            .entries
            .iter()
            .map(|_| id_gen.next_id().to_string())
            .collect();
        let protected_cues: Vec<ProtectedCueText> = subtitle
            .entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| protect_inline_formatting(&entry.text, idx))
            .collect();
        let terminology_texts: Vec<String> = protected_cues
            .iter()
            .map(|cue| cue.visible_text.clone())
            .collect();

        // 2. Terminology extraction + glossary merge.
        let generated_terms = self
            .extract_terminology(&terminology_texts, request)
            .await?;
        let effective_terminology = merge_terminology(generated_terms, &request.glossary_entries);

        // 3. Batched translation with cue ID validation. Missing IDs are
        //    retried once after all initial batches have completed.
        let mut translations: BTreeMap<String, String> = BTreeMap::new();
        let mut batch_count = 0usize;
        for chunk_indices in chunk_ranges(subtitle.entries.len(), self.batch_size) {
            let mut batch_cues: Vec<(String, String)> = Vec::with_capacity(chunk_indices.len());
            let mut batch_ids: Vec<String> = Vec::with_capacity(chunk_indices.len());
            for &i in &chunk_indices {
                batch_cues.push((cue_ids[i].clone(), protected_cues[i].prompt_text.clone()));
                batch_ids.push(cue_ids[i].clone());
            }

            let (map, issued_batches) = self
                .translate_batch_with_unknown_retry(
                    &batch_cues,
                    &batch_ids,
                    request,
                    &effective_terminology,
                )
                .await?;
            for (id, text) in map {
                translations.insert(id, text);
            }
            batch_count += issued_batches;
            log_translation_progress(translations.len(), cue_ids.len());
        }

        let mut empty_fallback_ids = BTreeSet::new();
        let missing_after_initial = missing_translation_indices(&cue_ids, &translations);
        if !missing_after_initial.is_empty() {
            let (retry_map, issued_batches) = self
                .retry_missing_translations(
                    &cue_ids,
                    &protected_cues,
                    &missing_after_initial,
                    request,
                    &effective_terminology,
                )
                .await?;
            for (id, text) in retry_map {
                translations.insert(id, text);
            }
            batch_count += issued_batches;

            for idx in missing_translation_indices(&cue_ids, &translations) {
                let id = cue_ids[idx].clone();
                translations.insert(id.clone(), String::new());
                empty_fallback_ids.insert(id);
            }
            log_translation_progress(translations.len(), cue_ids.len());
        }

        // 4. Reapply translated text to the original subtitle entries while
        //    preserving timing, ordering, and styling metadata.
        for ((entry, id), protected) in subtitle
            .entries
            .iter_mut()
            .zip(cue_ids.iter())
            .zip(protected_cues.iter())
        {
            if let Some(translated) = translations.get(id) {
                if empty_fallback_ids.contains(id) {
                    entry.text = String::new();
                } else {
                    entry.text = restore_inline_formatting(translated, protected)?;
                }
            }
        }

        let translated_cue_count = subtitle.entries.len();
        Ok(TranslationResult {
            subtitle,
            outcome: TranslationOutcome {
                effective_terminology,
                translated_cue_count,
                batch_count,
            },
        })
    }

    /// Parse subtitle text content and translate it.
    ///
    /// Convenience wrapper that detects the format using
    /// [`FormatManager::parse_auto`] before delegating to
    /// [`Self::translate_subtitle`].
    pub async fn translate_content(
        &self,
        content: &str,
        request: &TranslationRequest,
    ) -> Result<TranslationResult> {
        let subtitle = self.format_manager.parse_auto(content)?;
        self.translate_subtitle(subtitle, request).await
    }

    /// Run only the terminology extraction pass.
    ///
    /// Exposed so callers can preview the terminology map before issuing
    /// translation requests.
    pub async fn extract_terminology(
        &self,
        cue_texts: &[String],
        request: &TranslationRequest,
    ) -> Result<TerminologyMap> {
        let prompt = build_terminology_prompt(
            &request.target_language,
            request.source_language.as_deref(),
            cue_texts,
            request.glossary_text.as_deref(),
            request.context.as_deref(),
        );
        let messages = vec![
            json!({"role": "system", "content": TERMINOLOGY_SYSTEM_MESSAGE}),
            json!({"role": "user", "content": prompt}),
        ];
        let response = self.ai_provider.chat_completion(messages).await?;
        parse_terminology_response(&response)
    }

    async fn retry_missing_translations(
        &self,
        cue_ids: &[String],
        protected_cues: &[ProtectedCueText],
        missing_indices: &[usize],
        request: &TranslationRequest,
        terminology: &TerminologyMap,
    ) -> Result<(BTreeMap<String, String>, usize)> {
        let mut retry_cues = Vec::with_capacity(missing_indices.len());
        let mut retry_ids = Vec::with_capacity(missing_indices.len());
        for &idx in missing_indices {
            retry_cues.push((
                cue_ids[idx].clone(),
                protected_cues[idx].prompt_text.clone(),
            ));
            retry_ids.push(cue_ids[idx].clone());
        }

        self.translate_batch_with_unknown_retry(&retry_cues, &retry_ids, request, terminology)
            .await
    }

    async fn translate_batch_with_unknown_retry(
        &self,
        batch_cues: &[(String, String)],
        batch_ids: &[String],
        request: &TranslationRequest,
        terminology: &TerminologyMap,
    ) -> Result<(BTreeMap<String, String>, usize)> {
        match self
            .translate_batch_once(batch_cues, batch_ids, request, terminology)
            .await
        {
            Ok(map) => Ok((map, 1)),
            Err(err) if is_unknown_cue_id_error(&err) => {
                // stderr diagnostic — never written to stdout. Suppressed
                // when --quiet is set or when JSON output mode is active so
                // structured stdout is not accompanied by free-form chatter.
                if !crate::cli::output::is_quiet() && !crate::cli::output::active_mode().is_json() {
                    eprintln!(
                        "⚠ Translation response contained an unknown cue ID; discarding the batch response and retrying once."
                    );
                }
                match self
                    .translate_batch_once(batch_cues, batch_ids, request, terminology)
                    .await
                {
                    Ok(map) => Ok((map, 2)),
                    Err(retry_err) if is_unknown_cue_id_error(&retry_err) => {
                        Err(SubXError::ai_service(format!(
                            "Translation response still contained an unknown cue ID after retry; failing this file: {retry_err}"
                        )))
                    }
                    Err(retry_err) => Err(retry_err),
                }
            }
            Err(err) => Err(err),
        }
    }

    async fn translate_batch_once(
        &self,
        batch_cues: &[(String, String)],
        batch_ids: &[String],
        request: &TranslationRequest,
        terminology: &TerminologyMap,
    ) -> Result<BTreeMap<String, String>> {
        let prompt = build_translation_prompt(
            &request.target_language,
            request.source_language.as_deref(),
            terminology,
            request.glossary_text.as_deref(),
            request.context.as_deref(),
            batch_cues,
        );
        let messages = vec![
            json!({"role": "system", "content": TRANSLATION_SYSTEM_MESSAGE}),
            json!({"role": "user", "content": prompt}),
        ];
        let response = self.ai_provider.chat_completion(messages).await?;
        Ok(parse_translation_response_partial(&response, batch_ids)?
            .into_iter()
            .collect())
    }
}

/// Parse a UTF-8 glossary text file into [`GlossaryEntry`] values.
///
/// The expected format is one mapping per line in the form `source = target`
/// or `source -> target`. Empty lines and lines starting with `#` are
/// ignored. Lines that do not contain a recognized separator are skipped
/// silently so free-form prose context can coexist with structured entries.
pub fn parse_glossary_text(text: &str) -> Vec<GlossaryEntry> {
    let mut out = Vec::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let separator = if line.contains("->") {
            "->"
        } else if line.contains('=') {
            "="
        } else {
            continue;
        };
        let mut parts = line.splitn(2, separator);
        let source = parts.next().map(str::trim).unwrap_or("").to_string();
        let target = parts.next().map(str::trim).unwrap_or("").to_string();
        if source.is_empty() || target.is_empty() {
            continue;
        }
        out.push(GlossaryEntry { source, target });
    }
    out
}

fn chunk_ranges(total: usize, batch_size: usize) -> Vec<Vec<usize>> {
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < total {
        let end = (start + batch_size).min(total);
        chunks.push((start..end).collect());
        start = end;
    }
    chunks
}

fn missing_translation_indices(
    cue_ids: &[String],
    translations: &BTreeMap<String, String>,
) -> Vec<usize> {
    cue_ids
        .iter()
        .enumerate()
        .filter_map(|(idx, id)| (!translations.contains_key(id)).then_some(idx))
        .collect()
}

fn log_translation_progress(processed_cues: usize, total_cues: usize) {
    // Diagnostic progress chatter. JSON mode and --quiet both suppress this
    // free-form stderr output to keep machine-readable consumers clean.
    if crate::cli::output::is_quiet() || crate::cli::output::active_mode().is_json() {
        return;
    }
    eprintln!(
        "{}",
        format_translation_progress(processed_cues, total_cues)
    );
}

fn format_translation_progress(processed_cues: usize, total_cues: usize) -> String {
    format!("📊 Translation Progress:\n   Processed cues: {processed_cues}/{total_cues}")
}

#[derive(Debug, Clone)]
struct ProtectedCueText {
    prompt_text: String,
    visible_text: String,
    markers: Vec<(String, String)>,
}

fn protect_inline_formatting(text: &str, cue_index: usize) -> ProtectedCueText {
    let mut prompt_text = String::new();
    let mut visible_text = String::new();
    let mut markers = Vec::new();
    let mut offset = 0usize;

    while offset < text.len() {
        let remaining = &text[offset..];

        if let Some(end_offset) = html_like_tag_end(remaining) {
            let token = &text[offset..offset + end_offset];
            push_format_marker(cue_index, token, &mut prompt_text, &mut markers);
            offset += end_offset;
            continue;
        }

        if let Some(end_offset) = ass_override_tag_end(remaining) {
            let token = &text[offset..offset + end_offset];
            push_format_marker(cue_index, token, &mut prompt_text, &mut markers);
            offset += end_offset;
            continue;
        }

        let ch = remaining
            .chars()
            .next()
            .expect("offset is always inside a non-empty string slice");
        prompt_text.push(ch);
        visible_text.push(ch);
        offset += ch.len_utf8();
    }

    ProtectedCueText {
        prompt_text,
        visible_text,
        markers,
    }
}

fn html_like_tag_end(text: &str) -> Option<usize> {
    if !text.starts_with('<') {
        return None;
    }
    let end = text.find('>')? + 1;
    (end > 2).then_some(end)
}

fn ass_override_tag_end(text: &str) -> Option<usize> {
    if !text.starts_with('{') {
        return None;
    }
    let end = text.find('}')? + 1;
    let token = &text[..end];
    token.contains('\\').then_some(end)
}

fn push_format_marker(
    cue_index: usize,
    token: &str,
    prompt_text: &mut String,
    markers: &mut Vec<(String, String)>,
) {
    let placeholder = format!("__SUBX_FMT_{}_{}__", cue_index, markers.len());
    prompt_text.push_str(&placeholder);
    markers.push((placeholder, token.to_string()));
}

fn restore_inline_formatting(translated: &str, protected: &ProtectedCueText) -> Result<String> {
    let mut restored = translated.to_string();
    for (placeholder, token) in &protected.markers {
        let count = restored.matches(placeholder).count();
        if count != 1 {
            return Err(SubXError::ai_service(format!(
                "Translation response must preserve formatting placeholder {placeholder} exactly once"
            )));
        }
        restored = restored.replace(placeholder, token);
    }
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;
    use std::time::Duration;

    use crate::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
    use crate::services::ai::{
        AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
    };

    struct ScriptedAI {
        responses: Mutex<Vec<String>>,
    }

    impl ScriptedAI {
        fn new(responses: Vec<&str>) -> Arc<Self> {
            Arc::new(Self {
                responses: Mutex::new(responses.into_iter().map(|s| s.to_string()).collect()),
            })
        }
    }

    #[async_trait]
    impl AIProvider for ScriptedAI {
        async fn analyze_content(&self, _r: AnalysisRequest) -> Result<MatchResult> {
            unreachable!()
        }

        async fn verify_match(&self, _r: VerificationRequest) -> Result<ConfidenceScore> {
            unreachable!()
        }

        async fn chat_completion(&self, _messages: Vec<serde_json::Value>) -> Result<String> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err(SubXError::ai_service("no scripted response left"));
            }
            Ok(responses.remove(0))
        }
    }

    fn sample_subtitle() -> Subtitle {
        let metadata = SubtitleMetadata::new(SubtitleFormatType::Srt);
        let mut sub = Subtitle::new(SubtitleFormatType::Srt, metadata);
        sub.entries.push(SubtitleEntry::new(
            1,
            Duration::from_secs(1),
            Duration::from_secs(2),
            "Hello Alice".to_string(),
        ));
        sub.entries.push(SubtitleEntry::new(
            2,
            Duration::from_secs(3),
            Duration::from_secs(4),
            "Goodbye Alice".to_string(),
        ));
        sub
    }

    #[tokio::test]
    async fn translation_engine_translates_in_order() {
        let term_resp = r#"{"terms":[{"source":"Alice","target":"愛麗絲"}]}"#;
        // Single batch with batch_size=10
        let cues_resp = r#"{"translations":[{"id":"__ID0__","text":"哈囉 愛麗絲"},{"id":"__ID1__","text":"再見 愛麗絲"}]}"#;
        let provider = ScriptedAI::new(vec![term_resp, cues_resp]);

        // We patch responses lazily: capture cue ids after engine runs.
        // Easier: use an interceptor that rewrites placeholder ids.
        struct PlaceholderAI {
            inner: Arc<ScriptedAI>,
            captured_ids: Mutex<Vec<String>>,
        }
        #[async_trait]
        impl AIProvider for PlaceholderAI {
            async fn analyze_content(&self, _r: AnalysisRequest) -> Result<MatchResult> {
                unreachable!()
            }
            async fn verify_match(&self, _r: VerificationRequest) -> Result<ConfidenceScore> {
                unreachable!()
            }
            async fn chat_completion(&self, messages: Vec<serde_json::Value>) -> Result<String> {
                // Capture cue ids from the last user prompt and patch the
                // scripted response accordingly.
                let prompt = messages
                    .last()
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut response = self.inner.chat_completion(messages).await?;
                if response.contains("__ID0__") {
                    let ids: Vec<String> = prompt
                        .lines()
                        .filter_map(|l| l.trim().strip_prefix("- id: "))
                        .map(|s| s.trim().to_string())
                        .collect();
                    let mut captured = self.captured_ids.lock().unwrap();
                    *captured = ids.clone();
                    for (i, id) in ids.iter().enumerate() {
                        response = response.replace(&format!("__ID{}__", i), id);
                    }
                }
                Ok(response)
            }
        }

        let provider: Arc<dyn AIProvider> = Arc::new(PlaceholderAI {
            inner: provider,
            captured_ids: Mutex::new(Vec::new()),
        });
        let engine = TranslationEngine::new(provider, 10).unwrap();
        let request = TranslationRequest {
            target_language: "zh-TW".to_string(),
            source_language: Some("en".to_string()),
            glossary_text: None,
            context: None,
            glossary_entries: vec![],
        };
        let result = engine
            .translate_subtitle(sample_subtitle(), &request)
            .await
            .unwrap();
        assert_eq!(result.subtitle.entries.len(), 2);
        assert_eq!(result.subtitle.entries[0].text, "哈囉 愛麗絲");
        assert_eq!(result.subtitle.entries[1].text, "再見 愛麗絲");
        assert_eq!(result.outcome.translated_cue_count, 2);
        assert_eq!(result.outcome.batch_count, 1);
        assert_eq!(
            result.outcome.effective_terminology.get("Alice").unwrap(),
            "愛麗絲"
        );
        // Timing preserved.
        assert_eq!(
            result.subtitle.entries[0].start_time,
            Duration::from_secs(1)
        );
        assert_eq!(result.subtitle.entries[1].end_time, Duration::from_secs(4));
    }

    #[tokio::test]
    async fn empty_subtitle_returns_empty_outcome() {
        let provider: Arc<dyn AIProvider> = ScriptedAI::new(vec![]);
        let engine = TranslationEngine::new(provider, 5).unwrap();
        let metadata = SubtitleMetadata::new(SubtitleFormatType::Srt);
        let sub = Subtitle::new(SubtitleFormatType::Srt, metadata);
        let request = TranslationRequest {
            target_language: "zh-TW".to_string(),
            source_language: None,
            glossary_text: None,
            context: None,
            glossary_entries: vec![],
        };
        let result = engine.translate_subtitle(sub, &request).await.unwrap();
        assert_eq!(result.outcome.translated_cue_count, 0);
        assert_eq!(result.outcome.batch_count, 0);
    }

    #[test]
    fn batch_size_zero_is_rejected() {
        let provider: Arc<dyn AIProvider> = ScriptedAI::new(vec![]);
        let err = TranslationEngine::new(provider, 0).unwrap_err();
        assert!(err.to_string().contains("batch size"));
    }

    #[test]
    fn parse_glossary_text_handles_multiple_separators() {
        let text = "# comment\nAlice = 艾莉絲\nBob -> 鮑伯\n\n";
        let entries = parse_glossary_text(text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].source, "Alice");
        assert_eq!(entries[0].target, "艾莉絲");
        assert_eq!(entries[1].source, "Bob");
        assert_eq!(entries[1].target, "鮑伯");
    }

    #[test]
    fn protect_and_restore_inline_formatting_tokens() {
        let protected = protect_inline_formatting(r#"<i>{\b1}Hello{\b0}</i>"#, 3);
        assert_eq!(
            protected.prompt_text,
            "__SUBX_FMT_3_0____SUBX_FMT_3_1__Hello__SUBX_FMT_3_2____SUBX_FMT_3_3__"
        );
        assert_eq!(protected.visible_text, "Hello");

        let translated = "__SUBX_FMT_3_0____SUBX_FMT_3_1__你好__SUBX_FMT_3_2____SUBX_FMT_3_3__";
        let restored = restore_inline_formatting(translated, &protected).unwrap();
        assert_eq!(restored, r#"<i>{\b1}你好{\b0}</i>"#);
    }

    #[test]
    fn translation_progress_message_includes_processed_and_total_cues() {
        assert_eq!(
            format_translation_progress(42, 100),
            "📊 Translation Progress:\n   Processed cues: 42/100"
        );
    }
}
