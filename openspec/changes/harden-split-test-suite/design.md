## Context

B3 landed the split. `subx-core` has its own `tests/` (40 files), `benches/` (2), `tests/fixtures/formats/` (22) and `assets/` (3); `subx-cli` keeps 71 live test files, its `tests/cli/` tree and its shims; `tests/common/` has been cut three ways, with 1,529 LOC behind `subx-core`'s `test-support` feature, 133 LOC staying CLI-side, and 493 LOC deleted for having no consumer. Every fixture and asset read resolves from `CARGO_MANIFEST_DIR`. No test in either repository resolves a library item through `subx-cli`'s D11 re-exports any more.

What B3 did not do is the hygiene work its own census turned up. It measured itself at ~14.5 h against a one-workday budget and split at the honest seam: everything that had to happen atomically with the move stayed, and three additive bodies of work moved here. B4 is those three, and the reason they are additive is worth stating precisely, because it is what makes the split safe rather than merely convenient:

- **The revivals touch files B3 does not touch.** All twelve orphans are, by definition, compiled by nothing, so B3's build is identical whether they are present or absent.
- **The `cargo_bin` rewrites touch call sites inside files that stay in `subx-cli` and are not moved by B3.** They are a search-and-replace over 17 files whose contents B3 edits only for library imports, in a different part of each file.
- **The coverage measurement cannot be taken before the move lands**, and is worth taking only after the revivals, because eight newly-compiled files move both numerators.

Nothing downstream is delayed. C1 needs the coverage numbers, and B4 precedes C1.

## Goals / Non-Goals

**Goals:**

- Close the never-compiled class: every `.rs` under a `tests/` tree in either repository is compiled by some test target, and a mechanical check keeps it that way.
- Make the binary a test spawns unnameable-wrongly: a bad name becomes a compile error.
- Produce two measured coverage numbers and two derived floors, with a policy that makes them defensible rather than arbitrary, and hand the wiring to C1 in writing.
- Leave both crates green, with more tests running than before and no production behaviour changed.

**Non-Goals:**

- Changing production code. If a revived test cannot pass without one, the test is deleted and the finding is recorded. B4 is hygiene, not bug-fixing.
- Authoring new tests. The four deleted files are deleted precisely because reviving them would be authoring.
- Wiring the coverage numbers into `scripts/check_coverage.sh`, `scripts/check_coverage.ps1`, the CI environment variables, `.llvm-cov.toml`, the two junit paths, or `default-members`. All of that is C1's, on the same division B1, B2 and B3 all honoured.
- Replacing `TestWorkspace`'s `cargo run --` spawning with `CARGO_BIN_EXE_subx-cli`. B3 recorded it as an open question; it is a behaviour change to eight test files and belongs in its own change (Open Questions).
- Touching `openspec/specs/**`, bumping `subx-cli` to 2.0.0, or altering anything B3 moved.

## Decisions

### Decision 1: the twelve orphans are triaged by one criterion, and it is written down before any of them is opened

Twelve files, 1,104 LOC, are wired to nothing. The temptation is to wire all twelve and see what happens; the failure mode is spending an afternoon debugging assertions in code that has never executed, inside a change whose budget is 5.5 h.

**The criterion: a file is revived when its imports already resolve, or resolve after a rewrite this change is performing anyway. It is deleted when reviving it requires authoring code that does not exist.**

Applied, it separates the twelve cleanly and without judgement calls:

**Revived — eight files, 1,001 LOC.**

