## Context

Five changes have run. A0 pruned five phantom dependencies and deleted an orphan module. A1 cut the thirteen `core`/`services` → `crate::cli` edges behind the `core::report::Reporter` seam and installed a guard test so they cannot come back. A2 relocated `input_handler`, the sync pairing logic and `create_default_output_path` into `core`, and moved `exit_code`/`user_friendly_message` out of `SubXError` into `subx_cli::cli::error_ext::SubXErrorExt`. B1 created the `subx-core` repository, mounted it as a submodule at `subx-core/`, turned `subx-cli/Cargo.toml` into the workspace root, declared the dual `{ path, version }` dependency, and proved the wiring with `subx_cli::CORE_VERSION`.

The result is a two-crate skeleton with all the code in the wrong crate. `subx-core/src/lib.rs` holds one `const`. `subx-cli/src/` holds `cli/` (5,101 LOC), `commands/` (6,283), `core/` (21,467), `services/` (6,228), `config/` (7,647), `error.rs` (1,345), `lib.rs` (819) and `main.rs`. The first two belong where they are. The other four — 95 files, 36,687 LOC — do not.

B2 moves them. The premise the whole A-series was built on is that this can be a move rather than a redesign, and three greps over the current tree confirm the premise held:

| Check | Result |
|---|---|
| `` [`crate::…`] `` doc links inside `src/{core,services,config}` and `src/error.rs` | 45, **all** pointing at items that move with them; zero at `crate::cli` or `crate::commands` |
| `subx_cli::` occurrences inside the moving trees | 98, across 30 files, **all** doctest `use` lines naming `core`/`config`/`error`/`services`/`Result`/a macro; zero naming a CLI type |
| Trait impls in `src/cli` + `src/commands` whose self type will become foreign | 1 — `impl From<SyncMethodArg> for crate::core::sync::SyncMethod` (`src/cli/sync_args.rs:27`) |

So the code motion is genuinely mechanical. What is not mechanical, and what this document is about, is: how a move crosses a repository boundary at all; what shape `subx-core`'s public API takes, permanently, from this commit; how two manifests are derived from one when SDR §4's allocation disagrees with the use sites in four places; and whether B2 can land without B3.

Two constraints bound every answer. `broken_intra_doc_links = "deny"` is set in **both** manifests (B1 duplicated the `[lints.*]` blocks), so a doc link that crosses the new boundary in the wrong direction is a hard build failure with no repair available — core has no dependency on the CLI to resolve through. And SDR §8's verified list of ~40 items the Tauri GUI consumes is an API contract as of this change: after B2 the GUI's migration is meant to be a global `subx_cli::` → `subx_core::` substitution, and any path that does not survive that substitution is a break the split was supposed to avoid.

## Goals / Non-Goals

**Goals:**

- Move `src/core/`, `src/services/`, `src/config/` and `src/error.rs` into `subx-core/src/` at identical relative paths, with the contents byte-identical apart from doctest `use` lines, and with per-file history preserved on the core side.
- Author `subx-core/src/lib.rs` as the real crate root: the crate-level attributes that governed this code inside `subx-cli`, the four module declarations, the re-export set, `Result` and `VERSION`.
- Fix `subx-core`'s public API paths as a stability contract, chosen so that every item in SDR §8's list survives a `subx_cli::` → `subx_core::` substitution.
- Derive two correct manifests from one, checking SDR §4 against the use sites rather than transcribing it, and leaving no zero-use-site declaration in either.
- Make `archive-rar` and `slow-tests` real gates in core with pass-through forwarders in `subx-cli`.
- Complete the intra-doc link and doctest sweep in this change, so `cargo doc --no-deps --all-features` and `cargo test --doc --all-features` are green in both crates on the same commit.
- Leave `subx-cli` — binary, library facade, and all 136 test files — compiling and green, without depending on B3.

**Non-Goals:**

- Moving anything under `tests/`, `benches/`, `assets/` or `tests/fixtures/`. That is B3, including the `tests/common/` → `test-support` feature design, which is the series' single biggest hazard (implementation plan §6) and gets a change to itself.
- Moving `src/cli/` or `src/commands/`. SDR D7 keeps `commands/` in `subx-cli` (the GUI has zero references to `subx_cli::commands` and zero to any clap `*Args` type); SDR D8 keeps `clap` out of core permanently.
- Moving `App`. See Decision 4.
- Renaming, restructuring, merging or splitting any module, type, function or field. A move that also reshapes is a move whose regressions cannot be found by inspection.
- Changing any error message, exit code, `category()` string, `machine_code()`, JSON envelope field, CLI flag or configuration key. `category()`'s literals are load-bearing for the GUI's `core.{category}` i18n keys (SDR §8).
- Bumping `subx-cli` to 2.0.0, reworking `cargo publish`, adding Dependabot, or adding CI to the `subx-core` repository. All C1's (SDR §7); B1 already itemised them as deferred.
- Widening `scripts/quality_check.sh` or `scripts/check_coverage.sh` to `--workspace`, or setting per-crate coverage thresholds. B1 deliberately left the scripts alone and C1 owns the decision; B2 keeps that arrangement and verifies `--workspace` by hand.
- Adding `#[deprecated]` to the back-compat re-exports. AGENTS.md forbids new `#[deprecated]`; SDR D11 prescribes rustdoc prose.
- Adding `openspec/` to the `subx-core` repository, or moving any capability. C2a/C2b.

## Decisions

### Decision 1: The move crosses a repository boundary, so `git mv` cannot do it — history is carried by `git filter-repo` plus an unrelated-histories merge

`subx-core/` is a **gitlink**: `subx-cli`'s index holds exactly one entry for it, mode `160000`, and nothing beneath it is tracked by `subx-cli`. `git mv src/core subx-core/src/core` therefore does not do what the shorthand "`git mv`, preserving history" suggests — the destination path is not a place `subx-cli`'s index can hold files, and the source history is in a repository the destination has never heard of. Naming the mechanism precisely matters, because the naïve reading produces a `subx-core` whose entire history is one commit called "import core sources", with `git blame` dead for 36,687 lines.

Three options, and the cheap one is not the obvious one:

