## Why

B3 split the test suite across the two crates and, while doing it, measured itself at ~14.5 h against a one-workday budget. Rather than overflow silently it named its own seam: the atomic half — the `test-support` mechanism, the `tests/common/` split, the 40-file move with fixtures, benches and assets, and the import rewrites — is work that cannot be deferred, because between B2 and it `subx-core` has no tests of its own and its `slow-tests` read fixtures that are not there. The other half is additive, touches no file the move touches, and can be verified in isolation.

This is series change **B4**, and it is that other half. It exists because the census B3 ran turned up three bodies of work that SDR §6 did not know were in scope, and none of them is a relocation:

1. **Twelve test files, 1,104 LOC, have never been compiled.** Cargo auto-discovers only `tests/*.rs`; `tests/cli/`, `tests/commands/`, `tests/parallel/` and `tests/sync/` are reached only through top-level `#[path = "…"]` harness shims, and only **15** shims exist (SDR originally recorded 25). Five files under `tests/cli/`, all five under `tests/commands/`, `tests/parallel/integration_tests.rs` and `tests/sync/integration_tests.rs` are wired to nothing. A2 found one of them — `tests/cli/input_handler_tests.rs` — and handed it forward; it is one instance of a class, not an isolated defect.

2. **The binary spawned by 17 test files is named by a runtime string, and three of those strings are wrong.** `tests/integration_tests.rs:24,38,58` calls `Command::cargo_bin("subx")`. No such target exists — `[[bin]] name = "subx-cli"` — so `unwrap()` would panic. It never has, because `test_full_workflow` (`:68`) is `#[ignore]`d. The other 23 sites name the binary correctly today and are wrong in the same way: by construction rather than by content.

3. **The 75% coverage floor is still one number over a workspace that now has two very different denominators.** `subx-core` holds 36,687 of 49,069 measured lines and inherits nearly all the tested surface; `subx-cli` is left with `src/cli/` (5,101) and `src/commands/` (6,283), much of it clap derives and terminal rendering, with `src/main.rs` excluded. A single floor over both is no longer a statement about either.

All three are genuinely independent of B3. The revivals touch files B3 leaves alone; the binary-name rewrites touch call sites in files that stay in `subx-cli` unmoved; and the coverage measurement cannot be taken until the move has landed anyway. B4 therefore runs after B3 and before C1, which needs the numbers.

**One allocation detail B3's census forced, and which is the only place the two changes interlock.** Three of the twelve orphans are core-bound (`tests/cli/input_handler_tests.rs`, `tests/parallel/integration_tests.rs`, `tests/sync/integration_tests.rs`). Flattening a file into `subx-core/tests/*.rs` *is* reviving it, because Cargo then auto-discovers it. B3 therefore moves the **40 live** core-bound files and leaves those three where they are; B4 moves and revives them together. B3's ownership census remains 43 core-bound / 80 CLI-bound — that is the classification and SDR §6 records it — but the physical move is 40 in B3 and 3 in B4.

## What Changes

**1. Eight never-compiled files are revived.** Each is wired to a harness shim or flattened to a top-level path, its imports are repaired, and then it is *run*:

| File | LOC | Destination | Repair |
|---|---|---|---|
| `tests/cli/config_args_tests.rs` | 65 | `subx-cli` | shim only |
| `tests/cli/detect_encoding_args_tests.rs` | 24 | `subx-cli` | shim only |
| `tests/cli/ui_tests.rs` | 35 | `subx-cli` | shim; `subx_cli::services::ai::AiUsageStats` → `subx_core::…`; confirm `cli::ui::display_ai_usage` survived A1 |
| `tests/cli/sync_manual_offset_integration_tests.rs` | 39 | `subx-cli` | shim; two `cargo_bin` sites converted here |
| `tests/commands/sync_command_manual_offset_tests.rs` | 107 | `subx-cli` | flatten to `tests/`; `subx_cli::config::test_service::…` → `subx_core::…` |
| `tests/cli/input_handler_tests.rs` | 108 | `subx-core` | A2's handoff; `crate::cli::InputPathHandler` → `subx_core::core::input::InputPathHandler` |
| `tests/parallel/integration_tests.rs` | 356 | `subx-core` | flatten; `subx_cli::` → `subx_core::` |
| `tests/sync/integration_tests.rs` | 267 | `subx-core` | flatten; `subx_cli::` → `subx_core::`; its flat `use common::{TestFileManager, AudioMockGenerator, …}` (`:16-17`) has never matched `common/mod.rs`'s shape and becomes `subx_core::test_support::…` |

**2. Four are deleted, and the criterion that separates them from the eight is stated once.** `tests/commands/{cache_command_tests, config_command_tests, detect_encoding_tests, sync_command_tests}.rs` import `create_test_cache_files`, `create_test_config` and `create_utf8_subtitle_file` from `common::command_helpers` — a module B3 deletes because it is three lines of `//!` and defines nothing. Reviving them means authoring three helpers and then debugging four sets of assertions that have never executed. That is writing new tests. The 1-byte `tests/config_basic_integration.rs` is deleted with them.

The triage rule: **a file is revived when its imports already resolve, or resolve after a rewrite this change is performing anyway; it is deleted when reviving it requires authoring code that does not exist.**

