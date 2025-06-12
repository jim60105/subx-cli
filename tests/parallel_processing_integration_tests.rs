use subx_cli::config::TestConfigBuilder;
use subx_cli::core::parallel::{
    FileProcessingTask, ProcessingOperation, TaskResult, TaskScheduler,
};
use tempfile::TempDir;

mod common;
use common::{
    mock_generators::{SubtitleFormat, SubtitleGenerator},
    mock_openai_helper::MockOpenAITestHelper,
    test_data_generators::MatchResponseGenerator,
};

#[tokio::test]
async fn test_batch_file_processing() {
    // Create test environment with multiple subtitle files
    let temp = TempDir::new().unwrap();
    let _config = TestConfigBuilder::new()
        .with_task_priorities(true)
        .with_auto_balance_workers(true)
        .build_config();

    // Use SubtitleGenerator to create test files
    let test_files = vec!["test1.srt", "test2.srt", "test3.srt"];

    for name in &test_files {
        let file_path = temp.path().join(name);
        let subtitle_gen = SubtitleGenerator::new(SubtitleFormat::Srt).generate_short_test();
        subtitle_gen.save_to_file(&file_path).await.unwrap();
    }

    let scheduler = TaskScheduler::new_with_defaults();
    let mut tasks: Vec<Box<dyn subx_cli::core::parallel::Task + Send + Sync>> = Vec::new();

    for name in &test_files {
        let path = temp.path().join(name);
        let task: Box<dyn subx_cli::core::parallel::Task + Send + Sync> =
            Box::new(FileProcessingTask {
                input_path: path,
                output_path: None,
                operation: ProcessingOperation::ValidateFormat,
            });
        tasks.push(task);
    }

    let results = scheduler.submit_batch_tasks(tasks).await;
    assert_eq!(results.len(), test_files.len());
    for r in results {
        assert!(matches!(r, TaskResult::Success(_)));
    }
}

#[tokio::test]
async fn test_parallel_command_integration() {
    // Create test video and subtitle folder structure
    let temp = TempDir::new().unwrap();
    let video_dir = temp.path().join("videos");
    let subtitle_dir = temp.path().join("subtitles");
    tokio::fs::create_dir_all(&video_dir).await.unwrap();
    tokio::fs::create_dir_all(&subtitle_dir).await.unwrap();
    // Create sample video and subtitle files
    let video = video_dir.join("video1.mkv");
    let subtitle = subtitle_dir.join("sub1.srt");
    tokio::fs::write(&video, b"dummy").await.unwrap();
    tokio::fs::write(&subtitle, b"dummy").await.unwrap();

    // Set up mock AI service
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_success(&MatchResponseGenerator::multiple_matches())
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    let result = subx_cli::commands::match_command::execute_parallel_match(
        temp.path(),
        true,
        None,
        &config_service,
    )
    .await;
    assert!(result.is_ok());
}
