mod common;
use common::cli_helpers::CLITestHelper;
use std::path::PathBuf;
use tokio::fs;

/// 測試批次模式下只有字幕文件時顯示跳過消息
#[tokio::test]
async fn test_batch_mode_subtitle_only_skip_message() {
    let helper = CLITestHelper::new();
    let workspace = helper.temp_dir_path().to_path_buf();

    // 創建只有字幕文件的目錄
    fs::create_dir_all(workspace.join("media")).await.unwrap();
    let src_srt = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src_srt, workspace.join("media/subtitle.srt"))
        .await
        .unwrap();

    // 執行批次同步
    let result = helper
        .run_command_with_config(&["sync", "-b", "media"])
        .await;

    // 驗證結果
    assert!(
        result.success,
        "Command should succeed even when skipping files"
    );
    assert!(
        result.stdout.contains("✗ Skip sync for") || result.stderr.contains("✗ Skip sync for"),
        "Should show skip message for subtitle without video. stdout: {}, stderr: {}",
        result.stdout,
        result.stderr
    );
    assert!(
        result.stdout.contains("no video files found in directory")
            || result.stderr.contains("no video files found in directory"),
        "Should explain why skipping. stdout: {}, stderr: {}",
        result.stdout,
        result.stderr
    );

    // 確保沒有生成同步後的文件
    assert!(
        !workspace.join("media/subtitle_synced.srt").exists(),
        "Should not create synced file for subtitle without video"
    );
}

/// 測試批次模式下混合文件的正確行為
#[cfg(feature = "slow-tests")]
#[tokio::test]
async fn test_batch_mode_mixed_files_correct_behavior() {
    let helper = CLITestHelper::new();
    let workspace = helper.temp_dir_path().to_path_buf();

    // 創建混合文件的目錄
    fs::create_dir_all(workspace.join("media")).await.unwrap();

    // 添加有對應視頻的字幕
    let src_video = PathBuf::from("assets/SubX - The Subtitle Revolution.mp4");
    let src_srt = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src_video, workspace.join("media/video.mp4"))
        .await
        .unwrap();
    fs::copy(&src_srt, workspace.join("media/video.srt"))
        .await
        .unwrap();

    // 添加沒有對應視頻的字幕
    fs::copy(&src_srt, workspace.join("media/orphan.srt"))
        .await
        .unwrap();

    // 執行批次同步
    let result = helper
        .run_command_with_config(&["sync", "-b", "media"])
        .await;

    // 驗證結果
    assert!(result.success, "Command should succeed");

    // 檢查跳過消息
    let output = format!("{}\n{}", result.stdout, result.stderr);
    assert!(
        output.contains("✗ Skip sync for") && output.contains("orphan.srt"),
        "Should show skip message for orphan subtitle. Output: {}",
        output
    );
    assert!(
        output.contains("no matching video"),
        "Should explain why skipping orphan subtitle. Output: {}",
        output
    );

    // 確保只有匹配的文件被處理
    assert!(
        workspace.join("media/video_synced.srt").exists(),
        "Should create synced file for matched video-subtitle pair"
    );
    assert!(
        !workspace.join("media/orphan_synced.srt").exists(),
        "Should not create synced file for orphan subtitle"
    );
}

/// 測試批次模式下多個只有字幕文件的處理
#[tokio::test]
async fn test_batch_mode_multiple_subtitle_only_files() {
    let helper = CLITestHelper::new();
    let workspace = helper.temp_dir_path().to_path_buf();

    // 創建只有多個字幕文件的目錄
    fs::create_dir_all(workspace.join("media")).await.unwrap();
    let src_srt = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");

    for i in 1..=3 {
        fs::copy(
            &src_srt,
            workspace.join("media").join(format!("subtitle{}.srt", i)),
        )
        .await
        .unwrap();
    }

    // 執行批次同步
    let result = helper
        .run_command_with_config(&["sync", "-b", "media"])
        .await;

    // 驗證結果
    assert!(result.success, "Command should succeed");

    let output = format!("{}\n{}", result.stdout, result.stderr);

    // 檢查所有字幕文件都被跳過
    for i in 1..=3 {
        assert!(
            output.contains(&format!("subtitle{}.srt", i)),
            "Should mention subtitle{}.srt in skip message. Output: {}",
            i,
            output
        );
    }

    // 確保沒有生成任何同步後的文件
    for i in 1..=3 {
        assert!(
            !workspace
                .join("media")
                .join(format!("subtitle{}_synced.srt", i))
                .exists(),
            "Should not create synced file for subtitle{}.srt",
            i
        );
    }
}

/// 測試不同字幕格式的跳過行為
#[tokio::test]
async fn test_batch_mode_different_subtitle_formats_skip() {
    let helper = CLITestHelper::new();
    let workspace = helper.temp_dir_path().to_path_buf();

    // 創建目錄
    fs::create_dir_all(workspace.join("media")).await.unwrap();

    // 創建不同格式的字幕文件（沒有對應視頻）
    let srt_content = "1\n00:00:01,000 --> 00:00:03,000\nTest subtitle\n\n";
    fs::write(workspace.join("media/test.srt"), srt_content)
        .await
        .unwrap();

    let vtt_content = "WEBVTT\n\n1\n00:00:01.000 --> 00:00:03.000\nTest subtitle\n\n";
    fs::write(workspace.join("media/test.vtt"), vtt_content)
        .await
        .unwrap();

    let ass_content = "[Script Info]\nTitle: Test\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,20,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,0,0,2,0,0,0,1\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,Test subtitle\n";
    fs::write(workspace.join("media/test.ass"), ass_content)
        .await
        .unwrap();

    // 執行批次同步
    let result = helper
        .run_command_with_config(&["sync", "-b", "media"])
        .await;

    // 驗證結果
    assert!(result.success, "Command should succeed");

    let output = format!("{}\n{}", result.stdout, result.stderr);

    // 檢查所有格式的字幕文件都被跳過
    for ext in &["srt", "vtt", "ass"] {
        assert!(
            output.contains(&format!("test.{}", ext)),
            "Should mention test.{} in skip message. Output: {}",
            ext,
            output
        );
    }

    // 確保沒有生成任何同步後的文件
    for ext in &["srt", "vtt", "ass"] {
        assert!(
            !workspace
                .join("media")
                .join(format!("test_synced.{}", ext))
                .exists(),
            "Should not create synced file for test.{}",
            ext
        );
    }
}
