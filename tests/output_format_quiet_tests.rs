//! Top-level harness so `tests/cli/output_format_quiet.rs` is
//! discovered by Cargo as an integration test crate.

#[path = "common/mod.rs"]
mod common;

#[path = "cli/output_format_quiet.rs"]
mod output_format_quiet;
