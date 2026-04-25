//! Integration tests for translation prompt builders and response parsers.
//!
//! These tests assert prompt content (terminology naming policy,
//! transliteration vs semantic preference, glossary inclusion, terminology
//! map injection) and the structural validation rules that the translation
//! response parser enforces (missing, unknown, duplicate cue IDs, malformed
//! terminology entries).

use std::collections::BTreeMap;

use subx_cli::services::ai::translation_prompts::{
    TERMINOLOGY_SYSTEM_MESSAGE, TRANSLATION_SYSTEM_MESSAGE, build_terminology_prompt,
    build_translation_prompt, parse_terminology_response, parse_translation_response,
    parse_translation_response_partial,
};

#[test]
fn terminology_prompt_encodes_naming_policy() {
    let cues = vec![
        "Alice meets Bob in Wonderland.".to_string(),
        "Alice waves at Bob.".to_string(),
    ];
    let prompt = build_terminology_prompt("zh-TW", Some("en"), &cues, None, None);

    // Languages are explicit.
    assert!(prompt.contains("Target language: zh-TW"));
    assert!(prompt.contains("Source language: en"));

    // Established conventional translations are preferred when available.
    assert!(
        prompt.to_lowercase().contains("conventional"),
        "prompt must instruct the model to use established conventional translations: {prompt}"
    );

    // Coined translations prefer phonetic transliteration before semantic
    // translation.
    let lower = prompt.to_lowercase();
    let translit = lower
        .find("transliteration")
        .expect("must mention transliteration");
    let semantic = lower
        .find("semantic translation")
        .expect("must mention semantic translation");
    assert!(
        translit < semantic,
        "transliteration guidance must appear before semantic translation guidance"
    );

    // Cue text must be present so the model can extract terms.
    assert!(prompt.contains("Alice meets Bob in Wonderland."));

    // The schema hint is included.
    assert!(prompt.contains("\"terms\""));
}

#[test]
fn terminology_prompt_auto_detects_when_source_language_omitted() {
    let cues = vec!["Alice waves.".to_string()];
    let prompt = build_terminology_prompt("ja", None, &cues, None, None);
    assert!(prompt.contains("Source language: auto-detect"));
}

#[test]
fn terminology_prompt_includes_user_glossary_text_and_context() {
    let cues = vec!["Alice waves.".to_string()];
    let prompt = build_terminology_prompt(
        "zh-TW",
        Some("en"),
        &cues,
        Some("Alice = 艾莉絲"),
        Some("anime fansub terminology"),
    );
    assert!(prompt.contains("Alice = 艾莉絲"));
    assert!(prompt.contains("anime fansub terminology"));
}

#[test]
fn translation_prompt_includes_terminology_map_and_cue_ids() {
    let mut term = BTreeMap::new();
    term.insert("Alice".to_string(), "愛麗絲".to_string());
    let cues = vec![
        (
            "01900000-1111-7000-8000-000000000001".to_string(),
            "Hi Alice".to_string(),
        ),
        (
            "01900000-1111-7000-8000-000000000002".to_string(),
            "Bye Alice".to_string(),
        ),
    ];
    let prompt = build_translation_prompt(
        "zh-TW",
        Some("en"),
        &term,
        None,
        Some("Use formal tone"),
        &cues,
    );

    assert!(prompt.contains("Target language: zh-TW"));
    assert!(prompt.contains("Source language: en"));
    assert!(prompt.contains("Alice -> 愛麗絲"));
    assert!(prompt.contains("Use formal tone"));
    assert!(prompt.contains("01900000-1111-7000-8000-000000000001"));
    assert!(prompt.contains("01900000-1111-7000-8000-000000000002"));
    assert!(prompt.contains("\"translations\""));
    assert!(prompt.contains("formatting placeholder"));
}

#[test]
fn translation_prompt_omits_terminology_section_when_map_empty() {
    let term = BTreeMap::new();
    let cues = vec![(
        "00000000-aaaa-7000-8000-000000000000".to_string(),
        "Hi".to_string(),
    )];
    let prompt = build_translation_prompt("zh-TW", None, &term, None, None, &cues);
    assert!(!prompt.contains("Terminology map"));
}

