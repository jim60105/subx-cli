## 1. Relocate the UUIDv7 generator and rename to `Uuidv7Generator`

- [x] 1.1 Move `src/core/translation/uuidv7.rs` to `src/core/uuidv7.rs` and add `pub mod uuidv7;` to `src/core/mod.rs`.
- [x] 1.2 Rename the struct `CueIdGenerator` to `Uuidv7Generator` in the relocated file; rename the public free function `generate_cue_ids` to `generate_ids`. Update rustdoc to describe the broader (matcher + translation) responsibility.
- [x] 1.3 In `src/core/translation/mod.rs`, remove the `pub mod uuidv7;` declaration and add `pub use crate::core::uuidv7::{Uuidv7Generator as CueIdGenerator, generate_ids as generate_cue_ids, unix_time_ms};` so existing translation call sites compile unchanged.
- [x] 1.4 Update `src/core/translation/engine.rs` import (`use crate::core::translation::uuidv7::CueIdGenerator;`) to point at the re-exported alias.
- [x] 1.5 Run `cargo nextest run --filter-expr 'test(translation)' || true` and the `src/core/uuidv7.rs::tests` module; both SHALL pass without changes.

## 2. Adopt UUIDv7 in media discovery

- [x] 2.1 Change `generate_file_id` in `src/core/matcher/discovery.rs` from `fn generate_file_id(path: &std::path::Path, file_size: u64) -> String` to `fn generate_file_id(generator: &mut crate::core::uuidv7::Uuidv7Generator) -> String`. Implementation: format the next ID as `format!("file_{}", generator.next_id().hyphenated())`.
- [x] 2.2 In `FileDiscovery::scan_directory` (and any nested helpers in `discovery.rs`), construct one `Uuidv7Generator` at the top of the scan and thread `&mut generator` through every classification site that produces a `MediaFile`.
- [x] 2.3 Repeat 2.2 for `FileDiscovery::scan_file_list`.
- [x] 2.4 Update every `generate_file_id(...)` call site in `src/core/matcher/engine.rs` (lines ~1412, ~1434, ~1708, ~1730 and any others) to use the new signature. Where the call site is inside a helper that currently constructs ad-hoc `MediaFile` values from a single path/size pair, instantiate a fresh local `Uuidv7Generator` for that helper.
- [x] 2.5 Replace the unit test `test_deterministic_id_generation` in `discovery.rs` with `test_uuidv7_id_generation` asserting: returned string starts with `file_`, has length 41, parses as a UUID with version 7, and a second call from the same generator returns a strictly-greater `unix_time_ts` than the first.
- [x] 2.6 Update `test_recursive_mode_with_unique_ids`, `test_media_file_structure_with_unique_id`, and any other tests that asserted hex-shape IDs to assert UUIDv7 shape and uniqueness.
- [x] 2.7 Run `cargo nextest run --filter-expr 'test(matcher::discovery) | test(matcher::engine)' || true` and confirm all matcher tests pass.

## 3. Adopt UUIDv7 in parallel processing

- [x] 3.1 In `src/core/parallel/worker.rs`, replace `Uuid::new_v4()` at line 51 (`WorkerPool::execute`) and line 222 (`Worker::new`) with `Uuid::now_v7()`.
- [x] 3.2 In `src/core/parallel/scheduler.rs`, replace `Uuid::new_v4()` at line 619 (`CounterTask::task_id`) with `Uuid::now_v7()`.
- [x] 3.3 Add unit tests in `src/core/parallel/worker.rs::tests` that assert `Worker::new().id().get_version_num() == 7` and that two consecutive `Worker::new()` calls produce different IDs.
- [x] 3.4 Add a unit test that exercises `WorkerPool::execute` with a trivial task and inspects (via a small accessor or by reading the active workers map under lock) that the dispatched `worker_id` has UUID version `7`.
- [x] 3.5 In `src/core/parallel/scheduler.rs::tests`, add a regression assertion that `CounterTask::task_id()` parses as a UUID with version `7`.
- [x] 3.6 Run `cargo nextest run --filter-expr 'test(parallel)' || true` and confirm all parallel tests pass.

## 4. Drop the `uuid/v4` Cargo feature

- [x] 4.1 Edit `Cargo.toml` so the `uuid` dependency reads `uuid = { version = "1.3", features = ["v7"] }`.
- [x] 4.2 Run `cargo build` and confirm there are zero references to `Uuid::new_v4` left in `src/` (`grep -rn "new_v4" src/` SHALL return nothing).
- [x] 4.3 Run `cargo build --tests` to confirm test code likewise compiles without the `v4` feature.

## 5. Silence ad-hoc match-engine debug output in JSON mode

