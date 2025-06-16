//! 整合測試：驗證 Bug 21 - Match 指令 Copy 模式快取目標目錄錯誤問題的修復
//!
//! 測試場景：
//! 1. 執行 dry-run 產生快取
//! 2. 執行實際的 copy 操作
//! 3. 驗證檔案被複製到正確的目錄（視訊檔案目錄而非字幕目錄）

use log::debug;
use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;

mod common;
use common::mock_openai_helper::MockOpenAITestHelper;
use common::test_data_generators::MatchResponseGenerator;

// 使用異步互斥鎖避免環境變數競爭條件
static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn test_bug_21_match_cache_copy_mode_correct_target_directory() {
    // 初始化日誌記錄
    let _ = env_logger::try_init();

    // 使用異步互斥鎖避免環境變數競爭條件
    let _guard = TEST_MUTEX.lock().await;
    debug!("開始測試 Bug 21: match 快取 copy 模式正確目標目錄");

    // 建立測試目錄結構
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();
    debug!("建立測試目錄: {:?}", test_root);

    // 設定快取目錄
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }
    debug!("設定 XDG_CONFIG_HOME 為: {:?}", test_root);

    // 建立分離的視訊和字幕目錄結構
    let video_dir = test_root.join("videos");
    let subtitle_dir = test_root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    // 建立測試檔案
    let video_file = video_dir.join("movie.mp4");
    let subtitle_file = subtitle_dir.join("movie.srt");
    fs::write(&video_file, "fake video content").unwrap();
    fs::write(
        &subtitle_file,
        "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n",
    )
    .unwrap();

    debug!("建立測試檔案:");
    debug!("  視訊檔案: {}", video_file.display());
    debug!("  字幕檔案: {}", subtitle_file.display());

    // 掃描檔案獲取實際的檔案 ID
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, true).unwrap();
    debug!("掃描目錄，找到 {} 個檔案", files.len());

    let video_file_info = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file_info = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();
    debug!(
        "找到視訊檔案: {} (id: {})",
        video_file_info.name, video_file_info.id
    );
    debug!(
        "找到字幕檔案: {} (id: {})",
        subtitle_file_info.name, subtitle_file_info.id
    );

    // 建立模擬 AI 服務，設定僅預期一次 API 呼叫
    let mock_helper = MockOpenAITestHelper::new().await;
    debug!("建立模擬 AI 助手於: {}", mock_helper.base_url());

    // 設定僅預期一次 API 呼叫（用於第一次執行）
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(
                &video_file_info.id,
                &subtitle_file_info.id,
            ),
            1,
        )
        .await;
    debug!("設定模擬預期為 1 次 API 呼叫");

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();
    debug!("建立設定服務與模擬 AI 伺服器");

    // 第一次執行：dry-run 建立快取
    debug!("執行第一次 match 指令 (dry-run) 以建立快取");
    let args_dry_run = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_dry_run, &config_service)
        .await
        .unwrap();
    debug!("第一次 match 指令執行完成");

    // 驗證原始檔案仍然存在
    assert!(video_file.exists(), "視訊檔案應該仍然存在");
    assert!(subtitle_file.exists(), "字幕檔案應該仍然存在");

    // 第二次執行：實際的 copy 操作（使用快取）
    debug!("執行第二次 match 指令 (copy 模式，應該使用快取)");
    let args_copy = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: false, // 實際執行
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_copy, &config_service)
        .await
        .unwrap();
    debug!("第二次 match 指令執行完成");

    // 關鍵驗證：檢查檔案是否被複製到正確的目錄
    let expected_copy_location = video_dir.join("movie.srt");
    let wrong_location = subtitle_dir.join("movie.srt"); // 原始位置（不應該有新檔案）

    debug!("驗證檔案位置:");
    debug!("  預期的複製位置: {}", expected_copy_location.display());
    debug!("  錯誤的位置: {}", wrong_location.display());

    // 主要驗證：檔案應該被複製到視訊目錄
    assert!(
        expected_copy_location.exists(),
        "字幕檔案應該被複製到視訊檔案目錄: {}",
        expected_copy_location.display()
    );

    // 驗證原始檔案仍然存在（copy 模式）
    assert!(
        subtitle_file.exists(),
        "原始字幕檔案應該仍然存在: {}",
        subtitle_file.display()
    );

    // 驗證複製的檔案內容正確
    let original_content = fs::read_to_string(&subtitle_file).unwrap();
    let copied_content = fs::read_to_string(&expected_copy_location).unwrap();
    assert_eq!(
        original_content, copied_content,
        "複製的檔案內容應該與原始檔案相同"
    );

    // 驗證模擬伺服器僅接收到一次請求
    debug!("驗證模擬預期");
    mock_helper.verify_expectations().await;
    debug!("模擬預期驗證成功");

    debug!("✅ Bug 21 修復驗證成功：copy 模式下快取正確保持目標目錄");
}

