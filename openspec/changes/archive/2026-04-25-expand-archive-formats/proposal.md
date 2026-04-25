## Why

SubX-CLI currently supports transparent archive extraction only for ZIP and RAR
files. Users working with subtitle collections distributed as `.7z` or
`.tar.gz`/`.tgz` archives must manually extract them before processing, breaking
the seamless `-i archive.7z` workflow. 7-Zip is widely used in Asian subtitle
communities for its superior compression ratio, while `.tar.gz` is the de-facto
standard on Linux and in open-source subtitle repositories. Adding these formats
closes a significant usability gap and brings SubX-CLI to feature-parity with
users' real-world archive workflows.

## What Changes

- **Add 7-Zip (.7z) archive extraction**: Extend `ArchiveFormat` and
  `extract_archive` to handle `.7z` files via a pure-Rust crate, applying the
  same security invariants (path-traversal rejection, decompression-bomb
  protection, symlink rejection) as ZIP/RAR.
- **Add tar.gz/tgz archive extraction**: Extend `ArchiveFormat` and
  `extract_archive` to handle `.tar.gz` and `.tgz` files via the `tar` +
  `flate2` crates, applying the same security invariants.
- **Refactor `archive.rs` to module directory (SRP)**: Split the monolithic
  `src/core/archive.rs` into `src/core/archive/` with per-format modules
  (`zip.rs`, `rar.rs`, `sevenz.rs`, `targz.rs`), shared validation helpers
  (`common.rs`), and a coordinating `mod.rs`. Each format owns its extraction
  logic; shared security invariants are DRY in `common.rs`.
- **Update format detection**: Add `.7z`, `.tar.gz`, and `.tgz` to the
  extension-based `detect_format()` dispatcher.
- **Update `-i` input handling**: `InputPathHandler::collect_files()` already
  dispatches through `detect_format()`; new formats are automatically picked up
  once detection and extraction functions exist.
- **Update `--no-extract` semantics**: The flag already gates all archive
  extraction generically; no behavioral change needed, but documentation and
  specs must reference the new extensions.
- **No feature-flag gating**: Unlike RAR (which depends on C++ via `unrar_sys`),
  the 7z, tar, and flate2 crates are pure Rust with no system dependencies, so
  they can be always-on like ZIP—no optional feature flag required.

## Capabilities

### New Capabilities

_(none — the archive-extraction capability already exists; this change modifies it)_

### Modified Capabilities

- `archive-extraction`: Add 7z, tar.gz, and tgz format support to format
  detection, extraction, and security invariants. Update the explicit scenario
  that previously stated `.tar.gz` SHALL NOT be extracted.
- `input-path-handling`: Extend the recognized archive extension list from
  `.zip`/`.rar` to also include `.7z`, `.tar.gz`, `.tgz`.
- `media-discovery`: Extend the archive-exclusion list so `.7z`, `.tar.gz`,
  `.tgz` are not classified as media files.

## Impact

- **Code**: `src/core/archive.rs` (new extractors + format variants),
  `src/cli/input_handler.rs` (no code change expected — already generic),
  unit tests, integration tests.
- **Dependencies**: Add `sevenz-rust` (pure-Rust 7z decompression), `tar`
  (streaming tar reader), `flate2` (gzip decompression) to `Cargo.toml` as
  always-on dependencies.
- **CI/CD**: No new feature flags; existing test matrix covers all new code.
  `release.yml` unchanged.
- **Breaking changes**: None. Existing behavior is strictly additive.
