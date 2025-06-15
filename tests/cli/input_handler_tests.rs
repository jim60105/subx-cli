//! 單元測試：InputPathHandler 檔案與目錄處理

use crate::cli::InputPathHandler;
use crate::error::SubXError;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_input_handler_from_files() -> Result<(), SubXError> {
    let tmp = TempDir::new().unwrap();
    let f1 = tmp.path().join("a.srt");
    let f2 = tmp.path().join("b.ass");
    fs::write(&f1, b"").unwrap();
    fs::write(&f2, b"").unwrap();

    let handler = InputPathHandler::from_args(&[f1.clone(), f2.clone()], false)?;
    let mut files = handler.collect_files()?;
    files.sort();
    assert_eq!(files, vec![f1.clone(), f2.clone()]);
    Ok(())
}

#[test]
fn test_input_handler_from_directories() -> Result<(), SubXError> {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let f1 = dir.join("a.srt");
    let f2 = dir.join("b.txt");
    fs::write(&f1, b"").unwrap();
    fs::write(&f2, b"").unwrap();

    let handler = InputPathHandler::from_args(&[dir.to_path_buf()], false)?
        .with_extensions(&["srt"]);
    let files = handler.collect_files()?;
    assert_eq!(files, vec![f1.clone()]);
    Ok(())
}

#[test]
fn test_input_handler_mixed() -> Result<(), SubXError> {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("d");
    fs::create_dir(&dir).unwrap();
    let f1 = dir.join("a.srt");
    fs::write(&f1, b"").unwrap();
    let f2 = tmp.path().join("b.srt");
    fs::write(&f2, b"").unwrap();

    let handler = InputPathHandler::from_args(&[dir.clone(), f2.clone()], false)?
        .with_extensions(&["srt"]);
    let mut files = handler.collect_files()?;
    files.sort();
    assert_eq!(files, vec![f1.clone(), f2.clone()]);
    Ok(())
}

#[test]
fn test_input_handler_validation() {
    let err = InputPathHandler::from_args(&[PathBuf::from("nonexistent")], false)
        .unwrap_err();
    match err {
        SubXError::PathNotFound(p) => assert_eq!(p, PathBuf::from("nonexistent")),
        _ => panic!("Expected PathNotFound error"),
    }
}

#[test]
fn test_directory_scanning_flat_and_recursive() -> Result<(), SubXError> {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let f1 = dir.join("a.srt");
    let nested = dir.join("nested");
    fs::create_dir(&nested).unwrap();
    let f2 = nested.join("b.srt");
    fs::write(&f1, b"").unwrap();
    fs::write(&f2, b"").unwrap();

    let handler_flat = InputPathHandler::from_args(&[dir.to_path_buf()], false)?
        .with_extensions(&["srt"]);
    let files_flat = handler_flat.collect_files()?;
    assert_eq!(files_flat, vec![f1.clone()]);

    let handler_rec = InputPathHandler::from_args(&[dir.to_path_buf()], true)?
        .with_extensions(&["srt"]);
    let mut files_rec = handler_rec.collect_files()?;
    files_rec.sort();
    assert_eq!(files_rec, vec![f1.clone(), f2.clone()]);
    Ok(())
}

#[test]
fn test_file_extension_filtering() -> Result<(), SubXError> {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let f1 = dir.join("a.srt");
    let f2 = dir.join("b.ass");
    let f3 = dir.join("c.txt");
    fs::write(&f1, b"").unwrap();
    fs::write(&f2, b"").unwrap();
    fs::write(&f3, b"").unwrap();

    let handler = InputPathHandler::from_args(&[dir.to_path_buf()], false)?
        .with_extensions(&["ass"]);
    let files = handler.collect_files()?;
    assert_eq!(files, vec![f2.clone()]);
    Ok(())
}
