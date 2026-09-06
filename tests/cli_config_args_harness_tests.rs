//! Harness shim: Cargo auto-discovers only `tests/*.rs`, so this top-level
//! crate root exists solely to compile `tests/cli/config_args_tests.rs` as an
//! integration test target. Shims contain no tests of their own.

#[path = "cli/config_args_tests.rs"]
mod config_args_tests;
