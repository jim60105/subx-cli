## Why

B1 built an empty destination. `subx-core/` exists, it is a submodule, it is a workspace member, and `subx-cli` links against it — but the crate holds one `pub const VERSION` and nothing else. All ~36.7k LOC of library code still sits in `subx-cli/src/`, which means the Tauri GUI still has to depend on `subx-cli` to reach `MatchEngine`, `ComponentFactory` and `FormatManager`, and `subx-core` on crates.io would be an empty shell. Every remaining change in the series — B3's test split, C1's per-crate coverage thresholds, C2a/C2b's capability migration, D1/D2's work on `subx-core/src/**` — is blocked on the code actually being there.

This is series change **B2**, and it is the physical move:

| Moves to `subx-core/src/` | Files | LOC |
|---|---|---|
| `src/core/**` | 64 | 21,467 |
| `src/services/**` | 20 | 6,228 |
| `src/config/**` | 10 | 7,647 |
| `src/error.rs` | 1 | 1,345 |
| **Total** | **95** | **36,687** |

A0–A2 exist so that this is a move and not a redesign, and the evidence says they succeeded. Three greps over the code as it stands confirm it:

- **All 45 `` [`crate::…`] `` intra-doc links inside the four moving trees point at items that are themselves moving** (`crate::error::SubXError::*`, `crate::config::validator`, `crate::core::formats::*`, `crate::services::ai::*`). Not one points at `crate::cli` or `crate::commands`. `crate::` therefore keeps resolving inside `subx-core` with **zero** rewrites — the opposite of the outcome the implementation plan (§6, hazard 2) warned about.
- **All 98 `subx_cli::` occurrences inside the moving trees are doctest `use` lines naming `core`, `config`, `error`, `services`, `Result`, or one of five `#[macro_export]` macros.** After A2 deleted the `MatchArgs` example, **zero** doctests in the moving trees reference a CLI type. The sweep is a mechanical `subx_cli::` → `subx_core::` rewrite across 30 files with no rewrite-or-delete judgement calls left in it.
- **The nine `crate::cli::output::…` reads and four `use crate::cli::display_ai_usage;` imports are gone** (A1), and `operation_error_from` no longer calls `user_friendly_message()` (A2). The boundary guard test A1 added keeps it that way.

What is genuinely hard about B2 is not the `git mv`. It is four things the preparatory changes could not do:

1. **The move crosses a repository boundary.** `subx-core/` is a gitlink in `subx-cli`'s index; `git mv src/core subx-core/src/core` cannot work, because the destination is not in `subx-cli`'s worktree-as-tracked. History preservation needs an explicit mechanism (`design.md` Decision 1).
2. **Two manifests have to be authored from one.** SDR §4 fixes the allocation, but a use-site grep finds four places where SDR §4 is wrong about `subx-cli`'s half, and those have to be corrected rather than transcribed.
3. **`subx-core`'s public API shape is decided here and is contract from here on.** SDR §8 lists ~40 items the GUI consumes by their current paths. A "tidier" flattened surface would break every one of them.
4. **`subx-cli` has to keep compiling — including its 136-file test suite — without B3.** It does, and the reason is precise rather than hopeful (`design.md` Decision 9).

## What Changes

**1. The sources move.** `src/core/`, `src/services/`, `src/config/` and `src/error.rs` leave `subx-cli` and arrive at the identical relative paths under `subx-core/src/`. No file is renamed, no item is renamed, no signature changes, no module is restructured. History is carried across the repository boundary with `git filter-repo` plus an unrelated-histories merge, so `git log` and `git blame` keep working inside `subx-core` (Decision 1).

**2. `subx-core/src/lib.rs` is authored** — replacing B1's placeholder, and inheriting the crate-level attributes that governed the moved code inside `subx-cli`:

