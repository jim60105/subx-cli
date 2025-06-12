mod common;
use common::{
    MatchResponseGenerator, MockChatCompletionResponse, MockOpenAITestHelper, MockUsageStats,
};

use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;

/// 載荷測試：模擬大量檔案處理並驗證效能
#[tokio::test]
async fn test_high_load_scenario() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // 建立大量測試檔案
    create_multiple_test_files(&root, 50);

    let mock_helper = MockOpenAITestHelper::new().await;
    // 設定具有延遲的回應以模擬網路條件
    mock_helper
        .setup_delayed_response(
            100, // 100ms 延遲
            MockChatCompletionResponse {
                content: MatchResponseGenerator::multiple_matches(),
                model: "gpt-4o-mini".to_string(),
                usage: Some(MockUsageStats {
                    prompt_tokens: 500,
                    completion_tokens: 200,
                    total_tokens: 700,
                }),
            },
        )
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .with_parallel_settings(8, 200)
        .build_service();

    let args = MatchArgs {
        path: root.to_path_buf(),
        dry_run: true,
        confidence: 50,
        recursive: true,
        backup: true,
        copy: true,
        move_files: false,
    };

    let start_time = std::time::Instant::now();
    let result = match_command::execute(args, &config_service).await;
    let elapsed = start_time.elapsed();

    assert!(result.is_ok());
    println!("高載荷測試完成時間: {:?}", elapsed);
    // 驗證在合理時間內完成（應小於 30 秒）
    assert!(elapsed < std::time::Duration::from_secs(30));
}

/// 記憶體穩定性測試：多次執行以檢測潛在的記憶體洩漏
#[tokio::test]
async fn test_memory_stability() {
    for iteration in 1..=10 {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        create_test_files(&root);

        let mock_helper = MockOpenAITestHelper::new().await;
        mock_helper
            .mock_chat_completion_success(&MatchResponseGenerator::successful_single_match())
            .await;

        let config_service = TestConfigBuilder::new()
            .with_mock_ai_server(&mock_helper.base_url())
            .build_service();

        let args = MatchArgs {
            path: root.to_path_buf(),
            dry_run: true,
            confidence: 50,
            recursive: true,
            backup: true,
            copy: false,
            move_files: false,
        };

        let result = match_command::execute(args, &config_service).await;
        assert!(result.is_ok(), "迭代 {} 失敗", iteration);

        // 清理資源
        drop(mock_helper);
        drop(config_service);
        drop(temp_dir);
    }
}

fn create_multiple_test_files(root: &std::path::Path, count: usize) {
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    for i in 1..=count {
        fs::write(
            video_dir.join(format!("movie{}.mp4", i)),
            format!("video{}", i),
        )
        .unwrap();
        fs::write(
            subtitle_dir.join(format!("movie{}.srt", i)),
            format!("1\n00:00:01,000 --> 00:00:02,000\nSubtitle {}\n", i),
        )
        .unwrap();
    }
}

fn create_test_files(root: &std::path::Path) {
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();
    fs::write(video_dir.join("movie.mp4"), "video").unwrap();
    fs::write(subtitle_dir.join("movie.srt"), "subtitle").unwrap();
}
