## Why

B2 moved 36,687 LOC of library code into `subx-core` and left the entire test suite behind. That was deliberate and it worked — B2's Decision 9 proved that every first path segment the test suite imports from `subx_cli` is covered by the D11 back-compatibility re-exports, so `cargo nextest run` stayed green across the B2 boundary with **zero** test edits. The price is that `subx-core` today ships 36,687 LOC of library code whose integration tests all live in another repository and reach it through a legacy alias. A standalone clone of `subx-core` runs only the `#[cfg(test)]` modules inside `src/**`; its `tests/` directory is empty; its `benches/` directory does not exist; and the fixtures its own slow-tests read are not there.

This is series change **B3**, and it is the change that gives each crate the tests that belong to it.

It is also the change the implementation plan (§6, hazard 1) names as the one most likely to overflow its budget, on the grounds that `tests/common/` — 13 files, 2,670 LOC, consumed by 40+ test files through two different inclusion mechanisms — has no cross-repository sharing mechanism in Cargo. That warning is correct about the shape of the problem. It is wrong about the size, and so is SDR §6's file-count arithmetic. Both were derived from greps rather than from the code, and this change re-derived them from the code.

**Six findings change what B3 actually is.** Each is reproducible from the current tree.

1. **The 89/17 split is wrong. The real split is 42 / 81.** SDR §6 classifies a file as core-bound if it contains `use subx_cli::…`. But `subx_cli::cli::` and `subx_cli::commands::` are *also* `subx_cli::` — and those two modules stay in `subx-cli` permanently (SDR D7/D8). **39 of the 89 files import `subx_cli::cli::` or `subx_cli::commands::`**; they construct clap `*Args` structs and call `match_command::execute` and friends. They are CLI integration tests and they cannot move. Only **42 files (6,826 LOC)** import library internals exclusively.

2. **Eight more files drive the binary without `assert_cmd`.** `CLITestHelper::run_command_with_config` (`tests/common/cli_helpers.rs:233`) spawns `cargo run --` from `env!("CARGO_MANIFEST_DIR")`. Seven top-level `sync_*` files and `tests/cli/output_format_sync.rs` use it. They import no library internals at all, so SDR's grep missed them in both directions — but they are binary-driving tests and they stay in `subx-cli`. Their existence, not `assert_cmd`, is the real reason a helper has to stay CLI-side.

3. **`CLITestHelper` is not `assert_cmd`-based.** SDR §6's resolution says "the `assert_cmd`-based `CLITestHelper` stays in `subx-cli/tests/common/`". `cli_helpers.rs` does not mention `assert_cmd`. It is a `TempDir` + `TestConfigService` fixture builder with one bolted-on `impl` block that shells out to Cargo. The correct seam runs *through* the file, not around it.

4. **Four of the thirteen `tests/common/` modules have no consumers at all**, and a fifth has consumers but no functions. `validators.rs` (274 LOC), `parallel_helpers.rs` (97), `sync_helpers.rs` (78) and `integration_test_macros.rs` (41) are referenced by nothing. `command_helpers.rs` is three lines of module documentation and defines nothing, yet four files import three functions from it. That is **493 LOC — 18% of `tests/common/` — that does not need a home on either side.**

5. **Twelve test files have never been compiled.** Cargo does not auto-discover `tests/cli/`, `tests/commands/`, `tests/parallel/` or `tests/sync/`, and only 15 harness shims exist (SDR §6 says 25). Five files under `tests/cli/`, all five under `tests/commands/`, `tests/parallel/integration_tests.rs` and `tests/sync/integration_tests.rs` — **1,104 LOC** — are wired to nothing. A2 found one of them (`tests/cli/input_handler_tests.rs`) and explicitly handed it to B3; it is not an isolated defect but an instance of a class.

6. **B2 left two latent breakages that only B3 can close.** `src/core/formats/tests_support.rs:63` resolves fixtures from `CARGO_MANIFEST_DIR/tests/fixtures/formats/`, and `src/services/vad/audio_loader.rs:33` reads `assets/SubX - The Subtitle Revolution.mp4` from the working directory. Both files are now in `subx-core`, where neither path exists. The first is invisible because it is `slow-tests`-gated; the second fails whenever `cargo test -p subx-core` is run from the workspace root.

`.llvm-cov.toml` is a seventh finding of a different kind: **nothing reads it.** No script, workflow or manifest references the file, and `scripts/check_coverage.sh` passes every flag on the command line. Its `exclude-from-report` patterns have never been in force, so "make them work across two crates" is not a problem this change has to solve — it is a problem this change has to stop pretending exists. B4 records the finding and the replacement mechanism with its coverage numbers; C1 acts on it.

