//! Integration tests for handling duplicate rename conflicts in match command
//!
//! Tests scenarios where multiple files would be renamed to the same target name,
//! verifying that the AutoRename conflict resolution strategy correctly handles
//! sequential filename conflict resolution with numeric suffixes.

use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;

mod common;
use common::mock_openai_helper::MockOpenAITestHelper;

/// Test that when AI returns multiple matches that would result in the same target filename,
/// the AutoRename conflict resolution correctly applies numeric suffixes sequentially.
#[tokio::test]
async fn test_multiple_files_rename_to_same_target_with_auto_rename() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create directory structure
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    // Create one video file that will be the target for multiple subtitle matches
    fs::write(video_dir.join("movie.mp4"), "video content").unwrap();

    // Create multiple subtitle files that will all be matched to the same video
    let subtitle1_path = subtitle_dir.join("sub1.srt");
    let subtitle2_path = subtitle_dir.join("sub2.srt");
    let subtitle3_path = subtitle_dir.join("sub3.srt");

    fs::write(
        &subtitle1_path,
        "1\n00:00:01,000 --> 00:00:02,000\nFirst subtitle\n",
    )
    .unwrap();
    fs::write(
        &subtitle2_path,
        "1\n00:00:01,000 --> 00:00:02,000\nSecond subtitle\n",
    )
    .unwrap();
    fs::write(
        &subtitle3_path,
        "1\n00:00:01,000 --> 00:00:02,000\nThird subtitle\n",
    )
    .unwrap();

    // Echo the matcher's own UUIDv7 IDs back at it: pair the single
    // discovered video against every discovered subtitle so the
    // command attempts to rename them all onto the same target name.
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_echoing_one_to_many(1, 3, 0.90)
        .await;

    // Execute match command with copy mode to test rename conflicts
    let args = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(root.to_path_buf()),
        dry_run: false,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
        no_extract: false,
    };

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    let result = match_command::execute(args, &config_service).await;
    assert!(result.is_ok(), "Match command should execute successfully");

    // Verify that all original files still exist (copy mode)
    assert!(
        subtitle1_path.exists(),
        "Original subtitle1 should still exist"
    );
    assert!(
        subtitle2_path.exists(),
        "Original subtitle2 should still exist"
    );
    assert!(
        subtitle3_path.exists(),
        "Original subtitle3 should still exist"
    );

    // Verify that the target files were created with proper conflict resolution
    let expected_targets = vec![
        video_dir.join("movie.srt"),   // First file gets the base name
        video_dir.join("movie.1.srt"), // Second file gets .1 suffix
        video_dir.join("movie.2.srt"), // Third file gets .2 suffix
    ];

    for (i, target_path) in expected_targets.iter().enumerate() {
        assert!(
            target_path.exists(),
            "Target file {} should exist at {:?}",
            i + 1,
            target_path
        );

        // Verify the content matches one of the original files
        let target_content = fs::read_to_string(target_path)
            .unwrap_or_else(|_| panic!("Should be able to read target file {:?}", target_path));

        let original_contents = vec![
            fs::read_to_string(&subtitle1_path).unwrap(),
            fs::read_to_string(&subtitle2_path).unwrap(),
            fs::read_to_string(&subtitle3_path).unwrap(),
        ];

        assert!(
            original_contents.contains(&target_content),
            "Target file content should match one of the original files"
        );
    }

    // Verify that no other files were created
    let video_dir_entries: Vec<_> = fs::read_dir(&video_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().unwrap().is_file())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();

    assert_eq!(
        video_dir_entries.len(),
        4, // 1 video + 3 subtitle files
        "Should have exactly 4 files in video directory: {:?}",
        video_dir_entries
    );

    mock_helper.verify_expectations().await;
}

