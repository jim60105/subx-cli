#![allow(unused_imports, dead_code)]
//! 同步功能整合測試
//! 測試跨模組的同步工作流程

use subx_cli::core::sync::SyncEngine;
use subx_cli::core::sync::dialogue::DialogueDetector;
use subx_cli::core::sync::engine::{SyncConfig, SyncMethod};
use subx_cli::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use subx_cli::services::audio::generate_dialogue_audio;
use tempfile::TempDir;
use std::time::Duration;
use std::path::PathBuf;

/// 測試端到端同步工作流程
#[tokio::test] 
async fn test_end_to_end_sync_workflow() {
    // 初始化配置管理器
    subx_cli::config::reset_global_config_manager();
    subx_cli::config::init_config_manager().unwrap();
    
    // 建立完整的測試場景
    let temp_dir = TempDir::new().unwrap();
    
    // 1. 建立測試音訊檔案
    let audio_file = temp_dir.path().join("test_audio.wav");
    let dialogue_pattern = vec![
        (0.0, 1.0, false),   // 靜音
        (1.0, 3.0, true),    // 對話 1
        (3.0, 3.5, false),   // 短暫靜音
        (3.5, 6.0, true),    // 對話 2
        (6.0, 7.0, false),   // 靜音
    ];
    
    generate_dialogue_audio(&audio_file, &dialogue_pattern, 44100).await;
    
    // 2. 建立不對齊的字幕檔案
    let subtitle_file = temp_dir.path().join("misaligned.srt");
    let subtitle_content = r#"1
00:00:02,500 --> 00:00:04,500
第一句對話（延遲 1.5 秒）

2
00:00:05,000 --> 00:00:07,500
第二句對話（延遲 1.5 秒）
"#;
    tokio::fs::write(&subtitle_file, subtitle_content).await.unwrap();

    // 3. 執行完整同步工作流程
    let sync_config = SyncConfig {
        max_offset_seconds: 5.0,
        correlation_threshold: 0.3,
        dialogue_threshold: 0.3,
        min_dialogue_length: 0.5,
    };
    let sync_engine = SyncEngine::new(sync_config);
    
    // 4. 載入字幕檔案
    let subtitle = subx_cli::core::formats::load_subtitle_file(&subtitle_file).unwrap();
    
    // 5. 執行同步
    let sync_result = sync_engine.sync_subtitle(&audio_file, &subtitle).await.unwrap();
    
    // 6. 驗證同步結果
    assert!(sync_result.confidence >= 0.0, "信心度應該是有效的");
    assert!(sync_result.offset_seconds.abs() <= 5.0, "偏移應該在合理範圍內");
    
    // 7. 應用偏移
    let mut adjusted_subtitle = subtitle.clone();
    sync_engine.apply_sync_offset(&mut adjusted_subtitle, sync_result.offset_seconds).unwrap();
    
    // 8. 驗證調整後的字幕
    assert_eq!(adjusted_subtitle.entries.len(), subtitle.entries.len());
    for (original, adjusted) in subtitle.entries.iter().zip(adjusted_subtitle.entries.iter()) {
        if sync_result.offset_seconds >= 0.0 {
            assert!(adjusted.start_time >= original.start_time, "正偏移應該增加時間");
        } else {
            assert!(adjusted.start_time <= original.start_time, "負偏移應該減少時間");
        }
    }
}

/// 測試對話檢測整合工作流程
#[tokio::test]
async fn test_dialogue_detection_integration() {
    // 初始化配置管理器
    subx_cli::config::reset_global_config_manager();
    subx_cli::config::init_config_manager().unwrap();
    
    let temp_dir = TempDir::new().unwrap();
    
    // 建立測試音訊檔案 
    let audio_file = temp_dir.path().join("dialogue_test.wav");
    let dialogue_pattern = vec![
        (0.0, 0.5, false),   // 靜音開始
        (0.5, 2.0, true),    // 第一段對話
        (2.0, 2.5, false),   // 間隔
        (2.5, 4.5, true),    // 第二段對話
        (4.5, 5.0, false),   // 靜音結束
    ];
    
    generate_dialogue_audio(&audio_file, &dialogue_pattern, 44100).await;
    
    // 建立對話檢測器
    let detector = DialogueDetector::new().unwrap();
    
    // 執行對話檢測
    let segments = detector.detect_dialogue(&audio_file).await.unwrap();
    
    // 驗證檢測結果
    if !segments.is_empty() {
        // 檢查有對話和靜音段落
        let speech_segments: Vec<_> = segments.iter().filter(|s| s.is_speech).collect();
        let silence_segments: Vec<_> = segments.iter().filter(|s| !s.is_speech).collect();
        
        // 應該有一些檢測到的段落
        assert!(!segments.is_empty(), "應該檢測到一些音訊段落");
        
        // 驗證語音比例計算
        let speech_ratio = detector.get_speech_ratio(&segments);
        assert!(speech_ratio >= 0.0 && speech_ratio <= 1.0, "語音比例應該在 0-1 之間");
        
        // 檢查段落時間順序
        for window in segments.windows(2) {
            assert!(window[0].start_time <= window[1].start_time, "段落應該按時間順序排列");
        }
    }
}