| File | LOC | Destination | Why it qualifies |
|---|---|---|---|
| `tests/cli/config_args_tests.rs` | 65 | `subx-cli` | `clap::Parser` + `subx_cli::cli::{Cli, Commands, ConfigAction}`; all resolve |
| `tests/cli/detect_encoding_args_tests.rs` | 24 | `subx-cli` | same shape |
| `tests/cli/ui_tests.rs` | 35 | `subx-cli` | `cli::ui`, `cli::table` resolve; `services::ai::AiUsageStats` needs the `subx_core::` rewrite B4 is doing anyway |
| `tests/cli/sync_manual_offset_integration_tests.rs` | 39 | `subx-cli` | `assert_cmd`; its two `cargo_bin` sites are converted by phase 3 regardless |
| `tests/commands/sync_command_manual_offset_tests.rs` | 107 | `subx-cli` | self-contained; `subx_cli::config::test_service::TestConfigService` needs the same rewrite |
| `tests/cli/input_handler_tests.rs` | 108 | `subx-core` | A2's explicit handoff; `crate::cli::InputPathHandler` → `subx_core::core::input::InputPathHandler` is the rewrite A2 already performed everywhere else |
| `tests/parallel/integration_tests.rs` | 356 | `subx-core` | pure `subx_cli::core::parallel::*`; the `subx_cli::` → `subx_core::` rewrite is the same one B3 applied to 40 sibling files |
| `tests/sync/integration_tests.rs` | 267 | `subx-core` | same, plus one helper-import repair (below) |

**Deleted — four files, 103 LOC, plus one 1-byte placeholder.** `tests/commands/{cache_command_tests, config_command_tests, detect_encoding_tests, sync_command_tests}.rs` import `create_test_cache_files`, `create_test_config` and `create_utf8_subtitle_file` from `common::command_helpers`. That module is three lines of `//!` and defines nothing — it never has. Reviving these four means writing three helpers and then debugging four sets of never-executed assertions, which is authoring. `tests/config_basic_integration.rs` (a single newline, which Cargo compiles into an empty test crate) goes with them.

**`tests/sync/integration_tests.rs` is the one file that needs a judgement call, and it still passes the criterion.** Its `mod common; use common::{TestFileManager, AudioMockGenerator, SubtitleGenerator, SubtitleFormat};` (`:16-17`) is a *flat* import that has never matched `tests/common/mod.rs`'s shape — `mod.rs` re-exports modules, not items — so the file would not have compiled even with a shim. But the repair is not authoring: every one of those four types exists, and after B3 all four live at known paths under `subx_core::test_support`. The line becomes `use subx_core::test_support::{file_managers::TestFileManager, mock_generators::{AudioMockGenerator, SubtitleGenerator, SubtitleFormat}};`. That is a path rewrite, not a new helper.

**What happens when a revived test fails.** The rule is fixed in advance so nobody has to decide under time pressure: if a revived test fails on an assertion about behaviour that has legitimately changed since it was written, the **assertion** is updated to characterise current behaviour. If it cannot pass without a production change, it is deleted and the finding is recorded in the CHANGELOG for someone to pick up. A test that has never executed is not a regression, and B4 is not the change that repairs behaviour.

### Decision 1a: three of the orphans are core-bound, and moving them *is* reviving them — which is why B3 left them behind

B3's ownership census is 43 core-bound / 80 CLI-bound, and SDR §6 records it. But `tests/cli/input_handler_tests.rs`, `tests/parallel/integration_tests.rs` and `tests/sync/integration_tests.rs` are among the 43 *and* among the twelve orphans, and the two subdirectory files become live the instant they are flattened into `subx-core/tests/*.rs`, because Cargo auto-discovers everything there. B3 could not move them without reviving them, and could not move them unflattened without importing dead code into a fresh repository and failing its own `grep -rn 'subx_cli' subx-core/tests/` check.

**Resolution: B3 physically moves the 40 live core-bound files; B4 moves and revives the remaining three.** The classification is unchanged and stays where SDR §6 records it; only the physical move is split 40/3. This is the single place where the two changes interlock, and it is stated in both proposals so neither reads as if it moved 43.

The consequence for B4 is that it does cross the repository boundary, for three files. That is 531 LOC of `git mv`-equivalent plus an import rewrite, in a change that is otherwise confined to editing files in place. It is the riskiest thing B4 does, and it is bounded: if the move goes wrong, the three files were compiled by nothing before it and are compiled by nothing after a revert.

### Decision 2: the orphan class is closed by a mechanical check, in the file that already owns boundary checking

Wiring twelve files fixes twelve files. The class recurs the next time somebody adds `tests/cli/new_thing.rs` and forgets the shim — which is exactly how all twelve arose, over the repository's history, without anyone noticing.

