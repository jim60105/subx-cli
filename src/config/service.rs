//! Configuration service system for dependency injection and test isolation.
//!
//! This module provides a clean abstraction for configuration management
//! that enables dependency injection and complete test isolation without
//! requiring unsafe code or global state resets.

use crate::{Result, config::Config, error::SubXError};
use config::{Config as ConfigCrate, ConfigBuilder, Environment, File, builder::DefaultState};
use log::debug;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Configuration service trait for dependency injection.
///
/// This trait abstracts configuration loading and reloading operations,
/// allowing different implementations for production and testing environments.
pub trait ConfigService: Send + Sync {
    /// Get the current configuration.
    ///
    /// Returns a clone of the current configuration state. This method
    /// may use internal caching for performance.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading or validation fails.
    fn get_config(&self) -> Result<Config>;

    /// Reload configuration from sources.
    ///
    /// Forces a reload of configuration from all configured sources.
    /// This is useful for dynamic configuration updates.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration reloading fails.
    fn reload(&self) -> Result<()>;
}

/// Production configuration service implementation.
///
/// This service loads configuration from multiple sources in order of priority:
/// 1. Environment variables (highest priority)
/// 2. User configuration file
/// 3. Default configuration file (lowest priority)
///
/// Configuration is cached after first load for performance.
pub struct ProductionConfigService {
    config_builder: ConfigBuilder<DefaultState>,
    cached_config: Arc<RwLock<Option<Config>>>,
}

impl ProductionConfigService {
    /// Create a new production configuration service.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration builder cannot be initialized.
    pub fn new() -> Result<Self> {
        let config_builder = ConfigCrate::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::from(Self::user_config_path()).required(false))
            .add_source(Environment::with_prefix("SUBX").separator("_"));

        Ok(Self {
            config_builder,
            cached_config: Arc::new(RwLock::new(None)),
        })
    }

    /// Create a configuration service with custom sources.
    ///
    /// This allows adding additional configuration sources for specific use cases.
    ///
    /// # Arguments
    ///
    /// * `sources` - Additional configuration sources to add
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration builder cannot be updated.
    pub fn with_custom_file(mut self, file_path: PathBuf) -> Result<Self> {
        self.config_builder = self.config_builder.add_source(File::from(file_path));
        Ok(self)
    }

    /// Get the user configuration file path.
    ///
    /// Returns the path to the user's configuration file, which is typically
    /// located in the user's configuration directory.
    fn user_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("subx")
            .join("config.toml")
    }

    /// Load and validate configuration from all sources.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading or validation fails.
    fn load_and_validate(&self) -> Result<Config> {
        debug!("ProductionConfigService: Loading configuration from sources");

        // Build configuration from all sources
        let config_crate = self.config_builder.build_cloned().map_err(|e| {
            debug!("ProductionConfigService: Config build failed: {}", e);
            SubXError::config(format!("Failed to build configuration: {}", e))
        })?;

        // Deserialize to our Config struct
        let app_config: Config = match config_crate.try_deserialize() {
            Ok(config) => config,
            Err(e) => {
                debug!(
                    "ProductionConfigService: Config deserialization failed, using defaults: {}",
                    e
                );
                Config::default()
            }
        };

        // Validate the configuration
        crate::config::validator::validate_config(&app_config).map_err(|e| {
            debug!("ProductionConfigService: Config validation failed: {}", e);
            SubXError::config(format!("Configuration validation failed: {}", e))
        })?;

        debug!("ProductionConfigService: Configuration loaded and validated successfully");
        Ok(app_config)
    }
}

impl ConfigService for ProductionConfigService {
    fn get_config(&self) -> Result<Config> {
        // Check cache first
        {
            let cache = self.cached_config.read().unwrap();
            if let Some(config) = cache.as_ref() {
                debug!("ProductionConfigService: Returning cached configuration");
                return Ok(config.clone());
            }
        }

        // Load configuration
        let app_config = self.load_and_validate()?;

        // Update cache
        {
            let mut cache = self.cached_config.write().unwrap();
            *cache = Some(app_config.clone());
        }

        Ok(app_config)
    }

    fn reload(&self) -> Result<()> {
        debug!("ProductionConfigService: Reloading configuration");

        // Clear cache to force reload
        {
            let mut cache = self.cached_config.write().unwrap();
            *cache = None;
        }

        // Trigger reload by calling get_config
        self.get_config()?;

        debug!("ProductionConfigService: Configuration reloaded successfully");
        Ok(())
    }
}

impl Default for ProductionConfigService {
    fn default() -> Self {
        Self::new().expect("Failed to create default ProductionConfigService")
    }
}
