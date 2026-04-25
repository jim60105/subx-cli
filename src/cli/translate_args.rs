//! Subtitle translation command-line arguments and validation.
//!
//! This module defines the [`TranslateArgs`] structure for the `translate`
//! subcommand. It mirrors the conventions used by the existing mutating
//! subtitle commands (`convert`, `sync`) and supports positional inputs,
//! repeated `-i/--input`, recursive traversal, and archive expansion through
//! the shared [`InputPathHandler`].
//!
//! # Examples
//!
//! ```bash
//! # Translate a single SRT file into Traditional Chinese
//! subx translate movie.srt --target-language zh-TW
//!
//! # Batch translate a directory recursively with a glossary and tone hint
//! subx translate ./subs --recursive \
//!     --target-language ja \
//!     --glossary glossary.txt \
//!     --context "Use formal tone"
//! ```
#![allow(clippy::needless_borrows_for_generic_args)]

use crate::cli::InputPathHandler;
use crate::error::SubXError;
use clap::Args;
use std::path::PathBuf;

/// Command-line arguments for AI-assisted subtitle translation.
///
/// All input collection flags follow the same conventions as the other
/// mutating subtitle commands. Validation rules are intentionally enforced
/// before any AI call so that user mistakes (missing glossary file, empty
/// `--target-language`, etc.) do not consume API quota.
#[derive(Args, Debug, Clone)]
pub struct TranslateArgs {
    /// Positional file or directory paths to translate.
    #[arg(value_name = "PATH", num_args = 0..)]
    pub paths: Vec<PathBuf>,

    /// Specify file or directory paths to process via repeated `-i/--input`.
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input_paths: Vec<PathBuf>,

    /// Recursively process subdirectories.
    #[arg(short, long)]
    pub recursive: bool,

    /// Required target language code or name (for example, `zh-TW`, `ja`,
    /// `English`). When omitted on the CLI, the value falls back to
    /// `translation.default_target_language` from configuration; if neither
    /// is provided the command fails with a usage-style error.
    #[arg(
        short = 't',
        long = "target-language",
        value_name = "LANG",
        help = "Target language code or name (e.g. zh-TW, ja, English)"
    )]
    pub target_language: Option<String>,

    /// Optional source language hint. When omitted, the AI provider is asked
    /// to detect or accept the source language automatically.
    #[arg(
        short = 's',
        long = "source-language",
        value_name = "LANG",
        help = "Optional source language code (default: auto-detect)"
    )]
    pub source_language: Option<String>,

    /// Path to a UTF-8 text glossary file. Glossary entries take precedence
    /// over the AI-generated terminology map.
    #[arg(long = "glossary", value_name = "PATH")]
    pub glossary: Option<PathBuf>,

    /// Inline context guidance forwarded to the translation prompt (for
    /// example, `"Use formal business tone"`).
    #[arg(long = "context", value_name = "TEXT")]
    pub context: Option<String>,

    /// Output file (single-input mode) or directory (batch mode) for the
    /// translated subtitle(s).
    #[arg(short = 'o', long = "output", value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Disable automatic archive extraction for `-i` inputs.
    #[arg(long, default_value_t = false)]
    pub no_extract: bool,

    /// Overwrite existing translated output files.
    ///
    /// `--overwrite` is accepted as a visible alias for consistency with the
    /// user-facing command documentation.
    #[arg(
        long,
        visible_alias = "overwrite",
        default_value_t = false,
        conflicts_with = "replace"
    )]
    pub force: bool,

    /// Replace each source subtitle file with its translated content.
    /// Existing backup settings still apply.
    #[arg(long, default_value_t = false, conflicts_with = "force")]
    pub replace: bool,
}