/// Test sequential conflict resolution when target files already exist
#[tokio::test]
async fn test_conflict_resolution_with_existing_target_files() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    // Create video file
    fs::write(video_dir.join("movie.mp4"), "video content").unwrap();

    // Pre-create target files to test conflict resolution
    fs::write(video_dir.join("movie.srt"), "existing content").unwrap();
    fs::write(video_dir.join("movie.1.srt"), "existing content 1").unwrap();

    // Create subtitle files to be matched
    let subtitle1_path = subtitle_dir.join("sub1.srt");
    let subtitle2_path = subtitle_dir.join("sub2.srt");

    fs::write(
        &subtitle1_path,
        "1\n00:00:01,000 --> 00:00:02,000\nNew subtitle 1\n",
    )
    .unwrap();
    fs::write(
        &subtitle2_path,
        "1\n00:00:01,000 --> 00:00:02,000\nNew subtitle 2\n",
    )
    .unwrap();

    // Discover the on-disk file set without pre-baking IDs into a
    // static mock: the matcher generates fresh UUIDv7 IDs on every
    // scan, so we mount a closure-based wiremock responder that parses
    // the captured request and pairs the video against only the new
    // sub1/sub2 subtitles (filtering out the pre-existing movie.srt /
    // movie.1.srt entries by filename).
    let mock_helper = MockOpenAITestHelper::new().await;
    mount_filename_filtered_video_to_subs_mock(
        mock_helper.server(),
        &["sub1.srt", "sub2.srt"],
        0.90,
    )
    .await;

    // Execute match command
    let args = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(root.to_path_buf()),
        dry_run: false,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
        no_extract: false,
    };

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    let result = match_command::execute(args, &config_service).await;
    assert!(result.is_ok(), "Match command should execute successfully");

    // Verify existing files remain unchanged
    assert_eq!(
        fs::read_to_string(video_dir.join("movie.srt")).unwrap(),
        "existing content",
        "Pre-existing movie.srt should remain unchanged"
    );
    assert_eq!(
        fs::read_to_string(video_dir.join("movie.1.srt")).unwrap(),
        "existing content 1",
        "Pre-existing movie.1.srt should remain unchanged"
    );

    // Verify new files were created with higher numeric suffixes
    assert!(
        video_dir.join("movie.2.srt").exists(),
        "New file should be created as movie.2.srt"
    );
    assert!(
        video_dir.join("movie.3.srt").exists(),
        "New file should be created as movie.3.srt"
    );

    // Verify content of new files
    let new_content_1 = fs::read_to_string(video_dir.join("movie.2.srt")).unwrap();
    let new_content_2 = fs::read_to_string(video_dir.join("movie.3.srt")).unwrap();

    let original_contents = vec![
        fs::read_to_string(&subtitle1_path).unwrap(),
        fs::read_to_string(&subtitle2_path).unwrap(),
    ];

    assert!(
        original_contents.contains(&new_content_1),
        "movie.2.srt should contain content from one of the original files"
    );
    assert!(
        original_contents.contains(&new_content_2),
        "movie.3.srt should contain content from one of the original files"
    );

    mock_helper.verify_expectations().await;
}

