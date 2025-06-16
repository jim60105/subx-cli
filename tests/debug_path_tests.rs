#[cfg(test)]
mod debug_path_tests {
    use std::fs;
    use subx_cli::cli::{InputPathHandler, MatchArgs};
    use tempfile::TempDir;

    #[test]
    fn debug_get_directories_behavior() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let video_dir = root.join("videos");
        let subtitle_dir = root.join("subtitles");
        fs::create_dir_all(&video_dir).unwrap();
        fs::create_dir_all(&subtitle_dir).unwrap();

        fs::write(video_dir.join("movie.mp4"), "video").unwrap();
        fs::write(subtitle_dir.join("movie.srt"), b"content").unwrap();

        // Test the get_input_handler and get_directories logic
        let args = MatchArgs {
            input_paths: vec![],
            recursive: false,
            path: Some(root.to_path_buf()),
            dry_run: false,
            confidence: 80,
            backup: false,
            copy: true,
            move_files: false,
        };

        let input_handler = args.get_input_handler().unwrap();
        println!("Input handler paths: {:?}", input_handler.paths);

        let directories = input_handler.get_directories();
        println!("Directories: {:?}", directories);

        // What should happen: directories should contain root
        assert_eq!(directories.len(), 1);
        assert!(directories.contains(&root.to_path_buf()));

        // Now test FileDiscovery directly
        let discovery = subx_cli::core::matcher::FileDiscovery::new();
        let files = discovery.scan_directory(root, true).unwrap();
        println!("Files found: {:?}", files.len());
        for file in &files {
            println!("  {:?}: {:?} (ID: {})", file.file_type, file.path, file.id);
        }

        let videos: Vec<_> = files
            .iter()
            .filter(|f| matches!(f.file_type, subx_cli::core::matcher::MediaFileType::Video))
            .collect();
        let subtitles: Vec<_> = files
            .iter()
            .filter(|f| {
                matches!(
                    f.file_type,
                    subx_cli::core::matcher::MediaFileType::Subtitle
                )
            })
            .collect();

        println!("Videos found: {}", videos.len());
        if !videos.is_empty() {
            println!("  Video ID: {}", videos[0].id);
        }
        println!("Subtitles found: {}", subtitles.len());
        if !subtitles.is_empty() {
            println!("  Subtitle ID: {}", subtitles[0].id);
        }

        assert!(!videos.is_empty(), "Should find video files");
        assert!(!subtitles.is_empty(), "Should find subtitle files");
    }
}