`tests/core_cli_boundary.rs` is the right home. A1 created it to assert `src/core/` and `src/services/` contain no `crate::cli`; B2 repointed it at `subx-core/src/`; B3 widened it to assert no test resolves through the D11 re-exports. It already resolves the directories it walks from `env!("CARGO_MANIFEST_DIR")` rather than the working directory — the property B2's `crate-topology` spec requires of it — and it already fails with a `file:line` list.

B4 adds a fifth assertion: enumerate every `.rs` under the non-auto-discovered subdirectories of `subx-cli/tests/` and `subx-core/tests/`, collect the set of `#[path = "…"]` targets declared by every `tests/*.rs`, and fail on any file in the first set absent from the second. After B4 the first set is `subx-cli/tests/cli/**` and nothing else, because `tests/commands/`, `tests/parallel/` and `tests/sync/` are gone and `subx-core/tests/` is flat.

**Why not a build-script or a Cargo feature.** Cargo has no mechanism to fail on an unreferenced file under `tests/`, and a `build.rs` added purely for this would run on every build of a published crate for the benefit of a repository check. A test is the right shape: it runs where every other check runs, it costs a directory walk, and it is deleted along with the problem if the problem is ever solved another way.

**Why the check lives in `subx-cli` and covers both trees.** `subx-cli` can see `subx-core/tests/` through the submodule mount; `subx-core` cannot see `subx-cli` at all, and B2's `crate-topology` spec forbids it from trying. The check is therefore one-directional by construction, like every other cross-crate assertion in this series.

### Decision 3: the binary is named at compile time, which fixes the `cargo_bin("subx")` defect by removing the class rather than the instance

`tests/integration_tests.rs:24,38,58` calls `Command::cargo_bin("subx")`. `[[bin]] name = "subx-cli"`, so the lookup fails and `unwrap()` panics. It has never fired because `test_full_workflow` (`:68`) carries `#[ignore]` — which is why a defect this visible survived in a repository with a 75% coverage gate.

Correcting the three strings would fix three lines. The class is fixed by removing the string:

| | `Command::cargo_bin("subx-cli")` | `Command::new(env!("CARGO_BIN_EXE_subx-cli"))` |
|---|---|---|
| Wrong name | runtime `unwrap()` panic, inside a test | **compile error** |
| Renamed target | silently breaks every spawn test | build fails at the first one |
| Resolution | runtime search of the target directory | compile-time absolute path to the artefact built for *this* package |
| Multi-package workspace | can pick up a stale or sibling artefact | cannot |
| `unwrap()` | required | none |

The last row of the resolution column is not theoretical any more: after B1 the repository is a workspace with two packages sharing `target/`, and after B3 both have test targets in it.

All 26 sites are converted — 23 that name the binary correctly and 3 that do not — because converting only the broken ones would leave the mechanism that made them possible in place. The rule is normative in the delta so the next `assert_cmd` test is written the same way.

**`#[ignore]` is lifted from `test_full_workflow` and the test is run.** A skipped test with an unresolvable binary name is two defects wearing one coat, and fixing only the name would leave the test still not running. If it fails on stale assertions, Decision 1's rule applies: characterise, do not repair. If it cannot pass without a production change, the `#[ignore]` goes back with a comment naming the reason — which is strictly better than today, where the attribute carries no explanation at all.

### Decision 4: one instrumented run, split reporting — because per-package instrumentation would measure packaging, not coverage

`scripts/check_coverage.sh:369` runs `cargo llvm-cov nextest --profile <p> --workspace --no-report`, then `cargo llvm-cov report --json --summary-only`. B4 keeps **exactly one instrumented run** and splits only the report, grouping the per-file summaries by source-path prefix — `subx-core/src/**` versus `src/**`.

The alternative is `cargo llvm-cov -p subx-core` and `cargo llvm-cov -p subx-cli`, and it is wrong for a specific, checkable reason: the 71 CLI-side test files drive `commands::match_command::execute`, which composes `subx_core::core::matcher`, `subx_core::core::formats` and `subx_core::services::ai`. Under per-package instrumentation none of that execution counts toward `subx-core`, and core's number would fall for a reason that has nothing to do with how well it is tested. Splitting the report preserves the cross-crate attribution, which is what makes a per-crate floor a statement about coverage rather than about packaging.

