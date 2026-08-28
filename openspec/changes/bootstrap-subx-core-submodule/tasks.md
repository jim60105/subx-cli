## 1. Preconditions and Baseline

- [ ] 1.1 Confirm A0, A1 and A2 have landed: `grep -rn "crate::cli" src/core src/services` returns **0** non-test hits, `src/cli/validation.rs` no longer exists, `src/core/input/mod.rs` and `src/cli/error_ext.rs` do exist, and `Cargo.toml` no longer declares `notify`, `once_cell`, `tokio-util`, `winapi` or `libc`
- [ ] 1.2 Run `scripts/quality_check.sh` once as the pre-change baseline (main agent only) and record that it is green — after this change the same script must still be green with no new flags
- [ ] 1.3 Record the baseline `cargo llvm-cov` line-coverage percentage from the most recent CI `coverage` job, so task 7.6 can confirm the workspace figure has not moved
- [ ] 1.4 Confirm `git submodule status` reports nothing and `.gitmodules` does not exist, so this is genuinely the first submodule in the repository

## 2. Create the `subx-core` Repository

- [ ] 2.1 Create the public GitHub repository `jim60105/subx-core` with default branch `main`, description "Core subtitle processing library for SubX", and licence GPL-3.0-or-later; do not initialise it with GitHub's generated `README`, `.gitignore` or `LICENSE` — every file is authored below
- [ ] 2.2 Copy `subx-cli/LICENSE` (674 lines, GNU GPL v3 text) verbatim to `subx-core/LICENSE`
- [ ] 2.3 Copy `subx-cli/.gitignore` verbatim to `subx-core/.gitignore` (`/target/`, `**/*.rs.bk`, `.idea/`, `.DS_Store`, `Thumbs.db`, `.env`, `*.log`, `junit.xml`, `target/nextest/*/junit.xml`, `tmp/`); do **not** add `Cargo.lock` to it — the lockfile is committed per `design.md` Decision 5b
- [ ] 2.4 Copy `subx-cli/.gitattributes` verbatim to `subx-core/.gitattributes`, keeping the `tests/fixtures/formats/** -text` rule and its two comment lines even though the fixtures do not arrive until B3
- [ ] 2.5 Copy `subx-cli/rustfmt.toml` verbatim to `subx-core/rustfmt.toml` (`edition = "2024"`, `max_width = 100`)
- [ ] 2.6 Copy `subx-cli/.config/nextest.toml` to `subx-core/.config/nextest.toml`, preserving all four profiles (`default`, `ci`, `quick`, `full`), both `[profile.*.junit]` tables and every setting byte-for-byte, but rewriting the Chinese comments at `:7`, `:10`, `:13`, `:16`, `:19`, `:22`, `:26`, `:34`, `:40`, `:47`, `:56` in English; do **not** change any `path` value — the junit resolution question is C1's (`design.md` Decision 5a)
- [ ] 2.7 Copy `subx-cli/.llvm-cov.toml` to `subx-core/.llvm-cov.toml` with the Chinese comments rewritten in English and the `"src/main.rs"` entry removed from `exclude-from-report` (a library has no binary entry point), leaving `["benches/*", "tests/*"]`
- [ ] 2.8 Copy `subx-cli/.codegraph/.gitignore` verbatim to `subx-core/.codegraph/.gitignore` (the 3-line self-ignoring file: `*` then `!.gitignore` under its comment header)
- [ ] 2.9 Write `subx-core/AGENTS.md` derived from `subx-cli/AGENTS.md`: keep `Coding Conventions` (`:191-219`), `Testing Conventions` (`:220-289`), `Documentation Conventions` (`:290-299`), `Configuration System` (`:300-352`), the `CPU-Intensive Operations — Main Agent Only` rule (`:53-73`) and the `CodeGraph` block (`:413-421`); drop `Execution Flow` (`:76-84`), the CLI rows of `Module Guide` / `Common Edit Targets` (`:112-144`), `Cargo Features` (`:183-190`) and `CI/CD Pipeline` (`:387-412`); add a new `## Repository Layout` section stating that this repository is consumed as a git submodule at `subx-cli/subx-core/`, that changes are committed here first and the pointer bumped in `subx-cli` second, that `Cargo.toml` must never contain a `[workspace]` table or any workspace inheritance, and that `[profile.*]` lives only in `subx-cli/Cargo.toml`
- [ ] 2.10 Write `subx-core/README.md`: what the crate is, its GPL-3.0-or-later licence, `cargo add subx-core`, its relationship to `subx-cli` (submodule at `subx-core/`, workspace member) and to the Tauri GUI at `jim60105/subx`, and an explicit note that at this commit the crate is a placeholder whose contents arrive with the source migration
- [ ] 2.11 Write `subx-core/Cargo.toml` with `[package]` (`name = "subx-core"`, `version = "1.0.0"`, `edition = "2024"`, `authors = ["Jim Chen <Jim@ChenJ.im>"]`, a library-flavoured `description`, `license = "GPL-3.0-or-later"`, `repository` and `homepage` pointing at `https://github.com/jim60105/subx-core`, `keywords = ["subtitle", "library", "ai", "video"]`, `categories = ["multimedia", "text-processing"]`, and an `exclude` list mirroring `subx-cli`'s minus `plans/` and `scripts/test_*.sh`), then `[package.metadata.docs.rs]` (`all-features = true`, `rustdoc-args = ["--cfg", "docsrs"]`), `[features] default = []`, and literal copies of `[lints.rustdoc]`, `[lints.clippy]` and `[lints.rust]` from `subx-cli/Cargo.toml`, followed by empty `[dependencies]` and `[dev-dependencies]` tables
- [ ] 2.12 Verify `subx-core/Cargo.toml` contains **no** `[workspace]` table, **no** `[profile.release]` or `[profile.dev]`, **no** `<field>.workspace = true` of any kind, and **no** `[[bin]]` — `grep -nE '^\[workspace|^\[profile|\.workspace *= *true|^\[\[bin\]\]' subx-core/Cargo.toml` must return nothing
- [ ] 2.13 Write `subx-core/src/lib.rs`: a crate-level `//!` header naming the crate, its GPL-3.0-or-later licence, and its role as the library half of SubX, plus exactly one public item — `pub const VERSION: &str = env!("CARGO_PKG_VERSION");` — with rustdoc carrying `# Examples` per the project's rustdoc rule, and a `#[cfg(test)]` unit test asserting `VERSION` is non-empty and equals `env!("CARGO_PKG_VERSION")`
- [ ] 2.14 In a scratch directory **outside** any `subx-cli` checkout, run `cargo build`, `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo test`, and `cargo package --list` inside `subx-core`; all must pass, and `cargo package --list` must not list `Cargo.lock`, `.github/` or `target/`
- [ ] 2.15 Commit `subx-core/Cargo.lock` as generated by task 2.14's build, then push the initial commit to `main`

## 3. Attach the Submodule

- [ ] 3.1 From the `subx-cli` repository root, run `git submodule add -b main https://github.com/jim60105/subx-core subx-core`
- [ ] 3.2 Confirm `.gitmodules` now contains `path = subx-core`, `url = https://github.com/jim60105/subx-core` and `branch = main`; add the `branch` line by hand if `git submodule add -b` did not write it
- [ ] 3.3 Confirm `git submodule status` prints the pinned commit with no `+` or `-` prefix, and that `git status` shows `.gitmodules` and `subx-core` as the only additions
- [ ] 3.4 Confirm `cargo build` still succeeds at this point — nothing references the new directory yet, so a failure here means something else regressed

## 4. Convert `subx-cli/Cargo.toml` into the Workspace Root

- [ ] 4.1 Insert `[workspace]` with `members = [".", "subx-core"]` and `resolver = "3"` as the **first** table of `subx-cli/Cargo.toml`, above `[package]`, with a short English comment explaining that `subx-core` is a git submodule
- [ ] 4.2 Confirm `[profile.release]` and `[profile.dev]` are untouched and still the last two tables of `subx-cli/Cargo.toml`, and that they exist in no other manifest
- [ ] 4.3 Confirm no `default-members` key is added — the default member set stays the root package, per `design.md` Decision 2
- [ ] 4.4 Run `cargo metadata --no-deps --format-version 1` and confirm `workspace_members` lists exactly `subx-cli` and `subx-core`
- [ ] 4.5 Run `cargo build` and confirm Cargo emits **no** warning about profiles in a non-root package
- [ ] 4.6 Temporarily run `git submodule deinit -f subx-core`, confirm `cargo build` now fails at workspace-manifest load naming `subx-core/Cargo.toml`, then restore with `git submodule update --init --recursive` — this is the evidence for the spec's "uninitialised submodule fails at manifest load" scenario

## 5. Wire and Prove the Dependency

- [ ] 5.1 Add `subx-core = { path = "subx-core", version = "1.0" }` to `subx-cli/Cargo.toml`'s `[dependencies]`, at the top of the table under a `# SubX core library (git submodule at subx-core/)` comment
- [ ] 5.2 Add `pub const CORE_VERSION: &str = subx_core::VERSION;` to `src/lib.rs` immediately after the existing `pub const VERSION` (`src/lib.rs:114`), with rustdoc explaining that it reports the linked `subx-core` version, that it is distinct from `VERSION`, and carrying a `# Examples` block that compiles
- [ ] 5.3 Add `subx-core` to the module/architecture list in `src/lib.rs`'s crate-level `//!` header (`src/lib.rs:17-25`), noting that the core engines are supplied by the `subx_core` crate
- [ ] 5.4 Add a unit test to `src/lib.rs`'s existing `#[cfg(test)]` module (beside the `VERSION` tests at `:569` and `:574`) named `core_version_matches_declared_requirement`, asserting `!CORE_VERSION.is_empty()` and that `CORE_VERSION.split('.').next() == Some("1")`, with a comment explaining that this is the only local guard against submodule/version drift because `path` resolution ignores the `version` key
- [ ] 5.5 Confirm `subx_cli::VERSION` is unchanged in value and meaning, and that no `pub use subx_core::VERSION;` was added anywhere
- [ ] 5.6 Run `cargo build` and confirm `Cargo.lock` gains exactly one `[[package]]` entry for `subx-core` with no `source` key (path dependencies carry none) and no new registry packages; never hand-edit the lockfile
- [ ] 5.7 Confirm the feature tables are untouched: `subx-cli`'s `archive-rar = ["dep:unrar"]` and `slow-tests = []` stay exactly as A0 left them, and `subx-core` declares only `default = []` — the SDR §4 pass-through arrangement is B2's

## 6. CI Checkout Wiring

- [ ] 6.1 Add `with: submodules: recursive` to `actions/checkout@v6` in `.github/workflows/build-test-audit-coverage.yml` job `test` (`:26`)
- [ ] 6.2 Add the same to job `archive-rar` (`:99`)
- [ ] 6.3 Add the same to job `security` (`:145`) — `actions-rust-lang/audit` resolves against the workspace lockfile and needs the member manifest present
- [ ] 6.4 Add the same to job `coverage` (`:157`)
- [ ] 6.5 Add `submodules: recursive` **and** `fetch-depth: 0` to `actions/checkout@v6` in `.github/workflows/release.yml` job `create-release` (`:18`), which extracts release notes from `CHANGELOG.md` at `:20-34`
- [ ] 6.6 Add `submodules: recursive` to job `build` (`:95`)
- [ ] 6.7 Add `submodules: recursive` **and** `fetch-depth: 0` to job `publish-crates` (`:192`), whose `cargo publish` walks submodules recursively for uncommitted changes
- [ ] 6.8 Confirm the workflow diff contains **only** those additions: `grep -c 'submodules: recursive' .github/workflows/*.yml` totals 7, and `git diff .github/` shows no change to the `cargo publish` line (`release.yml:196`), the four `hashFiles('**/Cargo.lock')` cache keys, the codecov upload paths, or the build matrix — the publish rework, pointer-version assertion and per-crate coverage thresholds are C1's

## 7. Verify End to End

- [ ] 7.1 Clone `subx-cli` fresh with `git clone --recurse-submodules` into a scratch directory and confirm `cargo build` succeeds with no manual submodule command
- [ ] 7.2 In the same scratch clone, confirm `cargo run -- --version` prints the same string as before this change
- [ ] 7.3 Run `cargo clippy --workspace -- -D warnings` and confirm it is clean, including `subx-core/src/lib.rs`; do **not** add `--workspace` to `scripts/quality_check.sh` — widening the scripts is B3/C1 work
- [ ] 7.4 Run `cargo fmt -- --check` from the `subx-cli` root and confirm it also format-checks `subx-core/src/lib.rs` using `subx-core/rustfmt.toml`
- [ ] 7.5 Run `cargo package --list --allow-dirty -p subx-cli` and confirm no file under `subx-core/` is listed in `subx-cli`'s own archive; then inspect the generated manifest to confirm the `subx-core` dependency retains `version` and has lost `path`
- [ ] 7.6 Run `scripts/check_coverage.sh -T -p ci --lcov lcov.info` (main agent only) and confirm the workspace line-coverage percentage matches the task 1.3 baseline within rounding — a `pub const` emits no executable coverage regions — and that the 75% threshold still passes
- [ ] 7.7 Confirm `git status` is clean inside `subx-core/` and that `git submodule status` shows no `+` prefix, so the pointer is committed and the tree is not dirty ahead of any future publish

## 8. Documentation

- [ ] 8.1 Add a "Building from source" note to `README.md` giving `git clone --recurse-submodules https://github.com/jim60105/subx-cli`, the `git submodule update --init --recursive` repair for existing clones, and the `git config submodule.recurse true` recommendation with the caveat that it does not apply to `git clone`
- [ ] 8.2 Mirror the same note in `README.zh-TW.md`, keeping the surrounding section structure and heading style of that file
- [ ] 8.3 Add a `## Repository Layout` section to `AGENTS.md` (before `## Architecture` at `:74`) describing the two repositories, the submodule at `subx-core/`, the workspace root, the prohibition on a `[workspace]` table or workspace inheritance inside `subx-core`, the root-only ownership of `[profile.*]`, and the clone/update commands; leave the stale "7 targets" release-matrix claim at `:396-403` for C3
- [ ] 8.4 Add a short two-crate topology paragraph to `docs/tech-architecture.md` ahead of its module map, stating that `subx-cli` is the workspace root and `subx-core` is a submodule-mounted member, and noting that the module allocation itself lands with the source migration; the full rewrite is C3's
- [ ] 8.5 Add a `### Added` entry under `[Unreleased]` in `CHANGELOG.md:9` recording the new `subx-core` crate and repository (1.0.0, GPL-3.0-or-later), its mounting as a git submodule at `subx-core/`, the `subx-cli` Cargo workspace, and the new `subx_cli::CORE_VERSION` constant
- [ ] 8.6 Add a `### Changed` entry under `[Unreleased]` recording that building from source now requires submodule initialisation, and that all seven CI checkout steps fetch submodules recursively

## 9. Quality Gate

- [ ] 9.1 Run `cargo fmt` and `cargo clippy -- -D warnings` and fix all warnings
- [ ] 9.2 Run `cargo nextest run --filter-expr 'test(version)' || true` and confirm the targeted modules pass
- [ ] 9.3 Run `scripts/quality_check.sh` once at the end (main agent only — do not invoke from sub-agents) and ensure it is green
- [ ] 9.4 Run `cargo test --doc --all-features` to confirm rustdoc examples still compile
