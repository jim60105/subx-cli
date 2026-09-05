## Why

`Cargo.toml` declares five crates that have **zero use sites** anywhere in `src/`, `tests/`, or `benches/`, and declares a sixth in the wrong table:

| Manifest entry | Location | Reality |
|---|---|---|
| `notify = "8.0"` | `Cargo.toml:95` | 0 hits. Nothing in the crate watches the filesystem. |
| `once_cell = "1.19"` | `Cargo.toml:141` | 0 hits. The process-global output mode uses `std::sync::OnceLock` (`src/cli/output.rs:78-79`). |
| `tokio-util = { version = "0.7", features = ["codec"] }` | `Cargo.toml:128` | 0 hits. Filed under a `# VAD` comment, but the VAD path never uses a codec. |
| `winapi = { version = "0.3", features = ["winuser"] }` | `Cargo.toml:195-196` | 0 hits. The only entry in `[target.'cfg(windows)'.dependencies]`. |
| `libc = "0.2"` | `Cargo.toml:198-199` | 0 hits. The only entry in `[target.'cfg(unix)'.dependencies]`. |
| `hound = "3.5"` | `Cargo.toml:129` | 0 `src/` hits; used by exactly 5 files under `tests/`. It belongs in `[dev-dependencies]`. |

Removing the five dead entries drops **seven packages** out of the resolved graph — `notify`, `notify-types`, `inotify`, `inotify-sys`, `fsevent-sys`, `kqueue`, `kqueue-sys` — every one of them a platform-specific FFI shim that the binary never calls. `cargo audit` currently resolves against all of them, so an advisory on any of those crates would fail CI over code that does not exist.

The manifest also carries two cosmetic defects: the `[features]` block sits **above** `[package]` (`Cargo.toml:1-5`) carrying a Chinese comment `# 測試用 feature flag`, which violates the project's English-only comment rule, and the comment `# Configuration management dependencies: path resolution and multi-core detection` is duplicated at `Cargo.toml:113` where it describes the audio-processing block that follows it. Finally, `src/cli/validation.rs` is a **0-byte orphan** that `src/cli/mod.rs:32-43` never declares — it is unreachable from the crate root yet still ships inside the published `.crate` archive.

This is series change **A0**. SDR §4 fixes the per-crate dependency allocation for the upcoming `subx-core` split; carrying five phantom dependencies into that split would duplicate dead weight into *both* manifests and force a second cleanup pass. Doing it first, in the existing single crate, makes the later manifest authoring a straight transcription of SDR §4.

## What Changes

- Delete five dependency entries with zero use sites from `Cargo.toml`: `notify` (`:95`), `tokio-util` (`:128`), `once_cell` (`:141`), `winapi` (`:196`), `libc` (`:199`). Entries are **deleted**, never commented out.
- Delete the now-empty `[target.'cfg(windows)'.dependencies]` and `[target.'cfg(unix)'.dependencies]` tables (`Cargo.toml:194-199`) together with the `# Cross-platform dependencies` heading, since removing their sole entries leaves them empty.
- Move `hound = "3.5"` out of `[dependencies]` (`:129`) into `[dev-dependencies]` (after `:151`). Its five consumers are all integration tests: `tests/vad_integration_tests.rs`, `tests/vad_performance_tests.rs`, `tests/vad_audio_processor_tests.rs`, `tests/sync_engine_integration_tests.rs`, `tests/sync_engine_performance_tests.rs`.
- Delete the 0-byte orphan `src/cli/validation.rs`. It is not declared in `src/cli/mod.rs` and nothing references it.
- Normalise the manifest layout: move the `[features]` block (`Cargo.toml:1-5`) to sit **below** `[package]` and its metadata, and rewrite `# 測試用 feature flag` in English. Remove the stale duplicated `# Configuration management dependencies: path resolution and multi-core detection` comment at `:113` (the real one is at `:136`), leaving `# Audio processing` to head the `symphonia` block.
- Regenerate `Cargo.lock` by running `cargo build` (and `cargo build --features archive-rar` so the optional `unrar` subtree is re-resolved). The lockfile is **never** hand-edited.
- Update the `supply-chain-hardening` capability so that a manifest entry with no use site is itself a spec violation, and so that the `cargo audit` gate is defined over the **resolved** dependency graph of actually-used crates rather than over whatever the manifest happens to list.

This change touches only `Cargo.toml`, `Cargo.lock`, one file deletion, and documentation. It is deliberately isolated so it can run in parallel with every other change in the series. No source code, no public API, and no CLI surface is affected.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `supply-chain-hardening`: Gains three requirements — every declared dependency SHALL have at least one use site (and dev-only crates SHALL live in `[dev-dependencies]`), the dependency manifest SHALL follow a fixed section order with English-only comments, and the published crate SHALL contain no source file unreachable from the crate root. The existing "CI cargo audit gate" requirement is restated so that the audited surface is the lockfile-resolved graph and so that `Cargo.lock` is machine-regenerated, never hand-edited.

## Impact

- **Code:** `Cargo.toml` (five dependency deletions, one table move, two empty target tables removed, `[features]` relocated, two comments fixed); `Cargo.lock` (regenerated — the seven-package `notify` subtree drops out of the 452-package graph); `src/cli/validation.rs` **deleted** (0 bytes, orphan). No `.rs` file content changes.
- **Tests:** No test source changes. The five `hound` consumers keep compiling unchanged because integration tests already resolve `[dev-dependencies]`. Benches (`benches/retry_performance.rs`, `benches/file_id_generation_bench.rs`) have zero `hound` hits and are unaffected.
- **APIs:** None. No public item is added, removed, or changed. `src/cli/validation.rs` was never reachable, so its deletion is invisible to consumers.
- **Dependencies:** Removed from `[dependencies]`: `notify` 8.0, `tokio-util` 0.7, `once_cell` 1.19. Removed from `[target.'cfg(windows)'.dependencies]`: `winapi` 0.3. Removed from `[target.'cfg(unix)'.dependencies]`: `libc` 0.2. Moved `[dependencies]` → `[dev-dependencies]`: `hound` 3.5. Note that `once_cell`, `tokio-util`, `winapi`, and `libc` remain in `Cargo.lock` as **transitive** dependencies of other crates (`rustls`/`tempfile`/`wiremock`, `reqwest`/`h2`, `socks`/`unrar_sys`, and 35 others respectively) — this change removes the *direct* declarations, not those subtrees. Only the `notify` subtree leaves the graph entirely.
- **Documentation:** `CHANGELOG.md` gains `[Unreleased]` → `### Removed` and `### Changed` entries. `docs/tech-architecture.md:509-591` mirrors the manifest in a hand-maintained TOML block that already lists all five dead crates (`:564`, `:568`, `:572`, `:575`, plus `hound` at `:534`) and is stale in other ways too (`tokio` shown with `"full"`, `symphonia` with `"all"`, plus `dialoguer` and `md5`, neither of which is in the manifest at all) — that block is re-synced against the real `Cargo.toml`. No user-facing guide changes.
