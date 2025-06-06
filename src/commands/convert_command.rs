use crate::cli::ConvertArgs;
use crate::core::formats::converter::{ConversionConfig, FormatConverter};

/// 執行格式轉換命令
pub async fn execute(args: ConvertArgs) -> crate::Result<()> {
    let config = ConversionConfig {
        preserve_styling: true, // 從配置讀取
        target_encoding: args.encoding.clone(),
        keep_original: args.keep_original,
        validate_output: true,
    };
    let converter = FormatConverter::new(config);

    if args.input.is_file() {
        // 單檔案轉換
        let output_path = args
            .output
            .unwrap_or_else(|| args.input.with_extension(args.format.to_string()));
        let result = converter
            .convert_file(&args.input, &output_path, &args.format.to_string())
            .await?;
        if result.success {
            println!(
                "✓ 轉換完成: {} -> {}",
                args.input.display(),
                output_path.display()
            );
        } else {
            println!("✗ 轉換失敗");
            for error in result.errors {
                println!("  錯誤: {}", error);
            }
        }
    } else {
        // 批量轉換
        let results = converter
            .convert_batch(&args.input, &args.format.to_string(), true)
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
