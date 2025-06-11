use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use subx_cli::core::matcher::engine::resolve_filename_conflict;
use subx_cli::core::parallel::{FileRelocationTask, ProcessingOperation, TaskResult};
use tempfile::TempDir;

#[tokio::test]
async fn test_resolve_filename_conflict_basic() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("file.txt");
    let mut f = File::create(&file).unwrap();
    writeln!(f, "content").unwrap();
    let new_path = resolve_filename_conflict(file.clone());
    assert_eq!(
        new_path.file_name().unwrap().to_str().unwrap(),
        "file.1.txt"
    );
}

#[tokio::test]
async fn test_copy_to_video_folder_basic() {
    let temp = TempDir::new().unwrap();
    let src_dir = temp.path().join("src");
    let dst_dir = temp.path().join("dst");
    fs::create_dir(&src_dir).unwrap();
    fs::create_dir(&dst_dir).unwrap();
    let src_file = src_dir.join("sub.srt");
    fs::write(&src_file, b"hello").unwrap();
    let dst_file = dst_dir.join("sub.srt");
    let task = FileRelocationTask {
        operation: ProcessingOperation::CopyToVideoFolder {
            source: src_file.clone(),
            target: dst_file.clone(),
        },
        backup_enabled: false,
    };
    let result = task.execute();
    assert!(matches!(result, TaskResult::Success(_)));
    assert!(dst_file.exists() && src_file.exists());
    assert_eq!(fs::read(&dst_file).unwrap(), b"hello");
}

#[tokio::test]
async fn test_move_to_video_folder_basic() {
    let temp = TempDir::new().unwrap();
    let src_dir = temp.path().join("src");
    let dst_dir = temp.path().join("dst");
    fs::create_dir(&src_dir).unwrap();
    fs::create_dir(&dst_dir).unwrap();
    let src_file = src_dir.join("sub.srt");
    fs::write(&src_file, b"bye").unwrap();
    let dst_file = dst_dir.join("sub.srt");
    let task = FileRelocationTask {
        operation: ProcessingOperation::MoveToVideoFolder {
            source: src_file.clone(),
            target: dst_file.clone(),
        },
        backup_enabled: false,
    };
    let result = task.execute();
    assert!(matches!(result, TaskResult::Success(_)));
    assert!(dst_file.exists() && !src_file.exists());
    assert_eq!(fs::read(&dst_file).unwrap(), b"bye");
}
