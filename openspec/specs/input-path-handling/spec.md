# Input Path Handling

## Purpose

Hold the CLI half of the input-path agreement: archive-aware output-directory resolution as consumed from command call sites, and the rule that the clap input argument structs are thin adapters over core collection with `crate::cli` legacy re-exports. Implemented in `src/cli/` argument structs and their command call sites. `InputPathHandler` itself — path merging, extension filtering, traversal modes, collection APIs, the no-extract switch, archive-origin mapping, and core-owned output-location rules — is specified by the same-named `input-path-handling` capability in `subx-core`.

## Requirements

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