# Tasks — publish-subx-core-from-own-ci

## 1. subx-core repository (committed in the submodule; push pending — see Execution Notes)

- [x] 1.1 Create `subx-core/.github/workflows/release.yml`: `v*` tag trigger; `create-release` job with the CHANGELOG awk extraction over the core repository's own `CHANGELOG.md` and the `-`-in-tagname prerelease rule; `publish-crates` job gated off prerelease tags, with clean-tree assertion, mechanical `--allow-dirty` absence guard, tag-matches-manifest + already-published probes, `cargo publish -p subx-core --dry-run`, then the real `cargo publish -p subx-core --token $CARGO_REGISTRY_TOKEN`; comments record the token requirement, the recovery rule, and why no Cargo floor exists.
- [x] 1.2 Rewrite `subx-core/CHANGELOG.md` `[1.0.0]` for library consumers (modules/APIs, reporting seam, `Send + Sync` formats, `test-support` feature), pending wording flipped to honest release-prep ("publishes from this repository's own release tag; until that tag's run completes, consumable from its git URL") — no proposal IDs, no internal jargon.
- [x] 1.3 Add crates.io + docs.rs badge rows to `subx-core/README.md`; correct its Installation prose to own-tag publishing.
- [x] 1.4 Replace the "no release workflow in this repository and none is planned" paragraph in `subx-core/AGENTS.md` (outside the `SHARED_CONVENTIONS` region) with the own-`v*`-tag release workflow description + ordering contract.

## 2. subx-cli publish job restructure

- [x] 2.1 `.github/workflows/release.yml` `publish-crates`: rename to "Publish subx-cli to crates.io"; rewrite the recovery comment for the single-upload job; drop the Cargo ≥ 1.90 assertion (floor belonged to `--workspace`), the `--workspace` dry run, the member-selection probe, and `$PUBLISH_ARGS`; keep submodule clean/`--allow-dirty`/gitlink-ancestry guards.
- [x] 2.2 Add the precondition step: read the caret requirement for `subx-core` from `cargo metadata` (`.kind == null` = normal dep), probe `https://index.crates.io/su/bx/subx-core` for a published non-pre-release version satisfying it (same major, componentwise `>=` the stripped bound via `sort -V`); fail with the "tag v<X> in jim60105/subx-core first" remedy on a miss. Probe logic exercised locally against 14 version-list/requirement cases.
- [x] 2.3 Keep tag-versus-manifest + already-published(`subx-cli`) probes as their own step; `cargo publish -p subx-cli --dry-run`; single real upload `cargo publish -p subx-cli --token ${{ secrets.CARGO_REGISTRY_TOKEN }}`.

## 3. Donor docs, changelog, badges

- [x] 3.1 Rewrite `CHANGELOG.md` `[2.0.0]` user-facing per the changelog convention (headline: two crates; binary behaviour unchanged from 1.9.1; library surface added; honest present-future publication wording per-repository); fold `[Unreleased]` in (JSON-progress fix named; spec-split bookkeeping one Documentation bullet); remove proposal IDs / file:line citations / CI internals; `[1.9.1]`-and-older bytes and the end-of-file link block untouched (verified byte-identical below the section).
- [x] 3.2 Rewrite the "Releases publish **both crates**" paragraph in `AGENTS.md` for per-repository publishing + ordering contract (outside the shared region).
- [x] 3.3 Update `docs/tech-architecture.md` CI/CD + Distribution prose for the new topology, still honestly not-yet-published.
- [x] 3.4 `README.md` + `README.zh-TW.md`: `subx-core` crates.io/docs.rs badge rows in the library section; prose corrected to own-tag publishing (badges render "not found" until tags are cut — accepted release-prep state).

## 4. Verification

- [x] 4.1 `openspec validate publish-subx-core-from-own-ci --strict` passes; `openspec validate --all --strict` and `--specs --strict` green.
- [x] 4.2 `scripts/quality_check.sh --check-spec-governance` green (shared region between the two AGENTS.md still byte-identical).
- [x] 4.3 YAML-validity of both touched/created workflow files; caret probe logic unit-tested locally; `cargo metadata` requirement extraction verified against the real manifest.
- [x] 4.4 `cargo check --workspace` sanity (no code changed; guards manifest/lockfile untouched).
- [x] 4.5 Workflow awk extraction over both rewritten CHANGELOGs yields the clean `[2.0.0]` / `[1.0.0]` sections; no proposal-ID or CI-internals leakage; `[1.9.1]`-and-older tail byte-identical (`cmp` proof).

## Execution Notes

- **No push, no tag, no publish** anywhere in this stage (user directive). Both repos' release workflows stay untriggered.
- **subx-core landing state**: the core-side edits (tasks 1.1–1.4) were pushed to the core repository's default branch (renamed `main` → `master` afterwards), and the donor gitlink now names the pushed core commit. Landing order (coordinator): push core `master` → bump the donor gitlink to the pushed core commit → donor CI green → then the tags per the runbook (core `v1.0.0` first, CLI `v2.0.0` after core's publish job is green).
- Badge rows for `subx-core` (and the already-present `subx-cli` crates.io badges) render "not found" until the tags publish — the accepted release-prep state per C3's deferred badge decision.
