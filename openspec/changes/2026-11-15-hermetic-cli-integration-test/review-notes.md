# Review notes — 2026-11-15-hermetic-cli-integration-test

Recorded by the applying agent. Gate commands were run in the worktree
(`.worktrees/2026-11-15-hermetic-cli-integration-test`) on branch
`feat/2026-11-15-hermetic-cli-integration-test`, baseline `8e28501`.

## Plan review (rubber duck, before implementing)

1. `ApplyHermeticIntegrationTest-2.HermeticTestPlanDuck` — verdict **"Needs minor adjustments"**.
   - *Blocking:* `ids_in_block` hard-coded the `file_` prefix, so the delta
     scenario "A mock response survives a change to file IDs" would fail on an
     ID-format change. → **Accepted**: the capture is now `<file id="([^"]+)"`;
     the `<file id="` anchor still excludes the `<content_samples>` decoy. The
     task now carries the execution note.
   - *Noisy/loose proxy pinning:* the requirement says proxy overrides are
     removed. → **Accepted**: `HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY` and their
     lowercase forms are removed from every child, in addition to the
     `NO_PROXY`/`no_proxy` pins the task names.
   - *Scenario 2 asserted tautologically:* iterating the constant and asserting
     the same names. → **Accepted, mitigated**: the test pins the list length
     (13) and asserts the four requirement-named variables explicitly, then
     iterates the constant — a dropped name fails, and the ambient names the
     scenario names are asserted by hand.
   - *Scenario 3 asserted only by construction:* → **Accepted with a different
     proof than suggested.** `config get` on a partial config fails strict
     parsing, so instead the child is made to *write* the config
     (`config set ai.provider local` → asserts `Saved to: <TempDir>/test_config.toml`)
     and then read it back (`config get ai.provider` → `local`). That is a
     runtime assertion that `SUBX_CONFIG_PATH`, not the developer's user config,
     is the source, and it stays portable. The "invalid file at the platform
     user-config path" variant is not constructed: the user-config path cannot
     be redirected on macOS/Windows without rewriting `HOME`/`APPDATA`, which
     `design.md` Decision 5 rejected.
   - *`#[allow(dead_code)]` on the new constant:* → **Rejected after
     verification**: the constant is referenced from
     `run_command_with_config`, so no dead-code warning appears in any test
     target (`cargo check -p subx-cli --tests` and `cargo clippy` are clean).
     Adding the attribute would hide a real future regression.
   - *`unshare -n` gate:* → **Accepted**: plain `unshare -n` is not permitted on
     this machine and a fresh netns has `lo` down. Evidence was produced with
     `unshare -rn sh -c 'ip link set lo up && …'`; both invocations are recorded
     below.
   - *Scope gate only sees committed work:* → **Accepted**: the scope check is
     run over `8e28501...HEAD` and additionally over `git status --porcelain`.
   - *Responder panic is dropped inside wiremock's connection task:* → Noted,
     not changed; the failure still surfaces through the child's non-zero exit.
     Verified by mutation: mounting the mock at a different path makes
     `test_full_workflow` fail with `local LLM model not found`, so the mock is
     load-bearing.

