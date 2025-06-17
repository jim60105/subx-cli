//! SubX: Intelligent Subtitle Processing Library
//!
//! SubX is a comprehensive Rust library for intelligent subtitle file processing,
//! featuring AI-powered matching, format conversion, audio synchronization,
//! and advanced encoding detection capabilities.
//!
//! # Key Features
//!
//! - **AI-Powered Matching**: Intelligent subtitle file matching and renaming
//! - **Format Conversion**: Support for multiple subtitle formats (SRT, ASS, VTT, etc.)
//! - **Audio Synchronization**: Advanced audio-subtitle timing adjustment
//! - **Encoding Detection**: Automatic character encoding detection and conversion
//! - **Parallel Processing**: High-performance batch operations
//! - **Configuration Management**: Flexible multi-source configuration system
//!
//! # Architecture Overview
//!
//! The library is organized into several key modules:
//!
//! - [`cli`] - Command-line interface and argument parsing
//! - [`commands`] - Implementation of all SubX commands
//! - [`config`] - Configuration management and validation
//! - [`core`] - Core processing engines (formats, matching, sync)
//! - [`error`] - Comprehensive error handling system
//! - [`services`] - External service integrations (AI, audio processing)
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use subx_cli::config::{TestConfigService, ConfigService};
//!
//! // Create a configuration service
//! let config_service = TestConfigService::with_defaults();
//! let config = config_service.config();
//!
//! // Use the configuration for processing...
//! ```
//!
//! # Error Handling
//!
//! All operations return a [`Result<T>`] type that wraps [`error::SubXError`]:
//!
//! ```rust
//! use subx_cli::{Result, error::SubXError};
//!
//! fn example_operation() -> Result<String> {
//!     // This could fail with various error types
//!     Err(SubXError::config("Missing configuration"))
//! }
//! ```
//!
//! # Configuration
//!
//! SubX supports dependency injection-based configuration:
//!
//! ```rust,no_run
//! use subx_cli::config::{TestConfigService, Config};
//!
//! // Create configuration service with AI settings
//! let config_service = TestConfigService::with_ai_settings("openai", "gpt-4.1");
//! let config = config_service.config();
//!
//! // Access configuration values
//! println!("AI Provider: {}", config.ai.provider);
//! println!("AI Model: {}", config.ai.model);
//! ```
//!
//! # Performance Considerations
//!
//! - Use [`core::parallel`] for batch operations on large file sets
//! - Configure appropriate cache settings for repeated operations
//! - Consider memory usage when processing large audio files
//!
//! # Thread Safety
//!
//! The library is designed to be thread-safe where appropriate:
//! - Configuration manager uses `Arc<RwLock<T>>` for shared state
//! - File operations include rollback capabilities for atomicity
//! - Parallel processing uses safe concurrency patterns
//!
//! # Feature Flags
//!
//! SubX supports several optional features:
//! ```text
//! - ai - AI service integrations (default)
//! - audio - Audio processing capabilities (default)  
//! - parallel - Parallel processing support (default)
//! ```
#![allow(
    clippy::new_without_default,
    clippy::manual_clamp,
    clippy::useless_vec,
    clippy::items_after_test_module,
    clippy::needless_borrow
)]

/// Library version string.
///
/// This constant provides the current version of the SubX library,
/// automatically populated from `Cargo.toml` at compile time.
///
/// # Examples
///
/// ```rust
/// use subx_cli::VERSION;
///
/// println!("SubX version: {}", VERSION);
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod cli;
pub mod commands;
pub mod config;
pub use config::Config;
// Re-export new configuration service system
pub use config::{
    ConfigService, EnvironmentProvider, ProductionConfigService, SystemEnvironmentProvider,
    TestConfigBuilder, TestConfigService, TestEnvironmentProvider,
};
pub mod core;
pub mod error;
/// Convenient type alias for `Result<T, SubXError>`.
///
/// This type alias simplifies error handling throughout the SubX library
/// by providing a default error type for all fallible operations.
pub type Result<T> = error::SubXResult<T>;

pub mod services;

/// Main application structure with dependency injection support.
///
/// The `App` struct provides a programmatic interface to SubX functionality,
/// designed for embedding SubX in other Rust applications or for advanced
/// use cases requiring fine-grained control over configuration and execution.
///
/// # Use Cases
///
/// - **Embedding**: Use SubX as a library component in larger applications
/// - **Testing**: Programmatic testing of SubX functionality with custom configurations
/// - **Automation**: Scripted execution of SubX operations without shell commands
/// - **Custom Workflows**: Building complex workflows that combine multiple SubX operations
///
/// # vs CLI Interface
///
/// | Feature | CLI (`subx` command) | App (Library API) |
/// |---------|---------------------|-------------------|
/// | Usage | Command line tool | Embedded in Rust code |
/// | Config | Files + Environment | Programmatic injection |
/// | Output | Terminal/stdout | Programmatic control |
/// | Error Handling | Exit codes | Result types |
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust,no_run
/// use subx_cli::{App, config::ProductionConfigService};
/// use std::sync::Arc;
///
/// # async fn example() -> subx_cli::Result<()> {
/// let config_service = Arc::new(ProductionConfigService::new()?);
/// let app = App::new(config_service);
///
/// // Execute operations programmatically
/// app.match_files("/movies", true).await?; // dry run
/// app.convert_files("/subs", "srt", Some("/output")).await?;
/// # Ok(())
/// # }
/// ```
///
/// ## With Custom Configuration
///
/// ```rust,no_run
/// use subx_cli::{App, config::{TestConfigService, Config}};
/// use std::sync::Arc;
///
/// # async fn example() -> subx_cli::Result<()> {
/// let mut config_service = TestConfigService::with_ai_settings("openai", "gpt-4");
///
/// let app = App::new(Arc::new(config_service));
/// app.match_files("/path", false).await?;
/// # Ok(())
/// # }
/// ```
pub struct App {
    config_service: std::sync::Arc<dyn config::ConfigService>,
}

