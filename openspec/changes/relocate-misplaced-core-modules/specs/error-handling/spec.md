## ADDED Requirements

### Requirement: Library and Binary Error Surface Split

The `SubXError` surface SHALL be partitioned so that machine contracts belong to the library and presentation contracts belong to the binary.

**Library half — inherent items on `SubXError` in `src/error.rs`:**

- The enum itself and all of its variants, every `From` conversion, every helper constructor, and `ApiErrorSource`.
- `category()`, `machine_code()`, and `hint()`.

**Binary half — `pub trait SubXErrorExt` in `src/cli/error_ext.rs`, implemented for `SubXError`:**

- `fn exit_code(&self) -> i32`
- `fn user_friendly_message(&self) -> String`

Both trait methods SHALL carry the bodies they had as inherent methods, unchanged, so that no exit code, message, prefix, or `Hint:` line differs from before the split. Callers SHALL import the trait (`use crate::cli::error_ext::SubXErrorExt;`) at the four in-crate sites that need it: `src/main.rs` and `src/cli/output.rs`'s `ErrorEnvelope::from_error`.

Additional constraints:

- Code under `src/core/` and `src/services/` SHALL NOT call `exit_code()` or `user_friendly_message()`, and SHALL NOT import `SubXErrorExt`. Where such code needs a rendered message it SHALL use `Display` (`to_string()`), optionally combined with `hint()`.
- `hint()` SHALL remain an inherent method on `SubXError` even though its returned prose names the `subx-cli` binary and its flags. Its rustdoc SHALL record that the text is written for the terminal, that library consumers should treat the return value as an availability signal rather than display copy, and that the identity of the variants returning `Some` is the stable part of the contract.
- The `OutputModeUnsupported` variant SHALL remain a variant of the core enum even though only the binary constructs it, so that `category()` and `machine_code()` keep their wildcard-free exhaustive matches. Its rustdoc SHALL record that only the binary constructs it.

#### Scenario: Presentation methods require the extension trait
- **GIVEN** a module that holds a `SubXError` value and does not import `SubXErrorExt`
- **WHEN** it calls `err.exit_code()` or `err.user_friendly_message()`
- **THEN** compilation SHALL fail, because neither is an inherent method

#### Scenario: Machine contracts need no import
- **GIVEN** any module holding a `SubXError` value, with no trait imported
- **WHEN** it calls `err.category()`, `err.machine_code()`, or `err.hint()`
- **THEN** all three calls SHALL compile and SHALL return the same values as before the split

#### Scenario: Core does not depend on presentation
- **GIVEN** the source trees `src/core/` and `src/services/`
- **WHEN** they are searched for `SubXErrorExt`, `exit_code`, and `user_friendly_message`
- **THEN** there SHALL be no call site and no import of any of them

#### Scenario: Core renders operation errors through Display
- **GIVEN** the audit path in `src/core/matcher/engine.rs` that turns a failed file operation into per-operation error metadata
- **WHEN** it renders the error's `message` field
- **THEN** it SHALL use the error's `Display` output, and for the only variant it constructs — `SubXError::FileOperationFailed` — that output SHALL be byte-identical to `user_friendly_message()`, preserving the per-item message contract of the `machine-readable-output` capability

## MODIFIED Requirements

### Requirement: User-Facing Error Formatting

`SubXError::Display` (derived via `thiserror`) SHALL produce a concise single-line English message prefixed by the error category. `SubXErrorExt::user_friendly_message()` — defined in `src/cli/error_ext.rs` and implemented for `SubXError` — SHALL additionally append a newline and a `Hint:` line with remediation guidance for the major categories (`Config`, `Api`, `AiService`, `SubtitleFormat`, `AudioProcessing`, `FileMatching`, `Other`). All messages, prefixes, and hints SHALL be written in English. The process entry point in `src/main.rs` SHALL import `SubXErrorExt` and render failures via `eprintln!("{}", e.user_friendly_message())` — i.e. the multi-line, hinted form.

`Display` remains an inherent, library-side capability of `SubXError`; `user_friendly_message()` is a binary-side capability and is unavailable to callers that have not imported the trait.

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

#### Scenario: File-operation failures render identically either way
- **GIVEN** `SubXError::FileOperationFailed("could not rename".into())`
- **WHEN** both `to_string()` and `user_friendly_message()` are called
- **THEN** the two strings SHALL be equal, so that library-side rendering of this variant matches binary-side rendering exactly

### Requirement: Process Exit Code Mapping

