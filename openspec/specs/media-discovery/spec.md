# Media Discovery

## Purpose

Discover and classify media files from a user-provided input path, produce deterministic file IDs used for AI matching and caching, and apply extension-based filtering with configurable recursion. Implemented in `src/core/matcher/discovery.rs` (`FileDiscovery`, `MediaFile`, `MediaFileType`, `generate_file_id`) and consumed by `src/core/matcher/engine.rs` (`MatchEngine`).

## Requirements

### Requirement: Recognized Video and Subtitle Extensions

`FileDiscovery::new` SHALL classify files by lowercased file extension using a fixed set of recognized extensions: video extensions `mp4`, `mkv`, `avi`, `mov`, `wmv`, `flv`, `m4v`, `webm`; subtitle extensions `srt`, `ass`, `vtt`, `sub`, `ssa`, `idx`. Files whose extension is outside both sets SHALL be ignored during scans.

#### Scenario: Video and subtitle files recognized
- **GIVEN** a directory containing `movie.mp4`, `movie.srt`, and `note.txt`
- **WHEN** `FileDiscovery::scan_directory` is called on the directory
- **THEN** the returned list SHALL include entries for `movie.mp4` (classified as `MediaFileType::Video`) and `movie.srt` (classified as `MediaFileType::Subtitle`), and SHALL NOT include `note.txt`

#### Scenario: Extension matching is case-insensitive
- **GIVEN** a file named `Episode.MKV`
- **WHEN** `FileDiscovery::classify_file` is invoked
- **THEN** the file SHALL be classified as `MediaFileType::Video` and the resulting `MediaFile.extension` SHALL equal `mkv`

### Requirement: Deterministic File Identifier

The system SHALL generate a stable identifier for every discovered file via `generate_file_id(path, file_size)`, which SHALL hash the canonicalized (absolute) path and the file size with `std::collections::hash_map::DefaultHasher` and format the result as `file_<16 lowercase hex digits>`. The resulting identifier SHALL therefore be exactly 21 characters long, SHALL start with `file_`, SHALL be identical for repeated calls with the same path and size, and SHALL differ when either the path or the size differs.

#### Scenario: Same path and size produce the same id
- **GIVEN** two calls `generate_file_id(Path::new("test/file.mkv"), 1000)`
- **WHEN** both calls complete
- **THEN** both SHALL return identical strings of the form `file_<16 hex digits>` that start with `file_` and have length 21

#### Scenario: Different size produces a different id
- **GIVEN** `generate_file_id(Path::new("test/file.mkv"), 1000)` and `generate_file_id(Path::new("test/file.mkv"), 2000)`
- **WHEN** both calls complete
- **THEN** the two returned identifiers SHALL NOT be equal

#### Scenario: Absolute path used for hashing
- **GIVEN** a file accessed once via a relative path and once via its absolute path, both pointing at the same on-disk location
- **WHEN** `generate_file_id` is called for each access with the same `file_size`
- **THEN** both calls SHALL return the same identifier, because the path is canonicalized before hashing (falling back to the provided path when `canonicalize` fails)

### Requirement: Recursion Controlled by Scan Flag

`FileDiscovery::scan_directory(root, recursive)` SHALL descend into subdirectories when `recursive = true` and SHALL limit traversal to the root directory only (`max_depth(1)`) when `recursive = false`. The recursion flag originates from the CLI `--recursive` option as consumed by the match command.

#### Scenario: Non-recursive scan ignores subdirectories
- **GIVEN** a directory containing `video1.mp4`, `video2.mkv`, `subtitle1.srt`, and a subdirectory `season1/` holding `episode1.mp4` and `episode1.srt`
- **WHEN** `scan_directory(root, false)` is invoked
- **THEN** the returned list SHALL contain exactly two `Video` entries (`video1.mp4`, `video2.mkv`) and one `Subtitle` entry (`subtitle1.srt`), and SHALL NOT contain any entry whose `relative_path` references `episode1`

