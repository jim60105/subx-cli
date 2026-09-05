# Timeline Sync

## Purpose

Align subtitle timing to a video's audio track by applying either an automatic offset derived from local Voice Activity Detection (VAD) or a user-supplied manual offset, for single pairs or batch directories. Implemented in `src/commands/sync_command.rs`, `src/cli/sync_args.rs` (thin CLI adapter), `src/core/sync/` (pairing resolution, default output paths, engine), and `src/services/vad/`.

## Requirements

### Requirement: Sync Method Selection

The system SHALL support two sync methods selected via `--method`: `vad` (local Voice Activity Detection) and `manual` (user-supplied offset). When `--method` is omitted the engine SHALL fall back to the method declared by `sync.default_method` in configuration.

#### Scenario: Manual mode requires an explicit offset
- **GIVEN** the user passes `--method manual` without `--offset`
- **WHEN** argument validation runs
- **THEN** validation SHALL fail with the message `Manual method requires --offset parameter.`

#### Scenario: VAD detector is required unconditionally
- **GIVEN** VAD is disabled in configuration or the VAD detector fails to initialize
- **WHEN** `SyncEngine::new` is called
- **THEN** engine construction SHALL unconditionally return a configuration error stating that the VAD detector is required but unavailable, regardless of which sync method the user ultimately selects

### Requirement: Offset Clamping Against Maximum

The system SHALL enforce `sync.max_offset_seconds`: manual offsets exceeding this absolute value SHALL be rejected with an error, and VAD-detected offsets exceeding it SHALL be clamped (preserving sign) and accompanied by a warning in the sync result.

#### Scenario: Manual offset exceeds maximum
- **GIVEN** `sync.max_offset_seconds = 60` and the user supplies `--offset 120`
- **WHEN** `apply_manual_offset` runs
- **THEN** the call SHALL return a configuration error referencing `sync.max_offset_seconds` and the subtitle entries SHALL remain unchanged

#### Scenario: VAD offset clamping
- **GIVEN** `sync.max_offset_seconds = 30` and VAD detects an offset of 45s
- **WHEN** `vad_detect_sync_offset` returns
- **THEN** the resulting `SyncResult.offset_seconds` SHALL equal 30 (sign preserved), `SyncResult.warnings` SHALL contain a message explaining the clamping, and `additional_info` SHALL record the original and clamped values

### Requirement: Subtitle Timing Application

The system SHALL shift every subtitle entry's start and end time by the applied offset, clamping negative results to zero rather than producing negative timestamps.

#### Scenario: Positive offset delays subtitles
- **GIVEN** a subtitle entry with `start_time = 10s` and the engine applies a +2.5s offset
- **WHEN** `apply_manual_offset` runs
- **THEN** the entry's new `start_time` SHALL be 12.5s

#### Scenario: Negative offset clamps to zero
- **GIVEN** a subtitle entry with `start_time = 1s` and the engine applies a -5s offset within the maximum
- **WHEN** `apply_manual_offset` runs
- **THEN** the entry's new `start_time` SHALL be `Duration::ZERO` rather than a negative value

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

### Requirement: Dry-Run Mode

The system SHALL support `--dry-run` to analyze and display proposed synchronization results without writing an output file.

#### Scenario: Dry-run produces no output file
- **GIVEN** the user runs `subx sync --dry-run ...`
- **WHEN** the command completes
- **THEN** the sync result SHALL be printed but no output subtitle file SHALL be written to disk

### Requirement: Manual Offset Without Video

The system SHALL permit manual-offset synchronization with only a subtitle input (no video file required) when `--method manual` and `--offset <value>` are provided, applying the offset to every subtitle entry and writing the adjusted output file.

#### Scenario: Manual sync of a standalone subtitle
- **GIVEN** the user runs `subx sync --method manual --offset 2.5 <subtitle.srt>` with no `--video` argument
- **WHEN** the command executes
- **THEN** argument validation SHALL pass and the command SHALL produce an adjusted output subtitle without requiring a video file

### Requirement: Force Overwrite of Existing Output

The system SHALL refuse to overwrite an existing output file by default and SHALL require `--force` to permit overwriting; when `--force` is passed, the existing file SHALL be replaced. Implemented in `src/commands/sync_command.rs`.

#### Scenario: Existing output without --force is rejected
- **GIVEN** the intended output path already exists and `--force` is not provided
- **WHEN** `subx sync` attempts to write the output
- **THEN** the command SHALL fail with an error message containing `Output file already exists` and advising the user to pass `--force`

