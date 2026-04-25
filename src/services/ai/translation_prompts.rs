//! Prompt builders and response parsers for AI-driven subtitle translation.
//!
//! Two prompt families are supported:
//!
//! - Terminology extraction prompts collect proper nouns from a subtitle file
//!   into a structured source-to-target term map. The prompt encodes the
//!   naming policy: prefer established conventional translations, then
//!   phonetic transliteration, and only fall back to semantic translation
//!   when transliteration would mislead.
//! - Cue translation prompts ask the provider to translate a batch of cue
//!   texts identified by stable `UUIDv7` cue IDs. Responses are validated
//!   against the requested ID set so duplicate and unknown IDs are rejected;
//!   callers may either require complete coverage or retry omitted IDs.
//!
//! The helpers in this module are intentionally provider-neutral; they only
//! produce strings and parse strings, leaving HTTP and retry concerns to the
//! individual provider clients (`OpenAIClient`, `OpenRouterClient`,
//! `AzureOpenAIClient`).

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Deserialize;
use serde_json::Value;

use crate::Result;
use crate::error::SubXError;

const UNKNOWN_CUE_ID_ERROR_MARKER: &str = "Translation response contained unknown cue id";

/// System message for terminology extraction prompts.
pub const TERMINOLOGY_SYSTEM_MESSAGE: &str = "You are a professional subtitle terminology assistant. \
Identify recurring proper nouns such as person names and place names that need consistent \
translation. Prefer established conventional translations when they exist in the target \
language. When coining a new translation, prefer phonetic transliteration before semantic \
translation, and use semantic translation only when transliteration would be misleading. \
Respond with strict JSON only.";

/// System message for batched cue translation prompts.
pub const TRANSLATION_SYSTEM_MESSAGE: &str = "You are a professional subtitle translator. \
Translate visible cue text into the requested target language while preserving meaning, \
tone, and the cue ID associated with each line. Use the supplied terminology map exactly \
when the source contains a listed term. Respond with strict JSON only.";

/// Build a terminology extraction prompt.
///
/// # Arguments
///
/// * `target_language` - BCP-47 or natural-language target identifier.
/// * `source_language` - Optional source language hint.
/// * `cue_texts` - Visible cue texts in subtitle order. The terminology pass
///   typically processes the entire file so recurring names can be detected.
/// * `glossary_text` - Optional user-provided glossary content; included in
///   the prompt as authoritative guidance.
/// * `context` - Optional inline domain/tone guidance.
pub fn build_terminology_prompt(
    target_language: &str,
    source_language: Option<&str>,
    cue_texts: &[String],
    glossary_text: Option<&str>,
    context: Option<&str>,
) -> String {
    let mut prompt = String::new();
    prompt.push_str(
        "Extract recurring proper nouns (people, places, organizations, fictional named \
entities) from the subtitle text below and produce a translation map.\n\n",
    );
    prompt.push_str(&format!("Target language: {}\n", target_language));
    if let Some(src) = source_language {
        prompt.push_str(&format!("Source language: {}\n", src));
    } else {
        prompt.push_str("Source language: auto-detect\n");
    }

    prompt.push_str(
        "\nNaming policy:\n\
        - If a target-language conventional translation exists for a name, use it.\n\
        - Otherwise prefer phonetic transliteration over semantic translation.\n\
        - Use semantic translation only when transliteration is unsuitable or would mislead.\n\
        - Do not invent terms that do not appear in the source text.\n\
        - Return an empty map if no proper nouns recur.\n",
    );

    if let Some(glossary) = glossary_text {
        if !glossary.trim().is_empty() {
            prompt.push_str("\nUser glossary (authoritative, prefer these mappings):\n");
            prompt.push_str(glossary.trim());
            prompt.push('\n');
        }
    }
    if let Some(ctx) = context {
        if !ctx.trim().is_empty() {
            prompt.push_str("\nAdditional context:\n");
            prompt.push_str(ctx.trim());
            prompt.push('\n');
        }
    }

    prompt.push_str("\nSubtitle text (one cue per line):\n");
    for text in cue_texts {
        let single_line: String = text.replace('\n', " ");
        prompt.push_str(&format!("- {}\n", single_line));
    }

    prompt.push_str(
        "\nResponse format must be strict JSON with this shape and no additional commentary:\n\
{\n\
  \"terms\": [\n\
    { \"source\": \"Alice\", \"target\": \"愛麗絲\" }\n\
  ]\n\
}\n",
    );
    prompt
}

