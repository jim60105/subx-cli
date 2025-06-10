use tempfile::TempDir;
use std::env;
use std::fs;
use subx_cli::cli::{ConfigArgs, ConfigAction};
use subx_cli::commands::config_command;
use crate::common::command_helpers::create_test_config;

#[tokio::test]
async fn test_show_config_display() {
    let temp = TempDir::new().unwrap();
    let config_path = create_test_config(&temp).await;
    env::set_var("SUBX_CONFIG_PATH", &config_path);
    let args = ConfigArgs { action: ConfigAction::List };
    let result = config_command::execute(args).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_set_config_value() {
    let temp = TempDir::new().unwrap();
    let config_path = create_test_config(&temp).await;
    env::set_var("SUBX_CONFIG_PATH", &config_path);
    let args = ConfigArgs {
        action: ConfigAction::Set { key: "ai.model".to_string(), value: "gpt-foo".to_string() },
    };
    let result = config_command::execute(args).await;
    assert!(result.is_ok());
    let content = fs::read_to_string(config_path).unwrap();
    assert!(content.contains("gpt-foo"));
}
