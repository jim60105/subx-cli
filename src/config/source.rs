//! Configuration sources for loading partial configuration.

use crate::cli::ConfigArgs;
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
pub struct EnvSource {
    prefix: String,
}

impl EnvSource {
    /// Create a new environment source with a variable prefix.
    pub fn new(prefix: String) -> Self {
        Self { prefix }
    }
}

impl ConfigSource for EnvSource {
    fn load(&self) -> Result<PartialConfig, ConfigError> {
        let mut cfg = PartialConfig::default();
        if let Ok(val) = std::env::var(format!("{}OPENAI_API_KEY", self.prefix)) {
            cfg.ai.api_key = Some(val);
        }
        if let Ok(val) = std::env::var(format!("{}AI_MODEL", self.prefix)) {
            cfg.ai.model = Some(val);
        }
        Ok(cfg)
    }

    fn priority(&self) -> u8 {
        5
    }

    fn source_name(&self) -> &'static str {
        "environment"
    }
}

/// Command-line arguments configuration source.
pub struct ArgsSource {
    args: ConfigArgs,
}

impl ArgsSource {
    /// Create a new args source from parsed CLI arguments.
    pub fn new(args: ConfigArgs) -> Self {
        Self { args }
    }
}

impl ConfigSource for ArgsSource {
    fn load(&self) -> Result<PartialConfig, ConfigError> {
        let cfg = PartialConfig::default();
        // TODO: map ConfigArgs to PartialConfig
        let _ = &self.args;
        Ok(cfg)
    }

    fn priority(&self) -> u8 {
        0
    }

    fn source_name(&self) -> &'static str {
        "command_line"
    }
}
