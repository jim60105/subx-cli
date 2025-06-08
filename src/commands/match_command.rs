use crate::cli::display_match_results;
use crate::cli::MatchArgs;
use crate::config::Config;
use crate::core::matcher::{MatchConfig, MatchEngine};
use crate::error::SubXError;
use crate::services::ai::{AIProvider, OpenAIClient};
use crate::Result;

/// 執行 Match 命令，支援 Dry-run 與實際操作，並允許注入 AI 服務以便測試
pub async fn execute(args: MatchArgs) -> Result<()> {
    // 載入配置與建立 AI 客戶端
    let config = Config::load()?;
    let api_key = config
        .ai
        .api_key
        .clone()
        .ok_or_else(|| SubXError::config("需要設定 OPENAI API 金鑰"))?;
    let ai_client: Box<dyn AIProvider> = Box::new(OpenAIClient::new(
        api_key,
        config.ai.model.clone(),
        config.ai.temperature,
        config.ai.retry_attempts,
        config.ai.retry_delay_ms,
    ));
    execute_with_client(args, ai_client).await
}

/// 執行 Match 流程，支援 Dry-run 與實際操作，AI 客戶端由外部注入
pub async fn execute_with_client(args: MatchArgs, ai_client: Box<dyn AIProvider>) -> Result<()> {
    // 載入配置並初始化匹配引擎
    let config = Config::load()?;
    let match_config = MatchConfig {
        confidence_threshold: args.confidence as f32 / 100.0,
        max_sample_length: config.ai.max_sample_length,
        // 永遠進行內容分析，以便 Dry-run 時也能產生並快取結果
        enable_content_analysis: true,
        backup_enabled: args.backup || config.general.backup_enabled,
    };
    let engine = MatchEngine::new(ai_client, match_config);

    // 執行匹配運算
    let operations = engine.match_files(&args.path, args.recursive).await?;

    // 顯示對映結果表格
    display_match_results(&operations, args.dry_run);

    if args.dry_run {
        engine
            .save_cache(&args.path, args.recursive, &operations)
            .await?;
    } else {
        // 執行檔案操作
        engine.execute_operations(&operations, args.dry_run).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::execute_with_client;
    use crate::cli::MatchArgs;
    use crate::services::ai::{
        AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
    };
    use async_trait::async_trait;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    struct DummyAI;
    #[async_trait]
    impl AIProvider for DummyAI {
        async fn analyze_content(&self, _req: AnalysisRequest) -> crate::Result<MatchResult> {
            Ok(MatchResult {
                matches: Vec::new(),
                confidence: 0.0,
                reasoning: String::new(),
            })
        }
        async fn verify_match(&self, _req: VerificationRequest) -> crate::Result<ConfidenceScore> {
            panic!("verify_match should not be called in dry-run test");
        }
    }

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

        // 指定快取路徑到臨時資料夾
        std::env::set_var("XDG_CONFIG_HOME", media_dir.path());

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
        execute_with_client(args, Box::new(DummyAI)).await?;

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