## What Changes

**1. Tests are allocated by what they exercise, not by what they import.** Four classes, one rule each:

| Class | Files | LOC | Destination | Rule |
|---|---|---|---|---|
| Binary-driving (`assert_cmd`) | 17 | 3,217 | `subx-cli/tests/` | spawns the binary |
| Binary-driving (`cargo run`) | 8 | 1,488 | `subx-cli/tests/` | spawns the binary |
| CLI-library | 39 | 8,605 | `subx-cli/tests/` | names `subx_cli::cli` or `subx_cli::commands` |
| Core-library | 42 | 6,826 | `subx-core/tests/` | names only `config`/`core`/`error`/`services`/`Result` |
| Harness shims | 15 | 91 | follow their target | |
| Shared helpers | 13 | 2,670 | split (see 2) | |
| Unclassifiable | 2 | 109 | see 5 | `tests/cli/input_handler_tests.rs` and the 1-byte `tests/config_basic_integration.rs` |

A2 shifts one file across the line: after `InputPathHandler` lands in `subx_core::core::input`, `tests/archive_input_extraction_tests.rs` (its only import) becomes core-library, making the final ownership counts **43 core / 80 CLI**. Twelve of those 123 files have never been compiled; B3 physically moves the **40 live** core-bound ones and B4 takes the rest (see item 5). A1 adds two `assert_cmd` files and two shims, and `tests/core_cli_boundary.rs`, which stays in `subx-cli` because it walks `subx-core/src/` from the outside.

The 40 moving files have every `subx_cli::` rewritten to `subx_core::`. The 71 live staying files have every `subx_cli::{config, core, error, services}` rewritten to `subx_core::…` **too** — so that after B3 no test in either repository resolves through a D11 re-export.

**2. `tests/common/` is split three ways, not two.** Measured by consumer class:

| Module | LOC | Core consumers | CLI consumers | Destination |
|---|---|---|---|---|
| `cli_helpers.rs` (fixture half) | ~400 | 2 | 9 | `subx_core::test_support::workspace` |
| `cli_helpers.rs` (`run_command_*` impl) | ~100 | 0 | 9 | `subx-cli/tests/common/cli_helpers.rs` |
| `file_managers.rs` | 259 | 4 | 0 | `subx_core::test_support::file_managers` |
| `mock_openai_helper.rs` | 324 | 1 | 11 | `subx_core::test_support::mock_openai` |
| `mock_azure_openai_helper.rs` | 404 | 4 | 0 | `subx_core::test_support::mock_azure_openai` |
| `mock_generators.rs` | 467 | 1 | 1 | `subx_core::test_support::mock_generators` |
| `test_data_generators.rs` | 75 | 5 | 8 | `subx_core::test_support::responses` |
| `json_output.rs` | 133 | 0 | 4 | `subx-cli/tests/common/json_output.rs` |
| `validators.rs` | 274 | 0 | 0 | **deleted** |
| `parallel_helpers.rs` | 97 | 0 | 0 | **deleted** |
| `sync_helpers.rs` | 78 | 0 | 0 | **deleted** |
| `integration_test_macros.rs` | 41 | 0 | 0 | **deleted** |
| `command_helpers.rs` | 3 | 0 | 4 (all dead) | **deleted** |

`subx-core` gains `src/test_support/` behind a new `test-support` feature. `subx-cli` enables it with a second `subx-core` declaration under `[dev-dependencies]`; `subx-core`'s own integration tests reach it through a path-only self dev-dependency. Both mechanisms are verified — the feature is on for `cargo test` in both crates and off for `cargo build --release`, and `cargo package` strips the self dev-dependency from the published manifest (`design.md` Decision 2).

**3. `CLITestHelper` is renamed to `TestWorkspace` and loses its process-spawning half.** The remaining fixture builder needs nothing but `subx_core::{Result, config::{ConfigService, TestConfigService}}` — all of which SDR D10 already made unconditional public API. The `run_command_*` methods become an extension trait `CliRun` in `subx-cli/tests/common/cli_helpers.rs`, because `env!("CARGO_MANIFEST_DIR")` must expand to `subx-cli`'s root for `cargo run --` to find the binary.

**4. Both benches move to `subx-core/benches/`** with their `[[bench]]` declarations, and `subx-cli`'s `[[bench]]` tables are deleted. `retry_performance.rs` imports `subx_cli::services::ai::retry`; `file_id_generation_bench.rs` has no crate coupling but belongs with the matcher.

