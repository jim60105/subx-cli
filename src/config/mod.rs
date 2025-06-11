//! Configuration management module.
//!
//! This module provides the new dependency injection-based configuration service system.

// Configuration type definitions
mod config_legacy;

// New configuration service system
pub mod builder;
pub mod service;
pub mod test_macros;
pub mod test_service;

// Configuration validation and utilities
pub mod validator;

// Re-export configuration types
pub use config_legacy::{
    AIConfig, Config, FormatsConfig, GeneralConfig, OverflowStrategy, ParallelConfig, SyncConfig,
};

// Re-export the configuration service system
pub use builder::TestConfigBuilder;
pub use service::{ConfigService, ProductionConfigService};
pub use test_service::TestConfigService;
