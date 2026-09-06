# Input Path Handling

## Purpose

Resolve the heterogeneous input-path arguments used across `match`, `convert`, `sync`, and `detect-encoding` into a single filtered list of files, supporting positional paths, repeated `-i/--input` flags, string paths, and optional recursion. Implemented in `src/core/input/mod.rs` (`InputPathHandler`), with `crate::cli` re-exports retained as legacy aliases.

## Requirements

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

The system SHALL accept individual file paths (not just directories) as inputs. If a file has a recognised archive extension (`.zip`, `.rar`, `.7z`, `.tar.gz`, `.tgz`) and archive extraction is enabled, the system SHALL extract the archive to a temporary directory and include the extracted files in the result instead of the archive path itself. For non-archive files, the system SHALL return them unchanged when they match the configured extension filter.

#### Scenario: Single-file input
- **GIVEN** the user runs `subx convert movie.srt`
- **WHEN** `collect_files` runs
- **THEN** the returned list SHALL contain exactly `movie.srt`

#### Scenario: Archive file input is extracted
- **GIVEN** the user runs `subx convert subs.zip` and the zip contains
  `movie.srt` and `movie2.ass`
- **WHEN** `collect_files` runs
- **THEN** the returned list SHALL contain the extracted `movie.srt` and
  `movie2.ass` from the temp directory, and SHALL NOT contain `subs.zip`

#### Scenario: 7z archive file input is extracted
- **GIVEN** the user runs `subx convert subs.7z` and the 7z contains
  `movie.srt`
- **WHEN** `collect_files` runs
- **THEN** the returned list SHALL contain the extracted `movie.srt` from
  the temp directory, and SHALL NOT contain `subs.7z`

#### Scenario: Tar.gz archive file input is extracted
- **GIVEN** the user runs `subx convert subs.tar.gz` and the archive
  contains `movie.srt`
- **WHEN** `collect_files` runs
- **THEN** the returned list SHALL contain the extracted `movie.srt` from
  the temp directory, and SHALL NOT contain `subs.tar.gz`

#### Scenario: Archive file with --no-extract is skipped
- **GIVEN** the user runs `subx convert subs.zip --no-extract`
- **WHEN** `collect_files` runs
- **THEN** `subs.zip` SHALL be treated as a regular file, SHALL fail
  the extension filter (`.zip` is not a subtitle extension), and SHALL
  NOT appear in the result

### Requirement: Mixed File And Directory Inputs

The system SHALL accept a mixture of file, directory, and archive entries within the same input list; on `collect_files()` it SHALL return the matched files from every supplied directory (filtered by the configured extensions and traversal mode), the extracted contents of every recognised archive (when extraction is enabled), and every directly supplied file that matches the extension filter. Exercised by `tests/match_combined_paths_tests.rs::test_match_command_with_individual_files_and_directories` and `tests/unified_path_handling_tests.rs::test_input_path_handler_merge`.

#### Scenario: Files from directories plus individual file paths
- **GIVEN** two directories `dir1/` (containing `video1.mp4`, `subtitle1.srt`) and `dir2/` (containing `video2.mkv`, `subtitle2.srt`), and an input list of `[video1.mp4, dir2, subtitle1.srt]`
- **WHEN** `get_input_handler().collect_files()` runs non-recursively with the video+subtitle extension filter
- **THEN** the returned list SHALL contain all four files: `video1.mp4`, `subtitle1.srt`, `video2.mkv`, and `subtitle2.srt`

#### Scenario: Files, directories, and archives mixed
- **GIVEN** inputs `[video1.mp4, dir2/, subs.7z]` where `dir2/` contains
  `video2.mkv` and `subtitle2.srt`, and `subs.7z` contains `extra.srt`
- **WHEN** `collect_files()` runs with video+subtitle extension filter
- **THEN** the returned list SHALL contain `video1.mp4`, `video2.mkv`,
  `subtitle2.srt`, and `extra.srt`

### Requirement: Directory Deduplication

