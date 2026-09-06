## Context

By the time this change runs, the split is structurally complete and operationally untouched. `subx-cli` is a Cargo workspace root with `subx-core` mounted as a submodule at `subx-core/`; 36,687 LOC of library code and 40 test files live in a second repository; the `test-support` feature carries 1,529 LOC of shared helpers across a boundary Cargo has no mechanism for; and B4 has measured what each crate's coverage actually is.

None of that reached the pipeline. Four changes in a row deliberately drew the same line, and each of them said so in writing:

- B1: its CI contract was "seven `submodules: recursive` lines and two `fetch-depth: 0` lines — nothing more", and its Decision 8 named the two pointer-staleness mitigations it was leaving here.
- B2: "Bumping `subx-cli` to 2.0.0, reworking `cargo publish`, adding Dependabot, or adding CI to the `subx-core` repository. All C1's."
- B3: "Touching `scripts/check_coverage.sh`, `scripts/quality_check.sh`, the CI environment, `default-members`, or the two junit paths. **C1 implements**."
- B4: an explicit one-list handoff — per-crate threshold plumbing in both coverage scripts, the CI environment variables, the `--ignore-filename-regex` value, the fate of the two `.llvm-cov.toml` files, the junit paths, and `default-members`.

That discipline is why this change is tractable: every decision it makes has a written premise, and none of them requires re-deriving a fact. It is also why the change is large. C1 is the only place in the series where the accumulated CI debt of six predecessors comes due, and it carries a release event on top.

Two facts frame everything below. First, **`subx-cli` cannot be published until `subx-core` is on crates.io**, because `cargo package` strips `path` and leaves a registry requirement for a crate that has never existed there. Second, **`cargo publish` treats a moved submodule pointer as a dirty working directory** and has done since Cargo 1.39; the parent repository is dirty in Cargo's eyes the moment the gitlink moves without being committed, and the flag that silences that is not a workaround but a permanent record in the published archive.

## Goals / Non-Goals

**Goals:**

- Make the release pipeline capable of publishing two coupled crates, in the right order, with the preconditions checked before anything irreversible happens.
- Perform the release event the split has been accumulating toward: `subx-cli` 2.0.0, `subx-core` 1.0.0.
- Close the 43-skipped-test-file hole in the quality gate, so that `scripts/quality_check.sh` is again what AGENTS.md claims it is.
- Wire B4's measured coverage numbers into the scripts, the CI environment and the report, using a mechanism the coverage tool actually reads.
- Give `subx-core` a CI of its own, so a core commit is validated in the repository that received it rather than in the one that eventually points at it.
- Turn the submodule pointer from an unchecked integer into an asserted, monitored, reviewable input.
- Repair the two defects the earlier changes queued behind all of this: the nextest junit path and the over-broad cache-key glob.

**Non-Goals:**

- Changing the release artifact set. The matrix stays at five targets, no musl entry is added, asset names are unchanged, and `scripts/install.sh` is not touched. AGENTS.md's stale "7 targets" claim is C3's, per B1's task 8.3.
- Rewriting documentation prose beyond the CHANGELOG entries and the specific AGENTS.md / `docs/tech-architecture.md` paragraphs this change makes untrue. The two-crate documentation rewrite is C3's.
- Introducing `cargo-release` or `release-please` (Decision 13).
- Touching `openspec/specs/**` capability allocation. C2a and C2b own that.
- Changing any `.rs` file, public item, CLI flag, configuration key, JSON envelope field, error variant or category string in either repository.
- Adding `--all-targets` to the scripts' clippy invocation, or replacing `TestWorkspace`'s `cargo run --` spawning. Both are real improvements and both are recorded in Open Questions.

## Decisions

### Decision 1: `cargo publish --workspace`, with the two-stage publish as a documented recovery path rather than the default

Two shapes can publish two coupled crates.

**The two-stage publish** is what this project would have had to do before Cargo 1.90:

```bash
cargo publish -p subx-core --token "$TOKEN"
cargo publish -p subx-cli  --token "$TOKEN"
```

It works, and its failure mode is well known: the second command resolves `subx-core = "1.0"` against the registry index, which may not yet carry the version the first command just uploaded. Cargo has waited on index propagation since 1.66 ("Waiting on `subx-core` to propagate…"), so this is usually a delay rather than a failure — but the wait has a timeout, and a timeout in a release job leaves exactly the half-published state described below.

**`cargo publish --workspace`** (Cargo ≥ 1.90) packages every selected member, orders them topologically, verifies each against the packaged artifacts of its already-selected dependencies rather than against the registry, and uploads in order. It removes the propagation window entirely, because `subx-cli`'s verification build never asks the index for `subx-core` at all. SDR D5 locks it as the preferred form and it is chosen here.

**The version floor and how CI guarantees it.** `--workspace` on `cargo publish` requires Cargo ≥ 1.90. The `publish-crates` job installs `dtolnay/rust-toolchain@stable`, which today resolves to something far above that — but "today" is not a guarantee, and the failure mode of an under-old toolchain is an `unexpected argument '--workspace'` error in the middle of a tagged release. Three options were considered:

- *Pin the toolchain* (`dtolnay/rust-toolchain@master` with `toolchain: "1.90.0"`). Rejected: it freezes the compiler that builds the verification artifacts at the floor forever, and a pinned old toolchain is a second thing to remember to raise.
- *Declare `rust-version = "1.90"` in the manifest.* Rejected, and this one is worth being precise about: `rust-version` is the crate's **MSRV** — a promise to consumers about what compiles their build of the library. The floor here is a property of the **publishing tool**, not of the source. Encoding one as the other would tell every downstream consumer that they need Cargo 1.90 to use `subx-cli`, which is false.
- *Keep `@stable` and assert.* Chosen. The job runs a guard step before anything else:

  ```bash
  MIN=1.90.0
  HAVE=$(cargo --version | awk '{print $2}')
  printf '%s\n%s\n' "$MIN" "$HAVE" | sort -V -C || {
    echo "cargo $HAVE is older than $MIN, which 'cargo publish --workspace' requires" >&2
    exit 1
  }
  ```

  It turns an unexplained flag error deep in a release into a named precondition failure in the job's first step, and it costs one second.

