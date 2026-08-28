## 1. Preconditions and Baseline

- [ ] 1.1 Confirm B4 landed: `subx-core/tests/` holds 43 files and is flat (no subdirectories), `subx-cli/tests/{commands,parallel,sync}/` no longer exist, `grep -rn 'cargo_bin' tests/ subx-core/tests/` returns **0**, and `tests/core_cli_boundary.rs` contains the orphan-file assertion
- [ ] 1.2 Read the two measured coverage values and the two derived floors out of B4's `[Unreleased]` CHANGELOG entries in both repositories, and record them here — they are the numbers task 4.6 writes into the scripts and task 4.8 writes into CI. If a derived floor exceeds B4's minimum (75 for `subx-core`, 65 for `subx-cli`), the derived value is the one that gets written
- [ ] 1.3 Run `scripts/quality_check.sh -p ci --full` once (main agent only) and record the reported test count — it reflects `subx-cli` alone, and task 2.6 compares against it
- [ ] 1.4 Run `scripts/check_coverage.sh -T -p ci --full` once (main agent only) and record the combined percentage, so task 4.9 can attribute any movement to the new `--ignore-filename-regex` rather than to noise
- [ ] 1.5 Confirm `git submodule status` shows no `+`, `-` or `U` prefix and `git -C subx-core status --porcelain` is empty, so the tree is publish-clean before anything is edited
- [ ] 1.6 Confirm `cargo --version` reports 1.90.0 or later locally, since tasks 7.5 and 9.6 run `cargo publish --workspace --dry-run`

## 2. Close the Workspace Gate Hole

- [ ] 2.1 In a scratch workspace outside this tree (root package + one member, `edition = "2024"`, `resolver = "3"`, a pass-through feature declared on both and a member-only feature), re-verify the six cases in `design.md` Decision 5's table before editing anything real — in particular that `cargo check --workspace --features <name>` succeeds when only one selected package declares the feature, and errors only when neither does
- [ ] 2.2 Add `default-members = [".", "subx-core"]` to `Cargo.toml`'s `[workspace]` table, directly beneath `members`, with an English comment stating that it makes a bare `cargo nextest run` / `cargo clippy` cover both crates
- [ ] 2.3 Confirm `cargo run -- --version`, `cargo build --release --features archive-rar` and `cargo install --path . --dry-run`-equivalent behaviour are unaffected by 2.2; in particular confirm `cargo build --features archive-rar` does **not** error with a multi-package feature-selection message
- [ ] 2.4 Add `--workspace` to `scripts/quality_check.sh`'s four cargo invocations: `cargo check` (`:227` verbose, `:229` quiet), `cargo clippy` (`:237` verbose, `:239` quiet), `cargo nextest run … -E 'kind(lib)'` (`:328`) and `cargo nextest run … --ignore-default-filter` (`:331`)
- [ ] 2.5 Add the matching `--workspace` to `scripts/quality_check.ps1`'s counterparts at `:196`, `:200`, `:212`, `:216`, `:306` and `:311`; leave `cargo fmt -- --check` (`:206`) alone — `cargo-fmt` already enumerates workspace targets
- [ ] 2.6 Run `scripts/quality_check.sh -p ci --full` (main agent only) and confirm the reported test count is now materially higher than task 1.3's, and that the run is green; investigate any new clippy or test failure in `subx-core` here rather than at the end of the change
- [ ] 2.7 Add `--workspace` to `.github/workflows/build-test-audit-coverage.yml`'s bare `cargo clippy -- -D warnings` (`:61`), `cargo check --features archive-rar` (`:133`) and `cargo nextest run --profile ci --features archive-rar` (`:139`) — the last one matters most, because `subx-core` owns the archive-extraction tests after B3
- [ ] 2.8 Leave `scripts/check_coverage.sh:369` unchanged; it has carried `--workspace` since B1

## 3. Repair the Queued Defects

