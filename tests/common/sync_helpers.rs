use tempfile::TempDir;
use std::path::PathBuf;
use tokio::fs;
use subx_cli::services::audio::generate_dialogue_audio;

/// Create audio file with dialogue pattern in tests
pub async fn create_test_audio_with_dialogue() -> PathBuf {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("dialogue.wav");
    // Dialogue pattern: silence 0–1s, dialogue 1–3s, silence 3–3.5s, dialogue 3.5–6s, silence 6–7s
    let dialogue_pattern = vec![
        (0.0, 1.0, false),
        (1.0, 3.0, true),
        (3.0, 3.5, false),
        (3.5, 6.0, true),
        (6.0, 7.0, false),
    ];
    generate_dialogue_audio(&path, &dialogue_pattern, 44100)
        .await
        .unwrap();
    path
}

/// Create an SRT file with audio delay alignment (example: 1.5s delay)
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
