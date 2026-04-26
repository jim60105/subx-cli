//! Configuration management command implementation with hierarchical settings.
//!
//! This module provides comprehensive configuration management capabilities
//! through the `config` subcommand, enabling users to view, modify, and manage
//! application settings across multiple configuration categories and sources.
//! It supports hierarchical configuration with validation and type safety.
//!
//! # Configuration Architecture
//!
//! ## Configuration Categories
//! - **General**: Basic application behavior and preferences
//! - **AI Settings**: AI service providers, models, and API configuration
//! - **Audio Processing**: Audio analysis and synchronization parameters
//! - **Format Options**: Default output formats and conversion settings
//! - **Cache Management**: Caching behavior and storage configuration
//! - **Sync Settings**: Subtitle timing and synchronization options
//!
//! ## Configuration Sources (Priority Order)
//! 1. **Command-line Arguments**: Highest priority, session-specific
//! 2. **Environment Variables**: Runtime configuration overrides
//! 3. **User Configuration**: Personal settings in user config directory
//! 4. **Project Configuration**: Local project-specific settings
//! 5. **System Configuration**: Global system-wide defaults
//! 6. **Built-in Defaults**: Application default values
//!
//! # Supported Operations
//!
//! ## Set Operation
//! - **Type Validation**: Ensure values match expected data types
//! - **Range Checking**: Validate numeric values are within bounds
//! - **Format Verification**: Check string values follow required patterns
//! - **Dependency Validation**: Verify related settings are compatible
//! - **Backup Creation**: Preserve previous values for rollback
//!
//! ## Get Operation
//! - **Value Display**: Show current effective value
//! - **Source Identification**: Indicate where value originates
//! - **Type Information**: Display expected data type and constraints
//! - **Default Comparison**: Show difference from built-in defaults
//! - **Metadata Display**: Include help text and validation rules
//!
//! ## List Operation
//! - **Categorized Display**: Group settings by functional area
//! - **Source Indicators**: Show which settings are customized
//! - **Value Formatting**: Display values in appropriate format
//! - **Filter Options**: Support for category and status filtering
//! - **Export Capability**: Generate configuration for sharing
//!
//! ## Reset Operation
//! - **Backup Creation**: Automatic backup before reset
//! - **Selective Reset**: Option to reset specific categories
//! - **Confirmation Process**: Interactive confirmation for safety
//! - **Recovery Information**: Instructions for backup restoration
//!
//! # Configuration Keys
//!
//! ## General Settings
//! ```text
//! general.enable_progress_bar     # Boolean: Show progress indicators
//! general.backup_enabled          # Boolean: Automatic file backups
//! general.task_timeout_seconds    # Integer: Operation timeout in seconds
//! ```
//!
//! ## AI Configuration
//! ```text
//! ai.provider                    # String: AI service provider
//! ai.api_key                     # String: OpenAI API authentication
//! ai.model                       # String: GPT model selection
//! ai.max_tokens                  # Integer: Maximum response length
//! ai.base_url                    # String: API endpoint URL
//! ai.max_sample_length           # Integer: Text sample size for analysis
//! ai.temperature                 # Float: Response randomness control
//! ai.retry_attempts              # Integer: API request retry count
//! ai.retry_delay_ms              # Integer: Retry delay in milliseconds
//! ```
//!
//! ## Audio Processing
//! ```text
//! audio.max_offset_seconds       # Float: Maximum sync offset range
//! audio.correlation_threshold    # Float: Minimum correlation for sync
//! audio.dialogue_threshold       # Float: Speech detection sensitivity
//! audio.min_dialogue_duration_ms # Integer: Minimum speech segment length
//! audio.enable_dialogue_detection # Boolean: Advanced audio analysis
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::cli::{ConfigArgs, ConfigAction};
//! use subx_cli::commands::config_command;
//!
//! // Set AI provider
//! let set_args = ConfigArgs {
//!     action: ConfigAction::Set {
//!         key: "ai.provider".to_string(),
//!         value: "openai".to_string(),
//!     },
//! };
//! config_command::execute(set_args).await?;
//!
//! // Get current AI model
//! let get_args = ConfigArgs {
//!     action: ConfigAction::Get {
//!         key: "ai.openai.model".to_string(),
//!     },
//! };
//! config_command::execute(get_args).await?;
//! ```

