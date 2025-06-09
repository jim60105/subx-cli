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
//!
//! # Reset to defaults
//! subx config reset
//! ```

// src/cli/config_args.rs
use clap::{Args, Subcommand};

/// Command-line arguments for configuration management operations.
///
/// The config command provides comprehensive tools for managing SubX
/// configuration settings. It supports viewing, modifying, and resetting
/// configuration values across all configuration categories and sources.
///
/// # Configuration Management Features
///
/// - **Hierarchical Settings**: Manage settings across multiple configuration levels
/// - **Type Safety**: Automatic validation of configuration values
/// - **Interactive Editing**: User-friendly prompts for complex settings
/// - **Backup/Restore**: Automatic backup before destructive operations
/// - **Import/Export**: Share configuration profiles between systems
///
/// # Configuration Scope
///
/// Configuration changes can affect:
/// - **User Settings**: Personal preferences and credentials
/// - **Project Settings**: Local project-specific overrides
/// - **Runtime Behavior**: How the application processes files
/// - **Performance**: Processing speed and resource usage
/// - **Integration**: External service connections and API settings
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::{ConfigArgs, ConfigAction};
///
/// // Set a configuration value
/// let set_args = ConfigArgs {
///     action: ConfigAction::Set {
///         key: "ai.provider".to_string(),
///         value: "openai".to_string(),
///     },
/// };
///
/// // Get a configuration value
/// let get_args = ConfigArgs {
///     action: ConfigAction::Get {
///         key: "ai.provider".to_string(),
///     },
/// };
/// ```
#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// The configuration management action to perform.
    ///
    /// Specifies which configuration operation should be executed.
    /// Each action provides different capabilities for viewing and
    /// modifying configuration settings with appropriate validation
    /// and error handling.
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
    /// Set a configuration value with validation and type checking.
    ///
    /// Updates a specific configuration setting with the provided value.
    /// The operation includes comprehensive validation to ensure the new
    /// value is compatible with the setting's type and constraints.
    ///
    /// # Key Format
    ///
    /// Configuration keys use dot notation to navigate the hierarchy:
    /// - `ai.provider` - AI service provider selection
    /// - `ai.openai.api_key` - OpenAI API key
    /// - `general.enable_progress_bar` - Progress bar display preference
    /// - `audio.max_offset_seconds` - Maximum sync offset range
    ///
    /// # Value Types and Examples
    ///
    /// ```bash
    /// # String values
    /// subx config set ai.provider "openai"
    /// subx config set ai.openai.api_key "sk-..."
    ///
    /// # Boolean values
    /// subx config set general.enable_progress_bar true
    /// subx config set ai.enable_cache false
    ///
    /// # Numeric values
    /// subx config set audio.max_offset_seconds 30.0
    /// subx config set ai.max_sample_length 2000
    ///
    /// # Array values (JSON format)
    /// subx config set ai.supported_models '["gpt-4", "gpt-3.5-turbo"]'
    /// ```
    ///
    /// # Validation Rules
    ///
    /// - **API Keys**: Must match provider-specific format requirements
    /// - **URLs**: Must be valid HTTP/HTTPS endpoints
    /// - **File Paths**: Must be accessible and have appropriate permissions
    /// - **Numeric Ranges**: Must fall within acceptable min/max values
    /// - **Enum Values**: Must match predefined valid options
    ///
    /// # Error Handling
    ///
    /// Common validation errors and solutions:
    /// - **Invalid key**: Check key spelling and available options
    /// - **Type mismatch**: Verify value format matches expected type
    /// - **Range error**: Ensure numeric values are within valid range
    /// - **Permission error**: Check file/directory access permissions
    Set {
        /// Configuration key in dot notation (e.g., "ai.provider", "general.timeout").
        ///
        /// Specifies the configuration setting to modify using hierarchical
        /// dot notation. The key must correspond to a valid configuration
        /// setting as defined in the application's configuration schema.
        ///
        /// # Key Categories
        ///
        /// - **ai.***: AI service configuration and credentials
        /// - **audio.***: Audio processing and synchronization settings
        /// - **general.***: Basic application behavior and preferences
        /// - **cache.***: Caching behavior and storage settings
        /// - **format.***: Default output formats and encoding options
        ///
        /// # Examples
        /// ```bash
        /// subx config set ai.provider openai
        /// subx config set general.enable_progress_bar false
        /// subx config set audio.correlation_threshold 0.8
        /// ```
        key: String,

        /// New value for the configuration setting.
        ///
        /// The value to assign to the specified configuration key. The value
        /// will be validated against the setting's type and constraints before
        /// being applied. String values containing spaces should be quoted.
        ///
        /// # Type Conversion
        ///
        /// Values are automatically converted to the appropriate type:
        /// - **Strings**: Used as-is or with quotes for spaces
        /// - **Booleans**: "true", "false", "1", "0", "yes", "no"
        /// - **Numbers**: Integer or floating-point notation
        /// - **Arrays**: JSON array format for complex values
        ///
        /// # Special Values
        ///
        /// - **Empty string**: `""` to clear string settings
        /// - **Null/None**: `null` to unset optional settings
        /// - **Environment variables**: `${VAR_NAME}` for dynamic values
        ///
        /// # Examples
        /// ```bash
        /// subx config set ai.openai.api_key "sk-1234567890abcdef"
        /// subx config set general.timeout 30
        /// subx config set audio.enabled true
        /// subx config set cache.max_size_mb 512
        /// ```
        value: String,
    },

    /// Retrieve and display a specific configuration value.
    ///
    /// Displays the current value of a configuration setting along with
    /// metadata such as the source of the setting (user config, system
    /// default, environment variable, etc.) and any applicable constraints.
    ///
    /// # Output Format
    ///
    /// The command displays:
    /// - **Current Value**: The effective value being used
    /// - **Source**: Where the value originates (user, system, environment)
    /// - **Default Value**: The built-in default if different from current
    /// - **Type Information**: Expected value type and constraints
    /// - **Description**: Human-readable explanation of the setting
    ///
    /// # Examples
    ///
    /// ```bash
    /// subx config get ai.provider
    /// # Output:
    /// # ai.provider = "openai"
    /// # Source: user config (/home/user/.config/subx/config.toml)
    /// # Default: "openai"
    /// # Type: String (enum: openai, anthropic, local)
    /// # Description: AI service provider for subtitle matching
    ///
    /// subx config get general.enable_progress_bar
    /// # Output:
    /// # general.enable_progress_bar = true
    /// # Source: system default
    /// # Type: Boolean
    /// # Description: Show progress bars during long operations
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Debugging**: Verify current configuration values
    /// - **Documentation**: Understand setting sources and constraints
    /// - **Validation**: Confirm settings are applied correctly
    /// - **Troubleshooting**: Identify configuration-related issues
    Get {
        /// Configuration key to retrieve (e.g., "ai.provider", "general.timeout").
        ///
        /// Specifies which configuration setting to display. The key must
        /// correspond to a valid configuration setting. Use `subx config list`
        /// to see all available configuration keys.
        ///
        /// # Wildcard Support
        ///
        /// Future enhancement will support wildcard patterns:
        /// - `ai.*` - All AI-related settings
        /// - `*.timeout` - All timeout settings
        /// - `general.*` - All general application settings
        ///
        /// # Examples
        /// ```bash
        /// subx config get ai.provider
        /// subx config get general.enable_progress_bar
        /// subx config get audio.correlation_threshold
        /// ```
        key: String,
    },

    /// List all configuration settings with their current values.
    ///
    /// Displays a comprehensive overview of all configuration settings
    /// organized by category. This provides a complete view of the current
    /// configuration state and helps identify settings that may need adjustment.
    ///
    /// # Output Organization
    ///
    /// Settings are grouped by category:
    /// - **General**: Basic application behavior
    /// - **AI**: AI service configuration and parameters
    /// - **Audio**: Audio processing and synchronization
    /// - **Cache**: Caching behavior and storage
    /// - **Format**: Output format and encoding preferences
    ///
    /// # Information Displayed
    ///
    /// For each setting:
    /// - **Key**: Full configuration key path
    /// - **Value**: Current effective value
    /// - **Source**: Configuration source (user/system/env/default)
    /// - **Type**: Data type and constraints
    /// - **Status**: Modified/default indicator
    ///
    /// # Filtering Options
    ///
    /// Future enhancements will include:
    /// - Category filtering (`--category ai`)
    /// - Modified-only view (`--modified-only`)
    /// - Source filtering (`--source user`)
    /// - Output format options (`--format json`)
    ///
    /// # Examples
    ///
    /// ```bash
    /// subx config list
    /// # Output:
    /// # [General]
    /// # enable_progress_bar = true (default)
    /// # timeout = 30 (user)
    /// #
    /// # [AI]
    /// # provider = "openai" (user)
    /// # openai.api_key = "sk-***" (env: OPENAI_API_KEY)
    /// # max_sample_length = 2000 (default)
    /// #
    /// # [Audio]
    /// # max_offset_seconds = 30.0 (default)
    /// # correlation_threshold = 0.8 (user)
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Configuration Review**: Audit current settings
    /// - **Troubleshooting**: Identify problematic configurations
    /// - **Documentation**: Generate configuration documentation
    /// - **Migration**: Prepare for configuration transfer
    List,

    /// Reset configuration to default values with backup creation.
    ///
    /// Restores all configuration settings to their built-in default values.
    /// This operation creates a backup of the current configuration before
    /// making changes, allowing for easy recovery if needed.
    ///
    /// # Reset Scope
    ///
    /// The reset operation affects:
    /// - **User Configuration**: Settings in user config directory
    /// - **Project Configuration**: Local project-specific settings
    /// - **Cached Values**: Processed configuration cache
    ///
    /// # Data Preserved
    ///
    /// The following are NOT affected by reset:
    /// - **Environment Variables**: Runtime configuration overrides
    /// - **Command-line Arguments**: Temporary session overrides
    /// - **System Configuration**: Global system-wide settings
    /// - **Application Data**: Caches, logs, and processing results
    ///
    /// # Backup Process
    ///
    /// Before resetting:
    /// 1. **Backup Creation**: Current config saved with timestamp
    /// 2. **Verification**: Ensure backup was created successfully
    /// 3. **Reset Application**: Apply default values
    /// 4. **Validation**: Verify new configuration is valid
    ///
    /// # Backup Location
    ///
    /// Backups are stored in the configuration directory:
    /// ```
    /// ~/.config/subx/backups/config_backup_YYYYMMDD_HHMMSS.toml
    /// ```
    ///
    /// # Recovery
    ///
    /// To restore from backup:
    /// ```bash
    /// # Copy backup back to main config
    /// cp ~/.config/subx/backups/config_backup_20240101_120000.toml \
    ///    ~/.config/subx/config.toml
    /// ```
    ///
    /// # Confirmation
    ///
    /// This is a destructive operation that requires user confirmation:
    /// ```bash
    /// subx config reset
    /// # Output:
    /// # ⚠ This will reset all configuration to defaults
    /// # ⚠ Current config will be backed up to: ~/.config/subx/backups/...
    /// # Continue? [y/N]:
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Fresh Start**: Clean slate for new configuration
    /// - **Troubleshooting**: Eliminate configuration-related issues
    /// - **Testing**: Ensure tests run with predictable defaults
    /// - **Migration**: Prepare for major version upgrades
    ///
    /// # Examples
    ///
    /// ```bash
    /// subx config reset
    /// # Interactive confirmation and backup creation
    ///
    /// subx config reset --force
    /// # Skip confirmation (future enhancement)
    ///
    /// subx config reset --dry-run
    /// # Show what would be reset without making changes
    /// ```
    Reset,
}