**Rejected — plain copy plus a single import commit.** `mv` the directories, `git rm` them in `subx-cli`, `git add` them in `subx-core`, commit both. Takes ten minutes and loses everything: `git log` on `subx-core/src/core/matcher/engine.rs` shows one commit, `git blame` attributes every line to the import, and `git log --follow` cannot cross the boundary because there is no shared object graph. For a 21k-LOC engine with years of history, that is a real and permanent loss of the ability to answer "why is this line here".

**Rejected — `git subtree split`.** It rewrites a single prefix to the repository root. Four prefixes are needed (`src/core`, `src/services`, `src/config`, `src/error.rs`), and none of them wants to become the root — they want to stay at `src/core` etc. under a new root. Composing four splits into one coherent history is more work than the option below, for a worse result.

**Chosen — `git filter-repo` on a scratch clone, fetched into `subx-core` and merged with `--allow-unrelated-histories`.**

```
git clone https://github.com/jim60105/subx-cli /tmp/core-history
cd /tmp/core-history
git filter-repo --path src/core --path src/services --path src/config --path src/error.rs
```

The paths the split needs are the paths they already have, so **no path rewriting is required** — `filter-repo` keeps `src/core/...` as `src/core/...`, which is exactly where they belong under `subx-core/`'s root. The filtered history is then fetched into `subx-core` and merged:

```
cd subx-core
git remote add history /tmp/core-history && git fetch history
git merge --allow-unrelated-histories history/main
```

The merge is conflict-free by construction: B1's initial commit contains `src/lib.rs`, `Cargo.toml` and configuration; the filtered history contains only `src/core/**`, `src/services/**`, `src/config/**` and `src/error.rs`. The two trees are disjoint.

**What this buys:** every commit that ever touched those 95 files is in `subx-core`'s history, at its real path, with its real author and date. `git blame` and `git log` work. **What it costs:** one merge commit with two roots, which reads oddly in `git log --graph` and is worth a note in `subx-core/README.md`. It does **not** rewrite or force-push `subx-core`'s existing `main` — B1's initial commit stays reachable and unchanged, so nobody has to re-clone.

`subx-cli`'s side is a plain `git rm -r src/core src/services src/config && git rm src/error.rs`. Its own history is untouched: the files' past stays visible in `subx-cli` too, which is the correct outcome — the code genuinely was there.

**Fallback if `git filter-repo` is unavailable:** do the plain copy (option one), and separately push the filtered history to `subx-core` as an orphan branch `pre-split-history` for archaeology. That preserves the ability to answer historical questions at the cost of `git blame` on `main`. It is strictly worse and should only be reached for if `filter-repo` genuinely cannot be installed; it is recorded so the decision is not silently downgraded.

### Decision 2: `subx-core`'s public API keeps the exact module paths — `subx_core::core::matcher::…`, not `subx_core::matcher::…`

The `core::` segment is undeniably odd. `subx_core::core::matcher::MatchEngine` stutters, `subx_core::core::formats::Subtitle` reads like a mistake, and `core` shadows Rust's own `core` crate at the crate root (as it already does inside `subx-cli`). A flattened surface — `subx_core::matcher::MatchEngine`, `subx_core::formats::Subtitle` — is what anyone would design from scratch.

**It is not chosen, and the reason is arithmetic.** SDR §8 enumerates the Tauri GUI's consumed surface, verified across all sixteen `.rs` files of `../subx/src-tauri/src/`. Counted by path prefix:

| Prefix | Items |
|---|---|
| `core::matcher::…` | `FileDiscovery`, `MediaFile`, `MediaFileType`, `MatchEngine`, `MatchOperation`, `MatchConfig`, `engine::{FileRelocationMode, ConflictResolution, apply_unique_target_paths, OperationOutcome, OperationError}` |
| `core::formats::…` | `manager::FormatManager`, `Subtitle`, `converter::{FormatConverter, ConversionConfig, ConversionResult}` |
| `core::sync::…` | `SyncEngine`, `SyncMethod`, and (after A2) `create_default_output_path` |
| `core::translation::…` | `TranslationEngine`, `TranslationRequest`, `parse_glossary_text` |
| `core::…` (direct) | `ComponentFactory`, `lock::acquire_subx_lock`, `file_manager::FileManager` |
| `core::input::…` (after A2) | `InputPathHandler`, `CollectedFiles` |

That is roughly 30 of the ~40 consumed items sitting under `core::`. A flatten breaks **every one of them**, and it breaks them in the same change that is supposed to make the GUI's migration a mechanical `subx_cli::` → `subx_core::` substitution (implementation plan §7 states exactly that: "把 `subx_cli::` 全域換成 `subx_core::`"). Trading a one-line-per-file migration for a thirty-item hand-audit, in exchange for removing a stutter, is a bad trade — and it would be paid by an out-of-tree repository on someone else's schedule.

There is a second, quieter reason. `subx-cli`'s D11 back-compat re-exports (`pub use subx_core::core;`) only work as a compatibility layer if the module *tree* underneath is unchanged. Flattening core and then reconstructing a fake `core` module inside `subx-cli` to preserve the old paths would mean maintaining two divergent trees for the same items — the worst of both.

**Alternative considered — flatten, and add `pub use core::*;`-style aliases at the root for compatibility.** Rejected: glob re-exports of eight modules at the crate root would drag `formats`, `matcher`, `sync`, `parallel`, `language`, `archive`, `lock`, `uuidv7`, `fs_util`, `input`, `report`, `factory` and `file_manager` into one flat namespace with real collision risk (`sync` vs `tokio::sync` idioms, `formats::converter` vs `translation`), and rustdoc would show every item twice, which under `broken_intra_doc_links = "deny"` makes ambiguous links a build error rather than a nuisance.

**Alternative considered — flatten now, since `subx-core` is at 1.0.0 and nothing has been published yet.** Rejected on the same arithmetic: crates.io has not seen the crate, but the GUI has seen these paths for the crate's entire life under the `subx_cli::` name, and it is the only consumer that exists.

**Accepted cost:** `subx_core::core::` is permanent, and a future flatten is a 2.0.0 event for `subx-core`. `subx-core/src/lib.rs`'s crate-level rustdoc says so explicitly, so a future contributor finds a decision rather than an oversight.

### Decision 2a: no `pub use core::report;` alias

