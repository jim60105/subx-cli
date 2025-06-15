use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use tempfile::TempDir;
use subx_cli::cli::InputPathHandler;
use subx_cli::error::SubXError;

fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut f = File::create(&path).expect("create file");
    write!(f, "{}", content).unwrap();
    path
}

#[test]
fn test_from_args_and_validate() {
    let dir = TempDir::new().unwrap();
    let file = create_temp_file(&dir, "a.txt", "hello");
    let handler = InputPathHandler::from_args(&[file.clone()], false).unwrap();
    assert_eq!(handler.paths, vec![file]);
    assert!(!handler.recursive);
}

#[test]
fn test_file_extension_filtering() {
    let dir = TempDir::new().unwrap();
    let f1 = create_temp_file(&dir, "ok.srt", "x");
    let f2 = create_temp_file(&dir, "no.mp4", "x");
    let handler = InputPathHandler::from_args(&[dir.path().to_path_buf()], false)
        .unwrap()
        .with_extensions(&["srt"]);
    let files = handler.collect_files().unwrap();
    assert_eq!(files, vec![f1]);
}

#[test]
fn test_flat_vs_recursive_scanning() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    let f1 = create_temp_file(&dir, "root.srt", "");
    let subdir = dir.path().join("sub");
    let f2 = create_temp_file(&TempDir::new().unwrap(), "sub.srt", "");
    // flat scan only root
    let handler_flat = InputPathHandler::from_args(&[dir.path().to_path_buf()], false)
        .unwrap()
        .with_extensions(&["srt"]);
    let flat = handler_flat.collect_files().unwrap();
    assert_eq!(flat, vec![f1]);
    // recursive scan includes subdir
    let handler_rec = InputPathHandler::from_args(&[dir.path().to_path_buf()], true)
        .unwrap()
        .with_extensions(&["srt"]);
    let rec = handler_rec.collect_files().unwrap();
    assert!(rec.contains(&f1) && rec.contains(&PathBuf::from("sub/sub.srt")));
}

#[test]
fn test_invalid_path_error() {
    let result = InputPathHandler::from_args(&[PathBuf::from("no_such")], false);
    assert!(matches!(result, Err(SubXError::PathNotFound(_))));
}