**What happens when one of two uploads fails partway.** This is the question the two-stage form and `--workspace` answer identically, because the constraint is the registry's, not Cargo's: **a crates.io upload is irreversible.** A version that has been accepted cannot be unpublished, only yanked, and the same `name@version` can never be uploaded again. So if `subx-core@1.0.0` uploads and `subx-cli@2.0.0` then fails verification or upload:

1. The correct move is **not** to re-run the job. A second `cargo publish --workspace` would attempt `subx-core@1.0.0` again and be rejected, and the operator would then be tempted to bump core's version to get past it — creating a `subx-core` release that exists only because a retry needed it to.
2. The correct move is to fix the cause and publish the remaining member alone: `cargo publish -p subx-cli --token …`. At that point `subx-core@1.0.0` genuinely is on the index, so the ordinary single-crate path is correct and the propagation window has already elapsed.
3. If the uploaded `subx-core` artifact is itself the problem, it is **yanked**, core's patch version is bumped, and the pointer moves — a new release, not a repair of the old one.

This procedure is written into the `release-distribution` delta as a normative requirement rather than left as folklore, because it will be needed exactly once, under time pressure, by whoever is watching the release fail.

**Not automated.** The workflow does not retry, does not yank and does not bump. A half-published release is a state that wants a human reading the error, and the automation that would handle it correctly is larger and more dangerous than the situation it addresses.

### Decision 2: the publish job's preconditions run before anything irreversible, and `--allow-dirty` is prohibited rather than discouraged

`create-release` runs first and creates a public GitHub Release; `build` attaches five artifacts; `publish-crates` runs last. By the time the publish job starts, two of the three visible outputs of a release already exist. Everything that can be checked cheaply is therefore checked in `publish-crates`' first three steps, before the first upload:

1. **The Cargo floor** (Decision 1).
2. **The submodule is committed and clean.**

   ```bash
   git submodule status --recursive | grep -E '^[-+U]' && exit 1
   test -z "$(git -C subx-core status --porcelain)"
   ```

   The first line catches the three states `git submodule status` prefixes: `-` (not initialised), `+` (the checked-out commit differs from the one recorded in the index — i.e. the pointer moved and was not committed) and `U` (merge conflict). The second catches an otherwise-clean checkout with modified files inside the submodule worktree. Together they are the two conditions Cargo's recursive dirty check will find, discovered here with a message that says "submodule" rather than there with a message that says "working directory".
3. **`cargo publish --workspace --dry-run`.** Performs the full package-and-verify cycle with no upload. This is the step that would catch a `path`-only dependency, a missing `version` key, an `exclude` list that drops a file the build needs, or a member whose manifest does not survive normalisation. It is the acceptance criterion SDR §12 already sets for this batch.

**`--allow-dirty` is prohibited, not discouraged.** It is the one flag that makes all of the above go away, and its cost is invisible at the moment it is used: `cargo package` records `"dirty": true` in `.cargo_vcs_info.json` inside the `.crate` archive, permanently, for every future reader of that artifact. There is no situation in this project where the correct response to the dirty check is to bypass it — a dirty submodule means the published crate does not correspond to any commit, which is the property the VCS metadata exists to assert. The prohibition is normative in both `release-distribution` and (already) `crate-topology`, and the job carries a mechanical guard:

```bash
grep -rn -- '--allow-dirty' .github/workflows/ && exit 1
```

so that a future edit adding it fails the release rather than succeeding quietly.

### Decision 3: the workflow decides which members to publish, rather than assuming both always need it

At the 2.0.0 tag both members need publishing: `subx-cli` moves 1.9.1 → 2.0.0 and `subx-core` 1.0.0 has never been uploaded. That is not the steady state. Most subsequent `subx-cli` releases will not change `subx-core`'s version, and `cargo publish --workspace` would then attempt to re-upload a version the registry already holds.

Rather than depend on unverified tool behaviour — whether Cargo skips already-published members or errors on them — the job determines the answer itself, from the registry, in eight lines:

```bash
CORE_VER=$(cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name=="subx-core") | .version')
if curl -sf "https://index.crates.io/su/bx/subx-core" \
     | jq -sr '.[].vers' | grep -qx "$CORE_VER"; then
  echo "PUBLISH_ARGS=--workspace --exclude subx-core" >> "$GITHUB_ENV"
else
  echo "PUBLISH_ARGS=--workspace" >> "$GITHUB_ENV"
fi
```

The sparse index is the registry's own source of truth, it needs no authentication, and a 404 (the crate has never been published) is correctly read by `curl -sf` as "not present". `--exclude` is a first-class `cargo publish` flag, so the reduced case is still one command with the same ordering guarantees.

The same probe is run for `subx-cli` and used as an assertion rather than a selector: if `subx-cli@<tagged version>` is already on the index, the tag is a mistake and the job fails loudly instead of half-publishing.

### Decision 4: `subx-cli`'s scripts become workspace-aware; `subx-core` gets a small script of its own rather than a port

The two options the brief poses — make the scripts workspace-aware, or duplicate them into the core repository — are not alternatives here, because they answer different questions.

**`subx-cli`'s scripts must become workspace-aware regardless of what core gets.** After B3, `cargo nextest run` without `--workspace` skips 43 test files and exits 0. That is not a stylistic issue; it is a gate that reports success over a third of the suite it claims to cover. `cargo check`, `cargo clippy` and `cargo doc` have the same shape. Four invocations in `quality_check.sh` (`:227`/`:229`, `:237`/`:239`, `:328`, `:331`) and their four counterparts in `quality_check.ps1` gain `--workspace`. `check_coverage.sh:369` already has it.