**3. The orphan class is closed by a check, not by a sweep.** `tests/core_cli_boundary.rs` gains an assertion that enumerates every `.rs` under the non-auto-discovered subdirectories of both crates' `tests/` trees and compares it against the set of `#[path]` targets declared by the top-level files, failing on any file in the first set that is not in the second. Once B4 lands, `subx-core/tests/` is flat and has no shims at all, and `subx-cli/tests/cli/` is fully covered.

**4. All 26 binary-spawn sites are resolved at compile time.** `Command::cargo_bin("subx-cli")` (23 sites across 16 files) and `Command::cargo_bin("subx")` (3 sites in `tests/integration_tests.rs`) become `Command::new(env!("CARGO_BIN_EXE_subx-cli"))`. Cargo defines that variable at compile time for every binary target of the package under test, so a renamed binary becomes a build failure rather than a runtime panic, no `unwrap()` is needed, and the path names the exact artefact built for this package — which matters now that two packages share a target directory. `#[ignore]` is lifted from `test_full_workflow` and the test is actually run.

**5. Coverage is measured per crate and the floors are derived, not chosen.** One instrumented `cargo llvm-cov nextest --workspace` run, reported per crate by source-path prefix, so that a `subx-cli` test exercising `subx-core` lines still counts toward core. Floors are ratchets set to `floor(measured − 3)`, clamped to minima of **75%** for `subx-core`, **65%** for `subx-cli`, with the combined workspace floor staying at **75%** as the gate that may not regress. B4 measures, derives and records; **C1 wires the numbers into `scripts/check_coverage.sh`, `scripts/check_coverage.ps1` and the CI environment.**

**6. Two coverage-tooling facts are handed to C1 in writing.** `.llvm-cov.toml` is referenced by no script, workflow or manifest in either repository — its `exclude-from-report` patterns have never been in force — so the real mechanism is a single `--ignore-filename-regex` on the `cargo llvm-cov report` invocation, and B4 specifies the regex. And `default-members` is still unset, so a bare `cargo nextest run` acts on the root package alone; after B3 that silently skips 43 test files. B4 verifies the hazard empirically and records it next to the numbers.

Nothing about the CLI's observable behaviour changes. No production source file is edited: the only `src/` change in either repository is `tests/core_cli_boundary.rs`, which is a test. `subx-cli` stays at `1.9.1`.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `cross-crate-testing`: B3 creates this capability and specifies the mechanism by which nested test directories are reached — that Cargo auto-discovers only `tests/*.rs`, that a subdirectory file needs exactly one harness shim, that shims hold only module declarations, and that a plain `mod common;` is preferred over a redundant `#[path]`. B3 deliberately stops there, because twelve orphan files still exist when it lands and a requirement it cannot satisfy would be a false claim. B4 restates **"Test Files Are Reached Through Harness Shims"** in full, carrying B3's three scenarios over verbatim and adding the enforcement half: the mechanical set-equality check between subdirectory contents and `#[path]` targets, the rule that an orphan is a defect to be wired, relocated or deleted rather than left in place, and the revive-versus-delete criterion. B4 additionally adds two requirements B3 does not carry at all, because B3 does not do the work they govern: **"The Binary Under Test Is Named at Compile Time"** and **"Per-Crate Coverage Floors Over a Single Instrumented Run"**.

## Impact

- **Code:** No production source file in either repository is edited. `tests/core_cli_boundary.rs` gains one assertion. Eight test files are wired up or relocated and repaired; five files (four tests plus one 1-byte placeholder) are deleted. `tests/commands/`, `tests/parallel/` and `tests/sync/` cease to exist in `subx-cli`; `subx-core/tests/` remains flat.
- **Tests:** 26 `Command::cargo_bin(…)` call sites across 17 files become `Command::new(env!("CARGO_BIN_EXE_subx-cli"))`. `tests/integration_tests.rs:68`'s `#[ignore]` is removed. Eight previously uncompiled files enter the suite — 5 in `subx-cli`, 3 in `subx-core` — adding roughly 1,001 LOC of test code that has never run, some of which is expected to need its assertions updated to characterise current behaviour. Where a revived test cannot pass without a production change, it is deleted and the finding is recorded rather than fixed here.
- **APIs:** None. B4 adds, removes and changes no public item in either crate.
- **Dependencies:** None. B3 already reallocated every `[dev-dependencies]` entry by use site and deleted the three with none.
- **Documentation:** `AGENTS.md` in both repositories gains the `CARGO_BIN_EXE_subx-cli` contract, the orphan-file rule and the revive-versus-delete criterion, and `AGENTS.md:47`'s bare "Coverage threshold is **75%**" becomes the three derived numbers. `docs/tech-architecture.md`'s coverage paragraph (`:627-631`) is updated for two crates. `CHANGELOG.md` gains `[Unreleased]` entries in both repositories under `### Added`, `### Fixed` and `### Removed`.
- **Coverage:** the instrumented run stays a single `--workspace` invocation; only the report is split. B4 produces two measured numbers and two derived floors, plus the `--ignore-filename-regex` value `(^|/)(tests|benches)/|(^|/)src/main\.rs$|(^|/)src/test_support/` — the last term because `src/test_support/`'s own coverage is meaningless and would otherwise inflate `subx-core`'s numerator. The eight revived files move both numbers upward by an unknown amount, which is precisely why the measurement is taken after the revivals rather than before.