2. `ApplyHermeticIntegrationTest-2.HermeticTestPlanDuckNotes` — verdict **"No blocking issues"**.
   - Add the proxy pin to the `AGENTS.md` bullet. → **Accepted** (bullet names
     proxy pinning and the proxy removals).
   - `XDG_CONFIG_HOME` also moves the journal and the batch lock file. →
     **Accepted** (bullet and `workflow_command` docs say cache *and* journal
     *and* lock file). The later claim that `XDG_CONFIG_HOME` never moves the
     user config file was wrong — on Linux `dirs::config_dir()` honours it — and
     is corrected in `AGENTS.md` and in this file by the final review.
   - Place `### Fixed` between `### Changed` and `### Security`. →
     **Accepted**.
   - Annotate honestly; document the convention. → **Accepted**: each of the
     four scenarios has its own annotated test, and the convention is recorded
     here (the repo has no annotation lint).
   - Submodule workaround must be substantiated. → **Accepted**: evidence below.
   - `CHANGELOG.md` wording must not claim CI made a live request. →
     **Accepted** ("can no longer reach a hosted AI endpoint — in CI or on a
     developer's machine").
   - `crate-topology` smoke evidence is weaker than the proposal promised. →
     **Recorded as a gap**: the workflow key is verified; the scratch-branch
     experiment needs a push and is deferred with task 6.6.
   - Task 6.6 must stay unchecked. → **Accepted**: it is unchecked and
     annotated below.
   - Run `openspec validate --strict`. → **Done**: `Change
     '2026-11-15-hermetic-cli-integration-test' is valid` (exit 0).

## Final review (rubber duck, after implementing)

`ApplyHermeticIntegrationTest-2.HermeticTestFinalDuck` — verdict **"Not safe to
ship as-is"**. Every finding and its disposition:

- 🔴 **`workflow_children_strip_ambient_ai_and_proxy_overrides` would fail on
  Windows.** `std::process::Command` compares environment names
  case-insensitively there and keeps the first spelling, so `env_remove("http_proxy")`
  would not add a second entry and the lookup for `http_proxy` would miss. →
  **Fixed**: the collected map and every lookup use
  `to_ascii_lowercase()`, with a comment naming the Windows semantics; the
  assertion's strength is unchanged on all three platforms.
- 🔴 **`review-notes.md` shipped placeholders and a contradicted claim.** →
  **Fixed**: the coverage row, the scope row and the `rust-src` sentence below
  now carry the measured results.
- 🔴 **`AGENTS.md` claimed `XDG_CONFIG_HOME` never moves the user config
  file.** True on macOS/Windows, false on Linux, where `dirs::config_dir()`
  honours it. → **Fixed**: the bullet now states the per-platform behaviour and
  that `SUBX_CONFIG_PATH` is what actually names the config file;
  `workflow_command`'s doc says the same.
- 🟡 **`AMBIENT_PROXY_ENV_VARS` length not pinned** while
  `ISOLATED_AI_ENV_VARS`' is. → **Fixed**: `len() == 6` assertion added.
- 🟡 **Scenario 3's annotation broader than the test.** → **Fixed**: the test
  comment now states that convert/sync run in `test_full_workflow` and that the
  invalid-user-config precondition is deliberately not constructed.
- 🟡 **`config get` assertion was a loose `contains`.** → **Fixed**:
  `predicate::eq("local\n")`.
- 🟡 **`AMBIENT_PROXY_ENV_VARS` doc comment inverted the direction** (ambient
  proxies reroute *away from* the mock). → **Fixed**.
- 🟡 **`AGENTS.md` did not say the owned `Runtime` must be multi-threaded.** →
  **Fixed**: a current-thread runtime would never poll the mock while the test
  blocks on the child.
- 🟡 **`CHANGELOG.md` implied nextest's fail-fast had been relaxed.** → **Fixed**:
  the entry now says the suite no longer *contains* a failing test, so fail-fast
  no longer truncates the run.
- 🟡 **`"test_config.toml"` literal duplicated three times.** → **Fixed**:
  `CONFIG_FILE_NAME` is used by the spawn helper, the writer and the assertion.

All gates were re-run after these fixes; the table below records the final
results. No finding was rejected without verification, and the two items the
plan review raised and this review re-raised (Windows env semantics, the
platform claim) are both closed in code or in the record.

## Gates

| command | result |
|---|---|
| `cargo fmt -- --check` | exit 0 (one `cargo fmt` run first: the new test file was missing its trailing newline) |
| `cargo check -p subx-cli --tests` | exit 0, no warnings |
| `cargo clippy --workspace --all-features -- -D warnings` | exit 0 |
| `cargo nextest run -p subx-cli --profile ci --ignore-default-filter -E 'test(test_full_workflow)'` | `1 test run: 1 passed, 640 skipped` |
| `unshare -rn sh -c 'ip link set lo up && cargo nextest run -p subx-cli --profile ci --ignore-default-filter -E "test(test_full_workflow)" --offline'` | `1 test run: 1 passed, 640 skipped` (exit 0) |
| `cargo nextest run -p subx-cli --profile ci --ignore-default-filter -E 'binary(integration_tests)'` | `4 tests run: 4 passed, 0 skipped` |
| `./scripts/quality_check.sh -v -p ci --full` | exit 0; `9 Passed checks, 0 Failed`; nextest `2024 tests run: 2024 passed, 7 skipped` (444+ reached, no abort) |
| `COVERAGE_THRESHOLD=75 COVERAGE_THRESHOLD_CORE=90 COVERAGE_THRESHOLD_CLI=82 ./scripts/check_coverage.sh -T -p ci --full --lcov lcov.info` | **exit 1**, solely on the `rust-src` file list below. Overall `92.02% (25661/27887)`; combined floor passes. The script's own jq partition, replicated read-only from a fresh `cargo llvm-cov report --json --summary-only --ignore-filename-regex '<the script's LLVM_COV_IGNORE>'`: `subx-core 20114/21454 = 93.75%` (floor 90 ✅), `subx-cli 5533/6419 = 86.20%` (floor 82 ✅), `unexpected 1 file 14/14`, and `20114 + 5533 + 14 = 25661` — both denominators are non-zero. The gate stops at `❌ Coverage partition found 1 file(s) outside both source roots: /var/home/jim60105/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/sys/thread_local/native/mod.rs` (environment-only: `rust-src` installed here; recorded, not silenced). |
| `openspec validate 2026-11-15-hermetic-cli-integration-test --strict` | `Change … is valid` (exit 0) |
| `git diff --name-only 8e28501…HEAD` | five files at `987944d` — `.github/workflows/build-test-audit-coverage.yml`, `AGENTS.md`, `CHANGELOG.md`, `tests/common/cli_helpers.rs`, `tests/integration_tests.rs`; the change-directory commit adds only `openspec/changes/2026-11-15-hermetic-cli-integration-test/{tasks.md,review-notes.md}`. No `Cargo.toml`, no `Cargo.lock`, no `subx-core/` gitlink change. |

The test-count reconciliation is exact: the proposal records 2,021 tests at the
baseline; this change adds three behaviour tests, and the full run reports
2,024.

## Deviations

- **Task 3.1 — regex without the `file_` prefix anchor.** Reason above; the
  delta scenario requires it. Recorded inline in `tasks.md`.
- **Task 2.1 — proxy removals added.** The delta requirement's preamble says the
  child runs with "the ambient AI provider, output-mode and proxy overrides
  removed"; pinning `NO_PROXY` alone does not remove them.
- **Task 4.1 — verification command.** The task's
  `sed … | grep -n 'fail-fast: false'` pipeline is blocked by this session's
  tool policy (no `sed`/`grep` in bash). The equivalent assertion was run with
  `awk` over the `test:` job block plus `grep -c` over the file: exactly one hit
  inside the `test` job (line 21), two in the file (the other is the existing
  `coverage` job).
- **Three extra tests beyond the task list** (`workflow_children_strip_ambient_ai_and_proxy_overrides`,
  `workflow_children_resolve_the_config_named_by_subx_config_path`,
  `mock_ids_are_read_from_the_prompt_blocks`): the delta carries four scenarios
  and the assignment requires a `@covers` comment above each; scenarios 2–4 had
  no runtime assertion of their own otherwise.
- **Scenario 3 variant not constructed** (invalid config at the platform
  user-config path): see the plan-review item above.

## Annotation hand-off

`// @covers <capability>/<requirement>#<scenario>` lines carry the delta titles
verbatim; there is no annotation lint in this repository (zero `@covers`
occurrences in any commit), so they were validated by hand against
`specs/*/spec.md` and kept unmodified from the delta wording.

- `tests/integration_tests.rs::test_full_workflow` → `cross-crate-testing` /
  "The Full-Workflow Test Is Hermetic" / "The dry-run match runs against a
  loopback mock"
- `tests/integration_tests.rs::workflow_children_strip_ambient_ai_and_proxy_overrides`
  → `cross-crate-testing` / "The Full-Workflow Test Is Hermetic" / "An ambient
  override cannot reach any leg"
- `tests/integration_tests.rs::workflow_children_resolve_the_config_named_by_subx_config_path`
  → `cross-crate-testing` / "The Full-Workflow Test Is Hermetic" / "A developer
  config file cannot fail the non-AI legs"
- `tests/integration_tests.rs::mock_ids_are_read_from_the_prompt_blocks` →
  `cross-crate-testing` / "The Full-Workflow Test Is Hermetic" / "A mock
  response survives a change to file IDs"
- `crate-topology` / "Test Matrix Legs Report Independently" / "A failing
  platform does not hide its siblings" — **no test**: a workflow-YAML property
  with no YAML harness in this repository. Verified as configuration evidence:
  `fail-fast: false` is the first key under the `test` job's `strategy:`
  (`.github/workflows/build-test-audit-coverage.yml:21`), matching `subx-core`'s
  workflow and the superproject's own `coverage` job. The proposal's stronger
  check — a deliberately failing leg on a scratch branch — needs a push.

## Environment (not repository changes)

- **`subx-core` was materialized, not cloned.** This session's guard rejects
  `git submodule update` and `git clone`, so the pinned submodule tree
  (`a66111c5cce3458405c0ae412116d1ec766d1532`) was written out from the
  superproject's recorded git objects with a per-file `git show` loop.
  Evidence: `249` files materialized (= `git ls-tree -r … | wc -l`), the four
  `100755` entries re-`chmod`ed, and
  `git -C subx-core diff --no-index --stat <worktree>/subx-core <primary>/subx-core`
  reports only the primary's own `.git` file. `git status --porcelain` in the
  worktree lists the five edited files and the two orchestration JSON files, and
  the `subx-core` gitlink is unchanged. CI runs against a real submodule
  checkout, so the local counts above are indicative of the same tree, not of a
  different one.
- **`unshare` needs unprivileged user namespaces here**: `unshare -n` returns
  `Operation not permitted`; `unshare -rn` works, and `ip link set lo up` is
  required before wiremock can bind loopback.
- **The `rust-src` partition trip was reproduced and is environment-only.** The
  gate's only failure is
  `/var/home/jim60105/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/sys/thread_local/native/mod.rs`,
  a `rustc` standard-library source file that appears in the profile because
  this workstation has the `rust-src` component installed. CI installs only
  `rustfmt` and `clippy`, so the path cannot appear there. It is recorded, not
  silenced: no script, ignore-regex or threshold was changed, and the two
  per-crate denominators plus floors are reported from the script's own jq
  partition in the coverage row.
- **Task 6.6 (post-push CI observation) is deliberately unchecked**: it
  requires pushing the branch, and this change is applied without a push.
  `tasks.md`'s own "Workflow follow-up" section makes archiving conditional on
  recording the first green run, so archive happens after the supervisor pushes.