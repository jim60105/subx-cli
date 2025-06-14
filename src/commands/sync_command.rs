//! 重構後的同步命令，支援新的多方法同步引擎

use crate::cli::SyncArgs;
use crate::config::ConfigService;
use crate::core::formats::manager::FormatManager;
use crate::core::sync::{SyncEngine, SyncMethod, SyncResult};
use crate::{Result, error::SubXError};

/// 執行同步命令
pub async fn execute(args: SyncArgs, config_service: &dyn ConfigService) -> Result<()> {
    // 使用內建驗證方法
    if let Err(msg) = args.validate() {
        return Err(SubXError::CommandExecution(msg));
    }

    let config = config_service.get_config()?;

    // 建立同步引擎
    let sync_engine = SyncEngine::new(config.sync.clone(), config_service).await?;

    // 載入字幕檔案
    let format_manager = FormatManager::new();
    let mut subtitle = format_manager.load_subtitle(&args.subtitle)?;

    if args.verbose {
        println!("🎬 載入字幕檔案: {}", args.subtitle.display());
        println!("📄 字幕條目數: {}", subtitle.entries.len());
    }

    let sync_result = if let Some(manual_offset) = args.offset {
        // 手動偏移模式
        if args.verbose {
            println!("⚙️  使用手動偏移: {:.3}s", manual_offset);
        }

        sync_engine.apply_manual_offset(&mut subtitle, manual_offset)?;

        // 建立手動偏移結果
        SyncResult {
            offset_seconds: manual_offset,
            confidence: 1.0,
            method_used: crate::core::sync::SyncMethod::Manual,
            correlation_peak: 0.0,
            processing_duration: std::time::Duration::ZERO,
            warnings: Vec::new(),
            additional_info: None,
        }
    } else {
        // 自動同步模式
        let method = determine_sync_method(&args, &config.sync.default_method)?;
        let video_path = args
            .video
            .as_ref()
            .ok_or_else(|| SubXError::config("Video path required for automatic sync"))?;

        if args.verbose {
            println!("🔍 開始同步分析...");
            println!("   方法: {:?}", method);
            println!("   分析窗口: {}s", args.window);
            println!("   視訊檔案: {}", video_path.display());
        }

        // 應用 CLI 配置覆蓋
        let mut sync_config = config.sync.clone();
        apply_cli_overrides(&mut sync_config, &args)?;

        let result = sync_engine
            .detect_sync_offset(video_path.as_path(), &subtitle, Some(method))
            .await?;

        if args.verbose {
            println!("✅ 分析完成:");
            println!("   檢測偏移: {:.3}s", result.offset_seconds);
            println!("   信心度: {:.1}%", result.confidence * 100.0);
            println!("   處理時間: {:?}", result.processing_duration);
        }

        // 應用檢測到的偏移
        if !args.dry_run {
            sync_engine.apply_manual_offset(&mut subtitle, result.offset_seconds)?;
        }

        result
    };

    // 顯示結果
    display_sync_result(&sync_result, args.verbose);

    // 儲存結果（除非是 dry run）
    if !args.dry_run {
        let output_path = args.get_output_path();

        // 檢查檔案是否存在且沒有 force 標記
        if output_path.exists() && !args.force {
            return Err(SubXError::CommandExecution(format!(
                "輸出檔案已存在: {}. 使用 --force 覆蓋。",
                output_path.display()
            )));
        }

        format_manager.save_subtitle(&subtitle, &output_path)?;

        if args.verbose {
            println!("💾 同步後字幕已儲存至: {}", output_path.display());
        } else {
            println!("同步後字幕已儲存至: {}", output_path.display());
        }
    } else {
        println!("🔍 乾跑模式 - 未儲存檔案");
    }

    Ok(())
}

/// Maintain consistency with other commands
pub async fn execute_with_config(
    args: SyncArgs,
    config_service: std::sync::Arc<dyn ConfigService>,
) -> Result<()> {
    execute(args, config_service.as_ref()).await
}

fn determine_sync_method(args: &SyncArgs, default_method: &str) -> Result<SyncMethod> {
    // 如果 CLI 指定了方法，使用它
    if let Some(ref method_arg) = args.method {
        return Ok(method_arg.clone().into());
    }

    // 否則使用配置檔案中的預設方法
    match default_method {
        "whisper" => Ok(SyncMethod::WhisperApi),
        "vad" => Ok(SyncMethod::LocalVad),
        "auto" => Ok(SyncMethod::Auto),
        _ => Ok(SyncMethod::Auto),
    }
}

fn apply_cli_overrides(config: &mut crate::config::SyncConfig, args: &SyncArgs) -> Result<()> {
    // 更新分析時間窗口
    if args.window != 30 {
        config.analysis_window_seconds = args.window;
    }

    // 應用 Whisper 特定覆蓋
    if let Some(ref model) = args.whisper_model {
        config.whisper.model = model.clone();
    }
    if let Some(ref language) = args.whisper_language {
        config.whisper.language = language.clone();
    }
    if let Some(temperature) = args.whisper_temperature {
        config.whisper.temperature = temperature;
    }

    // 應用 VAD 特定覆蓋
    if let Some(sensitivity) = args.vad_sensitivity {
        config.vad.sensitivity = sensitivity;
    }
    if let Some(chunk_size) = args.vad_chunk_size {
        config.vad.chunk_size = chunk_size;
    }

    Ok(())
}

fn display_sync_result(result: &SyncResult, verbose: bool) {
    if verbose {
        println!("\n=== 同步結果 ===");
        println!("使用方法: {:?}", result.method_used);
        println!("檢測偏移: {:.3} 秒", result.offset_seconds);
        println!("信心度: {:.1}%", result.confidence * 100.0);
        println!("處理時間: {:?}", result.processing_duration);

        if !result.warnings.is_empty() {
            println!("\n警告:");
            for warning in &result.warnings {
                println!("  ⚠️  {}", warning);
            }
        }

        if let Some(info) = &result.additional_info {
            if let Ok(pretty_info) = serde_json::to_string_pretty(info) {
                println!("\n額外資訊:");
                println!("{}", pretty_info);
            }
        }
    } else {
        println!(
            "✅ 同步完成: 偏移 {:.3}s (信心度: {:.1}%)",
            result.offset_seconds,
            result.confidence * 100.0
        );
    }
}
