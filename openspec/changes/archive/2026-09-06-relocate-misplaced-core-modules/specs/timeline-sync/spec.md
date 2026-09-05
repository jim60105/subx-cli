## ADDED Requirements

### Requirement: Core-Owned Sync Pairing Resolution

Deciding whether a `sync` invocation is a single pair or a batch, and auto-pairing a lone video or subtitle with its sibling on disk, SHALL be a documented core API rather than implicit behaviour of an argument-parser struct.

The core module `crate::core::sync` (`src/core/sync/mod.rs`) SHALL expose:

- `pub enum SyncMode { Single { video: PathBuf, subtitle: PathBuf }, Batch(InputPathHandler) }` — the resolved outcome.
- `pub enum BatchRequest { Off, Auto, Directory(PathBuf) }` — a parser-agnostic encoding of the `--batch [DIR]` tri-state, replacing clap's `Option<Option<PathBuf>>`.
- `pub struct SyncPairingRequest` with the fields `positional_paths: Vec<PathBuf>`, `input_paths: Vec<PathBuf>`, `video: Option<PathBuf>`, `subtitle: Option<PathBuf>`, `batch: BatchRequest`, `recursive: bool`, `no_extract: bool`, and `manual: bool`.
- `pub fn resolve_sync_pairing(request: &SyncPairingRequest) -> Result<SyncMode, SubXError>`.
- `pub const SYNC_VIDEO_EXTENSIONS: &[&str]` = `["mp4", "mkv", "avi", "mov"]` and `pub const SYNC_SUBTITLE_EXTENSIONS: &[&str]` = `["srt", "ass", "vtt", "sub"]`, which SHALL be the single definition of those lists for both pairing and handler construction.

`resolve_sync_pairing` SHALL apply the following algorithm, in order:

1. **Batch selection.** If `batch != BatchRequest::Off`, or `input_paths` is non-empty, or any entry of `positional_paths` has no file extension, the result SHALL be `SyncMode::Batch`. The handler's path list SHALL be the batch directory (when `batch == Directory(d)`) followed by `input_paths` followed by `positional_paths`; when that list is empty it SHALL default to `["."]`. The handler SHALL be built with the caller's `recursive` and `no_extract` values and with the union of `SYNC_VIDEO_EXTENSIONS` and `SYNC_SUBTITLE_EXTENSIONS` as its extension filter.
2. **Single positional path.** With exactly one positional path and no batch trigger, the extension SHALL be lower-cased and classified. If it is in `SYNC_VIDEO_EXTENSIONS`, the path becomes the video and the resolver SHALL probe the path's parent directory (or `.` when it has none) for `<stem>.<ext>` over `SYNC_SUBTITLE_EXTENSIONS` **in declaration order**, taking the first entry for which `Path::exists()` is true. If it is in `SYNC_SUBTITLE_EXTENSIONS`, the path becomes the subtitle and the same probe runs over `SYNC_VIDEO_EXTENSIONS`. Any other extension SHALL classify as neither.
3. **Two positional paths.** With exactly two positional paths, each SHALL be classified by its lower-cased extension against the two lists; no filesystem probing occurs.
4. **Explicit options.** Otherwise, `video` and `subtitle` SHALL be used as supplied.
5. **Manual-mode relaxation.** When `manual` is true and a subtitle has been resolved but no video has, the result SHALL be `SyncMode::Single` with an **empty** `PathBuf` as the video, signalling "no video required".
6. **Failure.** When no `SyncMode` can be produced, the call SHALL return `Err(SubXError::InvalidSyncConfiguration)`.

`SyncArgs::get_sync_mode` (`src/cli/sync_args.rs`) SHALL become a thin adapter that populates a `SyncPairingRequest` from its own fields — translating `Option<Option<PathBuf>>` into `BatchRequest` and `is_manual_mode()` into `manual` — and returns `resolve_sync_pairing`'s result unchanged. It SHALL contain no filesystem access and no pairing logic of its own. `crate::cli::SyncMode` SHALL remain available as a legacy re-export of `crate::core::sync::SyncMode`, documented as such in rustdoc and without a `#[deprecated]` attribute.

The batch *pairing* performed afterwards inside `src/commands/sync_command.rs` (the filename-stem prefix heuristic and the single-video/single-subtitle override) is a separate, command-level concern and is unaffected by this requirement.

#### Scenario: Lone video positional finds its subtitle on disk
- **GIVEN** a directory containing `movie.mp4` and `movie.srt`, and a `SyncPairingRequest` whose only positional path is `movie.mp4`
- **WHEN** `resolve_sync_pairing` runs
- **THEN** it SHALL return `SyncMode::Single { video: movie.mp4, subtitle: movie.srt }`

#### Scenario: Lone subtitle positional finds its video on disk
- **GIVEN** a directory containing `movie.mkv` and `movie.ass`, and a `SyncPairingRequest` whose only positional path is `movie.ass`
- **WHEN** `resolve_sync_pairing` runs
- **THEN** it SHALL return `SyncMode::Single { video: movie.mkv, subtitle: movie.ass }`

