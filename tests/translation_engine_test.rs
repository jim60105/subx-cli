//! Integration tests for the subtitle translation engine.
//!
//! Covers UUIDv7 cue ID guarantees, glossary override precedence, batching
//! and ordering, missing-cue retry, and fallback behavior when the AI response
//! keeps omitting a cue ID. These tests exercise the engine without any real network access
//! by feeding deterministic responses through an in-process `AIProvider`
//! double that captures cue IDs from the prompt and rewrites placeholder
//! `__ID<N>__` tokens in the canned response.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use subx_cli::Result;
use subx_cli::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use subx_cli::core::translation::{
    GlossaryEntry, TranslationEngine, TranslationRequest, generate_cue_ids,
};
use subx_cli::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};
use uuid::Uuid;

/// Extract the 48-bit `unix_time_ts` field from a UUIDv7 value.
fn unix_time_ms(id: &Uuid) -> u64 {
    let bytes = id.as_bytes();
    ((bytes[0] as u64) << 40)
        | ((bytes[1] as u64) << 32)
        | ((bytes[2] as u64) << 24)
        | ((bytes[3] as u64) << 16)
        | ((bytes[4] as u64) << 8)
        | (bytes[5] as u64)
}

/// Test double that returns canned responses in order. When the canned text
/// contains `__ID<N>__` placeholders, those are substituted with cue IDs
/// extracted from the most recent translation prompt so the response stays
/// in sync with the engine-generated UUIDv7 values.
struct PlaceholderAI {
    responses: Mutex<Vec<String>>,
}

impl PlaceholderAI {
    fn new(responses: Vec<&str>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into_iter().map(String::from).collect()),
        })
    }
}

#[async_trait]
impl AIProvider for PlaceholderAI {
    async fn analyze_content(&self, _r: AnalysisRequest) -> Result<MatchResult> {
        unreachable!("translation tests do not call analyze_content")
    }

    async fn verify_match(&self, _r: VerificationRequest) -> Result<ConfidenceScore> {
        unreachable!("translation tests do not call verify_match")
    }

    async fn chat_completion(&self, messages: Vec<serde_json::Value>) -> Result<String> {
        let prompt = messages
            .last()
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let mut response = {
            let mut responses = self.responses.lock().unwrap();
            assert!(
                !responses.is_empty(),
                "PlaceholderAI ran out of canned responses"
            );
            responses.remove(0)
        };

        if response.contains("__ID") {
            let ids: Vec<String> = prompt
                .lines()
                .filter_map(|l| l.trim().strip_prefix("- id: "))
                .map(|s| s.trim().to_string())
                .collect();
            for (i, id) in ids.iter().enumerate() {
                response = response.replace(&format!("__ID{}__", i), id);
            }
        }
        if response.contains("__FMT") {
            let placeholders = formatting_placeholders(&prompt);
            for (i, placeholder) in placeholders.iter().enumerate() {
                response = response.replace(&format!("__FMT{}__", i), placeholder);
            }
        }
        Ok(response)
    }
}

fn formatting_placeholders(prompt: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
    let mut search_start = 0usize;
    while let Some(start_rel) = prompt[search_start..].find("__SUBX_FMT_") {
        let start = search_start + start_rel;
        let after_prefix = start + 2;
        let Some(end_rel) = prompt[after_prefix..].find("__") else {
            break;
        };
        let end = after_prefix + end_rel + 2;
        placeholders.push(prompt[start..end].to_string());
        search_start = end;
    }
    placeholders
}

fn make_subtitle(texts: &[&str]) -> Subtitle {
    let metadata = SubtitleMetadata::new(SubtitleFormatType::Srt);
    let mut sub = Subtitle::new(SubtitleFormatType::Srt, metadata);
    for (i, text) in texts.iter().enumerate() {
        let start = Duration::from_secs((i as u64) * 2 + 1);
        let end = Duration::from_secs((i as u64) * 2 + 2);
        sub.entries
            .push(SubtitleEntry::new(i + 1, start, end, text.to_string()));
    }
    sub
}

#[test]
fn cue_ids_are_uuidv7_with_strictly_increasing_unix_timestamps() {
    let ids = generate_cue_ids(5);
    assert_eq!(ids.len(), 5);
    let mut last_ts = 0u64;
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(id.get_version_num(), 7, "cue id #{i} must be UUIDv7: {id}");
        let ts = unix_time_ms(id);
        if i > 0 {
            assert!(
                ts > last_ts,
                "cue id #{i} unix_time_ts must be strictly greater (got {ts} after {last_ts})"
            );
            assert!(
                ts - last_ts >= 1,
                "cue ids must be spaced by at least 1ms (delta = {})",
                ts - last_ts
            );
        }
        last_ts = ts;
    }
}

