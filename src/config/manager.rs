//! Configuration manager core module.

use std::sync::{Arc, RwLock};

use crate::config::partial::PartialConfig;
use crate::config::source::ConfigSource;

/// Error type for configuration operations.
#[derive(Debug)]
pub enum ConfigError {
    /// I/O error when reading or writing configuration.
    Io(std::io::Error),
    /// Parsing error for configuration content.
    ParseError(String),
    /// Invalid configuration value: (field, message).
    InvalidValue(String, String),
    /// General validation error.
    ValidationError(String),
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::Io(err)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(err) => write!(f, "I/O error: {}", err),
            ConfigError::ParseError(err) => write!(f, "Parse error: {}", err),
            ConfigError::InvalidValue(field, msg) => {
                write!(f, "Invalid value for {}: {}", field, msg)
            }
            ConfigError::ValidationError(err) => write!(f, "Validation error: {}", err),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Manager to load and merge configuration from multiple sources.
pub struct ConfigManager {
    sources: Vec<Box<dyn ConfigSource>>,
    config: Arc<RwLock<PartialConfig>>,
}

impl ConfigManager {
    /// Create a new configuration manager.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            config: Arc::new(RwLock::new(PartialConfig::default())),
        }
    }

    /// Add a configuration source.
    pub fn add_source(mut self, source: Box<dyn ConfigSource>) -> Self {
        self.sources.push(source);
        self
    }

    /// Load configuration by merging all sources in order of priority.
    pub fn load(&self) -> Result<(), ConfigError> {
        let mut merged = PartialConfig::default();
        let mut sources = self.sources.iter().collect::<Vec<_>>();
        sources.sort_by_key(|s| s.priority());
        for source in sources {
            let cfg = source.load()?;
            merged.merge(cfg)?;
        }
        let mut lock = self.config.write().unwrap();
        *lock = merged;
        Ok(())
    }

    /// Get current configuration.
    pub fn config(&self) -> Arc<RwLock<PartialConfig>> {
        Arc::clone(&self.config)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
