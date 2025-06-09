use crate::Result;
use crate::core::formats::encoding::EncodingDetector;
use log::error;
use std::path::Path;

/// 檔案編碼檢測命令
pub fn detect_encoding_command(file_paths: &[String], verbose: bool) -> Result<()> {
    let detector = EncodingDetector::new()?;
    for file in file_paths {
        if !Path::new(file).exists() {
            error!("檔案不存在: {}", file);
            continue;
        }
        match detector.detect_file_encoding(file) {
            Ok(info) => {
                let name = Path::new(file)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(file);
                println!("檔案: {}", name);
                println!(
                    "  編碼: {:?} (信心度: {:.1}%) BOM: {}",
                    info.charset,
                    info.confidence * 100.0,
                    if info.bom_detected { "是" } else { "否" }
                );
                let sample = if verbose {
                    info.sample_text.clone()
                } else if info.sample_text.len() > 50 {
                    format!("{}...", &info.sample_text[..47])
                } else {
                    info.sample_text.clone()
                };
                println!("  樣本文字: {}\n", sample);
            }
            Err(e) => error!("檢測 {} 編碼失敗: {}", file, e),
        }
    }
    Ok(())
}
