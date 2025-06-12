//! Config 系統整合測試：ProductionConfigService 檔案 I/O 與 reset_to_defaults

#[path = "common/file_managers.rs"]
mod file_managers;
use file_managers::TestFileManager;
use std::sync::Arc;
use subx_cli::config::{
    ConfigService, ProductionConfigService, TestConfigBuilder, TestEnvironmentProvider,
};
use toml;

/// 測試從自訂檔案載入配置
#[tokio::test]
async fn test_load_config_from_custom_file() {
    let mut fm = TestFileManager::new();
    let dir = fm.create_isolated_test_directory("cfg_load").await.unwrap();
    let path = dir.join("custom.toml");

    // 寫入自訂配置檔
    let custom = TestConfigBuilder::new()
        .with_ai_provider("anthropic")
        .build_config();
    let toml = toml::to_string_pretty(&custom).unwrap();
    tokio::fs::write(&path, toml).await.unwrap();

    // 透過 ProductionConfigService with_custom_file 載入
    let svc = ProductionConfigService::new()
        .unwrap()
        .with_custom_file(path.clone())
        .unwrap();
    let loaded = svc.get_config().unwrap();
    assert_eq!(loaded.ai.provider, "anthropic");
}

/// 測試 reset_to_defaults 會覆寫配置檔為預設內容
#[tokio::test]
async fn test_reset_to_defaults_writes_default() {
    let mut fm = TestFileManager::new();
    let dir = fm
        .create_isolated_test_directory("cfg_reset")
        .await
        .unwrap();
    let path = dir.join("cfg_reset.toml");

    // 寫入無效或自定義內容
    tokio::fs::write(&path, "invalid content").await.unwrap();

    // 透過 SUBX_CONFIG_PATH 指向測試檔案
    let mut env = TestEnvironmentProvider::new();
    env.set_var("SUBX_CONFIG_PATH", path.to_str().unwrap());
    let svc = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();
    svc.reset_to_defaults().unwrap();

    let content = tokio::fs::read_to_string(&path).await.unwrap();
    // 預設配置應包含 ai.provider 項目
    assert!(
        content.contains("provider = \"openai\""),
        "reset 文件內容不正確: {}",
        content
    );
    // 確認載入為預設值
    let parsed: subx_cli::config::Config = toml::from_str(&content).unwrap();
    assert_eq!(
        parsed.ai.provider,
        subx_cli::config::Config::default().ai.provider
    );
}
