use subx_cli::{
    config::{ConfigService, TestConfigBuilder},
    test_with_config,
};

// 確保所有可設定的配置項目都可被 get/set
#[test]
fn test_config_get_set_consistency() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            let keys = vec![
                "ai.provider",
                "ai.model",
                "ai.api_key",
                "ai.base_url",
                "ai.max_sample_length",
                "ai.temperature",
                "ai.max_tokens",
                "ai.retry_attempts",
                "ai.retry_delay_ms",
                "formats.default_output",
                "formats.default_encoding",
                "formats.preserve_styling",
                "formats.encoding_detection_confidence",
                "sync.default_method",
                "sync.max_offset_seconds",
                "sync.vad.enabled",
                "sync.vad.sensitivity",
                "sync.vad.chunk_size",
                "sync.vad.sample_rate",
                "sync.vad.padding_chunks",
                "sync.vad.min_speech_duration_ms",
                "sync.vad.speech_merge_gap_ms",
                "general.backup_enabled",
                "general.max_concurrent_jobs",
                "general.task_timeout_seconds",
                "general.enable_progress_bar",
                "general.worker_idle_timeout_seconds",
                "parallel.max_workers",
                "parallel.task_queue_size",
                "parallel.enable_task_priorities",
                "parallel.auto_balance_workers",
                "parallel.overflow_strategy",
            ];
            for key in keys {
                assert!(
                    config_service
                        .set_config_value(key, &config_service.get_config_value(key).unwrap())
                        .is_ok(),
                    "Cannot set config key: {}",
                    key
                );
                assert!(
                    config_service.get_config_value(key).is_ok(),
                    "Cannot get config key: {}",
                    key
                );
            }
        }
    );
}

// 測試 VAD 配置的完整 get/set 循環
#[test]
fn test_vad_config_cli_support() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            config_service
                .set_config_value("sync.vad.enabled", "false")
                .unwrap();
            assert_eq!(
                config_service.get_config_value("sync.vad.enabled").unwrap(),
                "false"
            );

            config_service
                .set_config_value("sync.vad.sensitivity", "0.8")
                .unwrap();
            assert_eq!(
                config_service
                    .get_config_value("sync.vad.sensitivity")
                    .unwrap(),
                "0.8"
            );

            config_service
                .set_config_value("sync.vad.chunk_size", "1024")
                .unwrap();
            assert_eq!(
                config_service
                    .get_config_value("sync.vad.chunk_size")
                    .unwrap(),
                "1024"
            );

            config_service
                .set_config_value("sync.vad.sample_rate", "22050")
                .unwrap();
            assert_eq!(
                config_service
                    .get_config_value("sync.vad.sample_rate")
                    .unwrap(),
                "22050"
            );

            config_service
                .set_config_value("sync.vad.padding_chunks", "5")
                .unwrap();
            assert_eq!(
                config_service
                    .get_config_value("sync.vad.padding_chunks")
                    .unwrap(),
                "5"
            );

            config_service
                .set_config_value("sync.vad.min_speech_duration_ms", "50")
                .unwrap();
            assert_eq!(
                config_service
                    .get_config_value("sync.vad.min_speech_duration_ms")
                    .unwrap(),
                "50"
            );

            config_service
                .set_config_value("sync.vad.speech_merge_gap_ms", "300")
                .unwrap();
            assert_eq!(
                config_service
                    .get_config_value("sync.vad.speech_merge_gap_ms")
                    .unwrap(),
                "300"
            );
        }
    );
}

// 測試 VAD 配置的驗證邏輯
#[test]
fn test_vad_config_validation() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            assert!(
                config_service
                    .set_config_value("sync.vad.sensitivity", "1.5")
                    .is_err()
            );
            assert!(
                config_service
                    .set_config_value("sync.vad.chunk_size", "100")
                    .is_err()
            );
            assert!(
                config_service
                    .set_config_value("sync.vad.sample_rate", "12000")
                    .is_err()
            );
        }
    );
}
