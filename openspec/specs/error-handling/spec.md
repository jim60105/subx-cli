# Error Handling

## Purpose

Provide a single unified error taxonomy (`SubXError`) for every SubX subcommand, with typed variants, automatic error conversions, chained source information, user-facing English messages with remediation hints, and deterministic per-category process exit codes. Implemented in `src/error.rs`, surfaced at the process boundary by `src/main.rs`, and referenced throughout `src/commands/`, `src/core/`, and `src/services/`.

## Requirements

### Requirement: Typed Error Taxonomy

The system SHALL expose a single top-level error enum `SubXError` (`src/error.rs`) covering at minimum the following categories, each represented by a distinct variant: I/O (`Io`), configuration (`Config`), subtitle format (`SubtitleFormat`), AI service (`AiService`, `Api`), audio processing including VAD (`AudioProcessing`), file discovery/matching (`FileMatching`), file-existence outcomes (`FileAlreadyExists`, `FileNotFound`, `InvalidFileName`, `FileOperationFailed`), path handling (`NoInputSpecified`, `InvalidPath`, `PathNotFound`, `DirectoryReadError`), sync validation (`InvalidSyncConfiguration`), unsupported-file-type rejection (`UnsupportedFileType`), generic command execution (`CommandExecution`), and a catch-all `Other(#[from] anyhow::Error)`. The crate SHALL also publish `pub type SubXResult<T> = Result<T, SubXError>`.

#### Scenario: All SubX operations return Result rather than panicking
- **GIVEN** any SubX subcommand entry point under `src/commands/`
- **WHEN** a recoverable failure (invalid input, missing file, network error, decoder error, AI service error, etc.) occurs
- **THEN** the function SHALL return `Err(SubXError::…)` instead of panicking or aborting the process

#### Scenario: Helper constructors exist for common variants
- **GIVEN** the public API of `SubXError`
- **WHEN** application code calls `SubXError::config(msg)`, `SubXError::subtitle_format(fmt, msg)`, `SubXError::audio_processing(msg)`, `SubXError::ai_service(msg)`, or `SubXError::file_matching(msg)`
- **THEN** each helper SHALL produce the corresponding variant whose `Display` output starts with the documented category prefix (e.g., `Configuration error: …`, `Subtitle format error [SRT]: …`, `Audio processing error: …`, `AI service error: …`, `File matching error: …`)

### Requirement: Automatic Error Conversions

