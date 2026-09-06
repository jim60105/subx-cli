# Error Handling

## Purpose

Hold the presentation half of the error-handling agreement: user-facing English formatting with remediation hints, deterministic per-category process exit codes, top-level rendering at the process boundary, the no-panic obligation on subcommands, output-mode-aware rendering, and the `SubXErrorExt` binary extension trait. Implemented in `src/cli/error_ext.rs` and surfaced by `src/main.rs`. The `SubXError` taxonomy, its conversions and sources, library-side `Display`, and the machine-readable category/code contract are specified by the same-named `error-handling` capability in `subx-core`.

## Requirements

### Requirement: User-Facing Error Formatting

`SubXErrorExt::user_friendly_message()` — defined in `src/cli/error_ext.rs` and implemented for `SubXError` — SHALL append to the error's `Display` output a newline and a `Hint:` line with remediation guidance for the major categories (`Config`, `Api`, `AiService`, `SubtitleFormat`, `AudioProcessing`, `FileMatching`, `Other`). All messages, prefixes, and hints SHALL be written in English. The process entry point in `src/main.rs` SHALL import `SubXErrorExt` and render failures via `eprintln!("{}", e.user_friendly_message())` — i.e. the multi-line, hinted form.

`Display`'s own contract — a concise single-line English message prefixed by the error category, inherent on `SubXError` and available without importing any trait — is specified by the `error-handling` capability's *Display Is the Library's Error Rendering* requirement in `subx-core`. `user_friendly_message()` is a binary-side capability and is unavailable to callers that have not imported the trait.

The English-language rule is deliberately stated in both halves. It is a project-wide editorial constraint rather than an obligation on a particular function, and dropping it from either half would let that side's prose drift without violating anything. It SHALL NOT be treated as a duplication to be removed.

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

### Requirement: No Panics On Recoverable Errors

SubX subcommands under `src/commands/` SHALL NOT panic, `unwrap`, or `expect` on conditions that represent user-facing recoverable failures (invalid configuration, missing or unreadable files, unsupported formats, network failures, AI response errors, empty inputs, etc.); every such failure SHALL instead be returned as an appropriately typed `SubXError` up to the process entry point, which renders it per this capability's *Top-Level Error Rendering* requirement.

The equivalent obligation on library code — that the configuration loader and the match engine surface invalid input as `SubXError::Config` / `SubXError::FileMatching` rather than aborting — is specified by the `error-handling` capability's *Library Code Surfaces Recoverable Failures as Errors* requirement in `subx-core`. Verified here by `tests/match_engine_error_handling_integration_tests.rs`, which is CLI-bound under B3's ownership test.

#### Scenario: Match-engine failure renders through the unified pipeline
- **GIVEN** a match-engine call that fails (e.g. no matching files)
- **WHEN** the error reaches `main`
- **THEN** stderr SHALL contain the category-prefixed message (e.g. `File matching error: …`) and the process SHALL exit with the mapped code (`6` for `FileMatching`)

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

### Requirement: Binary Error Surface Adds Presentation Through an Extension Trait

The binary's presentation contracts SHALL be added to `SubXError` from outside the library, through an extension trait, and SHALL NOT be inherent methods on the type.

- `pub trait SubXErrorExt`, defined in `src/cli/error_ext.rs` and implemented for `SubXError`, SHALL provide exactly `fn exit_code(&self) -> i32` and `fn user_friendly_message(&self) -> String`.
- Both methods SHALL carry the bodies they had as inherent methods before A2's split, unchanged, so that no exit code, message, prefix, or `Hint:` line differs from the pre-split behaviour.
- Callers SHALL import the trait (`use crate::cli::error_ext::SubXErrorExt;`) at the sites that need it: `src/main.rs` and `ErrorEnvelope::from_error` in `src/cli/output.rs`.
- The trait SHALL NOT be re-exported in a way that makes either method reachable without an explicit import, because the import is what documents that a presentation contract is being used.
- The library-side half of this contract — which items remain inherent on `SubXError`, that library code may not call these two methods, and that `hint()` and `OutputModeUnsupported` stay in the core enum — is specified by the `error-handling` capability's *Library Error Surface Holds Only Machine Contracts* requirement in `subx-core`.

#### Scenario: Presentation methods require the extension trait
- **GIVEN** a module that holds a `SubXError` value and does not import `SubXErrorExt`
- **WHEN** it calls `err.exit_code()` or `err.user_friendly_message()`
- **THEN** compilation SHALL fail, because neither is an inherent method

#### Scenario: The trait lives in the binary crate
- **GIVEN** a consumer that depends on `subx-core` and not on `subx-cli`
- **WHEN** it searches the library's public API for `exit_code` and `user_friendly_message`
- **THEN** neither SHALL be present, and the consumer SHALL be able to obtain a rendered message only through `Display`, optionally combined with `hint()`