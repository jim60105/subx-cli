## Context

SubX-CLI v1.5.2 supports transparent archive extraction for ZIP and RAR when
users supply archive files via `-i`. The current architecture places all
extraction logic in a single `src/core/archive.rs` file: an `ArchiveFormat`
enum, an extension-based `detect_format()`, and per-format `extract_xxx()`
functions. As we add 7z and tar.gz support (bringing the total to 4 formats),
this single-file approach violates the Single Responsibility Principle. Each
format has distinct crate dependencies, error handling patterns, and entry-type
semantics; stuffing them all into one file makes the module harder to maintain,
test, and extend.

The 7-Zip format is extremely popular for subtitle distribution in Asian
communities. The `.tar.gz` / `.tgz` format is ubiquitous on Linux. Both are
absent today, forcing users to pre-extract manually.

## Goals / Non-Goals

**Goals:**

- G1: Support 7-Zip (`.7z`) archive extraction with the same security
  guarantees as ZIP/RAR (path-traversal, decompression-bomb, symlink rejection).
- G2: Support tar-gzip (`.tar.gz`, `.tgz`) archive extraction with the same
  security guarantees.
- G3: Keep new formats always-on (no optional feature flag) since the crates are
  pure Rust with no native dependencies.
- G4: Refactor `src/core/archive.rs` into a module directory
  (`src/core/archive/`) following SRP: one file per format, shared validation
  helpers, and a common extraction trait.

**Non-Goals:**

- NG1: Tar files without gzip compression (plain `.tar`) — uncommon for subtitle
  distribution, can be added later.
- NG2: Other compressed tar variants (`.tar.bz2`, `.tar.xz`, `.tar.zst`) — same
  rationale; can be added incrementally.
- NG3: Magic-byte / content-sniffing detection — the existing extension-only
  approach is sufficient and simpler.
- NG4: Extracting archives found during directory traversal — existing
  direct-input-only policy is preserved.

## Decisions

### D0: SRP Module Structure — Trait + Per-Format Modules

**Choice:** Refactor the monolithic `src/core/archive.rs` into a module
directory with clear separation of concerns:

```
src/core/archive/
├── mod.rs       — ArchiveFormat enum, detect_format(), extract_archive()
│                  dispatcher, re-exports, public API surface
├── common.rs    — Shared validation helpers: validate_entry_path(),
│                  ExtractionLimits (MAX_EXPANDED_SIZE, MAX_ENTRY_COUNT),
│                  size/count tracking struct
├── zip.rs       — extract_zip() using `zip` crate
├── rar.rs       — extract_rar() using `unrar` crate (feature-gated)
├── sevenz.rs    — extract_7z() using `sevenz-rust` crate
└── targz.rs     — extract_tar_gz() using `tar` + `flate2` crates
```

Each format module contains exactly one public extraction function with the
common signature:
```rust
pub(super) fn extract_xxx(archive_path: &Path, dest_dir: &Path) -> io::Result<Vec<PathBuf>>
```

The `common.rs` module provides shared security validation that all extractors
call:
- `validate_entry_path(dest_dir, entry_path) -> io::Result<PathBuf>` — rejects
  path traversal and absolute paths
- `ExtractionLimits` — tracks cumulative extracted size and entry count, returns
  error when limits exceeded

**Rationale:** With 4 formats, the single-file approach leads to a ~600+ line
file mixing unrelated crate imports, error types, and extraction logic. SRP
demands each format lives in its own module. The shared `common.rs` ensures
security invariants stay consistent and DRY across formats. The public API
(`detect_format`, `extract_archive`) remains unchanged — callers are unaffected.

**Alternatives considered:**
- Trait-based dispatch (`dyn ArchiveExtractor`) — over-engineering for 4 static
  formats with known-at-compile-time dispatch. Simple `match` is sufficient.
- Keep single file — violates SRP, makes format-specific changes risky due to
  merge conflicts and cognitive load.

### D1: 7z Crate Selection — `sevenz-rust`

**Choice:** Use the `sevenz-rust` crate for 7-Zip decompression.

