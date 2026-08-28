## 1. Baseline

- [ ] 1.1 Confirm B3 landed: `subx-core/tests/` holds the 40 moved files plus `fixtures/formats/` (22 files), `subx-core/benches/` holds `retry_performance.rs` and `file_id_generation_bench.rs`, `subx-core/assets/` holds the `.mp4`, `.mp3` and `.srt`, and `tests/common/` holds exactly `mod.rs`, `cli_helpers.rs` and `json_output.rs`
- [ ] 1.2 Confirm `cargo nextest run --workspace` is green and `cargo bench --no-run --workspace` builds both benches before editing anything
- [ ] 1.3 Re-derive the orphan set: list every `.rs` under `tests/cli/`, `tests/commands/`, `tests/parallel/`, `tests/sync/` and subtract the set of `#[path = "…"]` targets declared by `tests/*.rs`; confirm the **12** files named in design.md Decision 1 and that none of them is compiled by any target
- [ ] 1.4 Re-derive the spawn sites: `grep -rn 'cargo_bin' tests/ subx-core/tests/` and confirm **26** hits across 17 files — 23 naming `"subx-cli"` and 3 naming `"subx"` at `tests/integration_tests.rs:24,38,58`
- [ ] 1.5 Confirm `tests/common/command_helpers.rs` no longer exists (B3 deleted it) and that `create_test_cache_files`, `create_test_config` and `create_utf8_subtitle_file` have no definition anywhere in `tests/` outside `TestFileManager::create_test_config`
- [ ] 1.6 Record the current workspace coverage percentage from the last known-good CI run as the pre-revival baseline; do **not** run `scripts/check_coverage.sh` here — it is reserved for phase 8 and the quality gate

## 2. Delete the Unrevivable

- [ ] 2.1 Re-grep for each of `create_test_cache_files`, `create_test_config` and `create_utf8_subtitle_file` to confirm no definition exists, then `git rm tests/commands/cache_command_tests.rs`, `tests/commands/config_command_tests.rs`, `tests/commands/detect_encoding_tests.rs`, `tests/commands/sync_command_tests.rs`
- [ ] 2.2 `git rm tests/config_basic_integration.rs` (a single newline, which Cargo compiles into an empty test crate)
- [ ] 2.3 Run `cargo nextest run --workspace` and confirm the result is unchanged — none of the five had a target

## 3. Revive the Five CLI-Side Files

- [ ] 3.1 Add `tests/cli_config_args_harness_tests.rs` containing only a `//!` header and `#[path = "cli/config_args_tests.rs"] mod config_args_tests;`, then run `cargo nextest run --filter-expr 'test(config_args)' || true`
- [ ] 3.2 Add `tests/cli_detect_encoding_args_harness_tests.rs` with `#[path = "cli/detect_encoding_args_tests.rs"] mod detect_encoding_args_tests;` and run it
- [ ] 3.3 Add `tests/cli_ui_harness_tests.rs` with `#[path = "cli/ui_tests.rs"] mod ui_tests;`; rewrite `tests/cli/ui_tests.rs:3`'s `use subx_cli::services::ai::AiUsageStats;` to `use subx_core::services::ai::AiUsageStats;`, and confirm `subx_cli::cli::ui::{create_progress_bar, display_ai_usage}` and `subx_cli::cli::table::{MatchDisplayRow, create_match_table}` still exist after A1 moved AI-usage printing behind `Reporter` — if `display_ai_usage` is gone, delete `test_display_ai_usage_outputs` and record it in the CHANGELOG
- [ ] 3.4 Add `tests/cli_sync_manual_offset_harness_tests.rs` with `#[path = "cli/sync_manual_offset_integration_tests.rs"] mod sync_manual_offset_integration_tests;` and run it; its two `Command::cargo_bin("subx-cli")` sites (`:17`, `:33`) are converted in phase 6
- [ ] 3.5 Move `tests/commands/sync_command_manual_offset_tests.rs` to `tests/sync_command_manual_offset_tests.rs` (flattening it makes it auto-discovered and needs no shim), rewrite `use subx_cli::config::test_service::TestConfigService;` to `use subx_core::config::test_service::TestConfigService;`, and run it
- [ ] 3.6 Delete the now-empty `tests/commands/` directory
- [ ] 3.7 Run each revived file; where an assertion fails because behaviour has legitimately changed, update the **assertion** to characterise current behaviour — never change production code to satisfy a test that has never executed. If a file cannot pass without a production change, `git rm` it and record the finding for the CHANGELOG

