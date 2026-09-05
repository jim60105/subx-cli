## ADDED Requirements

### Requirement: Core-Owned Input Collection

The input collection algorithm SHALL be owned by the core library, not by the argument-parsing layer. Specifically:

- `InputPathHandler` and `CollectedFiles`, together with every associated item (`from_args`, `merge_paths_from_multiple_sources`, `with_extensions`, `with_no_extract`, `validate`, `get_directories`, `collect_files`, and the private `matches_extension` / `extract_and_collect` / `scan_directory_flat` / `scan_directory_recursive` helpers; `CollectedFiles::{new, with_archives, archive_origin, into_paths}` and its `Deref` / `AsRef` impls), SHALL be defined in `src/core/input/mod.rs` and reachable as `crate::core::input::{InputPathHandler, CollectedFiles}`.
- The module SHALL NOT reference `clap`, `crate::cli`, or any other argument-parsing type. Its only permitted dependencies are the standard library, `log`, `tempfile`, `crate::core::archive`, and `crate::error`.
- `crate::cli` SHALL continue to re-export both types so that existing consumers written against `crate::cli::{InputPathHandler, CollectedFiles}` keep compiling. The re-export SHALL be documented in rustdoc as a legacy alias naming the new location; it SHALL NOT carry a `#[deprecated]` attribute, because the project forbids introducing new ones.
- No in-crate call site SHALL reach the types through the legacy alias; every `use` inside `src/` SHALL name `crate::core::input`.
- Rustdoc examples inside the module SHALL be expressible without any argument-parsing type, so that they remain compilable once the module ships in a library crate that cannot depend on the binary crate.

#### Scenario: Core module has no argument-parser coupling
- **GIVEN** the file `src/core/input/mod.rs`
- **WHEN** its imports and rustdoc examples are inspected
- **THEN** they SHALL contain no reference to `clap`, to `crate::cli`, or to any `*Args` type

#### Scenario: Legacy CLI alias still resolves
- **GIVEN** a consumer that writes `use subx_cli::cli::{CollectedFiles, InputPathHandler};`
- **WHEN** the crate is compiled
- **THEN** the import SHALL resolve to the `crate::core::input` types and SHALL produce no deprecation warning

#### Scenario: In-crate call sites use the core path
- **GIVEN** the source tree under `src/`
- **WHEN** it is searched for `cli::InputPathHandler` and `cli::CollectedFiles`
- **THEN** the only matches SHALL be the legacy re-export declarations themselves

## MODIFIED Requirements

### Requirement: Unified Path Merging

The system SHALL provide `InputPathHandler::merge_paths_from_multiple_sources(optional_paths, input_paths, string_paths)` — defined in `src/core/input/mod.rs` and reachable as `crate::core::input::InputPathHandler::merge_paths_from_multiple_sources` — so that each command can combine its positional `Option<PathBuf>`, its repeated `-i` arguments, and any additional string-path arguments into one deduplicated `Vec<PathBuf>`. The function SHALL take plain `&[Option<PathBuf>]`, `&[PathBuf]` and `&[String]` slices so that it is callable without any argument-parser type; the CLI's `*Args::get_input_handler` methods SHALL be thin adapters that extract those slices from their clap structs.

#### Scenario: Positional and `-i` paths merged
- **GIVEN** the user runs `subx match ./dirA -i ./dirB -i ./dirC`
- **WHEN** `MatchArgs::get_input_handler` resolves paths
- **THEN** the resulting handler SHALL contain `./dirA`, `./dirB`, and `./dirC`

#### Scenario: No input at all is rejected
- **GIVEN** a command that requires at least one input source and the user supplies none
- **WHEN** `merge_paths_from_multiple_sources` is called with empty inputs
- **THEN** the call SHALL return an error (for example `SubXError::NoInputSpecified`) rather than returning an empty list silently

#### Scenario: Merging is callable without a clap struct
- **GIVEN** a caller that is not a CLI command — for example a GUI front end or a unit test
- **WHEN** it calls `merge_paths_from_multiple_sources` with hand-built slices
- **THEN** the call SHALL compile and behave identically to the same call made from a `*Args::get_input_handler` adapter

### Requirement: Directory Deduplication

`InputPathHandler::get_directories()` SHALL return a deduplicated set of directories that covers every supplied input (using each file's parent directory and each supplied directory itself), such that the same directory reached via multiple input paths SHALL appear exactly once in the returned list. Implemented in `src/core/input/mod.rs` using a `HashSet` and exercised by `tests/unified_path_handling_tests.rs::test_get_directories`.

#### Scenario: Same directory reached via two inputs
- **GIVEN** an input list containing a directory `dir1` and a file `dir1/file2.srt` whose parent is `dir1`
- **WHEN** `get_directories()` is called on the resulting handler
- **THEN** the returned list SHALL contain `dir1` exactly once

### Requirement: No-Extract CLI Flag

Archive expansion SHALL be controlled by a core builder method and surfaced by a CLI flag, with the two kept distinct:

- `InputPathHandler::with_no_extract(bool)` (`src/core/input/mod.rs`) SHALL be the core-level switch. When it is set to `true`, `collect_files()` SHALL treat archive files as opaque regular files, subject to the normal extension filter.
- Each command that uses `InputPathHandler` (`match`, `convert`, `sync`, `detect-encoding`) SHALL accept a `--no-extract` boolean flag (default `false`) and SHALL forward its value to `with_no_extract` when building the handler. The flag definition remains a CLI concern; the behaviour it selects remains a core concern.

#### Scenario: --no-extract disables archive expansion
- **GIVEN** the user runs `subx match -i subs.zip --no-extract`
- **WHEN** `collect_files()` runs
- **THEN** `subs.zip` SHALL NOT be extracted and SHALL be subject to the
  normal extension filter

#### Scenario: Non-CLI caller selects the same behaviour
- **GIVEN** a caller that builds an `InputPathHandler` directly and calls `.with_no_extract(true)` without any command-line parsing
- **WHEN** `collect_files()` runs over an archive input
- **THEN** the archive SHALL be treated as a regular file, identically to the `--no-extract` invocation
