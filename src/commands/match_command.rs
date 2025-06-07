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
        enable_content_analysis: true,
        backup_enabled: args.backup || config.general.backup_enabled,
    };
    let engine = MatchEngine::new(
        Box::new(OpenAIClient::new(api_key, config.ai.model.clone())),
        match_config,
    );

    // 若為預覽模式，顯示提示，但仍執行匹配並快取結果，僅跳過實際檔案操作
    if args.dry_run {
        println!("\n{} 預覽模式 - 未實際執行操作", "ℹ".blue().bold());
    }
    // 執行匹配 (Dry-run 同時快取結果)
    let operations = engine.match_files(&args.path, args.recursive).await?;
    if args.dry_run {
        engine
            .save_cache(&args.path, args.recursive, &operations)
            .await?;
        return Ok(());
    }

    // 執行檔案操作
    engine.execute_operations(&operations, args.dry_run).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::execute;
    use crate::cli::MatchArgs;
    use crate::config::Config;
    use crate::core::matcher::cache::CacheData;
    use md5;
    use serde_json;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use toml;

    #[tokio::test]
    async fn dry_run_creates_cache_file() -> crate::Result<()> {
        // 建立臨時媒體資料夾
        let media_dir = tempdir().unwrap();
        let media_path = media_dir.path().join("media");
        fs::create_dir_all(&media_path)?;

        // 將 XDG_CONFIG_HOME 指向臨時目錄，並設定 API 金鑰
        std::env::set_var("XDG_CONFIG_HOME", media_dir.path());
        std::env::set_var("OPENAI_API_KEY", "test_key");

        // 計算目前配置雜湊，用於產生相容的快取檔案
        let cfg = Config::load()?;
        let toml = toml::to_string(&cfg).unwrap();
        let config_hash = format!("{:x}", md5::compute(toml));

        // 準備快取檔案路徑與內容
        let cache_dir = dirs::config_dir().unwrap().join("subx");
        fs::create_dir_all(&cache_dir)?;
        let cache_path = cache_dir.join("match_cache.json");
        let cache_data = CacheData {
            cache_version: "1.0".to_string(),
            directory: media_path.to_string_lossy().to_string(),
            file_snapshot: Vec::new(),
            match_operations: Vec::new(),
            created_at: 0,
            ai_model_used: config_hash.clone(),
            config_hash: config_hash.clone(),
        };
        let json = serde_json::to_string_pretty(&cache_data).expect("序列化 CacheData 成功");
        fs::write(&cache_path, json)?;

        // 執行 dry-run
        let args = MatchArgs {
            path: PathBuf::from(&media_path),
            dry_run: true,
            recursive: false,
            confidence: 80,
            backup: false,
        };
        execute(args).await?;

        // 驗證快取檔案仍然存在
        assert!(cache_path.exists(), "快取檔案應在 dry_run 後建立");
        Ok(())
    }
}
