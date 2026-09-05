//! Shared harness for the CLI-side integration tests. The workspace/fixture
//! builder, mock servers, generators and file managers live in
//! `subx_core::test_support` (a `test-support`-gated module, enabled for this
//! crate's test targets through `[dev-dependencies]`); only the
//! binary-spawning half of the old `CLITestHelper` (`common::cli_helpers`,
//! implemented as the `CliRun` extension trait) and the JSON-mode assertions
//! (`common::json_output`) remain here.

pub mod cli_helpers;
pub mod json_output;
