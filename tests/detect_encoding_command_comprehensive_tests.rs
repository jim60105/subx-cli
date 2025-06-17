//! Comprehensive tests for detect encoding command functionality.
//!
//! This module provides test coverage for the detect_encoding_command functions,
//! testing various scenarios and edge cases according to the testing guidelines
//! in `docs/testing-guidelines.md`.

use std::fs;
use subx_cli::cli::DetectEncodingArgs;
use subx_cli::commands::detect_encoding_command::{
    detect_encoding_command, detect_encoding_command_with_config,
};
use subx_cli::config::TestConfigService;
use tempfile::TempDir;

mod detect_encoding_command_tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_encoding_command_with_utf8_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create UTF-8 subtitle file
        let utf8_file = temp_path.join("test_utf8.srt");
        let utf8_content = "1\n00:00:01,000 --> 00:00:03,000\nHello, 世界! UTF-8 test\n\n";
        fs::write(&utf8_file, utf8_content.as_bytes()).unwrap();

        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![utf8_file.clone()],
            recursive: false,
            file_paths: vec![],
        };

        let result = detect_encoding_command(&args);
        assert!(
            result.is_ok(),
            "detect_encoding_command should succeed with UTF-8 file"
        );
    }

    #[tokio::test]
    async fn test_detect_encoding_command_with_ascii_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create ASCII subtitle file
        let ascii_file = temp_path.join("test_ascii.srt");
        let ascii_content = "1\n00:00:01,000 --> 00:00:03,000\nHello, world! ASCII test\n\n";
        fs::write(&ascii_file, ascii_content.as_bytes()).unwrap();

        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![ascii_file.clone()],
            recursive: false,
            file_paths: vec![],
        };

        let result = detect_encoding_command(&args);
        assert!(
            result.is_ok(),
            "detect_encoding_command should succeed with ASCII file"
        );
    }

    #[tokio::test]
    async fn test_detect_encoding_command_verbose_mode() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test file
        let test_file = temp_path.join("test_verbose.srt");
        let content = "1\n00:00:01,000 --> 00:00:03,000\nThis is a test subtitle with some longer content for verbose mode testing\n\n";
        fs::write(&test_file, content.as_bytes()).unwrap();

        let args = DetectEncodingArgs {
            verbose: true,
            input_paths: vec![test_file.clone()],
            recursive: false,
            file_paths: vec![],
        };

        let result = detect_encoding_command(&args);
        assert!(
            result.is_ok(),
            "detect_encoding_command should succeed in verbose mode"
        );
    }

    #[tokio::test]
    async fn test_detect_encoding_command_with_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create multiple test files
        let file1 = temp_path.join("test1.srt");
        let file2 = temp_path.join("test2.vtt");
        let file3 = temp_path.join("test3.ass");

        fs::write(
            &file1,
            "1\n00:00:01,000 --> 00:00:03,000\nFirst subtitle\n\n",
        )
        .unwrap();
        fs::write(
            &file2,
            "WEBVTT\n\n00:00:01.000 --> 00:00:03.000\nSecond subtitle\n\n",
        )
        .unwrap();
        fs::write(
            &file3,
            "[Script Info]\nTitle: Test\n\n[V4+ Styles]\n\n[Events]\n",
        )
        .unwrap();

        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![file1.clone(), file2.clone(), file3.clone()],
            recursive: false,
            file_paths: vec![],
        };

        let result = detect_encoding_command(&args);
        assert!(
            result.is_ok(),
            "detect_encoding_command should succeed with multiple files"
        );
    }

    #[tokio::test]
    async fn test_detect_encoding_command_with_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a real file alongside nonexistent file
        let real_file = temp_path.join("real.srt");
        fs::write(
            &real_file,
            "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n",
        )
        .unwrap();

        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec![
                "/nonexistent/file.srt".to_string(),
                real_file.to_string_lossy().to_string(),
            ],
        };

        // The command should succeed overall because it will continue processing valid files
        // and just log errors for nonexistent ones
        let result = detect_encoding_command(&args);
        assert!(
            result.is_ok(),
            "detect_encoding_command should handle nonexistent files gracefully"
        );
    }

    #[tokio::test]
    async fn test_detect_encoding_command_with_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create empty file
        let empty_file = temp_path.join("empty.srt");
        fs::write(&empty_file, "").unwrap();

        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![empty_file.clone()],
            recursive: false,
            file_paths: vec![],
        };

        let result = detect_encoding_command(&args);
        assert!(
            result.is_ok(),
            "detect_encoding_command should handle empty files"
        );
    }

    #[tokio::test]
    async fn test_detect_encoding_command_with_binary_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create binary file (not a valid subtitle)
        let binary_file = temp_path.join("binary.srt");
        let binary_data = vec![0u8, 1u8, 2u8, 255u8, 254u8, 253u8];
        fs::write(&binary_file, binary_data).unwrap();

        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![binary_file.clone()],
            recursive: false,
            file_paths: vec![],
        };

        let result = detect_encoding_command(&args);
        // Should not fail, but may not detect a valid encoding
        assert!(
            result.is_ok(),
            "detect_encoding_command should handle binary files"
        );
    }

    #[tokio::test]
    async fn test_detect_encoding_command_with_config() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test file
        let test_file = temp_path.join("test_config.srt");
        let content = "1\n00:00:01,000 --> 00:00:03,000\nTest with config service\n\n";
        fs::write(&test_file, content.as_bytes()).unwrap();

        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![test_file.clone()],
            recursive: false,
            file_paths: vec![],
        };

        let config_service = TestConfigService::with_defaults();

        let result = detect_encoding_command_with_config(args, &config_service);
        assert!(
            result.is_ok(),
            "detect_encoding_command_with_config should succeed"
        );
    }

    #[tokio::test]
    async fn test_detect_encoding_command_with_legacy_file_paths() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test file
        let test_file = temp_path.join("legacy_test.srt");
        let content = "1\n00:00:01,000 --> 00:00:03,000\nLegacy file paths test\n\n";
        fs::write(&test_file, content.as_bytes()).unwrap();

        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec![test_file.to_string_lossy().to_string()],
        };

        let result = detect_encoding_command(&args);
        assert!(
            result.is_ok(),
            "detect_encoding_command should work with legacy file_paths"
        );
    }

    #[tokio::test]
    async fn test_detect_encoding_command_recursive_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create nested directory structure with subtitle files
        let sub_dir = temp_path.join("subtitles");
        fs::create_dir(&sub_dir).unwrap();

        let file1 = temp_path.join("root.srt");
        let file2 = sub_dir.join("nested.srt");

        fs::write(
            &file1,
            "1\n00:00:01,000 --> 00:00:03,000\nRoot subtitle\n\n",
        )
        .unwrap();
        fs::write(
            &file2,
            "1\n00:00:01,000 --> 00:00:03,000\nNested subtitle\n\n",
        )
        .unwrap();

        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![temp_path.to_path_buf()],
            recursive: true,
            file_paths: vec![],
        };

        let result = detect_encoding_command(&args);
        assert!(
            result.is_ok(),
            "detect_encoding_command should handle recursive directory processing"
        );
    }

    #[tokio::test]
    async fn test_detect_encoding_command_no_input_specified() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec![],
        };

        let result = detect_encoding_command(&args);
        // Should fail with NoInputSpecified error
        assert!(
            result.is_err(),
            "detect_encoding_command should fail when no input is specified"
        );
    }

    #[tokio::test]
    async fn test_detect_encoding_command_with_different_extensions() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create files with different subtitle extensions
        let extensions = vec!["srt", "vtt", "ass", "ssa", "sub", "txt"];
        let mut files = Vec::new();

        for ext in extensions {
            let file = temp_path.join(format!("test.{}", ext));
            let content = match ext {
                "vtt" => "WEBVTT\n\n00:00:01.000 --> 00:00:03.000\nTest subtitle\n\n",
                "ass" | "ssa" => "[Script Info]\nTitle: Test\n\n[V4+ Styles]\n\n[Events]\n",
                _ => "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n",
            };
            fs::write(&file, content.as_bytes()).unwrap();
            files.push(file);
        }

        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: files,
            recursive: false,
            file_paths: vec![],
        };

        let result = detect_encoding_command(&args);
        assert!(
            result.is_ok(),
            "detect_encoding_command should handle different subtitle file extensions"
        );
    }
}
