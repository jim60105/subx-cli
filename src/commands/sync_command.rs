//! 重構後的同步命令，支援新的多方法同步引擎

use crate::cli::SyncArgs;
use crate::config::ConfigService;
use crate::core::formats::manager::FormatManager;
use crate::core::sync::{SyncEngine, SyncMethod, SyncResult};
use crate::{Result, error::SubXError};
use std::path::Path;

/// 執行同步命令
pub async fn execute(args: SyncArgs, config_service: &dyn ConfigService) -> Result<()> {
    let config = config_service.get_config()?;

    // 建立同步引擎
    let sync_engine = SyncEngine::new(config.sync.clone(), config_service).await?;

    // 載入字幕檔案
    let format_manager = FormatManager::new();
    let mut subtitle = format_manager.load_subtitle(&args.subtitle)?;

    let sync_result = if let Some(manual_offset) = args.offset {
        // 手動偏移模式
        sync_engine.apply_manual_offset(&mut subtitle, manual_offset)?
    } else {
        // 自動同步模式
        let method = determine_sync_method(&args, &config.sync.default_method)?;
        sync_engine
            .detect_sync_offset(&args.video, &subtitle, Some(method))
            .await?
    };

    // 顯示結果
    display_sync_result(&sync_result);

    // 如果不是手動模式，應用檢測到的偏移
    if args.offset.is_none() {
        sync_engine.apply_manual_offset(&mut subtitle, sync_result.offset_seconds)?;
    }

    // 儲存同步後的字幕
    let output_path = determine_output_path(&args.subtitle)?;
    format_manager.save_subtitle(&subtitle, &output_path)?;

    println!("Synchronized subtitle saved to: {}", output_path.display());
    Ok(())
}

fn determine_sync_method(args: &SyncArgs, default_method: &str) -> Result<SyncMethod> {
    // 從命令列參數或配置中確定同步方法
    // 注意：這需要對應的 CLI 參數更新（在後續 backlog 中）
    match default_method {
        "whisper" => Ok(SyncMethod::WhisperApi),
        "vad" => Ok(SyncMethod::LocalVad),
        "auto" => Ok(SyncMethod::Auto),
        _ => Ok(SyncMethod::Auto),
    }
}

fn display_sync_result(result: &SyncResult) {
    println!("=== Synchronization Result ===");
    println!("Method used: {:?}", result.method_used);
    println!("Detected offset: {:.3} seconds", result.offset_seconds);
    println!("Confidence: {:.1}%", result.confidence * 100.0);
    println!("Processing time: {:?}", result.processing_duration);

    if !result.warnings.is_empty() {
        println!("\nWarnings:");
        for warning in &result.warnings {
            println!("  ⚠️  {}", warning);
        }
    }

    if let Some(info) = &result.additional_info {
        if let Ok(pretty_info) = serde_json::to_string_pretty(info) {
            println!("\nAdditional information:");
            println!("{}", pretty_info);
        }
    }
}

fn determine_output_path(input_path: &Path) -> Result<std::path::PathBuf> {
    // 建立同步後檔案的輸出路徑
    let mut output_path = input_path.to_path_buf();

    if let Some(stem) = input_path.file_stem().and_then(|s| s.to_str()) {
        if let Some(extension) = input_path.extension().and_then(|s| s.to_str()) {
            let new_filename = format!("{}_synced.{}", stem, extension);
            output_path.set_file_name(new_filename);
        }
    }

    Ok(output_path)
}