```rust
#![allow(
    clippy::new_without_default, clippy::manual_clamp, clippy::useless_vec,
    clippy::items_after_test_module, clippy::needless_borrow,
    clippy::uninlined_format_args, clippy::collapsible_if
)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

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

SDR §2.1's optional shorter alias `pub use core::report;` is **not** added — Decision 2a gives the reasoning.

**3. `subx-cli/src/lib.rs` is slimmed** to `App`, its seven methods, `VERSION`, B1's `CORE_VERSION`, and the SDR D11 back-compat re-exports (`pub use subx_core::{config, core, error, services};` plus `Config`, the seven config-service items, and `pub type Result<T>`). `App` stays in `subx-cli` and is not re-exported by core: `App::run` calls `<cli::Cli as clap::Parser>::parse()` and `App::handle_command` takes `cli::Commands`, so it is unambiguously CLI-side (SDR D7/D8). `VERSION` stays `subx-cli`'s own `env!("CARGO_PKG_VERSION")` and is **not** aliased to `subx_core::VERSION` — B1's `crate-topology` spec requires the two constants to stay distinct. Per AGENTS.md no `#[deprecated]` is added; the re-exports are marked legacy in rustdoc prose only.

**4. The manifests split.** SDR §4's core dependency set moves into `subx-core/Cargo.toml`; `subx-cli` keeps the CLI set. Four corrections to SDR §4's `subx-cli` half, each backed by a use-site grep:

| SDR §4 says | Reality | Resolution |
|---|---|---|
| `anyhow` is a `subx-cli` dependency | **0** non-`crate::` hits in `src/cli`, `src/commands`, `src/main.rs`, `src/lib.rs`. `SubXError::Other(#[from] anyhow::Error)` is core's. | Do **not** declare `anyhow` in `subx-cli`. A0 made a zero-use-site declaration a `supply-chain-hardening` violation. |
| `dirs` is core-only | Used in **production** at `src/commands/cache_command.rs:218` (`get_config_dir`) | `subx-cli` declares `dirs` **as well**. Both crates may depend on the same registry crate. |
| `subx-cli` tokio features are `rt-multi-thread`, `macros` | `src/commands/match_command.rs:770` uses `tokio::time::{Duration, interval}` in production code | `subx-cli` tokio features become `["rt-multi-thread", "macros", "time"]`. |
| `subx-core` dev-deps are hound, mockall, wiremock, rstest, test-case, pretty_assertions, tokio-test, criterion | Only `mockall` (2 sites) and `wiremock` (6 sites) are used by the `#[cfg(test)]` modules **inside** the moving `src/**` | `subx-core` declares only `mockall` and `wiremock` at B2. B3 adds the rest when `tests/` and `benches/` arrive, so no manifest entry ever has zero use sites. |

`subx-cli` additionally gains `tempfile` and `async-trait` under `[dev-dependencies]` (used by `#[cfg(test)]` modules in `src/cli/sync_args.rs:474`, `src/commands/convert_command.rs:493`, `src/commands/match_command.rs:827`). `subx-cli`'s existing `[dev-dependencies]` are otherwise left alone — pruning them is B3's, once the test suite's final composition is known.

**5. Features become real gates in core with pass-through forwarders in the CLI** (SDR §4):

| Feature | `subx-core` | `subx-cli` |
|---|---|---|
| `archive-rar` | `["dep:unrar"]` — the only `#[cfg(feature = "archive-rar")]` sites are `src/core/archive/rar.rs:13,113` and `src/core/archive/mod.rs:223` | `["subx-core/archive-rar"]` |
| `slow-tests` | `[]` — gates five `#[cfg(…feature = "slow-tests")]` sites under `src/core/formats/**` | `["subx-core/slow-tests"]`, and still a real `cfg` for the four `tests/sync_*` files that use it until B3 moves them |
| `default` | `[]` | `[]` |

**6. The intra-doc link and doctest sweep**, which `broken_intra_doc_links = "deny"` makes a hard build gate in both crates. Its shape, verified against the current tree:

- **Inside `subx-core`:** all 45 `` [`crate::…`] `` links keep resolving unchanged, because every target moves with them. Verified, not assumed — the sweep is a re-grep and a `cargo doc`, not an edit pass.
- **Doctests inside `subx-core`:** 98 `subx_cli::` occurrences across 30 files become `subx_core::`. Zero of them name a CLI type.
- **Inside `subx-cli`:** 98 `crate::{core,config,error,services}` references across 17 files under `src/cli/` and `src/commands/` are rewritten to `subx_core::…`. They would still resolve through the D11 re-exports; they are rewritten anyway, so the legacy re-exports are load-bearing for out-of-tree consumers only and can be deleted later without touching in-crate code (Decision 7).
- **Direction rule:** `subx-core` may not name `subx-cli` in code **or** in a doc link, ever — there is no dependency edge to resolve one. A1's boundary guard test is widened to enforce the doc-link half.

**7. The one thing that is not a pure move.** `src/error.rs`'s `#[cfg(test)] mod tests` (from `:169`) contains assertions calling `exit_code()` and `user_friendly_message()` — the two methods A2 moved to `subx_cli::cli::error_ext::SubXErrorExt`. They cannot travel into core. They are split: the `secrets-protection` variant audit `test_no_api_key_leaks_in_any_variant` stays in core covering `Display` and `Debug`, and a mirrored `user_friendly_message()` assertion is added to `src/cli/error_ext.rs`'s test module, together with the `exit_code` mapping tests. If A2 already did this, B2 verifies and skips.

**8. Twelve `#[macro_export]` macros land at `subx_core`'s crate root**, because that is where `#[macro_export]` puts them regardless of which module defines them, and `pub use subx_core::config;` cannot reach them. Eleven have zero call sites anywhere; one (`test_with_config`) is used by `tests/config_set_integration_tests.rs:4`. `subx-cli` re-exports all twelve at its root so no test import changes (Decision 10).

Tests, benches, assets and fixtures do **not** move — that is B3. `subx-cli`'s version stays `1.9.1` — the 2.0.0 bump is C1's. No CLI flag, configuration key, JSON envelope field, error variant, category string or machine code changes, and the binary's observable behaviour is identical.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `crate-topology`: B1 fixed the repository, workspace, submodule and versioning **structure** and explicitly deferred per-module ownership. B2 supplies it. Its "Repository Ownership and Submodule Mount" requirement is restated so its closing paragraph points at the new module-ownership rule instead of declaring the question out of scope. Seven requirements are added: which modules each crate owns and the prohibition on a crate holding a module that belongs to the other; the rule that `subx-core` SHALL NOT reference `subx-cli` in code or in a rustdoc link, in either direction of the doc-link graph; the public API path stability contract covering SDR §8's consumed surface, including the prohibition on flattening `subx_core::core::…`; the crate-root ownership of `#[macro_export]` macros and crate-level inner attributes; the feature pass-through contract; the back-compat re-export policy in `subx-cli` and its rustdoc-prose-only legacy marking; and the dependency allocation rule that every dependency is declared in the crate whose sources use it, with no zero-use-site entry in either manifest.

## Impact