#### Scenario: Recursive scan walks subdirectories
- **GIVEN** the same layout as above
- **WHEN** `scan_directory(root, true)` is invoked
- **THEN** the returned list SHALL contain three `Video` entries and two `Subtitle` entries, including an entry whose `relative_path` contains `episode1`

### Requirement: MediaFile Metadata Population

For every recognized file, `FileDiscovery::classify_file` SHALL populate a `MediaFile` containing: the deterministic `id` from `generate_file_id`; the absolute on-disk `path`; the classification (`Video` or `Subtitle`); the file `size` in bytes from `std::fs::metadata`; the full filename as `name`; the lowercased `extension` (without the leading dot); and a `relative_path` computed as the file path stripped of the scan root with backslashes normalized to forward slashes for cross-platform consistency.

#### Scenario: Relative path normalization
- **GIVEN** a root directory and a video file at `<root>/season1/episode1.mkv`
- **WHEN** `scan_directory(root, true)` is invoked
- **THEN** the resulting entry SHALL have `name = "episode1.mkv"`, `extension = "mkv"`, and `relative_path = "season1/episode1.mkv"` regardless of the host operating system's native path separator

### Requirement: Explicit File List Scanning

`FileDiscovery::scan_file_list(paths)` SHALL process a caller-provided slice of absolute file paths and SHALL apply the same extension filter and deterministic id scheme as directory scanning. Non-existent paths and paths that are not regular files SHALL be skipped silently; files whose extension is not in the recognized sets SHALL also be skipped. For inputs handled through this path, `MediaFile.relative_path` SHALL equal `MediaFile.name` (the bare filename) because there is no scan root to strip.

#### Scenario: Mixed inputs filtered
- **GIVEN** a path list containing one existing `.mp4`, one existing `.srt`, one existing `.txt`, one directory, and one path that does not exist
- **WHEN** `scan_file_list` is invoked
- **THEN** the returned vector SHALL contain exactly two `MediaFile` entries — the `.mp4` classified as `Video` and the `.srt` classified as `Subtitle` — and no error SHALL be returned for the skipped entries

### Requirement: Missing Scan Root Reported as Error

`FileDiscovery::scan_directory` SHALL return an error (propagated from `walkdir`) when the supplied root path does not exist, rather than silently returning an empty result.

#### Scenario: Non-existent directory
- **GIVEN** a path that does not exist on disk
- **WHEN** `scan_directory(path, false)` is invoked
- **THEN** the call SHALL return an `Err`

#### Scenario: Existing empty directory
- **GIVEN** an existing but empty directory
- **WHEN** `scan_directory(path, false)` is invoked
- **THEN** the call SHALL return `Ok(vec![])` with no entries

### Requirement: Archive Extensions Excluded from Media Classification

`FileDiscovery` SHALL NOT include `.zip` or `.rar` in its recognized
video or subtitle extension sets. Archive files SHALL be intercepted by
`InputPathHandler` before reaching `FileDiscovery`. If an archive file
somehow reaches `scan_file_list` or `scan_directory`, it SHALL be silently
skipped like any other unrecognized extension.

#### Scenario: Zip file passed to scan_file_list is skipped
- **WHEN** `scan_file_list` receives a path list containing `subs.zip`
- **THEN** the returned `Vec<MediaFile>` SHALL NOT contain an entry for
  `subs.zip`

### Requirement: Temp Directory Paths Accepted

`FileDiscovery::scan_file_list` and `scan_directory` SHALL accept paths
rooted in the system temp directory without special handling. No path
validation SHALL reject files solely because they reside under
`std::env::temp_dir()`.

#### Scenario: Extracted files in temp dir are discovered
- **GIVEN** subtitle files extracted from an archive into `/tmp/subx-XXXX/`
- **WHEN** `scan_file_list` is called with those paths
- **THEN** each file with a recognized extension SHALL be returned as a
  `MediaFile` with correct classification and metadata
