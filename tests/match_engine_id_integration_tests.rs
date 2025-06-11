use async_trait::async_trait;
use std::fs;
use subx_cli::core::matcher::{
    ConflictResolution, FileDiscovery, FileRelocationMode, MatchConfig, MatchEngine,
};
use subx_cli::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, FileMatch, MatchResult, VerificationRequest,
};
use tempfile::TempDir;

/// Mock AI client that returns file ID-based matches
struct MockAIClientWithIds;

#[async_trait]
impl AIProvider for MockAIClientWithIds {
    async fn analyze_content(&self, request: AnalysisRequest) -> subx_cli::Result<MatchResult> {
        // Extract file IDs from the request
        let video_ids: Vec<String> = request
            .video_files
            .iter()
            .filter_map(|f| {
                if f.starts_with("ID:") {
                    f.split('|').next()?.strip_prefix("ID:")
                } else {
                    None
                }
            })
            .map(|s| s.trim().to_string())
            .collect();

        let subtitle_ids: Vec<String> = request
            .subtitle_files
            .iter()
            .filter_map(|f| {
                if f.starts_with("ID:") {
                    f.split('|').next()?.strip_prefix("ID:")
                } else {
                    None
                }
            })
            .map(|s| s.trim().to_string())
            .collect();

        // Create matches based on IDs (simple 1:1 mapping for test)
        let mut matches = Vec::new();
        for (i, video_id) in video_ids.iter().enumerate() {
            if let Some(subtitle_id) = subtitle_ids.get(i) {
                matches.push(FileMatch {
                    video_file_id: video_id.clone(),
                    subtitle_file_id: subtitle_id.clone(),
                    confidence: 0.95,
                    match_factors: vec!["id_based_test_match".to_string()],
                });
            }
        }

        Ok(MatchResult {
            matches,
            confidence: 0.9,
            reasoning: "Mock AI analysis using file IDs".to_string(),
        })
    }

    async fn verify_match(
        &self,
        _request: VerificationRequest,
    ) -> subx_cli::Result<ConfidenceScore> {
        Ok(ConfidenceScore {
            score: 0.9,
            factors: vec!["test_factor".to_string()],
        })
    }
}

#[tokio::test]
async fn test_file_id_based_matching_integration() {
    // Create test directory with files
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create test files that simulate the user's bug report scenario
    fs::write(root.join("video1.mkv"), b"video content 1").unwrap();
    fs::write(root.join("video2.mkv"), b"video content 2").unwrap();
    fs::write(root.join("subtitle1.srt"), b"subtitle content 1").unwrap();
    fs::write(root.join("subtitle2.srt"), b"subtitle content 2").unwrap();

    // Discover files and verify they have IDs
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(root, false).unwrap();

    // Verify all files have proper IDs
    for file in &files {
        assert!(!file.id.is_empty());
        assert!(file.id.starts_with("file_"));
        assert_eq!(file.id.len(), 21); // "file_" + 16 hex chars
        assert!(file.name.contains('.')); // Full filename with extension
    }

    // Test the matching engine with ID-based AI client
    let config = MatchConfig {
        confidence_threshold: 0.8,
        max_sample_length: 1024,
        enable_content_analysis: true,
        backup_enabled: false,
        relocation_mode: FileRelocationMode::None,
        conflict_resolution: ConflictResolution::AutoRename,
    };

    let engine = MatchEngine::new(Box::new(MockAIClientWithIds), config);
    let operations = engine.match_files(root, false).await.unwrap();

    // Verify that operations were created (not the "No matching file pairs found" case)
    assert!(
        !operations.is_empty(),
        "Expected match operations to be generated"
    );

    // Verify that the operations contain the correct file references
    for op in &operations {
        assert!(!op.video_file.id.is_empty());
        assert!(!op.subtitle_file.id.is_empty());
        assert!(op.confidence >= 0.8);
    }

    println!(
        "✅ ID-based matching integration test passed: {} operations generated",
        operations.len()
    );
}

#[tokio::test]
async fn test_user_reported_bug_15_scenario() {
    // Recreate the exact file structure from the bug report
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create the files mentioned in bug #15
    let files = vec![
        "[Noumin Kanren no Skill][01][BDRIP][1080P][H264_FLACx2].mkv",
        "[Yozakura-san Chi no Daisakusen][01][BDRIP][1080P][H264_AC3].mkv",
        "[Yozakura-san Chi no Daisakusen][02][BDRIP][1080P][H264_AC3].mkv",
        "[Yozakura-san Chi no Daisakusen][03][BDRIP][1080P][H264_AC3].mkv",
        "Noumin Kanren no Skill S01E01-[1080p][BDRIP][x265.FLAC].cht.srt",
        "Noumin Kanren no Skill S01E01-[1080p][BDRIP][x265.FLAC].cht.vtt",
        "夜桜さんちの大作戦 第01話 「桜の指輪」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass",
        "夜桜さんちの大作戦 第02話 「夜桜の命」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass",
        "夜桜さんちの大作戦 第03話 「気持ち」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass",
    ];

    for filename in files {
        fs::write(
            root.join(filename),
            format!("content for {}", filename).as_bytes(),
        )
        .unwrap();
    }

    // Discover files
    let discovery = FileDiscovery::new();
    let media_files = discovery.scan_directory(root, false).unwrap();

    // Verify we have the expected number of files
    let videos: Vec<_> = media_files
        .iter()
        .filter(|f| {
            matches!(
                f.file_type,
                subx_cli::core::matcher::discovery::MediaFileType::Video
            )
        })
        .collect();
    let subtitles: Vec<_> = media_files
        .iter()
        .filter(|f| {
            matches!(
                f.file_type,
                subx_cli::core::matcher::discovery::MediaFileType::Subtitle
            )
        })
        .collect();

    assert_eq!(videos.len(), 4, "Expected 4 video files");
    assert_eq!(subtitles.len(), 5, "Expected 5 subtitle files");

    // Verify all files have unique IDs
    let mut all_ids = std::collections::HashSet::new();
    for file in &media_files {
        assert!(
            all_ids.insert(file.id.clone()),
            "Duplicate ID found: {}",
            file.id
        );
        assert!(file.id.starts_with("file_"));
        // Verify filename includes extension (fixes the original bug)
        assert!(
            file.name.contains('.'),
            "Filename should include extension: {}",
            file.name
        );
    }

    println!(
        "✅ Bug #15 scenario test passed: {} unique files with IDs",
        media_files.len()
    );
}
