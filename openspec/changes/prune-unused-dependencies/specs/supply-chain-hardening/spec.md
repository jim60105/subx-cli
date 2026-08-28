## ADDED Requirements

### Requirement: Every Declared Dependency Has a Use Site

Every entry in `Cargo.toml`'s `[dependencies]`, `[dev-dependencies]`, and `[target.'cfg(…)'.dependencies]` tables SHALL have at least one use site in the crate's source trees. A use site is any occurrence, in a file under `src/`, `tests/`, or `benches/`, of `use <crate>`, `extern crate <crate>`, or a path-qualified `<crate>::` reference — counted across **all** `cfg` branches, not only the branches that compile on the reviewing platform.

- A manifest entry with zero use sites in all three trees SHALL be deleted. Carrying such an entry is itself a violation of this requirement, independent of whether the crate has an advisory against it.
- A dependency whose only use sites are under `tests/` or `benches/` SHALL be declared in `[dev-dependencies]`, not in `[dependencies]`.
- A dependency SHALL be removed by deletion. It SHALL NOT be retained as a commented-out line, and it SHALL NOT be relocated behind an off-by-default feature in order to keep it in the manifest.
- When removing the sole entry of a `[target.'cfg(…)'.dependencies]` table leaves that table empty, the empty table SHALL be removed as well.
- This requirement applies per manifest. When the project is split across more than one crate, each crate's manifest SHALL satisfy it against its own source trees.

At the time of writing, the following entries had zero use sites and SHALL NOT reappear without a use site accompanying them: `notify`, `once_cell` (superseded by `std::sync::OnceLock`, used at `src/cli/output.rs:78-79`), `tokio-util`, `winapi`, `libc`.

#### Scenario: Dependency with no use site is rejected

- **GIVEN** `Cargo.toml` declares a crate under `[dependencies]`
- **WHEN** the source trees `src/`, `tests/`, and `benches/` contain no `use`, `extern crate`, or `<crate>::` reference to it in any `cfg` branch
- **THEN** the manifest SHALL be treated as non-conforming and the entry SHALL be deleted

#### Scenario: Test-only dependency belongs in dev-dependencies

- **GIVEN** the `hound` crate is referenced by `tests/vad_integration_tests.rs`, `tests/vad_performance_tests.rs`, `tests/vad_audio_processor_tests.rs`, `tests/sync_engine_integration_tests.rs`, and `tests/sync_engine_performance_tests.rs`, and by no file under `src/`
- **WHEN** `Cargo.toml` is inspected
- **THEN** `hound` SHALL be declared under `[dev-dependencies]` and SHALL NOT appear under `[dependencies]`

#### Scenario: Platform-gated dependency is checked across all cfg branches

- **GIVEN** a crate is declared under `[target.'cfg(windows)'.dependencies]` or `[target.'cfg(unix)'.dependencies]`
- **WHEN** its use sites are counted
- **THEN** the count SHALL be taken over the source text of all files regardless of the host platform, so that a `#[cfg(windows)]`-only use site is found on a Unix host and vice versa

#### Scenario: Emptied target table is removed

- **GIVEN** `[target.'cfg(unix)'.dependencies]` contains exactly one entry and that entry has no use site
- **WHEN** the entry is deleted
- **THEN** the now-empty `[target.'cfg(unix)'.dependencies]` table SHALL also be deleted rather than left as an empty header

#### Scenario: Removal is deletion, not commenting out

- **GIVEN** a dependency is being removed for having no use site
- **WHEN** the manifest edit is made
- **THEN** the entry SHALL be absent from the file, and SHALL NOT be preserved as a `#`-prefixed comment line

### Requirement: Dependency Manifest Layout and Comment Language

`Cargo.toml` SHALL be readable top-to-bottom as a description of the crate, so that a dependency review can be performed against the file alone.

- The `[package]` table SHALL be the first table in the manifest, and `[package.metadata.*]` sub-tables SHALL immediately follow it without unrelated tables interleaved.
- The `[features]` table SHALL appear below the `[package]` group, never above it.
- Every comment in `Cargo.toml` SHALL be written in English, consistent with the project-wide rule that all code comments, rustdoc, and commit messages are English.
- A section comment SHALL describe the entries that follow it. A comment left behind after its entries are removed, or duplicated above an unrelated block, SHALL be deleted.

#### Scenario: Features table sits below the package table

- **WHEN** `Cargo.toml` is inspected
- **THEN** `[package]` SHALL be the first table, `[package.metadata.docs.rs]` SHALL follow it directly, and `[features]` SHALL appear after that group rather than at the top of the file