It is also cheaper. A `--profile ci --full` instrumented workspace run is the single slowest operation in the repository; running it twice to get two numbers that are worse than the ones a single run already contains is not a trade worth making.

### Decision 4a: `.llvm-cov.toml` is inert, and B4 says so rather than working around it

Grepping `scripts/`, `.github/` and both manifests for `llvm-cov.toml` returns nothing in either repository. Every flag the coverage script uses is passed on the command line. Its `exclude-from-report = ["benches/*", "tests/*", "src/main.rs"]` has therefore never been in force — not before the split, and not after it. B1 duplicated the file into `subx-core` "for parity" and explicitly left the question to C1.

So the question "how are its relative patterns made to work in both crates" has no answer, because the premise is false. The real mechanism is a single argument on the report invocation, and B4 specifies its value:

```
--ignore-filename-regex '(^|/)(tests|benches)/|(^|/)src/main\.rs$|(^|/)src/test_support/'
```

Three of those four terms reproduce what the dead file claimed. The fourth, `src/test_support/`, is new and is B4's: B3 moved 1,529 LOC of mock helpers into `subx-core/src/`, and their own coverage is meaningless — they are exercised incidentally by whatever uses them, and including them would inflate `subx-core`'s numerator by an amount unrelated to how well the library is tested.

**Applying the flag is C1's**, together with deciding whether the two dead `.llvm-cov.toml` files are deleted or made load-bearing. B4's obligation is to record the finding and the value so C1 does not have to re-derive either.

### Decision 5: the floors are ratchets derived from a measurement, not numbers chosen in advance

| Scope | Floor | Reasoning |
|---|---|---|
| Workspace (combined) | **75%** — unchanged | The existing contract, and the gate that may not regress. B3 moved code without changing behaviour, so the combined figure should be unchanged within noise; if it is not, something was lost. |
| `subx-core` | **75%** | Core holds 36,687 of 49,069 measured lines — 75% of the codebase — plus every in-`src` unit test, 43 integration files, both benches, and the cross-attributed coverage from `subx-cli`'s 71. The combined figure was already dominated by core, so applying the existing number to the crate that dominates it is the one choice that introduces no new assumption. |
| `subx-cli` | **65%** | Its denominator is `src/cli/` (5,101) + `src/commands/` (6,283) + a slimmed `lib.rs`, with `src/main.rs` excluded — roughly 11,400 lines. 80 test files is a good ratio, but `src/cli/` is largely clap derives, `ui.rs` terminal rendering, `table.rs` and the process-global output mode, none of it reachable in-process without spawning. Solving `0.75 × 49,069 = x × 36,687 + y × 12,382` with core at or a little above 75% puts the CLI in the mid-60s. 65 is the floor beneath that, not a target. |

**The mechanism is a ratchet.** B4 measures each crate once with `--profile ci --full`, sets each floor to `floor(measured − 3)` clamped to the minima above, and records both the measured value and the derived floor in the CHANGELOG. A floor may be raised; it may never be lowered without a proposal saying why. The 3-point buffer absorbs the run-to-run variance that `retries = 2` and parallel scheduling introduce, and it is the same buffer in both crates so neither gets a quieter gate.

**The measurement is taken after the revivals, not before.** Eight newly-compiled files — 1,001 LOC of tests, three of them in `subx-core` — move both numerators by an unknown amount. Measuring first would derive floors from a suite that is about to change, and would then have to be redone.

**Handed to C1, explicitly and in one list:** the per-crate `COVERAGE_THRESHOLD_CORE` / `COVERAGE_THRESHOLD_CLI` plumbing in `scripts/check_coverage.sh` and `scripts/check_coverage.ps1`; the CI environment variables at `.github/workflows/build-test-audit-coverage.yml:186,195`; the `--ignore-filename-regex` value from Decision 4a; the fate of the two `.llvm-cov.toml` files; the two junit paths (B1 Decision 5a); and `default-members`. That last one has teeth: with it unset, a bare `cargo nextest run` acts on the root package alone, which after B3 silently skips 43 test files. B3 documents the `--workspace` workaround because the hazard appears the moment its move lands; B4 verifies it empirically and states the normative requirement, because B4 is where the coverage contract is written. The overlap is deliberate and is noted in both changes.