/// Build a batched cue translation prompt.
///
/// # Arguments
///
/// * `target_language` - Required target language for the output cue text.
/// * `source_language` - Optional source language hint.
/// * `terminology` - Effective terminology map (glossary entries already
///   merged on top of generated terms).
/// * `glossary_text` - Optional user glossary text; included in addition to
///   the terminology map for tone/style guidance.
/// * `context` - Optional inline guidance such as "Use formal tone".
/// * `cues` - Cue ID and visible text pairs in subtitle order.
pub fn build_translation_prompt(
    target_language: &str,
    source_language: Option<&str>,
    terminology: &BTreeMap<String, String>,
    glossary_text: Option<&str>,
    context: Option<&str>,
    cues: &[(String, String)],
) -> String {
    let mut prompt = String::new();
    prompt.push_str(
        "Translate each subtitle cue into the requested target language. Each cue has a \
unique ID that you MUST repeat in your response. Translate only human-visible text. Preserve every \
SUBX formatting placeholder token exactly where it appears; these placeholders represent subtitle \
formatting that must not be translated. Do not include timestamps.\n\n",
    );
    prompt.push_str(&format!("Target language: {}\n", target_language));
    if let Some(src) = source_language {
        prompt.push_str(&format!("Source language: {}\n", src));
    } else {
        prompt.push_str("Source language: auto-detect\n");
    }

    if !terminology.is_empty() {
        prompt.push_str(
            "\nTerminology map (use these translations exactly when the source \
text contains the listed term):\n",
        );
        for (source, target) in terminology {
            prompt.push_str(&format!("- {} -> {}\n", source, target));
        }
    }
    if let Some(glossary) = glossary_text {
        if !glossary.trim().is_empty() {
            prompt.push_str("\nUser glossary (authoritative tone/term guidance):\n");
            prompt.push_str(glossary.trim());
            prompt.push('\n');
        }
    }
    if let Some(ctx) = context {
        if !ctx.trim().is_empty() {
            prompt.push_str("\nAdditional context:\n");
            prompt.push_str(ctx.trim());
            prompt.push('\n');
        }
    }

    prompt.push_str("\nCues to translate:\n");
    for (id, text) in cues {
        let single_line: String = text.replace('\n', " ");
        prompt.push_str(&format!("- id: {}\n  text: {}\n", id, single_line));
    }

    prompt.push_str(
        "\nResponse format must be strict JSON with this shape and no additional commentary. \
Include every requested id exactly once and translate only the visible text:\n\
{\n\
  \"translations\": [\n\
    { \"id\": \"<UUIDv7>\", \"text\": \"<translated text>\" }\n\
  ]\n\
}\n",
    );
    prompt
}

#[derive(Debug, Deserialize)]
struct RawTerminologyEntry {
    source: String,
    target: String,
}

#[derive(Debug, Deserialize)]
struct RawTerminologyResponse {
    terms: Vec<RawTerminologyEntry>,
}

