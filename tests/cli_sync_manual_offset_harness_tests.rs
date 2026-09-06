//! Harness shim: Cargo auto-discovers only `tests/*.rs`, so this top-level
//! crate root exists solely to compile
//! `tests/cli/sync_manual_offset_integration_tests.rs` as an integration test
//! target. Shims contain no tests of their own.

#[path = "cli/sync_manual_offset_integration_tests.rs"]
mod sync_manual_offset_integration_tests;
