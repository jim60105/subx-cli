//! Integration tests for `subx translate` CLI argument parsing and
//! validation.
//!
//! These tests live as a top-level integration test (per AGENTS.md) and
//! exercise the public CLI surface end-to-end: they verify that clap accepts
//! the documented flag combinations, that `validate` rejects misuses, and
//! that `--glossary` is treated as a filesystem path while `--context` is
//! passed through as inline text.

use clap::Parser;
use std::path::PathBuf;
use subx_cli::cli::{Cli, Commands, TranslateArgs};
use tempfile::tempdir;

fn parse_translate(args: &[&str]) -> TranslateArgs {
    let cli = Cli::try_parse_from(args).expect("CLI parse should succeed");
    match cli.command {
        Commands::Translate(a) => a,
        other => panic!("Expected Translate command, got {:?}", other),
    }
}

#[test]
fn translate_parses_minimum_invocation_with_target_language() {
    let args = parse_translate(&[
        "subx-cli",
        "translate",
        "movie.srt",
        "--target-language",
        "zh-TW",
    ]);

    assert_eq!(args.paths, vec![PathBuf::from("movie.srt")]);
    assert_eq!(args.target_language.as_deref(), Some("zh-TW"));
    assert!(args.source_language.is_none());
    assert!(args.glossary.is_none());
    assert!(args.context.is_none());
    assert!(args.output.is_none());
    assert!(!args.force);
    assert!(!args.replace);
    assert!(!args.no_extract);
}

#[test]
fn translate_allows_target_language_omission_at_parse_time() {
    // Clap intentionally allows omission so the configured default can win;
    // command-level resolution enforces presence.
    let args = parse_translate(&["subx-cli", "translate", "movie.srt"]);
    assert!(args.target_language.is_none());
}

#[test]
fn translate_parses_full_flag_combo_including_force_no_extract() {
    let args = parse_translate(&[
        "subx-cli",
        "translate",
        "-i",
        "dir1",
        "-i",
        "extra.srt",
        "--recursive",
        "--target-language",
        "ja",
        "--source-language",
        "en",
        "--glossary",
        "glossary.txt",
        "--context",
        "Use formal tone",
        "--output",
        "out/",
        "--no-extract",
        "--force",
    ]);

    assert!(args.paths.is_empty());
    assert_eq!(
        args.input_paths,
        vec![PathBuf::from("dir1"), PathBuf::from("extra.srt")]
    );
    assert!(args.recursive);
    assert_eq!(args.target_language.as_deref(), Some("ja"));
    assert_eq!(args.source_language.as_deref(), Some("en"));
    assert_eq!(args.glossary, Some(PathBuf::from("glossary.txt")));
    assert_eq!(args.context.as_deref(), Some("Use formal tone"));
    assert_eq!(args.output, Some(PathBuf::from("out/")));
    assert!(args.no_extract);
    assert!(args.force);
    assert!(!args.replace);
}

#[test]
fn translate_accepts_overwrite_alias_for_force() {
    let args = parse_translate(&[
        "subx-cli",
        "translate",
        "movie.srt",
        "--target-language",
        "zh-TW",
        "--overwrite",
    ]);
    assert!(args.force);
}

#[test]
fn translate_rejects_force_with_replace_at_parse_time() {
    let err = Cli::try_parse_from([
        "subx-cli",
        "translate",
        "movie.srt",
        "--target-language",
        "zh-TW",
        "--force",
        "--replace",
    ])
    .expect_err("--force and --replace conflict");
    assert!(err.to_string().contains("cannot be used"));
}

#[test]
fn translate_validate_accepts_existing_glossary_file() {
    let tmp = tempdir().expect("tempdir");
    let glossary_path = tmp.path().join("glossary.txt");
    std::fs::write(&glossary_path, "Alice = 艾莉絲\n").unwrap();

    let args = TranslateArgs {
        paths: vec![PathBuf::from("a.srt")],
        input_paths: vec![],
        recursive: false,
        target_language: Some("zh-TW".to_string()),
        source_language: None,
        glossary: Some(glossary_path),
        context: Some("Use formal tone".to_string()),
        output: None,
        no_extract: false,
        force: false,
        replace: false,
    };
    args.validate()
        .expect("existing glossary file should validate");
}

#[test]
fn translate_validate_rejects_glossary_directory() {
    // The glossary value must be treated as a filesystem path; a directory
    // is not a valid glossary file even though it exists.
    let tmp = tempdir().expect("tempdir");
    let dir_path = tmp.path().to_path_buf();

    let args = TranslateArgs {
        paths: vec![PathBuf::from("a.srt")],
        input_paths: vec![],
        recursive: false,
        target_language: Some("zh-TW".to_string()),
        source_language: None,
        glossary: Some(dir_path),
        context: None,
        output: None,
        no_extract: false,
        force: false,
        replace: false,
    };
    let err = args
        .validate()
        .expect_err("glossary pointing at a directory must be rejected");
    let msg = format!("{err:?}");
    assert!(msg.contains("Glossary"), "unexpected error: {msg}");
}

#[test]
fn translate_validate_treats_context_as_inline_text_not_path() {
    // The context flag accepts arbitrary inline guidance; it must not be
    // resolved against the filesystem.
    let nonexistent = "/this/path/does/not/exist/and/should/not/be/checked";
    let args = TranslateArgs {
        paths: vec![PathBuf::from("a.srt")],
        input_paths: vec![],
        recursive: false,
        target_language: Some("zh-TW".to_string()),
        source_language: None,
        glossary: None,
        context: Some(nonexistent.to_string()),
        output: None,
        no_extract: false,
        force: false,
        replace: false,
    };
    args.validate()
        .expect("context must be treated as inline text, not a path");
}

#[test]
fn translate_validate_rejects_empty_target_language() {
    let args = TranslateArgs {
        paths: vec![PathBuf::from("a.srt")],
        input_paths: vec![],
        recursive: false,
        target_language: Some("   ".to_string()),
        source_language: None,
        glossary: None,
        context: None,
        output: None,
        no_extract: false,
        force: false,
        replace: false,
    };
    let err = args.validate().expect_err("empty target language fails");
    let msg = format!("{err:?}");
    assert!(msg.contains("target-language"), "unexpected: {msg}");
}

#[test]
fn translate_validate_rejects_force_with_replace() {
    let args = TranslateArgs {
        paths: vec![PathBuf::from("a.srt")],
        input_paths: vec![],
        recursive: false,
        target_language: Some("zh-TW".to_string()),
        source_language: None,
        glossary: None,
        context: None,
        output: None,
        no_extract: false,
        force: true,
        replace: true,
    };
    let err = args
        .validate()
        .expect_err("force/overwrite and replace must be rejected");
    assert!(err.to_string().contains("--replace"));
}

#[test]
fn translate_validate_rejects_empty_source_language() {
    let args = TranslateArgs {
        paths: vec![PathBuf::from("a.srt")],
        input_paths: vec![],
        recursive: false,
        target_language: Some("zh-TW".to_string()),
        source_language: Some("  ".to_string()),
        glossary: None,
        context: None,
        output: None,
        no_extract: false,
        force: false,
        replace: false,
    };
    let err = args
        .validate()
        .expect_err("empty source language fails when provided");
    let msg = format!("{err:?}");
    assert!(msg.contains("source-language"), "unexpected: {msg}");
}