SDR §2.1 names the eventual public path for A1's `Reporter` seam as `subx_core::report`, and A1's Decision 1 explicitly left the alias as an additive choice for B2: "B2 may add `pub use core::report;` … Nothing in this change forecloses it."

**B2 declines it.** The alias would be a partial flatten of exactly the kind Decision 2 rejects, and it would create the inconsistency a reader trips over first: `report` at the root, `matcher` two levels down, with no principle distinguishing them. The one argument that justifies tolerating `core::` everywhere else — an existing consumer whose paths must not move — does not apply to `report`, which A1 created and which has **zero** consumers outside the crate. There is nothing to be compatible with, so the tie is broken by consistency.

Under `broken_intra_doc_links = "deny"` a second path is also not free: two routes to one module make `` [`report`] `` ambiguous from some scopes, and rustdoc renders the module twice.

The alias remains additive and can be introduced the day a consumer asks for it. Declining it now costs nothing; adding it now costs a permanent inconsistency.

### Decision 3: `subx-core/src/lib.rs` inherits the crate-level attributes, not just the module list

The four moving trees were compiled under `subx-cli`'s crate root attributes (`src/lib.rs:90-100`). Crate-level inner attributes do not travel with a file; if `subx-core/src/lib.rs` omits them, 36,687 LOC are suddenly linted under different rules and `cargo clippy -- -D warnings` fails on code nobody changed. The full set moves:

```rust
#![allow(
    clippy::new_without_default, clippy::manual_clamp, clippy::useless_vec,
    clippy::items_after_test_module, clippy::needless_borrow,
    clippy::uninlined_format_args, clippy::collapsible_if
)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
```

`subx-cli/src/lib.rs` keeps the identical block. The two are duplicated rather than shared, for the same reason B1's Decision 3 duplicates `[lints.*]`: workspace inheritance is prohibited, and a standalone `subx-core` clone has no parent to inherit from. `clippy::items_after_test_module` matters specifically — `src/error.rs:169` opens a `#[cfg(test)] mod tests` roughly a thousand lines before the file ends, and without the allow the move alone would produce a clippy error.

Module-level inner attributes travel with their files and need no action: `src/core/mod.rs:17` (`#![allow(dead_code)]`), `src/core/formats/mod.rs:132`, `src/core/matcher/mod.rs:161`, `src/services/mod.rs:99`, `src/config/mod.rs:2` (`#![allow(deprecated)]`), `src/config/service.rs:1`.

The crate root also carries the re-export set. It is exactly SDR D11's list, restated from `subx-cli/src/lib.rs:113-133` with the `cli`/`commands` declarations dropped:

```rust
pub mod config;
pub mod core;
pub mod error;
pub mod services;

pub use config::Config;
pub use config::{
    ConfigService, EnvironmentProvider, ProductionConfigService, SystemEnvironmentProvider,
    TestConfigBuilder, TestConfigService, TestEnvironmentProvider,
};

pub type Result<T> = error::SubXResult<T>;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

`VERSION` is B1's, kept verbatim — B1's `crate-topology` spec requires it, and `subx_cli::CORE_VERSION` reads it.

The crate-level `//!` rustdoc is rewritten rather than copied. `subx-cli/src/lib.rs`'s header describes a CLI (`- [`cli`] - Command-line interface and argument parsing`, `- [`commands`] - Implementation of all SubX commands`), and both of those links would be broken-intra-doc-link errors in core. It also carries a stale "Feature Flags" block naming `ai`, `audio` and `parallel`, none of which has ever existed in the manifest; core's header lists the two features that do exist (`archive-rar`, `slow-tests`). The four `//!` doctests in that header (`use subx_cli::config::{TestConfigService, ConfigService};`, `use subx_cli::{Result, error::SubXError};`, and two more) are core-only in content and move across with their `use` lines rewritten.

### Decision 4: `App` stays in `subx-cli`, and `subx-cli`'s re-export surface is exactly SDR D11 — no more, and specifically not `VERSION`

`App` looks like a library type and is documented as one ("Use SubX as a library component in larger applications"). It is not portable to core, and the reason is in its own signatures:

- `App::run` (`src/lib.rs:262`) calls `<cli::Cli as clap::Parser>::parse()` — clap, which SDR D8 bars from core permanently.
- `App::handle_command` (`:306`) takes `cli::Commands` and dispatches through `crate::commands::dispatcher::dispatch_command`.
- `match_files` (`:340`), `convert_files` (`:382`), `sync_files` (`:440`) and `sync_files_with_offset` (`:502`) each build a clap `*Args` literal (`cli::MatchArgs`, `cli::ConvertArgs`, `cli::SyncArgs`) and call `handle_command`.

Only `new` (`:212`), `new_with_production_config` (`:236`), `config_service` (`:527`) and `get_config` (`:539`) are clap-free, and they are a thin wrapper over `ConfigService` that core already exposes directly. `App` is a programmatic façade over the *CLI's* command set; it belongs with the command set. SDR §8 confirms the GUI does not consume it, and the only test that does is `tests/config_service_integration_tests.rs:7`, which stays in `subx-cli` under B3's split.

The re-export block in the slimmed `subx-cli/src/lib.rs`:

```rust
pub use subx_core::{config, core, error, services};
pub use subx_core::Config;
pub use subx_core::{
    ConfigService, EnvironmentProvider, ProductionConfigService, SystemEnvironmentProvider,
    TestConfigBuilder, TestConfigService, TestEnvironmentProvider,
};
pub type Result<T> = subx_core::error::SubXResult<T>;
```

Three points about what is *not* in it:

- **`VERSION` is not re-exported.** SDR D11 lists `VERSION` among the names that stay available at `subx_cli::`, and it does — as `subx-cli`'s own `env!("CARGO_PKG_VERSION")`, unchanged. Aliasing it to `subx_core::VERSION` would violate B1's `crate-topology` requirement "The Wiring Is Proven by a Compile-Time Reference", whose scenario *The two version constants stay distinct* states that each constant SHALL report its own crate's version. The two version lines diverge at C1 (2.0.0 vs 1.0.0), so an alias would make `subx_cli::VERSION` start lying. `subx_cli::CORE_VERSION` remains the way to read core's version, exactly as B1 designed it.
- **`Result` is a type alias, not a re-export.** `pub type Result<T> = subx_core::error::SubXResult<T>;` and `pub use subx_core::Result;` denote the same type and are interchangeable for consumers; the alias form is kept because it is what `src/lib.rs` already says, so the diff is one path segment rather than a construct change.
- **`App`, `cli` and `commands` are not touched.** They are `subx-cli`'s own.

