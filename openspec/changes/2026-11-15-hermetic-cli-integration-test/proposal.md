# Proposal

## Why

`Build, Test, Audit & Coverage` has been red on `master` since the 2.0.0 release (`run 34144825604`, 2026-09-07) and is red today. One test is responsible: `tests/integration_tests.rs::test_full_workflow`, which calls `test_match_command` first and stops the suite there.

The failure has two stages, and the second one is the real defect:

1. `tests/integration_tests.rs:25` sets `OPENAI_API_KEY="test"`. The config gate rejects keys shorter than 10 characters (`subx-core/src/config/validation.rs:85-91`), so the CLI exits 2 with `Configuration error: API key is too short`.
2. With a 10+ character dummy key the command proceeds, and `MatchEngine::match_file_list_with_audit` unconditionally calls `ai_client.analyze_content()` (`subx-core/src/core/matcher/engine.rs:1573`). The default provider is hosted OpenAI, so the run performs a real request to `https://api.openai.com/v1/chat/completions` and fails with 401. **The test has never had an AI endpoint under its control**; the API key was only ever a means of getting past validation on the way to a live network call.

Three consequences follow, and each one is why this is not a cosmetic test fix:

- nextest stops at the first failing binary, so only ~444 of 2,021 tests execute. The coverage job then partitions a truncated LCOV: `subx-core` is reported as `0/0` and `subx-cli` at `77.49%`, below its 82% floor. Red coverage is therefore a *symptom*, not a second bug.
- The failing Ubuntu leg cancels the Windows and macOS legs: the superproject's `test` job matrix has no `fail-fast: false`, while `subx-core`'s own identical workflow already sets it (`subx-core/.github/workflows/build-test-audit-coverage.yml:20-21`) and the superproject's own `coverage` job sets it. The superproject is the diverging side, so the same defect is never reported per platform.
- Every developer who runs `scripts/quality_check.sh` hits the same failure locally, including on machines with valid OpenAI credentials in their shell — which is exactly the environment dependence the test should not have.

`925f6ae` (subx-core 1.0.1, `zip` 9, lockfile refresh) did not cause this: the same failure predates it. That merge is only the reason CI must go green again now.

## What Changes

**1. The whole full-workflow test gets one isolated child environment.** A helper in `tests/integration_tests.rs` builds every child command for the three legs (match, convert, sync): `current_dir(dir)`, `SUBX_CONFIG_PATH` pointing at one temporary config file, `XDG_CONFIG_HOME` inside the test `TempDir`, `NO_PROXY`/`no_proxy` pinned to loopback, and the removal of `SUBX_OUTPUT` plus every variable in `tests/common/cli_helpers::ISOLATED_AI_ENV_VARS`. Today only the match leg sets anything, and convert/sync still read the developer's real `~/.config/subx/config.toml`, so an invalid user config fails them.

**2. The match leg gets its AI endpoint from a mock server it starts.** `wiremock` 0.6 is already a `subx-cli` `[dev-dependencies]` entry. The test mounts `POST /v1/chat/completions` on a loopback `MockServer` and writes the temporary TOML with `ai.provider = "local"`, `ai.model = "mock-model"` and `ai.base_url = "<server.uri()>/v1"`. The `local` provider is the only one that legitimately accepts an `http://` base URL; hosted providers are rejected by `require_https_for_hosted_provider`. The mock answers with an OpenAI-compatible envelope whose assistant content is the analysis JSON, with the file IDs read from the request prompt's `<video_files>` / `<subtitle_files>` blocks instead of hard-coded.

**3. The shared environment list stops being copied inline.** The thirteen AI-provider variable names already maintained inside `tests/common/cli_helpers.rs` become `pub const ISOLATED_AI_ENV_VARS: &[&str]`; `run_command_with_config` and the new workflow spawn helper both iterate it.

**4. The superproject test matrix stops cancelling its siblings.** `fail-fast: false` is added to the `test` job's strategy, matching `subx-core`'s workflow and the superproject's `coverage` job. No `subx-core` file changes, so the submodule pointer does not move.