`SubXErrorExt::exit_code()` SHALL map variants to stable, non-zero exit codes used by `src/main.rs` when the application terminates with an error: `Io → 1`, `Config → 2`, `Api → 3`, `AiService → 3`, `SubtitleFormat → 4`, `AudioProcessing → 5`, `FileMatching → 6`, and every other variant → `1`. On successful completion the process SHALL exit with code `0`.

Exit codes are a property of the process, not of the error taxonomy, so this mapping SHALL live in the binary's extension trait (`src/cli/error_ext.rs`) rather than as an inherent method on `SubXError`. The numeric mapping itself SHALL NOT change.

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

`src/main.rs` SHALL be the single place that converts a `SubXError` into terminal output and a process exit code. It SHALL import `crate::cli::error_ext::SubXErrorExt` for that purpose. On `Err(e)` it SHALL write `e.user_friendly_message()` to standard error via `eprintln!` and then call `std::process::exit(e.exit_code())`. Subcommand implementations SHALL NOT call `std::process::exit` or write category-prefixed error messages to stderr themselves; they SHALL return `Result` up to the entry point.

#### Scenario: Failure path writes to stderr and exits with category code
- **GIVEN** `subx_cli::cli::run().await` returns `Err(SubXError::config("bad key"))`
- **WHEN** `main` handles the error
- **THEN** the program SHALL print the multi-line user-friendly message (including the `Hint:` line) to stderr and call `std::process::exit(2)`

### Requirement: Stable Machine-Readable Category and Code

`SubXError` SHALL expose three pure helper methods on every variant, as **inherent** methods available without importing any trait:

- `pub fn category(&self) -> &'static str` returning a stable snake_case identifier from the closed set: `io`, `config`, `subtitle_format`, `ai_service`, `api`, `audio_processing`, `file_matching`, `file_already_exists`, `file_not_found`, `invalid_file_name`, `file_operation_failed`, `command_execution`, `no_input_specified`, `invalid_path`, `path_not_found`, `directory_read_error`, `invalid_sync_configuration`, `unsupported_file_type`, `other`.
- `pub fn machine_code(&self) -> &'static str` returning a stable upper-snake-case identifier prefixed with `E_` (for example `E_IO`, `E_CONFIG`, `E_SUBTITLE_FORMAT`, `E_AI_SERVICE`, `E_API`, `E_AUDIO_PROCESSING`, `E_FILE_MATCHING`, `E_FILE_ALREADY_EXISTS`, `E_FILE_NOT_FOUND`, `E_INVALID_FILE_NAME`, `E_FILE_OPERATION_FAILED`, `E_COMMAND_EXECUTION`, `E_NO_INPUT_SPECIFIED`, `E_INVALID_PATH`, `E_PATH_NOT_FOUND`, `E_DIRECTORY_READ_ERROR`, `E_INVALID_SYNC_CONFIGURATION`, `E_UNSUPPORTED_FILE_TYPE`, `E_OTHER`).
- `pub fn hint(&self) -> Option<&'static str>` returning a short remediation string, or `None` where none applies.

The `category()` and `machine_code()` implementations SHALL use an exhaustive `match` (no wildcard arm) so the compiler enforces updates whenever a new variant is added; the `OutputModeUnsupported` variant SHALL therefore remain part of the enum. All three helpers SHALL be pure (no I/O, no allocation) and SHALL NOT change `Display`, `SubXErrorExt::exit_code`, or `SubXErrorExt::user_friendly_message`.

Because these three are the contract consumed by non-terminal front ends, they SHALL remain inherent methods on `SubXError` and SHALL NOT be moved to the binary's extension trait — including `hint()`, whose prose is deliberately CLI-flavoured (see the "Library and Binary Error Surface Split" requirement).

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

#### Scenario: Hint is reachable without the presentation trait
- **GIVEN** a consumer that imports only `SubXError` and no extension trait
- **WHEN** it calls `err.hint()`
- **THEN** the call SHALL compile and SHALL return `Some(_)` for exactly the variants that returned `Some(_)` before the split

### Requirement: Process-Boundary Rendering Honors Output Mode

The process-boundary error rendering in `src/main.rs` SHALL consult the active output mode:

- In `text` mode (default), errors SHALL be printed as today via `print_error` and the existing user-friendly message path; behavior is unchanged.
- In `json` mode, errors SHALL be emitted as the JSON error envelope on stdout (a single document terminated by `\n`), using `category()`, `machine_code()`, `hint()`, `SubXErrorExt::exit_code()`, and `SubXErrorExt::user_friendly_message()` as inputs. The envelope is assembled by `ErrorEnvelope::from_error` in `src/cli/output.rs`, which imports `SubXErrorExt` for the latter two.

In both modes the process SHALL exit with `SubXErrorExt::exit_code`.

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
