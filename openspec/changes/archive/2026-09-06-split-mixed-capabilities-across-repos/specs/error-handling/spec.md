## REMOVED Requirements

### Requirement: Typed Error Taxonomy

**Reason**: Core half of the split. `SubXError`, its variants, its helper constructors and `SubXResult` are all in `src/error.rs`, which is `subx-core` after B2, and the Tauri GUI consumes six of the variants and three of the constructors directly (SDR §8). Re-added by `import-split-capability-specs`, at `openspec/specs/error-handling/spec.md` in that repository. It leaves this repository's half of the capability; it does not leave the project.

**Migration**: Its first scenario is phrased over "any SubX subcommand entry point under `src/commands/`". The obligation — that a recoverable failure returns `Err(SubXError::…)` rather than panicking — is a property of the library's callers, and the command-entry-point half of it is stated by this capability's *No Panics On Recoverable Errors* requirement, which stays in `subx-cli`. On arrival the scenario SHALL be re-phrased over any caller of a fallible library function, with `subx-cli`'s command entry points named as one such caller and qualified, so that the arriving requirement does not cite an unqualified `src/commands/` path.

### Requirement: Automatic Error Conversions

**Reason**: Core half of the split. Every `From` impl the requirement enumerates is in `src/error.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Chained Error Sources

**Reason**: Core half of the split. The `#[from]` and `#[source]` attributes are on the variants in `src/error.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: API Error Source Enumeration

**Reason**: Core half of the split. `ApiErrorSource` and `SubXError::whisper_api` are in `src/error.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: The sentence "both `Api` and `AiService` SHALL share exit code `3`" refers to a mapping that A2 moved to `SubXErrorExt::exit_code()` in `src/cli/error_ext.rs`. On arrival it SHALL be re-phrased to name the *category* both variants share and to attribute the numeric mapping to this capability's *Process Exit Code Mapping* requirement in `subx-cli`, so that the arriving requirement does not assert an obligation over a `subx-cli` file. The scenario's `exit_code()` assertion SHALL be re-phrased the same way.

### Requirement: Sanitized upstream error messages

**Reason**: Core half of the split. The 500-character truncation and the header stripping are applied where the AI response body is read, under `src/services/ai/`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: No sensitive data in error chains

**Reason**: Core half of the split. The URL redaction and the API-key exclusion are enforced in the error construction sites under `src/services/ai/`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: The `local-llm-provider` capability in `subx-core` cites this requirement's URL clause, and C2a qualified that citation as being in `subx-cli`. `import-split-capability-specs` SHALL re-qualify it to unqualified form, because after this change both capabilities are in `subx-core`.

### Requirement: Stable Machine-Readable Category and Code

**Reason**: Core half of the split. A2 makes `category()`, `machine_code()` and `hint()` inherent methods on `SubXError` in `src/error.rs` and states explicitly that they SHALL NOT move to the binary's extension trait. `category()` is load-bearing for the GUI, which formats it into the i18n key `core.{category}` and branches on three literal values (SDR §8), so its home must be the crate the GUI depends on. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: Its closing sentence — "SHALL NOT change `Display`, `SubXErrorExt::exit_code`, or `SubXErrorExt::user_friendly_message`" — names two items that live in `src/cli/error_ext.rs`. On arrival the two `SubXErrorExt` names SHALL be qualified as belonging to `subx-cli`, and the *Category and exit code mapping are consistent* scenario's `exit_code()` assertion SHALL be attributed to this capability's `subx-cli` half in the same way as *API Error Source Enumeration* above.

### Requirement: Library and Binary Error Surface Split

**Reason**: Split in two, and this title is retired rather than retained, because the requirement defines both halves by construction and neither half answers the question the title asks. A2 wrote it as two labelled blocks — "Library half — inherent items on `SubXError` in `src/error.rs`" and "Binary half — `pub trait SubXErrorExt` in `src/cli/error_ext.rs`" — plus three additional constraints, and after B2 those two blocks are in two repositories. The library block becomes *Library Error Surface Holds Only Machine Contracts*, added in `subx-core` by `import-split-capability-specs`; the binary block becomes *Binary Error Surface Adds Presentation Through an Extension Trait*, added below.

**Migration**: The clause-by-clause allocation, which SHALL be applied exactly so that no obligation is lost and none is stated twice:
- To `subx-core`: the enum, all variants, every `From` conversion, every helper constructor, `ApiErrorSource`, `category()`, `machine_code()`, `hint()`; the prohibition on `src/core/` and `src/services/` calling `exit_code()` or `user_friendly_message()` or importing the trait, together with the instruction to use `Display` (optionally with `hint()`) instead; the `hint()` rustdoc obligation; and the `OutputModeUnsupported` retention with its rustdoc note. Scenarios *Machine contracts need no import*, *Core does not depend on presentation* and *Core renders operation errors through Display*. The core half additionally carries the narrow guarantee that `Display` is unchanged by the split.
- To `subx-cli`: `SubXErrorExt` and its two methods, the "bodies unchanged, so that no exit code, message, prefix, or `Hint:` line differs from before the split" guarantee, and the import sites. Scenario *Presentation methods require the extension trait*.
- The `subx-core` half's citation of `src/cli/error_ext.rs` SHALL be qualified as `subx-cli:src/cli/error_ext.rs` wherever the prohibition needs to name the trait's location.

## MODIFIED Requirements

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

### Requirement: No Panics On Recoverable Errors

SubX subcommands under `src/commands/` SHALL NOT panic, `unwrap`, or `expect` on conditions that represent user-facing recoverable failures (invalid configuration, missing or unreadable files, unsupported formats, network failures, AI response errors, empty inputs, etc.); every such failure SHALL instead be returned as an appropriately typed `SubXError` up to the process entry point, which renders it per this capability's *Top-Level Error Rendering* requirement.

The equivalent obligation on library code — that the configuration loader and the match engine surface invalid input as `SubXError::Config` / `SubXError::FileMatching` rather than aborting — is specified by the `error-handling` capability's *Library Code Surfaces Recoverable Failures as Errors* requirement in `subx-core`. Verified here by `tests/match_engine_error_handling_integration_tests.rs`, which is CLI-bound under B3's ownership test.

#### Scenario: Match-engine failure renders through the unified pipeline
- **GIVEN** a match-engine call that fails (e.g. no matching files)
- **WHEN** the error reaches `main`
- **THEN** stderr SHALL contain the category-prefixed message (e.g. `File matching error: …`) and the process SHALL exit with the mapped code (`6` for `FileMatching`)

## ADDED Requirements

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
