## Context

C1 made the release event one command over two repositories' worth of code: `cargo publish --workspace` in `subx-cli`'s `publish-crates` job. Its companion specs froze that shape — `crate-topology` forbade a publish job in the core repository outright, and `release-distribution` made `--workspace` the normative mechanism. This change unpicks exactly that knot and must say plainly why the properties C1 wanted survive the unpicking.

## Decisions

### Decision 1 — The safety property is "published before consumed", not "published together"

C1's stated reason for single-pathing publication was that a second publish path "permit[s] a library version to reach the registry that no superproject release pins." Reading the requirement against the actual consumers inverts it: the GUI at `jim60105/subx` consumes `subx-core` without ever resolving `subx-cli`, so for that consumer a core version that *only* arrives pinned by a CLI release is the unusable one. The property that matters at the CLI boundary is narrower and checkable: at the moment `subx-cli@X` is uploaded, the index must already carry a `subx-core` version satisfying the caret requirement `subx-cli`'s manifest declares. That is one HTTP probe of `https://index.crates.io/su/bx/subx-core` — no shared job required.

The residual risk the old requirement guarded is real but different: a core version published by core's own tag may not be the version any CLI release pins. That is acceptable by design — independent version lines are the whole point of the caret — and the gitlink ancestry assertion in the CLI's publish job still guarantees the CLI's *source* corresponds to a core commit on core's `main`.

### Decision 2 — One publish per owning repository, tagged independently

Each repository's `v*` tag is the release trigger for exactly one crate:
- `jim60105/subx-core` tag `v<core-version>` → core's `release.yml` publishes `subx-core`.
- `jim60105/subx-cli` tag `v<cli-version>` → cli's `release.yml` builds the five-target matrix, creates the GitHub Release, and publishes `subx-cli` alone.

Consequences accepted deliberately:
- The core repo's workflow is a *new* file named `release.yml` (not a copy of the superproject's — the superproject's carries a build matrix and installer concerns the library does not have). The "same name so the files stay diffable" rule applies to `build-test-audit-coverage.yml`, which keeps holding; `release.yml` in the two repositories are deliberately different files with deliberately different bodies.
- The published core artifact's provenance is the standalone tree. That is not a regression; it is the tree crates.io, docs.rs and the GUI resolve, and it removes the workspace-membership question (`[profile]` tables, `default-members`, lockfile resolution) from the publish path entirely.

### Decision 3 — The CLI's precondition is a caret-satisfying index probe, not an equality probe

The old job probed for exact `name@version` equality to decide which members to exclude. The new probe answers a different question: *does any published `subx-core` version satisfy `subx-cli`'s caret requirement?* A caret `1.0` accepts any `1.x >= 1.0`, so the probe implements semver caret semantics over the index's version list with plain shell + `jq`: same major, and `(major, minor, patch)` componentwise `>=` the requirement, ignoring pre-release versions. It reads the requirement from `cargo metadata --no-deps`, never from manifest text, keeping the C1 rule "compare Cargo's normalised requirement" alive. A miss fails the job with the exact remedy: *publish `subx-core` first by tagging `v<version>` in `jim60105/subx-core`*.

The `--dry-run` that follows (`cargo publish -p subx-cli --dry-run`) is the backstop: Cargo resolves the stripped manifest against the index, so a probe that passed on a yanked-but-listed version still fails loudly before upload. The probe exists to make the *common* violation (forgot to tag core) legible at second zero, not to replace `--dry-run`.

### Decision 4 — What the CLI job keeps, what it drops, and why each choice is honest

