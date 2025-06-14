use subx_cli::config::SyncConfig;
use toml;

/// Integration tests for the new sync configuration structure serialization and defaults.
#[test]
fn test_parse_new_sync_config_from_toml() {
    // Test parsing just the sync portion
    let toml_str = r#"
default_method = "vad"
analysis_window_seconds = 20
max_offset_seconds = 30.0

[whisper]
enabled = false
model = "whisper-1"
language = "en"
temperature = 0.2
timeout_seconds = 10
max_retries = 1
retry_delay_ms = 500
fallback_to_vad = false
min_confidence_threshold = 0.6

[vad]
enabled = true
sensitivity = 0.5
chunk_size = 256
sample_rate = 8000
padding_chunks = 1
min_speech_duration_ms = 50
speech_merge_gap_ms = 100
"#;
    let sync: SyncConfig = toml::from_str(&toml_str).expect("Failed to parse sync TOML");
    assert_eq!(sync.default_method, "vad");
    assert_eq!(sync.analysis_window_seconds, 20);
    assert_eq!(sync.max_offset_seconds, 30.0);
    assert!(!sync.whisper.enabled);
    assert_eq!(sync.whisper.language, "en");
    assert_eq!(sync.whisper.min_confidence_threshold, 0.6);
    assert!(sync.vad.enabled);
    assert_eq!(sync.vad.chunk_size, 256);
    assert_eq!(sync.vad.sample_rate, 8000);
}
