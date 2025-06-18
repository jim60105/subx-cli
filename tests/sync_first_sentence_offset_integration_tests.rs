//! Integration test for first sentence sync offset using sample assets

use std::path::Path;
use subx_cli::config::TestConfigBuilder;
use subx_cli::core::formats::manager::FormatManager;
use subx_cli::core::sync::{SyncEngine, SyncMethod};

#[tokio::test]
#[ignore = "Requires audio processing environment for VAD synchronization"]
async fn test_sync_first_sentence_with_assets() {
    // Build test configuration with VAD enabled
    let config = TestConfigBuilder::new()
        .with_vad_enabled(true)
        .build_config();
    let sync_config = config.sync;
    let sync_engine = SyncEngine::new(sync_config).expect("Failed to create SyncEngine");

    // Load audio and subtitle assets
    let audio_path = Path::new("assets/SubX - The Subtitle Revolution.mp3");
    let subtitle_path = Path::new("assets/SubX - The Subtitle Revolution.srt");
    let subtitle = FormatManager::new()
        .load_subtitle(subtitle_path)
        .expect("Failed to load subtitle file");

    // Detect synchronization offset
    let result = sync_engine
        .detect_sync_offset(audio_path, &subtitle, Some(SyncMethod::Auto))
        .await
        .expect("Failed to detect sync offset");

    // Extract detection details
    let info = result.additional_info.expect("Missing additional_info");
    let speech_start = info["first_speech_start"]
        .as_f64()
        .expect("Missing first_speech_start") as f32;
    let expected_start = info["expected_subtitle_start"]
        .as_f64()
        .expect("Missing expected_subtitle_start") as f32;
    let offset = result.offset_seconds;

    // Verify offset calculation: speech_start - expected_start equals offset
    assert!(
        (speech_start - expected_start - offset).abs() < 0.01,
        "Offset calculation mismatch: speech_start ({speech_start}) - expected_start ({expected_start}) != offset ({offset})"
    );

    // Apply the detected offset and verify alignment of first subtitle entry
    let mut adjusted = subtitle.clone();
    sync_engine
        .apply_manual_offset(&mut adjusted, offset)
        .expect("Failed to apply manual offset");

    let adjusted_start = adjusted.entries[0].start_time.as_secs_f32();
    assert!(
        (adjusted_start - speech_start).abs() < 0.05,
        "First subtitle entry not aligned: adjusted_start ({adjusted_start}) vs speech_start ({speech_start})"
    );
}