**`subx-core` cannot reach those scripts.** A submodule has its own worktree; `subx-core/scripts/` does not exist and the parent's is not on any path a standalone clone can see. Core's CI and a contributor working in a lone clone both need something to run.

**Porting the parent's 356-line and 443-line scripts is rejected.** Those two files contain argument parsing, four nextest profiles, colour handling, a coverage table renderer, a per-file search mode, a JSON summariser and a threshold comparator. Duplicating them creates two implementations of one contract that must be kept in agreement by review, in two repositories, with no mechanism that notices when they drift — and drift is guaranteed, because C1 is *itself* the change editing them.

**Chosen: `subx-core/scripts/quality_check.sh` is written new and deliberately small** — on the order of a dozen lines running `cargo fmt -- --check`, `cargo clippy --all-features -- -D warnings`, `cargo check --all-features`, `cargo doc --all-features --no-deps`, `cargo test --doc --all-features` and `cargo nextest run --profile "${1:-default}" --features slow-tests`, with `set -euo pipefail` and no options beyond an optional profile name. Core's workflow calls the script rather than restating the commands, so there is one definition of core's gate rather than two.

**What this does to AGENTS.md's "single source of truth for QA" rule.** The rule survives, restated precisely rather than weakened: for a change to `subx-cli`, or to the pair, the authority is `subx-cli/scripts/quality_check.sh`, which after this change covers **both** crates. For a change made inside a standalone `subx-core` clone, `subx-core/scripts/quality_check.sh` is the local gate, and the authoritative run still happens in the superproject at the moment the pointer is bumped — which `crate-topology` already requires to be a commit in `subx-cli`. There is one authority; there is now also a cheaper local check that is a strict subset of it. Both `AGENTS.md` files say exactly that.

### Decision 5: `--workspace` in the scripts **and** `default-members` in the manifest — and the hazard that turned out not to exist

`--workspace` in the scripts closes the hole for every gate. It does not close it for the contributor who types `cargo nextest run`, which is precisely what `AGENTS.md:41-44` instructs them to type. Setting `default-members = [".", "subx-core"]` closes that one, by making the bare command mean both crates.

The reason to hesitate was concrete rather than theoretical. Six existing invocations pass a bare feature name — `release.yml:144`'s `cargo build --release --features archive-rar --target …`, `build-test-audit-coverage.yml:133,139`, and `quality_check.sh`'s `--features slow-tests` — and Cargo historically required `package/feature` syntax once more than one package was selected. If that restriction still applied, `default-members` (or `--workspace`) would break the release build.

**It was verified, not assumed.** A scratch workspace of exactly the target shape — edition 2024, `resolver = "3"`, a root package plus one member, `archive-rar` declared on both with the root's as a pass-through, `test-support` declared only on the member — was built and exercised:

| Invocation | Result |
|---|---|
| `cargo check --workspace --features archive-rar` (feature on both) | succeeds |
| `cargo check --workspace --features test-support` (feature on the member only) | succeeds |
| `cargo check --workspace --all-features` | succeeds |
| `cargo check --workspace --features nope` (feature on neither) | fails: `none of the selected packages contains this feature: nope` |
| `default-members = [".", "b"]` + bare `cargo check --features archive-rar` | succeeds |
| `default-members = [".", "b"]` + bare `cargo run` | succeeds, runs the root package's binary |

So a bare feature name applies to whichever selected packages declare it, and errors only when none does — which is the behaviour this project wants, and which makes the pass-through features SDR §4 arranges work exactly as intended across both selection mechanisms.

**Both are applied, and the redundancy is the point.** They protect against different failures with different owners: the manifest key protects a human typing a bare command from a silently partial run, and the explicit flag protects the gate from a future manifest edit that removes the key. Neither subsumes the other, and together they cost one line and eight flags.

**The trade-off, stated honestly.** `default-members` makes the bare `cargo nextest run` slower — 123 test files instead of 80 — and makes `cargo build` walk two manifests. Both are correct costs: the previous speed was the speed of not running a third of the suite. `cargo run`, `cargo install --path .` and the release matrix's `--target` builds are unaffected, as the table above shows.

### Decision 6: one instrumented run, three gated numbers, and where each number comes from

B4's `cross-crate-testing` spec already fixes the contract: instrumentation runs once across the workspace so a `subx-cli` test exercising `subx-core` lines is credited to core, and only the *report* is split. C1 implements it and adds nothing to the contract.

`scripts/check_coverage.sh:369` stays exactly as it is. The two report invocations (`:392` `--lcov`, `:400` `--json --summary-only`) each gain the same flag:

```
--ignore-filename-regex '(^|/)(tests|benches)/|(^|/)src/main\.rs$|(^|/)src/test_support/'
```

**The two must be identical or the gate is lying.** `:392` produces the `lcov.info` uploaded to Codecov and `:400` produces the JSON the threshold is computed from; if only one carries the exclusions, the number CI enforces and the number Codecov displays describe different codebases. The value is B4's, verbatim, including its fourth term — `src/test_support/`, which the dead `.llvm-cov.toml` never mentioned because it did not exist when that file was written.

**The split.** `llvm-cov`'s JSON reports absolute paths in `.data[0].files[].filename`, so the partition is a substring test on `/subx-core/src/`: everything matching is core, everything else under the workspace root is `subx-cli`. Lines covered and lines counted are summed per group and a percentage derived from the sums — not an average of per-file percentages, which would weight a 3-line module the same as a 900-line engine.

**Three thresholds, three meanings:**

| Variable | Default | What it means |
|---|---|---|
| `COVERAGE_THRESHOLD` | 75 | The combined workspace figure. Unchanged, and the gate that may not regress — B4's spec says so and B3 verified that a pure relocation does not move it. |
| `COVERAGE_THRESHOLD_CORE` | B4's derived floor, minimum 75 | `subx-core`'s share, credited across crates. |
| `COVERAGE_THRESHOLD_CLI` | B4's derived floor, minimum 65 | `subx-cli`'s share: `src/cli/` and `src/commands/`, much of it clap derives and terminal rendering, with `src/main.rs` excluded. |