#[tokio::test]
async fn translation_engine_preserves_order_and_terminology() {
    let term_resp = r#"{"terms":[{"source":"Alice","target":"愛麗絲"}]}"#;
    let cues_resp = r#"{"translations":[{"id":"__ID0__","text":"哈囉 愛麗絲"},{"id":"__ID1__","text":"再見 愛麗絲"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, cues_resp]);

    let engine = TranslationEngine::new(provider, 10).expect("batch_size > 0");
    let subtitle = make_subtitle(&["Hello Alice", "Goodbye Alice"]);

    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: Some("en".to_string()),
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();

    assert_eq!(result.subtitle.entries.len(), 2);
    assert_eq!(result.subtitle.entries[0].text, "哈囉 愛麗絲");
    assert_eq!(result.subtitle.entries[1].text, "再見 愛麗絲");
    // Timing is preserved.
    assert_eq!(
        result.subtitle.entries[0].start_time,
        Duration::from_secs(1)
    );
    assert_eq!(result.subtitle.entries[0].end_time, Duration::from_secs(2));
    assert_eq!(
        result.subtitle.entries[1].start_time,
        Duration::from_secs(3)
    );
    assert_eq!(result.subtitle.entries[1].end_time, Duration::from_secs(4));
    // Terminology recorded in outcome.
    assert_eq!(result.outcome.translated_cue_count, 2);
    assert_eq!(result.outcome.batch_count, 1);
    assert_eq!(
        result.outcome.effective_terminology.get("Alice").unwrap(),
        "愛麗絲"
    );
}

#[tokio::test]
async fn translation_engine_glossary_overrides_generated_terms() {
    // Generated map suggests Alice -> 愛麗絲 but the user glossary mandates
    // Alice -> 艾莉絲. The effective terminology in the outcome must use the
    // glossary value.
    let term_resp = r#"{"terms":[{"source":"Alice","target":"愛麗絲"}]}"#;
    let cues_resp = r#"{"translations":[{"id":"__ID0__","text":"哈囉 艾莉絲"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, cues_resp]);

    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["Hello Alice"]);
    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: None,
        glossary_text: Some("Alice = 艾莉絲".to_string()),
        context: None,
        glossary_entries: vec![GlossaryEntry {
            source: "Alice".to_string(),
            target: "艾莉絲".to_string(),
        }],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    assert_eq!(
        result.outcome.effective_terminology.get("Alice").unwrap(),
        "艾莉絲",
        "glossary entry must override AI-generated terminology"
    );
    assert_eq!(result.subtitle.entries[0].text, "哈囉 艾莉絲");
}

#[tokio::test]
async fn translation_engine_splits_into_multiple_batches_in_order() {
    // 3 cues with batch size 2 -> 1 terminology pass + 2 translation
    // batches. The placeholder substitution keeps each batch's IDs aligned
    // with the engine-generated values.
    let term_resp = r#"{"terms":[]}"#;
    let batch1 = r#"{"translations":[{"id":"__ID0__","text":"一"},{"id":"__ID1__","text":"二"}]}"#;
    let batch2 = r#"{"translations":[{"id":"__ID0__","text":"三"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, batch1, batch2]);

    let engine = TranslationEngine::new(provider, 2).unwrap();
    let subtitle = make_subtitle(&["one", "two", "three"]);
    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: None,
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    assert_eq!(result.outcome.batch_count, 2);
    assert_eq!(result.outcome.translated_cue_count, 3);
    let texts: Vec<&str> = result
        .subtitle
        .entries
        .iter()
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(texts, vec!["一", "二", "三"]);
}

#[tokio::test]
async fn translation_engine_retries_missing_cue_after_initial_batches() {
    // The first initial batch omits the second cue. The engine must finish the
    // second initial batch before retrying the missing cue once.
    let term_resp = r#"{"terms":[]}"#;
    let batch1 = r#"{"translations":[{"id":"__ID0__","text":"一"}]}"#;
    let batch2 = r#"{"translations":[{"id":"__ID0__","text":"三"}]}"#;
    let retry = r#"{"translations":[{"id":"__ID0__","text":"二補"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, batch1, batch2, retry]);

    let engine = TranslationEngine::new(provider, 2).unwrap();
    let subtitle = make_subtitle(&["one", "two", "three"]);

    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: None,
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    let texts: Vec<&str> = result
        .subtitle
        .entries
        .iter()
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(texts, vec!["一", "二補", "三"]);
    assert_eq!(
        result.outcome.batch_count, 3,
        "two initial batches plus one retry request"
    );
}

#[tokio::test]
async fn translation_engine_fills_empty_when_retry_still_omits_cue() {
    let term_resp = r#"{"terms":[]}"#;
    let batch = r#"{"translations":[{"id":"__ID0__","text":"一"}]}"#;
    let retry = r#"{"translations":[]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, batch, retry]);

    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["one", "{\\i1}two{\\i0}"]);
    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: None,
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    assert_eq!(result.subtitle.entries[0].text, "一");
    assert_eq!(
        result.subtitle.entries[1].text, "",
        "still-missing retry cue falls back to empty text even when source had formatting"
    );
    assert_eq!(result.outcome.batch_count, 2);
}

