use tempfile::TempDir;
use subx_cli::commands::detect_encoding_command;
use crate::common::command_helpers::create_utf8_subtitle_file;

#[tokio::test]
async fn test_detect_single_file_encoding() {
    let temp = TempDir::new().unwrap();
    let file = create_utf8_subtitle_file(&temp).await;
    let result = detect_encoding_command(&[file.to_string_lossy().to_string()], false);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_nonexistent_file() {
    let result = detect_encoding_command(&["noexist.srt".to_string()], false);
    assert!(result.is_ok());
}
