## REMOVED Requirements

### Requirement: Core-Owned Input Collection

**Reason**: Core half of the split, and the requirement A2 wrote precisely to make this move possible: it defines `InputPathHandler` and `CollectedFiles` in `src/core/input/mod.rs` and forbids the module from referencing `clap` or `crate::cli`. Re-added by `import-split-capability-specs`, at `openspec/specs/input-path-handling/spec.md` in that repository. It leaves this repository's half of the capability; it does not leave the project.

**Migration**: Three of its clauses name `crate::cli`. The legacy re-export obligation ("`crate::cli` SHALL continue to re-export both types"), the in-crate call-site rule, and the *Legacy CLI alias still resolves* scenario all constrain `subx-cli` files. They SHALL be lifted into this capability's `subx-cli` half — bullet 5 of *Input Argument Structs Are Thin Adapters Over Core Collection*, added below — and the arriving requirement SHALL retain only the module-definition, no-clap-coupling and rustdoc-example clauses, with the re-export named as `subx-cli`'s obligation in prose. Scenarios *Core module has no argument-parser coupling* and *In-crate call sites use the core path* stay with the core half, the latter re-phrased over `subx-core`'s own `src/` tree.

### Requirement: Unified Path Merging

**Reason**: Core half of the split. A2 defines `merge_paths_from_multiple_sources` in `src/core/input/mod.rs` and requires it to take plain slices so that it is callable without any argument-parser type; the GUI is one such caller. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The closing clause "the CLI's `*Args::get_input_handler` methods SHALL be thin adapters that extract those slices from their clap structs" is a `subx-cli` obligation and SHALL be lifted into bullet 4 of *Input Argument Structs Are Thin Adapters Over Core Collection*. The *Positional and `-i` paths merged* scenario reaches the function through `MatchArgs::get_input_handler`; on arrival the invocation SHALL be named as `subx-cli`'s match command, leaving `merge_paths_from_multiple_sources` as the normative subject.

### Requirement: Extension Filtering

**Reason**: Core half of the split. `with_extensions` and the filter it installs are in `src/core/input/mod.rs` after A2. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The clause "each command SHALL apply the whitelist appropriate to its domain (for example `match` uses video + subtitle extensions, `convert` uses subtitle extensions, `detect-encoding` uses subtitle + `txt`)" is a `subx-cli` obligation and SHALL be lifted into bullet 2 of *Input Argument Structs Are Thin Adapters Over Core Collection*. The sole scenario is phrased over `ConvertArgs::get_input_handler().collect_files()`; because the core half would otherwise carry no scenario, it SHALL be re-phrased over a handler built directly with `with_extensions(&["srt", "ass", "vtt", "sub", "ssa"])`, and the CLI-invocation form appears as the gathered requirement's scenario.

### Requirement: Recursive vs Flat Traversal

**Reason**: Core half of the split. `scan_directory_flat` and `scan_directory_recursive` are private helpers of `src/core/input/mod.rs` under A2. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: Both scenarios say "`--recursive` passed". The flag is `subx-cli`'s and the traversal is core's, so on arrival the flag SHALL be named as `subx-cli`'s option and the recursion mode named as the handler's own state — the treatment C2a applied to `media-discovery`'s *Recursion Controlled by Scan Flag*. The flag definition and its forwarding are bullet 1 of the gathered CLI requirement.

### Requirement: Direct File Inputs Pass Through

**Reason**: Core half of the split. The archive-extension recognition and the temp-directory extraction are `extract_and_collect` in `src/core/input/mod.rs`, over `crate::core::archive`. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: Four of its five scenarios are phrased as `subx convert <path>` invocations whose THEN is entirely about `collect_files`'s return value; they SHALL be carried over with the invocation named as `subx-cli`'s convert command. The fifth, *Archive file with --no-extract is skipped*, is the flag's scenario and SHALL be carried over the same way, because its THEN is also purely about the returned list; the flag's own definition is bullet 1 of the gathered CLI requirement.

### Requirement: Mixed File And Directory Inputs

**Reason**: Core half of the split. The mixed-input handling is `collect_files` in `src/core/input/mod.rs`. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: Both test citations are CLI-bound under B3's ownership test — `tests/match_combined_paths_tests.rs` and `tests/unified_path_handling_tests.rs` both name `subx_cli::cli` types, and B3's design records the second explicitly as staying in `subx-cli` because it also imports `ConvertArgs`, `MatchArgs`, `SyncArgs`, `DetectEncodingArgs`, `OutputSubtitleFormat` and `SyncMethodArg`. Both SHALL be qualified as `subx-cli:tests/match_combined_paths_tests.rs` and `subx-cli:tests/unified_path_handling_tests.rs` on arrival.

### Requirement: Directory Deduplication

**Reason**: Core half of the split. A2 already names "Implemented in `src/core/input/mod.rs` using a `HashSet`". Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The citation `tests/unified_path_handling_tests.rs::test_get_directories` is CLI-bound (see above) and SHALL be qualified as `subx-cli:tests/unified_path_handling_tests.rs::test_get_directories` on arrival.

### Requirement: Invalid Path Surfacing

**Reason**: Core half of the split. `collect_files` returns `SubXError::InvalidPath` from `src/core/input/mod.rs`. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The requirement's rationale clause "so that the CLI caller can surface a clear error instead of silently producing an empty result" describes a consumer, not an obligation on a `subx-cli` file. On arrival "the CLI caller" SHALL be generalised to "the caller", because after the split the GUI is a caller too and the error is returned to whoever asked.

### Requirement: CollectedFiles Return Type

**Reason**: Core half of the split. `CollectedFiles`, its `Deref<Target = Vec<PathBuf>>` and its `TempDir` ownership are defined in `src/core/input/mod.rs` under A2, and the GUI consumes the type directly (SDR §8). Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: No-Extract CLI Flag

