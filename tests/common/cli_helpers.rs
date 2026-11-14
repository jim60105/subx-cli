//! CLI-side test helpers: binary spawning on top of
//! [`subx_core::test_support::workspace::TestWorkspace`] and the shared
//! JSON-mode assertions re-exported by `tests/common/mod.rs`.

use subx_core::test_support::workspace::TestWorkspace;

/// Variables that select or configure an AI provider.
///
/// Removed from every child a test spawns through this harness, so the test's
/// own endpoint stays authoritative: a `SUBX_AI_*` override, an ambient
/// `OPENAI_BASE_URL` or a real `OPENAI_API_KEY` in the developer's shell must
/// never decide which provider the child builds. Both this file's
/// `run_command_with_config` and the full-workflow helper in
/// `tests/integration_tests.rs` iterate this list instead of spelling it out.
///
/// Scope: the other binary-spawning harnesses in this crate still carry their
/// own inline copy; folding them in needs its own harness shim and is a
/// follow-up change, not a rider on this one.
pub const ISOLATED_AI_ENV_VARS: &[&str] = &[
    "OPENAI_API_KEY",
    "OPENAI_BASE_URL",
    "OPENROUTER_API_KEY",
    "AZURE_OPENAI_API_KEY",
    "AZURE_OPENAI_ENDPOINT",
    "AZURE_OPENAI_API_VERSION",
    "AZURE_OPENAI_DEPLOYMENT_ID",
    "LOCAL_LLM_BASE_URL",
    "LOCAL_LLM_API_KEY",
    "SUBX_AI_PROVIDER",
    "SUBX_AI_APIKEY",
    "SUBX_AI_BASE_URL",
    "SUBX_AI_MODEL",
];

/// Command execution result for testing.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub status_code: i32,
}

/// Extension trait carrying the binary-spawning half of the former
/// `CLITestHelper`, implemented for core's `TestWorkspace`.
///
/// It cannot live in `subx-core`: `run_command_with_config` spawns
/// `cargo run --`, and `env!("CARGO_MANIFEST_DIR")` inside the trait body
/// resolves to the crate that *defines* it — in `subx-core` that would point
/// at the library crate instead of the CLI binary. Keeping the definition
/// here preserves the environment-isolating spawn contract verbatim.
#[allow(dead_code)]
pub trait CliRun {
    /// Execute a command with the test configuration.
    fn run_command_with_config(
        &self,
        args: &[&str],
    ) -> impl std::future::Future<Output = CommandResult> + Send;

    /// Execute a command and assert it succeeds.
    fn run_command_expect_success(
        &self,
        args: &[&str],
    ) -> impl std::future::Future<Output = CommandResult> + Send;

    /// Assert that a command result was successful.
    fn assert_command_success(&self, result: &CommandResult);

    /// Assert that a command result failed.
    fn assert_command_failure(&self, result: &CommandResult);
}

impl CliRun for TestWorkspace {
    /// Execute a command with the test configuration.
    async fn run_command_with_config(&self, args: &[&str]) -> CommandResult {
        use std::process::Command;

        let mut cmd = Command::new("cargo");
        cmd.arg("run")
            .arg("--")
            .args(args)
            // Run commands from the project root so Cargo.toml is found
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            // Ensure program operates within test workspace
            .env("SUBX_WORKSPACE", self.temp_dir_path())
            .env(
                "SUBX_CONFIG_PATH",
                self.temp_dir_path().join("test_config.toml"),
            )
            .env("SUBX_TEST_MODE", "true")
            .env("RUST_LOG", "debug");
        // Isolate from the developer's shell: the AI-provider env vars
        // (e.g. `OPENAI_BASE_URL=http://...`) must not leak in, otherwise
        // the strict config gate (hosted provider + `http://` base URL)
        // rejects the merged config and the CLI exits with a config error.
        for var in ISOLATED_AI_ENV_VARS {
            cmd.env_remove(*var);
        }

        match cmd.output() {
            Ok(output) => CommandResult {
                success: output.status.success(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                status_code: output.status.code().unwrap_or(-1),
            },
            Err(e) => CommandResult {
                success: false,
                stdout: String::new(),
                stderr: format!("Failed to execute command: {}", e),
                status_code: -1,
            },
        }
    }

    /// Execute a command and assert it succeeds.
    #[allow(dead_code)]
    async fn run_command_expect_success(&self, args: &[&str]) -> CommandResult {
        let result = self.run_command_with_config(args).await;
        if !result.success {
            panic!(
                "Command failed:\nArgs: {:?}\nStdout: {}\nStderr: {}",
                args, result.stdout, result.stderr
            );
        }
        result
    }

    /// Assert that a command result was successful.
    #[allow(dead_code)]
    fn assert_command_success(&self, result: &CommandResult) {
        if !result.success {
            panic!(
                "Command failed:\nStdout: {}\nStderr: {}",
                result.stdout, result.stderr
            );
        }
    }

    /// Assert that a command result failed.
    #[allow(dead_code)]
    fn assert_command_failure(&self, result: &CommandResult) {
        if result.success {
            panic!(
                "Command unexpectedly succeeded:\nStdout: {}\nStderr: {}",
                result.stdout, result.stderr
            );
        }
    }
}
