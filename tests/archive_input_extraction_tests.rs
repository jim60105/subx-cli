//! Integration tests for archive-aware input handling in `InputPathHandler`.
//!
//! These tests cover:
//! - Group 7: Archive-aware `collect_files`
//! - Group 9: Output directory resolution (archive origin for regular files)
//! - Group 10: Error handling and edge cases (corrupted, nested, empty archives)

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use subx_cli::cli::InputPathHandler;

/// Helper: create a zip archive at `zip_path` with the given `(name, content)` entries.
fn create_zip(zip_path: &Path, entries: &[(&str, &[u8])]) {
    let file = fs::File::create(zip_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, content) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap();
}

// ---------------------------------------------------------------------------
// Group 7: Archive-aware collect_files
// ---------------------------------------------------------------------------

#[test]
fn test_zip_input_produces_extracted_files() {
    let tmp = TempDir::new().unwrap();
    let zip_path = tmp.path().join("subs.zip");
    create_zip(
        &zip_path,
        &[
            ("a.srt", b"1\n00:00:01,000 --> 00:00:02,000\nHello\n"),
            ("b.srt", b"1\n00:00:01,000 --> 00:00:02,000\nWorld\n"),
        ],
    );

    let handler = InputPathHandler::from_args(std::slice::from_ref(&zip_path), false)
        .unwrap()
        .with_extensions(&["srt", "ass", "vtt", "sub"]);
    let collected = handler.collect_files().unwrap();

    assert_eq!(collected.len(), 2);
    for p in collected.iter() {
        assert_ne!(p, &zip_path, "zip path itself should not be returned");
        assert_eq!(p.extension().and_then(|e| e.to_str()), Some("srt"));
        assert!(p.exists(), "extracted file should exist on disk");
    }
}

#[test]
fn test_mixed_archive_directory_file_input() {
    let tmp = TempDir::new().unwrap();

    // 1. Zip archive containing two subtitles
    let zip_path = tmp.path().join("archive.zip");
    create_zip(
        &zip_path,
        &[("zipped1.srt", b"content1"), ("zipped2.srt", b"content2")],
    );

    // 2. Directory with an srt file and a non-matching txt file
    let dir = tmp.path().join("subs_dir");
    fs::create_dir(&dir).unwrap();
    let dir_srt = dir.join("dir_file.srt");
    fs::write(&dir_srt, b"from dir").unwrap();
    fs::write(dir.join("notes.txt"), b"ignore me").unwrap();

    // 3. Direct .srt file
    let direct_srt = tmp.path().join("direct.srt");
    fs::write(&direct_srt, b"direct content").unwrap();

    let handler =
        InputPathHandler::from_args(&[zip_path.clone(), dir.clone(), direct_srt.clone()], false)
            .unwrap()
            .with_extensions(&["srt", "ass", "vtt", "sub"]);

    let collected = handler.collect_files().unwrap();

    // 2 from zip + 1 from directory + 1 direct = 4
    assert_eq!(collected.len(), 4, "collected = {:?}", &*collected);
    assert!(collected.contains(&direct_srt));
    assert!(collected.contains(&dir_srt));
    // Two files originated from the zip
    let from_zip: Vec<_> = collected
        .iter()
        .filter(|p| collected.archive_origin(p).is_some())
        .collect();
    assert_eq!(from_zip.len(), 2);
}

#[test]
fn test_no_extract_skips_extraction() {
    let tmp = TempDir::new().unwrap();
    let zip_path = tmp.path().join("subs.zip");
    create_zip(&zip_path, &[("a.srt", b"hello")]);

    let handler = InputPathHandler::from_args(std::slice::from_ref(&zip_path), false)
        .unwrap()
        .with_extensions(&["srt", "ass", "vtt", "sub"])
        .with_no_extract(true);
    let collected = handler.collect_files().unwrap();

    // zip extension doesn't match filter, so no files are returned
    // and no archive origins are tracked.
    assert!(
        collected.is_empty(),
        "no_extract with non-matching extension filter should yield no files, got {:?}",
        &*collected
    );
    assert!(collected.archive_origin(&zip_path).is_none());
}

#[test]
fn test_archive_in_traversed_directory_not_extracted() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("bundle");
    fs::create_dir(&dir).unwrap();

    // Put a zip inside the directory.
    let inner_zip = dir.join("inner.zip");
    create_zip(&inner_zip, &[("inside.srt", b"should not appear")]);

    // Also put a regular subtitle file.
    let srt = dir.join("real.srt");
    fs::write(&srt, b"real content").unwrap();

    let handler = InputPathHandler::from_args(std::slice::from_ref(&dir), false)
        .unwrap()
        .with_extensions(&["srt", "ass", "vtt", "sub"]);
    let collected = handler.collect_files().unwrap();

    // Only the direct .srt should be found - the zip inside the directory is
    // not extracted, and .zip doesn't match the extension filter either.
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0], srt);
    // No archive origins because no archive was extracted.
    assert!(collected.archive_origin(&srt).is_none());
}

#[test]
fn test_collected_files_temp_dir_cleanup() {
    let tmp = TempDir::new().unwrap();
    let zip_path = tmp.path().join("subs.zip");
    create_zip(&zip_path, &[("a.srt", b"hello")]);

    let handler = InputPathHandler::from_args(std::slice::from_ref(&zip_path), false)
        .unwrap()
        .with_extensions(&["srt", "ass", "vtt", "sub"]);
    let collected = handler.collect_files().unwrap();

    assert_eq!(collected.len(), 1);
    let extracted_path = collected[0].clone();
    assert!(extracted_path.exists(), "extracted file should exist");

    // Determine the temp root (the parent chain up to but not including the
    // system temp directory). We rely on the fact that the extracted file
    // lives inside a TempDir owned by `collected`.
    let temp_root_parent = extracted_path.parent().unwrap().to_path_buf();

    drop(collected);

    // After drop, the temp directory should be cleaned up.
    assert!(
        !extracted_path.exists(),
        "extracted file should be removed after CollectedFiles is dropped"
    );
    assert!(
        !temp_root_parent.exists()
            || fs::read_dir(&temp_root_parent)
                .map(|mut d| d.next().is_none())
                .unwrap_or(true),
        "temp directory should be cleaned up after drop"
    );
}

