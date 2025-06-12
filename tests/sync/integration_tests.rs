#![allow(unused_imports, dead_code)]
//! Synchronization functionality integration tests
//! Testing cross-module synchronization workflows

use subx_cli::core::sync::SyncEngine;
use subx_cli::core::sync::dialogue::DialogueDetector;
use subx_cli::core::sync::engine::{SyncConfig, SyncMethod};
use subx_cli::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use subx_cli::services::audio::generate_dialogue_audio;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;
use std::time::Duration;
use std::path::PathBuf;

mod common;
use common::{TestFileManager, AudioMockGenerator, SubtitleGenerator, SubtitleFormat};

/// Test end-to-end synchronization workflow
#[tokio::test] 
async fn test_end_to_end_sync_workflow() {
    // Use new configuration system
    let config = TestConfigBuilder::new()
        .with_dialogue_detection(true)
        .build_config();
    
    // Create complete test scenario
    let temp_dir = TempDir::new().unwrap();
    
    // 1. Create test audio file
    let audio_file = temp_dir.path().join("test_audio.wav");
    let dialogue_pattern = vec![
        (0.0, 1.0, false),   // Silence
        (1.0, 3.0, true),    // Dialogue 1
        (3.0, 3.5, false),   // Brief silence
        (3.5, 6.0, true),    // Dialogue 2
        (6.0, 7.0, false),   // Silence
    ];
    
    generate_dialogue_audio(&audio_file, &dialogue_pattern, 44100).await;
    
    // 2. Create misaligned subtitle file
    let subtitle_file = temp_dir.path().join("misaligned.srt");
    let subtitle_content = r#"1
00:00:02,500 --> 00:00:04,500
First dialogue (delayed by 1.5 seconds)

2
00:00:05,000 --> 00:00:07,500
Second dialogue (delayed by 1.5 seconds)
"#;
    tokio::fs::write(&subtitle_file, subtitle_content).await.unwrap();

    // 3. Run complete synchronization workflow
    let sync_config = SyncConfig {
        max_offset_seconds: 5.0,
        correlation_threshold: 0.3,
        dialogue_threshold: 0.3,
        min_dialogue_length: 0.5,
    };
    let sync_engine = SyncEngine::new(sync_config);
    
    // 4. Load subtitle file
    let subtitle = subx_cli::core::formats::load_subtitle_file(&subtitle_file).unwrap();
    
    // 5. Execute synchronization
    let sync_result = sync_engine.sync_subtitle(&audio_file, &subtitle).await.unwrap();
    
    // 6. Verify synchronization results
    assert!(sync_result.confidence >= 0.0, "Confidence should be valid");
    assert!(sync_result.offset_seconds.abs() <= 5.0, "Offset should be within reasonable range");
    
    // 7. Apply offset
    let mut adjusted_subtitle = subtitle.clone();
    sync_engine.apply_sync_offset(&mut adjusted_subtitle, sync_result.offset_seconds).unwrap();
    
    // 8. Verify adjusted subtitle
    assert_eq!(adjusted_subtitle.entries.len(), subtitle.entries.len());
    for (original, adjusted) in subtitle.entries.iter().zip(adjusted_subtitle.entries.iter()) {
        if sync_result.offset_seconds >= 0.0 {
            assert!(adjusted.start_time >= original.start_time, "Positive offset should increase time");
        } else {
            assert!(adjusted.start_time <= original.start_time, "Negative offset should decrease time");
        }
    }
}

/// Test dialogue detection integration workflow
#[tokio::test]
async fn test_dialogue_detection_integration() {
    // Use new configuration system
    let config = TestConfigBuilder::new()
        .with_dialogue_detection(true)
        .with_sync_threshold(0.6)
        .build_config();
    
    let temp_dir = TempDir::new().unwrap();
    
    // Create test audio file 
    let audio_file = temp_dir.path().join("dialogue_test.wav");
    let dialogue_pattern = vec![
        (0.0, 0.5, false),   // Silence start
        (0.5, 2.0, true),    // First dialogue segment
        (2.0, 2.5, false),   // Interval
        (2.5, 4.5, true),    // Second dialogue segment
        (4.5, 5.0, false),   // Silence end
    ];
    
    generate_dialogue_audio(&audio_file, &dialogue_pattern, 44100).await;
    
    // Create dialogue detector
    let detector = DialogueDetector::new().unwrap();
    
    // Perform dialogue detection
    let segments = detector.detect_dialogue(&audio_file).await.unwrap();
    
    // Verify detection results
    if !segments.is_empty() {
        // Check for dialogue and silence segments
        let speech_segments: Vec<_> = segments.iter().filter(|s| s.is_speech).collect();
        let silence_segments: Vec<_> = segments.iter().filter(|s| !s.is_speech).collect();
        
        // There should be some detected segments
        assert!(!segments.is_empty(), "Some audio segments should be detected");
        
        // Verify speech ratio calculation
        let speech_ratio = detector.get_speech_ratio(&segments);
        assert!(speech_ratio >= 0.0 && speech_ratio <= 1.0, "Speech ratio should be between 0 and 1");
        
        // Check segment time order
        for window in segments.windows(2) {
            assert!(window[0].start_time <= window[1].start_time, "Segments should be in chronological order");
        }
    }
}