C1 does **not** choose these numbers. B4 measured each crate once with `--profile ci --full`, set each floor to `floor(measured − 3)` clamped to the minima above, and recorded both the measured value and the derived floor in its CHANGELOG entry. C1's task list reads them from there and writes them into the two script defaults and the two CI environment blocks; if B4's derived value is higher than the minimum, the derived value is what gets written, because a floor may be raised and never lowered.

All three are compared, and any one below its floor fails the run. Reporting shows all three regardless of outcome, so a near-miss is visible before it becomes a failure.

### Decision 7: `.llvm-cov.toml` is deleted in both repositories, not wired up

The file exists in `subx-cli` and — since B1 copied it "for parity" — in `subx-core`. Grepping `scripts/`, `.github/` and both manifests in both repositories for `llvm-cov.toml` returns nothing. Its `exclude-from-report`, `output-formats`, `output-dir`, `lcov-output-path`, `include-ffi` and `ignore-filename-regex` keys have never had any effect on any run, before or after the split. Every flag the coverage scripts use is passed on the command line.

Two ways to resolve it:

**Make it load-bearing.** Have the scripts read the exclusion pattern out of the file and pass it to `cargo llvm-cov report`. This satisfies B1's duplication requirement unchanged and gives one editable definition per repository. It is rejected on failure mode: it means a hand-rolled TOML reader in Bash and a second one in PowerShell, and the way that fails is by returning an empty string. An empty `--ignore-filename-regex` does not error — it excludes nothing, inflating both denominators with test and bench lines and *lowering* the measured percentage until someone "fixes" the floor. A silent wrong answer in a gate is worse than three literal copies of a regex.

**Delete both.** Chosen. The exclusion set then has exactly one expression per invocation site — two in `check_coverage.sh`, two in `check_coverage.ps1`, one in core's coverage job — with a normative value recorded in B4's `cross-crate-testing` spec, so a copy that drifts is a spec violation rather than an oversight. And it removes the artifact that caused this in the first place: a file that looks exactly like configuration and is not.

This is also the reading B4's spec already requires — "A configuration file that no script, workflow or manifest references SHALL NOT be relied upon, and its presence SHALL NOT be taken as evidence that its contents are in force." Keeping an inert copy in each repository is keeping the evidence that misled two changes.

**Consequence, handled rather than glossed.** B1's `crate-topology` requirement *Configuration Duplicated into the Core Repository* lists `.llvm-cov.toml` among the files that SHALL be present in `subx-core`. Deleting it makes that requirement false, so this change restates the requirement in full without that bullet — and, while there, adds the two things a submodule equally cannot inherit and which C1 is creating: `.github/workflows/` and `scripts/quality_check.sh`. That is why a third delta spec exists (Decision 14).

### Decision 8: the junit path is repaired for `default` only, and the `ci` profile is verified rather than changed

SDR §5 flagged the `ci` profile. B1 Decision 5a corrected the diagnosis and handed the repair here, and the corrected mechanism is what makes the fix a one-word change: **nextest resolves `junit.path` relative to that profile's store directory, `target/nextest/<profile-name>/`.**

- `[profile.ci.junit] path = "junit.xml"` therefore lands on `target/nextest/ci/junit.xml`, which is exactly what `build-test-audit-coverage.yml:90` and `:213` upload. **Correct today; not touched.**
- `[profile.default.junit] path = "target/nextest/junit.xml"` (`.config/nextest.toml:55-57`) expands to `target/nextest/default/target/nextest/junit.xml`. **This is the defect**, and `.gitignore:19-20`'s two competing patterns — a bare `junit.xml` *and* `target/nextest/*/junit.xml` — are the fossil record of somebody trying to ignore both possible answers.

The repair is `path = "junit.xml"`, in both repositories' copies of the file.

**The table also moves.** `[profile.default.junit]` currently sits at the very bottom of `.config/nextest.toml`, after `[profile.ci]`, `[profile.quick]` and `[profile.full]` — 45 lines away from the `[profile.default]` it configures, and directly beneath a profile it does not configure. That distance is a sufficient explanation for how a wrong path survived; the correct `[profile.ci.junit]` sits immediately under `[profile.ci]` and has been right the whole time. The table moves to directly under `[profile.default]`, in both repositories.

**`.gitignore` collapses to one pattern.** `/target/` is already ignored root-anchored, so `target/nextest/*/junit.xml` was always subsumed and is deleted. The bare `junit.xml` line is kept, because it is the one that catches a junit file written somewhere other than `target/` — which is what the defect was producing.

**Verification is empirical, not by reading.** The task list runs `cargo nextest run --workspace --profile ci -E 'test(version)'` and asserts `target/nextest/ci/junit.xml` exists, and the same for `default`. B1's Decision 5a was itself a correction of a confident reading; this change does not repeat that pattern.

The Chinese comments in `subx-cli/.config/nextest.toml` (`:7`, `:10`, `:13`, `:16`, `:19`, `:22`, `:26`, `:34`, `:40`, `:47`, `:56`) are rewritten in English at the same time. B1 wrote the core copy in English and deliberately left the parent's divergent so its own diff stayed confined to checkout steps; this is the change that closes that gap, and it is the same file being edited anyway.

### Decision 9: the shape of `subx-core`'s CI, and the two places it may diverge

Without a workflow of its own, a `subx-core` commit is compiled by nothing until somebody moves the pointer. The first signal arrives inside a `subx-cli` pull request, attributed to the wrong repository, after the fact. `subx-core/.github/workflows/build-test-audit-coverage.yml` — named after the parent's so the two remain diffable with `diff` rather than by reading — fixes that with three jobs.