- [x] 5.1 In `src/core/matcher/engine.rs::match_file_list_with_audit`, gate the `🔍 AI Analysis Results:` block (lines ~747–758) on a local `let json_mode = crate::cli::output::active_mode().is_json();` followed by `if !json_mode { eprintln!(...); ... }`.
- [x] 5.2 In `src/core/matcher/engine.rs::resolve_filename_conflict`, gate the `Warning: Skipping relocation due to existing file: ...` `eprintln!` (lines ~1262) and the `Warning: Conflict resolution prompt not implemented, using auto-rename` `eprintln!` (line ~1308) on the same `is_json()` check.
- [x] 5.3 Audit the rest of `engine.rs` (lines around 813–815 and any provider response echo path) for any other unconditional `eprintln!`/`println!` calls and apply the same `is_json()` guard.
- [x] 5.4 Audit `src/core/parallel/worker.rs` (e.g., the worker error `eprintln!` near lines 129–134) and `src/commands/` for unconditional `eprintln!`/`println!` calls reachable in JSON-mode execution paths and apply the same `is_json()` guard.
- [x] 5.5 Run a repository-wide `grep -rEn 'eprintln!|println!' src/core/ src/commands/` and verify every remaining call site is either inside a `cli::ui` helper, gated on `is_json()`, or otherwise covered by an explicit text-mode condition.

## 6. Migrate existing tests away from deterministic-ID assumptions

- [x] 6.1 Run `grep -rEn "file_[0-9a-f]{16}" tests/ src/` to enumerate every test that hard-codes the old `file_<16-hex>` shape. Triage each match.
- [x] 6.2 For tests that precompute file IDs via a first call to `FileDiscovery` and then assert equality of those IDs against a later invocation (`tests/output_format_match_tests.rs`, `tests/output_format_cross_command_tests.rs`, `tests/match_engine_id_integration_tests.rs`, and any cache-related tests under `tests/`), migrate them to either:
  - drive the matcher with a `MockOpenAITestHelper` configured to **echo** the IDs from the captured request (so the AI response always references whatever IDs the matcher just generated); or
  - assert on canonical filesystem paths instead of IDs.
- [x] 6.3 After migration, re-run `grep -rEn "file_[0-9a-f]{16}" tests/ src/` and confirm zero matches remain.
- [x] 6.4 Run `cargo nextest run --filter-expr 'test(output_format)' || true` and confirm all JSON-output integration tests still pass.

## 7. Regression tests for JSON-mode silence

- [x] 7.1 Add `tests/match_command_json_silence_test.rs` (top-level integration test, not a nested module) that uses `MockOpenAITestHelper` to stub a successful `analyze_content` response with several candidates, runs the `match` command via the in-process CLI entry point with `--output json --dry-run`, and asserts:
  - stdout contains exactly one JSON envelope (validated via `assert_json_stdout_clean`-style logic) with `command == "match"` and `status == "ok"`;
  - stderr does NOT contain the bytes `🔍`, `Total matches:`, `Preview:`, `Warning: Skipping relocation`, `Warning: Conflict resolution prompt not implemented`, or any line beginning with `   - file_`.
- [x] 7.2 Add a parallel test that runs the same flow without `--output json` and asserts the `🔍 AI Analysis Results:` block IS present on stderr (proving the gate is conditional, not a deletion).
- [x] 7.3 Add a third test that drives the live (non-dry-run) path with a target file already existing on disk so `ConflictResolution::Skip` fires, runs with `--output json`, and asserts stderr does NOT contain `Warning: Skipping relocation`.
- [x] 7.4 Run `cargo nextest run --filter-expr 'test(match_command_json_silence)' || true` and confirm all three tests pass.

## 8. Documentation and final quality checks

- [x] 8.1 Update `AGENTS.md`, `docs/tech-architecture.md`, and `docs/machine-readable-output.md` to reference the shared `crate::core::uuidv7` module, the unified UUIDv7 identifier scheme, and the tightened JSON-mode stderr discipline. Remove any text that still describes the `file_<16 hex chars>` shape or the relaxed "stderr MAY contain free-form chatter" wording.
- [x] 8.2 Update the doc-comment at the top of `src/cli/output.rs` (lines 12–20) and the doc comment on `Cli.output` in `src/cli/mod.rs` (lines 80–85) so the internal rustdoc agrees with the new spec.
- [x] 8.3 Review `README.md` and `README.zh-TW.md` for any text describing the relaxed stderr contract or the old `file_<hex>` ID shape; update if found, otherwise leave unchanged.
- [x] 8.4 Add a `### Changed` entry to `CHANGELOG.md` summarizing (a) the UUIDv7 migration and the resulting library-API break in `subx_cli::core::matcher::discovery::generate_file_id`, (b) the removal of the `uuid/v4` Cargo feature, and (c) the JSON-mode stderr tightening.
- [x] 8.5 Run `scripts/quality_check.sh` once and confirm it exits zero (formatting, clippy `-D warnings`, doc build, doc tests, and the full nextest suite all pass).
- [x] 8.6 Run `cargo clippy -- -D warnings` and `cargo fmt --check` as a final guard before commit.