**5. The contract is recorded where the next test author will look.** `AGENTS.md`'s "Spawning the binary in tests" bullet gains the mock-server rule, and `CHANGELOG.md` gains an `[Unreleased]` `### Fixed` entry.

No production code, no production dependency, no crate feature, and no public API item changes. `test_full_workflow` keeps its exact assertions: exit 0 and a `Preview mode` line on stdout.

**Out of scope, deliberately:** `tests/cli/match_command_json_silence.rs`, `tests/cli/output_format_match.rs`, `tests/cli/ai_usage_output_characterization.rs`, `tests/cli/cache_apply_conflict_warning.rs`, `tests/cli/output_format_translate_success.rs`, `tests/cli/translation_progress_characterization.rs` and one case in `tests/cli/output_format_cross_command.rs` select the `local` provider through `SUBX_AI_*`/`LOCAL_LLM_*` variables and do not strip the ambient overrides. Nine further files carry their own inline copy of the same thirteen-name list. They are non-hermetic in the same way, and they each need their own harness shim wired through `mod common;`; converting them is a follow-up change with its own verification surface, not a rider on the CI-unblocking fix. The new requirement is therefore scoped to the full-workflow test it repairs, and the follow-up is recorded here rather than silently implied.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `cross-crate-testing`: adds **Requirement: The Full-Workflow Test Is Hermetic** — one isolated child environment for all three legs, a self-started loopback mock for the match analysis, and a mock response derived from the request rather than pinned to a file ID.
- `crate-topology`: adds **Requirement: Test Matrix Legs Report Independently** — every operating-system leg of the superproject's test matrix SHALL run to completion, closing the divergence from `subx-core`'s already-correct workflow.

## Impact

- **Code:** `tests/integration_tests.rs` gains one child-command helper, one mock responder (with a block-scoped ID extractor) and the rewritten `test_match_command`; the three existing assertions stay. `tests/common/cli_helpers.rs` gains one exported constant and loses its inline copy. `.github/workflows/build-test-audit-coverage.yml` gains `fail-fast: false` under the `test` job's `strategy`. `AGENTS.md` and `CHANGELOG.md` are updated.
- **Tests:** `test_full_workflow` passes without network access or credentials on Linux, Windows and macOS under the `ci` nextest profile (2 retries, 300 s slow-timeout). Because the suite no longer aborts at test 445, the coverage job measures the full suite and both per-crate floors are evaluated against complete numerators.
- **APIs:** none.
- **Dependencies:** none. `wiremock`, `regex`, `tempfile`, `assert_cmd` and `predicates` are already declared in `subx-cli`'s `[dev-dependencies]`; `serde_json` and `tokio` (`rt-multi-thread`) are normal dependencies. No `Cargo.lock` change and no `subx-core` gitlink change.
- **Documentation:** `AGENTS.md` (CLI-specific "Spawning the binary in tests" bullet) and `CHANGELOG.md` (`[Unreleased]`, new `### Fixed` subsection).

## Verification

- `cargo nextest run -p subx-cli --profile ci --ignore-default-filter -E 'test(test_full_workflow)'` exits 0, on all three operating systems.
- The same filter run under `unshare -n` on Linux still exits 0, demonstrating that no external endpoint is reached.
- The full `scripts/quality_check.sh -v -p ci --full` run completes the suite instead of aborting, and `scripts/check_coverage.sh -T -p ci --full --lcov lcov.info` reports a non-zero denominator for both crates with `subx-core >= 90` and `subx-cli >= 82`.
- `git diff --name-only` shows no change to any `Cargo.toml`, `Cargo.lock`, or `subx-core/` gitlink; `cargo audit` reports the same findings as `925f6ae`.
- The workflow file contains `fail-fast: false` under the `test` job, and a deliberately failing leg on a scratch branch leaves the other two legs running to completion.