- **Code:** 95 files (36,687 LOC) move from `subx-cli/src/` to `subx-core/src/` with byte-identical contents apart from the doctest `use` lines. `subx-core/src/lib.rs` is rewritten from B1's placeholder. `subx-cli/src/lib.rs` loses `pub mod {config, core, error, services}` (`:113-133`) and gains the D11 re-export block; `App` (`:190`) and its methods `new` (`:212`), `new_with_production_config` (`:236`), `run` (`:262`), `handle_command` (`:306`), `match_files` (`:340`), `convert_files` (`:382`), `sync_files` (`:440`), `sync_files_with_offset` (`:502`), `config_service` (`:527`), `get_config` (`:539`) are unchanged. 17 files under `src/cli/` and `src/commands/` have their `crate::{core,config,error,services}` paths rewritten to `subx_core::…` (98 lines). `src/main.rs` keeps its two `subx_cli::` paths (`cli::output`, `cli::RunOutcome`) and its `subx_cli::config::ProductionConfigService::new()` call, which resolves through the re-export. `src/cli/error_ext.rs` gains the relocated error tests. After this change `subx-cli/src/` contains exactly `cli/`, `commands/`, `lib.rs`, `main.rs`.
- **Tests:** No test file under `tests/` or `benches/` is moved and — with one exception — none is edited. All 90 files that `use subx_cli::…` keep compiling, because every first path segment they name (`config`, `core`, `cli`, `services`, `commands`, `Result`, `error`, `App`) is either still `subx-cli`'s own or is covered by a D11 re-export. The exception is `tests/config_set_integration_tests.rs:4`'s `use subx_cli::test_with_config;`, and even that is covered by re-exporting the twelve macros at `subx-cli`'s root, so the file is left untouched. A1's `tests/core_cli_boundary.rs` is rewritten to walk `subx-core/src/` from `CARGO_MANIFEST_DIR` and to also reject `subx_cli` and `` [`crate::cli` `` doc links. `src/error.rs`'s in-file test module is split between the two crates (see What Changes §7).
- **APIs:** *Added:* the entire `subx_core` public surface — `subx_core::{config, core, error, services}` and everything beneath them, `subx_core::{Config, ConfigService, EnvironmentProvider, ProductionConfigService, SystemEnvironmentProvider, TestConfigBuilder, TestConfigService, TestEnvironmentProvider, Result, VERSION}`, and twelve crate-root `#[macro_export]` macros. Every path under `subx_core::core::…`, `subx_core::config::…`, `subx_core::error::…` and `subx_core::services::…` is identical to today's `subx_cli::` path with the crate name swapped, so the GUI's migration is a global `subx_cli::` → `subx_core::` substitution plus one `Cargo.toml` line. *Changed in `subx-cli`:* `config`, `core`, `error` and `services` stop being inherent modules and become re-exports of another crate — source-compatible for path resolution, but a shape change that is the substance of SDR D6's 2.0.0 bump. *Unchanged:* `subx_cli::App` and every method on it, `subx_cli::VERSION`, `subx_cli::CORE_VERSION`, `subx_cli::cli::*`, `subx_cli::commands::*`.
- **Dependencies:** `subx-core` gains SDR §4's core set — thiserror, anyhow, serde, serde_json, toml, config, regex, encoding_rs, url, walkdir, uuid, futures, async-trait, tokio (`rt`, `sync`, `time`, `fs`, `macros`), reqwest, symphonia, voice_activity_detector, rubato, audioadapter-buffers, zip, tar, flate2, sevenz-rust2, unrar (optional), tempfile, dirs, num_cpus, log — plus `[dev-dependencies]` mockall and wiremock. `subx-cli` keeps subx-core, clap, clap_complete, colored, tabled, indicatif, env_logger, log, tokio (`rt-multi-thread`, `macros`, `time`), serde, serde_json, toml, **dirs**, and drops the other seventeen; it does **not** gain `anyhow`. `subx-cli` `[dev-dependencies]` gains tempfile and async-trait. `Cargo.lock` is regenerated by `cargo build`, never hand-edited; the total resolved package count should not change, since both crates draw from the same set.
- **Documentation:** `subx-core/README.md` loses its "placeholder" note and gains the real module map. `subx-core/AGENTS.md` gains the module-guide rows for `core/`, `services/`, `config/` and `error.rs`, and the rule that core may never name `subx-cli`. `AGENTS.md` (`:112-144` Module Guide, `:137` "Add/change CLI arguments") and `docs/tech-architecture.md` are updated to place each module in its crate. `CHANGELOG.md` gains `[Unreleased]` → `### Added`, `### Changed` and `### Removed` entries in both repositories. The full two-crate documentation rewrite is C3's; B2's job is only to stop the existing documents from being wrong.
- **Coverage:** the denominator does not change — `scripts/check_coverage.sh:369` already runs `cargo llvm-cov nextest --workspace` (since B1), and the same 36,687 LOC are measured, just under a different package name. The combined 75% floor therefore still applies unchanged and is expected to hold within rounding. Per-crate thresholds remain C1's. One asymmetry to watch, inherited from B1: `default-members` is unset, so `cargo clippy` and `cargo nextest run` still act on the root package alone; B2 verifies `cargo clippy --workspace -- -D warnings` by hand and leaves `scripts/quality_check.sh` for C1, exactly as B1 did.
