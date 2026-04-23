# Input Path Handling

## Purpose

Resolve the heterogeneous input-path arguments used across `match`, `convert`, `sync`, and `detect-encoding` into a single filtered list of files, supporting positional paths, repeated `-i/--input` flags, string paths, and optional recursion. Implemented in `src/cli/input_handler.rs` (`InputPathHandler`).

## Requirements

### Requirement: Unified Path Merging

The system SHALL provide `InputPathHandler::merge_paths_from_multiple_sources(optional_paths, input_paths, string_paths)` so that each command can combine its positional `Option<PathBuf>`, its repeated `-i` arguments, and any additional string-path arguments into one deduplicated `Vec<PathBuf>`.

#### Scenario: Positional and `-i` paths merged
- **GIVEN** the user runs `subx match ./dirA -i ./dirB -i ./dirC`
- **WHEN** `MatchArgs::get_input_handler` resolves paths
- **THEN** the resulting handler SHALL contain `./dirA`, `./dirB`, and `./dirC`

#### Scenario: No input at all is rejected
- **GIVEN** a command that requires at least one input source and the user supplies none
- **WHEN** `merge_paths_from_multiple_sources` is called with empty inputs
- **THEN** the call SHALL return an error (for example `SubXError::NoInputSpecified`) rather than returning an empty list silently

### Requirement: Extension Filtering

The system SHALL provide `with_extensions(&[&str])` to restrict collected files to a whitelist of extensions, and each command SHALL apply the whitelist appropriate to its domain (for example `match` uses video + subtitle extensions, `convert` uses subtitle extensions, `detect-encoding` uses subtitle + `txt`).

#### Scenario: Non-subtitle files ignored by convert
- **GIVEN** a directory containing `movie.srt`, `movie.mp4`, and `notes.txt`, and the convert command
- **WHEN** `ConvertArgs::get_input_handler().collect_files()` runs
- **THEN** the returned list SHALL include `movie.srt` and SHALL NOT include `movie.mp4` or `notes.txt`

### Requirement: Recursive vs Flat Traversal

The system SHALL collect files from directory inputs recursively when the `--recursive` flag is passed, and non-recursively (single directory level) otherwise.

#### Scenario: Recursive traversal
- **GIVEN** a directory tree with subtitle files at multiple nesting depths and `--recursive` passed
- **WHEN** `collect_files` runs
- **THEN** subtitle files from every depth SHALL be returned

#### Scenario: Flat traversal
- **GIVEN** the same tree without `--recursive`
- **WHEN** `collect_files` runs
- **THEN** only subtitle files directly inside the specified directories SHALL be returned

### Requirement: Direct File Inputs Pass Through

The system SHALL accept individual file paths (not just directories) as inputs and SHALL return them unchanged when they match the configured extension filter.

#### Scenario: Single-file input
- **GIVEN** the user runs `subx convert movie.srt`
- **WHEN** `collect_files` runs
- **THEN** the returned list SHALL contain exactly `movie.srt`
