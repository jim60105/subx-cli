## Context

SubX-CLI processes subtitle and video files supplied via `-i` flags or
positional arguments. The `InputPathHandler` (in `src/cli/input_handler.rs`)
resolves paths, validates existence, and expands directories into a flat
`Vec<PathBuf>`. Downstream, `FileDiscovery` classifies each path by
extension into `MediaFile` entries. No component is aware of archive
formats today — a `.zip` file is either silently skipped (does not match
any known extension) or produces an error.

Subtitle download sites commonly distribute files as `.zip` or `.rar`
archives, often containing multiple subtitle files organised in folders
(per-language, per-release-group). Users currently extract manually before
running SubX.

## Goals / Non-Goals

**Goals:**

- G1: Transparently extract `.zip` and `.rar` archives directly supplied
  as `-i` inputs so downstream commands process the contained files
  (archives discovered via directory traversal are NOT extracted)
- G2: Preserve archive-internal directory structure during extraction (many
  archives organise subtitles by language or release group)
- G3: Automatically clean up temporary extraction directories when the
  command finishes (whether success or error)
- G4: Work identically across `match`, `convert`, `sync`, and
  `detect-encoding` commands with zero command-specific archive logic
- G5: Provide `--no-extract` escape hatch for edge cases

**Non-Goals:**

- N1: Support for `.tar`, `.tar.gz`, `.7z`, or other archive formats
  (can be added later; `.zip` and `.rar` cover >95% of subtitle archives)
- N2: Streaming extraction — archives are fully extracted before processing
- N3: Archive creation or re-packing
- N4: Decryption of password-protected archives (warn and skip)
- N5: Nested archive extraction (archives inside archives are not extracted)
- N6: Extracting archives discovered via directory traversal (only directly
  specified archive files are extracted)

## Decisions

### D1: Extraction hook placement — inside `InputPathHandler::collect_files`

**Decision:** Intercept archive files in `collect_files()` before the
extension-whitelist filter runs. Only files that were directly specified
as `-i` inputs (not discovered via directory traversal) are checked. When
such a path has a recognised archive extension (`.zip`, `.rar`), extract
it to a `TempDir` and replace the single archive path with the extracted
file paths.

**Alternatives considered:**
- Hook in each command's `get_input_handler()`: duplicates logic across 4
  commands and the sync batch path.
- Hook in `FileDiscovery::scan_file_list`: too late — `collect_files` would
  have already filtered out the archive.

**Rationale:** `collect_files` is the single chokepoint for every command.
One change, total coverage.

### D2: Temp directory lifetime — `Arc<TempDir>` returned alongside paths

**Decision:** `collect_files` returns a new struct
`CollectedFiles { paths: Vec<PathBuf>, _temp_dirs: Vec<TempDir> }` (or
wraps them behind `Arc`). The temp directories live as long as the caller
holds the struct. When the struct is dropped, cleanup happens automatically.

**Alternatives considered:**
- Global cleanup via `atexit` / signal handler: brittle, hard to test.
- Letting the caller manage `TempDir` separately: breaks encapsulation;
  every command would need temp-dir awareness.

**Rationale:** RAII gives deterministic cleanup. Returning `CollectedFiles`
instead of bare `Vec<PathBuf>` keeps the API clean and composable.

### D3: Archive format detection — by file extension, not magic bytes

**Decision:** Recognise archives by extension (`.zip`, `.rar`) rather than
reading file headers.

**Alternatives considered:**
- Magic-byte sniffing: more robust for misnamed files, but adds I/O and
  complexity. Subtitle archives are almost never misnamed.

**Rationale:** Extension matching is consistent with how SubX already
classifies video and subtitle files (extension lists in `FileDiscovery`
and `InputPathHandler`).

### D4: Extraction crate — `zip` for `.zip`, `unrar` for `.rar`

**Decision:** Use the `zip` crate (pure Rust, MIT, widely used) for zip
archives. For `.rar`, use the `unrar` crate which wraps the UnRAR C++
library via `unrar_sys`.

**Alternatives considered:**
- `compress-tools` (wraps libarchive): handles many formats, but heavy C
  dependency and less granular error handling.
- `sevenz-rust`: only handles 7z.

**Rationale:** `zip` is pure Rust, zero native deps, and covers the
dominant format. `unrar` adds a build-time C++ dependency via `unrar_sys`,
which statically compiles the UnRAR source into `libunrar.a` — no runtime
library is required and the final binary is fully self-contained. The
`archive-rar` feature is **enabled in release builds** so published
binaries include RAR support out of the box. The feature remains opt-in
for development builds to avoid requiring a C++ toolchain.

### D5: No nested archive extraction

**Decision:** Archives found inside extracted archives are NOT extracted.
They are treated as regular files and subject to the normal extension
filter (which means they will be silently skipped since `.zip`/`.rar` are
not media extensions).