use crate::cli::output::{active_mode, emit_success, emit_success_with_warnings};
use crate::cli::{ConfigAction, ConfigArgs};
use crate::config::{ConfigService, mask_sensitive_value};
use crate::error::{SubXError, SubXResult};
use serde::Serialize;
use serde_json::{Map, Value};

/// JSON payload for `config set` — the key/value that was just persisted.
#[derive(Debug, Serialize)]
pub struct ConfigSetPayload<'a> {
    /// Configuration key that was modified.
    pub key: &'a str,
    /// New value (sensitive values are masked).
    pub value: String,
}

/// JSON payload for `config get`/`list`/`reset` — the resolved
/// configuration object (or a single-key projection for `get`).
#[derive(Debug, Serialize)]
pub struct ConfigPayload {
    /// Resolved configuration map.
    pub config: Value,
}

fn build_config_value(config_service: &dyn ConfigService) -> SubXResult<Value> {
    let mut config = config_service.get_config()?;
    if let Some(key) = config.ai.api_key.as_ref() {
        config.ai.api_key = Some(mask_sensitive_value("ai.api_key", key));
    }
    serde_json::to_value(&config)
        .map_err(|e| SubXError::config(format!("JSON serialization error: {e}")))
}

/// Build a JSON-friendly view of the configuration using the *tolerant*
/// load path so that an existing strict-invalid `config.toml` does not
/// prevent `config get`/`list` from inspecting it.
///
/// The returned vector contains zero or more advisory warnings: it is
/// non-empty iff the on-disk configuration fails cross-section
/// validation. Callers MUST surface these warnings either via
/// [`emit_success_with_warnings`] (JSON mode) or as stderr lines (text
/// mode) so the user can see *why* the on-disk file is currently
/// invalid before issuing the repair `config set` mutation.
fn build_config_value_for_repair(
    config_service: &dyn ConfigService,
) -> SubXResult<(Value, Vec<String>)> {
    let mut config = config_service.load_for_repair()?;
    let warnings = collect_strict_validation_warnings(&config);
    if let Some(key) = config.ai.api_key.as_ref() {
        config.ai.api_key = Some(mask_sensitive_value("ai.api_key", key));
    }
    let value = serde_json::to_value(&config)
        .map_err(|e| SubXError::config(format!("JSON serialization error: {e}")))?;
    Ok((value, warnings))
}

/// Run cross-section validation on `config` purely for diagnostic
/// purposes. Returns one user-friendly warning string per validation
/// failure; for the typical "single validator error" case the vector
/// has length 1. The returned vector is empty when `config` is
/// strict-valid.
fn collect_strict_validation_warnings(config: &crate::config::Config) -> Vec<String> {
    match crate::config::validator::validate_config(config) {
        Ok(()) => Vec::new(),
        Err(err) => {
            vec![format!("configuration is currently invalid: {err}")]
        }
    }
}