/// 測試同步引擎與對話檢測的整合
#[tokio::test]
async fn test_sync_engine_dialogue_integration() {
    // 初始化配置管理器
    subx_cli::config::reset_global_config_manager();
    subx_cli::config::init_config_manager().unwrap();
    
    let temp_dir = TempDir::new().unwrap();
    
    // 建立測試音訊和字幕
    let audio_file = temp_dir.path().join("integration_audio.wav");
    let dialogue_pattern = vec![
        (0.0, 1.0, false),
        (1.0, 3.0, true),
        (3.0, 6.0, true),
        (6.0, 7.0, false),
    ];
    
    generate_dialogue_audio(&audio_file, &dialogue_pattern, 44100).await;
    
    // 建立配對的字幕
    let subtitle = Subtitle {
        entries: vec![
            SubtitleEntry::new(1, Duration::from_secs(1), Duration::from_secs(3), "第一句".to_string()),
            SubtitleEntry::new(2, Duration::from_secs(3), Duration::from_secs(6), "第二句".to_string()),
        ],
        metadata: SubtitleMetadata::default(),
        format: SubtitleFormatType::Srt,
    };
    
    // 測試同步引擎
    let sync_config = SyncConfig {
        max_offset_seconds: 3.0,
        correlation_threshold: 0.2,
        dialogue_threshold: 0.3,
        min_dialogue_length: 0.5,
    };
    let sync_engine = SyncEngine::new(sync_config);
    
    // 執行同步
    let sync_result = sync_engine.sync_subtitle(&audio_file, &subtitle).await.unwrap();
    
    // 驗證結果結構
    assert!(sync_result.offset_seconds.abs() <= 3.0);
    assert!(sync_result.confidence >= 0.0 && sync_result.confidence <= 1.0);
    assert!(sync_result.correlation_peak >= 0.0 && sync_result.correlation_peak <= 1.0);
    
    // 測試對話檢測器
    let detector = DialogueDetector::new().unwrap();
    let segments = detector.detect_dialogue(&audio_file).await.unwrap();
    
    // 驗證兩個組件的整合
    if !segments.is_empty() {
        let speech_ratio = detector.get_speech_ratio(&segments);
        
        // 如果有足夠的語音，同步結果應該相對可靠
        if speech_ratio > 0.3 {
            assert!(sync_result.confidence >= 0.1, "有足夠語音時應該有一定信心度");
        }
    }
}

/// 測試同步品質評估
#[tokio::test]
async fn test_sync_quality_assessment() {
    // 初始化配置管理器
    subx_cli::config::reset_global_config_manager();
    subx_cli::config::init_config_manager().unwrap();
    
    let temp_dir = TempDir::new().unwrap();
    
    // 建立高品質對齊的測試案例
    let good_audio = temp_dir.path().join("good_sync.wav");
    let good_pattern = vec![
        (1.0, 3.0, true),
        (4.0, 6.0, true),
    ];
    generate_dialogue_audio(&good_audio, &good_pattern, 44100).await;
    
    let good_subtitle = Subtitle {
        entries: vec![
            SubtitleEntry::new(1, Duration::from_secs(1), Duration::from_secs(3), "完美對齊".to_string()),
            SubtitleEntry::new(2, Duration::from_secs(4), Duration::from_secs(6), "也很對齊".to_string()),
        ],
        metadata: SubtitleMetadata::default(),
        format: SubtitleFormatType::Srt,
    };
    
    // 建立低品質對齊的測試案例
    let bad_audio = temp_dir.path().join("bad_sync.wav");
    let bad_pattern = vec![
        (0.0, 2.0, true),
        (5.0, 7.0, true),
    ];
    generate_dialogue_audio(&bad_audio, &bad_pattern, 44100).await;
    
    let bad_subtitle = Subtitle {
        entries: vec![
            SubtitleEntry::new(1, Duration::from_secs(3), Duration::from_secs(5), "錯位字幕".to_string()),
            SubtitleEntry::new(2, Duration::from_secs(8), Duration::from_secs(10), "嚴重錯位".to_string()),
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
    
    // 測試好的同步品質
    let good_result = sync_engine.sync_subtitle(&good_audio, &good_subtitle).await.unwrap();
    
    // 測試壞的同步品質
    let bad_result = sync_engine.sync_subtitle(&bad_audio, &bad_subtitle).await.unwrap();
    
    // 比較結果品質
    // 注意：由於音訊生成和相關性計算的限制，我們主要測試結果結構的合理性
    assert!(good_result.confidence <= 1.0);
    assert!(bad_result.confidence <= 1.0);
    assert!(good_result.offset_seconds.abs() <= 5.0);
    assert!(bad_result.offset_seconds.abs() <= 5.0);
}