**5. Twelve never-compiled files are identified here and triaged by B4.** The census that produced the classification also produced the orphan list, and it is recorded in SDR §6 and in this change's `design.md` so the next reader does not have to re-derive it. B3 does not wire them up: eight need reviving, four need deleting, and the criterion separating them is a judgement this change has no budget left to make. Two consequences follow for B3 itself:

- **B3 moves the 40 *live* core-bound files, not all 43.** Three of the orphans are core-bound — `tests/cli/input_handler_tests.rs` (A2's handoff), `tests/parallel/integration_tests.rs`, `tests/sync/integration_tests.rs` — and flattening a file into `subx-core/tests/*.rs` is itself reviving it, because Cargo auto-discovers everything there. B3 cannot move them without doing B4's work, and cannot move them unflattened without importing dead code into a fresh repository. They stay where they are; B4 moves and revives them together. The ownership census remains 43/80 and SDR §6 records it — only the physical move is split 40/3.
- **B3 does not add the orphan-file check.** Twelve files are still unwired when this change lands, so the assertion would ship red. B4 adds it to `tests/core_cli_boundary.rs` after triaging them.

B3 does delete `tests/common/command_helpers.rs`, because `tests/common/` is this change's to reorganise and leaving an undeclared module behind would create exactly the orphan A0 deleted from `src/cli/validation.rs`. Its four dead consumers go with B4.

**6. Fixtures and assets stop depending on the working directory.** `tests/fixtures/formats/**` (22 files) moves to `subx-core/tests/fixtures/formats/`, and B1 already shipped the `-text` `.gitattributes` rule there so the CRLF and BOM bytes survive the `git add`. The four media assets move to `subx-core/assets/`, and every reader — including `src/services/vad/audio_loader.rs:33`, which B2 broke — resolves them from `CARGO_MANIFEST_DIR`. The eight binary-driving tests that stay in `subx-cli` read `subx-cli`'s own copy of the `.srt`, which is 3.4 KB and is duplicated rather than reached across the submodule boundary (`design.md` Decision 5).

**7. The 26 runtime binary-name lookups are handed to B4.** `Command::cargo_bin("subx-cli")` appears at 23 sites and `Command::cargo_bin("subx")` at three — `tests/integration_tests.rs:24,38,58`, a target name that has never existed and whose `unwrap()` has never panicked only because `test_full_workflow` is `#[ignore]`d. All 26 become `Command::new(env!("CARGO_BIN_EXE_subx-cli"))` in B4. None of the affected files is moved by B3, so the two changes touch different parts of them and do not conflict.

**8. Feature gates and dev-dependencies are re-derived from the post-split composition.** `slow-tests` keeps a real gate on both sides — five `#[cfg]` sites in `subx-core/src/core/formats/**` plus two moving test files, and three staying test files — so B2's pass-through-that-is-also-a-real-feature arrangement is preserved unchanged. `tokio-test`, `rstest` and `test-case` are deleted from `subx-cli`'s `[dev-dependencies]`: they have zero use sites anywhere in `src/`, `tests/` or `benches/`, and SDR §4's plan to move them to `subx-core` would plant three entries that violate B2's own "no zero-use-site entry" rule.

**9. Coverage measurement is handed to B4, and one hazard is documented here.** The floors cannot be derived until the suite has reached its final composition, and B4's eight revivals change it. What B3 does document, because B3 is what creates it, is that `default-members` is unset: a bare `cargo nextest run` acts on the root package alone and now silently skips all 43 files under `subx-core/tests/`. The `--workspace` workaround goes into both `AGENTS.md` files; the normative coverage contract and the numbers are B4's, and the tooling that enforces them is C1's.

Nothing about the CLI's observable behaviour changes: no flag, no configuration key, no JSON envelope field, no error variant, no category string, no machine code. `subx-cli` stays at `1.9.1`; the 2.0.0 bump is C1's.

## Capabilities

### New Capabilities

- `cross-crate-testing`: how a two-crate, two-repository project decides which crate owns a test, how test helpers are shared across a repository boundary that Cargo has no mechanism for, how a test file in a non-auto-discovered subdirectory is given a target, how fixtures and media assets are located without depending on the working directory, which crate owns a test feature gate, and how dev-dependencies are allocated once the suite is split. Two further requirements belong to this capability and are added by `harden-split-test-suite` (B4), which does the work they govern: the compile-time binary-name contract, and the per-crate coverage floors. B4 also extends this change's harness-shim requirement with its enforcement half.

### Modified Capabilities

_None._

## Impact

- **Code:** `subx-core` gains `src/test_support/` (7 modules, ~1,930 LOC relocated from `tests/common/`), `tests/` (43 files, ~7,300 LOC after the revivals), `tests/fixtures/formats/**` (22 files), `benches/` (2 files, 151 LOC) and `assets/` (4 files, 11.3 MB). `subx-cli` keeps `tests/` (80 files) and retains `tests/common/` with two modules (`cli_helpers.rs` reduced to `CommandResult` + the `CliRun` extension trait, and `json_output.rs`). `src/services/vad/audio_loader.rs:33` and `src/core/formats/tests_support.rs:63` — both now in `subx-core` — are corrected to resolve from `CARGO_MANIFEST_DIR`. 493 LOC of unreferenced helpers and 104 LOC of unrevivable tests are deleted.
- **Tests:** 40 live core-bound files move to `subx-core/tests/` and have `subx_cli::` → `subx_core::` applied. 71 live CLI-bound files stay; those among them that name `subx_cli::{config, core, error, services}` are rewritten to `subx_core::…` so the D11 re-exports lose their last in-repository consumer. `tests/core_cli_boundary.rs` (A1, repointed by B2) gains a fourth assertion: no file under either crate's `tests/` or `benches/` may name `subx_cli::{config, core, error, services}`, `subx_cli::Result`, or any of the twelve re-exported macros. Twelve never-compiled files are left untouched for B4, three of them core-bound and therefore not moved by B3.
- **APIs:** *Added to `subx-core`, feature-gated behind `test-support`:* `subx_core::test_support::{workspace::{TestWorkspace, OutputValidator, ValidationResult}, file_managers::TestFileManager, mock_openai::MockOpenAITestHelper, mock_azure_openai::MockAzureOpenAITestHelper, mock_generators::{AudioMockGenerator, SubtitleGenerator, DialogueSegment, AudioMetadata, SubtitleFormat, SubtitleEntry}, responses::MatchResponseGenerator}`. The module carries `#[allow(missing_docs)]` so that test scaffolding does not dilute the crate's rustdoc contract; `broken_intra_doc_links = "deny"` still applies to it. *Unchanged:* every non-gated path in both crates. *Deprecated in practice, not in code:* the D11 re-exports and the twelve macro re-exports survive B3 for the out-of-tree GUI, but lose every in-repository consumer; deleting them is a `subx-cli` major-version event outside this series.
- **Dependencies:** `subx-core` `[dependencies]` gains `wiremock` and `hound` as **optional** entries activated by `test-support` (`dep:wiremock`, `dep:hound`), which is what lets the mock helpers live in `src/` without weighing on a normal build. `subx-core` `[dev-dependencies]` gains `criterion` (2 benches), `pretty_assertions` (`format_roundtrip_tests.rs`), `hound`, `wiremock`, and the path-only self declaration `subx-core = { path = ".", features = ["test-support"] }`. `subx-cli` `[dev-dependencies]` gains `subx-core = { path = "subx-core", version = "1.0", features = ["test-support"] }`, `wiremock` (4 files) and `regex` (2 files); loses `criterion`, `pretty_assertions`, `hound`, `mockall`; and drops `tokio-test`, `rstest` and `test-case`, which have zero use sites anywhere. `subx-cli` loses both `[[bench]]` tables; `subx-core` gains them.
- **Documentation:** `AGENTS.md` in both repositories gains a "which crate does my test belong to" decision rule, the `test-support` mechanism, the harness-shim requirement for nested directories, the `CARGO_MANIFEST_DIR` rule for every fixture and asset read, and the `--workspace` warning that `default-members` now makes necessary. `AGENTS.md:47`'s coverage number is left alone for B4 to replace. `docs/tech-architecture.md`'s testing section is re-scoped to two crates. `CHANGELOG.md` gains `[Unreleased]` entries in both repositories under `### Added`, `### Changed`, `### Fixed` and `### Removed`. The full two-crate documentation rewrite remains C3's.
- **Coverage:** no floor changes here. The instrumented run stays a single `cargo llvm-cov nextest --workspace`, the same 49,069 lines are measured, and the combined 75% floor applies unchanged — B3 moves no production line, so the combined figure should not move. Per-crate measurement, the floors, the `--ignore-filename-regex` value and the `.llvm-cov.toml` finding are B4's; the script, CI and `default-members` plumbing remains C1's.