#### Scenario: Existing output with --force is overwritten
- **GIVEN** the intended output path already exists and the user passes `--force`
- **WHEN** `subx sync` writes the output
- **THEN** the existing file SHALL be replaced with the newly synchronized subtitle and the command SHALL succeed

### Requirement: Batch Prefix-Match Pairing

In batch mode, the system SHALL pair each video file with a subtitle in the same directory whose filename stem starts with the video's filename stem (prefix-match heuristic); subtitles that cannot be paired with any video under this heuristic SHALL be skipped with a per-file message containing `no matching video`.

#### Scenario: Orphan subtitle in a mixed directory
- **GIVEN** a directory containing a matched video/subtitle pair plus an additional subtitle with no corresponding video, under batch mode
- **WHEN** the command completes
- **THEN** the matched pair SHALL produce a synchronized output file and the orphan subtitle SHALL be skipped with a message containing `no matching video`

### Requirement: Batch Skip Directories Without Videos

In batch mode, when a directory contains subtitle files but no video files, every subtitle in that directory SHALL be skipped with a per-file message of the form `✗ Skip sync for <file>: no video files found in directory`, and no `*_synced.*` output files SHALL be created for that directory. An empty directory SHALL NOT cause a panic and SHALL produce no output files.

#### Scenario: Directory with no video files
- **GIVEN** a directory containing only subtitle files and no video files, and the user runs `subx sync --batch <dir>`
- **WHEN** the command completes
- **THEN** the command SHALL exit successfully, SHALL emit a skip message for each subtitle containing `no video files found in directory`, and SHALL NOT create any `*_synced.*` output files

#### Scenario: Empty directory
- **GIVEN** an empty directory and `subx sync --batch <empty-dir>`
- **WHEN** the command runs
- **THEN** the command SHALL not panic, SHALL exit successfully, and SHALL produce no output files

### Requirement: Batch Single-Pair Override

In batch mode, when a directory contains exactly one video file and exactly one subtitle file, the system SHALL pair them regardless of whether their filename stems share a prefix.

#### Scenario: Single video and single mismatched subtitle are still paired
- **GIVEN** a directory containing exactly one video file (`video.mp4`) and exactly one subtitle file (`subtitle.srt`) whose names do not share a prefix, under batch mode
- **WHEN** the command runs (e.g. in dry-run)
- **THEN** the two files SHALL be paired and processed without producing a skip message

### Requirement: CLI VAD Parameter Overrides

The system SHALL accept per-invocation overrides `--vad-sensitivity <0.0-1.0>` and `--window <seconds>` on the `sync` command; when provided, these values SHALL take precedence over the corresponding settings under `sync.vad` / `sync` configuration for that invocation only.

#### Scenario: VAD sensitivity override
- **GIVEN** `sync.vad.sensitivity = 0.25` in configuration and the user passes `--vad-sensitivity 0.8`
- **WHEN** the VAD detector runs for this invocation
- **THEN** it SHALL use sensitivity 0.8 and the on-disk configuration SHALL remain unchanged

### Requirement: VAD Audio Processing

The system SHALL provide `VadAudioProcessor` that loads an audio or video file, downmixes multi-channel audio to mono, preserves the file's original sample rate in the returned `AudioInfo`, and converts f32 PCM samples to i16. An invalid or non-existent path SHALL return an error rather than panic. Implemented in `src/services/vad/` and exercised by `tests/vad_audio_processor_tests.rs` and `tests/vad_integration_tests.rs`.

#### Scenario: Multi-channel input downmixed and sample rate preserved
- **GIVEN** a WAV input with sample rate 44100 Hz and two channels
- **WHEN** `VadAudioProcessor::load_and_prepare_audio_direct` processes it
- **THEN** the returned `AudioData.info.sample_rate` SHALL equal 44100 and `AudioData.info.channels` SHALL equal 1

#### Scenario: Float32 PCM converted to i16
- **GIVEN** a WAV file with one f32 sample equal to `0.5`
- **WHEN** the processor loads and prepares the file
- **THEN** the returned `samples[0]` SHALL be an `i16` value within 10 of `i16::MAX / 2`

#### Scenario: Non-existent audio path errors
- **GIVEN** a path that does not exist
- **WHEN** `load_and_prepare_audio_direct` is called
- **THEN** the call SHALL return an error rather than panic

### Requirement: VAD Detector Behavior

`LocalVadDetector` SHALL detect speech segments whose `start_time < end_time` and whose durations are bounded below by the configured `min_speech_duration_ms`; higher `sensitivity` SHALL produce at least as many segments as lower sensitivity on the same input. `VadSyncDetector::detect_sync_offset` SHALL return an error when the provided subtitle has no entries, and SHALL return `SyncMethod::LocalVad` as the method used.

