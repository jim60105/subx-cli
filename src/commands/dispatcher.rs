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
///     no_extract: false,
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
        Commands::Translate(args) => {
            crate::commands::translate_command::execute_with_config(args, config_service).await
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
        Commands::Translate(args) => {
            crate::commands::translate_command::execute(args, config_service).await
        }
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
    use crate::cli::{
        CacheAction, CacheArgs, ClearArgs, ClearType, ConfigAction, ConfigArgs, ConvertArgs,
        DetectEncodingArgs, GenerateCompletionArgs, MatchArgs, OutputSubtitleFormat, StatusArgs,
        SyncArgs,
    };
    use crate::config::TestConfigService;
    use clap_complete::Shell;

    // Helper to check that errors are expected filesystem/config errors in a test environment
    fn is_expected_test_error(e: &crate::error::SubXError) -> bool {
        let msg = format!("{e:?}");
        msg.contains("NotFound")
            || msg.contains("No subtitle files found")
            || msg.contains("No video files found")
            || msg.contains("Config")
            || msg.contains("no such file")
            || msg.contains("cannot find")
            || msg.contains("No input")
            || msg.contains("No files")
            || msg.contains("FileNotFound")
            || msg.contains("IoError")
            || msg.contains("PathNotFound")
            || msg.contains("InvalidInput")
            || msg.contains("CommandExecution")
            || msg.contains("NoInputSpecified")
    }

    fn make_match_args_dry_run() -> MatchArgs {
        MatchArgs {
            path: Some("/nonexistent_subx_test_path".into()),
            input_paths: vec![],
            dry_run: true,
            confidence: 80,
            recursive: false,
            backup: false,
            copy: false,
            move_files: false,
            no_extract: false,
        }
    }

    fn make_convert_args() -> ConvertArgs {
        ConvertArgs {
            input: Some("/nonexistent_subx_test_path".into()),
            input_paths: vec![],
            recursive: false,
            format: Some(OutputSubtitleFormat::Srt),
            output: None,
            keep_original: false,
            encoding: "utf-8".to_string(),
            no_extract: false,
        }
    }

    fn make_sync_args() -> SyncArgs {
        SyncArgs {
            positional_paths: vec![],
            video: None,
            subtitle: None,
            input_paths: vec![],
            recursive: false,
            offset: Some(1.0),
            method: None,
            window: 30,
            vad_sensitivity: None,
            output: None,
            verbose: false,
            dry_run: true,
            force: false,
            batch: None,
            no_extract: false,
        }
    }

    fn make_config_args_list() -> ConfigArgs {
        ConfigArgs {
            action: ConfigAction::List,
        }
    }

    fn make_generate_completion_args() -> GenerateCompletionArgs {
        GenerateCompletionArgs { shell: Shell::Bash }
    }

    fn make_cache_status_args() -> CacheArgs {
        CacheArgs {
            action: CacheAction::Status(StatusArgs { json: false }),
        }
    }

    fn make_cache_clear_args() -> CacheArgs {
        CacheArgs {
            action: CacheAction::Clear(ClearArgs {
                r#type: ClearType::All,
            }),
        }
    }

    fn make_detect_encoding_args() -> DetectEncodingArgs {
        DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec![],
            no_extract: false,
        }
    }

    // ── dispatch_command (Arc<dyn ConfigService>) ─────────────────────────────

    #[tokio::test]
    async fn test_dispatch_match_command() {
        let config_service = Arc::new(TestConfigService::with_ai_settings(
            "test_provider",
            "test_model",
        ));
        let result =
            dispatch_command(Commands::Match(make_match_args_dry_run()), config_service).await;
        match result {
            Ok(_) => {}
            Err(e) => assert!(is_expected_test_error(&e), "Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_convert_command() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        // The result may fail due to missing files; we only verify no panic occurs.
        let _result =
            dispatch_command(Commands::Convert(make_convert_args()), config_service).await;
    }

    #[tokio::test]
    async fn test_dispatch_sync_command() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        let result = dispatch_command(Commands::Sync(make_sync_args()), config_service).await;
        match result {
            Ok(_) => {}
            Err(e) => assert!(is_expected_test_error(&e), "Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_config_list_command() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        let result =
            dispatch_command(Commands::Config(make_config_args_list()), config_service).await;
        // Config list should either succeed or produce an expected config error
        match result {
            Ok(_) => {}
            Err(e) => assert!(is_expected_test_error(&e), "Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_generate_completion_command() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        // GenerateCompletion writes to stdout and returns Ok
        let result = dispatch_command(
            Commands::GenerateCompletion(make_generate_completion_args()),
            config_service,
        )
        .await;
        assert!(
            result.is_ok(),
            "GenerateCompletion should succeed: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_dispatch_cache_status_command() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        let _result =
            dispatch_command(Commands::Cache(make_cache_status_args()), config_service).await;
    }

    #[tokio::test]
    async fn test_dispatch_cache_clear_command() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        let _result =
            dispatch_command(Commands::Cache(make_cache_clear_args()), config_service).await;
    }

    #[tokio::test]
    async fn test_dispatch_detect_encoding_command() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        let result = dispatch_command(
            Commands::DetectEncoding(make_detect_encoding_args()),
            config_service,
        )
        .await;
        match result {
            Ok(_) => {}
            Err(e) => assert!(is_expected_test_error(&e), "Unexpected error: {e:?}"),
        }
    }

    // ── dispatch_command_with_ref (&dyn ConfigService) ────────────────────────

    #[tokio::test]
    async fn test_dispatch_with_ref_match_command() {
        let config_service = TestConfigService::with_ai_settings("test_provider", "test_model");
        let result =
            dispatch_command_with_ref(Commands::Match(make_match_args_dry_run()), &config_service)
                .await;
        match result {
            Ok(_) => {}
            Err(e) => assert!(is_expected_test_error(&e), "Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_with_ref_match_validation_error() {
        // copy + move together must fail with a CommandExecution validation error
        let config_service = TestConfigService::with_defaults();
        let args = MatchArgs {
            path: Some("/nonexistent_subx_test_path".into()),
            input_paths: vec![],
            dry_run: true,
            confidence: 80,
            recursive: false,
            backup: false,
            copy: true,
            move_files: true,
            no_extract: false,
        };
        let result = dispatch_command_with_ref(Commands::Match(args), &config_service).await;
        assert!(result.is_err(), "Expected validation error for copy+move");
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("CommandExecution") || msg.contains("copy") || msg.contains("move"),
            "Error should mention the conflicting flags: {msg}"
        );
    }

    #[tokio::test]
    async fn test_dispatch_with_ref_convert_command() {
        let config_service = TestConfigService::with_defaults();
        let _result =
            dispatch_command_with_ref(Commands::Convert(make_convert_args()), &config_service)
                .await;
    }

    #[tokio::test]
    async fn test_dispatch_with_ref_sync_command() {
        let config_service = TestConfigService::with_defaults();
        let result =
            dispatch_command_with_ref(Commands::Sync(make_sync_args()), &config_service).await;
        match result {
            Ok(_) => {}
            Err(e) => assert!(is_expected_test_error(&e), "Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_with_ref_config_list_command() {
        let config_service = TestConfigService::with_defaults();
        let result =
            dispatch_command_with_ref(Commands::Config(make_config_args_list()), &config_service)
                .await;
        match result {
            Ok(_) => {}
            Err(e) => assert!(is_expected_test_error(&e), "Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_with_ref_generate_completion_command() {
        let config_service = TestConfigService::with_defaults();
        let result = dispatch_command_with_ref(
            Commands::GenerateCompletion(make_generate_completion_args()),
            &config_service,
        )
        .await;
        assert!(
            result.is_ok(),
            "GenerateCompletion should succeed: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_dispatch_with_ref_cache_status_command() {
        let config_service = TestConfigService::with_defaults();
        let _result =
            dispatch_command_with_ref(Commands::Cache(make_cache_status_args()), &config_service)
                .await;
    }

    #[tokio::test]
    async fn test_dispatch_with_ref_cache_clear_command() {
        let config_service = TestConfigService::with_defaults();
        let _result =
            dispatch_command_with_ref(Commands::Cache(make_cache_clear_args()), &config_service)
                .await;
    }

    #[tokio::test]
    async fn test_dispatch_with_ref_detect_encoding_command() {
        let config_service = TestConfigService::with_defaults();
        let result = dispatch_command_with_ref(
            Commands::DetectEncoding(make_detect_encoding_args()),
            &config_service,
        )
        .await;
        match result {
            Ok(_) => {}
            Err(e) => assert!(is_expected_test_error(&e), "Unexpected error: {e:?}"),
        }
    }

    // ── Configuration is forwarded to subcommands ─────────────────────────────

    #[tokio::test]
    async fn test_dispatch_config_get_command() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        let args = ConfigArgs {
            action: ConfigAction::Get {
                key: "ai.provider".to_string(),
            },
        };
        let result = dispatch_command(Commands::Config(args), config_service).await;
        match result {
            Ok(_) => {}
            Err(e) => assert!(is_expected_test_error(&e), "Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_config_set_command() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        let args = ConfigArgs {
            action: ConfigAction::Set {
                key: "ai.provider".to_string(),
                value: "openai".to_string(),
            },
        };
        let result = dispatch_command(Commands::Config(args), config_service).await;
        match result {
            Ok(_) => {}
            Err(e) => assert!(is_expected_test_error(&e), "Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_generate_completion_zsh() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        let result = dispatch_command(
            Commands::GenerateCompletion(GenerateCompletionArgs { shell: Shell::Zsh }),
            config_service,
        )
        .await;
        assert!(result.is_ok(), "Zsh completion should succeed: {result:?}");
    }

    #[tokio::test]
    async fn test_dispatch_generate_completion_fish() {
        let config_service = Arc::new(TestConfigService::with_defaults());
        let result = dispatch_command(
            Commands::GenerateCompletion(GenerateCompletionArgs { shell: Shell::Fish }),
            config_service,
        )
        .await;
        assert!(result.is_ok(), "Fish completion should succeed: {result:?}");
    }
}
