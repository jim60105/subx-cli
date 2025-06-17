use crate::{Result, cli::Commands, config::ConfigService};
use std::sync::Arc;

/// Central command dispatcher to avoid code duplication.
///
/// This module provides a unified way to dispatch commands,
/// eliminating duplication between CLI and library API paths.
///
/// # Design Principles
///
/// - **Single Responsibility**: Each command dispatcher handles exactly one command type
/// - **Consistency**: Both CLI and App API use the same command execution logic
/// - **Error Handling**: Unified error handling across all command paths
/// - **Testability**: Easy to test individual command dispatch without full CLI setup
///
/// # Architecture
///
/// The dispatcher acts as a bridge between:
/// - CLI argument parsing (from `clap`)
/// - Command execution logic (in `commands` module)
/// - Configuration dependency injection
///
/// This eliminates the previous duplication where both `cli::run_with_config()`
/// and `App::handle_command()` had identical match statements.
///
/// # Examples
///
/// ```rust
/// use subx_cli::commands::dispatcher::dispatch_command;
/// use subx_cli::cli::{Commands, MatchArgs};
/// use subx_cli::config::TestConfigService;
/// use std::sync::Arc;
///
/// # async fn example() -> subx_cli::Result<()> {
/// let config_service = Arc::new(TestConfigService::with_defaults());
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
/// dispatch_command(Commands::Match(match_args), config_service).await?;
/// # Ok(())
/// # }
/// ```
pub async fn dispatch_command(
    command: Commands,
    config_service: Arc<dyn ConfigService>,
) -> Result<()> {
    match command {
        Commands::Match(args) => {
            crate::commands::match_command::execute_with_config(args, config_service).await
        }
        Commands::Convert(args) => {
            crate::commands::convert_command::execute_with_config(args, config_service).await
        }
        Commands::Sync(args) => {
            crate::commands::sync_command::execute_with_config(args, config_service).await
        }
        Commands::Config(args) => {
            crate::commands::config_command::execute_with_config(args, config_service).await
        }
        Commands::GenerateCompletion(args) => {
            let mut cmd = <crate::cli::Cli as clap::CommandFactory>::command();
            let cmd_name = cmd.get_name().to_string();
            let mut stdout = std::io::stdout();
            clap_complete::generate(args.shell, &mut cmd, cmd_name, &mut stdout);
            Ok(())
        }
        Commands::Cache(args) => {
            crate::commands::cache_command::execute_with_config(args, config_service).await
        }
        Commands::DetectEncoding(args) => {
            crate::commands::detect_encoding_command::detect_encoding_command_with_config(
                args,
                config_service.as_ref(),
            )?;
            Ok(())
        }
    }
}

/// Dispatch command with borrowed config service reference.
///
/// This version is used by the CLI interface where we have a borrowed reference
/// to the configuration service rather than an owned Arc.
pub async fn dispatch_command_with_ref(
    command: Commands,
    config_service: &dyn ConfigService,
) -> Result<()> {
    match command {
        Commands::Match(args) => {
            args.validate()
                .map_err(crate::error::SubXError::CommandExecution)?;
            crate::commands::match_command::execute(args, config_service).await
        }
        Commands::Convert(args) => {
            crate::commands::convert_command::execute(args, config_service).await
        }
        Commands::Sync(args) => crate::commands::sync_command::execute(args, config_service).await,
        Commands::Config(args) => {
            crate::commands::config_command::execute(args, config_service).await
        }
        Commands::GenerateCompletion(args) => {
            let mut cmd = <crate::cli::Cli as clap::CommandFactory>::command();
            let cmd_name = cmd.get_name().to_string();
            let mut stdout = std::io::stdout();
            clap_complete::generate(args.shell, &mut cmd, cmd_name, &mut stdout);
            Ok(())
        }
        Commands::Cache(args) => crate::commands::cache_command::execute(args).await,
        Commands::DetectEncoding(args) => {
            crate::commands::detect_encoding_command::detect_encoding_command_with_config(
                args,
                config_service,
            )?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{ConvertArgs, MatchArgs, OutputSubtitleFormat};
    use crate::config::TestConfigService;

    #[tokio::test]
    async fn test_dispatch_match_command() {
        let config_service = Arc::new(TestConfigService::with_ai_settings(
            "test_provider",
            "test_model",
        ));
        let args = MatchArgs {
            path: Some("/tmp/test".into()),
            input_paths: vec![],
            dry_run: true,
            confidence: 80,
            recursive: false,
            backup: false,
            copy: false,
            move_files: false,
        };

        // Should not panic and should handle the command
        let result = dispatch_command(Commands::Match(args), config_service).await;

        // The actual result depends on the test setup, but it should not panic
        // In a dry run mode, it should generally succeed
        match result {
            Ok(_) => {} // Success case
            Err(e) => {
                // Allow certain expected errors like missing files in test environment
                let error_msg = format!("{:?}", e);
                assert!(
                    error_msg.contains("NotFound")
                        || error_msg.contains("No subtitle files found")
                        || error_msg.contains("No video files found")
                        || error_msg.contains("Config"),
                    "Unexpected error: {:?}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_dispatch_convert_command() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        let args = ConvertArgs {
            input: Some("/tmp/nonexistent".into()),
            input_paths: vec![],
            recursive: false,
            format: Some(OutputSubtitleFormat::Srt),
            output: None,
            keep_original: false,
            encoding: "utf-8".to_string(),
        };

        // Should handle the command (even if it fails due to missing files)
        let _result = dispatch_command(Commands::Convert(args), config_service).await;
        // Just verify it doesn't panic - actual success depends on file existence
    }

    #[tokio::test]
    async fn test_dispatch_with_ref() {
        let config_service = TestConfigService::with_ai_settings("test_provider", "test_model");
        let args = MatchArgs {
            path: Some("/tmp/test".into()),
            input_paths: vec![],
            dry_run: true,
            confidence: 80,
            recursive: false,
            backup: false,
            copy: false,
            move_files: false,
        };

        // Test the reference version
        let result = dispatch_command_with_ref(Commands::Match(args), &config_service).await;

        match result {
            Ok(_) => {} // Success case
            Err(e) => {
                // Allow certain expected errors like missing files in test environment
                let error_msg = format!("{:?}", e);
                assert!(
                    error_msg.contains("NotFound")
                        || error_msg.contains("No subtitle files found")
                        || error_msg.contains("No video files found")
                        || error_msg.contains("Config"),
                    "Unexpected error: {:?}",
                    e
                );
            }
        }
    }
}
