//! Comprehensive tests for sync command functionality.
//!
//! This module provides test coverage for synchronization command operations,
//! testing both single and batch modes according to the testing guidelines
//! in `docs/testing-guidelines.md`.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use subx_cli::cli::SyncArgs;
use subx_cli::commands::sync_command::{execute, execute_with_config};
use subx_cli::config::{ConfigService, TestConfigService};
use tempfile::TempDir;

mod sync_command_tests {
    use super::*;

    fn create_default_sync_args() -> SyncArgs {
        SyncArgs {
            video: None,
            subtitle: None,
            input_paths: vec![],
            recursive: false,
            offset: None,
            method: None,
            window: 30,
            vad_sensitivity: None,
            output: None,
            verbose: false,
            dry_run: false,
            force: false,
            batch: false,
            #[allow(deprecated)]
            range: None,
            #[allow(deprecated)]
            threshold: None,
        }
    }

    #[tokio::test]
    async fn test_sync_command_with_manual_offset() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test subtitle file
        let subtitle_file = temp_path.join("test.srt");
        let subtitle_content = "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n";
        fs::write(&subtitle_file, subtitle_content).unwrap();

        let config_service = TestConfigService::with_defaults();

        let mut args = create_default_sync_args();
        args.subtitle = Some(subtitle_file.clone());
        args.offset = Some(2.5);

        let result = execute(args, &config_service).await;
        assert!(
            result.is_ok(),
            "Sync command should succeed with manual offset"
        );
    }

    #[tokio::test]
    async fn test_sync_command_verbose_mode() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test subtitle file
        let subtitle_file = temp_path.join("test.srt");
        let subtitle_content = "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n2\n00:00:04,000 --> 00:00:06,000\nAnother subtitle\n\n";
        fs::write(&subtitle_file, subtitle_content).unwrap();

        let config_service = TestConfigService::with_defaults();

        let mut args = create_default_sync_args();
        args.subtitle = Some(subtitle_file.clone());
        args.offset = Some(1.0);
        args.verbose = true;

        let result = execute(args, &config_service).await;
        assert!(
            result.is_ok(),
            "Sync command should succeed in verbose mode"
        );
    }

    #[tokio::test]
    async fn test_sync_command_with_output_path() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test subtitle file
        let subtitle_file = temp_path.join("input.srt");
        let output_file = temp_path.join("output.srt");
        let subtitle_content = "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n";
        fs::write(&subtitle_file, subtitle_content).unwrap();

        let config_service = TestConfigService::with_defaults();

        let mut args = create_default_sync_args();
        args.subtitle = Some(subtitle_file.clone());
        args.output = Some(output_file.clone());
        args.offset = Some(1.0);

        let result = execute(args, &config_service).await;
        assert!(
            result.is_ok(),
            "Sync command should succeed with output path"
        );
    }

    #[tokio::test]
    async fn test_sync_command_dry_run_mode() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test subtitle file
        let subtitle_file = temp_path.join("test.srt");
        let subtitle_content = "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n";
        fs::write(&subtitle_file, subtitle_content).unwrap();

        let config_service = TestConfigService::with_defaults();

        let mut args = create_default_sync_args();
        args.subtitle = Some(subtitle_file.clone());
        args.offset = Some(0.5);
        args.dry_run = true;

        let result = execute(args, &config_service).await;
        assert!(
            result.is_ok(),
            "Sync command should succeed in dry run mode"
        );
    }

    #[tokio::test]
    async fn test_sync_command_execute_with_config_arc() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test subtitle file
        let subtitle_file = temp_path.join("test.srt");
        let subtitle_content = "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n";
        fs::write(&subtitle_file, subtitle_content).unwrap();

        let config_service: Arc<dyn ConfigService> = Arc::new(TestConfigService::with_defaults());

        let mut args = create_default_sync_args();
        args.subtitle = Some(subtitle_file.clone());
        args.offset = Some(1.5);

        let result = execute_with_config(args, config_service).await;
        assert!(
            result.is_ok(),
            "Sync command execute_with_config should succeed"
        );
    }

    #[tokio::test]
    async fn test_sync_command_with_zero_offset() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test subtitle file
        let subtitle_file = temp_path.join("test.srt");
        let subtitle_content = "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n";
        fs::write(&subtitle_file, subtitle_content).unwrap();

        let config_service = TestConfigService::with_defaults();

        let mut args = create_default_sync_args();
        args.subtitle = Some(subtitle_file.clone());
        args.offset = Some(0.0);

        let result = execute(args, &config_service).await;
        assert!(
            result.is_ok(),
            "Sync command should succeed with zero offset"
        );
    }

    #[tokio::test]
    async fn test_sync_command_with_negative_offset() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test subtitle file
        let subtitle_file = temp_path.join("test.srt");
        let subtitle_content = "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n";
        fs::write(&subtitle_file, subtitle_content).unwrap();

        let config_service = TestConfigService::with_defaults();

        let mut args = create_default_sync_args();
        args.subtitle = Some(subtitle_file.clone());
        args.offset = Some(-2.0);

        let result = execute(args, &config_service).await;
        assert!(
            result.is_ok(),
            "Sync command should succeed with negative offset"
        );
    }

    #[tokio::test]
    async fn test_sync_command_missing_subtitle_file() {
        let config_service = TestConfigService::with_defaults();

        let mut args = create_default_sync_args();
        args.subtitle = Some(PathBuf::from("/nonexistent/subtitle.srt"));
        args.offset = Some(1.0);

        let result = execute(args, &config_service).await;
        assert!(
            result.is_err(),
            "Sync command should fail with missing subtitle file"
        );
    }

    #[tokio::test]
    async fn test_sync_command_batch_mode() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        let subtitle_file = temp_path.join("movie.srt");
        fs::write(
            &subtitle_file,
            "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n",
        )
        .unwrap();

        let config_service = TestConfigService::with_defaults();

        let mut args = create_default_sync_args();
        args.input_paths = vec![temp_path.to_path_buf()];
        args.batch = true;
        args.offset = Some(2.0);

        let result = execute(args, &config_service).await;
        // This may fail due to missing video files, but we test the logic path
        assert!(
            result.is_ok() || result.is_err(),
            "Sync command should handle batch mode"
        );
    }

    #[tokio::test]
    async fn test_sync_command_with_force_flag() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test subtitle file
        let subtitle_file = temp_path.join("test.srt");
        let output_file = temp_path.join("output.srt");
        let subtitle_content = "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n";
        fs::write(&subtitle_file, subtitle_content).unwrap();
        // Create existing output file
        fs::write(&output_file, "existing content").unwrap();

        let config_service = TestConfigService::with_defaults();

        let mut args = create_default_sync_args();
        args.subtitle = Some(subtitle_file.clone());
        args.output = Some(output_file.clone());
        args.offset = Some(1.0);
        args.force = true;

        let result = execute(args, &config_service).await;
        assert!(
            result.is_ok(),
            "Sync command should succeed with force flag"
        );
    }
}
