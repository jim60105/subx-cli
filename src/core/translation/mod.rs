//! Core subtitle translation engine.
//!
//! The translation engine sits above the existing [`crate::core::formats`]
//! pipeline. It parses subtitle files into the shared
//! [`crate::core::formats::Subtitle`] data model, builds AI translation
//! requests with stable `UUIDv7` cue IDs, validates AI responses, and
//! reapplies translated text back to the parsed entries while preserving
//! timing, cue ordering, cue counts, and metadata supported by the format
//! parser/writer pipeline.
//!
//! # Why UUIDv7 cue IDs?
//!
//! UUIDv7 IDs encode their generation order via the `unix_time_ts` field,
//! which makes batch logs, retries, and post-mortem auditing easier than
//! UUIDv4. The engine intentionally spaces adjacent cue ID generations by at
//! least 1 millisecond so each ID's `unix_time_ts` is strictly greater than
//! the previous cue ID timestamp, preventing same-millisecond ambiguity.
//!
//! # Module layout
//!
//! - [`request`] - request/response data structures.
//! - [`engine`] - high-level [`engine::TranslationEngine`].
//!
//! The UUIDv7 cue ID generator lives in [`crate::core::uuidv7`] and is
//! re-exported below for backward compatibility with the original
//! `core::translation::uuidv7` public path.

pub mod engine;
pub mod request;

/// Backward-compatibility shim for the original
/// `subx_cli::core::translation::uuidv7` module path.
///
/// The UUIDv7 generator was relocated to [`crate::core::uuidv7`] when it
/// became a shared dependency of the matcher and parallel layers; this
/// shim preserves the old import path for downstream code that still
/// references `subx_cli::core::translation::uuidv7::CueIdGenerator`.
pub mod uuidv7 {
    pub use crate::core::uuidv7::{
        Uuidv7Generator, Uuidv7Generator as CueIdGenerator, generate_ids,
        generate_ids as generate_cue_ids, unix_time_ms,
    };
}

pub use crate::core::uuidv7::{
    Uuidv7Generator as CueIdGenerator, generate_ids as generate_cue_ids, unix_time_ms,
};
pub use engine::{TranslationEngine, parse_glossary_text};
pub use request::{
    GlossaryEntry, TerminologyEntry, TerminologyMap, TranslationBatch, TranslationCue,
    TranslationOutcome, TranslationRequest, TranslationResult,
};