The system SHALL provide `From` conversions so that lower-level errors are automatically lifted into `SubXError` at `?` sites: `std::io::Error` → `SubXError::Io` (`#[from]`); `anyhow::Error` → `SubXError::Other` (`#[from]`); `reqwest::Error` → `SubXError::AiService`; `walkdir::Error` → `SubXError::FileMatching`; `symphonia::core::errors::Error` → `SubXError::AudioProcessing`; `config::ConfigError` → `SubXError::Config` (mapping `NotFound` to `Configuration file not found: <path>`); `serde_json::Error` → `SubXError::Config` with a `JSON serialization/deserialization error:` prefix; and `Box<dyn std::error::Error>` → `SubXError::AudioProcessing` (used by the resampler's `Box<dyn Error>` signature).

#### Scenario: std::io::Error lifts into SubXError::Io via ?
- **GIVEN** a function returning `SubXResult<T>` that calls a `std::io` API and propagates with `?`
- **WHEN** the underlying I/O call fails with `io::ErrorKind::NotFound`
- **THEN** the propagated error SHALL match `SubXError::Io(_)`

#### Scenario: Symphonia decode error becomes AudioProcessing
- **GIVEN** a `symphonia::core::errors::Error` produced while decoding
- **WHEN** it is converted into `SubXError` through `From`
- **THEN** the result SHALL match `SubXError::AudioProcessing { .. }` and its `Display` text SHALL start with `Audio processing error:`

#### Scenario: Config crate NotFound error is rewritten with a hint
- **GIVEN** a `config::ConfigError::NotFound("ai.api_key".into())`
- **WHEN** it is converted to `SubXError`
- **THEN** the result SHALL be `SubXError::Config { message }` where `message` contains `Configuration file not found:` and the original path

### Requirement: Chained Error Sources

The system SHALL preserve causal chains via `std::error::Error::source()`. Variants that wrap an underlying error SHALL do so using either `#[from]` (`Io`, `Other`) or `#[source]` (`DirectoryReadError.source`) so that downstream consumers — including integration tests and future logging layers — can walk the chain without losing context.

#### Scenario: DirectoryReadError exposes the originating io::Error
- **GIVEN** a `SubXError::DirectoryReadError { path, source }` produced when reading a directory fails
- **WHEN** a caller inspects `std::error::Error::source()` on the error
- **THEN** the returned reference SHALL be the wrapped `std::io::Error`

### Requirement: User-Facing Error Formatting

`SubXError::Display` (derived via `thiserror`) SHALL produce a concise single-line English message prefixed by the error category, whereas `SubXError::user_friendly_message()` SHALL additionally append a newline and a `Hint:` line with remediation guidance for the major categories (`Config`, `Api`, `AiService`, `SubtitleFormat`, `AudioProcessing`, `FileMatching`, `Other`). All messages, prefixes, and hints SHALL be written in English. The process entry point in `src/main.rs` SHALL render failures via `eprintln!("{}", e.user_friendly_message())` — i.e. the multi-line, hinted form.

#### Scenario: Display is a single-line English message
- **GIVEN** `SubXError::subtitle_format("SRT", "invalid timestamp")`
- **WHEN** `to_string()` is called
- **THEN** the output SHALL equal `Subtitle format error [SRT]: invalid timestamp` with no embedded newline

#### Scenario: Configuration error includes remediation hint
- **GIVEN** `SubXError::config("missing key")`
- **WHEN** `user_friendly_message()` is called
- **THEN** the returned string SHALL contain `Configuration error:` on the first line and `Hint: run 'subx-cli config --help' for details` on a subsequent line

#### Scenario: AI service error advises checking network and API key
- **GIVEN** `SubXError::ai_service("network failure")`
- **WHEN** `user_friendly_message()` is called
- **THEN** the returned string SHALL contain `AI service error:` and `check network connection` and `API key`

### Requirement: Process Exit Code Mapping

`SubXError::exit_code()` SHALL map variants to stable, non-zero exit codes used by `src/main.rs` when the application terminates with an error: `Io → 1`, `Config → 2`, `Api → 3`, `AiService → 3`, `SubtitleFormat → 4`, `AudioProcessing → 5`, `FileMatching → 6`, and every other variant → `1`. On successful completion the process SHALL exit with code `0`.

#### Scenario: Successful run exits 0
- **GIVEN** any SubX subcommand that completes without returning an error from `subx_cli::cli::run().await`
- **WHEN** `main` handles the `Ok(_)` branch
- **THEN** the process SHALL call `std::process::exit(0)`

#### Scenario: Category exit codes are stable
- **GIVEN** freshly constructed errors of each category
- **WHEN** `exit_code()` is called
- **THEN** the returned values SHALL be: `SubXError::config("x") → 2`, `SubXError::subtitle_format("SRT","x") → 4`, `SubXError::audio_processing("x") → 5`, `SubXError::file_matching("x") → 6`, `SubXError::ai_service("x") → 3`, `SubXError::whisper_api("x") → 3`, and `SubXError::Io(io::Error::new(NotFound,"x")) → 1`

#### Scenario: Unmapped variants default to exit code 1
- **GIVEN** a variant not explicitly listed in `exit_code` (e.g. `SubXError::FileAlreadyExists`, `SubXError::UnsupportedFileType`, `SubXError::Other(_)`)
- **WHEN** `exit_code()` is called
- **THEN** it SHALL return `1`

### Requirement: Top-Level Error Rendering

`src/main.rs` SHALL be the single place that converts a `SubXError` into terminal output and a process exit code. On `Err(e)` it SHALL write `e.user_friendly_message()` to standard error via `eprintln!` and then call `std::process::exit(e.exit_code())`. Subcommand implementations SHALL NOT call `std::process::exit` or write category-prefixed error messages to stderr themselves; they SHALL return `Result` up to the entry point.

#### Scenario: Failure path writes to stderr and exits with category code
- **GIVEN** `subx_cli::cli::run().await` returns `Err(SubXError::config("bad key"))`
- **WHEN** `main` handles the error
- **THEN** the program SHALL print the multi-line user-friendly message (including the `Hint:` line) to stderr and call `std::process::exit(2)`

### Requirement: API Error Source Enumeration

Errors originating from external HTTP APIs SHALL be modelled as `SubXError::Api { message, source: ApiErrorSource }` where `ApiErrorSource` distinguishes at least `OpenAI` and `Whisper`. The helper `SubXError::whisper_api(msg)` SHALL produce an `Api` variant whose source is `ApiErrorSource::Whisper`, and both `Api` and `AiService` SHALL share exit code `3`.

#### Scenario: Whisper API helper carries the Whisper source
- **GIVEN** `SubXError::whisper_api("rate limited")`
- **WHEN** the variant is inspected
- **THEN** it SHALL match `SubXError::Api { source: ApiErrorSource::Whisper, .. }` and `exit_code()` SHALL return `3`

### Requirement: No Panics On Recoverable Errors

SubX subcommands SHALL NOT panic, `unwrap`, or `expect` on conditions that represent user-facing recoverable failures (invalid configuration, missing or unreadable files, unsupported formats, network failures, AI response errors, empty inputs, etc.); every such failure SHALL instead return an appropriately typed `SubXError`. The configuration loader (`src/config/`) and the match engine (`src/core/matcher/`) SHALL both surface invalid input through `SubXError::Config` / `SubXError::FileMatching` rather than aborting, as verified by `tests/config_validation_tests.rs`, `tests/match_engine_error_display_integration_tests.rs`, and `tests/match_engine_error_handling_integration_tests.rs`.

#### Scenario: Invalid configuration value is reported, not panicked
- **GIVEN** a configuration value that fails validation (e.g. out-of-range `sync.vad.sensitivity`)
- **WHEN** validation runs
- **THEN** the code path SHALL return `Err(SubXError::Config { .. })` and the process SHALL NOT unwind via panic

#### Scenario: Match-engine failure renders through the unified pipeline
- **GIVEN** a match-engine call that fails (e.g. no matching files)
- **WHEN** the error reaches `main`
- **THEN** stderr SHALL contain the category-prefixed message (e.g. `File matching error: …`) and the process SHALL exit with the mapped code (`6` for `FileMatching`)

### Requirement: Sanitized upstream error messages

When an AI API returns an error response, the system SHALL truncate the error body to a maximum of 500 characters before including it in `SubXError`. The system SHALL strip any HTTP headers or request metadata from the error message to avoid leaking sensitive or excessive information to end users.

#### Scenario: long error body is truncated
- **WHEN** the AI API returns a 10 KiB error body
- **THEN** the `SubXError` message SHALL contain at most 500 characters of the body followed by `... (truncated)`

#### Scenario: short error body is preserved
- **WHEN** the AI API returns a 200-character error body
- **THEN** the full body SHALL be included in the error message

### Requirement: No sensitive data in error chains

Error types and messages SHALL NOT include API keys, authentication tokens, or full request/response URLs that contain query parameters. If a URL must be included, only the scheme, host, and path components SHALL be shown.

#### Scenario: error with URL strips query params
- **WHEN** an HTTP error includes a URL with query parameters
- **THEN** the error message SHALL show only `scheme://host/path`

#### Scenario: API key never in error message
- **WHEN** an AI service error occurs
- **THEN** the error message and its chain SHALL NOT contain any API key value
