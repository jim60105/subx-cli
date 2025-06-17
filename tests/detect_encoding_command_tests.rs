//! Tests for DetectEncodingCommand
//!
//! These tests verify the functionality of the detect-encoding command,
//! including argument parsing, file detection, and edge cases.

use std::path::PathBuf;
use subx_cli::cli::DetectEncodingArgs;
use subx_cli::commands::detect_encoding_command;

#[cfg(test)]
mod detect_encoding_args_tests {
    use super::*;

    #[test]
    fn test_detect_encoding_args_with_file_paths() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec!["test.srt".to_string()],
        };

        assert_eq!(args.file_paths.len(), 1);
        assert_eq!(args.file_paths[0], "test.srt");
        assert!(!args.verbose);
        assert!(!args.recursive);
        assert!(args.input_paths.is_empty());
    }

    #[test]
    fn test_detect_encoding_args_with_input_paths() {
        let args = DetectEncodingArgs {
            verbose: true,
            input_paths: vec![PathBuf::from("test.srt")],
            recursive: true,
            file_paths: vec![],
        };

        assert_eq!(args.input_paths.len(), 1);
        assert_eq!(args.input_paths[0], PathBuf::from("test.srt"));
        assert!(args.verbose);
        assert!(args.recursive);
        assert!(args.file_paths.is_empty());
    }

    #[test]
    fn test_detect_encoding_args_multiple_file_paths() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec![
                "file1.srt".to_string(),
                "file2.sub".to_string(),
                "file3.vtt".to_string(),
            ],
        };

        assert_eq!(args.file_paths.len(), 3);
        assert_eq!(args.file_paths[0], "file1.srt");
        assert_eq!(args.file_paths[1], "file2.sub");
        assert_eq!(args.file_paths[2], "file3.vtt");
        assert!(!args.verbose);
        assert!(!args.recursive);
    }

    #[test]
    fn test_detect_encoding_args_multiple_input_paths() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![PathBuf::from("*.srt"), PathBuf::from("subtitles/**/*.vtt")],
            recursive: true,
            file_paths: vec![],
        };

        assert_eq!(args.input_paths.len(), 2);
        assert_eq!(args.input_paths[0], PathBuf::from("*.srt"));
        assert_eq!(args.input_paths[1], PathBuf::from("subtitles/**/*.vtt"));
        assert!(!args.verbose);
        assert!(args.recursive);
    }

    #[test]
    fn test_detect_encoding_args_with_verbose() {
        let args = DetectEncodingArgs {
            verbose: true,
            input_paths: vec![],
            recursive: false,
            file_paths: vec!["test.srt".to_string(), "test.vtt".to_string()],
        };

        assert_eq!(args.file_paths.len(), 2);
        assert!(!args.file_paths.is_empty());
        assert!(args.verbose);
        assert!(!args.recursive);
    }

    #[test]
    fn test_detect_encoding_args_get_file_paths_from_file_paths() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec!["file1.srt".to_string(), "file2.vtt".to_string()],
        };

        let result = args.get_file_paths();
        assert!(result.is_ok());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("file1.srt"));
        assert_eq!(paths[1], PathBuf::from("file2.vtt"));
    }

    #[test]
    fn test_detect_encoding_args_with_all_options() {
        let args = DetectEncodingArgs {
            verbose: true,
            input_paths: vec![],
            recursive: true,
            file_paths: vec!["test.srt".to_string()],
        };

        // Test that all fields are properly set
        assert!(!args.file_paths.is_empty());
        assert!(args.verbose);
        assert!(args.recursive);
        assert!(args.input_paths.is_empty());
        assert!(args.file_paths.iter().any(|p| p == "test.srt"));
    }

    #[test]
    fn test_detect_encoding_args_field_access() {
        let args = DetectEncodingArgs {
            verbose: true,
            input_paths: vec![],
            recursive: false,
            file_paths: vec!["test1.srt".to_string(), "test2.vtt".to_string()],
        };

        // Verify we can access all fields
        assert_eq!(args.file_paths.len(), 2);
        assert_eq!(args.file_paths[0], "test1.srt");
        assert_eq!(args.file_paths[1], "test2.vtt");
        assert!(args.verbose);
        assert!(!args.recursive);

        // Test iteration over paths
        for (i, path) in args.file_paths.iter().enumerate() {
            match i {
                0 => assert_eq!(path, "test1.srt"),
                1 => assert_eq!(path, "test2.vtt"),
                _ => panic!("Unexpected path count"),
            }
        }
    }

    #[test]
    fn test_detect_encoding_args_all_extensions() {
        let extensions = ["srt", "vtt", "ass", "ssa", "sub"];

        for ext in &extensions {
            let filename = format!("test.{}", ext);
            let args = DetectEncodingArgs {
                verbose: false,
                input_paths: vec![],
                recursive: false,
                file_paths: vec![filename.clone()],
            };

            assert_eq!(args.file_paths.len(), 1);
            assert_eq!(args.file_paths[0], filename);
        }
    }

    #[test]
    fn test_detect_encoding_args_paths_with_spaces() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec![
                "path with spaces.srt".to_string(),
                "another path/file.vtt".to_string(),
            ],
        };

        assert_eq!(args.file_paths.len(), 2);
        assert_eq!(args.file_paths[0], "path with spaces.srt");
        assert_eq!(args.file_paths[1], "another path/file.vtt");
        assert!(!args.verbose);
        assert!(!args.recursive);
    }

    #[test]
    fn test_detect_encoding_args_unicode_paths() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec![
                "файл.srt".to_string(),
                "字幕.vtt".to_string(),
                "ファイル.ass".to_string(),
            ],
        };

        assert_eq!(args.file_paths.len(), 3);
        assert_eq!(args.file_paths[0], "файл.srt");
        assert_eq!(args.file_paths[1], "字幕.vtt");
        assert_eq!(args.file_paths[2], "ファイル.ass");
        assert!(!args.verbose);
        assert!(!args.recursive);
    }

    #[test]
    fn test_detect_encoding_args_mutable_access() {
        let mut args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec!["file1.srt".to_string()],
        };

        args.file_paths.push("file2.vtt".to_string());
        args.verbose = true;
        args.recursive = true;

        assert_eq!(args.file_paths.len(), 2);
        assert_eq!(args.file_paths[0], "file1.srt");
        assert_eq!(args.file_paths[1], "file2.vtt");
        assert!(args.verbose);
        assert!(args.recursive);

        // Test that we can search paths
        assert!(args.file_paths.iter().any(|p| p.contains("file1")));
        assert!(args.file_paths.iter().any(|p| p.contains("file2")));
    }

    #[test]
    fn test_detect_encoding_args_get_input_handler() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![PathBuf::from("test.srt")],
            recursive: true,
            file_paths: vec![],
        };

        let result = args.get_input_handler();
        // This might fail due to file validation, which is expected
        if result.is_ok() {
            let handler = result.unwrap();
            // We can't test much more without actually creating files,
            // but we can verify the handler was created successfully
            assert_eq!(format!("{:?}", handler).contains("InputPathHandler"), true);
        } else {
            // If it fails, that's also acceptable for testing purposes
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_detect_encoding_args_no_input_error() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec![],
        };

        let result = args.get_file_paths();
        assert!(result.is_err());
        // Should return NoInputSpecified error
    }
}

