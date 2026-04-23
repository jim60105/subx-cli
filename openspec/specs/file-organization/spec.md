# File Organization

## Purpose

Rename, copy, move, and safely mutate subtitle and video files on disk during match execution, with optional backup and conflict resolution. Implemented in `src/core/matcher/engine.rs` (operation generation and execution), `src/core/file_manager.rs` (rollback-aware mutations), and `src/core/fs_util.rs`.

## Requirements

### Requirement: Relocation Modes

The system SHALL support three relocation modes for matched subtitles — `None` (rename in place), `Copy`, and `Move` — as defined by `FileRelocationMode` and selected by the match command's `--copy` and `--move` flags.

#### Scenario: Rename in place (None)
- **GIVEN** a video and subtitle in the same directory and neither `--copy` nor `--move` is passed
- **WHEN** the engine executes operations
- **THEN** the subtitle SHALL be renamed in its original directory to match the video's base name

#### Scenario: Move across directories
- **GIVEN** a video in directory `A/` and its matched subtitle in directory `B/` with `--move`
- **WHEN** the engine executes operations
- **THEN** the subtitle SHALL be moved from `B/` into `A/` and SHALL no longer exist in `B/`

### Requirement: Conflict Resolution

The system SHALL resolve name collisions using the `ConflictResolution` policy (`Skip`, `AutoRename`, or `Prompt`). The match command SHALL use `AutoRename` by default, and `Prompt` SHALL unconditionally fall back to `AutoRename` (emitting a warning that interactive prompting is not implemented).

#### Scenario: Auto-renaming on collision
- **GIVEN** the target subtitle file name already exists at the destination
- **WHEN** the engine applies the operation under `AutoRename`
- **THEN** the engine SHALL assign a non-colliding name (for example by appending a disambiguating suffix) and complete the operation without overwriting the existing file

### Requirement: Rollback-Safe File Operations

The system SHALL use `FileManager` to track file creations and removals and SHALL support rolling back tracked operations in reverse order when an error occurs mid-batch.

#### Scenario: Rollback removes created files
- **GIVEN** a `FileManager` that has recorded the creation of `new.srt` via `record_creation`
- **WHEN** `FileManager::rollback` is called
- **THEN** `new.srt` SHALL be removed from the filesystem and `operation_count()` SHALL become zero

#### Scenario: Missing source file surfaces an error
- **GIVEN** `FileManager::remove_file` is called with a path that does not exist
- **WHEN** the call executes
- **THEN** it SHALL return `SubXError::FileNotFound` with the offending path

### Requirement: Backup Before Move

The system SHALL create a backup of the source subtitle file before moving it only when the relocation mode is `Move` and backups are enabled (either via `--backup` on the match command or `general.backup_enabled` in configuration). The system SHALL NOT create backups for `None` (rename-in-place) or `Copy` modes.

#### Scenario: Backup is created before a Move
- **GIVEN** the relocation mode is `Move`, backups are enabled, and the source subtitle is `B/movie.srt`
- **WHEN** the engine executes the operation
- **THEN** a backup of `B/movie.srt` SHALL be created before the move, and the subtitle SHALL then be moved into the video's directory

#### Scenario: No backup in Copy mode
- **GIVEN** the relocation mode is `Copy` and backups are enabled
- **WHEN** the engine executes the operation
- **THEN** no backup task SHALL be scheduled and the original subtitle SHALL be left untouched at its source location

#### Scenario: No backup in rename-in-place mode
- **GIVEN** the relocation mode is `None` (rename in place) and backups are enabled
- **WHEN** the engine executes the operation
- **THEN** no backup task SHALL be scheduled

### Requirement: AutoRename Sequential Suffixes

When multiple match operations would produce the same destination filename, the `AutoRename` conflict-resolution strategy SHALL assign distinct non-colliding names to each operation by appending sequential numeric suffixes, so that no operation overwrites a file written by a prior operation in the same batch. Exercised by `tests/match_duplicate_rename_conflict_tests.rs`.

#### Scenario: Multiple subtitles matched to one video
- **GIVEN** three subtitle files are all matched to the same video such that their ideal destination names collide
- **WHEN** the engine executes the operations under `ConflictResolution::AutoRename`
- **THEN** each of the three subtitles SHALL be written to a distinct path with a numeric suffix disambiguator and no existing file SHALL be overwritten