### Decision 6: what B4 deliberately does *not* take from B3

Three things were candidates for this change and stayed in B3, each for the same reason — they are part of the move rather than part of the cleanup:

- **Deleting `tests/common/command_helpers.rs`.** It is a `tests/common/` module, and `tests/common/` is B3's to split. B3 must remove `pub mod command_helpers;` from `mod.rs` in any case, and leaving the file behind would create exactly the kind of undeclared orphan A0 deleted from `src/cli/validation.rs`. B4 deletes its four dead consumers; B3 deletes the module. The sequence reads correctly in the history: the helper goes when its directory is reorganised, the tests go when the never-compiled class is triaged.
- **Converting the six staying `sync_*` tests' `assets/…` literals to `CARGO_MANIFEST_DIR`.** Asset resolution is B3's, because B3 is what moves the assets.
- **The `--workspace` hazard note in `AGENTS.md`.** B3 creates the hazard; B3 documents it. B4 owns the requirement.

## Risks / Trade-offs

- **Risk: a revived test fails and B4 spends its budget debugging code nobody asked it to touch.** → Mitigation: Decision 1 fixes the rule before any file is opened — characterise, do not repair; if it cannot pass without a production change, delete and record. The revivals are also their own phase, so they can be dropped wholesale without affecting the binary-name or coverage work.
- **Risk: `tests/sync/integration_tests.rs` (267 LOC, never executed, against `SyncEngine` and `DialogueDetector`) turns out to need more than an import repair.** → Mitigation: it is the largest and least certain of the eight, so it is sequenced last among the revivals; if it exceeds its share it is deleted under the same rule as the other four, which is a strictly better outcome than the status quo where it is neither compiled nor deleted.
- **Risk: moving three files across the repository boundary goes wrong in a change that is otherwise edit-in-place.** → Mitigation: the three are compiled by nothing before and after, so a failed move breaks no build; the task list moves them one at a time and runs each immediately; and B3 has already established the mechanism for exactly this crossing.
- **Risk: `Command::new(env!("CARGO_BIN_EXE_subx-cli"))` behaves differently from `Command::cargo_bin` in some case the 17 files depend on.** → Mitigation: both produce an `assert_cmd::Command`, so `.env`, `.arg`, `.assert()` and every predicate are unchanged; the difference is only how the path is obtained. The conversion is verified by running the full `assert_cmd` set, not by inspection, and `grep -rn 'cargo_bin' tests/ subx-core/tests/` must return zero afterwards.
- **Risk: lifting `#[ignore]` from `test_full_workflow` adds a flaky test to the suite.** → Mitigation: it is run before the attribute is removed permanently, and Decision 3 states the fallback — restore `#[ignore]` with a comment naming the reason, which is still an improvement on an unexplained attribute over an unresolvable binary name.
- **Risk: `subx-cli`'s measured coverage lands below 65% and the floor becomes a negotiation.** → Mitigation: the floor is derived from a measurement taken in this change rather than chosen in advance, and the *workspace* floor at 75% is the gate that actually protects against regression. If `subx-cli` measures below 65, that is a finding to hand to C1 with the numbers — and B4's own Open Questions already name the four deleted `tests/commands/` files as the obvious first place to spend on it.
- **Risk: the coverage run is the slowest operation in the repository and must not be run from a sub-agent or in parallel.** → Mitigation: AGENTS.md already forbids it, the task list marks the step "main agent only", and it is a single step taken once.
- **Trade-off: 1,001 LOC of never-executed test code enters the suite, some of it likely to need its assertions rewritten.** → Accepted. The alternative is 1,104 LOC that looks like coverage, is counted by nobody, and misleads every reader of the directory listing.
- **Trade-off: four test files covering `cache_command`, `config_command`, `detect_encoding_command` and `sync_command` are deleted rather than written.** → Accepted (Decision 1). They are genuine coverage `subx-cli` would benefit from, and writing them is a good change — just not this one.
- **Trade-off: the `--workspace` hazard is documented in B3 and specified in B4.** → Accepted (Decision 6). The hazard appears with B3's move and the contract is written with B4's floors; documenting it in the earlier change and normalising it in the later one is the honest ordering.

