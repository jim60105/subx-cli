## 1. Dependencies and Module Scaffolding

- [x] 1.1 Add `zip` crate to `[dependencies]` in `Cargo.toml`
- [x] 1.2 Add `unrar` crate to `[dependencies]` in `Cargo.toml` behind `archive-rar` feature flag (disabled by default)
- [x] 1.3 Promote `tempfile` from `[dev-dependencies]` to `[dependencies]`
- [x] 1.4 Create `src/core/archive.rs` module file with public interface stubs and register it in `src/core/mod.rs`
- [x] 1.5 Update CI workflows to install `libunrar-dev` and add a build/test job with `--features archive-rar` enabled

## 2. Archive Format Detection

- [x] 2.1 Implement `ArchiveFormat` enum (`Zip`, `Rar`) and `detect_format(path) -> Option<ArchiveFormat>` by case-insensitive extension matching
- [x] 2.2 Unit tests for format detection: `.zip`, `.ZIP`, `.rar`, `.Rar`, `.tar.gz` (None), `.srt` (None), no extension (None)

## 3. Zip Extraction

- [x] 3.1 Implement `extract_zip(archive_path, dest_dir) -> Result<Vec<PathBuf>>` using `zip` crate with path-traversal validation (reject entries escaping dest root) and symlink/hardlink rejection
- [x] 3.2 Add decompression bomb protection: track cumulative extracted size (1 GiB limit) and entry count (10,000 limit), abort with warning if exceeded
- [x] 3.3 Unit tests: valid zip extraction, empty zip, path-traversal entry rejected, symlink entry rejected, size limit exceeded, entry count limit exceeded, password-protected zip produces warning

## 4. RAR Extraction

- [x] 4.1 Implement `extract_rar(archive_path, dest_dir) -> Result<Vec<PathBuf>>` using `unrar` crate with path-traversal validation and symlink rejection, gated behind `#[cfg(feature = "archive-rar")]`
- [x] 4.2 Add decompression bomb protection identical to zip
- [x] 4.3 Implement stub `extract_rar` that returns a descriptive error when `archive-rar` feature is disabled
- [x] 4.4 Unit tests: valid rar extraction (feature-gated), path-traversal rejected, password-protected archive produces warning

## 5. Unified Extraction Dispatcher

- [x] 5.1 Implement `extract_archive(archive_path, dest_dir) -> Result<Vec<PathBuf>>` that dispatches to `extract_zip` or `extract_rar` based on `detect_format`, passes through errors as warnings
- [x] 5.2 Integration tests: zip extraction via dispatcher, rar extraction via dispatcher, unknown format returns None

## 6. CollectedFiles Return Type

- [x] 6.1 Define `CollectedFiles` struct in `src/cli/input_handler.rs` containing `paths: Vec<PathBuf>`, `_temp_dirs: Vec<TempDir>`, and `archive_origins: HashMap<PathBuf, PathBuf>` (temp root → archive path)
- [x] 6.2 Implement `Deref<Target = Vec<PathBuf>>`, `AsRef<[PathBuf]>`, and `into_paths() -> Vec<PathBuf>` for `CollectedFiles`
- [x] 6.3 Implement `archive_origin(&self, path: &Path) -> Option<&Path>` that checks if a path starts with any temp root key and returns the corresponding archive path
- [x] 6.4 Change `InputPathHandler::collect_files()` return type from `Result<Vec<PathBuf>>` to `Result<CollectedFiles>`
- [x] 6.5 Update all call sites of `collect_files()` to work with `CollectedFiles` (match_command, convert_command, sync_command, detect_encoding_command, `DetectEncodingArgs::get_file_paths`, and tests)

## 7. Archive-Aware collect_files

- [x] 7.1 Add `no_extract: bool` field to `InputPathHandler` and a `with_no_extract(bool)` builder method
- [x] 7.2 In `collect_files()`, for each directly-specified `-i` input that `is_file()`: check `detect_format()`; if archive and `!no_extract`, extract to a new `TempDir`, collect extracted paths (filtered by extension whitelist), record temp-root→archive mapping in `archive_origins`, and push the `TempDir` into `CollectedFiles._temp_dirs`. Archives found during directory traversal are NOT extracted.
- [x] 7.3 Handle extraction errors gracefully: log warning with archive name and error, skip the archive, continue with remaining inputs
- [x] 7.4 Integration tests: zip input produces extracted files, mixed archive+directory+file input, `--no-extract` skips extraction, archive inside traversed directory is NOT extracted

## 8. CLI --no-extract Flag

- [x] 8.1 Add `--no-extract` flag to `MatchArgs` and wire it into `get_input_handler()`
- [x] 8.2 Add `--no-extract` flag to `ConvertArgs` and wire it into `get_input_handler()`
- [x] 8.3 Add `--no-extract` flag to `SyncArgs` and wire it into `get_input_handler()` and also into `get_sync_mode()` batch path
- [x] 8.4 Add `--no-extract` flag to `DetectEncodingArgs` and wire it into `get_input_handler()`

## 9. Output Directory Resolution for Archive Sources

- [x] 9.1 In `convert_command`, when source file has an `archive_origin`, resolve default output path to the archive's parent directory instead of the source file's parent
- [x] 9.2 In `sync_command`, when source file has an `archive_origin`, resolve default output path to the archive's parent directory
- [x] 9.3 In `match_command`, when subtitle files originate from an archive, ensure relocation writes to the archive's parent directory (not the temp dir)
- [x] 9.4 Integration tests: convert/sync/match with archive input produce output beside the archive, explicit `-o` overrides archive origin

## 10. Error Handling and Edge Cases

- [x] 10.1 Ensure corrupted archive logs warning and does not abort command
- [x] 10.2 Ensure password-protected zip and rar both log descriptive warnings and continue
- [x] 10.3 Ensure temp dirs are cleaned up on both success and error paths (verify via tests that assert temp dir absence after `CollectedFiles` drop)
- [x] 10.4 Ensure archives inside extracted archives are NOT extracted (test: nested zip contains inner zip, inner zip is skipped)

## 11. Documentation and Polish

- [x] 11.1 Add rustdoc to all public items in `src/core/archive.rs`
- [x] 11.2 Update `docs/command-reference.md` with `--no-extract` flag documentation for all commands
- [x] 11.3 Update module-level docs in `src/cli/input_handler.rs` to mention archive extraction and `CollectedFiles`
- [x] 11.4 Run `cargo fmt`, `cargo clippy -- -D warnings`, and `scripts/quality_check.sh`

## 12. Release Pipeline

- [x] 12.1 Add `--features archive-rar` to `cargo build` in `.github/workflows/release.yml` so published binaries include RAR support (statically linked via `unrar_sys`)
- [x] 12.2 Update design.md D4 to reflect that `archive-rar` is enabled in release builds and `unrar_sys` statically compiles UnRAR source
- [x] 12.3 Update proposal.md and spec.md to reflect release pipeline and static linking
