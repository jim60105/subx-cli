use clap::Parser;
use subx_cli::cli::{Cli, Commands, DetectEncodingArgs};

#[test]
fn test_detect_encoding_args_verbose_and_paths() {
    let cli = Cli::try_parse_from(&[
        "subx", "detect-encoding", "--verbose", "file1.srt", "file2.ass",
    ])
    .expect("parse 'detect-encoding --verbose file1.srt file2.ass'");
    match cli.command {
        Commands::DetectEncoding(DetectEncodingArgs { verbose, file_paths }) => {
            assert!(verbose, "verbose flag should be true");
            assert_eq!(file_paths, vec!["file1.srt", "file2.ass"]);
        }
        _ => panic!("Expected DetectEncoding subcommand"),
    }
}

#[test]
fn test_detect_encoding_args_missing_paths_fails() {
    let err = Cli::try_parse_from(&["subx", "detect-encoding"]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("The following required arguments were not provided"));
}
