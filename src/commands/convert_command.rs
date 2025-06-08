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
