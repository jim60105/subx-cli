use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use subx_cli::cli::InputPathHandler;
use subx_cli::error::SubXError;

#[test]
fn test_input_handler_from_files() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("a.txt");
    fs::write(&file, "content").unwrap();

    let handler = InputPathHandler::from_args(&[file.clone()], false).unwrap();
    let files = handler.collect_files().unwrap();
    assert_eq!(files, vec![file]);
}

#[test]
fn test_input_handler_from_directories_flat() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();
    fs::write(dir.join("a.srt"), "").unwrap();
    fs::write(dir.join("b.ass"), "").unwrap();
    // nested
    let nested = dir.join("nest");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("c.srt"), "").unwrap();

    let handler = InputPathHandler::from_args(&[dir.to_path_buf()], false)
        .unwrap()
        .with_extensions(&["srt", "ass"]);
    let mut files = handler.collect_files().unwrap();
    files.sort();
    assert_eq!(files, vec![dir.join("a.srt"), dir.join("b.ass")]);
}

#[test]
fn test_input_handler_from_directories_recursive() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();
    fs::write(dir.join("a.srt"), "").unwrap();
    let nested = dir.join("nest");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("b.ass"), "").unwrap();

    let handler = InputPathHandler::from_args(&[dir.to_path_buf()], true)
        .unwrap()
        .with_extensions(&["srt", "ass"]);
    let mut files = handler.collect_files().unwrap();
    files.sort();
    assert_eq!(files, vec![dir.join("a.srt"), nested.join("b.ass")]);
}

#[test]
fn test_input_handler_invalid_path() {
    let err = InputPathHandler::from_args(&[PathBuf::from("no/such/path")], false)
        .unwrap_err();
    match err {
        SubXError::PathNotFound(p) => assert!(p.ends_with("no/such/path")),
        _ => panic!("unexpected error: {:?}", err),
    }
}
