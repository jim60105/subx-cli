## 1. Verify the Use-Site Evidence

- [ ] 1.1 Run `grep -rnE '(^|[^a-zA-Z_])(notify|once_cell|tokio_util|winapi|libc)::|use (notify|once_cell|tokio_util|winapi|libc)|extern crate (notify|once_cell|tokio_util|winapi|libc)' src tests benches` and confirm the hit count is **0** for each of the five crates before editing anything; if any crate returns a hit, stop and revise the proposal rather than deleting the entry
- [ ] 1.2 Run the same grep for `hound` and confirm it returns **0 hits under `src/`** and **12 hits across 5 files under `tests/`** (`tests/vad_integration_tests.rs:85,89,92,130,134,137`, `tests/vad_performance_tests.rs:40,44,47`, `tests/vad_audio_processor_tests.rs:1`, `tests/sync_engine_integration_tests.rs:1`, `tests/sync_engine_performance_tests.rs:1`) — this is the positive control proving the grep pattern works
- [ ] 1.3 Confirm `std::sync::OnceLock` is what `src/cli/output.rs:78-79` (`static ACTIVE_MODE`, `static QUIET`) actually uses, so that removing `once_cell` is provably not removing the mechanism behind those globals
- [ ] 1.4 Confirm `src/cli/validation.rs` is 0 bytes (`wc -c src/cli/validation.rs`) and that `validation` appears in no `mod` declaration in `src/cli/mod.rs:32-43`
- [ ] 1.5 Record the current `Cargo.lock` package count (`grep -c '^\[\[package\]\]' Cargo.lock`, expected **452**) as the baseline for the task 5.4 diff review

## 2. Prune the Dependency Tables

- [ ] 2.1 Delete `notify = "8.0"` and its section comment `# File change monitoring` (`Cargo.toml:94-95`)
- [ ] 2.2 Delete `tokio-util = { version = "0.7", features = ["codec"] }` (`Cargo.toml:128`), leaving the `# VAD` comment at `:127` to head the surviving `voice_activity_detector`/`rubato`/`audioadapter-buffers` entries
- [ ] 2.3 Delete `once_cell = "1.19"` and its section comment `# Once Cell for runtime initialization` (`Cargo.toml:140-141`)
- [ ] 2.4 Delete `winapi = { version = "0.3", features = ["winuser"] }` together with its now-empty `[target.'cfg(windows)'.dependencies]` table and the `# Cross-platform dependencies` heading (`Cargo.toml:194-196`)
- [ ] 2.5 Delete `libc = "0.2"` together with its now-empty `[target.'cfg(unix)'.dependencies]` table (`Cargo.toml:198-199`)
- [ ] 2.6 Move `hound = "3.5"` from `[dependencies]` (`Cargo.toml:129`) into `[dev-dependencies]` (`Cargo.toml:151-168`), placing it under a `# WAV fixture generation for VAD and sync integration tests` comment; keep the version requirement `"3.5"` unchanged
- [ ] 2.7 Confirm no `#`-commented carcass of any removed entry remains anywhere in `Cargo.toml`

## 3. Normalise the Manifest Layout

- [ ] 3.1 Cut the `[features]` block (`Cargo.toml:1-5`: `default`, `slow-tests`, `archive-rar = ["dep:unrar"]`) from the top of the file and reinsert it immediately after the `[package.metadata.docs.rs]` table (`Cargo.toml:37-39`), so `[package]` becomes the manifest's first table
- [ ] 3.2 Replace the Chinese comment `# 測試用 feature flag` with an English one describing both flags, e.g. `# Test-only feature flags: 'slow-tests' gates long-running tests; 'archive-rar' enables optional RAR extraction.`
- [ ] 3.3 Delete the stale duplicated comment `# Configuration management dependencies: path resolution and multi-core detection` at `Cargo.toml:113`, leaving `# Audio processing` (`:114`) to head the `symphonia` block; the correct instance at `:136` above `dirs`/`num_cpus` stays
- [ ] 3.4 Confirm the resulting table order reads `[package]` → `[package.metadata.docs.rs]` → `[features]` → `[lints.rustdoc]` → `[lints.clippy]` → `[lints.rust]` → `[dependencies]` → `[dev-dependencies]` → `[[bin]]` → `[[bench]]` → `[profile.release]` → `[profile.dev]`, with no `[target.'cfg(…)'.dependencies]` tables left

