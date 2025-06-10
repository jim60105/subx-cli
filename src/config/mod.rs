//! Configuration management module.
//!
//! This module provides both the traditional global configuration system
//! and the new dependency injection-based configuration service system.

// Include legacy configuration module
mod config_legacy;

// Re-export from config_legacy.rs for backward compatibility
#[allow(deprecated)]
pub use config_legacy::{
    AIConfig, Config, FormatsConfig, GeneralConfig, OverflowStrategy, ParallelConfig, SyncConfig,
    create_config_from_sources, create_config_with_overrides, create_test_config,
    init_config_manager, init_config_manager_new, load_config, load_config_new,
    reset_global_config_manager,
};

// New configuration service system
pub mod builder;
pub mod service;
pub mod test_macros;
pub mod test_service;

// Legacy configuration management modules
pub mod cache;
pub mod manager;
pub mod partial;
pub mod source;
pub mod validator;

// Re-export the new configuration service system
pub use builder::TestConfigBuilder;
pub use service::{ConfigService, ProductionConfigService};
pub use test_service::TestConfigService;

// Test-only exports
#[cfg(test)]
pub use test_macros::*;
