//! Configuration sources for loading partial configuration.

use crate::config::manager::ConfigError;
use crate::config::partial::PartialConfig;
use std::path::PathBuf;

/// Trait for configuration source.
pub trait ConfigSource: Send + Sync {
    /// Load partial configuration.
    fn load(&self) -> Result<PartialConfig, ConfigError>;
    /// Priority (lower = higher priority).
    fn priority(&self) -> u8;
    /// Source name for debugging.
    fn source_name(&self) -> &'static str;
    /// File system paths to watch for changes (only file-based sources need override).
    fn watch_paths(&self) -> Vec<PathBuf> {
        Vec::new()
    }
}

/// File-based configuration source.
pub struct FileSource {
    path: PathBuf,
}

impl FileSource {
    /// Create a new file source for the given path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl ConfigSource for FileSource {
    fn load(&self) -> Result<PartialConfig, ConfigError> {
        if !self.path.exists() {
            return Ok(PartialConfig::default());
        }
        let content = std::fs::read_to_string(&self.path)?;
        let cfg = toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))?;
        Ok(cfg)
    }

    fn priority(&self) -> u8 {
        10
    }

    fn source_name(&self) -> &'static str {
        "file"
    }
    fn watch_paths(&self) -> Vec<PathBuf> {
        vec![self.path.clone()]
    }
}

/// Environment variable configuration source.
pub struct EnvSource;

impl EnvSource {
    pub fn new() -> Self {
        Self
    }
}

impl ConfigSource for EnvSource {
    fn load(&self) -> Result<PartialConfig, ConfigError> {
        let mut config = PartialConfig::default();
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            config.ai.api_key = Some(api_key);
        }
        if let Ok(model) = std::env::var("SUBX_AI_MODEL") {
            config.ai.model = Some(model);
        }
        if let Ok(provider) = std::env::var("SUBX_AI_PROVIDER") {
            config.ai.provider = Some(provider);
        }
        if let Ok(backup) = std::env::var("SUBX_BACKUP_ENABLED") {
            config.general.backup_enabled = Some(backup.parse().unwrap_or(false));
        }
        Ok(config)
    }

    fn priority(&self) -> u8 {
        5 // 中等優先權，高於檔案但低於 CLI
    }

    fn source_name(&self) -> &'static str {
        "environment"
    }
}

/// Command line arguments configuration source.
pub struct CliSource;

impl CliSource {
    pub fn new() -> Self {
        Self {}
    }
}

impl ConfigSource for CliSource {
    fn load(&self) -> Result<PartialConfig, ConfigError> {
        Ok(PartialConfig::default())
    }

    fn priority(&self) -> u8 {
        1 // 最高優先權
    }

    fn source_name(&self) -> &'static str {
        "cli"
    }
}

impl Default for EnvSource {
    fn default() -> Self {
        EnvSource::new()
    }
}

impl Default for CliSource {
    fn default() -> Self {
        CliSource::new()
    }
}
