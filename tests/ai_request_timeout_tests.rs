use subx_cli::config::{Config, ConfigService, TestConfigService};

#[test]
fn test_ai_request_timeout_configuration() {
    // 測試預設超時值
    let config = Config::default();
    assert_eq!(config.ai.request_timeout_seconds, 120);

    // 測試配置服務支援
    let service = TestConfigService::with_defaults();

    // 測試獲取配置值
    let timeout_value = service
        .get_config_value("ai.request_timeout_seconds")
        .unwrap();
    assert_eq!(timeout_value, "120");

    // 測試設定配置值
    service
        .set_config_value("ai.request_timeout_seconds", "180")
        .unwrap();
    let new_timeout_value = service
        .get_config_value("ai.request_timeout_seconds")
        .unwrap();
    assert_eq!(new_timeout_value, "180");

    // 測試配置驗證
    let config = service.get_config().unwrap();
    assert_eq!(config.ai.request_timeout_seconds, 180);
}

#[test]
fn test_ai_request_timeout_validation() {
    let service = TestConfigService::with_defaults();

    // 測試有效值
    assert!(
        service
            .set_config_value("ai.request_timeout_seconds", "60")
            .is_ok()
    );
    assert!(
        service
            .set_config_value("ai.request_timeout_seconds", "300")
            .is_ok()
    );
    assert!(
        service
            .set_config_value("ai.request_timeout_seconds", "600")
            .is_ok()
    );

    // 測試無效值 - 太小
    let result = service.set_config_value("ai.request_timeout_seconds", "5");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must be at least 10 seconds")
    );

    // 測試無效值 - 太大
    let result = service.set_config_value("ai.request_timeout_seconds", "700");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("should not exceed 600 seconds")
    );
}