**Why this still justifies 2.0.0 (SDR D6, locked).** Every path a consumer wrote still resolves, so the change is source-compatible in the narrow sense — which is exactly the trap. Three things make it a major bump regardless:

1. **Type identity crosses a crate boundary.** `subx_cli::error::SubXError` and `subx_core::error::SubXError` are now the *same* type, which is fine — but a consumer that also depends on a *different* version of `subx-core` (say `1.0` transitively and `1.1` directly, or a future 2.x) gets two distinct `SubXError` types that no longer unify. That failure is invisible in `subx-cli`'s own build and appears only in a consumer's dependency graph. Semver requires the major bump precisely for hazards that a single-crate build cannot detect.
2. **The re-exports are temporary by design.** SDR D11 marks them legacy; they exist for the window between B2 and the GUI's migration PR, and their removal is a second breaking change. Shipping them under a minor bump would mean two majors where one suffices.
3. **A2 already broke the surface.** `SubXError::exit_code` and `user_friendly_message` became trait methods on `subx_cli::cli::error_ext::SubXErrorExt`. An inherent-method caller outside the crate is broken today; the 2.0.0 release is where that break is allowed to land.

The bump itself is C1's, per B1's Decision 6 — `subx-cli` stays at `1.9.1` through B2.

### Decision 5: `Test*` utilities stay unconditional public API of `subx-core`, ungated

SDR D10 locks this and the temptation to relitigate it is real, so the reasoning is recorded rather than assumed. `TestConfigService` (`src/config/test_service.rs`), `TestConfigBuilder` (`src/config/builder.rs`) and `TestEnvironmentProvider` (`src/config/environment.rs`) are 48 KB of production-compiled code whose only purpose is testing. Every instinct says `#[cfg(feature = "test-support")]`.

**They stay ungated, for four reasons:**

1. **The GUI's own test suite depends on them.** SDR §8 lists `config::{… TestConfigService, TestConfigBuilder, TestEnvironmentProvider}` in the verified consumed set. Gating them means the GUI must add `subx-core = { version = "1", features = ["test-support"] }` under `[dev-dependencies]` — and, because Cargo unifies features across a build graph, that would silently enable the feature for its normal dependency too, which is the confusing half-state a feature gate was supposed to prevent.
2. **AGENTS.md mandates them in this project's own tests.** "Always use `TestConfigService` / `TestConfigBuilder` in tests; never `ProductionConfigService`." Under a gate, forgetting the feature produces "cannot find `TestConfigService` in `config`" — an error that points at the wrong problem.
3. **B3 needs the decision to already be made.** B3 introduces a `test-support` feature for the `tests/common/` helpers (SDR §6). If the `Test*` types were also behind a gate, B3 would have to decide whether it is the *same* gate, and get feature unification right across `[dependencies]` and `[dev-dependencies]` in the same graph. Leaving them unconditional keeps B3's feature scoped to the thing it was invented for.
4. **Gating is not free to reverse, and B2 is the wrong change to attempt it in.** Moving a type and simultaneously changing its availability means a compile error cannot be attributed to either action.

**Accepted cost:** ~48 KB of test-only code compiles into every release build of every consumer. `[profile.release]`'s `lto = true` and `strip = true` remove most of what is unreachable. If it ever measurably matters, gating is a `subx-core` minor-with-default-on followed by a major-with-default-off, and it is a change that can be made in isolation — which is exactly the property B2 should preserve rather than spend.

### Decision 6: `SubXError`'s `From` impls land in core, and `subx-cli` needs no impl it can no longer write

`src/error.rs` carries eight conversions into `SubXError`:

| Source | Site | Crate it pulls in |
|---|---|---|
| `std::io::Error` | `:44` (`#[from]`) | `std` |
| `anyhow::Error` | `:165` (`#[from]`) | `anyhow` |
| `reqwest::Error` | `:955` | `reqwest` |
| `walkdir::Error` | `:967` | `walkdir` |
| `symphonia::core::errors::Error` | `:975` | `symphonia` |
| `config::ConfigError` | `:982` | `config` |
| `serde_json::Error` | `:996` | `serde_json` |
| `Box<dyn std::error::Error>` | `:1341` | `std` |

All six named crates are in SDR §4's `subx-core` set, so the impls compile in core with no manifest surprise. This is the reason `anyhow`, `reqwest`, `walkdir`, `symphonia`, `config` and `serde_json` are core dependencies even where core's *logic* barely touches them: the error type's conversion surface pins them.

**Does `subx-cli` lose the ability to write a conversion it needs?** No. `SubXError` becomes a foreign type in `subx-cli`, so `impl From<X> for SubXError` there would be an orphan-rule violation unless `X` is local. A grep of `src/cli/` and `src/commands/` for `impl` blocks whose self type will become foreign returns exactly two lines, and only one of them crosses the boundary:

- `src/cli/convert_args.rs:260` — `impl std::fmt::Display for OutputSubtitleFormat`. Local type. Unaffected.
- `src/cli/sync_args.rs:27` — `impl From<SyncMethodArg> for crate::core::sync::SyncMethod`.

The second is the interesting one, and it is **legal**. Its shape is `impl From<Local> for Foreign`: the trait is foreign (`From`), the self type is foreign (`subx_core::core::sync::SyncMethod`), and the trait's type parameter is local (`SyncMethodArg`). RFC 2451 ("re-balancing coherence", stable since Rust 1.41) permits `impl<...> ForeignTrait<LocalType> for ForeignType` when no uncovered type parameter precedes the local type — and here there are no generic parameters at all. So `sync_args.rs:27` compiles unchanged after the move, with only its path rewritten from `crate::core::sync::SyncMethod` to `subx_core::core::sync::SyncMethod`. A2's Decision 2a already established that this impl must live where the clap type lives; the orphan rule agrees.

