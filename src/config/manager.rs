//! Manager to load, merge, and watch configuration from multiple sources.
//!
//! The `ConfigManager` orchestrates loading partial configurations from
//! different [`ConfigSource`]s, merging them according to priority, and
//! notifying subscribers about configuration changes.
//!
//! # Examples
//!
//! ```rust
//! use std::path::PathBuf;
//! use subx_cli::config::manager::{ConfigManager, ConfigChangeEvent};
//! use subx_cli::config::source::FileSource;
//!
//! // Initialize manager with a file-based source
//! let manager = ConfigManager::new()
//!     .add_source(Box::new(FileSource::new(PathBuf::from("config.toml"))));
//!
//! // Load configuration and retrieve current settings
//! manager.load().expect("Failed to load configuration");
//! let cfg = manager.config().read().unwrap();
//! println!("Loaded configuration: {:?}", *cfg);
//!
//! // Watch for changes asynchronously (requires Tokio runtime)
//! let (mut rx, _watcher) = manager.watch().expect("Watcher setup failed");
//! tokio::spawn(async move {
//!     while rx.changed().await.is_ok() {
//!         match *rx.borrow() {
//!             ConfigChangeEvent::Updated => println!("Configuration updated"),
//!             ConfigChangeEvent::Error(ref e) => eprintln!("Error: {}", e),
//!             ConfigChangeEvent::Initial => (),
//!         }
//!     }
//! });
//! ```

use std::io;
use std::sync::{Arc, RwLock};

use log::debug;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::watch;

use crate::config::partial::PartialConfig;
use crate::config::source::ConfigSource;
use std::cmp::Reverse;

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

impl Clone for ConfigError {
    fn clone(&self) -> Self {
        match self {
            ConfigError::Io(err) => ConfigError::Io(io::Error::new(err.kind(), err.to_string())),
            ConfigError::ParseError(s) => ConfigError::ParseError(s.clone()),
            ConfigError::InvalidValue(f, m) => ConfigError::InvalidValue(f.clone(), m.clone()),
            ConfigError::ValidationError(s) => ConfigError::ValidationError(s.clone()),
        }
    }
}

/// Events signaled when configuration changes or errors occur.
#[derive(Debug, Clone)]
pub enum ConfigChangeEvent {
    /// Initial event when watcher starts.
    Initial,
    /// Configuration successfully updated.
    Updated,
    /// Error occurred during configuration load.
    Error(ConfigError),
}

/// Manager to load and merge configuration from multiple sources.
pub struct ConfigManager {
    sources: Vec<Box<dyn ConfigSource>>,
    config: Arc<RwLock<PartialConfig>>,
    change_notifier: watch::Sender<ConfigChangeEvent>,
}

impl ConfigManager {
    /// Create a new configuration manager.
    pub fn new() -> Self {
        let (tx, _rx) = watch::channel(ConfigChangeEvent::Initial);
        Self {
            sources: Vec::new(),
            config: Arc::new(RwLock::new(PartialConfig::default())),
            change_notifier: tx,
        }
    }

    /// Add a configuration source.
    pub fn add_source(mut self, source: Box<dyn ConfigSource>) -> Self {
        self.sources.push(source);
        self
    }

    /// Load configuration by merging all sources in order of priority.
    pub fn load(&self) -> Result<(), ConfigError> {
        debug!("ConfigManager: Starting to load configuration");
        let result: Result<(), ConfigError> = (|| {
            let mut merged = PartialConfig::default();
            let mut sources = self.sources.iter().collect::<Vec<_>>();
            // 按優先順序由低到高合併：先載入優先權低的來源，再讓優先權高的來源覆蓋
            sources.sort_by_key(|s| Reverse(s.priority()));

            debug!("ConfigManager: Loading {} sources in order", sources.len());
            for (i, source) in sources.iter().enumerate() {
                debug!(
                    "ConfigManager: Loading source {} - '{}' (priority {})",
                    i + 1,
                    source.source_name(),
                    source.priority()
                );
                let cfg = source.load()?;
                debug!(
                    "ConfigManager: Source '{}' returned cfg.ai.max_sample_length = {:?}",
                    source.source_name(),
                    cfg.ai.max_sample_length
                );
                merged.merge(cfg)?;
                debug!(
                    "ConfigManager: After merging '{}', merged.ai.max_sample_length = {:?}",
                    source.source_name(),
                    merged.ai.max_sample_length
                );
            }
            let mut lock = self.config.write().unwrap();
            *lock = merged;
            debug!(
                "ConfigManager: Final stored config.ai.max_sample_length = {:?}",
                lock.ai.max_sample_length
            );
            Ok(())
        })();
        match result {
            Ok(_) => {
                let _ = self.change_notifier.send(ConfigChangeEvent::Updated);
                Ok(())
            }
            Err(err) => {
                let _ = self
                    .change_notifier
                    .send(ConfigChangeEvent::Error(err.clone()));
                Err(err)
            }
        }
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

impl ConfigManager {
    /// Subscribe to configuration change events.
    pub fn subscribe_changes(&self) -> watch::Receiver<ConfigChangeEvent> {
        self.change_notifier.subscribe()
    }

    /// Watch file-based configuration sources for changes and auto-reload.
    /// Returns a receiver for change events and a watcher handle to keep alive.
    pub fn watch(self) -> notify::Result<(watch::Receiver<ConfigChangeEvent>, RecommendedWatcher)> {
        // Wrap manager in Arc to share with watcher closure
        let this = Arc::new(self);
        let tx = this.change_notifier.clone();
        let rx = this.change_notifier.subscribe();
        let this_clone = Arc::clone(&this);
        let tx_clone = tx.clone();
        let mut watcher: RecommendedWatcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| match res {
                Ok(event) => {
                    if matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                    ) {
                        if let Err(e) = this_clone.load() {
                            let _ = tx_clone.send(ConfigChangeEvent::Error(e));
                        }
                    }
                }
                Err(err) => {
                    let _ = tx_clone.send(ConfigChangeEvent::Error(ConfigError::Io(
                        io::Error::other(err.to_string()),
                    )));
                }
            },
            notify::Config::default(),
        )?;
        for source in &this.sources {
            for path in source.watch_paths() {
                watcher.watch(&path, RecursiveMode::NonRecursive)?;
            }
        }
        // initial load trigger
        if let Err(e) = this.load() {
            let _ = tx.send(ConfigChangeEvent::Error(e));
        }
        Ok((rx, watcher))
    }
}