## 4. Delete the Orphan Module

- [ ] 4.1 `git rm src/cli/validation.rs` (0-byte file, undeclared in `src/cli/mod.rs`)
- [ ] 4.2 Re-grep `src/` for `cli::validation` and `mod validation` to confirm zero references, and confirm the four remaining `validation` hits in `src/cli/` (`config_args.rs:114`, `config_args.rs:139`, `cache_args.rs:83`, `translate_args.rs:1`) are all English prose in doc comments, unrelated to the deleted file

## 5. Regenerate and Verify

- [ ] 5.1 Run `cargo build` to regenerate `Cargo.lock`; never hand-edit the lockfile
- [ ] 5.2 Run `cargo build --features archive-rar` so the optional `unrar`/`unrar_sys` subtree (the other `winapi` consumer) is re-resolved
- [ ] 5.3 Run `cargo bench --no-run` to confirm `benches/retry_performance.rs` and `benches/file_id_generation_bench.rs` still build after the `hound` table move
- [ ] 5.4 Review `git diff Cargo.lock` against the expectation in `design.md` Decision 5: exactly seven `[[package]]` entries removed — `notify`, `notify-types`, `inotify`, `inotify-sys`, `fsevent-sys`, `kqueue`, `kqueue-sys` — plus the edits to the `subx-cli` package's own `dependencies` list; `once_cell`, `tokio-util`, `winapi`, and `libc` MUST still be present as transitive entries
- [ ] 5.5 Run `cargo package --list --allow-dirty | grep '^src/'` and confirm `src/cli/validation.rs` no longer appears in the publish manifest
- [ ] 5.6 If the `x86_64-pc-windows-msvc` target is installed locally, run `cargo check --target x86_64-pc-windows-msvc` to prove no `#[cfg(windows)]` code path needed `winapi`; otherwise note that the CI coverage job's `windows-latest` leg is the verification

## 6. Documentation

- [ ] 6.1 Re-sync the `### Runtime Dependencies` TOML block in `docs/tech-architecture.md:511-576` against the real manifest: remove `notify` (`:564`), `once_cell` (`:568`), `winapi` + its target table (`:571-572`), `libc` + its target table (`:574-575`), and `hound` (`:534`); also correct the entries that were already stale — `tokio` is shown with `features = ["full"]` but is narrowed to `rt-multi-thread`/`macros`/`time`/`sync`/`fs`, `symphonia` is shown with `features = ["all"]` but lists nine explicit codecs, `rubato` is shown as `0.16.2` but is `2.0`, and `dialoguer` and `md5` are listed but are not in `Cargo.toml` at all
- [ ] 6.2 Add `hound = "3.5"` to the `### Dev Dependencies` block in `docs/tech-architecture.md:578-590`, and add the missing `regex` and `pretty_assertions` entries while the block is open
- [ ] 6.3 Add a `### Removed` entry under `[Unreleased]` in `CHANGELOG.md:9` listing the five pruned dependencies (`notify`, `once_cell`, `tokio-util`, `winapi`, `libc`) and noting that the `notify` subtree — seven packages — leaves the resolved dependency graph, shrinking the `cargo audit` surface
- [ ] 6.4 Add a `### Changed` entry under `[Unreleased]` covering the `hound` move to `[dev-dependencies]`, the `[features]`-below-`[package]` manifest reordering with English comments, and the removal of the unreachable `src/cli/validation.rs`

## 7. Quality Gate

- [ ] 7.1 Run `cargo fmt` and `cargo clippy -- -D warnings` and fix all warnings
- [ ] 7.2 Run `cargo nextest run --filter-expr 'test(vad) + test(sync_engine) + test(output_format)' || true` and confirm the targeted modules pass — these cover the five `hound` consumers and the `OnceLock`-backed output-mode globals
- [ ] 7.3 Run `scripts/quality_check.sh` once at the end (main agent only — do not invoke from sub-agents) and ensure it is green
- [ ] 7.4 Run `cargo test --doc --all-features` to confirm rustdoc examples still compile
