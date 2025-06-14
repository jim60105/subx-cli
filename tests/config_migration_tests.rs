use subx_cli::config::Config;
use toml;

/// Tests backward compatibility for old sync configuration keys.
/// Old sync settings should fail validation under the new structure.
#[test]
fn test_old_sync_config_migration_error() {
    let toml_str = r#"
[sync]
max_offset_seconds = 10.0
correlation_threshold = 0.8
"#;
    let config: Config = toml::from_str(&toml_str).expect("Parsing old sync block succeeded");
    // Under new structure, validation should reject missing new fields
    assert!(
        config.sync.validate().is_err(),
        "Old sync configuration should fail validation"
    );
}