## 4. Move and Revive the Three Core-Side Files

- [ ] 4.1 Move `tests/cli/input_handler_tests.rs` (108 LOC — A2's explicit handoff) to `subx-core/tests/input_handler_tests.rs`, rewriting `use crate::cli::InputPathHandler;` → `use subx_core::core::input::InputPathHandler;` and `use crate::error::SubXError;` → `use subx_core::error::SubXError;`; run `cargo nextest run -p subx-core --filter-expr 'test(input_handler)' || true`
- [ ] 4.2 Move `tests/parallel/integration_tests.rs` (356 LOC) to `subx-core/tests/parallel_integration_tests.rs`; rewrite `subx_cli::core::parallel::{TaskScheduler, WorkerPool}`, `subx_cli::core::parallel::task::{Task, TaskResult, TaskStatus, ProcessingOperation, FileProcessingTask}` and `subx_cli::core::parallel::scheduler::TaskPriority` to `subx_core::…`; keep its `#![allow(unused_imports, dead_code)]` inner attribute, which is legal at the top of an auto-discovered crate root; run it
- [ ] 4.3 Delete the now-empty `tests/parallel/` directory
- [ ] 4.4 Move `tests/sync/integration_tests.rs` (267 LOC) to `subx-core/tests/sync_integration_tests.rs`; rewrite `subx_cli::core::sync::{SyncEngine, dialogue::DialogueDetector, engine::{SyncConfig, SyncMethod}}`, `subx_cli::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata, manager::FormatManager}`, `subx_cli::services::audio::generate_dialogue_audio` and `subx_cli::config::TestConfigBuilder` to `subx_core::…`
- [ ] 4.5 Replace `subx-core/tests/sync_integration_tests.rs:16-17`'s broken `mod common; use common::{TestFileManager, AudioMockGenerator, SubtitleGenerator, SubtitleFormat};` — a flat import that has never matched `tests/common/mod.rs`'s module-re-export shape — with `use subx_core::test_support::{file_managers::TestFileManager, mock_generators::{AudioMockGenerator, SubtitleGenerator, SubtitleFormat}};`, then run it
- [ ] 4.6 Delete the now-empty `tests/sync/` directory
- [ ] 4.7 Apply task 3.7's rule to these three as well; `sync_integration_tests.rs` is the largest and least certain of the eight, so if it exceeds its share it is deleted under the same criterion rather than debugged
- [ ] 4.8 Confirm `grep -rn 'subx_cli' subx-core/tests/` returns zero and `cargo nextest run --workspace` is green

## 5. Close the Orphan Class

- [ ] 5.1 Add a fifth assertion to `tests/core_cli_boundary.rs` (created by A1, repointed by B2, widened by B3): enumerate every `.rs` under the non-auto-discovered subdirectories of `subx-cli/tests/` and `subx-core/tests/`, resolved from `env!("CARGO_MANIFEST_DIR")` and never from the working directory; collect every `#[path = "…"]` target declared by files matching `tests/*.rs` in both trees; assert the first set is a subset of the second and fail with the `file:line`-style list of any file that is not
- [ ] 5.2 Confirm the check reports zero: after phases 2–4 the only non-auto-discovered subdirectory left in either repository is `subx-cli/tests/cli/`, and every file in it now has exactly one shim
- [ ] 5.3 Confirm `subx-core/tests/` is flat — no subdirectories other than `fixtures/` — and therefore has no shims at all
- [ ] 5.4 Verify the check actually fails when it should: temporarily add an empty `tests/cli/zz_probe.rs`, confirm the assertion fails and names it, then delete the probe

## 6. The Binary-Name Contract

- [ ] 6.1 Replace the 23 `Command::cargo_bin("subx-cli").unwrap()` call sites with `Command::new(env!("CARGO_BIN_EXE_subx-cli"))` across `tests/cli_integration_tests.rs:8,17,28`, `tests/cli/config_set_repair_invalid.rs:82`, `tests/cli/match_command_json_silence.rs:103`, `tests/cli/output_format_cache.rs:51`, `tests/cli/output_format_clap_errors.rs:35`, `tests/cli/output_format_config.rs:48`, `tests/cli/output_format_convert.rs:20`, `tests/cli/output_format_cross_command.rs:35`, `tests/cli/output_format_detect_encoding.rs:72,231`, `tests/cli/output_format_generate_completion.rs:40,89,108`, `tests/cli/output_format_jq.rs:56`, `tests/cli/output_format_match.rs:142`, `tests/cli/output_format_quiet.rs:28`, `tests/cli/output_format_translate.rs:75,115`, `tests/cli/output_format_translate_success.rs:92`, `tests/cli/sync_manual_offset_integration_tests.rs:17,33`, plus any site A1 added in `tests/cli/ai_usage_output_characterization.rs` or `tests/cli/translation_progress_characterization.rs`
- [ ] 6.2 Replace the three `Command::cargo_bin("subx").unwrap()` sites at `tests/integration_tests.rs:24,38,58` the same way — the name has never resolved and the `unwrap()` has never panicked only because `test_full_workflow` (`:68`) is `#[ignore]`d
- [ ] 6.3 Remove the `#[ignore]` from `tests/integration_tests.rs:68` and run `test_full_workflow`; if it fails on an assertion about behaviour that has changed, update the assertion; if it cannot pass without a production change, restore `#[ignore]` **with a comment naming the reason** and record the finding
- [ ] 6.4 Confirm `grep -rn 'cargo_bin' tests/ subx-core/tests/` returns zero hits in both repositories, and that no `use assert_cmd::Command;` import was left unused
- [ ] 6.5 Run the full `assert_cmd` set: `cargo nextest run --workspace --filter-expr 'test(output_format) + test(cli_integration) + test(integration_tests) + test(config_set_repair) + test(json_silence) + test(sync_manual_offset)' || true`

## 7. Verify

- [ ] 7.1 `cargo build --workspace` and `cargo build --release` at the workspace root
- [ ] 7.2 `cargo nextest run --workspace` — green; then `cargo nextest run --workspace --features slow-tests` — green
- [ ] 7.3 `cargo bench --no-run --workspace`
- [ ] 7.4 `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] 7.5 `cargo doc --no-deps --all-features` in both crates and `cargo test --doc --all-features`
- [ ] 7.6 Clone `subx-core` standalone into a scratch directory **outside** the `subx-cli` tree and run `cargo test` there, confirming the three newly moved files compile and pass without any `--features` flag on the command line
- [ ] 7.7 Commit the `subx-core` change and the `subx-cli` change carrying the moved gitlink together

## 8. Coverage Measurement and Per-Crate Floors

- [ ] 8.1 Run `scripts/check_coverage.sh -T -p ci --full` **once** (main agent only — it saturates all cores and must never be invoked from a sub-agent or in parallel) and capture the per-file JSON summary
- [ ] 8.2 Group the per-file summaries by source-path prefix and record two numbers: `subx-core/src/**` and `src/**`; confirm the combined workspace figure is at or above 75% and has not moved materially from task 1.6's baseline other than by the eight revived files' contribution
- [ ] 8.3 Derive each floor as `floor(measured − 3)`, clamped to a minimum of **75** for `subx-core` and **65** for `subx-cli`; record both the measured value and the derived floor for each crate
- [ ] 8.4 Record the exclusion regex the report needs — `(^|/)(tests|benches)/|(^|/)src/main\.rs$|(^|/)src/test_support/` — noting that `src/test_support/` is B4's addition because B3 moved 1,529 LOC of mock helpers into `subx-core/src/` whose own coverage is meaningless and would inflate core's numerator
- [ ] 8.5 Record the `.llvm-cov.toml` finding for C1: `grep -rn 'llvm-cov.toml' scripts/ .github/ Cargo.toml subx-core/Cargo.toml` returns zero in both repositories, so its `exclude-from-report` patterns have never been in force and the real mechanism is `--ignore-filename-regex` on the `cargo llvm-cov report` invocation; C1 decides whether both copies are deleted or made load-bearing
- [ ] 8.6 Verify the `default-members` hazard concretely: run `cargo nextest run` **without** `--workspace` and confirm it executes only `subx-cli`'s tests, skipping all 43 files in `subx-core/tests/`
- [ ] 8.7 Write the complete C1 handoff list into the CHANGELOG entry and `design.md`'s Open Questions if anything changed during measurement: per-crate `COVERAGE_THRESHOLD_CORE` / `COVERAGE_THRESHOLD_CLI` plumbing in `scripts/check_coverage.sh` and `scripts/check_coverage.ps1`, the CI environment variables at `.github/workflows/build-test-audit-coverage.yml:186,195`, the `--ignore-filename-regex` value, the two `.llvm-cov.toml` files, the two junit paths (B1 Decision 5a), and `default-members`

## 9. Documentation

- [ ] 9.1 Add the orphan-file rule to `AGENTS.md` and `subx-core/AGENTS.md`: Cargo auto-discovers only `tests/*.rs`, a file under a subdirectory needs exactly one top-level `#[path]` shim, and `tests/core_cli_boundary.rs` fails the build if one is missing
- [ ] 9.2 Add the revive-versus-delete criterion to both `AGENTS.md` files — revive when the imports already resolve or resolve after a rewrite already under way, delete when reviving would require authoring code that does not exist — together with the characterise-do-not-repair rule for a revived test with a stale assertion
- [ ] 9.3 Add the `CARGO_BIN_EXE_subx-cli` contract to `AGENTS.md`, stating that `Command::cargo_bin("…")` is prohibited and why (a wrong name becomes a compile error rather than a runtime panic, and the path names the artefact built for this package in a two-package workspace)
- [ ] 9.4 Replace `AGENTS.md:47`'s bare "Coverage threshold is **75%** line coverage" with the three numbers — workspace 75%, `subx-core` and `subx-cli` at their derived floors — and state the ratchet rule (`floor(measured − 3)`, may be raised, never lowered without a proposal)
- [ ] 9.5 Update `docs/tech-architecture.md`'s coverage paragraph (`:627-631`) for the per-crate floors and the single-instrumented-run/split-reporting mechanism
- [ ] 9.6 Add `[Unreleased]` entries to `CHANGELOG.md` in **both** repositories: `### Added` — the orphan-file check in `tests/core_cli_boundary.rs`, and the per-crate coverage floors with their measured values; `### Fixed` — eight test files that had never been compiled now run (five in `subx-cli`, three in `subx-core`), `tests/integration_tests.rs`'s three `Command::cargo_bin("subx")` calls naming a binary target that has never existed, and `test_full_workflow` no longer skipped; `### Changed` — all 26 binary spawn sites now resolve through `env!("CARGO_BIN_EXE_subx-cli")` instead of a runtime name lookup; `### Removed` — four `tests/commands/` files whose helper functions never existed, and the 1-byte `tests/config_basic_integration.rs`
- [ ] 9.7 Record in the CHANGELOG any test deleted under task 3.7, 4.7 or 6.3's rule, naming what it asserted, so the coverage it represented is not silently lost

## 10. Quality Gate

- [ ] 10.1 Run `cargo fmt` and `cargo clippy -- -D warnings` and fix all warnings
- [ ] 10.2 Run `cargo nextest run --workspace --filter-expr 'test(core_cli_boundary) + test(input_handler) + test(parallel_integration) + test(sync_integration) + test(config_args) + test(detect_encoding_args) + test(ui_tests) + test(sync_manual_offset) + test(sync_command_manual_offset) + test(full_workflow) + test(output_format)' || true` and confirm the targeted modules pass
- [ ] 10.3 Run `scripts/quality_check.sh` once at the end (main agent only — do not invoke from sub-agents) and ensure it is green
- [ ] 10.4 Run `cargo test --doc --all-features` to confirm rustdoc examples still compile