#[test]
fn test_archive_origin_resolves_correctly() {
    let tmp = TempDir::new().unwrap();
    let zip_path = tmp.path().join("pack.zip");
    create_zip(&zip_path, &[("ep1.srt", b"a"), ("ep2.srt", b"b")]);

    // Also include a regular file directly.
    let regular = tmp.path().join("loose.srt");
    fs::write(&regular, b"loose").unwrap();

    let handler = InputPathHandler::from_args(&[zip_path.clone(), regular.clone()], false)
        .unwrap()
        .with_extensions(&["srt", "ass", "vtt", "sub"]);
    let collected = handler.collect_files().unwrap();

    assert_eq!(collected.len(), 3);

    let mut extracted_count = 0;
    for p in collected.iter() {
        if p == &regular {
            assert!(collected.archive_origin(p).is_none());
        } else {
            let origin = collected
                .archive_origin(p)
                .expect("extracted files should have an archive origin");
            assert_eq!(origin, zip_path.as_path());
            extracted_count += 1;
        }
    }
    assert_eq!(extracted_count, 2);
}

// ---------------------------------------------------------------------------
// Group 9: Output directory resolution
// ---------------------------------------------------------------------------

#[test]
fn test_archive_origin_returns_none_for_regular_files() {
    let tmp = TempDir::new().unwrap();
    let f1 = tmp.path().join("a.srt");
    let f2 = tmp.path().join("b.srt");
    fs::write(&f1, b"a").unwrap();
    fs::write(&f2, b"b").unwrap();

    let handler = InputPathHandler::from_args(&[f1.clone(), f2.clone()], false)
        .unwrap()
        .with_extensions(&["srt", "ass", "vtt", "sub"]);
    let collected = handler.collect_files().unwrap();

    assert_eq!(collected.len(), 2);
    for p in collected.iter() {
        assert!(
            collected.archive_origin(p).is_none(),
            "regular file should not report an archive origin"
        );
    }
}

// ---------------------------------------------------------------------------
// Group 10: Error handling and edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_corrupted_archive_skipped_gracefully() {
    let tmp = TempDir::new().unwrap();

    // A file with .zip extension but clearly invalid content.
    let bad_zip = tmp.path().join("broken.zip");
    fs::write(&bad_zip, b"this is not a real zip file").unwrap();

    // A good subtitle file alongside it, to verify processing continues.
    let good = tmp.path().join("good.srt");
    fs::write(&good, b"valid").unwrap();

    let handler = InputPathHandler::from_args(&[bad_zip.clone(), good.clone()], false)
        .unwrap()
        .with_extensions(&["srt", "ass", "vtt", "sub"]);

    let collected = handler
        .collect_files()
        .expect("collect_files should not fail when an archive is corrupted");

    // Corrupted archive is skipped (logged as warning); the good .srt remains.
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0], good);
}

#[test]
fn test_nested_archive_not_extracted() {
    let tmp = TempDir::new().unwrap();

    // Build inner zip first in a scratch location.
    let inner_scratch = TempDir::new().unwrap();
    let inner_zip_src = inner_scratch.path().join("inner.zip");
    create_zip(&inner_zip_src, &[("deep.srt", b"deep content")]);
    let inner_bytes = fs::read(&inner_zip_src).unwrap();

    // Outer zip contains the inner zip plus a regular subtitle.
    let outer_zip = tmp.path().join("outer.zip");
    create_zip(
        &outer_zip,
        &[("inner.zip", &inner_bytes), ("top.srt", b"top content")],
    );

    let handler = InputPathHandler::from_args(std::slice::from_ref(&outer_zip), false)
        .unwrap()
        .with_extensions(&["srt", "ass", "vtt", "sub"]);
    let collected = handler.collect_files().unwrap();

    // Only `top.srt` should be returned. The nested `inner.zip` is extracted
    // as a file on disk but not recursively extracted, and .zip does not
    // match the extension filter.
    assert_eq!(collected.len(), 1, "collected = {:?}", &*collected);
    let only = &collected[0];
    assert_eq!(only.file_name().and_then(|s| s.to_str()), Some("top.srt"));
    assert!(collected.archive_origin(only).is_some());
    // And the nested zip exists on disk but is not part of the collected list.
    assert!(
        collected
            .iter()
            .all(|p| p.extension().and_then(|e| e.to_str()) != Some("zip")),
        "nested zip should not appear in collected files"
    );
}

#[test]
fn test_empty_archive_produces_no_files() {
    let tmp = TempDir::new().unwrap();
    let zip_path = tmp.path().join("empty.zip");
    create_zip(&zip_path, &[]);

    let handler = InputPathHandler::from_args(std::slice::from_ref(&zip_path), false)
        .unwrap()
        .with_extensions(&["srt", "ass", "vtt", "sub"]);
    let collected = handler.collect_files().unwrap();

    assert!(collected.is_empty());
}

// Silence unused import warning on Windows (if any of the helpers are
// conditionally compiled in future; currently all tests are cross-platform).
#[allow(dead_code)]
fn _unused(_p: PathBuf) {}
