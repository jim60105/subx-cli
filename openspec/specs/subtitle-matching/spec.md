# Subtitle Matching

## Purpose

Provide AI-assisted pairing of video files with their correct subtitle files across one or more input paths, producing rename, copy, or move operations while enforcing a user-controlled confidence threshold. Implemented primarily in `src/core/matcher/engine.rs`, `src/commands/match_command.rs`, and `src/cli/match_args.rs`.

## Requirements

### Requirement: AI-Based File Pairing

The system SHALL use a configured AI provider to analyze video and subtitle file names (and optional subtitle content samples) and return candidate video-subtitle pairings with a per-pair confidence score.

#### Scenario: Successful match with sufficient confidence
- **GIVEN** a directory containing at least one video file and one subtitle file, and an AI provider is configured
- **WHEN** the user runs `subx match <path>`
- **THEN** the engine SHALL collect eligible files via `InputPathHandler`, send an `AnalysisRequest` to the AI provider, and generate rename operations for every pair whose confidence is greater than or equal to the configured threshold

#### Scenario: No input files available
- **GIVEN** the resolved input paths contain no video or subtitle files
- **WHEN** the match command executes
- **THEN** the command SHALL return an error `No files found to process` without calling the AI provider

### Requirement: Confidence Threshold Enforcement

The system SHALL accept a user-supplied confidence value in the range 0-100 (default 80) via `--confidence`, convert it to a 0.0-1.0 threshold, and discard any AI-proposed pair whose score falls below that threshold.

#### Scenario: Low-confidence pairs are filtered out
- **GIVEN** the user passes `--confidence 90` and the AI provider returns a candidate match with confidence 0.75
- **WHEN** the engine processes the AI response
- **THEN** the engine SHALL omit that candidate from the generated operations list

#### Scenario: Confidence outside valid range is rejected
- **GIVEN** the user passes `--confidence 150`
- **WHEN** the CLI parses the arguments
- **THEN** argument parsing SHALL fail with a validation error from `clap`

### Requirement: Dry-Run and Execution Modes

The system SHALL support a `--dry-run` mode that displays planned operations and persists them to the match cache without mutating files, and a default live mode that executes the operations.

#### Scenario: Dry-run preserves files
- **GIVEN** the user runs `subx match --dry-run <path>`
- **WHEN** the command completes
- **THEN** the planned operations SHALL be printed to the user and saved to the cache, and no file on disk SHALL be created, renamed, copied, moved, or deleted

#### Scenario: Live mode applies operations
- **GIVEN** the user runs `subx match <path>` without `--dry-run`
- **WHEN** the command completes successfully
- **THEN** the engine SHALL execute each operation through `execute_operations`, renaming subtitle files to match the paired video's base name plus the subtitle extension

### Requirement: File Relocation Modes

The system SHALL expose mutually exclusive `--copy` (`-c`) and `--move` (`-m`) flags that relocate the matched subtitle alongside its paired video; when neither is provided the subtitle SHALL be renamed in place.

#### Scenario: Copy and move are mutually exclusive
- **GIVEN** the user passes both `--copy` and `--move`
- **WHEN** the CLI runs `MatchArgs::validate`
- **THEN** validation SHALL fail with the message `Cannot use --copy and --move together. Please choose one operation mode.`

#### Scenario: Copy relocates matched subtitle
- **GIVEN** a video in directory `A/` and its matched subtitle in directory `B/`, and the user passes `--copy`
- **WHEN** the engine executes operations
- **THEN** the subtitle SHALL be copied into directory `A/` with a name derived from the video's base name, and the original subtitle in `B/` SHALL remain untouched

### Requirement: Optional Backup Before Move

The system SHALL create a backup of the source subtitle file before moving it only when the relocation mode is `Move` and backups are enabled (via `--backup` on the command or `general.backup_enabled` in configuration). The system SHALL NOT create backups in rename-in-place or `Copy` modes.

#### Scenario: Backup before Move
- **GIVEN** the user runs `subx match --move --backup <path>` and a matching subtitle is identified
- **WHEN** the engine executes the operation
- **THEN** a backup of the source subtitle SHALL be created before the file is moved to the video's directory

#### Scenario: No backup when only renaming in place
- **GIVEN** the user runs `subx match --backup <path>` without `--copy` or `--move`
- **WHEN** the engine executes the rename-in-place operation
- **THEN** no backup task SHALL be scheduled