#### Scenario: Manifest comments are English

- **WHEN** `Cargo.toml` is inspected
- **THEN** no comment SHALL contain non-English prose — in particular the previous `# 測試用 feature flag` SHALL have been replaced with an English description of `slow-tests` and `archive-rar`

#### Scenario: Orphaned section comment is removed with its entries

- **GIVEN** `# File change monitoring` heads only the `notify` entry and `# Once Cell for runtime initialization` heads only the `once_cell` entry
- **WHEN** those entries are deleted
- **THEN** their section comments SHALL be deleted with them, leaving no comment describing an absent dependency

### Requirement: Published Crate Contains No Unreachable Source Files

Every file under `src/` SHALL be reachable from the crate root through the module tree — that is, declared via `mod`/`pub mod` (directly or transitively from `src/lib.rs` or `src/main.rs`) or referenced by an explicit `#[path]` attribute.

`Cargo.toml`'s `exclude` list filters `tests/`, `benches/`, `assets/`, and media files out of the published `.crate` archive, but does not filter anything under `src/`. An undeclared file under `src/` therefore ships to crates.io in every release while contributing nothing to the build, and SHALL be deleted.

#### Scenario: Undeclared source file is deleted

- **GIVEN** `src/cli/validation.rs` exists, is 0 bytes, and is not declared in `src/cli/mod.rs`
- **WHEN** the crate's module tree is walked from `src/lib.rs` and `src/main.rs`
- **THEN** the file SHALL be found unreachable and SHALL be deleted from the repository

#### Scenario: Published archive contains only reachable sources

- **WHEN** `cargo package --list` is inspected
- **THEN** every listed path under `src/` SHALL correspond to a module reachable from the crate root

## MODIFIED Requirements

### Requirement: CI cargo audit gate

The project's existing `cargo audit` CI step SHALL be verified to fail the build pipeline on any direct dependency with a known vulnerability advisory. If the current configuration allows advisory-only warnings without failing, it SHALL be tightened to enforce failure.

The surface that `cargo audit` examines SHALL be the dependency graph resolved from `Cargo.lock`, and that graph SHALL contain only packages reachable from a manifest entry that satisfies the "Every Declared Dependency Has a Use Site" requirement. A package that enters the resolved graph solely through a declaration with no use site SHALL be treated as an audit-surface defect and removed by deleting the declaration, so that every audit failure is attributable to code the crate actually builds against.

`Cargo.lock` SHALL be regenerated by Cargo — by running a build or `cargo update` — and SHALL NEVER be hand-edited. After a manifest change, the regenerated lockfile SHALL be reviewed against the expected package delta before being committed.

Removing a direct declaration does not imply the package leaves the lockfile: a package retained as a transitive dependency of another crate legitimately remains in the resolved graph and remains in scope for the audit.

#### Scenario: vulnerable dependency fails CI

- **WHEN** a direct dependency has a RUSTSEC advisory
- **THEN** the CI pipeline fails

#### Scenario: clean dependencies pass CI

- **WHEN** no direct dependencies have advisories
- **THEN** the CI pipeline succeeds

#### Scenario: Unused declaration is not allowed to widen the audit surface

- **GIVEN** `notify` is declared in `Cargo.toml` with no use site, pulling `notify-types`, `inotify`, `inotify-sys`, `fsevent-sys`, `kqueue`, and `kqueue-sys` into the resolved graph
- **WHEN** the audit surface is reviewed
- **THEN** the declaration SHALL be deleted so that those seven packages leave the graph, rather than an advisory against any of them being suppressed or ignored

#### Scenario: Transitively-required package legitimately stays in the graph

- **GIVEN** the direct `once_cell`, `tokio-util`, `winapi`, and `libc` declarations are deleted for having no use site
- **WHEN** `Cargo.lock` is regenerated
- **THEN** those packages MAY still appear in the lockfile as transitive dependencies of crates that require them (for example `rustls` and `tempfile` for `once_cell`, `reqwest` and `h2` for `tokio-util`, `unrar_sys` for `winapi`, and `tokio` for `libc`), and their continued presence SHALL NOT be treated as a violation

#### Scenario: Lockfile is regenerated, never hand-edited

- **GIVEN** a dependency has been added, removed, or moved between manifest tables
- **WHEN** `Cargo.lock` is updated
- **THEN** the update SHALL be produced by running Cargo (for example `cargo build`, including once with `--features archive-rar` so the optional `unrar` subtree is re-resolved), and the resulting diff SHALL be reviewed rather than authored by hand