impl App {
    /// Create a new application instance with the provided configuration service.
    ///
    /// # Arguments
    ///
    /// * `config_service` - The configuration service to use
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use subx_cli::{App, config::TestConfigService};
    /// use std::sync::Arc;
    ///
    /// let config_service = Arc::new(TestConfigService::with_defaults());
    /// let app = App::new(config_service);
    /// ```
    pub fn new(config_service: std::sync::Arc<dyn config::ConfigService>) -> Self {
        Self { config_service }
    }

    /// Create a new application instance with the production configuration service.
    ///
    /// This is the default way to create an application instance for production use.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use subx_cli::App;
    ///
    /// # async fn example() -> subx_cli::Result<()> {
    /// let app = App::new_with_production_config()?;
    /// // Ready to use with production configuration
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the production configuration service cannot be created.
    pub fn new_with_production_config() -> Result<Self> {
        let config_service = std::sync::Arc::new(config::ProductionConfigService::new()?);
        Ok(Self::new(config_service))
    }

    /// Run the application with command-line argument parsing.
    ///
    /// This method provides a programmatic way to run SubX with CLI-style
    /// arguments, useful for embedding SubX in other Rust applications.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use subx_cli::{App, config::ProductionConfigService};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> subx_cli::Result<()> {
    /// let config_service = Arc::new(ProductionConfigService::new()?);
    /// let app = App::new(config_service);
    ///
    /// // This parses std::env::args() just like the CLI
    /// app.run().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if command execution fails.
    pub async fn run(&self) -> Result<()> {
        let cli = <cli::Cli as clap::Parser>::parse();
        self.handle_command(cli.command).await
    }

    /// Handle a specific command with the current configuration.
    ///
    /// This method allows programmatic execution of specific SubX commands
    /// without parsing command-line arguments.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use subx_cli::{App, cli::{Commands, MatchArgs}, config::TestConfigService};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> subx_cli::Result<()> {
    /// let config_service = Arc::new(TestConfigService::with_defaults());
    /// let app = App::new(config_service);
    ///
    /// let match_args = MatchArgs {
    ///     path: Some("/path/to/files".into()),
    ///     input_paths: vec![],
    ///     dry_run: true,
    ///     confidence: 80,
    ///     recursive: false,
    ///     backup: false,
    ///     copy: false,
    ///     move_files: false,
    /// };
    ///
    /// app.handle_command(Commands::Match(match_args)).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute
    ///
    /// # Errors
    ///
    /// Returns an error if command execution fails.
    pub async fn handle_command(&self, command: cli::Commands) -> Result<()> {
        // Use the centralized dispatcher to eliminate code duplication
        crate::commands::dispatcher::dispatch_command(command, self.config_service.clone()).await
    }

    /// Execute a match operation programmatically.
    ///
    /// This is a convenience method for programmatic usage without
    /// needing to construct the Commands enum manually.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use subx_cli::{App, config::TestConfigService};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> subx_cli::Result<()> {
    /// let config_service = Arc::new(TestConfigService::with_defaults());
    /// let app = App::new(config_service);
    ///
    /// // Match files programmatically
    /// app.match_files("/path/to/files", true).await?; // dry_run = true
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Arguments
    ///
    /// * `input_path` - Path to the directory or file to process
    /// * `dry_run` - Whether to perform a dry run (no actual changes)
    ///
    /// # Errors
    ///
    /// Returns an error if the match operation fails.
    pub async fn match_files(&self, input_path: &str, dry_run: bool) -> Result<()> {
        let args = cli::MatchArgs {
            path: Some(input_path.into()),
            input_paths: vec![],
            dry_run,
            confidence: 80,
            recursive: false,
            backup: false,
            copy: false,
            move_files: false,
        };
        self.handle_command(cli::Commands::Match(args)).await
    }

