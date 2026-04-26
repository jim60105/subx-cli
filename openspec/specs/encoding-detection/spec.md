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

### Requirement: Robust Handling of Empty and Binary Files

The system SHALL complete encoding detection for each supplied file without terminating the whole batch when the file is empty or contains binary (non-text) bytes; it SHALL either emit a normal detection report for the file or surface a per-file error while still processing subsequent inputs.

#### Scenario: Empty file
- **GIVEN** a zero-byte subtitle file supplied to `subx detect-encoding`
- **WHEN** the command runs
- **THEN** the command SHALL not panic and SHALL exit successfully after recording a per-file outcome

#### Scenario: Binary file
- **GIVEN** a file containing binary (non-text) bytes supplied to `subx detect-encoding`
- **WHEN** the command runs
- **THEN** the command SHALL not panic and SHALL exit successfully, emitting either a best-effort detection result or a per-file error message without aborting subsequent inputs

### Requirement: Low-Confidence Fallback To Default Encoding

When no encoding candidate scores above `formats.encoding_detection_confidence`, the detector SHALL fall back to the configured default encoding (e.g. UTF-8), SHALL report a fixed fallback confidence of `0.5`, and SHALL prefix the sample text with a `Low confidence detection, using default:` marker. When there are no candidates at all, the fallback SHALL instead use confidence `0.1` and prefix the sample with `Unable to detect encoding, using default:`. Implemented in `src/core/formats/encoding/detector.rs::select_best_encoding`.

#### Scenario: Best candidate below threshold
- **GIVEN** a byte sequence whose highest-scoring encoding candidate has a confidence strictly less than `formats.encoding_detection_confidence`
- **WHEN** the encoding detector selects a result
- **THEN** the returned `EncodingInfo.charset` SHALL be the configured default encoding, `EncodingInfo.confidence` SHALL equal `0.5`, and `EncodingInfo.sample_text` SHALL start with `Low confidence detection, using default:`

#### Scenario: No viable candidates at all
- **GIVEN** a byte sequence for which no charset yields a confidence above the internal lower bound
- **WHEN** the encoding detector selects a result
- **THEN** the returned `EncodingInfo.confidence` SHALL equal `0.1` and `EncodingInfo.sample_text` SHALL start with `Unable to detect encoding, using default:`

### Requirement: Legacy Positional File Paths Accepted

The system SHALL accept file paths passed through the legacy `file_paths: Vec<String>` argument on `DetectEncodingArgs` in addition to the newer `input_paths: Vec<PathBuf>` field, processing both sources equivalently. Exercised by `tests/detect_encoding_command_comprehensive_tests.rs::test_detect_encoding_command_with_legacy_file_paths`.

#### Scenario: Legacy string file path is detected
- **GIVEN** `DetectEncodingArgs { input_paths: vec![], file_paths: vec!["legacy.srt".into()], .. }`
- **WHEN** `detect_encoding_command` runs
- **THEN** the command SHALL succeed and SHALL emit an encoding report for `legacy.srt` just as if it had been passed via `input_paths`

### Requirement: Detect-Encoding Command Emits Structured JSON Payload

When the `detect-encoding` command runs with the global output mode set to `json`, it SHALL emit a single JSON envelope on stdout (per the `machine-readable-output` capability) and SHALL NOT render the human-friendly result table on stdout. The envelope's `data` object SHALL contain:

- `files` (array of objects with `path` (string path), `encoding` (string, e.g., `"UTF-8"`, `"GBK"`, `"Big5"`), `confidence` (number in `[0.0, 1.0]`), `has_bom` (bool), `status` (`"ok"` or `"error"`), and an optional `error` object with `code`, `category`, `message` when `status == "error"`).

When the command processes multiple paths and an individual file cannot be read or decoded, the affected entry SHALL carry `status == "error"` while the top-level envelope SHALL remain `status == "ok"` and the process exit code SHALL be `0`. A top-level error envelope SHALL be emitted only when the command receives a fatal error before any path is processed (e.g., a single missing path passed as the sole argument, or a fatal configuration error).

In `text` mode (the default) the existing per-file table output is unchanged.

#### Scenario: Single file UTF-8 with BOM
- **GIVEN** a subtitle file encoded in UTF-8 with BOM
- **WHEN** the user runs `subx-cli --output json detect-encoding <path>`
- **THEN** `data.files` SHALL contain exactly one element whose `encoding` matches `UTF-8` (case-insensitive permitted), `has_bom == true`, and `status == "ok"`

#### Scenario: Multiple files reported in array
- **GIVEN** three subtitle files passed via globs or `-i`
- **WHEN** the user runs `subx-cli --output json detect-encoding <paths>`
- **THEN** `data.files` SHALL contain exactly three entries, each populated with `path`, `encoding`, `confidence`, `has_bom`, and `status`

#### Scenario: Unreadable file in batch yields per-item error
- **GIVEN** two readable subtitle files and one path that does not exist passed together
- **WHEN** the user runs `subx-cli --output json detect-encoding <paths>`
- **THEN** the top-level envelope SHALL satisfy `status == "ok"`, `data.files` SHALL contain three entries (two with `status == "ok"`, one with `status == "error"` carrying an `error.category` of `path_not_found` or `file_not_found`), AND the process SHALL exit with status `0`

#### Scenario: Single missing path produces top-level error envelope
- **GIVEN** a single non-existent path passed as the only input
- **WHEN** the user runs `subx-cli --output json detect-encoding <missing>`
- **THEN** the envelope SHALL satisfy `status == "error"`, `error.category` SHALL be `"path_not_found"` or `"file_not_found"`, and the process exit code SHALL equal `SubXError::exit_code` for the underlying variant