**Rationale:** Simplicity. Nested archive extraction adds complexity
(isolated subdirectories, decompression bomb amplification, filename
collisions) for a rare use case. Users with nested archives can extract
the outer archive manually.

### D6: Direct-input-only extraction policy

**Decision:** Only archive files directly specified via `-i` (or
positional arguments) are eligible for extraction. Archives discovered
during directory traversal (when `-i` points to a directory) are NOT
extracted — they are treated as regular files and filtered by extension.

**Alternatives considered:**
- Extract all archives found during traversal: would surprise users who
  store archives alongside their media files. Also risks extracting
  large archives the user did not intend to process.

**Rationale:** Explicit is better than implicit. Users who want to
process an archive must name it directly.

### D7: `--no-extract` flag — shared across all commands

**Decision:** Add `--no-extract` (bool, default false) to a shared
`InputOptions` mixin or directly to each command's args struct. When set,
`collect_files` treats archive files as opaque — they pass through the
extension filter (and are likely skipped since `.zip` is not a media
extension).

**Rationale:** Allows users who genuinely have non-archive `.zip` files
(unlikely but possible) or who want to skip extraction overhead.

### D8: Security — path traversal and non-regular entry prevention

**Decision:** When extracting archive entries: (a) reject any entry whose
resolved path escapes the extraction root directory (zip-slip prevention),
and (b) reject any entry that is a symlink, hardlink, or other non-regular
file type. Both cases SHALL log a warning and skip the entry, continuing
extraction for remaining entries.

**Rationale:** Standard security practice for archive extraction. The `zip`
crate does not prevent path traversal by default. Symlinks and hardlinks
in archives can be used to escape the extraction sandbox, consistent with
the existing codebase policy of skipping symlinks
(`src/cli/input_handler.rs:306-337`, `file-operation-safety` spec).

### D9: Error handling for corrupted or password-protected archives

**Decision:** If extraction fails (corrupted data, password required,
unsupported compression method), log the error, print a warning naming the
archive, and skip it. Do not abort the entire command — other inputs may
still be valid.

**Rationale:** Batch workflows should not fail entirely because one of
many input archives is unreadable.

### D10: Output policy for archive-extracted files

**Decision:** When files originate from an archive extraction (i.e. they
reside in a temp directory), mutating commands MUST NOT write output back
into the temp tree. Instead:

- **`match` command**: When subtitle files come from an archive, the
  relocation mode MUST be `Copy` or `Move` with an explicit target
  directory. If the user has not specified a target directory and the
  relocation mode would place files inside the temp dir, the command SHALL
  automatically resolve the output to the directory containing the original
  archive file (the archive's parent directory).
- **`convert` command**: Output files SHALL be written to the archive's
  parent directory (or to `-o` if specified), not next to the temp-dir
  source.
- **`sync` command**: Output files SHALL be written to the archive's parent
  directory (or to `-o` if specified).
- **`detect-encoding` command**: Read-only — no output policy needed.

`CollectedFiles` SHALL track the mapping from temp-dir root to original
archive path so that commands can resolve the correct output directory.

**Alternatives considered:**
- Require explicit `-o` for all archive inputs: too restrictive for common
  use cases.
- Create a sibling directory `<archive-stem>/` next to the archive: adds
  filesystem side effects the user did not request.

**Rationale:** Placing output beside the original archive matches user
expectations — the archive acts as if it were a directory at that location.

### D11: `CollectedFiles` must provide `into_paths()` and `AsRef<[PathBuf]>`

**Decision:** In addition to `Deref<Target = Vec<PathBuf>>`,
`CollectedFiles` SHALL implement `into_paths() -> Vec<PathBuf>` for call
sites that consume the paths by value, and `AsRef<[PathBuf]>` for slice
access. Call sites like `DetectEncodingArgs::get_file_paths()` that
currently return `Vec<PathBuf>` SHALL be updated to return `CollectedFiles`
or call `into_paths()`.

**Rationale:** Some existing call sites consume `Vec<PathBuf>` by value.
`Deref` alone is insufficient for those patterns.

## Risks / Trade-offs

- **[Disk space]** Extraction doubles disk usage for the duration of the
  command → Mitigation: document the requirement; temp dirs are cleaned up
  on drop.
- **[Native dependency for RAR]** `unrar` requires `libunrar` at build
  time → Mitigation: make RAR support a cargo feature flag
  (`archive-rar`), **disabled by default**. CI and release builds
  do not need modification. Users opt in explicitly.
- **[Breaking API change]** `collect_files` return type changes from
  `Vec<PathBuf>` to `CollectedFiles` → Mitigation: implement `Deref`,
  `AsRef<[PathBuf]>`, and `into_paths()`. Update all call sites
  (approximately 6-8 locations).
- **[Zip-bomb / decompression bomb]** A small archive that expands to
  gigabytes → Mitigation: enforce a configurable maximum expanded size
  (default 1 GiB) and entry count limit (default 10,000 files); abort
  with error if exceeded.