## Migration Plan

Each step leaves both crates green, and no step depends on a later one.

1. **Baseline.** Confirm B3 landed: `subx-core/tests/` holds 40 files, `subx-core/benches/` holds 2, `tests/common/` holds `mod.rs`, `cli_helpers.rs` and `json_output.rs`, and `cargo nextest run --workspace` is green. Re-derive the twelve orphans and the 26 `cargo_bin` sites.
2. **Delete the five unrevivable files.** Nothing referenced them; `cargo nextest run --workspace` is unchanged.
3. **Revive the five CLI-side files**, one at a time, running each.
4. **Move and revive the three core-side files**, one at a time, running each. This is the only repository-boundary crossing.
5. **Add the orphan check** to `tests/core_cli_boundary.rs` and confirm it reports zero.
6. **Convert all 26 `cargo_bin` sites**, lift `#[ignore]` from `test_full_workflow`, and run the full `assert_cmd` set.
7. **Verify** both crates: `cargo nextest run --workspace`, `--features slow-tests`, `cargo bench --no-run --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and a standalone `subx-core` clone.
8. **Measure coverage once**, derive the floors, record the numbers and the C1 handoff list.
9. **Documentation** and `[Unreleased]` CHANGELOG entries in both repositories, then the quality gate on the main agent only.

**Rollback** is `git revert` in each repository. No public API changes, no manifest changes, no production source changes, and the eight revived files return to being compiled by nothing — which is where they were.

## Sizing

Estimated at **~5.5 h**, and the estimate is the residual of B3's own 14.5 h measurement rather than a fresh guess:

| Phase | Estimate |
|---|---|
| Baseline and orphan re-derivation | 0.25 h |
| Deleting the five unrevivable files | 0.25 h |
| Reviving five CLI-side files | 0.75 h |
| Moving and reviving three core-side files (531 LOC, one boundary crossing) | 1.25 h |
| The orphan check in `tests/core_cli_boundary.rs` | 0.25 h |
| 26 `cargo_bin` conversions and lifting `#[ignore]` | 0.5 h |
| Verification across both crates and a standalone clone | 0.75 h |
| Coverage measurement, floor derivation, C1 handoff list | 1 h |
| Documentation and CHANGELOG in both repositories | 0.5 h |
| **Total** | **5.5 h** |

B3 is correspondingly **~9 h**. The two items that could overrun are the same two B3 identified: the revivals, if more than one or two fail on stale assertions (each becomes an investigation, bounded by Decision 1's delete rule), and the coverage run, which is a single `--profile ci --full` instrumented workspace pass and cannot be parallelised or delegated.

If the revivals overrun badly, the seam inside B4 is between phase 5 and phase 6 of the task list: phases 6–8 (binary naming and coverage) depend on nothing the revivals produce except a slightly larger numerator, and C1 needs only the coverage numbers.

## Open Questions

- **Should the four deleted `tests/commands/` files be rewritten?** They cover `cache_command`, `config_command`, `detect_encoding_command` and `sync_command` at the command level, which is genuine coverage `subx-cli` needs. If `subx-cli`'s measured figure lands near its 65% floor, this is the obvious first place to spend, and the three missing helpers are trivial. Not blocking.
- **Should `TestWorkspace`'s `run_command_with_config` stop shelling out to `cargo run --`?** A nested Cargo invocation inside `cargo nextest run` serialises on the build lock and rebuilds the binary from inside a test. Decision 3 establishes the replacement (`CARGO_BIN_EXE_subx-cli`), and eight test files use it. B3 raised this and B4 deliberately does not take it, because it is a behaviour change rather than a naming change. Worth its own small change, after C1.
- **Do the two `.llvm-cov.toml` files stay?** They are read by nothing in either repository. C1 either deletes both or converts their intent into the `--ignore-filename-regex` Decision 4a specifies. B4 records; C1 decides.
- **Should `test_full_workflow` remain a single test?** It runs match, convert and sync in sequence against one temp directory and fails as a unit. If lifting `#[ignore]` shows it to be flaky, splitting it into three is the obvious repair — but that is authoring, so it is out of scope under Decision 1's own rule.