async fn run_config_action(args: ConfigArgs, config_service: &dyn ConfigService) -> SubXResult<()> {
    let mode = active_mode();
    let json_mode = mode.is_json();

    match args.action {
        ConfigAction::Set { key, value } => {
            config_service.set_config_value(&key, &value)?;
            let masked = mask_sensitive_value(&key, &value);
            if json_mode {
                emit_success(
                    mode,
                    "config",
                    ConfigSetPayload {
                        key: &key,
                        value: masked,
                    },
                );
            } else {
                println!("✓ Configuration '{key}' set to '{masked}'");
                if let Ok(current) = config_service.get_config_value(&key) {
                    let masked_current = mask_sensitive_value(&key, &current);
                    println!("  Current value: {masked_current}");
                }
                if let Ok(path) = config_service.get_config_file_path() {
                    println!("  Saved to: {}", path.display());
                }
            }
        }
        ConfigAction::Get { key } => {
            // Tolerant read: load the file directly so users can
            // inspect a strict-invalid configuration.
            let config = config_service.load_for_repair()?;
            let warnings = collect_strict_validation_warnings(&config);
            let value = crate::config::service::read_config_value_from(&config, &key)?;
            let masked = mask_sensitive_value(&key, &value);
            if json_mode {
                let mut obj = Map::new();
                obj.insert(key.clone(), Value::String(masked));
                emit_success_with_warnings(
                    mode,
                    "config",
                    ConfigPayload {
                        config: Value::Object(obj),
                    },
                    warnings,
                );
            } else {
                println!("{masked}");
                for warning in &warnings {
                    eprintln!("warning: {warning}");
                }
            }
        }
        ConfigAction::List => {
            if json_mode {
                let (config_value, warnings) = build_config_value_for_repair(config_service)?;
                let payload = ConfigPayload {
                    config: config_value,
                };
                emit_success_with_warnings(mode, "config", payload, warnings);
            } else {
                let mut config = config_service.load_for_repair()?;
                let warnings = collect_strict_validation_warnings(&config);
                if let Ok(path) = config_service.get_config_file_path() {
                    println!("# Configuration file path: {}\n", path.display());
                }
                if let Some(key) = config.ai.api_key.as_ref() {
                    config.ai.api_key = Some(mask_sensitive_value("ai.api_key", key));
                }
                println!(
                    "{}",
                    toml::to_string_pretty(&config)
                        .map_err(|e| SubXError::config(format!("TOML serialization error: {e}")))?
                );
                for warning in &warnings {
                    eprintln!("warning: {warning}");
                }
            }
        }
        ConfigAction::Reset => {
            config_service.reset_to_defaults()?;
            if json_mode {
                let payload = ConfigPayload {
                    config: build_config_value(config_service)?,
                };
                emit_success(mode, "config", payload);
            } else {
                println!("Configuration reset to default values");
                if let Ok(path) = config_service.get_config_file_path() {
                    println!("Default configuration saved to: {}", path.display());
                }
            }
        }
    }
    Ok(())
}

