use crate::cli::MatchArgs;
use crate::config::Config;
use crate::core::matcher::{MatchConfig, MatchEngine};
use crate::error::SubXError;
use crate::services::ai::OpenAIClient;
use crate::Result;

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

    // 執行匹配 (Dry-run 同時快取結果)
    let operations = engine.match_files(&args.path, args.recursive).await?;
    if args.dry_run {
        engine
            .save_cache(&args.path, args.recursive, &operations)
            .await?;
    }

    // 執行檔案操作或預覽
    engine.execute_operations(&operations, args.dry_run).await?;
    Ok(())
}
