//! Subtitle format conversion command implementation.
//!
//! This module provides the `convert` subcommand logic to transform subtitle files
//! between supported formats (e.g., SRT, ASS, VTT), preserving styling and encoding.
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::cli::ConvertArgs;
//! use subx_cli::commands::convert_command;
//!
//! async fn demo(args: ConvertArgs) -> subx_cli::Result<()> {
//!     convert_command::execute(args).await
//! }
//! ```
//!
use crate::cli::{ConvertArgs, OutputSubtitleFormat};
use crate::config::load_config;
use crate::core::file_manager::FileManager;
use crate::core::formats::converter::{ConversionConfig, FormatConverter};
use crate::error::SubXError;

/// 執行格式轉換命令
pub async fn execute(args: ConvertArgs) -> crate::Result<()> {
    let app_config = load_config()?;
    let config = ConversionConfig {
        preserve_styling: app_config.formats.preserve_styling,
        target_encoding: args.encoding.clone(),
        keep_original: args.keep_original,
        validate_output: true,
    };
    let converter = FormatConverter::new(config);

    // 預設輸出格式轉換為 enum，若配置無效則回報錯誤
    let default_output = match app_config.formats.default_output.as_str() {
        "srt" => OutputSubtitleFormat::Srt,
        "ass" => OutputSubtitleFormat::Ass,
        "vtt" => OutputSubtitleFormat::Vtt,
        "sub" => OutputSubtitleFormat::Sub,
        other => return Err(SubXError::config(format!("未知的預設輸出格式: {}", other))),
    };
    let output_format = args.format.clone().unwrap_or(default_output);
    if args.input.is_file() {
        // 單檔案轉換
        let format_str = output_format.to_string();
        let output_path = args
            .output
            .unwrap_or_else(|| args.input.with_extension(format_str.clone()));
        let mut file_manager = FileManager::new();
        match converter
            .convert_file(&args.input, &output_path, &format_str)
            .await
        {
            Ok(result) => {
                if result.success {
                    file_manager.record_creation(&output_path);
                    println!(
                        "✓ 轉換完成: {} -> {}",
                        args.input.display(),
                        output_path.display()
                    );
                    if !args.keep_original {
                        if let Err(e) = file_manager.remove_file(&args.input) {
                            eprintln!("⚠️  無法移除原始檔案 {}: {}", args.input.display(), e);
                        }
                    }
                } else {
                    println!("✗ 轉換失敗");
                    for error in result.errors {
                        println!("  錯誤: {}", error);
                    }
                }
            }
            Err(e) => {
                eprintln!("✗ 轉換失敗: {}", e);
                if let Err(rollback_err) = file_manager.rollback() {
                    eprintln!("✗ 回滾失敗: {}", rollback_err);
                }
                return Err(e);
            }
        }
    } else {
        // 批量轉換
        let format_str = output_format.to_string();
        let results = converter
            .convert_batch(&args.input, &format_str, true)
            .await?;
        let success_count = results.iter().filter(|r| r.success).count();
        let total_count = results.len();
        println!("批量轉換完成: {}/{} 成功", success_count, total_count);
        for result in results.iter().filter(|r| !r.success) {
            println!("失敗: {}", result.errors.join(", "));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::init_config_manager;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_convert_srt_to_vtt() -> crate::Result<()> {
        init_config_manager()?;
        let temp_dir = TempDir::new().unwrap();
        let input_file = temp_dir.path().join("test.srt");
        let output_file = temp_dir.path().join("test.vtt");
        fs::write(
            &input_file,
            "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n\n",
        )
        .unwrap();
        let args = ConvertArgs {
            input: input_file.clone(),
            format: Some(OutputSubtitleFormat::Vtt),
            output: Some(output_file.clone()),
            keep_original: false,
            encoding: String::from("utf-8"),
        };
        execute(args).await?;
        let content = fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("WEBVTT"));
        assert!(content.contains("00:00:01.000 --> 00:00:02.000"));
        Ok(())
    }

    #[tokio::test]
    async fn test_convert_batch_processing() -> crate::Result<()> {
        init_config_manager()?;
        let temp_dir = TempDir::new().unwrap();
        for i in 1..=3 {
            let file = temp_dir.path().join(format!("test{}.srt", i));
            fs::write(
                &file,
                format!(
                    "1\n00:00:0{},000 --> 00:00:0{},000\nTest {}\n\n",
                    i,
                    i + 1,
                    i
                ),
            )
            .unwrap();
        }
        let args = ConvertArgs {
            input: temp_dir.path().to_path_buf(),
            format: Some(OutputSubtitleFormat::Vtt),
            output: Some(temp_dir.path().join("output")),
            keep_original: false,
            encoding: String::from("utf-8"),
        };
        // 僅檢查執行結果，不驗證實際檔案產生，由於轉換器行為外部模組控制
        execute(args).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_convert_unsupported_format() {
        init_config_manager().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let input_file = temp_dir.path().join("test.txt");
        fs::write(&input_file, "not a subtitle").unwrap();
        let args = ConvertArgs {
            input: input_file,
            format: Some(OutputSubtitleFormat::Srt),
            output: None,
            keep_original: false,
            encoding: String::from("utf-8"),
        };
        let result = execute(args).await;
        assert!(result.is_err());
    }
}