/// Test move mode with conflict resolution
#[tokio::test]
async fn test_move_mode_with_duplicate_rename_conflicts() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    // Create video file
    fs::write(video_dir.join("movie.mp4"), "video content").unwrap();

    // Create subtitle files to be moved
    let subtitle1_path = subtitle_dir.join("sub1.srt");
    let subtitle2_path = subtitle_dir.join("sub2.srt");

    fs::write(
        &subtitle1_path,
        "1\n00:00:01,000 --> 00:00:02,000\nFirst subtitle\n",
    )
    .unwrap();
    fs::write(
        &subtitle2_path,
        "1\n00:00:01,000 --> 00:00:02,000\nSecond subtitle\n",
    )
    .unwrap();

    // Store original content for verification
    let original_content_1 = fs::read_to_string(&subtitle1_path).unwrap();
    let original_content_2 = fs::read_to_string(&subtitle2_path).unwrap();

    // Echo IDs back: pair the single video against both discovered
    // subtitles so the matcher attempts to rename them both to the
    // same target name.
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_echoing_one_to_many(1, 2, 0.90)
        .await;

    // Execute match command with move mode
    let args = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(root.to_path_buf()),
        dry_run: false,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
        no_extract: false,
    };

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    let result = match_command::execute(args, &config_service).await;
    assert!(result.is_ok(), "Match command should execute successfully");

    // Verify original files were moved (no longer exist in original location)
    assert!(
        !subtitle1_path.exists(),
        "Original subtitle1 should be moved"
    );
    assert!(
        !subtitle2_path.exists(),
        "Original subtitle2 should be moved"
    );

    // Verify target files exist with conflict resolution naming
    let target_files = vec![video_dir.join("movie.srt"), video_dir.join("movie.1.srt")];

    for target_path in &target_files {
        assert!(
            target_path.exists(),
            "Target file should exist at {:?}",
            target_path
        );
    }

    // Verify content was preserved during move
    let moved_content_1 = fs::read_to_string(&target_files[0]).unwrap();
    let moved_content_2 = fs::read_to_string(&target_files[1]).unwrap();

    let original_contents = vec![original_content_1, original_content_2];

    assert!(
        original_contents.contains(&moved_content_1),
        "First moved file should contain original content"
    );
    assert!(
        original_contents.contains(&moved_content_2),
        "Second moved file should contain original content"
    );

    mock_helper.verify_expectations().await;
}

/// Mount a wiremock responder that scans the captured prompt for
/// `ID:file_<uuid> | Name:<filename>` pairs, picks the first video ID,
/// and pairs it against every subtitle whose filename appears in
/// `subtitle_filenames`.
async fn mount_filename_filtered_video_to_subs_mock(
    server: &wiremock::MockServer,
    subtitle_filenames: &[&str],
    confidence: f64,
) {
    use serde_json::{Value, json};
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, Request, Respond, ResponseTemplate};

    struct R {
        names: Vec<String>,
        confidence: f64,
    }
    impl Respond for R {
        fn respond(&self, request: &Request) -> ResponseTemplate {
            let body: Value = serde_json::from_slice(&request.body).expect("valid JSON");
            let prompt = body
                .get("messages")
                .and_then(|m| m.as_array())
                .and_then(|arr| arr.last())
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("");
            // Each entry looks like:
            // - ID:file_<uuid> | Name:<filename> | Path:<path>
            let entry_re = regex::Regex::new(
                r"ID:(file_[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}) \| Name:([^|]+?) \| Path:",
            )
            .unwrap();
            let mut entries: Vec<(String, String)> = Vec::new();
            for cap in entry_re.captures_iter(prompt) {
                entries.push((cap[1].to_string(), cap[2].trim().to_string()));
            }
            let video_section_end = prompt.find("\nSubtitle files:\n").unwrap_or(prompt.len());
            let video_id = entries
                .iter()
                .find(|(id, _)| {
                    if let Some(idx) = prompt.find(id.as_str()) {
                        idx < video_section_end
                    } else {
                        false
                    }
                })
                .map(|(id, _)| id.clone())
                .unwrap_or_default();
            let matches: Vec<Value> = entries
                .iter()
                .filter(|(id, name)| id != &video_id && self.names.iter().any(|n| n == name))
                .map(|(id, _)| {
                    json!({
                        "video_file_id": video_id,
                        "subtitle_file_id": id,
                        "confidence": self.confidence,
                        "match_factors": ["filename_similarity", "content_correlation"],
                    })
                })
                .collect();
            let content = json!({
                "matches": matches,
                "confidence": self.confidence,
                "reasoning": "Filename-filtered echo response",
            })
            .to_string();
            let response_body = json!({
                "choices": [
                    { "message": { "content": content }, "finish_reason": "stop" }
                ],
                "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
                "model": "gpt-4.1-mini"
            });
            ResponseTemplate::new(200).set_body_json(response_body)
        }
    }

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer mock-api-key"))
        .respond_with(R {
            names: subtitle_filenames.iter().map(|s| s.to_string()).collect(),
            confidence,
        })
        .mount(server)
        .await;
}