**`test`, and why the matrix is not narrowed.** `[ubuntu-latest, windows-latest, macos-latest]`, the same three the parent runs. The temptation is to run core on Linux only and let the parent cover the rest, and it is wrong for this codebase specifically: core is where every platform-sensitive path now lives — encoding detection, path handling and case sensitivity, audio decoding, VAD, archive extraction. Narrowing the matrix would move the first Windows signal for those modules to a pointer bump, which is the exact failure this workflow exists to prevent. The job checks out (no submodules — `subx-core` has none), installs the toolchain plus `cargo-nextest`, caches on `hashFiles('Cargo.lock')`, and runs `scripts/quality_check.sh ci`.

**`security`.** `actions-rust-lang/audit` against `subx-core/Cargo.lock`. This is not redundant with the parent's audit: the parent audits the **workspace** lockfile, which is the union of both manifests' resolutions. A consumer of `subx-core` alone — the Tauri GUI, `docs.rs`, anyone running `cargo add subx-core` — resolves core's own, smaller graph, and that graph is what core's lockfile records. Auditing only the union means core's actual published surface is audited by nobody. This is normative in the `supply-chain-hardening` delta.

**`coverage`, which measures but does not gate.** Ubuntu only. It runs the same single instrumented pass with the same `--ignore-filename-regex` and uploads to Codecov — and enforces **no floor**. The reason is arithmetic: B4's `subx-core` floor was derived from a *workspace-attributed* measurement, in which the 80 `subx-cli` test files that drive `commands::match_command::execute` credit their execution of `subx_core::core::matcher` to core. A standalone run has the same denominator and a strictly smaller numerator. Enforcing the workspace-derived floor there would fail permanently; inventing a second, lower floor would mean inventing a number nobody has measured. So core's own job reports and uploads, the enforcing gate stays where the whole picture exists, and this change's task list **measures and records** core's standalone figure so that a future change can set a standalone floor from evidence.

**What core's CI does not have, and why.** No `build` matrix, no cross-compilation, no smoke test, no `install.sh` step, no release job and — the one that deserves stating — **no `publish-crates`**. `subx-core` is published by the parent's `cargo publish --workspace`, because that is the mechanism D5 chose and because the submodule pointer is what defines which core commit a given `subx-cli` release corresponds to. Two publish paths for one crate would allow a `subx-core` version to reach crates.io that no `subx-cli` release pins, which is the same class of drift the pointer assertions exist to catch.

**How far the two may diverge before they are two projects.** The rule written into `crate-topology`: core's workflow SHALL run the same checks over the same platform matrix as the parent's `test` job, and MAY differ only in (a) omitting jobs that build or publish an artifact core does not produce, and (b) reporting rather than gating a measurement whose reference value was derived elsewhere. Any other divergence is a drift to be reconciled, not a local choice. That is a narrow enough licence to keep the files diffable and wide enough to cover the two differences this change actually introduces.

**The cost, named.** Core's release cadence is coupled to the parent's: a core-only fix still needs a pointer bump and a `subx-cli` release to reach crates.io. That is a real constraint on D6's "independent versions" — the *versions* are independent, the *release events* are not — and it is recorded in Open Questions with the shape a decoupling would take.

### Decision 10: pointer staleness gets three checks, because there are three different ways to be wrong

B1 installed the two structural mitigations (`branch = main` in `.gitmodules`, documented `git config submodule.recurse true`) and named the two it left here. Implementing them properly means recognising that "the pointer is wrong" is three distinct conditions:

**(a) The pinned version does not satisfy the declared requirement.** `subx-cli` asks for `^1.0`; the pinned commit could declare `2.0.0`. Nothing in Cargo notices, because inside the workspace `path` wins outright and the `version` key is never consulted for resolution. B1 added a local unit test asserting the major component; the CI job is its counterpart, reading both values from the same source rather than from a grep:

```bash
META=$(cargo metadata --no-deps --format-version 1)
CORE=$(jq -r '.packages[]|select(.name=="subx-core")|.version' <<<"$META")
REQ=$(jq -r '.packages[]|select(.name=="subx-cli")|.dependencies[]|select(.name=="subx-core")|.req' <<<"$META")
```

`cargo metadata` normalises `version = "1.0"` into the explicit requirement `^1.0`, so the comparison is against a canonical form rather than against whatever was typed. The check asserts the pinned version's major matches the requirement's, and its minor is not below the requirement's — the caret rule, implemented directly, with its limits stated in a comment rather than pretending to be a general semver solver.

**(b) The pinned commit is not reachable from the tracked branch.** A pointer can name a commit that was force-pushed away, or one that lives on a core branch that was never merged. `git -C subx-core fetch origin main` followed by `git -C subx-core merge-base --is-ancestor HEAD origin/main` catches both.

This check cannot be hard everywhere, and pretending otherwise would make it useless. On a **pull request**, the matching `subx-core` commit is legitimately still in review, so the check reports and does not fail. On a **push to `master`** and inside **`publish-crates`**, it is a hard failure — those are the two moments where a pointer into nowhere becomes permanent. Stating that split is what makes the check survive contact with the workflow it is meant to protect.

**(c) The pointer is behind and nobody noticed.** Neither assertion above fires when the pointer is simply old. That is a monitoring problem, not an assertion problem, and it is what `.github/dependabot.yml` is for:

```yaml
version: 2
updates:
  - package-ecosystem: gitsubmodules
    directory: "/"
    schedule:
      interval: weekly
```

Dependabot's `gitsubmodules` updater proposes a pull request moving the pointer to the tip of the tracked branch — which is precisely why B1 insisted on `branch = main` in `.gitmodules`, since without it "the tip" has no referent. The proposed bump then runs through checks (a) and (b) like any other change.

**`cargo` and `github-actions` ecosystems are deliberately not added.** Both are defensible additions and neither is this change's. A `cargo` ecosystem entry interacts with the `cargo audit` gate and with two lockfiles, and a `github-actions` entry would open pull requests against the same workflow files this change is rewriting. Recorded in Open Questions.

