# Timeline Sync

## Purpose

Hold the CLI half of the timeline-sync agreement: single-file and batch modes, dry-run mode, manual-offset-without-video framing, force-overwrite behavior, batch pairing heuristics (prefix matching, skip-without-videos, single-pair override), CLI VAD parameter overrides, the structured JSON payload of `sync`, and the rule that the sync argument struct is a thin adapter over core pairing. Implemented in `src/commands/sync_command.rs` and `src/cli/sync_args.rs`. The sync engine, pairing resolution, default output paths, offset clamping, timing application, and VAD processing rules are specified by the same-named `timeline-sync` capability in `subx-core`.

## Requirements

### Requirement: Single-File and Batch Modes

The `sync` command SHALL support a single-pair mode (via `--video` + `--subtitle`, positional paths, or manual mode with only a subtitle) and a batch mode (via `--batch [DIR]` combined with `-i`, positional paths, or an explicit directory) that pairs videos with subtitles inside the same directory.

The decision between the two modes, and the auto-pairing that backs single-pair mode, SHALL be performed by the `timeline-sync` capability's *Core-Owned Sync Pairing Resolution* requirement in `subx-core`, whose `resolve_sync_pairing` is reachable from this crate as `subx_cli::core::sync::resolve_sync_pairing` through the re-export surface the `crate-topology` capability specifies. The `sync` command's clap struct SHALL contribute only the flag definitions and the field-to-request adaptation; it SHALL NOT read the filesystem.

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

### Requirement: Sync Argument Struct Is a Thin Adapter Over Core Pairing

`SyncArgs` and the `sync` command SHALL own the flag surface and the legacy aliases, and nothing else. Every pairing and path-derivation behaviour is specified by the `timeline-sync` capability in `subx-core`; this requirement states what remains in `src/cli/sync_args.rs`.

1. **Method flag and its validation.** The command SHALL accept `--method` with the values `vad` and `manual`, and SHALL omit the option to select the configured `sync.default_method`. When `--method manual` is supplied without `--offset`, `SyncArgs::validate` SHALL fail with the message `Manual method requires --offset parameter.` This is argument validation, not engine behaviour: no core file is consulted to produce it.
2. **Pairing adapter.** `SyncArgs::get_sync_mode` SHALL populate a `SyncPairingRequest` from its own fields — translating clap's `Option<Option<PathBuf>>` for `--batch` into `BatchRequest`, and `is_manual_mode()` into `manual` — and SHALL return `resolve_sync_pairing`'s result unchanged. It SHALL contain no filesystem access and no pairing logic of its own.
3. **Output-path adapter.** `SyncArgs::get_output_path` SHALL derive its default through the core `create_default_output_path`, and SHALL NOT reimplement the `<file_stem>_synced.<extension>` derivation. In-crate callers, including `src/commands/sync_command.rs`, SHALL reference the core path rather than the legacy alias.
4. **Legacy aliases.** `crate::cli::SyncMode` and `crate::cli::sync_args::create_default_output_path` SHALL remain available as legacy re-exports of their `subx-core` originals, documented as such in rustdoc, without a `#[deprecated]` attribute, so that existing consumers keep compiling.

#### Scenario: Manual mode requires an explicit offset
- **GIVEN** the user passes `--method manual` without `--offset`
- **WHEN** argument validation runs
- **THEN** validation SHALL fail with the message `Manual method requires --offset parameter.`

#### Scenario: The CLI adapter adds no behaviour
- **GIVEN** any `SyncArgs` value
- **WHEN** `SyncArgs::get_sync_mode` is called
- **THEN** the result SHALL equal `resolve_sync_pairing` applied to the `SyncPairingRequest` built from that value's fields

#### Scenario: Legacy sync aliases still resolve
- **GIVEN** a consumer that writes `use subx_cli::cli::SyncMode;` or `use subx_cli::cli::sync_args::create_default_output_path;`
- **WHEN** the crate is compiled
- **THEN** both imports SHALL resolve to their `subx-core` originals and SHALL produce no deprecation warning