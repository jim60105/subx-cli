//! Top-level wiring entry that exposes
//! `tests/cli/output_format_sync.rs` to Cargo's integration-test
//! discovery (Cargo only auto-discovers `tests/*.rs`, not nested
//! directories).

#[path = "common/mod.rs"]
mod common;

#[path = "cli/output_format_sync.rs"]
mod output_format_sync;
