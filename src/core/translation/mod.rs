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
//! - [`uuidv7`] - UUIDv7 generator with strict 1ms spacing.
//! - [`engine`] - high-level [`engine::TranslationEngine`].

pub mod engine;
pub mod request;
pub mod uuidv7;

pub use engine::{TranslationEngine, parse_glossary_text};
pub use request::{
    GlossaryEntry, TerminologyEntry, TerminologyMap, TranslationBatch, TranslationCue,
    TranslationOutcome, TranslationRequest, TranslationResult,
};
pub use uuidv7::{CueIdGenerator, generate_cue_ids};