- [ ] 3.1 In `.config/nextest.toml`, move the `[profile.default.junit]` table from the end of the file (`:55-57`) to directly beneath `[profile.default]`, and change `path = "target/nextest/junit.xml"` to `path = "junit.xml"`, with an English comment stating that nextest resolves `junit.path` relative to `target/nextest/<profile>/`
- [ ] 3.2 Leave `[profile.ci.junit] path = "junit.xml"` exactly as it is — B1 Decision 5a established it already lands on `target/nextest/ci/junit.xml`, which `build-test-audit-coverage.yml:90` and `:213` upload
- [ ] 3.3 Rewrite every Chinese comment in `.config/nextest.toml` (`:7`, `:10`, `:13`, `:16`, `:19`, `:22`, `:26`, `:34`, `:40`, `:47`, `:56`) in English, closing the divergence B1 Decision 5 deliberately left between this file and `subx-core/.config/nextest.toml`
- [ ] 3.4 Apply tasks 3.1 and 3.2 identically to `subx-core/.config/nextest.toml`, then confirm the two files differ only in comment wording with `diff .config/nextest.toml subx-core/.config/nextest.toml`
- [ ] 3.5 Verify the junit paths empirically: run `cargo nextest run --workspace --profile ci -E 'test(version)'` and assert `target/nextest/ci/junit.xml` exists; run the same with `--profile default` and assert `target/nextest/default/junit.xml` exists and that `target/nextest/default/target/` does **not**
- [ ] 3.6 In `.gitignore`, delete the now-subsumed `target/nextest/*/junit.xml` pattern (`:20`), keeping `junit.xml` (`:19`) and its `# JUnit XML test results` heading; `/target/` at `:2` already covers everything under the target directory. Apply the same edit to `subx-core/.gitignore`
- [ ] 3.7 Re-run `grep -rn 'llvm-cov.toml' scripts/ .github/ Cargo.toml subx-core/ ` immediately before deleting, and confirm it returns **0** hits in both repositories — this is the third independent confirmation, after B3's and B4's
- [ ] 3.8 Delete `.llvm-cov.toml` and `subx-core/.llvm-cov.toml`
- [ ] 3.9 Narrow the four cache keys in `build-test-audit-coverage.yml` from `hashFiles('**/Cargo.lock')` to `hashFiles('Cargo.lock')`: `:43`, `:49`, `:55` in job `test` and `:118`, `:124`, `:130` in job `archive-rar` (six steps in total across the two jobs); the workspace build resolves against the root lockfile alone and `subx-core/Cargo.lock` is inert inside the workspace

## 4. Per-Crate Coverage Wiring

