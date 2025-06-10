use tempfile::TempDir;
use std::env;
use subx_cli::cli::{CacheArgs, CacheAction};
use subx_cli::commands::cache_command;
use crate::common::command_helpers::create_test_cache_files;

#[tokio::test]
async fn test_clear_cache_success() {
    let temp = TempDir::new().unwrap();
    env::set_var("XDG_CONFIG_HOME", temp.path());
    let cache_path = create_test_cache_files(&temp).await;
    let args = CacheArgs { action: CacheAction::Clear };
    let result = cache_command::execute(args).await;
    assert!(result.is_ok());
    assert!(!cache_path.exists());
}

#[tokio::test]
async fn test_clear_cache_no_file() {
    let temp = TempDir::new().unwrap();
    env::set_var("XDG_CONFIG_HOME", temp.path());
    let args = CacheArgs { action: CacheAction::Clear };
    let result = cache_command::execute(args).await;
    assert!(result.is_ok());
}
