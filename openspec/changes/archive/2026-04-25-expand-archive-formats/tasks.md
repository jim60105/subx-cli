## 1. SRP Refactor — Module Structure

- [x] 1.1 Convert `src/core/archive.rs` to `src/core/archive/mod.rs` (move file, preserve public API)
- [x] 1.2 Extract shared validation helpers into `src/core/archive/common.rs`: `validate_entry_path()`, `ExtractionLimits` (size/count tracker), constants (MAX_EXPANDED_SIZE, MAX_ENTRY_COUNT)
- [x] 1.3 Extract ZIP extraction into `src/core/archive/zip.rs` with `pub(super) fn extract_zip()`
- [x] 1.4 Extract RAR extraction into `src/core/archive/rar.rs` with `pub(super) fn extract_rar()` (feature-gated)
- [x] 1.5 Verify all existing tests pass after refactor (`cargo nextest run || true`)

## 2. Feasibility & Dependencies

- [x] 2.1 Validate `sevenz-rust` API: confirm entry-by-entry extraction via `decompress_with_extract_fn`, verify entry type/anti-item detection, and assess dictionary memory allocation control
- [x] 2.2 Add `sevenz-rust`, `tar`, and `flate2` as always-on dependencies in `Cargo.toml`
- [x] 2.3 Add `SevenZip` and `TarGz` variants to `ArchiveFormat` enum in `src/core/archive/mod.rs`
- [x] 2.4 Update `detect_format()` to recognise `.7z`, `.tar.gz` (compound extension via case-insensitive `file_name()` suffix), and `.tgz` (case-insensitive)
- [x] 2.5 Add dispatch arms for `SevenZip` and `TarGz` in `extract_archive()`

## 3. 7z Extraction

- [x] 3.1 Implement `src/core/archive/sevenz.rs` with `pub(super) fn extract_7z(archive_path, dest_dir) -> io::Result<Vec<PathBuf>>` using `sevenz-rust` with `common::validate_entry_path()`, `common::ExtractionLimits`, anti-item/symlink handling, and empty-archive handling
- [x] 3.2 Add unit tests for `extract_7z`: valid extraction, path-traversal rejection, empty archive, bomb protection (size and count limits)

## 4. Tar-Gzip Extraction

- [x] 4.1 Implement `src/core/archive/targz.rs` with `pub(super) fn extract_tar_gz(archive_path, dest_dir) -> io::Result<Vec<PathBuf>>` using `tar` + `flate2` with manual per-entry iteration, `common::validate_entry_path()`, `common::ExtractionLimits`, entry-type filtering (Regular + Directory only), and empty-archive handling
- [x] 4.2 Add unit tests for `extract_tar_gz`: valid extraction, symlink rejection, hard-link rejection, path-traversal rejection, empty archive, bomb protection (size and count limits)

## 5. Format Detection Tests

- [x] 5.1 Add unit tests for `detect_format()` covering `.7z`, `.tar.gz`, `.tgz`, case-insensitive `.TAR.GZ`, and negative cases (`.tar.bz2`, `.gz`)

## 6. Integration Tests

- [x] 6.1 Add integration tests in `tests/archive_input_extraction_tests.rs` for 7z input via `-i` flag: extraction, `--no-extract` bypass, archive-origin mapping, corrupted archive handling
- [x] 6.2 Add integration tests for tar.gz/tgz input via `-i` flag: extraction, `--no-extract` bypass, archive-origin mapping, corrupted archive handling
- [x] 6.3 Add integration test for mixed archive types in single input list (zip + 7z + tar.gz)

## 7. Media Discovery Update

- [x] 7.1 Verify that `.7z`, `.tar.gz`, `.tgz` are already excluded from media classification by FileDiscovery (they are not in the recognized extension sets); add explicit test assertions

## 8. Documentation & Quality

- [x] 8.1 Update `src/core/archive/mod.rs` module-level rustdoc to mention 7z and tar.gz support and the module structure
- [x] 8.2 Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo clippy --all-features -- -D warnings`
- [x] 8.3 Run `timeout 240 scripts/quality_check.sh` and fix any issues
- [x] 8.4 Run `timeout 240 scripts/check_coverage.sh -T` to verify coverage is not degraded
