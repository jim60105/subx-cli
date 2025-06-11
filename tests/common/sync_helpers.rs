use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

/// Create mock audio file for tests
/// 建立測試用的模擬音訊檔案
#[allow(dead_code)]
pub async fn create_test_audio_with_dialogue() -> PathBuf {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("dialogue.wav");

    // 建立一個簡單的模擬音訊檔案用於測試
    // 在真實實作中，這會呼叫音訊生成服務
    fs::write(&path, b"RIFF....WAVE").await.unwrap();
    path
}

/// Create an SRT file with audio delay alignment (example: 1.5s delay)
#[allow(dead_code)]
pub async fn create_misaligned_subtitle_file() -> PathBuf {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("misaligned.srt");
    let content = r#"1
00:00:02,500 --> 00:00:04,500
First dialogue (1.5 seconds delay)

2
00:00:05,000 --> 00:00:07,500
Second dialogue (1.5 seconds delay)
"#;
    fs::write(&path, content).await.unwrap();
    path
}

/// Create perfectly synced test pair (audio and subtitle)
#[allow(dead_code)]
pub async fn create_well_synced_pair() -> (PathBuf, PathBuf) {
    let audio = create_test_audio_with_dialogue().await;
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("aligned.srt");
    let content = r#"1
00:00:01,000 --> 00:00:03,000
Dialogue segment 1

2
00:00:03,500 --> 00:00:06,000
Dialogue segment 2
"#;
    fs::write(&path, content).await.unwrap();
    (audio, path)
}

/// Create severely misaligned test pair (audio and subtitle)
#[allow(dead_code)]
pub async fn create_poorly_synced_pair() -> (PathBuf, PathBuf) {
    let audio = create_test_audio_with_dialogue().await;
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("misaligned_long.srt");
    let content = r#"1
00:00:05,000 --> 00:00:07,000
Incorrect delay dialogue
"#;
    fs::write(&path, content).await.unwrap();
    (audio, path)
}

/// Create audio samples with energy pattern for energy analyzer testing
#[allow(dead_code)]
pub fn create_test_audio_samples_with_pattern() -> Vec<f32> {
    let mut samples = Vec::new();
    // Alternating silence and high-energy segments
    for block in 0..10 {
        let value = if block % 2 == 0 { 0.0 } else { 1.0 };
        for _ in 0..512 {
            samples.push(value);
        }
    }
    samples
}