- [ ] 4.1 Define the exclusion pattern once at the top of `scripts/check_coverage.sh` as `LLVM_COV_IGNORE='(^|/)(tests|benches)/|(^|/)src/main\.rs$|(^|/)src/test_support/'`, exactly the value B4's `cross-crate-testing` spec fixes
- [ ] 4.2 Pass `--ignore-filename-regex "$LLVM_COV_IGNORE"` to **both** report invocations — `cargo llvm-cov report --lcov --output-path` (`:392`) and `cargo llvm-cov report --json --summary-only` (`:400`) — and add a comment stating that the two must stay identical or the gated number and the Codecov-displayed number describe different codebases
- [ ] 4.3 Add `COVERAGE_THRESHOLD_CORE` and `COVERAGE_THRESHOLD_CLI` alongside the existing `COVERAGE_THRESHOLD` (`:25-26`), each with a `DEFAULT_*` constant and environment-variable override, plus `--threshold-core` / `--threshold-cli` flags in `parse_args` (`:127-188`) and matching lines in `usage` (`:47-75`)
- [ ] 4.4 Add a `show_per_crate_coverage()` function that partitions `.data[0].files[]` by whether `.filename` contains `/subx-core/src/`, sums `.summary.lines.covered` and `.summary.lines.count` within each group, and derives each percentage from the sums — never as a mean of per-file percentages, which would weight a 3-line module like a 900-line engine
- [ ] 4.5 Make `show_overall_coverage()` (`:244-292`) report and gate all three numbers: the combined total against `COVERAGE_THRESHOLD`, the `subx-core` group against `COVERAGE_THRESHOLD_CORE`, and the `subx-cli` group against `COVERAGE_THRESHOLD_CLI`; all three are printed regardless of outcome, and the function returns non-zero if any one is below its floor
- [ ] 4.6 Set the two new defaults to the values recorded in task 1.2 (B4's `floor(measured − 3)`, clamped to minima of 75 for core and 65 for CLI); leave `DEFAULT_THRESHOLD=75.0` unchanged — it is the gate that may not regress
- [ ] 4.7 Port tasks 4.1–4.6 to `scripts/check_coverage.ps1`: the same regex on both report invocations (`:356`, `:364`), two new `[double]` parameters with `COVERAGE_THRESHOLD_CORE` / `COVERAGE_THRESHOLD_CLI` environment fallbacks beside the existing `$Threshold` block (`:88-97`), and the same partition implemented over PowerShell's native JSON objects rather than through `jq`
- [ ] 4.8 Add `COVERAGE_THRESHOLD_CORE` and `COVERAGE_THRESHOLD_CLI` to both `env:` blocks of the `coverage` job in `build-test-audit-coverage.yml` (`:187-188` Linux/macOS, `:197-198` Windows), matching the script defaults
- [ ] 4.9 Run `scripts/check_coverage.sh -T -p ci --full` (main agent only), confirm all three numbers are reported and all three pass, and compare the combined figure against task 1.4's baseline — any movement is attributable to the exclusions now being applied for the first time and SHALL be explained, not absorbed
- [ ] 4.10 Assert the partition is complete: the two groups' `lines.count` sums SHALL equal the total `lines.count` the same JSON reports, so a mis-filed or dropped file is detected rather than silently excluded

## 5. Submodule Pointer Assertions and Dependabot

- [ ] 5.1 Add a `submodule-pointer` job to `.github/workflows/build-test-audit-coverage.yml` running on `ubuntu-latest`, with `actions/checkout@v6` carrying `submodules: recursive` and `fetch-depth: 0`, and `dtolnay/rust-toolchain@stable`
- [ ] 5.2 In that job, assert the version agreement by reading both values from one `cargo metadata --no-deps --format-version 1` call — `.packages[]|select(.name=="subx-core")|.version` and `.packages[]|select(.name=="subx-cli")|.dependencies[]|select(.name=="subx-core")|.req` — and failing unless the pinned version's major matches the requirement's and its minor is not below the requirement's; comment that `cargo metadata` normalises `version = "1.0"` to `^1.0`, and that the comparison implements the caret rule directly rather than being a general semver solver
- [ ] 5.3 In the same job, assert reachability with `git -C subx-core fetch origin main` followed by `git -C subx-core merge-base --is-ancestor HEAD origin/main`, gated so it is advisory on `pull_request` and blocking on `push` to `master`; add a comment naming the reason — on a pull request the matching `subx-core` commit may legitimately still be in review
- [ ] 5.4 In the same job, assert `.gitmodules` still records `branch = main` for the `subx-core` submodule, since Dependabot's `gitsubmodules` ecosystem and `git submodule update --remote` both need that referent
- [ ] 5.5 Verify each of 5.2, 5.3 and 5.4 by deliberately breaking it on a scratch branch and confirming the job fails with a message naming the specific condition; restore afterwards
- [ ] 5.6 Create `.github/dependabot.yml` with `version: 2` and a single `updates` entry: `package-ecosystem: gitsubmodules`, `directory: "/"`, `schedule: { interval: weekly }`. Do **not** add `cargo` or `github-actions` ecosystems — both are recorded in `design.md`'s Open Questions

## 6. The `subx-core` Repository's Own CI

- [ ] 6.1 Write `subx-core/scripts/quality_check.sh`: `set -euo pipefail`, an optional first argument naming a nextest profile (default `default`), and six invocations — `cargo fmt -- --check`, `cargo check --all-features`, `cargo clippy --all-features -- -D warnings`, `cargo doc --all-features --no-deps --document-private-items`, `cargo test --doc --all-features`, `cargo nextest run --profile "$PROFILE" --features slow-tests`. Keep it deliberately small; it is not a port of the superproject's 356-line script (`design.md` Decision 4)
- [ ] 6.2 `chmod +x subx-core/scripts/quality_check.sh` and run it locally inside a standalone clone of `subx-core` placed **outside** the `subx-cli` tree, confirming it is green
- [ ] 6.3 Write `subx-core/.github/workflows/build-test-audit-coverage.yml` — the same filename as the superproject's, so the two are diffable — with `on: push`/`pull_request` restricted to `main` and `paths-ignore: ['**/*.md']`, and `env: CARGO_TERM_COLOR: always`
- [ ] 6.4 Give it a `test` job over `[ubuntu-latest, windows-latest, macos-latest]`: `actions/checkout@v6` with **no** submodule options (`subx-core` has none), `dtolnay/rust-toolchain@stable` with `rustfmt, clippy`, `taiki-e/install-action@v2` for `cargo-nextest`, three `actions/cache@v5` steps keyed on `hashFiles('Cargo.lock')`, then `scripts/quality_check.sh ci` on Unix; add a PowerShell-invoked equivalent or run the script through `bash` on Windows, whichever keeps one definition of the check set
- [ ] 6.5 Give it a `security` job running `actions-rust-lang/audit@v1.2.7` against `subx-core`'s own committed lockfile, with a comment stating that the superproject audits the workspace **union** and that no consumer of `subx-core` alone ever resolves that union
- [ ] 6.6 Give it a `coverage` job on `ubuntu-latest` only: `cargo-llvm-cov` and `cargo-nextest`, one `cargo llvm-cov nextest --profile ci --features slow-tests --no-report` run, then `cargo llvm-cov report --lcov --output-path lcov.info --ignore-filename-regex '(^|/)(tests|benches)/|(^|/)src/main\.rs$|(^|/)src/test_support/'`, and a Codecov upload. It SHALL **not** enforce a threshold — add a comment giving the reason from `design.md` Decision 9 (a standalone run has the same denominator and a strictly smaller numerator than the workspace-attributed measurement B4's floor was derived from)
- [ ] 6.7 Add no `build` matrix, no cross-compilation, no smoke test, no installer step and no `publish-crates` job; confirm with `grep -n 'cargo publish' subx-core/.github/workflows/*.yml` returning **0**
- [ ] 6.8 Push the core branch and confirm all three jobs are green **before** the pointer is moved in `subx-cli` — validating core before the pointer bump is the entire purpose of this workflow
- [ ] 6.9 Run the coverage job once and record `subx-core`'s **standalone** percentage in the CHANGELOG entry, so a future change can derive a standalone floor from a measurement rather than by scaling B4's workspace-attributed number

## 7. Rework `publish-crates`

- [ ] 7.1 In `.github/workflows/release.yml`, keep the `publish-crates` job's `if: ${{ !contains(github.ref_name, '-') }}` guard (`:190`) and B1's checkout block (`submodules: recursive`, `fetch-depth: 0`) unchanged
- [ ] 7.2 Add a first step **Assert Cargo supports workspace publishing**: read `cargo --version`, compare against `1.90.0` with `printf '%s\n%s\n' "$MIN" "$HAVE" | sort -V -C`, and fail with a message naming the floor. Do not pin the toolchain and do not add `rust-version` to the manifest — `design.md` Decision 1 records why each is wrong
- [ ] 7.3 Add a step **Assert the submodule is committed and clean**: fail on any `-`, `+` or `U` prefix from `git submodule status --recursive`, and on any output from `git -C subx-core status --porcelain`, with a message that names the submodule
- [ ] 7.4 Add a step **Assert `--allow-dirty` is absent**: `grep -rn -- '--allow-dirty' .github/workflows/ && exit 1`, with a comment stating that the flag records `"dirty": true` in the published archive's `.cargo_vcs_info.json`
- [ ] 7.5 Add a step **Dry run**: `cargo publish --workspace --dry-run`
- [ ] 7.6 Add a step **Select publishable members**: read each member's version from `cargo metadata --no-deps`, probe the sparse index (`https://index.crates.io/su/bx/subx-core` and `…/subx-cli`) with `curl -sf … | jq -sr '.[].vers' | grep -qx "$VER"`, fail if `subx-cli`'s tagged version is already present, and export `PUBLISH_ARGS` as either `--workspace` or `--workspace --exclude subx-core`. Treat `curl -sf`'s non-zero exit (HTTP 404, crate never published) as "not present"
- [ ] 7.7 Replace the single `cargo publish --token …` (`:196`) with `cargo publish $PUBLISH_ARGS --token ${{ secrets.CARGO_REGISTRY_TOKEN }}`
- [ ] 7.8 Add a comment block above the job recording the recovery procedure from `design.md` Decision 1: a crates.io upload is irreversible, the job does not retry, and a half-published release is completed with `cargo publish -p subx-cli` rather than by re-running the workspace publish or bumping the already-published member
- [ ] 7.9 Confirm the `build` matrix is untouched and still has exactly five entries, none of them musl: `grep -c 'asset_name:' .github/workflows/release.yml` returns 5 and `grep -c 'musl' .github/workflows/release.yml` returns only the two explanatory comment hits — this keeps the existing `release-distribution` matrix and asset-naming scenarios satisfied

## 8. Version Bump to 2.0.0

- [ ] 8.1 Change `Cargo.toml:8` from `version = "1.9.1"` to `version = "2.0.0"`; leave `subx-core/Cargo.toml`'s `version = "1.0.0"` and `subx-cli`'s `subx-core = { path = "subx-core", version = "1.0" }` unchanged
- [ ] 8.2 Regenerate `Cargo.lock` by running `cargo build` and once more with `cargo build --features archive-rar`; never hand-edit it, and review the diff to confirm the only change is `subx-cli`'s own `version` field
- [ ] 8.3 Confirm `cargo run -- --version` now reports `2.0.0`, and that `src/lib.rs`'s `VERSION` (from `env!("CARGO_PKG_VERSION")`) and `CORE_VERSION` (from `subx_core::VERSION`) report `2.0.0` and `1.0.0` respectively — two distinct constants with two distinct meanings, per B1 Decision 7
- [ ] 8.4 Run `cargo nextest run --workspace -E 'test(version)'` and confirm B1's `core_version_matches_declared_requirement` test still passes with the two versions now diverged
- [ ] 8.5 Add a `## [2.0.0] - <release date>` section to `CHANGELOG.md` directly beneath `## [Unreleased]` (`:9`), folding in every `[Unreleased]` entry A0–B4 accumulated, with `### Added`, `### Changed`, `### Removed`, `### Migration` and `### Documentation`. Its first line under `### Changed` SHALL state that the major bump is a **library**-surface change and that the command-line behaviour, flags, configuration keys and JSON envelopes are unchanged
- [ ] 8.6 Verify the heading mechanically rather than by eye: run `awk "/^## \[2.0.0\]/{flag=1; next} /^## \[/{if(flag) exit} flag" CHANGELOG.md` — the exact expression from `release.yml:29` — and confirm the output is non-empty. A missing heading does not fail the release; `release.yml:32-34` silently substitutes a generic body
- [ ] 8.7 Create `subx-core/CHANGELOG.md` with the Keep a Changelog header, an `## [Unreleased]` section, and a `## [1.0.0] - <release date>` section describing the crate's extraction from `subx-cli`, its public surface, and its GPL-3.0-or-later licence. This answers the open question B1 deferred: C1 is the change that publishes `subx-core@1.0.0`, and a 1.0.0 release with no release record is a defect at the moment of publication
- [ ] 8.8 Commit the moved submodule pointer in `subx-cli` in the same commit as the version bump, so the tree is publish-clean at the tag

## 9. End-to-End Verification

- [ ] 9.1 `cargo fmt -- --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` across both crates
- [ ] 9.2 `cargo nextest run --workspace --profile ci --features slow-tests || true` and confirm both crates' suites ran, by checking the reported test count against task 2.6's
- [ ] 9.3 In a standalone clone of `subx-core` outside the `subx-cli` tree, run `scripts/quality_check.sh` and confirm it is green with no reference to the superproject
- [ ] 9.4 `git clone --recurse-submodules` `subx-cli` into a scratch directory and confirm `cargo build` and `scripts/quality_check.sh -p ci` are green from a cold checkout
- [ ] 9.5 `cargo package --list --allow-dirty -p subx-cli` and confirm no file under `subx-core/` appears in `subx-cli`'s own archive, that `.llvm-cov.toml` is gone from the listing, and that the packaged manifest declares `subx-core` with its `version` and no `path`
- [ ] 9.6 Run `cargo publish --workspace --dry-run` **locally, before any tag is pushed** — this is SDR §12's acceptance criterion for this batch and the last cheap check before an irreversible operation
- [ ] 9.7 Confirm all seven `submodules: recursive` lines and both `fetch-depth: 0` lines B1 added are still present: `grep -c 'submodules: recursive' .github/workflows/*.yml` totals 7 and `grep -c 'fetch-depth: 0' .github/workflows/release.yml` returns 2 (the `submodule-pointer` job adds a third `submodules: recursive` and a third `fetch-depth: 0` in the other file — record the new totals explicitly so the assertion stays meaningful)
- [ ] 9.8 Confirm `grep -rn -- '--allow-dirty' .github/workflows/ subx-core/.github/workflows/` returns **0** in both repositories

## 10. Documentation

- [ ] 10.1 Update `AGENTS.md:36-44` so the `scripts/quality_check.sh` rule states that the script now covers **both** crates, and so the `cargo nextest run || true` instruction notes that `default-members` makes the bare command cover both while the scripts pass `--workspace` explicitly
- [ ] 10.2 Replace `AGENTS.md:47`'s bare "Coverage threshold is **75%** line coverage" with the three floors — combined, `subx-core` and `subx-cli` — naming the environment variables that carry them
- [ ] 10.3 Update `AGENTS.md`'s CI/CD paragraph (`:387-412`) to describe the two-crate publish flow, the `submodule-pointer` job, Dependabot's `gitsubmodules` entry and `subx-core`'s own CI; leave the stale "7 targets" release-matrix claim at `:396-403` for C3, as B1's task 8.3 arranged
- [ ] 10.4 Update `AGENTS.md:159-162`'s script table for the new `--threshold-core` / `--threshold-cli` flags, and add `subx-core/scripts/quality_check.sh` to the repository-layout section B1 added
- [ ] 10.5 Add the same three sections to `subx-core/AGENTS.md`: its own `scripts/quality_check.sh`, its own CI and what that CI deliberately does not do, and the fact that the crate is published from `subx-cli`'s release workflow rather than from its own repository
- [ ] 10.6 Update `docs/tech-architecture.md`'s CI/coverage paragraph (`:627-631`) for two workflows, three coverage floors and the deleted `.llvm-cov.toml`; the full two-crate rewrite remains C3's
- [ ] 10.7 Fold the following into `CHANGELOG.md`'s `## [2.0.0]` section from task 8.5: `### Changed` — the two-crate publish flow, workspace-aware quality scripts, `default-members`, narrowed cache keys, per-crate coverage floors with their measured values; `### Added` — `subx-core` 1.0.0 on crates.io, its own CI, the `submodule-pointer` job, Dependabot; `### Fixed` — the `[profile.default.junit]` path and the two competing `.gitignore` patterns; `### Removed` — `.llvm-cov.toml` from both repositories, with a line stating it was read by nothing and that exclusions are now a `--ignore-filename-regex` argument; `### Migration` — that `subx_cli`'s library modules are now re-exports of `subx-core` and that the binary is unchanged
- [ ] 10.8 Add matching `## [1.0.0]` entries to `subx-core/CHANGELOG.md` for its own CI, its quality script and its standalone coverage figure from task 6.9

## 11. Quality Gate

- [ ] 11.1 Run `cargo fmt` and `cargo clippy -- -D warnings` and fix all warnings
- [ ] 11.2 Run `cargo nextest run --filter-expr 'test(version)' || true` and confirm the targeted modules pass
- [ ] 11.3 Run `scripts/quality_check.sh` once at the end (main agent only — do not invoke from sub-agents) and ensure it is green
- [ ] 11.4 Run `cargo test --doc --all-features` to confirm rustdoc examples still compile