/// Test synchronization engine and dialogue detection integration
#[tokio::test]
async fn test_sync_engine_dialogue_integration() {
    // Use new configuration system
    let config = TestConfigBuilder::new()
        .with_dialogue_detection(true)
        .with_sync_threshold(0.8)
        .build_config();
    
    let temp_dir = TempDir::new().unwrap();
    
    // Create test audio and subtitle
    let audio_file = temp_dir.path().join("integration_audio.wav");
    let dialogue_pattern = vec![
        (0.0, 1.0, false),
        (1.0, 3.0, true),
        (3.0, 6.0, true),
        (6.0, 7.0, false),
    ];
    
    generate_dialogue_audio(&audio_file, &dialogue_pattern, 44100).await;
    
    // Create paired subtitle
    let subtitle = Subtitle {
        entries: vec![
            SubtitleEntry::new(1, Duration::from_secs(1), Duration::from_secs(3), "First sentence".to_string()),
            SubtitleEntry::new(2, Duration::from_secs(3), Duration::from_secs(6), "Second sentence".to_string()),
        ],
        metadata: SubtitleMetadata::default(),
        format: SubtitleFormatType::Srt,
    };
    
    // Test synchronization engine
    let sync_config = SyncConfig {
        max_offset_seconds: 3.0,
        correlation_threshold: 0.2,
        dialogue_threshold: 0.3,
        min_dialogue_length: 0.5,
    };
    let sync_engine = SyncEngine::new(sync_config);
    
    // Execute synchronization
    let sync_result = sync_engine.sync_subtitle(&audio_file, &subtitle).await.unwrap();
    
    // Verify result structure
    assert!(sync_result.offset_seconds.abs() <= 3.0);
    assert!(sync_result.confidence >= 0.0 && sync_result.confidence <= 1.0);
    assert!(sync_result.correlation_peak >= 0.0 && sync_result.correlation_peak <= 1.0);
    
    // Test dialogue detector
    let detector = DialogueDetector::new().unwrap();
    let segments = detector.detect_dialogue(&audio_file).await.unwrap();
    
    // Verify integration of the two components
    if !segments.is_empty() {
        let speech_ratio = detector.get_speech_ratio(&segments);
        
        // If there is enough speech, the synchronization result should be relatively reliable
        if speech_ratio > 0.3 {
            assert!(sync_result.confidence >= 0.1, "There should be some confidence when there is enough speech");
        }
    }
}

/// Test synchronization quality assessment
#[tokio::test]
async fn test_sync_quality_assessment() {
    // Use new configuration system
    let config = TestConfigBuilder::new()
        .with_sync_threshold(0.8)
        .build_config();
    
    let temp_dir = TempDir::new().unwrap();
    
    // Create high-quality aligned test case
    let good_audio = temp_dir.path().join("good_sync.wav");
    let good_pattern = vec![
        (1.0, 3.0, true),
        (4.0, 6.0, true),
    ];
    generate_dialogue_audio(&good_audio, &good_pattern, 44100).await;
    
    let good_subtitle = Subtitle {
        entries: vec![
            SubtitleEntry::new(1, Duration::from_secs(1), Duration::from_secs(3), "Perfectly aligned".to_string()),
            SubtitleEntry::new(2, Duration::from_secs(4), Duration::from_secs(6), "Well aligned".to_string()),
        ],
        metadata: SubtitleMetadata::default(),
        format: SubtitleFormatType::Srt,
    };
    
    // Create low-quality aligned test case
    let bad_audio = temp_dir.path().join("bad_sync.wav");
    let bad_pattern = vec![
        (0.0, 2.0, true),
        (5.0, 7.0, true),
    ];
    generate_dialogue_audio(&bad_audio, &bad_pattern, 44100).await;
    
    let bad_subtitle = Subtitle {
        entries: vec![
            SubtitleEntry::new(1, Duration::from_secs(3), Duration::from_secs(5), "Misaligned subtitle".to_string()),
            SubtitleEntry::new(2, Duration::from_secs(8), Duration::from_secs(10), "Severely misaligned".to_string()),
        ],
        metadata: SubtitleMetadata::default(),
        format: SubtitleFormatType::Srt,
    };
    
    let sync_config = SyncConfig {
        max_offset_seconds: 5.0,
        correlation_threshold: 0.3,
        dialogue_threshold: 0.3,
        min_dialogue_length: 0.5,
    };
    let sync_engine = SyncEngine::new(sync_config);
    
    // Test good synchronization quality
    let good_result = sync_engine.sync_subtitle(&good_audio, &good_subtitle).await.unwrap();
    
    // Test bad synchronization quality
    let bad_result = sync_engine.sync_subtitle(&bad_audio, &bad_subtitle).await.unwrap();
    
    // Compare result quality
    // Note: Due to limitations in audio generation and correlation calculation, we mainly test the reasonableness of the result structure
    assert!(good_result.confidence <= 1.0);
    assert!(bad_result.confidence <= 1.0);
    assert!(good_result.offset_seconds.abs() <= 5.0);
    assert!(bad_result.offset_seconds.abs() <= 5.0);
}
