//! Harness shim: Cargo auto-discovers only `tests/*.rs`, so this top-level
//! crate root exists solely to compile `tests/cli/detect_encoding_args_tests.rs`
//! as an integration test target. Shims contain no tests of their own.

#[path = "cli/detect_encoding_args_tests.rs"]
mod detect_encoding_args_tests;
