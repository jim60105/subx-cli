//! Configuration sources for loading partial configuration.
//!
//! This module defines the [`ConfigSource`] trait and its implementations
//! (`FileSource`, `EnvSource`, `CliSource`) for loading `PartialConfig`
//! from various sources such as files, environment variables, or CLI arguments.
//!
//! # Examples
//!
//! ```rust
//! use std::path::PathBuf;
//! use subx_cli::config::source::{ConfigSource, FileSource, EnvSource, CliSource};
//! use subx_cli::config::manager::ConfigError;
//! use subx_cli::config::partial::PartialConfig;
//!
//! // Load from a TOML configuration file
//! let file_src = FileSource::new(PathBuf::from("config.toml"));
//! let cfg: PartialConfig = file_src.load().expect("Failed to load file config");
//!
//! // Load from environment variables
//! let env_src = EnvSource::new();
//! let cfg = env_src.load().expect("Failed to load env config");
//!
//! // CLI source (placeholder for CLI argument-based config)
//! let cli_src = CliSource::new();
//! let cfg = cli_src.load().expect("Failed to load CLI config");
//! ```

use crate::config::manager::ConfigError;
use crate::config::partial::PartialConfig;
use log::debug;
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
        debug!("FileSource: Attempting to load from path: {:?}", self.path);
        debug!("FileSource: Path exists: {}", self.path.exists());

        if !self.path.exists() {
            debug!("FileSource: Path does not exist, returning default config");
            return Ok(PartialConfig::default());
        }

        let content = std::fs::read_to_string(&self.path).map_err(|e| {
            debug!("FileSource: Failed to read file: {}", e);
            e
        })?;
        debug!("FileSource: Read {} bytes from file", content.len());
        debug!("FileSource: File content:\n{}", content);

        let cfg: PartialConfig = toml::from_str(&content).map_err(|e| {
            debug!("FileSource: TOML parsing failed: {}", e);
            ConfigError::ParseError(e.to_string())
        })?;

        debug!("FileSource: Parsed successfully");
        debug!(
            "FileSource: cfg.ai.max_sample_length = {:?}",
            cfg.ai.max_sample_length
        );
        debug!("FileSource: cfg.ai.model = {:?}", cfg.ai.model);
        debug!("FileSource: cfg.ai.provider = {:?}", cfg.ai.provider);

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
        if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
            config.ai.base_url = Some(base_url);
        }
        if let Ok(backup) = std::env::var("SUBX_BACKUP_ENABLED") {
            config.general.backup_enabled = Some(backup.parse().unwrap_or(false));
        }
        Ok(config)
    }

    fn priority(&self) -> u8 {
        5 // Medium priority, higher than file but lower than CLI
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
        1 // Highest priority
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