#[derive(Debug, Deserialize)]
struct RawTranslationEntry {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct RawTranslationResponse {
    translations: Vec<RawTranslationEntry>,
}

/// Parse a terminology-extraction response into an ordered map.
///
/// Empty maps are valid; they indicate the AI provider could not find
/// recurring proper nouns and translation should still proceed.
///
/// # Errors
///
/// Returns [`SubXError::AiService`] when the response is not valid JSON, the
/// schema is wrong, any entry has an empty `source` or `target` value, or a
/// source term is duplicated.
pub fn parse_terminology_response(response: &str) -> Result<BTreeMap<String, String>> {
    let json_str = extract_json_object(response)
        .ok_or_else(|| SubXError::ai_service("Terminology response did not contain JSON"))?;
    let raw: RawTerminologyResponse = serde_json::from_str(json_str).map_err(|e| {
        SubXError::ai_service(format!("Failed to parse terminology response: {}", e))
    })?;

    let mut map = BTreeMap::new();
    for entry in raw.terms {
        let source = entry.source.trim().to_string();
        let target = entry.target.trim().to_string();
        if source.is_empty() || target.is_empty() {
            return Err(SubXError::ai_service(
                "Terminology entry has empty source or target",
            ));
        }
        if map.contains_key(&source) {
            return Err(SubXError::ai_service(format!(
                "Terminology response contained duplicate source term: {}",
                source
            )));
        }
        map.insert(source, target);
    }
    Ok(map)
}

/// Parse a translation-batch response and validate cue ID coverage.
///
/// Validates that:
///
/// - Response JSON parses against the documented schema.
/// - Every requested cue ID appears exactly once.
/// - No unknown cue IDs are returned.
///
/// # Errors
///
/// Returns [`SubXError::AiService`] for malformed JSON, missing cue IDs,
/// duplicate cue IDs, or unknown cue IDs.
pub fn parse_translation_response(
    response: &str,
    expected_ids: &[String],
) -> Result<HashMap<String, String>> {
    let translations = parse_translation_response_partial(response, expected_ids)?;
    if translations.len() != expected_ids.len() {
        let missing: Vec<&String> = expected_ids
            .iter()
            .filter(|id| !translations.contains_key(id.as_str()))
            .collect();
        return Err(SubXError::ai_service(format!(
            "Translation response missing cue ids: {:?}",
            missing
        )));
    }

    Ok(translations)
}

/// Parse a translation-batch response while allowing omitted expected IDs.
///
/// The parser still rejects malformed JSON, empty IDs, duplicate IDs, and
/// unknown IDs. Missing expected IDs are left out of the returned map so callers
/// can retry or apply a documented fallback policy.
///
/// # Errors
///
/// Returns [`SubXError::AiService`] for malformed JSON, empty cue IDs,
/// duplicate cue IDs, or unknown cue IDs.
pub fn parse_translation_response_partial(
    response: &str,
    expected_ids: &[String],
) -> Result<HashMap<String, String>> {
    let json_str = extract_json_object(response)
        .ok_or_else(|| SubXError::ai_service("Translation response did not contain JSON"))?;
    let raw: RawTranslationResponse = serde_json::from_str(json_str).map_err(|e| {
        SubXError::ai_service(format!("Failed to parse translation response: {}", e))
    })?;

    let expected_set: HashSet<&String> = expected_ids.iter().collect();
    let mut seen: HashSet<String> = HashSet::new();
    let mut translations: HashMap<String, String> = HashMap::new();

    for entry in raw.translations {
        let id = entry.id.trim().to_string();
        if id.is_empty() {
            return Err(SubXError::ai_service("Translation entry has empty cue id"));
        }
        if !expected_set.contains(&id) {
            return Err(unknown_cue_id_error(&id));
        }
        if !seen.insert(id.clone()) {
            return Err(SubXError::ai_service(format!(
                "Translation response contained duplicate cue id: {}",
                id
            )));
        }
        translations.insert(id, entry.text);
    }

    Ok(translations)
}

/// Check whether an error was produced for an unknown translation cue ID.
///
/// # Arguments
///
/// * `err` - Error returned by a translation response parser.
///
/// # Returns
///
/// Returns `true` when the error came from a response containing a cue ID that
/// was not present in the request for that batch.
pub fn is_unknown_cue_id_error(err: &SubXError) -> bool {
    matches!(err, SubXError::AiService(message) if message.contains(UNKNOWN_CUE_ID_ERROR_MARKER))
}

fn unknown_cue_id_error(id: &str) -> SubXError {
    SubXError::ai_service(format!("{UNKNOWN_CUE_ID_ERROR_MARKER}: {id}"))
}

/// Extract the outermost JSON object from a free-form AI response.
fn extract_json_object(response: &str) -> Option<&str> {
    let start = response.find('{')?;
    let end = response.rfind('}')?;
    if end < start {
        return None;
    }
    let candidate = &response[start..=end];
    // Ensure it parses as some JSON value before handing it back.
    if serde_json::from_str::<Value>(candidate).is_ok() {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminology_prompt_includes_policy() {
        let cues = vec!["Alice meets Wonderland.".to_string()];
        let prompt = build_terminology_prompt("zh-TW", Some("en"), &cues, None, None);
        assert!(prompt.contains("Target language: zh-TW"));
        assert!(prompt.contains("Source language: en"));
        assert!(prompt.contains("conventional translation"));
        assert!(prompt.contains("phonetic transliteration"));
        assert!(prompt.contains("Alice meets Wonderland."));
        assert!(prompt.contains("\"terms\""));
    }

    #[test]
    fn translation_prompt_lists_terminology_map() {
        let mut term = BTreeMap::new();
        term.insert("Alice".to_string(), "愛麗絲".to_string());
        let cues = vec![(
            "00000000-aaaa-7000-8000-000000000000".to_string(),
            "Hi Alice".to_string(),
        )];
        let prompt =
            build_translation_prompt("zh-TW", None, &term, None, Some("Use formal tone"), &cues);
        assert!(prompt.contains("Target language: zh-TW"));
        assert!(prompt.contains("Source language: auto-detect"));
        assert!(prompt.contains("Alice -> 愛麗絲"));
        assert!(prompt.contains("Use formal tone"));
        assert!(prompt.contains("00000000-aaaa-7000-8000-000000000000"));
        assert!(prompt.contains("\"translations\""));
    }

    #[test]
    fn parse_terminology_handles_empty_map() {
        let map = parse_terminology_response(r#"{"terms": []}"#).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn parse_terminology_rejects_empty_fields() {
        let err =
            parse_terminology_response(r#"{"terms":[{"source":"","target":"x"}]}"#).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn parse_translation_validates_ids() {
        let ids = vec!["a".to_string(), "b".to_string()];
        let resp = r#"{"translations":[{"id":"a","text":"x"},{"id":"b","text":"y"}]}"#;
        let map = parse_translation_response(resp, &ids).unwrap();
        assert_eq!(map.get("a").unwrap(), "x");
        assert_eq!(map.get("b").unwrap(), "y");
    }

    #[test]
    fn parse_translation_rejects_missing_id() {
        let ids = vec!["a".to_string(), "b".to_string()];
        let resp = r#"{"translations":[{"id":"a","text":"x"}]}"#;
        let err = parse_translation_response(resp, &ids).unwrap_err();
        assert!(err.to_string().contains("missing cue ids"));
    }

    #[test]
    fn parse_translation_rejects_unknown_id() {
        let ids = vec!["a".to_string()];
        let resp = r#"{"translations":[{"id":"z","text":"x"}]}"#;
        let err = parse_translation_response(resp, &ids).unwrap_err();
        assert!(err.to_string().contains("unknown cue id"));
    }

    #[test]
    fn parse_translation_rejects_duplicate_id() {
        let ids = vec!["a".to_string()];
        let resp = r#"{"translations":[{"id":"a","text":"x"},{"id":"a","text":"y"}]}"#;
        let err = parse_translation_response(resp, &ids).unwrap_err();
        assert!(err.to_string().contains("duplicate cue id"));
    }
}
