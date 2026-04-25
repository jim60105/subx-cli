## Why

SubX-CLI accepts directories and individual files through `-i`, but subtitle
collections frequently arrive as `.zip` or `.rar` archives downloaded from
subtitle sites. Users must manually extract
these archives before running any SubX command. This adds friction,
especially for batch workflows where dozens of archives need processing.
Transparent archive extraction eliminates that manual step and makes
SubX a true one-command pipeline from download to organized library.

## What Changes

- Detect archive files (`.zip`, `.rar`) among directly-specified `-i`
  file inputs in `InputPathHandler` (archives found via directory
  traversal are NOT extracted)
- Extract archive contents to a temporary directory, preserving internal
  folder structure
- Feed the extracted paths into the existing file discovery pipeline so
  all downstream commands (`match`, `convert`, `sync`, `detect-encoding`)
  process them identically to regular files
- Clean up temporary extraction directories when the command completes
- Add `--no-extract` flag to disable archive extraction for users who
  intentionally pass archive files
- Archives found inside extracted archives are NOT extracted (no nested
  archive support)
- Add `zip` and `unrar` crate dependencies for extraction (`unrar`
  behind `archive-rar` feature flag, which is enabled in release builds
  since `unrar_sys` statically compiles the UnRAR C++ source — no
  runtime dependency required)
- Route command output to the archive's parent directory (not the temp
  extraction dir) for mutating commands (`match`, `convert`, `sync`)
- Enable `--features archive-rar` in the release CI pipeline so
  published binaries include RAR support out of the box

## Capabilities

### New Capabilities
- `archive-extraction`: Transparent extraction of `.zip` and `.rar`
  archive files directly supplied via `-i`, including temp-directory
  lifecycle management and format detection

### Modified Capabilities
- `input-path-handling`: `InputPathHandler::collect_files` gains archive
  awareness — archive files are expanded rather than filtered out or
  treated as unknown file types
- `media-discovery`: `FileDiscovery` extension lists need `.zip` and
  `.rar` in no list (archives are intercepted before discovery), but
  discovery must accept paths rooted in temp directories

## Impact

- **Code**: `src/cli/input_handler.rs` (primary change site),
  new `src/core/archive.rs` module for extraction logic
- **Dependencies**: add `zip` (for `.zip`), `unrar` (for `.rar`, behind
  `archive-rar` feature flag — enabled in release builds; `unrar_sys`
  statically links the UnRAR C++ source so no runtime dependency),
  promote `tempfile` from `[dev-dependencies]` to `[dependencies]`
- **CLI**: new `--no-extract` flag on `match`, `convert`, `sync`,
  `detect-encoding` args
- **Performance**: extraction adds I/O overhead proportional to archive
  size; a 500 MB archive will need equal free disk space for the
  temp copy
- **Testing**: new unit and integration tests for both formats plus
  edge cases (password-protected, corrupted, empty archives)