impl TranslateArgs {
    /// Validate the user-supplied arguments before invoking the translation
    /// engine.
    ///
    /// Note: presence of a target language is validated by the command
    /// handler after combining CLI input with configured defaults; this
    /// function only validates that an explicitly provided `--target-language`
    /// is non-empty, along with the other guidance options.
    ///
    /// # Errors
    ///
    /// Returns [`SubXError::CommandExecution`] when:
    ///
    /// - `--target-language` is provided but empty after trimming.
    /// - `--source-language` is provided but empty after trimming.
    /// - `--context` is provided but empty after trimming.
    /// - `--glossary` points to a path that does not exist or is not a file.
    pub fn validate(&self) -> Result<(), SubXError> {
        if self.force && self.replace {
            return Err(SubXError::CommandExecution(
                "--force/--overwrite cannot be used with --replace".to_string(),
            ));
        }

        if let Some(target) = &self.target_language {
            if target.trim().is_empty() {
                return Err(SubXError::CommandExecution(
                    "--target-language must not be empty".to_string(),
                ));
            }
        }

        if let Some(src) = &self.source_language {
            if src.trim().is_empty() {
                return Err(SubXError::CommandExecution(
                    "--source-language must not be empty when provided".to_string(),
                ));
            }
        }

        if let Some(context) = &self.context {
            if context.trim().is_empty() {
                return Err(SubXError::CommandExecution(
                    "--context must not be empty when provided".to_string(),
                ));
            }
        }

        if let Some(glossary) = &self.glossary {
            if !glossary.exists() {
                return Err(SubXError::CommandExecution(format!(
                    "Glossary file does not exist: {}",
                    glossary.display()
                )));
            }
            if !glossary.is_file() {
                return Err(SubXError::CommandExecution(format!(
                    "Glossary path is not a regular file: {}",
                    glossary.display()
                )));
            }
        }

        Ok(())
    }

    /// Build an [`InputPathHandler`] from positional and `-i` paths,
    /// scoped to the subtitle file extensions supported by the rest of the
    /// pipeline.
    pub fn get_input_handler(&self) -> Result<InputPathHandler, SubXError> {
        let string_paths: Vec<String> = self
            .paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        let merged = InputPathHandler::merge_paths_from_multiple_sources(
            &[],
            &self.input_paths,
            &string_paths,
        )?;
        Ok(InputPathHandler::from_args(&merged, self.recursive)?
            .with_extensions(&["srt", "ass", "vtt", "sub", "ssa"])
            .with_no_extract(self.no_extract))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;
    use std::path::PathBuf;

    fn parse(args: &[&str]) -> TranslateArgs {
        let cli = Cli::try_parse_from(args).expect("parse should succeed");
        match cli.command {
            Commands::Translate(a) => a,
            _ => panic!("Expected Translate command"),
        }
    }

    #[test]
    fn test_target_language_optional_at_clap_level() {
        // Clap allows omission; presence is enforced by command handler so
        // the configured default can take effect.
        let args = parse(&["subx-cli", "translate", "movie.srt"]);
        assert!(args.target_language.is_none());
    }

    #[test]
    fn test_basic_invocation_parses_target_language() {
        let args = parse(&[
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
        assert!(!args.recursive);
        assert!(!args.no_extract);
        assert!(!args.force);
        assert!(!args.replace);
    }

    #[test]
    fn test_repeated_input_and_optional_flags() {
        let args = parse(&[
            "subx-cli",
            "translate",
            "-i",
            "d1",
            "-i",
            "f.srt",
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
            vec![PathBuf::from("d1"), PathBuf::from("f.srt")]
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
    fn test_validate_rejects_empty_target_language() {
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
        assert!(args.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_empty_context() {
        let args = TranslateArgs {
            paths: vec![PathBuf::from("a.srt")],
            input_paths: vec![],
            recursive: false,
            target_language: Some("ja".to_string()),
            source_language: None,
            glossary: None,
            context: Some("   ".to_string()),
            output: None,
            no_extract: false,
            force: false,
            replace: false,
        };
        assert!(args.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_missing_glossary() {
        let args = TranslateArgs {
            paths: vec![PathBuf::from("a.srt")],
            input_paths: vec![],
            recursive: false,
            target_language: Some("ja".to_string()),
            source_language: None,
            glossary: Some(PathBuf::from(
                "/nonexistent_subx_translate_glossary_file.txt",
            )),
            context: None,
            output: None,
            no_extract: false,
            force: false,
            replace: false,
        };
        let err = args.validate().expect_err("missing glossary should fail");
        let msg = format!("{err:?}");
        assert!(msg.contains("Glossary"), "unexpected error: {msg}");
    }

    #[test]
    fn test_validate_accepts_no_glossary() {
        let args = TranslateArgs {
            paths: vec![PathBuf::from("a.srt")],
            input_paths: vec![],
            recursive: false,
            target_language: Some("zh-TW".to_string()),
            source_language: Some("en".to_string()),
            glossary: None,
            context: Some("Tone".to_string()),
            output: None,
            no_extract: false,
            force: false,
            replace: false,
        };
        assert!(args.validate().is_ok());
    }
}
