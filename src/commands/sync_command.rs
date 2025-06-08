use crate::cli::SyncArgs;
use crate::config::load_config;
use crate::core::formats::manager::FormatManager;
use crate::core::formats::Subtitle;
use crate::core::matcher::{FileDiscovery, MediaFileType};
use crate::core::sync::{SyncConfig, SyncEngine, SyncResult};
use crate::error::SubXError;
use crate::Result;
use std::path::{Path, PathBuf};

/// 執行 Sync 命令
pub async fn execute(args: SyncArgs) -> Result<()> {
    let app_config = load_config()?;
    let config = SyncConfig {
        max_offset_seconds: args.range.unwrap_or(app_config.sync.max_offset_seconds),
        correlation_threshold: args
            .threshold
            .unwrap_or(app_config.sync.correlation_threshold),
        dialogue_threshold: app_config.sync.dialogue_detection_threshold,
        min_dialogue_length: app_config.sync.min_dialogue_duration_ms as f32 / 1000.0,
    };
    let sync_engine = SyncEngine::new(config);

    if let Some(manual_offset) = args.offset {
        let mut subtitle = load_subtitle(&args.subtitle).await?;
        sync_engine.apply_sync_offset(&mut subtitle, manual_offset as f32)?;
        save_subtitle(&subtitle, &args.subtitle).await?;
        println!("✓ 已應用手動偏移: {}秒", manual_offset);
    } else if args.batch {
        let media_pairs = discover_media_pairs(&args.video).await?;
        for (video_file, subtitle_file) in media_pairs {
            match sync_single_pair(&sync_engine, &video_file, &subtitle_file).await {
                Ok(result) => {
                    println!(
                        "✓ {} - 偏移: {:.2}秒 (信心度: {:.2})",
                        subtitle_file.display(),
                        result.offset_seconds,
                        result.confidence
                    );
                }
                Err(e) => {
                    println!("✗ {} - 錯誤: {}", subtitle_file.display(), e);
                }
            }
        }
    } else {
        let subtitle = load_subtitle(&args.subtitle).await?;
        let result = sync_engine.sync_subtitle(&args.video, &subtitle).await?;
        if result.confidence > 0.5 {
            let mut updated = subtitle;
            sync_engine.apply_sync_offset(&mut updated, result.offset_seconds)?;
            save_subtitle(&updated, &args.subtitle).await?;
            println!(
                "✓ 同步完成 - 偏移: {:.2}秒 (信心度: {:.2})",
                result.offset_seconds, result.confidence
            );
        } else {
            println!("⚠ 同步信心度較低 ({:.2})，建議手動調整", result.confidence);
        }
    }
    Ok(())
}

/// 載入字幕檔案並解析
async fn load_subtitle(path: &Path) -> Result<Subtitle> {
    let content = tokio::fs::read_to_string(path).await?;
    let mgr = FormatManager::new();
    let mut subtitle = mgr.parse_auto(&content)?;
    // 設定來源編碼
    subtitle.metadata.encoding = "utf-8".to_string();
    Ok(subtitle)
}

/// 序列化並儲存字幕檔案
async fn save_subtitle(subtitle: &Subtitle, path: &Path) -> Result<()> {
    let mgr = FormatManager::new();
    let text = mgr
        .get_format_by_extension(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default(),
        )
        .ok_or_else(|| SubXError::subtitle_format("Unknown", "未知的字幕格式"))?
        .serialize(subtitle)?;
    tokio::fs::write(path, text).await?;
    Ok(())
}

/// 掃描資料夾並配對影片與字幕檔案
async fn discover_media_pairs(dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(dir, true)?;
    let videos: Vec<_> = files
        .iter()
        .filter(|f| matches!(f.file_type, MediaFileType::Video))
        .cloned()
        .collect();
    let subs: Vec<_> = files
        .iter()
        .filter(|f| matches!(f.file_type, MediaFileType::Subtitle))
        .cloned()
        .collect();
    let mut pairs = Vec::new();
    for video in videos {
        if let Some(s) = subs.iter().find(|s| s.name == video.name) {
            pairs.push((video.path.clone(), s.path.clone()));
        }
    }
    Ok(pairs)
}

/// 同步單一媒體檔案
async fn sync_single_pair(
    engine: &SyncEngine,
    video: &Path,
    subtitle_path: &Path,
) -> Result<SyncResult> {
    let mut subtitle = load_subtitle(subtitle_path).await?;
    let result = engine.sync_subtitle(video, &subtitle).await?;
    engine.apply_sync_offset(&mut subtitle, result.offset_seconds)?;
    save_subtitle(&subtitle, subtitle_path).await?;
    Ok(result)
}
