//! Configuration management command-line arguments and operations.
//!
//! This module defines the command-line interface for configuration management
//! in SubX. It provides comprehensive tools for viewing, modifying, and maintaining
//! configuration settings that control various aspects of subtitle processing,
//! AI integration, and application behavior.
//!
//! # Configuration System
//!
//! SubX uses a hierarchical configuration system with multiple sources:
//! - **System-wide**: Global defaults for all users
//! - **User-specific**: Personal settings in user config directory
//! - **Project-specific**: Local settings for specific projects
//! - **Environment variables**: Runtime configuration overrides
//! - **Command-line**: Highest priority, temporary overrides
//!
//! # Configuration Categories
//!
//! - **General**: Basic application behavior and preferences
//! - **AI Settings**: API endpoints, models, and parameters
//! - **Audio Processing**: Audio analysis and synchronization settings
//! - **Format Options**: Default output formats and encoding preferences
//! - **Performance**: Parallel processing and caching configuration
//!
//! # Examples
//!
//! ```bash
//! # View all configuration settings
//! subx config list
//!
//! # Get specific setting
//! subx config get ai.provider
//!
//! # Set AI provider
//! subx config set ai.provider openai
//! subx config set ai.provider openrouter
//!
//! # Reset to defaults
//! subx config reset
//! ```

//! # Advanced Usage Examples
//!
//! ## Setting Complex Values
//! ```bash
//! # Set AI provider with API key
//! subx-cli config set ai.provider openai
//! subx-cli config set ai.provider openrouter
//! subx-cli config set ai.api_key "sk-1234567890abcdef"
//! subx-cli config set ai.api_key "test-openrouter-key"
//! subx-cli config set ai.base_url "https://api.openai.com/v1"
//! subx-cli config set ai.model "#{DEFAULT_FREE_MODEL}"
//!
//! # Configure audio processing and VAD settings
//! subx-cli config set sync.max_offset_seconds 15.0
//! subx-cli config set sync.default_method vad
//! subx-cli config set sync.vad.enabled true
//! subx-cli config set sync.vad.sensitivity 0.8
//!
//! # Set performance options
//! subx-cli config set parallel.max_workers 4
//! subx-cli config set general.max_concurrent_jobs 8
//! ```
//!
//! ## Boolean Value Formats
//! ```bash
//! # All of these set the value to true
//! subx-cli config set general.backup_enabled true
//! subx-cli config set general.backup_enabled 1
//! subx-cli config set general.backup_enabled yes
//! subx-cli config set general.backup_enabled on
//! subx-cli config set general.backup_enabled enabled
//!
//! # All of these set the value to false
//! subx-cli config set general.backup_enabled false
//! subx-cli config set general.backup_enabled 0
//! subx-cli config set general.backup_enabled no
//! subx-cli config set general.backup_enabled off
//! subx-cli config set general.backup_enabled disabled
//! ```
//!
//! ## Clearing Optional Values
//! ```bash
//! # Clear API key (set to None)
//! subx-cli config set ai.api_key ""
//! ```

// src/cli/config_args.rs
use clap::{Args, Subcommand};

/// Command-line arguments for configuration management operations.
#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// The configuration management action to perform
    #[command(subcommand)]
    pub action: ConfigAction,
}

/// Configuration management operations and subcommands.
///
/// Defines the available configuration management operations that can be
/// performed through the SubX CLI. Each operation provides specific
/// functionality for different aspects of configuration management.
///
/// # Operation Categories
///
/// - **Viewing**: Get and list operations for inspecting settings
/// - **Modification**: Set operation for changing configuration values
/// - **Maintenance**: Reset operation for restoring defaults
///
/// # Validation and Safety
///
/// All configuration operations include:
/// - **Type validation**: Ensure values match expected data types
/// - **Range checking**: Validate numeric values are within acceptable ranges
/// - **Format verification**: Check string values follow required patterns
/// - **Dependency checking**: Verify related settings are compatible
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::ConfigAction;
///
/// // Different configuration operations
/// let set_provider = ConfigAction::Set {
///     key: "ai.provider".to_string(),
///     value: "openai".to_string(),
/// };
///
/// let get_provider = ConfigAction::Get {
///     key: "ai.provider".to_string(),
/// };
///
/// let list_all = ConfigAction::List;
/// let reset_config = ConfigAction::Reset;
/// ```
#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Set a configuration value with validation and type checking
    Set {
        /// Configuration key in dot notation (e.g., "ai.provider", "general.timeout")
        key: String,
        /// New value for the configuration setting
        value: String,
    },

    /// Retrieve and display a specific configuration value
    Get {
        /// Configuration key to retrieve (e.g., "ai.provider", "general.timeout")
        key: String,
    },

    /// List all configuration settings with their current values
    List,

    /// Reset configuration to default values with backup creation
    Reset,
}