#[tokio::test]
async fn translation_engine_discards_unknown_cue_batch_and_retries_once() {
    let term_resp = r#"{"terms":[]}"#;
    // The first batch response includes one valid cue plus one hallucinated cue.
    // The valid cue from that response must be discarded with the rest of the
    // batch, so the final output must use the retry text.
    let hallucinated = r#"{"translations":[{"id":"__ID0__","text":"discard-me"},{"id":"01900000-9999-7000-8000-000000000000","text":"?"}]}"#;
    let retry =
        r#"{"translations":[{"id":"__ID0__","text":"一補"},{"id":"__ID1__","text":"二補"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, hallucinated, retry]);

    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["one", "two"]);
    let request = TranslationRequest {
        target_language: "ja".to_string(),
        source_language: None,
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    let texts: Vec<&str> = result
        .subtitle
        .entries
        .iter()
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(texts, vec!["一補", "二補"]);
    assert_eq!(
        result.outcome.batch_count, 2,
        "initial hallucinated response plus one successful retry"
    );
}

#[tokio::test]
async fn translation_engine_fails_when_unknown_cue_id_repeats_after_retry() {
    let term_resp = r#"{"terms":[]}"#;
    let hallucinated =
        r#"{"translations":[{"id":"01900000-9999-7000-8000-000000000000","text":"?"}]}"#;
    let retry_hallucinated =
        r#"{"translations":[{"id":"01900000-9999-7000-8000-000000000001","text":"?"}]}"#;
    let provider: Arc<dyn AIProvider> =
        PlaceholderAI::new(vec![term_resp, hallucinated, retry_hallucinated]);

    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["only cue"]);
    let request = TranslationRequest {
        target_language: "ja".to_string(),
        source_language: None,
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let err = engine
        .translate_subtitle(subtitle, &request)
        .await
        .expect_err("repeated unknown cue id must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("unknown cue ID after retry"),
        "unexpected: {msg}"
    );
}

#[tokio::test]
async fn translation_engine_preserves_supported_format_metadata_for_all_formats() {
    let cases = [
        (
            "srt",
            "1\n00:00:01,000 --> 00:00:02,000\nHello\n",
            SubtitleFormatType::Srt,
            Duration::from_secs(1),
            Duration::from_secs(2),
        ),
        (
            "vtt",
            "WEBVTT\n\n1\n00:00:01.000 --> 00:00:02.000\nHello\n",
            SubtitleFormatType::Vtt,
            Duration::from_secs(1),
            Duration::from_secs(2),
        ),
        (
            "sub",
            "{25}{50}Hello\n",
            SubtitleFormatType::Sub,
            Duration::from_secs(1),
            Duration::from_secs(2),
        ),
        (
            "ass",
            "[Script Info]\nScriptType: v4.00+\n\n[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:01.00,0:00:02.00,Default,,0000,0000,0000,,Hello\n",
            SubtitleFormatType::Ass,
            Duration::from_secs(1),
            Duration::from_secs(2),
        ),
    ];

    for (name, content, format, start, end) in cases {
        let term_resp = r#"{"terms":[]}"#;
        let cues_resp = r#"{"translations":[{"id":"__ID0__","text":"翻譯後"}]}"#;
        let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, cues_resp]);
        let engine = TranslationEngine::new(provider, 10).unwrap();
        let request = TranslationRequest {
            target_language: "zh-TW".to_string(),
            source_language: None,
            glossary_text: None,
            context: None,
            glossary_entries: vec![],
        };

        let result = engine
            .translate_content(content, &request)
            .await
            .unwrap_or_else(|e| panic!("{name} should translate: {e}"));
        assert_eq!(result.subtitle.format, format, "{name} format preserved");
        assert_eq!(result.subtitle.entries.len(), 1, "{name} cue count");
        let entry = &result.subtitle.entries[0];
        assert_eq!(entry.start_time, start, "{name} start time");
        assert_eq!(entry.end_time, end, "{name} end time");
        assert_eq!(entry.text, "翻譯後", "{name} visible text translated");
    }
}

#[tokio::test]
async fn translation_engine_preserves_inline_formatting_tokens() {
    let term_resp = r#"{"terms":[]}"#;
    let cues_resp = r#"{"translations":[{"id":"__ID0__","text":"__FMT0__你好 愛麗絲__FMT1__"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, cues_resp]);
    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["<i>Hello Alice</i>"]);
    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: Some("en".to_string()),
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    assert_eq!(result.subtitle.entries[0].text, "<i>你好 愛麗絲</i>");
}

#[tokio::test]
async fn translation_engine_rejects_missing_formatting_placeholder() {
    let term_resp = r#"{"terms":[]}"#;
    let cues_resp = r#"{"translations":[{"id":"__ID0__","text":"你好 愛麗絲"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, cues_resp]);
    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["{\\i1}Hello Alice{\\i0}"]);
    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: Some("en".to_string()),
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let err = engine
        .translate_subtitle(subtitle, &request)
        .await
        .expect_err("missing formatting placeholder must fail");
    assert!(err.to_string().contains("formatting placeholder"));
}