Kept, because each still has a referent once only one crate publishes:
- submodule committed-and-clean assertion (Cargo's publish-time dirty check still walks the submodule; the gitlink still names the core commit this binary was built against);
- mechanical `--allow-dirty` absence guard (still poisons `.cargo_vcs_info.json` if anyone ever adds it);
- hard gitlink-ancestry assertion (`publish` of `subx-cli` still compiles the submodule; a pointer off core's `main` is still unpublished provenance);
- tag-versus-manifest-version assertion and the `subx-cli@version`-already-published probe;
- `--dry-run` before upload; no automatic retry (an upload is still irreversible).

Dropped:
- `--workspace` / `--workspace --exclude subx-core` selection — nothing selects any more; the job publishes one named package;
- the Cargo ≥ 1.90 assertion — the floor was introduced *because* `cargo publish --workspace` needs 1.90. `cargo publish -p` has worked since long before. Asserting a floor the command does not need would be theatre, and the spec text C1 wrote says the floor is "a property of the publishing tool" — with the tool changed, the property is gone. The workflow comment states this so the deletion reads as a decision, not an oversight.
- the half-published recovery comment — the failure mode it documents (member A uploaded, member B failed) cannot occur in a single-upload job. Recovery for a defective single upload is the universal rule: yank, supersede with a new release.

### Decision 5 — The core workflow's preconditions are the transferable subset of the superproject's

The core release job runs: clean-tree assertion (`git status --porcelain` empty — the standalone tree has no submodule, so the submodule guard becomes this one); `--allow-dirty` mechanical absence guard (same grep, same reason); tag-versus-manifest-version assertion; already-published probe for `subx-core@<tagged version>` (fail loudly if the tag names a version already on the index); `cargo publish -p subx-core --dry-run`; then the real `cargo publish -p subx-core`. GitHub Release creation mirrors the superproject's pattern — same awk extraction over the repository's own `CHANGELOG.md`, same `-`-in-tagname ⇒ prerelease rule — so both repositories produce release notes the same way from their own changelogs. The job comment records that `CARGO_REGISTRY_TOKEN` must exist in this repository's Actions secrets (it does).

### Decision 6 — Spec governance: MODIFY the two requirements rather than leave them contradicted

`crate-topology`'s "The core repository SHALL NOT contain a publish job" and `release-distribution`'s "Publication SHALL be performed with `cargo publish --workspace`" are now false as norms, not merely as status reports. Under `spec-governance` they are amended by a change with delta specs — this one — not hand-edited. Both deltas restate their requirements in full (MODIFIED form). The C1-era prose that survives (dirty-check semantics, `--allow-dirty` poisoning, registry-probe-before-upload, no-retry, yank-and-supersede) is carried into the restated text; the C1-era prose that dies with the topology (`--workspace` mechanics, the 1.90 floor, intra-workspace topological ordering) dies in the delta.

### Decision 7 — The changelog rewrite is user-facing, and the link block stays put

Per the project's changelog convention and Keep a Changelog: `[2.0.0]` is rewritten for the person installing or depending on the crates — what the two-crate release means, what did and did not change for binary users and library consumers, the honest state of publication ("will publish when each repository's tag is cut", never "is published"). Proposal IDs (A0–C1), `file:line` citations, CI job internals, and measurement transcripts leave the release section; they remain recoverable from `git log` and the archived changes. `[Unreleased]`'s current entries (spec-split bookkeeping and the JSON-progress-bar fix) fold in: the fix is user-visible and stays named; the spec bookkeeping becomes one documentation bullet. Historical sections `[1.9.1]` and older are byte-frozen; the `[Unreleased]`/`[2.0.0]` compare links stay in the file-end link block, whose `[2.0.0]: …/compare/v1.9.1...v2.0.0` form is a forward reference that becomes live the moment the tag is cut.

## Risks / Trade-offs

- **A core version can exist on the index that no CLI tag ever pinned.** Accepted (Decision 1): it is exactly the version the GUI consumer wanted, and the CLI's probe + caret mean the CLI only ever consumes versions whose repository tag it could name.
- **Tag-order mistake (CLI tagged first).** Fails the CLI job at the probe step before any upload; the remedy is printed. No irreversible state.
- **Index propagation lag between core's publish and the CLI's probe.** `cargo publish` itself waits for its own upload to appear on the index before succeeding, so once core's job is green the version is already readable by the CLI's probe. No sleep logic needed.
- **Two workflows drift apart.** The awk extraction, prerelease rule, and guard comments are intentionally parallel; the delta specs name the shared shape so future drift is a spec-visible difference.

## Migration Plan

Not a code migration — a runbook recorded in the change's tasks and mirrored into AGENTS/docs:
1. Push core `main`; tag `v1.0.0` in `jim60105/subx-core`; core CI publishes `subx-core@1.0.0`.
2. Confirm the index lists it; bump the donor gitlink to the released core commit if it moved; merge/push `subx-cli` `master`; tag `v2.0.0`; CLI CI publishes `subx-cli@2.0.0` after the probe passes.
Rollback: neither step is possible to *undo*, so both jobs front-load every probe before their single upload — the rollback story is the precondition suite.

## Open Questions

None blocking. (Whether the GUI repository adds CI badges against the published crate is that repository's decision.)
