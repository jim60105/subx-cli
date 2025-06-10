//! Test helpers for CLI testing with dependency injection configuration.
//!
//! This module provides comprehensive testing utilities that support the new
//! dependency injection-based configuration system, enabling isolated and
//! parallel test execution without global state.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use subx_cli::Result;
use subx_cli::config::{ConfigService, TestConfigService};
use tempfile::TempDir;
use tokio::fs;

/// CLI test helper with dependency injection support.
///
/// This helper provides a complete testing environment with isolated configuration,
/// temporary directories, and utilities for CLI command testing.
pub struct CLITestHelper {
    temp_dir: TempDir,
    test_files: Vec<PathBuf>,
    config_service: Arc<dyn ConfigService>,
}

impl CLITestHelper {
    /// Create a new CLI test helper with default test configuration.
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let config_service = Arc::new(TestConfigService::with_defaults());

        Self {
            temp_dir,
            test_files: Vec::new(),
            config_service,
        }
    }

    /// Create a CLI test helper with custom configuration service.
    pub fn with_config_service(config_service: Arc<dyn ConfigService>) -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");

        Self {
            temp_dir,
            test_files: Vec::new(),
            config_service,
        }
    }

    /// Create a CLI test helper with AI settings.
    pub fn with_ai_settings(provider: &str, model: &str) -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let config_service = Arc::new(TestConfigService::with_ai_settings(provider, model));

        Self {
            temp_dir,
            test_files: Vec::new(),
            config_service,
        }
    }

    /// Create a CLI test helper with sync settings.
    pub fn with_sync_settings(correlation_threshold: f32, max_offset: f32) -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let config_service = Arc::new(TestConfigService::with_sync_settings(
            correlation_threshold,
            max_offset,
        ));

        Self {
            temp_dir,
            test_files: Vec::new(),
            config_service,
        }
    }

    /// Get the temporary directory path.
    pub fn temp_dir_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Get a reference to the configuration service.
    pub fn config_service(&self) -> Arc<dyn ConfigService> {
        self.config_service.clone()
    }

    /// Create an isolated test workspace with standard test files.
    pub async fn create_isolated_test_workspace(&mut self) -> Result<PathBuf> {
        let workspace = self.temp_dir.path().to_path_buf();

        // Create standard test file structure
        self.create_media_files(&workspace).await?;
        self.create_subtitle_files(&workspace).await?;
        self.create_config_file(&workspace).await?;

        Ok(workspace)
    }

    /// Create test media files for testing.
    async fn create_media_files(&mut self, workspace: &Path) -> Result<()> {
        let media_dir = workspace.join("media");
        fs::create_dir_all(&media_dir).await?;

        // Create dummy video files
        let video_files = [
            "movie1.mp4",
            "movie2.avi",
            "series_s01e01.mkv",
            "series_s01e02.mkv",
        ];

        for filename in &video_files {
            let file_path = media_dir.join(filename);
            fs::write(&file_path, b"dummy video content").await?;
            self.test_files.push(file_path);
        }

        Ok(())
    }

    /// Create test subtitle files for testing.
    async fn create_subtitle_files(&mut self, workspace: &Path) -> Result<()> {
        let subtitle_dir = workspace.join("subtitles");
        fs::create_dir_all(&subtitle_dir).await?;

        // Create SRT subtitle files
        let srt_content = r#"1
00:00:01,000 --> 00:00:03,000
Hello, this is a test subtitle.

2
00:00:04,000 --> 00:00:06,000
This is the second subtitle entry.
"#;

        let subtitle_files = [
            "subtitle1.srt",
            "subtitle2.srt",
            "series_s01e01.srt",
            "series_s01e02.srt",
        ];

        for filename in &subtitle_files {
            let file_path = subtitle_dir.join(filename);
            fs::write(&file_path, srt_content).await?;
            self.test_files.push(file_path);
        }

        Ok(())
    }

    /// Create a test configuration file.
    async fn create_config_file(&mut self, workspace: &Path) -> Result<()> {
        let config_content = format!(
            r#"
[general]
workspace = "{}"
log_level = "debug"
enable_progress_bar = false

[test]
isolated = true
parallel_safe = true
timestamp = "{}"

[ai]
provider = "test"
model = "test-model"

[sync]
correlation_threshold = 0.8
max_offset_seconds = 30.0

[formats]
default_output = "srt"
preserve_styling = false
"#,
            workspace.display(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let config_path = workspace.join("test_config.toml");
        fs::write(&config_path, config_content).await?;
        self.test_files.push(config_path);

        Ok(())
    }

    /// Create a test subtitle file with specific content.
    pub async fn create_subtitle_file(&mut self, filename: &str, content: &str) -> Result<PathBuf> {
        let file_path = self.temp_dir.path().join(filename);
        fs::write(&file_path, content).await?;
        self.test_files.push(file_path.clone());
        Ok(file_path)
    }

    /// Create a test video file.
    pub async fn create_video_file(&mut self, filename: &str) -> Result<PathBuf> {
        let file_path = self.temp_dir.path().join(filename);
        fs::write(&file_path, b"dummy video content").await?;
        self.test_files.push(file_path.clone());
        Ok(file_path)
    }

    /// Get a list of all created test files.
    pub fn test_files(&self) -> &[PathBuf] {
        &self.test_files
    }

    /// Clean up test files (called automatically on drop).
    pub fn cleanup(&mut self) {
        self.test_files.clear();
        // TempDir will handle cleanup automatically
    }
}

impl Default for CLITestHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CLITestHelper {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Output validator for testing command outputs.
pub struct OutputValidator {
    patterns: Vec<regex::Regex>,
    anti_patterns: Vec<regex::Regex>,
}

impl OutputValidator {
    /// Create a new output validator.
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            anti_patterns: Vec::new(),
        }
    }

    /// Add a pattern that should be found in the output.
    pub fn expect_pattern(mut self, pattern: &str) -> Self {
        self.patterns
            .push(regex::Regex::new(pattern).expect("Invalid regex pattern"));
        self
    }

    /// Add a pattern that should NOT be found in the output.
    pub fn reject_pattern(mut self, pattern: &str) -> Self {
        self.anti_patterns
            .push(regex::Regex::new(pattern).expect("Invalid regex pattern"));
        self
    }

    /// Validate the output against all patterns.
    pub fn validate(&self, output: &str) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Check expected patterns
        for pattern in &self.patterns {
            if pattern.is_match(output) {
                result.add_success(format!("Pattern found: {}", pattern.as_str()));
            } else {
                result.add_failure(format!("Pattern not found: {}", pattern.as_str()));
            }
        }

        // Check anti-patterns
        for pattern in &self.anti_patterns {
            if pattern.is_match(output) {
                result.add_failure(format!("Forbidden pattern found: {}", pattern.as_str()));
            } else {
                result.add_success(format!(
                    "Forbidden pattern correctly absent: {}",
                    pattern.as_str()
                ));
            }
        }

        result
    }
}

