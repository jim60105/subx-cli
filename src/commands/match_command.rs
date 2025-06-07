use crate::cli::MatchArgs;
use crate::config::Config;
use crate::core::matcher::{MatchConfig, MatchEngine};
use crate::error::SubXError;
use crate::services::ai::OpenAIClient;
use crate::Result;
use colored::Colorize;

/// 執行 Match 命令，支援 Dry-run 與實際操作
pub async fn execute(args: MatchArgs) -> Result<()> {
    // 載入配置
    let config = Config::load()?;
    let api_key = config
        .ai
        .api_key
        .clone()
        .ok_or_else(|| SubXError::config("需要設定 OPENAI API 金鑰"))?;

    // 建立匹配引擎
    let match_config = MatchConfig {
        confidence_threshold: args.confidence as f32 / 100.0,
        max_sample_length: config.ai.max_sample_length,
        // Dry-run 模式下不執行內容分析以避免實際呼叫 AI 服務
        enable_content_analysis: !args.dry_run,
        backup_enabled: args.backup || config.general.backup_enabled,
    };
    let engine = MatchEngine::new(
        Box::new(OpenAIClient::new(api_key, config.ai.model.clone())),
        match_config,
    );

    // 若為預覽模式，顯示提示並建立空快取，不呼叫 AI 分析或執行檔案操作
    if args.dry_run {
        println!("\n{} 預覽模式 - 未實際執行操作", "ℹ".blue().bold());
        engine.save_cache(&args.path, args.recursive, &[]).await?;
        return Ok(());
    }

    // 執行匹配並執行檔案操作
    let operations = engine.match_files(&args.path, args.recursive).await?;
    engine.execute_operations(&operations, args.dry_run).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::execute;
    use crate::cli::MatchArgs;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    /// Dry-run 模式下應建立快取檔案，且不實際執行任何檔案操作
    #[tokio::test]
    async fn dry_run_creates_cache_and_skips_execute_operations() -> crate::Result<()> {
        // 建立臨時媒體資料夾並放入示意影片與字幕檔
        let media_dir = tempdir()?;
        let media_path = media_dir.path().join("media");
        fs::create_dir_all(&media_path)?;
        let video = media_path.join("video.mkv");
        let subtitle = media_path.join("subtitle.ass");
        fs::write(&video, b"dummy")?;
        fs::write(&subtitle, b"dummy")?;

        // 指定快取路徑到臨時資料夾，並設定 API 金鑰
        std::env::set_var("XDG_CONFIG_HOME", media_dir.path());
        std::env::set_var("OPENAI_API_KEY", "test_key");

        // 確認尚未產生快取檔案
        let cache_path = dirs::config_dir()
            .unwrap()
            .join("subx")
            .join("match_cache.json");
        assert!(!cache_path.exists(), "測試開始時不應存在快取檔案");

        // 執行 dry-run
        let args = MatchArgs {
            path: PathBuf::from(&media_path),
            dry_run: true,
            recursive: false,
            confidence: 80,
            backup: false,
        };
        execute(args).await?;

        // 驗證已建立快取檔案，且原始檔案未被移動或刪除
        assert!(cache_path.exists(), "dry_run 後應建立快取檔案");
        assert!(
            video.exists(),
            "dry_run 不應執行 execute_operations，影片檔仍須存在"
        );
        assert!(
            subtitle.exists(),
            "dry_run 不應執行 execute_operations，字幕檔仍須存在"
        );
        Ok(())
    }
}