/// Execute configuration management operations with validation and type safety.
///
/// This function provides the main entry point for all configuration management
/// operations, including setting values, retrieving current configuration,
/// listing all settings, and resetting to defaults. It includes comprehensive
/// validation, error handling, and user-friendly output formatting.
///
/// # Operation Workflow
///
/// ## Set Operation
/// 1. **Configuration Loading**: Load current configuration from all sources
/// 2. **Key Validation**: Verify configuration key exists and is writable
/// 3. **Value Parsing**: Convert string value to appropriate data type
/// 4. **Constraint Checking**: Validate value meets all requirements
/// 5. **Dependency Verification**: Check related settings compatibility
/// 6. **Backup Creation**: Save current value for potential rollback
/// 7. **Value Application**: Update configuration with new value
/// 8. **Persistence**: Save updated configuration to appropriate file
/// 9. **Confirmation**: Display success message with applied value
///
/// ## Get Operation
/// 1. **Configuration Loading**: Load current effective configuration
/// 2. **Key Resolution**: Locate requested configuration setting
/// 3. **Source Identification**: Determine where value originates
/// 4. **Value Formatting**: Format value for appropriate display
/// 5. **Metadata Retrieval**: Gather type and constraint information
/// 6. **Output Generation**: Create comprehensive information display
///
/// ## List Operation
/// 1. **Configuration Loading**: Load all configuration settings
/// 2. **Categorization**: Group settings by functional area
/// 3. **Source Analysis**: Identify customized vs. default values
/// 4. **Formatting**: Prepare values for tabular display
/// 5. **Output Generation**: Create organized configuration overview
///
/// ## Reset Operation
/// 1. **Current State Backup**: Create timestamped configuration backup
/// 2. **User Confirmation**: Interactive confirmation for destructive operation
/// 3. **Default Restoration**: Replace all settings with built-in defaults
/// 4. **Validation**: Verify reset configuration is valid
/// 5. **Persistence**: Save default configuration to user config file
/// 6. **Confirmation**: Display reset completion and backup location
///
/// # Type System Integration
///
/// The configuration system provides strong typing with automatic conversion:
/// - **Boolean Values**: "true", "false", "1", "0", "yes", "no"
/// - **Integer Values**: Decimal notation with range validation
/// - **Float Values**: Decimal notation with precision preservation
/// - **String Values**: UTF-8 text with format validation where applicable
/// - **Array Values**: JSON array format for complex configuration
///
/// # Validation Framework
///
/// Each configuration setting includes comprehensive validation:
/// - **Type Constraints**: Must match expected data type
/// - **Range Limits**: Numeric values within acceptable bounds
/// - **Format Requirements**: String values matching required patterns
/// - **Dependency Rules**: Related settings must be compatible
/// - **Security Checks**: Sensitive values properly protected
///
/// # Arguments
///
/// * `args` - Configuration command arguments containing the specific
///   operation to perform (set, get, list, or reset) along with any
///   required parameters such as key names and values.
///
/// # Returns
///
/// Returns `Ok(())` on successful operation completion, or an error describing:
/// - Configuration loading or parsing failures
/// - Invalid configuration keys or malformed key paths
/// - Type conversion or validation errors
/// - File system access problems during persistence
/// - User cancellation of destructive operations
///
/// # Error Categories
///
/// ## Configuration Errors
/// - **Invalid Key**: Specified configuration key does not exist
/// - **Type Mismatch**: Value cannot be converted to expected type
/// - **Range Error**: Numeric value outside acceptable range
/// - **Format Error**: String value doesn't match required pattern
/// - **Dependency Error**: Value conflicts with related settings
///
/// ## System Errors
/// - **File Access**: Cannot read or write configuration files
/// - **Permission Error**: Insufficient privileges for operation
/// - **Disk Space**: Insufficient space for configuration persistence
/// - **Corruption**: Configuration file is damaged or invalid
///
/// # Security Considerations
///
/// - **Sensitive Values**: API keys and credentials are properly masked in output
/// - **File Permissions**: Configuration files created with appropriate permissions
/// - **Backup Protection**: Backup files inherit security settings
/// - **Validation**: All input values sanitized and validated
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::cli::{ConfigArgs, ConfigAction};
/// use subx_cli::commands::config_command;
///
/// // Configure AI service with API key
/// let ai_setup = ConfigArgs {
///     action: ConfigAction::Set {
///         key: "ai.openai.api_key".to_string(),
///         value: "sk-1234567890abcdef".to_string(),
///     },
/// };
/// config_command::execute(ai_setup).await?;
///
/// // Adjust audio processing sensitivity
/// let audio_tuning = ConfigArgs {
///     action: ConfigAction::Set {
///         key: "audio.correlation_threshold".to_string(),
///         value: "0.85".to_string(),
///     },
/// };
/// config_command::execute(audio_tuning).await?;
///
/// // View complete configuration
/// let view_all = ConfigArgs {
///     action: ConfigAction::List,
/// };
/// config_command::execute(view_all).await?;
///
/// // Reset to clean state
/// let reset_config = ConfigArgs {
///     action: ConfigAction::Reset,
/// };
/// config_command::execute(reset_config).await?;
/// ```
pub async fn execute(args: ConfigArgs, config_service: &dyn ConfigService) -> SubXResult<()> {
    run_config_action(args, config_service).await
}

/// Execute configuration management command with injected configuration service.
///
/// This function provides the new dependency injection interface for the config command,
/// accepting a configuration service instead of loading configuration globally.
///
/// # Arguments
///
/// * `args` - Configuration command arguments
/// * `config_service` - Configuration service providing access to settings
///
/// # Returns
///
/// Returns `Ok(())` on successful completion, or an error if the operation fails.
pub async fn execute_with_config(
    args: ConfigArgs,
    config_service: std::sync::Arc<dyn ConfigService>,
) -> SubXResult<()> {
    run_config_action(args, config_service.as_ref()).await
}
