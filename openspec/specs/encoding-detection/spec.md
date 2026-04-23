# Encoding Detection

## Purpose

Identify the character encoding of one or more subtitle files so users can diagnose display issues or plan conversions. Implemented in `src/commands/detect_encoding_command.rs`, `src/cli/detect_encoding_args.rs`, and `src/core/formats/encoding/`.

## Requirements

### Requirement: Per-File Encoding Report

The system SHALL, for each supplied file that exists, print the detected charset, confidence percentage, BOM presence indicator, and a sample of decoded text.

#### Scenario: Successful detection
- **GIVEN** a UTF-8 encoded subtitle file `movie.srt`
- **WHEN** the user runs `subx detect-encoding movie.srt`
- **THEN** the output SHALL include the file name, a line of the form `Encoding: <charset> (Confidence: <percent>%) BOM: <Yes|No>`, and a `Sample text:` line

#### Scenario: Missing file is skipped
- **GIVEN** a path that does not exist
- **WHEN** the user runs `subx detect-encoding <missing>`
- **THEN** the command SHALL emit an error via the logger (e.g. `log::error!`) indicating the path does not exist, and SHALL continue processing subsequent paths without terminating

### Requirement: Input Source Selection

The system SHALL accept target files either as positional arguments or via repeated `-i/--input` flags; the two input styles SHALL be mutually exclusive, and `-i` SHALL additionally honor `--recursive` directory traversal with a fixed subtitle-extension filter.

#### Scenario: Mutually exclusive input modes
- **GIVEN** the user passes both a positional file and `-i <dir>`
- **WHEN** the CLI parses the arguments
- **THEN** argument parsing SHALL fail with a conflict error

#### Scenario: Recursive directory scanning with `-i`
- **GIVEN** the user runs `subx detect-encoding -i <dir> --recursive`
- **WHEN** the command executes
- **THEN** the command SHALL detect encoding for every file within `<dir>` (recursively) whose extension is one of `srt`, `ass`, `vtt`, `ssa`, `sub`, `txt`

### Requirement: Verbose Sample Output

The system SHALL, when `--verbose` is passed, print the full sample text; otherwise it SHALL truncate samples longer than 50 characters with an ellipsis (`...`).

#### Scenario: Verbose mode prints full sample
- **GIVEN** a file with a sample of 300 characters and `--verbose`
- **WHEN** the command runs
- **THEN** the printed `Sample text:` line SHALL contain the full sample content without truncation
