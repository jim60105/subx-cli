## ADDED Requirements

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

The two methods SHALL NOT be defined in terms of one another, and `default_output_path` SHALL NOT be rewritten as `default_output_dir(input).join(..)`. For a bare relative input with no parent — for example `movie.srt` — `with_extension` yields `movie.vtt` while `default_output_dir` yields `.` and a join would yield `./movie.vtt`. The two name the same file but render differently, and the rendered form appears in CLI output, so the distinction is load-bearing. Both rustdoc entries SHALL state this.

Neither method SHALL touch the filesystem, and neither SHALL create a directory.

The purpose is to prevent output from being written into a temporary extraction directory, which is deleted when the `CollectedFiles` is dropped.

**Migration**: this requirement takes the archive-resolution rule out of *Output Directory Resolution for Archive Files*, which retains the command-level obligations it is composed with. The `match` sentence and the *Match relocates archive subtitle beside video* scenario of that requirement move to `subtitle-matching`'s *AI-Driven Language and Globally-Unique Target Naming*, where the relocation is now a named core function; they are not restated here, so the property is stated exactly once.

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
- **THEN** it SHALL return `/data`, and for a parentless relative input it SHALL return `.`

## MODIFIED Requirements

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

**Migration**: the archive-resolution rule itself, and the *Convert output goes beside archive* scenario, move to this capability's *Archive-Aware Output Location Resolution*, which `subx-core` owns. The `match` sentence and the *Match relocates archive subtitle beside video* scenario move to `subtitle-matching`'s *AI-Driven Language and Globally-Unique Target Naming*, because after that change the relocation is `apply_archive_origin_relocation` in `subx-core` and only the call is `match_command`'s. This requirement is therefore no longer wholly CLI-side: C2b classified it **L** on the evidence of five inline command call sites, and that classification becomes a **split** — this half stays in `subx-cli`, the resolution half migrates.

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