**Rationale:** Pure Rust, no C/C++ dependency, supports LZMA/LZMA2/BCJ/Delta
filters covering virtually all 7z files in the wild. Actively maintained.
Unlike `unrar_sys` (C++ static linking), `sevenz-rust` cross-compiles to all
tier-1 targets without special toolchain setup.

**Alternatives considered:**
- `lzma-rs` + manual 7z container parsing — too much work, reinventing the
  wheel.
- Shelling out to `7z` CLI — runtime dependency, not cross-platform, violates
  the project's embedded-library philosophy.

### D2: tar.gz Crate Selection — `tar` + `flate2`

**Choice:** Use the `tar` crate (streaming tar reader) with `flate2` for gzip
decompression. Both are de facto standards in the Rust ecosystem. Extraction
SHALL use manual per-entry iteration (`entries()` + `unpack()` on individual
entries after type/path validation), NOT `Archive::unpack()` / `unpack_in()`,
which replays metadata and follows symlinks without validation.

**Rationale:** `flate2` is already a transitive dependency (via `zip` crate).
`tar` is the canonical Rust tar implementation. Manual iteration gives full
control over entry type filtering and size tracking.

**Alternatives considered:**
- `async-tar` — unnecessary complexity; extraction runs in a blocking thread
  already.

### D3: Multi-Extension Detection for `.tar.gz`

**Choice:** Extend `detect_format()` to check compound extensions. The
function will first check if the filename ends with `.tar.gz` (case-insensitive)
before falling through to the single-extension match. `.tgz` is handled by
normal single-extension matching.

**Rationale:** `.tar.gz` is a two-part extension that cannot be detected by
`Path::extension()` alone (which returns only `gz`). Checking the full filename
suffix is simple and unambiguous.

### D4: No Feature Flag for New Formats

**Choice:** All new crates are unconditional dependencies. No `archive-7z` or
`archive-targz` feature flag.

**Rationale:** Both `sevenz-rust` and `tar`/`flate2` are pure Rust. They
add no system-level build requirements, unlike `unrar_sys` which requires a C++
compiler. The binary size increase is marginal (~200 KB compressed). Keeping
them always-on simplifies CI and user builds.

### D5: Security Invariant Consistency

**Choice:** All new extractors enforce the same security limits:
- MAX_EXPANDED_SIZE = 1 GiB
- MAX_ENTRY_COUNT = 10,000
- Path-traversal rejection (resolved path must be under extraction root)
- Symlink rejection
- Empty archives return empty list (no error)

**Rationale:** The limits are already defined as constants in `archive.rs`.
Reusing them ensures consistent behavior regardless of archive format.

## Risks / Trade-offs

- **[Risk]** `sevenz-rust` may not handle all 7z compression methods (e.g.,
  PPMD, encrypted headers).
  → **Mitigation:** The error-handling requirement already mandates logging a
  warning and skipping unextractable archives. Rare methods will degrade
  gracefully.

- **[Risk]** Tar archives can contain device nodes, FIFOs, and hard links
  beyond just symlinks.
  → **Mitigation:** Use `tar::Entry::header().entry_type()` to reject all
  entry types except `Regular` and `Directory`. This mirrors the ZIP extractor's
  approach.

- **[Risk]** Binary size increase from three new crates.
  → **Mitigation:** `flate2` is already a transitive dependency (adds 0 KB).
  `tar` is ~40 KB. `sevenz-rust` is ~150 KB compressed. Total delta is modest.

- **[Risk]** `sevenz-rust` decoder may allocate large dictionary memory
  (LZMA2 dictionaries up to 1.5 GiB) before per-entry size checks run.
  → **Mitigation:** Validate the crate's API in a feasibility spike (task 1.0)
  to confirm that `decompress_with_extract_fn` callback allows per-entry
  size tracking. If dictionary allocation cannot be bounded, consider gating
  7z behind an optional feature flag.

- **[Trade-off]** Not supporting plain `.tar` leaves a small gap.
  → **Accepted:** The architecture makes adding `.tar` trivial later (one enum
  variant + one function). `.tar` without compression is genuinely rare for
  subtitle distribution.
