use subx_cli::core::parallel::{
    FileProcessingTask, ProcessingOperation, Task, TaskResult, TaskScheduler,
};
use tempfile::TempDir;

#[tokio::test]
async fn test_batch_file_processing() {
    // 建立測試環境與多個字幕檔案
    let temp = TempDir::new().unwrap();
    let test_files = vec!["test1.srt", "test2.srt", "test3.srt"];
    for name in &test_files {
        let path = temp.path().join(name);
        tokio::fs::write(&path, "1\n00:00:01,000 --> 00:00:02,000\nTest\n")
            .await
            .unwrap();
    }
    let scheduler = TaskScheduler::new().unwrap();
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
    // 創建測試影片與字幕資料夾結構
    let temp = TempDir::new().unwrap();
    let video_dir = temp.path().join("videos");
    let subtitle_dir = temp.path().join("subtitles");
    tokio::fs::create_dir_all(&video_dir).await.unwrap();
    tokio::fs::create_dir_all(&subtitle_dir).await.unwrap();
    // 創建範例影片與字幕檔案
    let video = video_dir.join("video1.mkv");
    let subtitle = subtitle_dir.join("sub1.srt");
    tokio::fs::write(&video, b"dummy").await.unwrap();
    tokio::fs::write(&subtitle, b"dummy").await.unwrap();

    let result =
        subx_cli::commands::match_command::execute_parallel_match(temp.path(), true, None).await;
    assert!(result.is_ok());
}