### Decision 11: cache keys narrow to the workspace lockfile; the audit surface gains a second one

These look like the same issue and are opposites.

**Cache keys narrow.** Four steps in `build-test-audit-coverage.yml` (`:43`, `:49`, `:55`, and their `archive-rar` counterparts at `:118`, `:124`, `:130`) key on `hashFiles('**/Cargo.lock')`. Since B1 Decision 5b, `subx-core/Cargo.lock` is committed — so the glob now matches two files and combines both hashes. That is *more* invalidation than the build justifies: inside the workspace, `subx-core/Cargo.lock` is inert, because the workspace root's lockfile is the only one Cargo reads or writes. A change to core's standalone resolution that does not change the workspace resolution would evict every cache for nothing. The keys narrow to `hashFiles('Cargo.lock')`, which is exactly what the build resolves against. Nothing is lost: a core manifest change that *does* affect the workspace resolution changes the root lockfile by construction. This costs one cache-key churn, which B1 already predicted and accepted.

**The audit surface widens.** `cargo audit` in the parent resolves the **workspace** lockfile — the union of both manifests. That is the right surface for the shipped binary, and B1 correctly added `submodules: recursive` to the `security` job so the member manifest is present. It is the wrong surface for `subx-core` as a published crate, because no consumer of core alone ever resolves the union. Core's own repository audits its own lockfile (Decision 9), and `supply-chain-hardening` states the general rule: every crate this project publishes SHALL have its dependency graph audited in the repository that publishes it, against the lockfile that repository commits.

**Why the parent's `security` job needs the submodule at all,** stated in the spec rather than assumed: `cargo audit` reads `Cargo.lock`, but any lockfile regeneration, any `cargo metadata` call used to distinguish direct from transitive dependencies, and any `--deny` mode that relies on that distinction must load the workspace manifest — which fails outright when `subx-core/Cargo.toml` is absent.

### Decision 12: `subx-cli` 2.0.0, `subx-core` 1.0.0, and the header shape the release workflow parses

SDR D6 locks both numbers and B1 Decision 6 argued them. What C1 adds is the mechanics.

**`Cargo.toml:8` becomes `version = "2.0.0"`.** The major bump is about the *library* surface: `subx_cli::{config, core, error, services}` are now re-exports of another crate, `exit_code` and `user_friendly_message` became trait methods on `subx_cli::cli::error_ext::SubXErrorExt` in A2, and the crate is no longer where the engines live. The **binary** is unchanged in every observable respect, which is why the bump reads as surprising until you remember the crate has always had two audiences. The CHANGELOG entry says so in the first line, because the first question a user will ask is what broke.

**`subx-core` stays at 1.0.0 and `subx-cli` keeps asking for `^1.0`.** The version lines are independent by construction (`crate-topology` forbids either inheriting from the other), and nothing about the CLI's major bump implies anything about core's.

**`Cargo.lock` is regenerated by running Cargo, never hand-edited** — A0's `supply-chain-hardening` requirement, which applies to a version bump exactly as it applies to a dependency change.

**The CHANGELOG header is a machine contract.** `release.yml:29` extracts release notes with

```awk
awk "/^## \[$VERSION_NUM\]/{flag=1; next} /^## \[/{if(flag) exit} flag" CHANGELOG.md
```

where `VERSION_NUM` is the tag with its `v` stripped. So the section must begin with `## [2.0.0]` at column zero — the trailing ` - <date>` is fine, since the pattern is a prefix match — and must be terminated by the next `## [` heading. A missing or misspelled header does not fail the release; `release.yml:32-34` silently substitutes `Release v2.0.0`, which is worse than a failure because it ships. The task list therefore *runs the awk expression* against the edited file and asserts non-empty output, rather than checking the heading by eye.

**`subx-core/CHANGELOG.md` is created here, answering an open question B1 deferred.** B1 asked whether core wants a changelog from its first commit or from B2, and left it to C3. C1 has to answer it, because C1 is the change that publishes `subx-core@1.0.0` to crates.io, and a 1.0.0 release with no release record is a defect at the moment of publication rather than a documentation gap to be tidied later. C1 creates the file with a `## [1.0.0]` section describing the crate's extraction from `subx-cli`; C3 still owns the prose rewrite across both repositories.

### Decision 13: neither `cargo-release` nor `release-please` is introduced, and that is recorded as a constraint on future tooling

The flow today is manual: bump the version, write the CHANGELOG section, push a `v*` tag, and the workflow does the rest. Two obvious pieces of automation exist and neither is compatible with this project's shape:

- **`cargo-release`** explicitly refuses workspace members that are git submodules. Its whole model is committing and tagging across a workspace from one repository, and a member with its own index and its own history is outside that model.
- **`release-please`**'s rust strategy parses `Cargo.toml` and `Cargo.lock` across a workspace and does not model submodules at all. It would either miss `subx-core` or treat `subx-core/Cargo.toml` as a path inside the parent repository, and its generated commits would not move the gitlink.

Neither is in use today, so this costs nothing now. It is recorded because it is the kind of constraint that gets discovered at the worst moment — by someone adding release automation six months from now and finding that the tool cannot express the repository. The `release-distribution` delta states it normatively: release tooling introduced later SHALL be verified to handle a workspace member that is a git submodule, and SHALL NOT be adopted on the basis that it works for ordinary workspaces.

### Decision 14: no new capability; `crate-topology` is amended instead

The brief left this open, and the answer is no — with one consequence that needs stating.

The work in this change divides into three, and each part already has a capability whose subject it is:

| Part | Capability | Why it belongs there |
|---|---|---|
| Publish flow, ordering, preconditions, version bump, changelog header, `archive-rar` parity | `release-distribution` | It is literally "how SubX-CLI release artifacts are produced, named, validated, documented and consumed". Publication to crates.io is the one release channel that spec did not yet cover; adding it completes the capability rather than straining it. |
| Audit surface, pointer freshness and provenance | `supply-chain-hardening` | The pointer is a dependency reference in every sense that matters — it names external code by an identifier that can be stale, wrong, or unreachable. A0 already widened this capability from "advisories" to "the resolved graph is the audited surface". |
| Core's own CI, and which per-repository files must exist | `crate-topology` | B1 created this capability to describe the relationship between the two repositories, and it already contains *Submodule Checkout in Every CI Job* and *Configuration Duplicated into the Core Repository*. Core's workflow is a file a submodule cannot inherit; it belongs in the same list. |

A fourth capability — say `two-repository-ci` — would take the submodule contract, which is currently readable as one document, and split it so that "every job checks the submodule out" lives in one spec and "the core repository has its own CI" lives in another. Nobody reading either half would have the whole rule.

**The consequence.** Amending `crate-topology` means this change writes a third delta spec, beyond the two the brief named. It is unavoidable and it is small: one requirement restated in full with one bullet removed and two added (Decision 7), plus one new requirement for core's CI (Decision 9). The alternative — deleting `.llvm-cov.toml` while a sibling change's spec requires it to exist — would leave two changes in the same series contradicting each other, which is the specific failure the shared decision record exists to prevent.

## Risks / Trade-offs

