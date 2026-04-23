# Timeline Sync

## Purpose

Align subtitle timing to a video's audio track by applying either an automatic offset derived from local Voice Activity Detection (VAD) or a user-supplied manual offset, for single pairs or batch directories. Implemented in `src/commands/sync_command.rs`, `src/cli/sync_args.rs`, `src/core/sync/engine.rs`, and `src/services/vad/`.

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

#### Scenario: Batch mode without any input
- **GIVEN** the user passes `--batch` with no directory, no `-i`, no positional path, and no `--video` or `--subtitle`
- **WHEN** argument validation runs
- **THEN** validation SHALL fail with a message explaining that batch mode requires at least one input source

### Requirement: Dry-Run Mode

The system SHALL support `--dry-run` to analyze and display proposed synchronization results without writing an output file.

#### Scenario: Dry-run produces no output file
- **GIVEN** the user runs `subx sync --dry-run ...`
- **WHEN** the command completes
- **THEN** the sync result SHALL be printed but no output subtitle file SHALL be written to disk