impl Default for OutputValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of output validation.
#[derive(Debug)]
pub struct ValidationResult {
    successes: Vec<String>,
    failures: Vec<String>,
}

impl ValidationResult {
    fn new() -> Self {
        Self {
            successes: Vec::new(),
            failures: Vec::new(),
        }
    }

    fn add_success(&mut self, message: String) {
        self.successes.push(message);
    }

    fn add_failure(&mut self, message: String) {
        self.failures.push(message);
    }

    /// Check if validation passed (no failures).
    pub fn is_valid(&self) -> bool {
        self.failures.is_empty()
    }

    /// Get the list of validation failures.
    pub fn failures(&self) -> &[String] {
        &self.failures
    }

    /// Get the list of validation successes.
    pub fn successes(&self) -> &[String] {
        &self.successes
    }

    /// Get a summary of the validation result.
    pub fn summary(&self) -> String {
        format!(
            "Validation: {} successes, {} failures",
            self.successes.len(),
            self.failures.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cli_helper_creation() {
        let helper = CLITestHelper::new();
        assert!(helper.temp_dir_path().exists());
        assert!(helper.test_files().is_empty());
    }

    #[tokio::test]
    async fn test_cli_helper_with_ai_settings() {
        let helper = CLITestHelper::with_ai_settings("openai", "gpt-4");
        let config = helper.config_service().get_config().unwrap();
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4");
    }

    #[tokio::test]
    async fn test_cli_helper_workspace_creation() {
        let mut helper = CLITestHelper::new();
        let workspace = helper.create_isolated_test_workspace().await.unwrap();

        assert!(workspace.join("media").exists());
        assert!(workspace.join("subtitles").exists());
        assert!(workspace.join("test_config.toml").exists());
        assert!(!helper.test_files().is_empty());
    }

    #[tokio::test]
    async fn test_output_validator() {
        let validator = OutputValidator::new()
            .expect_pattern(r"✓.*success")
            .reject_pattern(r"✗.*error");

        let output = "✓ Operation completed successfully";
        let result = validator.validate(output);

        assert!(result.is_valid());
        assert_eq!(result.successes().len(), 2);
        assert_eq!(result.failures().len(), 0);
    }

    #[tokio::test]
    async fn test_output_validator_failure() {
        let validator = OutputValidator::new()
            .expect_pattern(r"✓.*success")
            .reject_pattern(r"✗.*error");

        let output = "✗ Operation failed with error";
        let result = validator.validate(output);

        assert!(!result.is_valid());
        assert_eq!(result.failures().len(), 2); // missing success pattern + forbidden error pattern
    }
}
