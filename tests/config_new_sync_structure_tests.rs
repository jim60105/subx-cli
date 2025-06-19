use subx_cli::config::SyncConfig;
use toml;

/// Integration tests for the new sync configuration structure serialization and defaults.
#[test]
fn test_parse_new_sync_config_from_toml() {
    // Test parsing just the sync portion
    let toml_str = r#"
default_method = "vad"
max_offset_seconds = 30.0

[vad]
enabled = true
sensitivity = 0.5
padding_chunks = 1
min_speech_duration_ms = 50
"#;
    let sync: SyncConfig = toml::from_str(&toml_str).expect("Failed to parse sync TOML");
    assert_eq!(sync.default_method, "vad");
    assert_eq!(sync.max_offset_seconds, 30.0);
    assert!(sync.vad.enabled);
    assert_eq!(sync.vad.padding_chunks, 1);
    assert_eq!(sync.vad.min_speech_duration_ms, 50);
}
