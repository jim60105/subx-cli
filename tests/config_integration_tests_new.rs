//! 配置系統整合測試

use subx_cli::config::{ConfigService, ProductionConfigService, TestConfigBuilder};

mod common;
use common::file_managers::TestFileManager;

#[test]
fn test_config_builder_basic_functionality() {
    // 測試配置建構器基本功能
    let config = TestConfigBuilder::new()
        .with_ai_provider("openai")
        .with_ai_model("gpt-4")
        .build_config();

    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4");
}

#[test]
fn test_config_builder_ai_settings() {
    // 測試 AI 相關設定
    let config = TestConfigBuilder::new()
        .with_ai_provider("anthropic")
        .with_ai_model("claude-3")
        .build_config();

    assert_eq!(config.ai.provider, "anthropic");
    assert_eq!(config.ai.model, "claude-3");
}

#[test]
fn test_config_service_interface() {
    // 測試配置服務介面
    let service = TestConfigBuilder::new()
        .with_ai_provider("test_provider")
        .with_ai_model("test_model")
        .build_service();

    let config = service.get_config().unwrap();
    assert_eq!(config.ai.provider, "test_provider");
    assert_eq!(config.ai.model, "test_model");

    // 測試重載功能
    assert!(service.reload().is_ok());
}

#[test]
fn test_production_config_service() {
    // 測試生產配置服務
    match ProductionConfigService::new() {
        Ok(service) => {
            let config = service.get_config().unwrap();
            // 驗證預設配置載入
            assert!(!config.ai.provider.is_empty());
        }
        Err(_) => {
            // 在測試環境中可能沒有配置檔案，這是正常的
        }
    }
}

#[test]
fn test_config_with_custom_settings() {
    // 測試自訂設定
    let config = TestConfigBuilder::new()
        .with_ai_provider("custom_provider")
        .build_config();

    assert_eq!(config.ai.provider, "custom_provider");
}

#[tokio::test]
async fn test_config_with_file_manager() {
    // 使用新的測試工具進行檔案管理測試
    let mut file_manager = TestFileManager::new();
    let test_dir = file_manager
        .create_isolated_test_directory("config_test")
        .await
        .unwrap();
    let test_dir_path = test_dir.to_path_buf();

    // 建立測試配置檔案
    let config_path = file_manager
        .create_test_config(&test_dir_path, "test_config.toml", &test_dir_path)
        .await
        .unwrap();

    assert!(config_path.exists());

    // 驗證配置檔案內容
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[general]"));
    assert!(content.contains("[ai]"));
    assert!(content.contains("[test]"));
}

#[test]
fn test_multiple_config_instances() {
    // 測試多個配置實例的隔離性
    let config1 = TestConfigBuilder::new()
        .with_ai_provider("provider1")
        .build_config();

    let config2 = TestConfigBuilder::new()
        .with_ai_provider("provider2")
        .build_config();

    // 確保配置實例間相互隔離
    assert_eq!(config1.ai.provider, "provider1");
    assert_eq!(config2.ai.provider, "provider2");
}

#[test]
fn test_config_builder_chaining() {
    // 測試配置建構器的鏈式調用
    let config = TestConfigBuilder::new()
        .with_ai_provider("openai")
        .with_ai_model("gpt-4")
        .build_config();

    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4");
}

#[test]
fn test_parallel_config_creation() {
    // 測試並行配置建立
    use std::thread;

    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                let config = TestConfigBuilder::new()
                    .with_ai_provider(&format!("provider_{}", i))
                    .build_config();
                config.ai.provider
            })
        })
        .collect();

    for (i, handle) in handles.into_iter().enumerate() {
        let provider = handle.join().unwrap();
        assert_eq!(provider, format!("provider_{}", i));
    }
}