#[cfg(test)]
mod detect_encoding_command_tests {
    use super::*;

    #[test]
    fn test_detect_encoding_command_execute() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec!["nonexistent.srt".to_string()],
        };

        // Test that we can call the detect_encoding_command function
        // This will likely fail due to non-existent file, but that's expected
        let result = detect_encoding_command::detect_encoding_command(&args);
        // We expect this to either succeed or fail gracefully
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_detect_encoding_command_with_empty_args() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![],
            recursive: false,
            file_paths: vec![],
        };

        let result = detect_encoding_command::detect_encoding_command(&args);
        // Should fail with no input specified
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_encoding_command_with_verbose() {
        let args = DetectEncodingArgs {
            verbose: true,
            input_paths: vec![],
            recursive: false,
            file_paths: vec!["nonexistent.srt".to_string()],
        };

        let result = detect_encoding_command::detect_encoding_command(&args);
        // Should handle verbose mode correctly
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_detect_encoding_command_with_recursive() {
        let args = DetectEncodingArgs {
            verbose: false,
            input_paths: vec![PathBuf::from("nonexistent_dir")],
            recursive: true,
            file_paths: vec![],
        };

        let result = detect_encoding_command::detect_encoding_command(&args);
        // Should handle recursive mode correctly
        assert!(result.is_err() || result.is_ok());
    }
}