`subx-cli` also gains nothing it must write. Its error paths construct `SubXError` through the existing helper constructors (`SubXError::config`, `::command_execution`) and propagate with `?` through `From` impls that already exist in core.

**One residue the move exposes.** `src/error.rs:169` opens a `#[cfg(test)] mod tests` whose later assertions call `exit_code()` (`:329-332`) and `user_friendly_message()` (`:338`, `:343`, and inside `test_no_api_key_leaks_in_any_variant`) — the two methods A2 moved onto `subx_cli::cli::error_ext::SubXErrorExt`. A2's Impact section enumerated the method bodies but not this test module, so B2 must assume it may still be there and check. If it is:

- The `exit_code` mapping assertions and the plain `user_friendly_message` assertions move into `src/cli/error_ext.rs`'s test module, beside A2's `Display == user_friendly_message` regression test.
- `test_no_api_key_leaks_in_any_variant` is the awkward one, because it is the executable form of the `secrets-protection` capability's variant audit and it reaches `crate::services::ai::error_sanitizer` (`src/services/ai/mod.rs:368`, a `pub mod`). It is **split, not moved**: the core copy keeps the exhaustive variant enumeration and asserts over `Display` and `Debug`; a mirror in `src/cli/error_ext.rs` asserts the same over `user_friendly_message()`, importing `subx_core::services::ai::error_sanitizer` for the sanitizing-construction half. Both halves keep their "if you add a new variant, extend this list" rustdoc.

This is the only place in B2 where a body is edited rather than moved, and it is called out here so it is not discovered mid-move.

### Decision 7: `crate::` needs no rewrite inside core; the CLI-side rewrite is done anyway, on purpose

The implementation plan (§6, hazard 2) names `broken_intra_doc_links = "deny"` as one of the three ways this series derails, and instructs A2 and B2 to fix every crossing link in the same change. Measured, the hazard is asymmetric and much smaller on the core side than feared.

**Inside `subx-core`: 45 links, zero rewrites.** Every `` [`crate::…`] `` in the moving trees names a target that moves with it — `crate::error::SubXError::SubtitleFormat` (8), `crate::error::SubXError::AiService` (4), `crate::config::validation` (4), `crate::config::validator` (3), `crate::config::field_validator` (3), `crate::core::formats::*` (8), `crate::core::uuidv7` (2), `crate::services::ai::*` (2), and eleven singletons. `crate::` means "this crate", and after the move this crate is `subx-core`, in which `error`, `config`, `core` and `services` are all top-level modules at the same relative paths. **The links keep resolving untouched.** The task is therefore a verification (`grep` for `crate::cli`/`crate::commands` returning zero, then `cargo doc --no-deps --all-features` in core) rather than an edit pass, and phrasing it as an edit pass would be the actual risk — 45 speculative rewrites is 45 chances to break a link that was fine.

**Doctests inside `subx-core`: 98 occurrences across 30 files, one blanket substitution.** Every one is `subx_cli::` followed by `core`, `config`, `error`, `services`, `Result`, or a macro name. `sed -i 's/subx_cli::/subx_core::/g'` over the moved files is correct and complete, and `cargo test --doc` in core proves it. A2 already deleted the one doctest that named a CLI type (`MatchArgs`), which is why there is no residue to judge case by case.

**Inside `subx-cli`: 98 references across 17 files, rewritten although they would still work.** `src/cli/` and `src/commands/` reach core through `crate::core::` (41), `crate::config::` (27), `crate::error::` (27) and `crate::services::` (3). Because `subx-cli/src/lib.rs` re-exports those four modules (Decision 4), `crate::core::matcher::MatchEngine` still resolves — rustc's resolver and rustdoc both follow `pub use`. Doing nothing would compile.

**They are rewritten to `subx_core::…` anyway**, for three reasons:

1. The D11 re-exports are declared legacy and scheduled for deletion once the GUI migrates. If in-crate code resolves through them, deleting them later is a 98-line change in a documentation-flavoured cleanup, not a one-line deletion.
2. Provenance becomes greppable. After the rewrite, `grep -rn "subx_core::" src/` is an exact inventory of what the CLI takes from core — which is the input C2b needs when splitting the MIXED capabilities.
3. It removes an ambiguity class that `broken_intra_doc_links = "deny"` punishes: a doc link written as `` [`crate::core::matcher::MatchEngine`] `` resolves through a re-export today and stops resolving the moment the re-export goes, in a change that has no reason to expect doc failures.

The blast radius is bounded and mechanical: 98 lines, 17 files, all of the form `use crate::X` → `use subx_core::X` or an inline `crate::X::Y` path. `src/main.rs` needs no change — its three `subx_cli::` paths are `cli::output`, `cli::RunOutcome` and `config::ProductionConfigService`, and the third resolves through the re-export, which is correct for a binary that is a consumer of its own library facade.

**The direction rule, stated normatively in the spec:** `subx-core` may never name `subx-cli` — not in a `use`, not in a path, not in a `` [`…`] `` doc link, not in a `#[cfg]`. There is no dependency edge in that direction and there never will be, so such a reference is unfixable rather than merely wrong. `subx-cli` may name `subx-core` freely in both code and doc links. A1's `tests/core_cli_boundary.rs` guard is repointed at `subx-core/src/` (resolved from `CARGO_MANIFEST_DIR`, never CWD) and widened to reject the token `subx_cli` and the string `crate::cli` in any line, comments included.

### Decision 8: the manifests are derived from the use sites, not transcribed from SDR §4

SDR §4's allocation was produced by a use-site grep and is right about `subx-core`. It is wrong in four places about `subx-cli`, and B2 is the change that finds out, so B2 fixes it. Each correction is backed by a grep that the task list repeats:

**`anyhow` is not a `subx-cli` dependency.** SDR §4 lists it. `grep -rnE '(^|[^a-zA-Z0-9_:])anyhow::' src/cli src/commands src/main.rs src/lib.rs` returns **0**. The only `anyhow` in the tree is `SubXError::Other(#[from] anyhow::Error)` (`src/error.rs:165`), which is core's. Declaring it in `subx-cli` would create precisely the zero-use-site entry A0 turned into a `supply-chain-hardening` spec violation, three changes after A0 removed five of them.

