//! Harness shim: Cargo auto-discovers only `tests/*.rs`, so this top-level
//! crate root exists solely to compile `tests/cli/ui_tests.rs` as an
//! integration test target. Shims contain no tests of their own.

#[path = "cli/ui_tests.rs"]
mod ui_tests;