**Reason**: Split in two, and this title is retired because after the split the requirement's core half is not about a flag. A2 already carved the requirement into a core bullet (`InputPathHandler::with_no_extract(bool)` in `src/core/input/mod.rs`) and a CLI bullet (the `--no-extract` flag on four commands, forwarded to `with_no_extract`). The core bullet becomes *No-Extract Collection Switch*, added in `subx-core` by `import-split-capability-specs`; the CLI bullet becomes bullet 1 of *Input Argument Structs Are Thin Adapters Over Core Collection*, added below.

**Migration**: Scenario allocation: *Non-CLI caller selects the same behaviour* goes to the core half; *--no-extract disables archive expansion* goes to the gathered CLI requirement. A2's closing sentence "The flag definition remains a CLI concern; the behaviour it selects remains a core concern" is the split's own statement and SHALL be dropped from both halves, because after the split it describes the two requirements rather than constraining either.

### Requirement: Archive Origin Mapping

**Reason**: Core half of the split. `CollectedFiles::archive_origin` and the temp-root-to-archive map are in `src/core/input/mod.rs` under A2. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: Its rationale clause "enabling commands to resolve output directories relative to the original archive location" names a consumer whose obligation is this capability's *Output Directory Resolution for Archive Files* requirement, which stays in `subx-cli`. On arrival the clause SHALL name that requirement as being in `subx-cli` rather than describing the command behaviour, so that the arriving requirement stops at the query API.

### Requirement: CollectedFiles Additional APIs

**Reason**: Core half of the split. `into_paths()` and the `AsRef<[PathBuf]>` impl are on `CollectedFiles` in `src/core/input/mod.rs`. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The closing sentence "Call sites such as `DetectEncodingArgs::get_file_paths()` that currently return `Vec<PathBuf>` SHALL be updated accordingly" is a `subx-cli` obligation and SHALL be lifted into bullet 3 of *Input Argument Structs Are Thin Adapters Over Core Collection*.

## ADDED Requirements

### Requirement: Input Argument Structs Are Thin Adapters Over Core Collection

The argument-parsing layer SHALL own the flag surface for input collection and nothing else. Every collection behaviour is specified by the `input-path-handling` capability in `subx-core`; this requirement states what remains on the `subx-cli` side, and it is written as one requirement rather than five because each item below is the same kind of obligation — declare a flag, forward its value, add no logic.

1. **Flag definitions and forwarding.** Each command that uses `InputPathHandler` (`match`, `convert`, `sync`, `detect-encoding`) SHALL accept a `--no-extract` boolean flag (default `false`) and a `--recursive` boolean flag, and SHALL forward their values to `InputPathHandler::with_no_extract` and to the handler's recursion mode when building the handler. Neither flag SHALL be interpreted in `src/cli/` or `src/commands/` beyond that forwarding.
2. **Domain extension whitelists.** Each command SHALL supply the extension whitelist appropriate to its domain when building its handler — `match` video + subtitle extensions, `convert` subtitle extensions, `detect-encoding` subtitle extensions plus `txt` — through `with_extensions`. The whitelist contents are a CLI decision; the filtering they select is not.
3. **Value-consuming call sites.** Call sites that consume collected paths by value, such as `DetectEncodingArgs::get_file_paths()`, SHALL use `CollectedFiles::into_paths()` or the `AsRef<[PathBuf]>` impl rather than reconstructing a `Vec<PathBuf>` by hand.
4. **Adapters contain no logic.** Every `*Args::get_input_handler` method SHALL be a thin adapter that extracts plain `&[Option<PathBuf>]`, `&[PathBuf]` and `&[String]` slices from its clap struct and passes them to `InputPathHandler::merge_paths_from_multiple_sources`. It SHALL NOT read the filesystem, SHALL NOT filter, and SHALL NOT deduplicate.
5. **Legacy aliases.** `crate::cli` SHALL continue to re-export `InputPathHandler` and `CollectedFiles` so that consumers written against `crate::cli::{InputPathHandler, CollectedFiles}` keep compiling. The re-export SHALL be documented in rustdoc as a legacy alias naming the type's real location in `subx-core`, and SHALL NOT carry a `#[deprecated]` attribute, because the project forbids introducing new ones. No in-crate call site SHALL reach the types through the alias; every `use` inside this crate's `src/` SHALL name the `subx-core` path.

#### Scenario: `--no-extract` disables archive expansion
- **GIVEN** the user runs `subx match -i subs.zip --no-extract`
- **WHEN** `collect_files()` runs
- **THEN** `subs.zip` SHALL NOT be extracted and SHALL be subject to the normal extension filter

#### Scenario: Non-subtitle files ignored by convert
- **GIVEN** a directory containing `movie.srt`, `movie.mp4`, and `notes.txt`, and the convert command
- **WHEN** `ConvertArgs::get_input_handler().collect_files()` runs
- **THEN** the returned list SHALL include `movie.srt` and SHALL NOT include `movie.mp4` or `notes.txt`

#### Scenario: The adapter adds no behaviour
- **GIVEN** any `*Args` value belonging to `match`, `convert`, `sync` or `detect-encoding`
- **WHEN** `get_input_handler` is called on it
- **THEN** the resulting handler SHALL equal the handler produced by calling `merge_paths_from_multiple_sources`, `with_extensions`, `with_no_extract` and the recursion setter directly with the same values, and the method body SHALL contain no filesystem access

#### Scenario: Legacy CLI alias still resolves
- **GIVEN** a consumer that writes `use subx_cli::cli::{CollectedFiles, InputPathHandler};`
- **WHEN** the crate is compiled
- **THEN** the import SHALL resolve to the `subx-core` types and SHALL produce no deprecation warning
