//! Subtitle translation command implementation.
//!
//! This module wires CLI argument validation, configuration loading, input
//! collection, safe output resolution, and translation engine invocation.
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::cli::TranslateArgs;
//! use subx_cli::commands::translate_command;
//! use subx_cli::config::TestConfigService;
//!
//! # async fn example() -> subx_cli::Result<()> {
//! let args = TranslateArgs {
//!     paths: vec!["movie.srt".into()],
//!     input_paths: vec![],
//!     recursive: false,
//!     target_language: Some("zh-TW".to_string()),
//!     source_language: None,
//!     glossary: None,
//!     context: None,
//!     output: None,
//!     no_extract: false,
//!     force: false,
//!     replace: false,
//! };
//! let config_service = TestConfigService::with_defaults();
//! translate_command::execute(args, &config_service).await?;
//! # Ok(())
//! # }
//! ```

use crate::cli::TranslateArgs;
use std::path::{Path, PathBuf};

use crate::config::ConfigService;
use crate::core::ComponentFactory;
use crate::core::translation::{TranslationRequest, parse_glossary_text};
use crate::error::SubXError;

/// Resolved translate command inputs after CLI/config defaulting.
///
/// This struct is filled in by [`execute`] before delegating to the
/// translation engine implemented in the `core` slice. It is exposed so the
/// engine slice can consume a stable, validated value type.
#[derive(Debug, Clone)]
pub struct TranslateExecution {
    /// Validated CLI arguments.
    pub args: TranslateArgs,
    /// Effective target language (`args.target_language` if non-empty,
    /// otherwise [`crate::config::TranslationConfig::default_target_language`]).
    pub target_language: String,
    /// Configured translation batch size.
    pub batch_size: usize,
}

struct ResolvedOutput {
    path: PathBuf,
    replaces_source: bool,
}

/// Resolve the effective target language using CLI > config precedence.
///
/// CLI `--target-language` always wins when non-empty. Otherwise, the
/// configured `translation.default_target_language` is used. If neither is
/// available, the function returns an error so the caller can surface a
/// usage-style failure before any AI request is sent.
fn resolve_target_language(
    args: &TranslateArgs,
    default: Option<&str>,
) -> Result<String, SubXError> {
    if let Some(cli) = args.target_language.as_deref() {
        let trimmed = cli.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    match default {
        Some(d) if !d.trim().is_empty() => Ok(d.trim().to_string()),
        _ => Err(SubXError::CommandExecution(
            "No target language provided. Pass --target-language or set \
             translation.default_target_language in the configuration."
                .to_string(),
        )),
    }
}

/// Execute the `translate` command.
///
/// # Arguments
///
/// * `args` - Parsed CLI arguments.
/// * `config_service` - Configuration service providing translation defaults.
///
/// # Errors
///
/// Returns an error if argument validation, input collection, AI translation,
/// or output writing fails.
pub async fn execute(args: TranslateArgs, config_service: &dyn ConfigService) -> crate::Result<()> {
    args.validate()
        .map_err(|e| SubXError::CommandExecution(e.to_string()))?;

    let config = config_service.get_config()?;
    let target_language =
        resolve_target_language(&args, config.translation.default_target_language.as_deref())?;

    let execution = TranslateExecution {
        args: args.clone(),
        target_language: target_language.clone(),
        batch_size: config.translation.batch_size,
    };

    let handler = execution
        .args
        .get_input_handler()
        .map_err(|e| SubXError::CommandExecution(e.to_string()))?;
    let collected = handler
        .collect_files()
        .map_err(|e| SubXError::CommandExecution(e.to_string()))?;
    if collected.is_empty() {
        return Ok(());
    }

    let glossary_text = match &execution.args.glossary {
        Some(path) => Some(std::fs::read_to_string(path).map_err(|e| {
            SubXError::FileOperationFailed(format!(
                "Failed to read glossary file {}: {e}",
                path.display()
            ))
        })?),
        None => None,
    };
    let glossary_entries = glossary_text
        .as_deref()
        .map(parse_glossary_text)
        .unwrap_or_default();

    let factory = ComponentFactory::new(config_service)?;
    let engine = factory.create_translation_engine()?;
    let mut failures = Vec::new();

    for input_path in collected.iter() {
        let output = match resolve_output_path(
            input_path,
            &collected,
            &execution.args,
            &execution.target_language,
        ) {
            Ok(output) => output,
            Err(err) => {
                eprintln!(
                    "✗ Translation setup failed for {}: {}",
                    input_path.display(),
                    err
                );
                failures.push(format!("{}: {err}", input_path.display()));
                continue;
            }
        };

        if let Err(err) = translate_one_file(
            &engine,
            input_path,
            &output,
            &execution,
            glossary_text.clone(),
            glossary_entries.clone(),
            config.general.backup_enabled,
        )
        .await
        {
            eprintln!("✗ Translation failed for {}: {}", input_path.display(), err);
            failures.push(format!("{}: {err}", input_path.display()));
        } else {
            println!(
                "✓ Translation completed: {} -> {}",
                input_path.display(),
                output.path.display()
            );
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(SubXError::CommandExecution(format!(
            "{} translation job(s) failed: {}",
            failures.len(),
            failures.join("; ")
        )))
    }
}

/// Execute the `translate` command with an owned configuration service.
///
/// Mirrors the convention used by the other subcommands so the dispatcher
/// can route both `Arc<dyn ConfigService>` and `&dyn ConfigService` paths.
pub async fn execute_with_config(
    args: TranslateArgs,
    config_service: std::sync::Arc<dyn ConfigService>,
) -> crate::Result<()> {
    execute(args, config_service.as_ref()).await
}

async fn translate_one_file(
    engine: &crate::core::translation::TranslationEngine,
    input_path: &Path,
    output: &ResolvedOutput,
    execution: &TranslateExecution,
    glossary_text: Option<String>,
    glossary_entries: Vec<crate::core::translation::GlossaryEntry>,
    backup_enabled: bool,
) -> crate::Result<()> {
    if output.path.exists() && !execution.args.force && !output.replaces_source {
        return Err(SubXError::FileAlreadyExists(
            output.path.display().to_string(),
        ));
    }

    let subtitle = engine.format_manager().load_subtitle(input_path)?;
    let request = TranslationRequest {
        target_language: execution.target_language.clone(),
        source_language: execution.args.source_language.clone(),
        glossary_text,
        context: execution.args.context.clone(),
        glossary_entries,
    };
    let result = engine.translate_subtitle(subtitle, &request).await?;

    if output.replaces_source && backup_enabled {
        let backup = backup_path(input_path);
        std::fs::copy(input_path, &backup).map_err(|e| {
            SubXError::FileOperationFailed(format!(
                "Failed to create backup {}: {e}",
                backup.display()
            ))
        })?;
    }

    if let Some(parent) = output.path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            SubXError::FileOperationFailed(format!(
                "Failed to create output directory {}: {e}",
                parent.display()
            ))
        })?;
    }
    engine
        .format_manager()
        .save_subtitle(&result.subtitle, &output.path)
}