- **Risk: `cargo publish --workspace --dry-run` behaves differently from the real publish, and the first genuine failure is discovered at the tag.** → Mitigation: the dry-run performs the full package-and-verify cycle including the cross-member resolution that is the novel part here, and it is run **locally, before the tag is pushed**, as an explicit task — not only inside the workflow where its first execution would be during a release. SDR §12 already sets it as batch 6's acceptance criterion.
- **Risk: `subx-core@1.0.0` uploads and `subx-cli@2.0.0` then fails, leaving a half-published release.** → Mitigation: Decision 1 fixes the recovery procedure in advance and writes it into the spec, so the operator is not deciding under pressure; every cheap precondition (Cargo floor, submodule cleanliness, dry-run, member selection, tag-already-published assertion) runs before the first upload, so the residual failure surface is verification failures that the dry-run would already have caught.
- **Risk: the ancestry check (Decision 10b) fires on every pull request whose core commit is still in review, and gets disabled.** → Mitigation: it is soft on pull requests by design and hard only on `master` pushes and in `publish-crates`. A check that must be ignored routinely is a check that will be ignored when it matters.
- **Risk: `default-members` changes the meaning of a bare Cargo command for every contributor, and something depends on the old meaning.** → Mitigation: the six feature-passing invocations were verified against a scratch workspace of the target shape before the decision was made (Decision 5's table), and `cargo run`, `cargo install --path .` and the `--target` release builds were included in that verification. The task list repeats the checks in the real tree before the key is committed.
- **Risk: `--workspace` in `quality_check.sh` turns the gate red on lint or test failures in `subx-core` that were never being checked.** → Mitigation: that is the change working, not failing — but it does mean the gate can go red for reasons unrelated to this change's edits. B3 and B4 both verified `cargo clippy --workspace --all-targets -- -D warnings` and `cargo nextest run --workspace` green in their own task lists, so the expected number of surprises is zero; the first `--workspace` run happens early in this change's task list rather than at the end, so a surprise is discovered while there is still budget.
- **Risk: the per-crate coverage partition mis-files a path and one crate's number is silently wrong.** → Mitigation: the partition is a substring test on `/subx-core/src/` over absolute paths, and the task list asserts that the two groups' line counts sum to the total the same JSON reports — a mis-filed or dropped file makes the sums disagree.
- **Risk: deleting `.llvm-cov.toml` removes an exclusion that was in force after all, and coverage moves.** → Mitigation: it was never in force — three independent greps (B3's, B4's, and this change's, over `scripts/`, `.github/` and both manifests in both repositories) return nothing. The task list re-runs the grep immediately before the deletion, and the coverage figure is measured before and after the `--ignore-filename-regex` is added, so the change attributable to the exclusions is observed rather than assumed.
- **Risk: core's CI diverges from the parent's over time until the two are separate projects with separate conventions.** → Mitigation: Decision 9 writes the divergence licence into `crate-topology` as two named exceptions, and the two workflow files carry the same name so `diff` is the review tool. A divergence outside those two exceptions is a spec violation rather than a judgement call.
- **Risk: the release cadence coupling (core can only reach crates.io through a `subx-cli` release) becomes painful once the GUI depends on `subx-core` directly.** → Mitigation: accepted deliberately (Decision 9), recorded in Open Questions with the shape of the decoupling, and cheap to revisit — adding a core-side publish workflow later is additive and breaks nothing that exists now.
- **Trade-off: three literal copies of the `--ignore-filename-regex` value.** → Accepted (Decision 7). The alternative is a hand-rolled TOML reader whose failure mode is an empty pattern and a silently wrong number. The value is normative in B4's spec, so a drifting copy is a spec violation.
- **Trade-off: `subx-core` gets a second, smaller quality script rather than a port of the parent's.** → Accepted (Decision 4). Two full implementations of one contract in two repositories would drift, and C1 is itself the change editing the parent's.
- **Trade-off: core's coverage job gates nothing.** → Accepted (Decision 9). A floor derived from a workspace-attributed measurement cannot be enforced over a standalone numerator, and inventing a second number would be inventing evidence. This change measures and records the standalone figure so a later change can set that floor honestly.
- **Trade-off: a third delta spec beyond the two the brief named.** → Accepted (Decision 14). Deleting a file a sibling change's spec requires, without amending that spec, is a contradiction inside one series.

## Migration Plan

Each step leaves the repository in a state that either builds and gates correctly or fails loudly, and the two irreversible steps are last.

1. **Baseline.** Confirm B4 landed and read its recorded numbers. Run `scripts/quality_check.sh` and `scripts/check_coverage.sh -T -p ci --full` once each to record the pre-change verdicts — including, deliberately, the fact that the test count reflects only `subx-cli`.
2. **Close the gate hole first.** Add `--workspace` to the four script invocations and their four PowerShell counterparts, add `default-members`, and re-run the gate. This is the step most likely to surface a surprise, and it is sequenced first so that a surprise is discovered with the whole budget still available.
3. **Repair the small defects.** Junit path and table position in both repositories, `.gitignore` collapse, Chinese comments in the parent's nextest config, `.llvm-cov.toml` deletion in both, cache-key narrowing. Each verified individually.
4. **Wire coverage.** `--ignore-filename-regex` on both report invocations in both scripts, the per-crate partition, the three thresholds, the CI environment blocks. Measure before and after so the effect of the exclusions is observed.
5. **Add the pointer machinery.** The `submodule-pointer` job and `.github/dependabot.yml`. Verify each assertion by deliberately breaking it in a scratch branch and confirming it fires.
6. **Build core's CI.** `subx-core/scripts/quality_check.sh`, then the workflow that calls it. Push to core and confirm green **before** the pointer moves, which is the whole point of the workflow existing.
7. **Rework `publish-crates`.** All six steps, verified with `cargo publish --workspace --dry-run` locally. Nothing is uploaded.
8. **Bump and document.** `Cargo.toml` to 2.0.0, `Cargo.lock` regenerated by Cargo, `CHANGELOG.md`'s `## [2.0.0]` section verified by running the workflow's own `awk` expression against it, `subx-core/CHANGELOG.md` created, both `AGENTS.md` files and `docs/tech-architecture.md`'s CI paragraph updated.
9. **Quality gate**, main agent only.

**Rollback** is `git revert` in each repository for everything through step 8; the version bump reverts as cleanly as any other manifest edit. The one step that cannot be reverted is the tag and the publish that follows it, which is why step 7's dry-run is a precondition of pushing the tag rather than a step inside the release.

## Sizing

Estimated at **~9.5 h** — one long workday, at the top of the budget rather than over it, and comparable to B3's measured 9 h.

| Phase | Estimate |
|---|---|
| Baseline, precondition checks, reading B4's recorded numbers | 0.25 h |
| Workspace-aware scripts (`.sh` + `.ps1`), `default-members`, CI `--workspace` | 1.25 h |
| Junit path, `.gitignore`, comment translation, `.llvm-cov.toml` deletion, cache keys | 0.5 h |
| Coverage wiring: regex, per-crate partition, three thresholds, both ports, CI env | 1.75 h |
| `submodule-pointer` job and Dependabot, each assertion verified by breaking it | 0.75 h |
| `subx-core`'s quality script and its CI workflow, pushed and green | 1.25 h |
| `publish-crates` rework, six steps, with a local `--workspace --dry-run` | 1.5 h |
| 2.0.0 bump, `Cargo.lock`, both CHANGELOGs, awk verification | 0.75 h |
| End-to-end verification across both repositories | 0.75 h |
| Documentation | 0.5 h |
| Quality gate | 0.25 h |
| **Total** | **9.5 h** |

**If it does not fit, the seam is phase 6 — `subx-core`'s own CI.** It is the one body of work that touches no file in `subx-cli`: a new script and a new workflow, both inside the core repository, neither of which anything in the parent depends on. Lifting it out would leave C1 at ~8.25 h and would delay only the "core commits are validated before the pointer moves" property, which is a *detection* improvement rather than a correctness one — the parent's `--workspace` gate still catches every core regression at the pointer bump. It is named here rather than allowed to overflow silently, which is the call B3 made and the reason B4 exists.

The item most likely to overrun is the coverage wiring, and specifically the PowerShell port: `check_coverage.ps1` parses the JSON natively rather than through `jq`, so the per-crate partition has to be written twice in two idioms with no shared code. It is sequenced after the Bash version so the second is a transcription of a working design rather than a parallel invention.

## Open Questions

- **Should the quality scripts' clippy invocation gain `--all-targets`?** Today it lints neither tests nor benches in either crate — 23,006 LOC of test code is outside the `-D warnings` gate entirely. B3 and B4 both run `cargo clippy --workspace --all-targets -- -D warnings` in their verification steps, so the tree is known clean, which makes this a cheap and real improvement. It is left out here because widening the gate and rewriting the pipeline in one change makes an unrelated failure hard to attribute. Worth its own small change.
- **Should `subx-core` eventually publish itself?** Decision 9 routes core's publication through the parent's `cargo publish --workspace`, which couples core's release cadence to `subx-cli`'s. If the Tauri GUI's need for core releases outpaces the CLI's, the decoupling is a core-side workflow triggered by a `v*` tag in `subx-core`, gated on the tagged commit being the one `subx-cli`'s pointer names — which is checkable, and which would keep the "no core version exists that no `subx-cli` release pins" property. Not needed today.
- **Should `subx-core`'s coverage job gain a standalone floor?** This change measures and records the standalone figure precisely so that question can be answered from evidence rather than by scaling B4's number. A floor set at `floor(standalone measured − 3)` would be the same ratchet policy B4 established, applied to a second, honestly separate measurement.
- **Should `cargo` and `github-actions` Dependabot ecosystems be added?** Both are defensible. The `cargo` ecosystem interacts with the audit gate and with two lockfiles that are updated by different mechanisms; `github-actions` would open pull requests against workflow files this change is rewriting. Adding either is a small, independent change once this one has settled.
- **Does the release matrix want its own submodule-pointer assertion?** `build` currently checks out with `submodules: recursive` and builds five artifacts. If the pointer were wrong, the artifacts would be built against the wrong core and the assertion in `publish-crates` would fire only after they were already attached to a public release. Moving the version-agreement check into `create-release` — before anything is published — is arguably better, and is a one-step move if the ordering proves to matter.
