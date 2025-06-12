//! Integration tests for match command cache reuse with copy/move modes.

use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
mod common;
use common::mock_openai_helper::MockOpenAITestHelper;
use common::test_data_generators::MatchResponseGenerator;

// 使用序列化測試來避免環境變數競爭
static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tokio::test]
async fn test_cache_reuse_preserves_copy_mode() {
    // 使用鎖確保測試序列化，避免環境變數競爭
    let _guard = TEST_MUTEX.lock().unwrap();

    // 使用固定的測試根目錄，確保快取路徑一致
    let test_root = std::path::Path::new("/tmp/subx_cache_test");
    if test_root.exists() {
        fs::remove_dir_all(test_root).unwrap();
    }
    fs::create_dir_all(test_root).unwrap();

    // 設定快取目錄
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    let video_dir = test_root.join("videos");
    let subtitle_dir = test_root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    fs::write(video_dir.join("video_copy.mp4"), "video").unwrap();
    fs::write(subtitle_dir.join("subtitle_copy.srt"), "sub").unwrap();

    // 發現檔案以獲取實際的檔案 ID
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, true).unwrap();

    let video_file = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();

    // 建立 mock AI 服務，使用實際的檔案 ID，設定只期望一次 API 呼叫
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(&video_file.id, &subtitle_file.id),
            1,
        )
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    // 第一次執行 dry-run 建立快取
    let args_preview = MatchArgs {
        path: test_root.to_path_buf(),
        dry_run: true,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_preview, &config_service)
        .await
        .unwrap();

    // 第二次執行相同的 dry-run 操作，應該使用快取（相同的目錄）
    let args_second = MatchArgs {
        path: test_root.to_path_buf(), // 使用相同的目錄
        dry_run: true,                 // 保持相同模式
        recursive: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_second, &config_service)
        .await
        .unwrap();

    // 驗證 mock server 只收到一次請求
    mock_helper.verify_expectations().await;
}

#[tokio::test]
async fn test_cache_reuse_preserves_move_mode() {
    // 使用鎖確保測試序列化，避免環境變數競爭
    let _guard = TEST_MUTEX.lock().unwrap();

    // 使用固定的測試根目錄，確保快取路徑一致
    let test_root = std::path::Path::new("/tmp/subx_cache_test_move");
    if test_root.exists() {
        fs::remove_dir_all(test_root).unwrap();
    }
    fs::create_dir_all(test_root).unwrap();

    // 設定快取目錄
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    let video_dir = test_root.join("videos");
    let subtitle_dir = test_root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    fs::write(video_dir.join("video_move.mp4"), "video").unwrap();
    fs::write(subtitle_dir.join("subtitle_move.srt"), "sub").unwrap();

    // 發現檔案以獲取實際的檔案 ID
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, true).unwrap();

    let video_file = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();

    // 建立 mock AI 服務，使用實際的檔案 ID，設定只期望一次 API 呼叫
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(&video_file.id, &subtitle_file.id),
            1,
        )
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    // 第一次執行 dry-run 建立快取
    let args_preview = MatchArgs {
        path: test_root.to_path_buf(),
        dry_run: true,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_preview, &config_service)
        .await
        .unwrap();

    // 第二次執行相同的 dry-run 操作，應該使用快取
    let args_second = MatchArgs {
        path: test_root.to_path_buf(),
        dry_run: true, // 保持相同模式
        recursive: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_second, &config_service)
        .await
        .unwrap();

    // 驗證 mock server 只收到一次請求
    mock_helper.verify_expectations().await;
}