#[test]
fn system_messages_are_distinct_and_nonempty() {
    assert!(!TERMINOLOGY_SYSTEM_MESSAGE.is_empty());
    assert!(!TRANSLATION_SYSTEM_MESSAGE.is_empty());
    assert_ne!(TERMINOLOGY_SYSTEM_MESSAGE, TRANSLATION_SYSTEM_MESSAGE);
    // Both messages should mention strict JSON to keep response parsing
    // contracts visible to the provider.
    assert!(TERMINOLOGY_SYSTEM_MESSAGE.to_lowercase().contains("json"));
    assert!(TRANSLATION_SYSTEM_MESSAGE.to_lowercase().contains("json"));
}

#[test]
fn parse_terminology_accepts_empty_map() {
    let map = parse_terminology_response(r#"{"terms": []}"#).unwrap();
    assert!(map.is_empty());
}

#[test]
fn parse_terminology_rejects_malformed_json() {
    let err = parse_terminology_response("not json").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("JSON") || msg.contains("parse") || msg.contains("Terminology"),
        "unexpected error: {msg}"
    );
}

#[test]
fn parse_terminology_rejects_empty_target_field() {
    let err =
        parse_terminology_response(r#"{"terms":[{"source":"Alice","target":""}]}"#).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("empty"));
}

#[test]
fn parse_terminology_rejects_missing_terms_array() {
    let err = parse_terminology_response(r#"{}"#).unwrap_err();
    assert!(err.to_string().contains("terminology response"));
}

#[test]
fn parse_terminology_rejects_duplicate_source_terms() {
    let err = parse_terminology_response(
        r#"{"terms":[{"source":"Alice","target":"愛麗絲"},{"source":"Alice","target":"艾莉絲"}]}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("duplicate source term"));
}

#[test]
fn parse_translation_accepts_complete_mapping() {
    let ids = vec!["a".to_string(), "b".to_string()];
    let resp = r#"{"translations":[{"id":"a","text":"x"},{"id":"b","text":"y"}]}"#;
    let map = parse_translation_response(resp, &ids).unwrap();
    assert_eq!(map.get("a").unwrap(), "x");
    assert_eq!(map.get("b").unwrap(), "y");
}

#[test]
fn parse_translation_rejects_missing_cue_id() {
    let ids = vec!["a".to_string(), "b".to_string()];
    let resp = r#"{"translations":[{"id":"a","text":"x"}]}"#;
    let err = parse_translation_response(resp, &ids).unwrap_err();
    assert!(err.to_string().contains("missing cue ids"));
}

#[test]
fn parse_translation_partial_allows_missing_cue_id() {
    let ids = vec!["a".to_string(), "b".to_string()];
    let resp = r#"{"translations":[{"id":"a","text":"x"}]}"#;
    let map = parse_translation_response_partial(resp, &ids).unwrap();
    assert_eq!(map.len(), 1);
    assert_eq!(map.get("a").unwrap(), "x");
    assert!(!map.contains_key("b"));
}

#[test]
fn parse_translation_rejects_unknown_cue_id() {
    let ids = vec!["a".to_string(), "b".to_string()];
    let resp =
        r#"{"translations":[{"id":"a","text":"x"},{"id":"b","text":"y"},{"id":"z","text":"?"}]}"#;
    let err = parse_translation_response(resp, &ids).unwrap_err();
    assert!(err.to_string().contains("unknown cue id"));
}

#[test]
fn parse_translation_rejects_duplicate_cue_id() {
    let ids = vec!["a".to_string(), "b".to_string()];
    let resp =
        r#"{"translations":[{"id":"a","text":"x"},{"id":"a","text":"x2"},{"id":"b","text":"y"}]}"#;
    let err = parse_translation_response(resp, &ids).unwrap_err();
    assert!(err.to_string().contains("duplicate cue id"));
}

#[test]
fn parse_translation_rejects_malformed_json() {
    let ids = vec!["a".to_string()];
    let err = parse_translation_response("not json", &ids).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("JSON") || msg.contains("parse") || msg.contains("Translation"),
        "unexpected error: {msg}"
    );
}
