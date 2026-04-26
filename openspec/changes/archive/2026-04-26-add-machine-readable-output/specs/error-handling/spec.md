## ADDED Requirements

### Requirement: Stable Machine-Readable Category and Code

`SubXError` SHALL expose two pure helper methods on every variant:

- `pub fn category(&self) -> &'static str` returning a stable snake_case identifier from the closed set: `io`, `config`, `subtitle_format`, `ai_service`, `api`, `audio_processing`, `file_matching`, `file_already_exists`, `file_not_found`, `invalid_file_name`, `file_operation_failed`, `command_execution`, `no_input_specified`, `invalid_path`, `path_not_found`, `directory_read_error`, `invalid_sync_configuration`, `unsupported_file_type`, `other`.
- `pub fn machine_code(&self) -> &'static str` returning a stable upper-snake-case identifier prefixed with `E_` (for example `E_IO`, `E_CONFIG`, `E_SUBTITLE_FORMAT`, `E_AI_SERVICE`, `E_API`, `E_AUDIO_PROCESSING`, `E_FILE_MATCHING`, `E_FILE_ALREADY_EXISTS`, `E_FILE_NOT_FOUND`, `E_INVALID_FILE_NAME`, `E_FILE_OPERATION_FAILED`, `E_COMMAND_EXECUTION`, `E_NO_INPUT_SPECIFIED`, `E_INVALID_PATH`, `E_PATH_NOT_FOUND`, `E_DIRECTORY_READ_ERROR`, `E_INVALID_SYNC_CONFIGURATION`, `E_UNSUPPORTED_FILE_TYPE`, `E_OTHER`).

The implementation SHALL use an exhaustive `match` (no wildcard arm) so the compiler enforces updates whenever a new variant is added. Both helpers SHALL be pure (no I/O, no allocation) and SHALL NOT change `Display`, `exit_code`, or `user_friendly_message`.

#### Scenario: Every variant has a category and machine code
- **GIVEN** any `SubXError` value
- **WHEN** `category()` and `machine_code()` are called
- **THEN** both calls SHALL return non-empty `&'static str` values from the documented closed sets

#### Scenario: Category and exit code mapping are consistent
- **GIVEN** a `SubXError::AiService(_)` value
- **WHEN** `category()`, `machine_code()`, and `exit_code()` are called
- **THEN** they SHALL return `"ai_service"`, `"E_AI_SERVICE"`, and `3` respectively

#### Scenario: Adding a new variant breaks the build until mapped
- **GIVEN** the source code is modified to add a new `SubXError` variant
- **WHEN** the crate is compiled
- **THEN** compilation SHALL fail in `category()` and `machine_code()` due to the exhaustive match, forcing the contributor to assign stable identifiers

### Requirement: Process-Boundary Rendering Honors Output Mode

The process-boundary error rendering in `src/main.rs` SHALL consult the active output mode:

- In `text` mode (default), errors SHALL be printed as today via `print_error` and the existing user-friendly message path; behavior is unchanged.
- In `json` mode, errors SHALL be emitted as the JSON error envelope on stdout (a single document terminated by `\n`), using `category()`, `machine_code()`, `exit_code()`, and `user_friendly_message()` as inputs.

In both modes the process SHALL exit with `SubXError::exit_code`.

Additionally, `main.rs` SHALL invoke clap via `Cli::try_parse()` and, on `Err(clap::Error)`, render either the standard clap text message (text mode) or a synthetic JSON error envelope (JSON mode) with `error.category == "argument_parsing"`, `error.code == "E_ARGUMENT_PARSING"`, and `error.exit_code` equal to `clap::Error::exit_code()`. The active output mode for clap errors is determined by an early argv/env sniff (see the `machine-readable-output` capability's "CLI Parsing Flow Honors Output Mode" requirement).

#### Scenario: Text mode unchanged at the boundary
- **GIVEN** a `SubXError::Config { .. }` produced by a subcommand
- **WHEN** the binary runs without `--output json`
- **THEN** stderr SHALL contain the existing `✗ <user_friendly_message>` line and the process SHALL exit with code `2`, identical to pre-change behavior

#### Scenario: JSON mode renders the error envelope
- **GIVEN** a `SubXError::AiService("network timeout".into())` produced by a subcommand
- **WHEN** the binary runs with `--output json`
- **THEN** stdout SHALL contain a single JSON object with `status == "error"`, `error.category == "ai_service"`, `error.code == "E_AI_SERVICE"`, `error.exit_code == 3`, and `error.message` equal to the value `user_friendly_message()` would have returned, and the process SHALL exit with code `3`

#### Scenario: Synthetic envelope for clap parse failures in JSON mode
- **GIVEN** the user invokes the binary with an unknown flag and tentative JSON mode (via `--output json` earlier in argv, or `SUBX_OUTPUT=json`)
- **WHEN** clap returns an `Err(clap::Error)`
- **THEN** stdout SHALL contain a synthetic JSON error envelope with `status == "error"`, `error.category == "argument_parsing"`, `error.code == "E_ARGUMENT_PARSING"`, and `error.exit_code` equal to `clap::Error::exit_code()`, AND the process SHALL exit with that exit code
