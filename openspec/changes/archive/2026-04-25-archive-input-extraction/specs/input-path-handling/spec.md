## MODIFIED Requirements

### Requirement: Direct File Inputs Pass Through

The system SHALL accept individual file paths (not just directories) as
inputs. If a file has a recognised archive extension (`.zip`, `.rar`) and
archive extraction is enabled, the system SHALL extract the archive to a
temporary directory and include the extracted files in the result instead
of the archive path itself. For non-archive files, the system SHALL return
them unchanged when they match the configured extension filter.

#### Scenario: Single-file input
- **GIVEN** the user runs `subx convert movie.srt`
- **WHEN** `collect_files` runs
- **THEN** the returned list SHALL contain exactly `movie.srt`

#### Scenario: Archive file input is extracted
- **GIVEN** the user runs `subx convert subs.zip` and the zip contains
  `movie.srt` and `movie2.ass`
- **WHEN** `collect_files` runs
- **THEN** the returned list SHALL contain the extracted `movie.srt` and
  `movie2.ass` from the temp directory, and SHALL NOT contain `subs.zip`

#### Scenario: Archive file with --no-extract is skipped
- **GIVEN** the user runs `subx convert subs.zip --no-extract`
- **WHEN** `collect_files` runs
- **THEN** `subs.zip` SHALL be treated as a regular file, SHALL fail
  the extension filter (`.zip` is not a subtitle extension), and SHALL
  NOT appear in the result

### Requirement: Mixed File And Directory Inputs

The system SHALL accept a mixture of file, directory, and archive entries
within the same input list; on `collect_files()` it SHALL return the
matched files from every supplied directory (filtered by the configured
extensions and traversal mode), the extracted contents of every recognised
archive (when extraction is enabled), and every directly supplied file that
matches the extension filter.

#### Scenario: Files, directories, and archives mixed
- **GIVEN** inputs `[video1.mp4, dir2/, subs.zip]` where `dir2/` contains
  `video2.mkv` and `subtitle2.srt`, and `subs.zip` contains `extra.srt`
- **WHEN** `collect_files()` runs with video+subtitle extension filter
- **THEN** the returned list SHALL contain `video1.mp4`, `video2.mkv`,
  `subtitle2.srt`, and `extra.srt`

## ADDED Requirements

### Requirement: CollectedFiles Return Type

`InputPathHandler::collect_files()` SHALL return a `CollectedFiles` struct
containing the collected `Vec<PathBuf>` and any `TempDir` handles created
during archive extraction. The struct SHALL implement `Deref<Target = Vec<PathBuf>>`
so that existing call sites treating the result as a `Vec<PathBuf>` continue
to work without modification. The `TempDir` handles SHALL be dropped (and
their directories cleaned up) when the `CollectedFiles` value goes out of
scope.

#### Scenario: CollectedFiles dereferences to Vec
- **WHEN** a caller uses `collected_files.len()` or iterates with `for p in &*collected_files`
- **THEN** the code SHALL compile and behave identically to operating on a `Vec<PathBuf>`

#### Scenario: Temp dirs survive until CollectedFiles is dropped
- **WHEN** `collect_files()` returns a `CollectedFiles` with extracted archive paths
- **THEN** the temp directories SHALL exist on disk while the `CollectedFiles` value is alive
- **AND** SHALL be deleted when the `CollectedFiles` value is dropped

### Requirement: No-Extract CLI Flag

Each command that uses `InputPathHandler` (`match`, `convert`, `sync`,
`detect-encoding`) SHALL accept a `--no-extract` boolean flag (default
`false`). When `--no-extract` is `true`, `collect_files()` SHALL treat
archive files as opaque regular files (subject to the normal extension
filter).

#### Scenario: --no-extract disables archive expansion
- **GIVEN** the user runs `subx match -i subs.zip --no-extract`
- **WHEN** `collect_files()` runs
- **THEN** `subs.zip` SHALL NOT be extracted and SHALL be subject to the
  normal extension filter

### Requirement: Archive Origin Mapping

`CollectedFiles` SHALL maintain a mapping from each temp-directory root
path to the original archive file path. This mapping SHALL be queryable
via `CollectedFiles::archive_origin(temp_path) -> Option<&Path>`, enabling
commands to resolve output directories relative to the original archive
location rather than the temp directory.

#### Scenario: Temp path resolves to archive origin
- **WHEN** a file `/tmp/subx-XXXX/movie.srt` was extracted from `/data/subs.zip`
- **THEN** `collected_files.archive_origin(Path::new("/tmp/subx-XXXX/movie.srt"))`
  SHALL return `Some(Path::new("/data/subs.zip"))`

#### Scenario: Non-archive path returns None
- **WHEN** a file `/data/movie.srt` was supplied directly (not from an archive)
- **THEN** `collected_files.archive_origin(Path::new("/data/movie.srt"))`
  SHALL return `None`

### Requirement: Output Directory Resolution for Archive Files

For mutating commands (`convert`, `sync`), when a source file originates
from an archive extraction and no explicit output directory is specified,
the system SHALL resolve the output directory to the parent directory of
the original archive file. For `match`, when a subtitle file originates
from an archive extraction, the system SHALL relocate the renamed subtitle
to the matched video file's parent directory (following pairing semantics).
This prevents output from being written into the temporary extraction
directory (which is deleted on drop).

#### Scenario: Convert output goes beside archive
- **GIVEN** the user runs `subx convert subs.zip` containing `movie.srt`
- **WHEN** conversion completes with no `-o` flag
- **THEN** the converted file SHALL be written to the same directory as
  `subs.zip`, not to the temp extraction directory

#### Scenario: Explicit -o overrides archive origin
- **GIVEN** the user runs `subx convert subs.zip -o /output/`
- **WHEN** conversion completes
- **THEN** the converted file SHALL be written to `/output/`

#### Scenario: Match relocates archive subtitle beside video
- **GIVEN** the user runs `subx match -i subs.zip -i /media/` where
  `subs.zip` contains `movie.srt` and `/media/` contains `movie.mp4`
- **WHEN** matching completes
- **THEN** the renamed subtitle SHALL be copied to `/media/` (the video's
  parent directory), not the temp extraction directory or the archive's
  parent directory

### Requirement: CollectedFiles Additional APIs

`CollectedFiles` SHALL implement `into_paths() -> Vec<PathBuf>` for call
sites that consume paths by value, and `AsRef<[PathBuf]>` for slice
access. Call sites such as `DetectEncodingArgs::get_file_paths()` that
currently return `Vec<PathBuf>` SHALL be updated accordingly.

#### Scenario: into_paths consumes CollectedFiles
- **WHEN** `collected_files.into_paths()` is called
- **THEN** a `Vec<PathBuf>` SHALL be returned and the `CollectedFiles`
  SHALL be consumed (temp dirs are dropped)
