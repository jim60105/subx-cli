//! Integration test for first sentence sync offset using sample assets

use std::path::Path;
use subx_cli::config::TestConfigBuilder;
use subx_cli::core::formats::manager::FormatManager;
use subx_cli::core::sync::{SyncEngine, SyncMethod};

#[cfg(feature = "slow-tests")]
#[tokio::test]
async fn test_sync_first_sentence_with_assets() {
    println!("[TEST] Building test configuration with VAD enabled...");
    // Build test configuration with VAD enabled
    let config = TestConfigBuilder::new()
        .with_vad_enabled(true)
        .build_config();
    let sync_config = config.sync;
    println!("[TEST] Creating SyncEngine instance...");
    let sync_engine = SyncEngine::new(sync_config).expect("Failed to create SyncEngine");

    // Load audio and subtitle assets
    let audio_path = Path::new("assets/SubX - The Subtitle Revolution.mp3");
    let subtitle_path = Path::new("assets/SubX - The Subtitle Revolution.srt");
    println!("[TEST] Loading subtitle file: {:?}", subtitle_path);
    let subtitle = FormatManager::new()
        .load_subtitle(subtitle_path)
        .expect("Failed to load subtitle file");

    println!(
        "[TEST] Starting VAD sync detection: audio = {:?}, subtitle entries = {}",
        audio_path,
        subtitle.entries.len()
    );
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

    println!(
        "[TEST] Detection result: speech_start = {:.3}, expected_start = {:.3}, offset = {:.3}, confidence = {:.3}",
        speech_start, expected_start, offset, result.confidence
    );

    // Verify offset calculation: speech_start - expected_start equals offset
    assert!(
        (speech_start - expected_start - offset).abs() < 0.01,
        "Offset calculation mismatch: speech_start ({speech_start}) - expected_start ({expected_start}) != offset ({offset})"
    );
    println!("[TEST] Offset calculation verified");

    // Apply the detected offset and verify alignment of first subtitle entry
    let mut adjusted = subtitle.clone();
    sync_engine
        .apply_manual_offset(&mut adjusted, offset)
        .expect("Failed to apply manual offset");

    let adjusted_start = adjusted.entries[0].start_time.as_secs_f32();
    println!(
        "[TEST] After applying offset: adjusted_start = {:.3}, speech_start = {:.3}",
        adjusted_start, speech_start
    );
    assert!(
        (adjusted_start - speech_start).abs() < 0.05,
        "First subtitle entry not aligned: adjusted_start ({adjusted_start}) vs speech_start ({speech_start})"
    );
    println!("[TEST] First subtitle alignment verified");
}
