//! Test file management tools, providing parallel test isolation and temporary file management.

use std::path::{Path, PathBuf};
use subx_cli::{Result, error::SubXError};
use tempfile::TempDir;
use tokio::fs;

/// Test file manager, supporting parallel test isolation
pub struct TestFileManager {
    temp_dirs: Vec<TempDir>,
    cleanup_on_drop: bool,
    isolation_enabled: bool,
}

impl TestFileManager {
    /// Create new test file manager
    pub fn new() -> Self {
        Self {
            temp_dirs: Vec::new(),
            cleanup_on_drop: true,
            isolation_enabled: true, // Default enable isolation mode
        }
    }

    /// Set to preserve files on failure
    pub fn preserve_on_failure(mut self) -> Self {
        self.cleanup_on_drop = false;
        self
    }

    /// Enable parallel isolation mode
    #[allow(dead_code)]
    pub fn enable_parallel_isolation(mut self) -> Self {
        self.isolation_enabled = true;
        self
    }

    /// Disable parallel isolation mode
    #[allow(dead_code)]
    pub fn disable_parallel_isolation(mut self) -> Self {
        self.isolation_enabled = false;
        self
    }

    /// Create isolated test directory
    pub async fn create_isolated_test_directory(&mut self, name: &str) -> Result<&Path> {
        let temp_dir = TempDir::new().map_err(|e| {
            SubXError::FileOperationFailed(format!("Failed to create temp dir: {}", e))
        })?;
        let path = temp_dir.path();

        if self.isolation_enabled {
            // Create a completely isolated environment for each test
            self.setup_isolated_environment(path, name).await?;
        }

        // For debugging convenience, create a symbolic link
        if cfg!(debug_assertions) {
            #[cfg(unix)]
            {
                let debug_path = format!("/tmp/subx_test_{}_{}", name, std::process::id());
                if let Err(e) = std::os::unix::fs::symlink(path, &debug_path) {
                    eprintln!("Failed to create debug symlink: {}", e);
                } else {
                    println!("Isolated test directory: {}", debug_path);
                }
            }
            #[cfg(windows)]
            {
                use std::env;
                if let Ok(temp_dir) = env::var("TEMP") {
                    let debug_path =
                        format!("{}\\subx_test_{}_{}", temp_dir, name, std::process::id());
                    println!("Isolated test directory: {}", path.display());
                    println!("Windows debug path reference: {}", debug_path);
                }
            }
        }

        self.temp_dirs.push(temp_dir);

        // Return the path of the last added temporary directory
        Ok(self.temp_dirs.last().unwrap().path())
    }

    /// Create test file
    pub async fn create_test_file(&self, dir: &Path, name: &str, content: &str) -> Result<PathBuf> {
        let file_path = dir.join(name);

        // Ensure the parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                SubXError::FileOperationFailed(format!("Failed to create directory: {}", e))
            })?;
        }

        fs::write(&file_path, content)
            .await
            .map_err(|e| SubXError::FileOperationFailed(format!("Failed to write file: {}", e)))?;

        Ok(file_path)
    }

    /// Create test configuration file
    pub async fn create_test_config(
        &self,
        dir: &Path,
        config_name: &str,
        workspace: &Path,
    ) -> Result<PathBuf> {
        let config_content = format!(
            r#"
[general]
workspace = "{}"
log_level = "debug"

[ai]
provider = "openai"
model = "gpt-4.1"
api_key = "test_key"

[sync]
correlation_threshold = 0.8
enable_dialogue_detection = true

[parallel]
max_workers = 4

[test]
isolated = true
parallel_safe = true
"#,
            workspace.display()
        );

        self.create_test_file(dir, config_name, &config_content)
            .await
    }

    /// Set up isolated configuration environment
    async fn setup_isolated_environment(&self, path: &Path, test_name: &str) -> Result<()> {
        // Create test-specific configuration to ensure no state sharing
        let config_content = format!(
            r#"
[general]
workspace = "{}"
log_level = "debug"

[test]
name = "{}"
isolated = true
parallel_safe = true
timestamp = "{}"
"#,
            path.display(),
            test_name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        fs::write(path.join("isolated_config.toml"), config_content)
            .await
            .map_err(|e| {
                SubXError::FileOperationFailed(format!("Failed to create isolated config: {}", e))
            })?;

        Ok(())
    }

    /// Get the count of all temporary directories
    pub fn temp_dir_count(&self) -> usize {
        self.temp_dirs.len()
    }

    /// Check if cleanup is enabled
    pub fn is_cleanup_enabled(&self) -> bool {
        self.cleanup_on_drop
    }

    /// Check if isolation is enabled
    pub fn is_isolation_enabled(&self) -> bool {
        self.isolation_enabled
    }
}

impl Drop for TestFileManager {
    fn drop(&mut self) {
        if !self.cleanup_on_drop {
            for temp_dir in &self.temp_dirs {
                println!("Preserving test directory: {:?}", temp_dir.path());
            }
        }
        // TempDir will automatically clean up unless set to preserve
    }
}

impl Default for TestFileManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_manager_creation() {
        let manager = TestFileManager::new();
        assert!(manager.is_cleanup_enabled());
        assert!(manager.is_isolation_enabled());
        assert_eq!(manager.temp_dir_count(), 0);
    }

    #[tokio::test]
    async fn test_create_isolated_directory() {
        let mut manager = TestFileManager::new();
        let dir = manager
            .create_isolated_test_directory("test")
            .await
            .unwrap();

        assert!(dir.exists());
        assert!(dir.join("isolated_config.toml").exists());
        assert_eq!(manager.temp_dir_count(), 1);
    }

    #[tokio::test]
    async fn test_create_test_file() {
        let mut manager = TestFileManager::new();
        let dir = manager
            .create_isolated_test_directory("test")
            .await
            .unwrap();
        let dir_path = dir.to_path_buf(); // Clone the path to avoid borrow conflicts

        let file_path = manager
            .create_test_file(&dir_path, "test.txt", "test content")
            .await
            .unwrap();

        assert!(file_path.exists());
        assert_eq!(
            fs::read_to_string(&file_path).await.unwrap(),
            "test content"
        );
    }

    #[tokio::test]
    async fn test_create_test_config() {
        let mut manager = TestFileManager::new();
        let dir = manager
            .create_isolated_test_directory("test")
            .await
            .unwrap();
        let dir_path = dir.to_path_buf(); // Clone the path to avoid borrow conflicts

        let config_path = manager
            .create_test_config(&dir_path, "test_config.toml", &dir_path)
            .await
            .unwrap();

        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).await.unwrap();
        assert!(content.contains("[general]"));
        assert!(content.contains("[test]"));
    }

    #[tokio::test]
    async fn test_preserve_on_failure() {
        let manager = TestFileManager::new().preserve_on_failure();
        assert!(!manager.is_cleanup_enabled());
    }

    #[tokio::test]
    async fn test_disable_isolation() {
        let manager = TestFileManager::new().disable_parallel_isolation();
        assert!(!manager.is_isolation_enabled());
    }
}
