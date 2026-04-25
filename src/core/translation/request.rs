//! Translation request/response data structures.
//!
//! These types describe the inputs and outputs of the translation engine
//! independently of any particular AI provider. They are intentionally small
//! and serializable so they can be reused by CLI helpers, integration tests,
//! and structured logging.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Effective terminology map (source term -> target term).
///
/// A [`BTreeMap`] is used so iteration order is deterministic, which keeps
/// translation prompts stable across runs.
pub type TerminologyMap = BTreeMap<String, String>;

/// A single terminology entry returned by the AI extraction pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyEntry {
    /// Source-language term as it appears in subtitle text.
    pub source: String,
    /// Translated term to use consistently in the target language.
    pub target: String,
}

/// User-provided glossary entry, parsed from a UTF-8 glossary file.
///
/// Glossary entries take precedence over AI-generated terminology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryEntry {
    /// Source-language term.
    pub source: String,
    /// Authoritative target-language translation.
    pub target: String,
}

/// One cue in a translation request, identified by its UUIDv7 cue ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TranslationCue {
    /// Stable UUIDv7 cue ID assigned in subtitle order.
    pub id: String,
    /// Visible cue text passed to the AI provider.
    pub text: String,
}

/// Input parameters for translating a single subtitle file.
#[derive(Debug, Clone)]
pub struct TranslationRequest {
    /// Required target language identifier (e.g. `zh-TW`, `ja`).
    pub target_language: String,
    /// Optional source language hint.
    pub source_language: Option<String>,
    /// Optional UTF-8 glossary text to include in prompts as authoritative
    /// guidance.
    pub glossary_text: Option<String>,
    /// Optional inline domain/tone guidance.
    pub context: Option<String>,
    /// Pre-parsed user glossary entries (already authoritative).
    pub glossary_entries: Vec<GlossaryEntry>,
}

/// A batch of cues sent in a single AI translation request.
#[derive(Debug, Clone)]
pub struct TranslationBatch {
    /// Cues in this batch, in subtitle order.
    pub cues: Vec<TranslationCue>,
}

/// Outcome metadata for a successful translation pass.
#[derive(Debug, Clone, Default)]
pub struct TranslationOutcome {
    /// Effective terminology map used for the translation prompt
    /// (glossary entries already merged on top of generated terms).
    pub effective_terminology: TerminologyMap,
    /// Number of cues that were translated.
    pub translated_cue_count: usize,
    /// Number of AI translation batches issued.
    pub batch_count: usize,
}

/// Result returned by the engine after translating a single subtitle file.
#[derive(Debug, Clone)]
pub struct TranslationResult {
    /// Translated subtitle in the format-agnostic representation. Use
    /// [`crate::core::formats::manager::FormatManager`] to serialize it back
    /// to a format-specific text representation.
    pub subtitle: crate::core::formats::Subtitle,
    /// Outcome metadata (terminology map, batch counts, etc.).
    pub outcome: TranslationOutcome,
}

/// Merge a generated terminology map with user glossary entries.
///
/// User glossary entries override generated entries with the same source
/// term, matching the policy in the OpenSpec change.
pub fn merge_terminology(generated: TerminologyMap, glossary: &[GlossaryEntry]) -> TerminologyMap {
    let mut merged = generated;
    for entry in glossary {
        let source = entry.source.trim().to_string();
        let target = entry.target.trim().to_string();
        if source.is_empty() || target.is_empty() {
            continue;
        }
        merged.insert(source, target);
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glossary_overrides_generated_terms() {
        let mut generated = TerminologyMap::new();
        generated.insert("Alice".to_string(), "愛麗絲".to_string());
        generated.insert("Bob".to_string(), "鮑伯".to_string());

        let glossary = vec![GlossaryEntry {
            source: "Alice".to_string(),
            target: "艾莉絲".to_string(),
        }];
        let merged = merge_terminology(generated, &glossary);
        assert_eq!(merged.get("Alice").unwrap(), "艾莉絲");
        assert_eq!(merged.get("Bob").unwrap(), "鮑伯");
    }

    #[test]
    fn empty_glossary_entries_are_ignored() {
        let mut generated = TerminologyMap::new();
        generated.insert("Alice".to_string(), "愛麗絲".to_string());
        let glossary = vec![GlossaryEntry {
            source: "  ".to_string(),
            target: "x".to_string(),
        }];
        let merged = merge_terminology(generated, &glossary);
        assert_eq!(merged.len(), 1);
    }
}
