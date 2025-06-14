use subx_cli::config::Config;
use toml;

/// Tests backward compatibility for old sync configuration keys.
/// Old sync settings should fail parsing under the new structure.
#[test]
fn test_old_sync_config_migration_error() {
    let toml_str = r#"
[sync]
max_offset_seconds = 10.0
correlation_threshold = 0.8
"#;

    // Under the new structure, old configuration should fail to parse
    // because required fields like 'default_method' are missing
    let result: Result<Config, _> = toml::from_str(&toml_str);
    assert!(
        result.is_err(),
        "Old sync configuration should fail to parse due to missing required fields"
    );
}