#[tokio::test]
async fn test_bug_21_comparison_dry_run_vs_actual_execution() {
    // 初始化日誌記錄
    let _ = env_logger::try_init();

    // 使用異步互斥鎖避免環境變數競爭條件
    let _guard = TEST_MUTEX.lock().await;
    debug!("開始測試 Bug 21: dry-run 與實際執行的一致性比較");

    // 建立測試目錄結構
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();

    // 設定快取目錄
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    // 建立分離的視訊和字幕目錄結構
    let video_dir = test_root.join("videos");
    let subtitle_dir = test_root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    // 建立測試檔案
    let video_file = video_dir.join("test_movie.mp4");
    let subtitle_file = subtitle_dir.join("test_subtitle.srt");
    fs::write(&video_file, "fake video content").unwrap();
    fs::write(
        &subtitle_file,
        "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n",
    )
    .unwrap();

    // 掃描檔案獲取實際的檔案 ID
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, true).unwrap();

    let video_file_info = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file_info = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();

    // 建立模擬 AI 服務，設定僅預期一次 API 呼叫
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(
                &video_file_info.id,
                &subtitle_file_info.id,
            ),
            1,
        )
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    // 第一次執行：dry-run 建立快取
    debug!("執行 dry-run 建立快取");
    let args_dry_run = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_dry_run, &config_service)
        .await
        .unwrap();

    // 驗證 dry-run 後沒有檔案被實際移動或複製
    let expected_copy_location = video_dir.join("test_movie.srt");
    assert!(
        !expected_copy_location.exists(),
        "dry-run 不應該建立實際檔案"
    );

    // 第二次執行：實際執行（使用快取）
    debug!("執行實際 copy 操作");
    let args_actual = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: false,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_actual, &config_service)
        .await
        .unwrap();

    // 驗證實際執行後檔案被正確複製
    assert!(
        expected_copy_location.exists(),
        "實際執行應該複製檔案到正確位置: {}",
        expected_copy_location.display()
    );

    // 驗證檔案內容一致
    let original_content = fs::read_to_string(&subtitle_file).unwrap();
    let copied_content = fs::read_to_string(&expected_copy_location).unwrap();
    assert_eq!(original_content, copied_content);

    // 驗證原始檔案仍然存在（copy 模式）
    assert!(subtitle_file.exists(), "原始檔案應該仍然存在");

    // 驗證模擬伺服器僅接收到一次請求
    mock_helper.verify_expectations().await;

    debug!("✅ dry-run 與實際執行一致性驗證成功");
}

#[tokio::test]
async fn test_bug_21_move_mode_cache_correctness() {
    // 初始化日誌記錄
    let _ = env_logger::try_init();

    // 使用異步互斥鎖避免環境變數競爭條件
    let _guard = TEST_MUTEX.lock().await;
    debug!("開始測試 Bug 21: move 模式快取正確性（對照測試）");

    // 建立測試目錄結構
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();

    // 設定快取目錄
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    // 建立分離的視訊和字幕目錄結構
    let video_dir = test_root.join("videos");
    let subtitle_dir = test_root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    // 建立測試檔案
    let video_file = video_dir.join("move_test.mp4");
    let subtitle_file = subtitle_dir.join("move_test.srt");
    fs::write(&video_file, "fake video content").unwrap();
    fs::write(
        &subtitle_file,
        "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n",
    )
    .unwrap();

    // 掃描檔案獲取實際的檔案 ID
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, true).unwrap();

    let video_file_info = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file_info = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();

    // 建立模擬 AI 服務
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(
                &video_file_info.id,
                &subtitle_file_info.id,
            ),
            1,
        )
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    // 第一次執行：dry-run 建立快取（move 模式）
    debug!("執行 move 模式 dry-run 建立快取");
    let args_dry_run = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_dry_run, &config_service)
        .await
        .unwrap();

    // 第二次執行：實際 move 操作（使用快取）
    debug!("執行實際 move 操作");
    let args_move = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: false,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_move, &config_service)
        .await
        .unwrap();

    // 驗證檔案被移動到正確位置
    let expected_move_location = video_dir.join("move_test.srt");
    assert!(
        expected_move_location.exists(),
        "字幕檔案應該被移動到視訊檔案目錄: {}",
        expected_move_location.display()
    );

    // 驗證原始檔案不再存在（move 模式）
    assert!(
        !subtitle_file.exists(),
        "原始字幕檔案應該已被移動: {}",
        subtitle_file.display()
    );

    // 驗證模擬伺服器僅接收到一次請求
    mock_helper.verify_expectations().await;

    debug!("✅ move 模式快取正確性驗證成功");
}