**`dirs` is also a `subx-cli` dependency.** SDR §4 assigns it to core alone. `src/commands/cache_command.rs:218` calls `dirs::config_dir()` inside `get_config_dir()` — production code, not a test. Both crates declare `dirs`; this is normal and costs nothing, since Cargo resolves one copy.

**`subx-cli`'s tokio needs the `time` feature.** SDR §4 gives it `rt-multi-thread` and `macros`. `src/commands/match_command.rs:770` opens `monitor_batch_execution` with `use tokio::time::{Duration, interval};` — production code, and `tokio::time` does not exist without the `time` feature. The declaration becomes `tokio = { version = "1.0", features = ["rt-multi-thread", "macros", "time"] }`. Feature unification would have hidden this (core enables `time`), which is exactly why it must be declared: a `subx-cli` that only compiles because a sibling turned a feature on is broken the moment the sibling stops.

**`subx-core`'s `[dev-dependencies]` are the two that are used, not the eight SDR §4 anticipates.** SDR §4's list (hound, mockall, wiremock, rstest, test-case, pretty_assertions, tokio-test, criterion) is the *post-B3* set. At B2 only the `#[cfg(test)]` modules inside the moved `src/**` exist, and they use `mockall` (2 sites) and `wiremock` (6 sites) and nothing else — `hound`, `rstest`, `test-case`, `pretty_assertions`, `tokio-test` and `criterion` all return **0**. `tempfile` (101 sites) and `regex` (14 sites) are used by those test modules but are already regular `[dependencies]` of core, so they need no dev entry. B2 therefore declares `mockall` and `wiremock` only; B3 adds the rest as it moves the tests and benches that need them. This keeps A0's "every declared dependency has a use site" requirement true at every commit rather than only at the end of the series.

Symmetrically, `subx-cli` gains `tempfile` and `async-trait` under `[dev-dependencies]` — used by `#[cfg(test)]` modules in `src/cli/sync_args.rs:474`, `src/commands/convert_command.rs:493` and `src/commands/match_command.rs:827`, all of which stay behind. Its other dev-dependencies (assert_cmd, predicates, mockall, rstest, test-case, wiremock, criterion, pretty_assertions, tokio-test, regex, hound) are **left alone**: several serve `tests/` files that B3 will move, and pruning them now would mean guessing B3's outcome. B3 owns that pass, as the implementation plan's conflict matrix already assigns.

