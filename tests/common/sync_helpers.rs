use tempfile::TempDir;
use std::path::PathBuf;
use tokio::fs;
use subx_cli::services::audio::generate_dialogue_audio;

/// 在測試中建立含有對話模式的音訊檔案
pub async fn create_test_audio_with_dialogue() -> PathBuf {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("dialogue.wav");
    // 對話模式：靜音 0–1s、對話 1–3s、靜音 3–3.5s、對話 3.5–6s、靜音 6–7s
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

/// 建立一個與音訊延遲對齊的 SRT 檔案（示例：延遲 1.5s）
pub async fn create_misaligned_subtitle_file() -> PathBuf {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("misaligned.srt");
    let content = r#"1
00:00:02,500 --> 00:00:04,500
第一句對話（延遲 1.5 秒）

2
00:00:05,000 --> 00:00:07,500
第二句對話（延遲 1.5 秒）
"#;
    fs::write(&path, content).await.unwrap();
    path
}

/// 建立與音訊完美同步的測試對（音訊與字幕）
pub async fn create_well_synced_pair() -> (PathBuf, PathBuf) {
    let audio = create_test_audio_with_dialogue().await;
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("aligned.srt");
    let content = r#"1
00:00:01,000 --> 00:00:03,000
對話片段 1

2
00:00:03,500 --> 00:00:06,000
對話片段 2
"#;
    fs::write(&path, content).await.unwrap();
    (audio, path)
}

/// 建立與音訊嚴重不同步的測試對（音訊與字幕）
pub async fn create_poorly_synced_pair() -> (PathBuf, PathBuf) {
    let audio = create_test_audio_with_dialogue().await;
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("misaligned_long.srt");
    let content = r#"1
00:00:05,000 --> 00:00:07,000
錯誤延遲對話
"#;
    fs::write(&path, content).await.unwrap();
    (audio, path)
}

/// 建立含有能量模式的音訊樣本，用於能量分析器測試
pub fn create_test_audio_samples_with_pattern() -> Vec<f32> {
    let mut samples = Vec::new();
    // 交替靜音與高能量區段
    for block in 0..10 {
        let value = if block % 2 == 0 { 0.0 } else { 1.0 };
        for _ in 0..512 {
            samples.push(value);
        }
    }
    samples
}