`InputPathHandler::get_directories()` SHALL return a deduplicated set of directories that covers every supplied input (using each file's parent directory and each supplied directory itself), such that the same directory reached via multiple input paths SHALL appear exactly once in the returned list. Implemented in `src/core/input/mod.rs` using a `HashSet` and exercised by `tests/unified_path_handling_tests.rs::test_get_directories`.

#### Scenario: Same directory reached via two inputs
- **GIVEN** an input list containing a directory `dir1` and a file `dir1/file2.srt` whose parent is `dir1`
- **WHEN** `get_directories()` is called on the resulting handler
- **THEN** the returned list SHALL contain `dir1` exactly once

### Requirement: Invalid Path Surfacing

`collect_files()` SHALL return `SubXError::InvalidPath(<path>)` when an input entry exists in the handler but is neither a regular file nor a directory (for example a broken symlink or special filesystem object), so that the CLI caller can surface a clear error instead of silently producing an empty result.

#### Scenario: Neither file nor directory
- **GIVEN** an input path that exists for validation purposes but resolves to neither a regular file nor a directory at collection time
- **WHEN** `collect_files()` runs
- **THEN** the call SHALL return `Err(SubXError::InvalidPath(..))` referencing the offending path

### Requirement: CollectedFiles Return Type

`InputPathHandler::collect_files()` SHALL return a `CollectedFiles` struct
containing the collected `Vec<PathBuf>` and any `TempDir` handles created
during archive extraction. The struct SHALL implement `Deref<Target = Vec<PathBuf>>`
so that existing call sites treating the result as a `Vec<PathBuf>` continue
to work without modification. The `TempDir` handles SHALL be dropped (and
their directories cleaned up) when the `CollectedFiles` value goes out of
scope.

#### Scenario: CollectedFiles dereferences to Vec
- **WHEN** a caller uses `collected_files.len()` or iterates with `for p in &*collected_files`
- **THEN** the code SHALL compile and behave identically to operating on a `Vec<PathBuf>`

#### Scenario: Temp dirs survive until CollectedFiles is dropped
- **WHEN** `collect_files()` returns a `CollectedFiles` with extracted archive paths
- **THEN** the temp directories SHALL exist on disk while the `CollectedFiles` value is alive
- **AND** SHALL be deleted when the `CollectedFiles` value is dropped

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

### Requirement: Archive Origin Mapping

`CollectedFiles` SHALL maintain a mapping from each temp-directory root
path to the original archive file path. This mapping SHALL be queryable
via `CollectedFiles::archive_origin(temp_path) -> Option<&Path>`, enabling
commands to resolve output directories relative to the original archive
location rather than the temp directory.

#### Scenario: Temp path resolves to archive origin
- **WHEN** a file `/tmp/subx-XXXX/movie.srt` was extracted from `/data/subs.zip`
- **THEN** `collected_files.archive_origin(Path::new("/tmp/subx-XXXX/movie.srt"))`
  SHALL return `Some(Path::new("/data/subs.zip"))`

#### Scenario: Non-archive path returns None
- **WHEN** a file `/data/movie.srt` was supplied directly (not from an archive)
- **THEN** `collected_files.archive_origin(Path::new("/data/movie.srt"))`
  SHALL return `None`

### Requirement: Output Directory Resolution for Archive Files

For mutating commands (`convert`, `sync`, `translate`), when a source file originates from an archive extraction and no explicit output location is specified, the command SHALL resolve the output location through `CollectedFiles`' archive-aware queries rather than computing it inline. Specifically:

- `convert` SHALL obtain its per-file output path from `CollectedFiles::default_output_path(input, &format)` whenever `--output` is absent (`subx-cli/src/commands/convert_command.rs`).
- `translate` SHALL obtain its default output directory from `CollectedFiles::default_output_dir(input)` and join its own language-suffixed filename onto it (`subx-cli/src/commands/translate_command.rs`).
- `sync`'s batch paths SHALL keep their conditional form: they SHALL redirect the output beside the archive **only** when the subtitle has an `archive_origin`, and SHALL otherwise leave the output location unset so it is derived downstream by `create_default_output_path` (`subx-cli/src/commands/sync_command.rs`). Rewriting these two sites to compute a path unconditionally SHALL NOT be done, because the distinction between an unset and a set output location governs the command's overwrite handling.

The command layer SHALL retain the obligations that are its own and that no core query can discharge:

- An explicit `--output` / `-o` value SHALL take precedence over the archive-aware default.
- When `--output` names a directory and the run has more than one input file — from multiple inputs, from a directory input, or from archive expansion — the command SHALL append the per-file name to it.
- `translate --replace` SHALL be refused for a subtitle that has an `archive_origin`, because replacing a file inside a temporary extraction directory writes to a location that is about to be deleted.

This prevents output from being written into the temporary extraction directory, which is deleted on drop.

#### Scenario: Explicit -o overrides archive origin
- **GIVEN** the user runs `subx convert subs.zip -o /output/`
- **WHEN** conversion completes
- **THEN** the converted file SHALL be written to `/output/`

#### Scenario: Convert with no -o defers to the core query
- **GIVEN** the user runs `subx convert subs.zip` containing `movie.srt`
- **WHEN** the command resolves the output path for `movie.srt`
- **THEN** it SHALL do so by calling `CollectedFiles::default_output_path` and SHALL NOT compute the archive parent directory itself, and the converted file SHALL be written beside `subs.zip`

#### Scenario: Replace mode refuses an archive-extracted subtitle
- **GIVEN** the user runs `subx translate --replace subs.zip` containing `movie.srt`
- **WHEN** the command resolves the output location for `movie.srt`
- **THEN** it SHALL fail with a command-execution error stating that `--replace` cannot be used for subtitles extracted from archives, and SHALL NOT write into the extraction directory

#### Scenario: Sync leaves a non-archive output unset
- **GIVEN** a batch `sync` run in which the paired subtitle was supplied directly rather than extracted from an archive, and no `--output` was given
- **WHEN** the command prepares the single-pair arguments
- **THEN** the output location SHALL remain unset so it is derived by `create_default_output_path`, and the command SHALL NOT substitute a computed path

### Requirement: CollectedFiles Additional APIs

`CollectedFiles` SHALL implement `into_paths() -> Vec<PathBuf>` for call
sites that consume paths by value, and `AsRef<[PathBuf]>` for slice
access. Call sites such as `DetectEncodingArgs::get_file_paths()` that
currently return `Vec<PathBuf>` SHALL be updated accordingly.

#### Scenario: into_paths consumes CollectedFiles
- **WHEN** `collected_files.into_paths()` is called
- **THEN** a `Vec<PathBuf>` SHALL be returned and the `CollectedFiles`
  SHALL be consumed (temp dirs are dropped)

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

### Requirement: Archive-Aware Output Location Resolution

`CollectedFiles` SHALL own the rule that decides where an output file belongs when its input may have come out of an archive, so that no caller has to re-derive it. Two methods on `CollectedFiles` (`subx-core/src/core/input/mod.rs`), beside `archive_origin`:

```rust
pub fn default_output_dir<'a>(&'a self, input: &'a Path) -> &'a Path;
pub fn default_output_path(&self, input: &Path, extension: &str) -> PathBuf;
```

`default_output_dir` SHALL return, in order of preference: the parent directory of the archive `input` was extracted from, when `archive_origin(input)` is `Some` and that archive has a parent; otherwise `input`'s own parent; otherwise `Path::new(".")`.

`default_output_path` SHALL return:

- when `archive_origin(input)` is `Some(archive)`: `archive.parent()` (or `Path::new(".")` when the archive has no parent) joined with `<stem>.<extension>`, where `<stem>` is `input`'s file stem or the literal `output` when it has none;
- otherwise: `input.with_extension(extension)`.

The two methods SHALL NOT be defined in terms of one another, and `default_output_path` SHALL NOT be rewritten as `default_output_dir(input).join(..)`. Both are verbatim extractions of the loops their commands have always run, and those commands print the resolved path, so byte-compatibility with the current rendering is the contract; deriving one from the other breaks it. The load-bearing subtlety is that `Path::new("movie.srt").parent()` is `Some("")`, not `None`: `default_output_dir` returns the empty path for a single-component input, and joining onto the empty path keeps the name bare (`movie.zh.srt`), so the empty result MUST NOT be "tidied" to `Path::new(".")` — that rewrite renders `./movie.zh.srt`, a CLI-visible text change. The same `Some("")` semantics carry through the archive arms of both methods (a relative archive path resolves through the empty path, joining to a bare name); each method's `unwrap_or(Path::new("."))` fallback is the verbatim behaviour of the loop it was extracted from and is reachable only for a literally-empty path. Both rustdoc entries SHALL state the empty-path rule.

Neither method SHALL touch the filesystem, and neither SHALL create a directory.

The purpose is to prevent output from being written into a temporary extraction directory, which is deleted when the `CollectedFiles` is dropped.

#### Scenario: Archive-extracted output resolves beside the archive
- **GIVEN** a `CollectedFiles` in which `/tmp/subx-XXXX/movie.srt` has `archive_origin` `/data/subs.zip`
- **WHEN** `default_output_path(Path::new("/tmp/subx-XXXX/movie.srt"), "vtt")` is called
- **THEN** it SHALL return `/data/movie.vtt`, not a path under `/tmp/subx-XXXX/`

#### Scenario: Non-archive output resolves beside the input
- **GIVEN** a `CollectedFiles` in which `/data/movie.srt` was supplied directly
- **WHEN** `default_output_path(Path::new("/data/movie.srt"), "vtt")` is called
- **THEN** it SHALL return `/data/movie.vtt`

#### Scenario: Bare filename keeps its bare form
- **GIVEN** a `CollectedFiles` in which the relative path `movie.srt` was supplied directly
- **WHEN** `default_output_path(Path::new("movie.srt"), "vtt")` is called
- **THEN** it SHALL return exactly `movie.vtt` and SHALL NOT return `./movie.vtt`

#### Scenario: Directory query follows the archive
- **GIVEN** a `CollectedFiles` in which `/tmp/subx-XXXX/movie.srt` has `archive_origin` `/data/subs.zip`
- **WHEN** `default_output_dir(Path::new("/tmp/subx-XXXX/movie.srt"))` is called
- **THEN** it SHALL return `/data`

#### Scenario: Directory query falls back to the input's parent
- **GIVEN** a `CollectedFiles` in which `/data/movie.srt` was supplied directly
- **WHEN** `default_output_dir(Path::new("/data/movie.srt"))` is called
- **THEN** it SHALL return `/data`, and for a single-component relative input such as `movie.srt` it SHALL return the empty path `Path::new("")` (because `Path::parent` of a single-component path is `Some("")`, and joining onto the empty path keeps the joined name bare), and for an input with no parent component at all such as `/` it SHALL return `.`