`[profile.release]` and `[profile.dev]` stay in `subx-cli/Cargo.toml` only (B1's Decision 2, and a hard requirement — Cargo warns on every build otherwise). `[[bin]]` and both `[[bench]]` entries stay in `subx-cli`; the benches keep compiling because `benches/retry_performance.rs`'s `use subx_cli::services::ai::retry;` resolves through the D11 re-export, and they move in B3.

### Decision 9: B2 lands on its own and keeps CI green — B3 is not required to land with it

This is the most consequential judgement in the change, because getting it wrong means either a two-day change disguised as two one-day changes, or a red `main`.

**The question.** 136 test files, 23,006 LOC, none of which B2 moves. 90 of them contain `subx_cli::` imports of library internals. If those imports stop resolving when the modules leave `subx-cli`, then B2 breaks the test suite and must either carry B3's import rewrite (making it a two-day change) or land in the same commit as B3 (making the pair a two-day change with no intermediate green state).

**The answer, measured rather than assumed.** Every first path segment that `tests/` and `benches/` import from `subx_cli` was enumerated:

| First segment | Files | Resolves after B2 because |
|---|---|---|
| `subx_cli::config` | 62 | `pub use subx_core::config;` |
| `subx_cli::core` | 43 | `pub use subx_core::core;` |
| `subx_cli::cli` | 42 | still `subx-cli`'s own module |
| `subx_cli::services` | 36 | `pub use subx_core::services;` |
| `subx_cli::commands` | 25 | still `subx-cli`'s own module |
| `subx_cli::Result` | 7 | `pub type Result<T>` retained |
| `subx_cli::error` | 2 | `pub use subx_core::error;` |
| `subx_cli::App` | 2 | still `subx-cli`'s own type |
| `subx_cli::test_with_config` | 1 | see below |

**The D11 re-export surface covers all of it but one line.** That is not a coincidence — D11 exists precisely so that "existing consumers keep compiling", and the test suite is the largest existing consumer. The single gap is `#[macro_export]` (Decision 10), and re-exporting the twelve macros at `subx-cli`'s root closes it without editing a test file.

**Therefore B2 carries zero test-import rewriting**, `cargo nextest run` stays green across the B2→B3 boundary, and the two changes are independently landable and independently revertable. The implementation plan's batch 4 acceptance condition ("`subx-cli/src/` 只剩 `cli/`、`commands/`、`lib.rs`、`main.rs`；兩份 manifest 各自成立") is met without touching `tests/`.

**Three things B2 must do to keep that true**, each a task rather than an assumption:

1. The re-export list must be **complete**, not approximate. The enumeration above is the acceptance criterion, and it is re-run as a task after the move.
2. `benches/` must be verified too — `cargo bench --no-run` — because `benches/retry_performance.rs` imports `subx_cli::services::ai::retry` and Cargo does not build benches during `nextest run`.
3. The `slow-tests` feature must remain a **declared feature of `subx-cli`**, not merely a forwarder, because four files under `tests/` still write `#[cfg(feature = "slow-tests")]` and are compiled against `subx-cli`'s feature set. `slow-tests = ["subx-core/slow-tests"]` satisfies both roles: it is a real `subx-cli` feature *and* it turns core's gate on.

**What is honestly deferred, and why that is acceptable.** Between B2 and B3 the test suite is in an architecturally wrong but functionally correct state: 89 files that exercise core logic live in `subx-cli/tests/` and reach it through legacy re-exports, so `subx-core`'s own coverage reads near-zero in isolation and its standalone `cargo test` runs only the `#[cfg(test)]` modules inside `src/**`. Nothing depends on that being otherwise until C1 sets per-crate thresholds, and C1 is gated on B3 by the implementation plan's dependency graph. The combined workspace figure — which is what `scripts/check_coverage.sh` measures and what the 75% floor applies to — is unchanged, because the same lines are measured under a different package name.

**Alternative considered — rewrite the 90 test files' imports in B2 so they point at `subx_core::` immediately.** Rejected. It is B3's work done early, in a change already carrying 36,687 LOC of motion, and it would make the B2 diff unreviewable. It also gains nothing: those files are *moving* in B3, and rewriting an import in a file that is about to move is work performed twice.

**Alternative considered — land B2 and B3 as one change.** Rejected. It fuses two unrelated failure modes — the manifest/API/link failure mode and the `tests/common/` cross-repo sharing failure mode, the latter of which the implementation plan (§6, hazard 1) says can inflate from one day to three on its own. B1's proposal made the identical argument for separating itself from B2, and it applies here with more force.

### Decision 10: the twelve `#[macro_export]` macros land at `subx_core`'s root, and `subx-cli` re-exports them

`src/config/test_macros.rs` defines twelve macros, all `#[macro_export]`: `test_with_config`, `test_production_config_with_env`, `test_production_config_with_openai_env`, `create_production_config_service_with_env`, `create_production_config_service_with_empty_env`, `test_with_default_config`, `test_with_ai_config`, `test_with_ai_config_and_key`, `test_with_sync_config`, `test_with_parallel_config`, `create_test_config_service`, `create_default_test_config_service`.

`#[macro_export]` places a macro at the **crate root**, irrespective of the module that defines it. So the moment `src/config/test_macros.rs` becomes `subx-core/src/config/test_macros.rs`, these become `subx_core::test_with_config!` and friends — and `pub use subx_core::config;` does **not** reach them, because they were never in the `config` module path to begin with. This is the one hole in the otherwise-complete D11 re-export surface, and it is easy to miss because the macros are invisible to a module-level grep.

**Blast radius is one line.** Eleven of the twelve have zero call sites anywhere in `src/`, `tests/` or `benches/` — they are referenced only from their own doctests. The twelfth, `test_with_config`, is imported by `tests/config_set_integration_tests.rs:4`.

**Decision: `subx-cli` re-exports all twelve at its crate root**, `pub use subx_core::{test_with_config, create_test_config_service, …};`, under the same legacy-rustdoc treatment as the module re-exports. `pub use` of a `#[macro_export]` macro is stable and behaves like any other re-export.

**Why all twelve rather than just the one with a call site:** the twelve are one API surface documented as a set in `test_macros.rs`'s module header. Re-exporting a subset would make `subx_cli::test_with_config!` work and `subx_cli::test_with_ai_config!` not, with no rule a reader could infer — and the D11 re-exports exist to be *boring*. The cost is eleven names in a `pub use` list.

**Why not delete the eleven unused macros instead:** tempting (AGENTS.md's "delete the item and update all call sites" rule is exactly this shape), and it may well be right — but it is an API deletion, and B2 is a move. Deleting public items in the same change that relocates 36,687 LOC means a consumer break cannot be attributed. It is recorded in Open Questions for C3 or a follow-up.

Their doctests move with them and get the same `subx_cli::` → `subx_core::` substitution as every other doctest.

## Risks / Trade-offs

- **Risk: the cross-repository move loses history and nobody notices until someone needs `git blame` on `engine.rs`.** → Mitigation: Decision 1 fixes the mechanism (`git filter-repo` + `--allow-unrelated-histories` merge) rather than leaving "git mv" as a hand-wave, and the task list verifies it explicitly by running `git log --oneline -- src/core/matcher/engine.rs` inside `subx-core` and requiring more than one commit.
- **Risk: a file is dropped or duplicated during a 95-file, four-tree move.** → Mitigation: the move is verified by count and by content, not by eye — `find` file counts (64 + 20 + 10 + 1) and a `git ls-files` diff between the pre-move `subx-cli` paths and the post-move `subx-core` paths must agree exactly, and `subx-cli/src/` must afterwards contain only `cli/`, `commands/`, `lib.rs`, `main.rs`.
- **Risk: the `subx_cli::` → `subx_core::` doctest substitution is applied to the CLI side by accident.** → Mitigation: the substitution is scoped to files under `subx-core/src/` *after* they have moved, never to `subx-cli/src/`, and the post-condition is asymmetric and checkable — `grep -rn 'subx_cli' subx-core/src/` returns zero, while `subx-cli/src/lib.rs`'s own doctests keep saying `subx_cli::`.
- **Risk: a crate-level attribute is left behind and 36,687 LOC fail `clippy -- -D warnings` on lints nobody introduced.** → Mitigation: Decision 3 enumerates the exact block, and `clippy::items_after_test_module` is called out by name because `src/error.rs:169` would trip it immediately. The task list runs `cargo clippy --workspace -- -D warnings` before the documentation phase, not after.
- **Risk: a rustdoc link crosses the boundary in the unfixable direction.** → Mitigation: measured to be zero today (45 links, all internal), enforced going forward by the widened boundary guard test and by the normative direction rule in the `crate-topology` delta. `cargo doc --no-deps --all-features` is run in **both** crates, including from a standalone `subx-core` clone outside the `subx-cli` tree, because that is the only configuration in which core's own `[lints.rustdoc]` block is what enforces the deny.
- **Risk: the manifest split silently relies on feature unification, and `subx-cli` stops compiling when built without core's features.** → Mitigation: Decision 8's tokio `time` correction is exactly this failure caught early. The task list additionally builds `subx-cli` with `--no-default-features` and runs `cargo build -p subx-cli` to confirm nothing depends on a feature only a sibling turns on.
- **Risk: the D11 re-export list is incomplete and 90 test files stop compiling.** → Mitigation: Decision 9's enumeration is a task, re-run after the move as a positive check (`cargo nextest run` green, `cargo bench --no-run` green) rather than as an argument. The `#[macro_export]` hole (Decision 10) is the one case the module-level enumeration would have missed, and it is closed explicitly.
- **Risk: `subx-core` acquires workspace inheritance while its manifest is being authored, and the workspace build stays green while a standalone clone breaks.** → Mitigation: B1's `crate-topology` spec already prohibits it normatively and B1's task list established the verification (clone into a scratch directory **outside** the `subx-cli` tree, then `cargo build` and `cargo fmt --check`). B2 repeats that verification verbatim, because B2 is the change that adds ~30 dependency lines to that manifest and is therefore where the mistake would be made.
- **Risk: `subx-core`'s coverage reads near-zero between B2 and B3 and someone "fixes" it by lowering the floor.** → Mitigation: the combined workspace figure is what `scripts/check_coverage.sh` measures and it does not move; the per-crate split is C1's and is gated on B3 by the dependency graph. The proposal's Coverage note says so, so a reader of the CI output finds the explanation.
- **Trade-off: `subx_core::core::` stutters, permanently.** → Accepted (Decision 2). ~30 of the GUI's ~40 consumed items sit under `core::`, and the alternative trades a one-line migration for a thirty-item hand-audit paid by an out-of-tree repository. The crate-level rustdoc records the reasoning so it reads as a decision.
- **Trade-off: ~48 KB of `Test*` scaffolding ships in every release build.** → Accepted (Decision 5, SDR D10). `lto = true` and `strip = true` recover most of it, and gating remains a clean isolated change if it ever measures.
- **Trade-off: `subx-cli` re-exports twelve macros that have one call site between them.** → Accepted (Decision 10). Eleven names in a `pub use` versus a partially-working macro surface with no inferable rule.
- **Trade-off: 89 core-facing test files sit in the wrong repository for the length of one change.** → Accepted (Decision 9). They keep passing, the combined coverage figure is unchanged, and nothing downstream depends on the split until C1 — which the dependency graph already sequences after B3.

## Migration Plan

Each step leaves the tree either building or failing loudly, and steps 2–5 are one commit in `subx-core` and one in `subx-cli` that must be pushed together (the gitlink is part of the second).

1. **Baseline.** Confirm A0–B1 landed: `grep -rn "crate::cli" src/core src/services` is 0, `src/core/input/mod.rs` and `src/cli/error_ext.rs` exist, `subx-core/` is initialised and `cargo build` is green. Record the workspace coverage percentage and the `git ls-files src/{core,services,config}` file list for later comparison.
2. **Produce the filtered history** (Decision 1) into a scratch clone, before touching either worktree. It is derived from `subx-cli`'s pre-move HEAD, so it must be produced first.
3. **Move the files.** `git rm` in `subx-cli`, place them under `subx-core/src/`, fetch and merge the filtered history in `subx-core`, then `git add` the merged tree. At this point neither crate compiles — this is the one window in the change where that is true, and it is closed by step 5.
4. **Author `subx-core/src/lib.rs`** (Decision 3) and `subx-core/Cargo.toml`'s `[dependencies]`, `[dev-dependencies]` and `[features]` (Decision 8). Run the doctest substitution over `subx-core/src/`. `cargo build -p subx-core` must now succeed; core is independently buildable before the CLI is repaired.
5. **Slim `subx-cli/src/lib.rs`**, split `subx-cli/Cargo.toml`, and rewrite the 98 `crate::{core,config,error,services}` paths across the 17 CLI-side files (Decisions 4, 7, 8). `cargo build` at the workspace root must now succeed. Commit the moved submodule pointer.
6. **Split the `src/error.rs` test module** if A2 left it whole (Decision 6), and repoint the boundary guard test at `subx-core/src/`.
7. **Verify:** `cargo nextest run` (unchanged test suite, still green), `cargo bench --no-run`, `cargo doc --no-deps --all-features` in both crates, `cargo test --doc --all-features`, `cargo clippy --workspace -- -D warnings`, and a standalone `subx-core` clone outside the tree.
8. **Documentation** and the `[Unreleased]` CHANGELOG entries in both repositories, then the quality gate on the main agent only.

**Rollback** is `git revert` in `subx-cli` (which restores `src/` and the old gitlink) plus `git revert` of the merge in `subx-core`. Nothing persists to disk, no data format changes, no configuration key moves, and the API surface is restored in one step because the re-exports made it path-compatible in the first place. The one non-reversible artifact is `subx-core`'s history merge, which is additive and harmless if the code is reverted on top of it.

## Sizing

Estimated at one workday, and the estimate is load-bearing rather than aspirational, because 36,687 LOC sounds like much more:

| Phase | Estimate |
|---|---|
| Filtered history + the physical move (Decision 1) | 1.5 h |
| `subx-core/src/lib.rs` + `subx-core/Cargo.toml` | 1 h |
| Slim `subx-cli/src/lib.rs` + split `subx-cli/Cargo.toml` | 1 h |
| 98-line path rewrite across 17 CLI-side files | 1 h |
| Doctest substitution + link verification in both crates | 1 h |
| `error.rs` test-module split | 0.5 h |
| Verification (build, nextest, bench, doc, clippy, standalone clone) | 1 h |
| Documentation + CHANGELOG (both repositories) | 1 h |

The LOC count is misleading because **no line of the 36,687 is edited** except doctest `use` lines. The real work is two manifests, one crate root, one slimmed crate root, and 98 mechanical path rewrites — and every one of those is enumerable in advance, which is the whole point of having spent A0–A2 and B1 first. The two items that could overrun are the `filter-repo` step (if the tool is not installed) and the `error.rs` test split (if A2 left more behind than expected); both have named fallbacks above.

## Open Questions

- **Should the eleven unused `#[macro_export]` macros be deleted?** They have zero call sites and AGENTS.md's rule is "delete the item and update all call sites". B2 re-exports them rather than deleting them (Decision 10), because an API deletion inside a 36,687-LOC move is unattributable. C3 or a small follow-up should decide. Not blocking.
- **Should `subx-core` get a `CHANGELOG.md` now?** B1 deferred the question to C3 on the grounds that core held no code. It now holds all of it, and B2 writes `[Unreleased]` entries in both repositories, which implies the file. B2 creates it if it does not exist; C3 still owns the format and the backfill decision.
- **Does the GUI migration PR happen immediately after B2?** The implementation plan §7 says it can: one `Cargo.toml` line plus a global `subx_cli::` → `subx_core::` substitution. It is out of this series' scope and is not gated on B3, but B2 is the change that makes the claim testable — the tasks include verifying the SDR §8 item list resolves under `subx_core::`.
- **Is `subx_core::core::report` the final home for A1's seam?** Decision 2a declines the `subx_core::report` alias on consistency grounds. If D2 (`expose-core-orchestration-apis`) grows the seam into the primary orchestration entry point, the question is worth reopening — additively, which is why declining now costs nothing.