fn resolve_output_path(
    input_path: &Path,
    collected: &crate::cli::CollectedFiles,
    args: &TranslateArgs,
    target_language: &str,
) -> crate::Result<ResolvedOutput> {
    if args.replace {
        if collected.archive_origin(input_path).is_some() {
            return Err(SubXError::CommandExecution(
                "--replace cannot be used for subtitles extracted from archives".to_string(),
            ));
        }
        return Ok(ResolvedOutput {
            path: input_path.to_path_buf(),
            replaces_source: true,
        });
    }

    let path = match &args.output {
        Some(output) => explicit_output_path(output, input_path, collected.len(), target_language)?,
        None => default_output_path(
            input_path,
            collected.archive_origin(input_path),
            target_language,
        ),
    };
    Ok(ResolvedOutput {
        path,
        replaces_source: false,
    })
}

fn explicit_output_path(
    output: &Path,
    input_path: &Path,
    input_count: usize,
    target_language: &str,
) -> crate::Result<PathBuf> {
    if input_count > 1 {
        if output.exists() && !output.is_dir() {
            return Err(SubXError::CommandExecution(format!(
                "Batch translation output must be a directory: {}",
                output.display()
            )));
        }
        if output.extension().is_some() {
            return Err(SubXError::CommandExecution(format!(
                "Batch translation output must be a directory: {}",
                output.display()
            )));
        }
        return Ok(output.join(translated_file_name(input_path, target_language)));
    }

    if output.is_dir() {
        Ok(output.join(translated_file_name(input_path, target_language)))
    } else {
        Ok(output.to_path_buf())
    }
}

fn default_output_path(
    input_path: &Path,
    archive_origin: Option<&Path>,
    target_language: &str,
) -> PathBuf {
    let base_dir = archive_origin
        .and_then(Path::parent)
        .or_else(|| input_path.parent())
        .unwrap_or_else(|| Path::new("."));
    base_dir.join(translated_file_name(input_path, target_language))
}

fn translated_file_name(input_path: &Path, target_language: &str) -> String {
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("subtitle");
    let ext = input_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("srt");
    format!("{stem}.{target_language}.{ext}")
}

fn backup_path(input_path: &Path) -> PathBuf {
    let ext = input_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if ext.is_empty() {
        input_path.with_extension("backup")
    } else {
        input_path.with_extension(format!("{ext}.backup"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TestConfigBuilder;
    use std::path::PathBuf;

    fn base_args() -> TranslateArgs {
        TranslateArgs {
            paths: vec![PathBuf::from("nonexistent.srt")],
            input_paths: vec![],
            recursive: false,
            target_language: Some("zh-TW".to_string()),
            source_language: None,
            glossary: None,
            context: None,
            output: None,
            no_extract: false,
            force: false,
            replace: false,
        }
    }

    #[tokio::test]
    async fn test_validation_runs_before_execution() {
        let mut args = base_args();
        args.target_language = Some("   ".to_string());
        let config_service = TestConfigBuilder::new().build_service();
        let err = execute(args, &config_service)
            .await
            .expect_err("empty target language must fail");
        let msg = format!("{err:?}");
        assert!(msg.contains("target-language"), "unexpected error: {msg}");
    }

    #[tokio::test]
    async fn test_uses_configured_default_target_language() {
        let mut args = base_args();
        args.target_language = None;
        let config_service = TestConfigBuilder::new()
            .with_translation_default_target_language("ja")
            .build_service();
        // The configured default should satisfy target-language resolution.
        // This fixture uses a nonexistent input, so the command fails later
        // during input collection instead of failing target-language resolution.
        let err = execute(args, &config_service)
            .await
            .expect_err("input collection error");
        let msg = format!("{err:?}");
        assert!(msg.contains("Path not found"), "unexpected: {msg}");
    }

    #[tokio::test]
    async fn test_missing_default_and_cli_target_language_fails() {
        let mut args = base_args();
        args.target_language = None;
        let config_service = TestConfigBuilder::new().build_service();
        let err = execute(args, &config_service)
            .await
            .expect_err("no target language must fail");
        let msg = format!("{err:?}");
        assert!(msg.contains("No target language"), "unexpected: {msg}");
    }
}