#### Scenario: Probe order follows the declared extension lists
- **GIVEN** a directory containing `movie.mp4`, `movie.srt`, and `movie.ass`, and a `SyncPairingRequest` whose only positional path is `movie.mp4`
- **WHEN** `resolve_sync_pairing` runs
- **THEN** the chosen subtitle SHALL be `movie.srt`, because `srt` precedes `ass` in `SYNC_SUBTITLE_EXTENSIONS`

#### Scenario: Manual mode accepts a subtitle with no video
- **GIVEN** a `SyncPairingRequest` with `manual == true` whose only positional path is `movie.srt`, and no `movie.<video-ext>` exists beside it
- **WHEN** `resolve_sync_pairing` runs
- **THEN** it SHALL return `SyncMode::Single` whose `subtitle` is `movie.srt` and whose `video` is an empty `PathBuf`

#### Scenario: Unpairable single positional is rejected
- **GIVEN** a `SyncPairingRequest` with `manual == false` whose only positional path is `movie.mp4`, with no subtitle file beside it
- **WHEN** `resolve_sync_pairing` runs
- **THEN** it SHALL return `Err(SubXError::InvalidSyncConfiguration)`

#### Scenario: Two positional paths are classified without probing
- **GIVEN** a `SyncPairingRequest` whose positional paths are `movie.srt` and `movie.mp4`, in that order
- **WHEN** `resolve_sync_pairing` runs
- **THEN** it SHALL return `SyncMode::Single { video: movie.mp4, subtitle: movie.srt }` without testing any other path for existence

#### Scenario: Each batch trigger selects batch mode
- **GIVEN** three `SyncPairingRequest` values that differ only in their batch trigger — one with `batch == Directory(dir)`, one with a non-empty `input_paths`, and one whose sole positional path has no extension
- **WHEN** `resolve_sync_pairing` runs for each
- **THEN** every call SHALL return `SyncMode::Batch`

#### Scenario: Batch with no usable paths defaults to the current directory
- **GIVEN** a `SyncPairingRequest` with `batch == BatchRequest::Auto`, empty `input_paths`, and empty `positional_paths`
- **WHEN** `resolve_sync_pairing` runs
- **THEN** the returned `SyncMode::Batch` handler's path list SHALL be exactly `["."]`

#### Scenario: The CLI adapter adds no behaviour
- **GIVEN** any `SyncArgs` value
- **WHEN** `SyncArgs::get_sync_mode` is called
- **THEN** the result SHALL equal `resolve_sync_pairing` applied to the `SyncPairingRequest` built from that value's fields

### Requirement: Core-Owned Default Output Path Derivation

Deriving the default synchronized-output filename SHALL be a core API. `crate::core::sync::create_default_output_path(input: &Path) -> PathBuf` (`src/core/sync/mod.rs`) SHALL return the input path with its file name replaced by `<file_stem>_synced.<extension>`, and SHALL return the input path unchanged when it has no file stem or no extension.

`crate::cli::sync_args::create_default_output_path` SHALL remain available as a legacy re-export documented in rustdoc, without a `#[deprecated]` attribute, so that existing consumers keep compiling. In-crate callers — `SyncArgs::get_output_path` and `src/commands/sync_command.rs` — SHALL reference the core path.

#### Scenario: Stem gains the `_synced` suffix
- **GIVEN** the input path `subs/movie.srt`
- **WHEN** `create_default_output_path` is called
- **THEN** it SHALL return `subs/movie_synced.srt`

#### Scenario: Extension is preserved verbatim
- **GIVEN** the input path `subs/movie.vtt`
- **WHEN** `create_default_output_path` is called
- **THEN** it SHALL return `subs/movie_synced.vtt`

#### Scenario: Extensionless input is returned unchanged
- **GIVEN** an input path with a file stem but no extension
- **WHEN** `create_default_output_path` is called
- **THEN** it SHALL return the input path unchanged

## MODIFIED Requirements

### Requirement: Single-File and Batch Modes

The system SHALL support a single-pair mode (via `--video` + `--subtitle`, positional paths, or manual mode with only a subtitle) and a batch mode (via `--batch [DIR]` combined with `-i`, positional paths, or an explicit directory) that pairs videos with subtitles inside the same directory.

The decision between the two modes, and the auto-pairing that backs single-pair mode, SHALL be performed by `crate::core::sync::resolve_sync_pairing` per the "Core-Owned Sync Pairing Resolution" requirement. The `sync` command's clap struct SHALL contribute only the flag definitions and the field-to-request adaptation; it SHALL NOT read the filesystem.

#### Scenario: Batch mode without any input
- **GIVEN** the user passes `--batch` with no directory, no `-i`, no positional path, and no `--video` or `--subtitle`
- **WHEN** argument validation runs
- **THEN** validation SHALL fail with a message explaining that batch mode requires at least one input source

#### Scenario: Mode selection is reproducible outside the CLI
- **GIVEN** a caller that constructs a `SyncPairingRequest` directly, without parsing a command line
- **WHEN** it calls `resolve_sync_pairing`
- **THEN** it SHALL receive the same `SyncMode` the `sync` command would have resolved for the equivalent arguments
