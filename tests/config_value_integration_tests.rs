//! Config 系統整合測試：get_config_value 方法驗證

use subx_cli::config::{ConfigService, TestConfigBuilder};

/// 驗證 get_config_value 正確返回預期值
#[test]
fn test_get_config_value_success() {
    let service = TestConfigBuilder::new()
        .with_ai_provider("provX")
        .with_ai_model("modelY")
        .with_ai_api_key("keyZ")
        .with_ai_base_url("https://api.test")
        .build_service();

    assert_eq!(service.get_config_value("ai.provider").unwrap(), "provX");
    assert_eq!(service.get_config_value("ai.model").unwrap(), "modelY");
    assert_eq!(service.get_config_value("ai.api_key").unwrap(), "keyZ");
    assert_eq!(
        service.get_config_value("ai.base_url").unwrap(),
        "https://api.test"
    );
}

/// 驗證 get_config_value 對未知鍵返回錯誤
#[test]
fn test_get_config_value_unknown_key() {
    let service = TestConfigBuilder::new().build_service();
    let err = service.get_config_value("unknown.key");
    assert!(err.is_err(), "Expected error for unknown key");
}