#### Scenario: Sensitivity monotonicity
- **GIVEN** the same audio input processed by two detectors configured with `sensitivity = 0.1` and `sensitivity = 0.9`
- **WHEN** both detectors run
- **THEN** the segment count at low sensitivity SHALL be less than or equal to the segment count at high sensitivity (tolerating a difference of at most 1)

#### Scenario: Empty subtitle rejected
- **GIVEN** a subtitle with zero entries
- **WHEN** `VadSyncDetector::detect_sync_offset` is awaited
- **THEN** it SHALL return an error whose message contains `No subtitle entries found`

### Requirement: First-Sentence Offset Annotation

When VAD-assisted sync computes an offset by aligning the first detected speech segment with the first subtitle entry, the resulting `SyncResult` SHALL populate `additional_info` with both `first_speech_start` and `expected_subtitle_start`, such that `offset_seconds == first_speech_start - expected_subtitle_start` (within a small tolerance). Exercised by `tests/sync_first_sentence_offset_integration_tests.rs`.

#### Scenario: Offset equals first-speech minus expected-start
- **GIVEN** a real audio+subtitle asset pair and a VAD-enabled `SyncEngine`
- **WHEN** `detect_sync_offset(..., Some(SyncMethod::Auto))` returns a result
- **THEN** `additional_info.first_speech_start - additional_info.expected_subtitle_start` SHALL equal `offset_seconds` within ±0.01

### Requirement: VAD Padding Chunks Configuration

The system SHALL apply `sync.vad.padding_chunks` (default `3`) as the number of non-speech chunks included before and after each detected speech segment when `LocalVadDetector` labels audio via the VAD backend. A change in `padding_chunks` SHALL be passed through to the VAD labeling step without requiring any other configuration change. Implemented in `src/services/vad/detector.rs` (the `vad.label(..., padding_chunks, ...)` call) and defined in `src/config/mod.rs::VadConfig`.

#### Scenario: Configured padding is applied to the VAD backend
- **GIVEN** `sync.vad.padding_chunks = 5` in configuration
- **WHEN** `LocalVadDetector` runs VAD labeling over an audio buffer
- **THEN** it SHALL invoke the underlying VAD label function with the padding-chunks argument equal to `5`

### Requirement: Sync Command Emits Structured JSON Payload

When the `sync` command runs with the global output mode set to `json`, it SHALL emit a single JSON envelope on stdout (per the `machine-readable-output` capability) and SHALL NOT print free-form progress chatter or status symbols on stdout. The envelope's `data` object SHALL contain:

- `method` (string identifying the active sync method, e.g., `"vad"`, `"manual"`).
- `inputs` (array of objects with `subtitle` (string path), optional `video` (string path), `detected_offset_ms` (integer or `null` when no detection occurred), `applied_offset_ms` (integer), `status` (`"ok"` or `"error"`), and an optional `error` object with `code`, `category`, `message` when `status == "error"`).
- `operations` (array of objects with `subtitle` (string path), `before_ms` (integer), `after_ms` (integer), `applied` (bool), `status` (`"ok"` or `"error"`), and an optional `error` object when `status == "error"`).

When the sync command processes multiple subtitle files in batch mode, individual per-file failures SHALL be represented as entries with `status == "error"` while the top-level envelope SHALL remain `status == "ok"` and the process exit code SHALL be `0`. A top-level error envelope SHALL be emitted only for whole-command failures such as `InvalidSyncConfiguration`, missing required inputs, or fatal I/O before any file is processed.

In `text` mode (the default) the sync command's existing UX is unchanged.

#### Scenario: VAD-based sync reports detected and applied offsets
- **GIVEN** a subtitle/video pair processed via the VAD method
- **WHEN** the user runs `subx-cli --output json sync <args>`
- **THEN** `data.method == "vad"`, `data.inputs[0].detected_offset_ms` SHALL be an integer, and `data.operations[0].applied` SHALL reflect whether the subtitle file was modified

#### Scenario: Manual sync reports applied offset
- **GIVEN** the user passes a manual offset via the CLI
- **WHEN** the command runs with `--output json`
- **THEN** `data.method == "manual"` and `data.inputs[0].applied_offset_ms` SHALL equal the user-provided offset converted to milliseconds

#### Scenario: Invalid sync configuration produces error envelope
- **GIVEN** the user supplies a sync configuration that fails validation
- **WHEN** the command runs with `--output json`
- **THEN** the envelope SHALL satisfy `status == "error"` and `error.category == "invalid_sync_configuration"`, and the process exit code SHALL match `SubXError::exit_code` for that variant

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