    /// Convert subtitle files programmatically.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use subx_cli::{App, config::TestConfigService};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> subx_cli::Result<()> {
    /// let config_service = Arc::new(TestConfigService::with_defaults());
    /// let app = App::new(config_service);
    ///
    /// // Convert to SRT format
    /// app.convert_files("/path/to/subtitles", "srt", Some("/output/path")).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Arguments
    ///
    /// * `input_path` - Path to subtitle files to convert
    /// * `output_format` - Target format ("srt", "ass", "vtt", etc.)
    /// * `output_path` - Optional output directory path
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion fails.
    pub async fn convert_files(
        &self,
        input_path: &str,
        output_format: &str,
        output_path: Option<&str>,
    ) -> Result<()> {
        let format = match output_format.to_lowercase().as_str() {
            "srt" => cli::OutputSubtitleFormat::Srt,
            "ass" => cli::OutputSubtitleFormat::Ass,
            "vtt" => cli::OutputSubtitleFormat::Vtt,
            "sub" => cli::OutputSubtitleFormat::Sub,
            _ => {
                return Err(error::SubXError::CommandExecution(format!(
                    "Unsupported output format: {}. Supported formats: srt, ass, vtt, sub",
                    output_format
                )));
            }
        };

        let args = cli::ConvertArgs {
            input: Some(input_path.into()),
            input_paths: vec![],
            recursive: false,
            format: Some(format),
            output: output_path.map(Into::into),
            keep_original: false,
            encoding: "utf-8".to_string(),
        };
        self.handle_command(cli::Commands::Convert(args)).await
    }

    /// Synchronize subtitle files programmatically.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use subx_cli::{App, config::TestConfigService};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> subx_cli::Result<()> {
    /// let config_service = Arc::new(TestConfigService::with_defaults());
    /// let app = App::new(config_service);
    ///
    /// // Synchronize using VAD method
    /// app.sync_files("/path/to/video.mp4", "/path/to/subtitle.srt", "vad").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Arguments
    ///
    /// * `video_path` - Path to video file for audio analysis
    /// * `subtitle_path` - Path to subtitle file to synchronize
    /// * `method` - Synchronization method ("vad", "manual")
    ///
    /// # Errors
    ///
    /// Returns an error if synchronization fails.
    pub async fn sync_files(
        &self,
        video_path: &str,
        subtitle_path: &str,
        method: &str,
    ) -> Result<()> {
        let sync_method = match method.to_lowercase().as_str() {
            "vad" => Some(cli::SyncMethodArg::Vad),
            "manual" => Some(cli::SyncMethodArg::Manual),
            _ => {
                return Err(error::SubXError::CommandExecution(format!(
                    "Unsupported sync method: {}. Supported methods: vad, manual",
                    method
                )));
            }
        };

        let args = cli::SyncArgs {
            video: Some(video_path.into()),
            subtitle: Some(subtitle_path.into()),
            input_paths: vec![],
            recursive: false,
            offset: None,
            method: sync_method,
            window: 30,
            vad_sensitivity: None,
            vad_chunk_size: None,
            output: None,
            verbose: false,
            dry_run: false,
            force: false,
            batch: false,
            #[allow(deprecated)]
            range: None,
            #[allow(deprecated)]
            threshold: None,
        };
        self.handle_command(cli::Commands::Sync(args)).await
    }

    /// Synchronize subtitle files with manual offset.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use subx_cli::{App, config::TestConfigService};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> subx_cli::Result<()> {
    /// let config_service = Arc::new(TestConfigService::with_defaults());
    /// let app = App::new(config_service);
    ///
    /// // Apply +2.5 second offset to subtitles
    /// app.sync_files_with_offset("/path/to/subtitle.srt", 2.5).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Arguments
    ///
    /// * `subtitle_path` - Path to subtitle file to synchronize
    /// * `offset` - Time offset in seconds (positive delays, negative advances)
    ///
    /// # Errors
    ///
    /// Returns an error if synchronization fails.
    pub async fn sync_files_with_offset(&self, subtitle_path: &str, offset: f32) -> Result<()> {
        let args = cli::SyncArgs {
            video: None,
            subtitle: Some(subtitle_path.into()),
            input_paths: vec![],
            recursive: false,
            offset: Some(offset),
            method: None,
            window: 30,
            vad_sensitivity: None,
            vad_chunk_size: None,
            output: None,
            verbose: false,
            dry_run: false,
            force: false,
            batch: false,
            #[allow(deprecated)]
            range: None,
            #[allow(deprecated)]
            threshold: None,
        };
        self.handle_command(cli::Commands::Sync(args)).await
    }

    /// Get a reference to the configuration service.
    ///
    /// This allows access to the configuration service for testing or
    /// advanced use cases.
    pub fn config_service(&self) -> &std::sync::Arc<dyn config::ConfigService> {
        &self.config_service
    }

    /// Get the current configuration.
    ///
    /// This is a convenience method that retrieves the configuration
    /// from the configured service.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading fails.
    pub fn get_config(&self) -> Result<config::Config> {
        self.config_service.get_config()
    }
}

/// Backward compatibility function that has been replaced by the new CLI interface.
///
/// This function has been deprecated. Use `cli::run()` instead.
#[deprecated(since = "0.2.0", note = "Use cli::run() instead")]
pub async fn run_with_legacy_config() -> Result<()> {
    // Use the new CLI interface instead
    cli::run().await
}
