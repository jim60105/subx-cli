## Context

The v1.7.0 release added Linux musl artifacts (`subx-linux-x86_64-musl`,
`subx-linux-aarch64-musl`) under the assumption that they could be
cross-compiled on the same matrix that produces the gnu artifacts. The
v1.7.1 release confirmed this assumption was wrong: both musl matrix
jobs failed at link time because **`ort` 2.0.0-rc.10 ships no
musl-targeted ONNX Runtime binaries**. `ort-sys` resolves prebuilt
runtime archives from a CDN-hosted manifest (`dist.txt`) which contains
exactly zero `*-unknown-linux-musl` rows; without an upstream
distribution there is nothing for `ort-sys` to download, and link
errors follow.

The v1.7.2 hot-fix removed the musl matrix entries from the
implementation (`Cross.toml`, `release.yml`, `scripts/install.sh`,
both READMEs, `CHANGELOG.md`) and shipped a successful gnu-only
release. However, the spec layer
(`openspec/specs/release-distribution/spec.md`) still mandates four
Linux artifacts including both musl variants, and still permits
`SUBX_LIBC=musl` / `--musl` as supported install paths. This change
reconciles the spec with the implementation.

**Constraints:**
- We cannot fix the upstream gap. ONNX Runtime upstream does not ship
  musl binaries, and source-building it for musl requires a different
  build system, hours of CI time, and ongoing maintenance — outside
  the scope of this hot-fix series.
- We must not silently 404 against deleted assets. The installer must
  fail loudly when a musl path is requested, with guidance.
- We must keep `cargo install subx-cli` viable for musl users, since
  that route compiles ONNX Runtime against the host toolchain and
  works on Alpine when the user has the relevant `apk` packages
  installed.

**Stakeholders:**
- Alpine / Void musl / OpenWrt / NixOS-musl users who used to (try to)
  `curl … | bash` the installer. They keep `cargo install subx-cli`
  as their supported path.
- Release engineers (CI). The matrix shrinks from 4 → 2 Linux jobs,
  reducing release wall-clock time and matrix flake surface.
- Future contributors who would otherwise reintroduce musl from the
  spec without checking upstream availability.

## Goals / Non-Goals

**Goals:**

- Update the `release-distribution` spec so that requirements,
  scenarios, and the asset-naming grammar all reflect a gnu-only
  Linux artifact contract.
- Make the installer's musl-rejection behavior a first-class
  contractual requirement (rather than implicit-by-omission), so a
  future regression that makes the installer silently fall through to
  a non-existent asset would be caught by the spec/test suite.
- Preserve every requirement that is still relevant after musl
  removal — exact-name asset matching, smoke-tests, fallback
  diagnostics, backward-compatibility guarantees for x86_64 Linux and
  macOS hosts.
- Document the decision (and its reversibility) in CHANGELOG and the
  proposal/design pair, so the next time someone wonders "why no
  musl?", the answer is one git-blame away.

**Non-Goals:**

- Do **not** attempt to source-build ONNX Runtime for musl. That is a
  separate, much larger change.
- Do **not** remove the `archive-rar` feature itself; it stays in the
  remaining gnu Linux artifacts, in macOS/Windows artifacts, and in
  `cargo install` builds.
- Do **not** change the `subx-cli` library's build-system support for
  musl. `cargo build --target *-unknown-linux-musl` still works for
  end users who have a local musl-compatible ONNX Runtime; we are
  only dropping the *release-pipeline* obligation.
- Do **not** delete the `--musl` / `SUBX_LIBC` parsing code from
  `install.sh`. Keeping the parser (and routing it to a clean error)
  is more user-friendly than treating these inputs as unknown flags.
- Do **not** modify any spec other than `release-distribution`.
- Do **not** introduce any new external dependencies, new CI tooling,
  or new file formats.

## Decisions

### Decision 1: Modify `release-distribution`, do not create a new capability

**What:** Record the entire change as a delta against the existing
`release-distribution` spec, with `MODIFIED` and `REMOVED` requirement
blocks plus one `ADDED` block for the explicit musl-rejection
contract.

**Why:** musl support was never its own capability — it was an option
within `release-distribution`. A new capability would fragment the
release contract across two spec files for no benefit, and would
break the proposal's Capabilities → spec mapping convention.

**Alternatives considered:**
- *Create `linux-musl-removed` capability* — rejected; a "negation
  capability" has no positive surface area to specify.
- *Inline edits without a delta* — rejected; the openspec workflow
  requires deltas for spec evolution and rejects direct edits to
  `openspec/specs/`.

### Decision 2: Use `MODIFIED` for the artifact-matrix and asset-naming requirements, not `REMOVED`

**What:** The `Linux release artifact matrix`, `Asset naming
convention`, `Cross-compilation for ARM64 Linux`, `Smoke test of
Linux artifacts`, `Installer asset selection`, `Exact asset-name
matching in installer`, `archive-rar feature parity across Linux
artifacts`, `Release documentation`, and `Changelog entry for new
artifacts` requirements all stay; their **content** changes to drop
musl.

**Why:** The requirement headings still describe valid contracts —
we still publish a Linux artifact matrix, we still need an asset
naming convention, etc. Renaming or removing the headings would
trigger spurious churn in tooling that links to them. The skill's
guidance is unambiguous: "If adding new concerns without changing
existing behavior, use ADDED instead. Otherwise, use MODIFIED with
the entire requirement block."

**Alternatives considered:**
- *Mark whole requirements as `REMOVED` and `ADDED` replacements* —
  rejected; doubles the surface area, breaks history continuity, and
  obscures the fact that the contract structure is preserved.

### Decision 3: Add a new `Installer musl-input rejection` requirement

**What:** A new `### Requirement: Installer musl-input rejection`
block under `## ADDED Requirements`, owning four scenarios:
`SUBX_LIBC=musl exits with guidance`, `--musl flag exits with
guidance`, `auto-detected musl host exits with guidance`, and
`exit code is 2 (usage error)`.

**Why:** Without this requirement, the rejection contract is implicit
— a future contributor could "helpfully" make the installer fall
through to gnu, or 404 silently, without violating any spec text.
That would either silently produce wrong-libc binaries (on
glibc-musl ABI mismatch) or recreate the v1.7.1 user experience of
unhelpful 404s. The new requirement turns the v1.7.2 implementation
into a contract.

**Alternatives considered:**
- *Bury the rejection contract inside `Installer asset selection`*
  — rejected; that requirement is about *successful* selection,
  whereas rejection is a different code path with different
  diagnostics. Splitting them keeps each requirement focused on one
  testable behavior.
- *Make the rejection requirement just emit a warning and continue*
  — rejected; ABI-mismatched binaries crash at runtime in confusing
  ways. Refusing up front with `exit 2` is the only safe default.

### Decision 4: Keep the exact-name asset-matching requirement

**What:** The `Exact asset-name matching in installer` requirement
stays, with its musl-specific scenarios (`musl override does not
match gnu asset`, `gnu host does not match musl asset`, `aarch64 gnu
does not match aarch64 musl`) replaced by a single scenario asserting
exact-name matching for the gnu host.

**Why:** Defense in depth. Even with no musl assets in the release,
exact-name matching prevents future asset-name collisions (e.g., a
hypothetical `subx-linux-x86_64-static` would otherwise be a
substring match against `subx-linux-x86_64`). The contract is cheap
to keep and prevents an entire class of regression.

**Alternatives considered:**
- *Delete the requirement* — rejected; the substring-collision risk
  is real, the implementation already does exact matching, and the
  test harness already covers it.

### Decision 5: Update the `Release documentation` requirement to remove the musl-opt-in scenario, replace with a `cargo install` scenario

**What:** Drop `Scenario: README documents musl opt-in`. Add
`Scenario: README documents musl install via cargo install` so musl
users get pointed somewhere productive.

**Why:** Symmetry. The original spec required README to document how
to opt **into** musl artifacts; the new spec must require it to
document the supported alternative. Without this, a future README
edit could silently strip the `cargo install` note and re-strand
musl users.

**Alternatives considered:**
- *Drop the scenario without replacement* — rejected; would
  weaken the user-facing-doc contract.

### Decision 6: Add a `### Removed` companion to the `Changelog entry for new artifacts` requirement

**What:** Modify the `Changelog entry for new artifacts` requirement
so it covers BOTH `### Added` (when artifacts are first published)
AND `### Removed` (when artifacts are dropped), rather than only the
former.

**Why:** The original requirement was framed only for the v1.7.0
launch. Symmetry with `Keep a Changelog` conventions and with the
project's existing `bump-version` skill demands that artifact removal
also be discoverable from the changelog alone. The v1.7.2 changelog
already has the `### Removed` block — this requirement ratifies the
practice.

**Alternatives considered:**
- *Leave the requirement as `Added`-only* — rejected; the v1.7.2
  changelog would then technically be unconstrained for the musl
  removal, which contradicts the proposal's `### Removed` line.

## Risks / Trade-offs

- **[Risk]** Some downstream packagers may have a `Setup` doc or
  internal runbook that says "set `SUBX_LIBC=musl` for Alpine" — they
  will see exit code 2 the next time they pull the installer. →
  **Mitigation**: The new exit message names `cargo install subx-cli`
  explicitly. The CHANGELOG `### Removed` block (already in v1.7.2)
  documents the change. The README and `README.zh-TW.md` install
  tables and notes (already in v1.7.2) also document it.
- **[Risk]** A future `ort` upgrade might restore upstream musl
  binaries; we will need to revert this delta and reintroduce the
  matrix entries. → **Mitigation**: The proposal and design both
  document the reversibility path; the v1.7.0 commit
  (`6c9f81e34dab84ee7497af80fce0d5f78135ea22`) introducing musl
  remains in history as a recipe.
- **[Trade-off]** We are codifying "musl is unsupported via the
  installer" rather than "musl is unsupported full stop". Source-build
  via `cargo install subx-cli` remains *possible* on musl hosts, but
  only when the user has supplied a musl-compatible ONNX Runtime
  (e.g., from a distro package or a manual build) and pointed `ort`
  at it via `ORT_LIB_LOCATION`. Plain `cargo install subx-cli` will
  NOT work because `ort`'s default `download-binaries` feature hits
  the same upstream prebuilt-manifest gap. The README, the installer
  error message, and the spec text all reflect this honestly; the
  spec's documentation requirement explicitly forbids promising the
  unprepared `cargo install` path.
- **[Trade-off]** Keeping `--musl` and `SUBX_LIBC=musl` parsing in
  `install.sh` (rejected with a friendly error) is marginally more
  code than removing them outright. The friendliness wins. Test
  coverage in `scripts/test_install.sh` (cases `g.4` and `g.5` added
  in v1.7.2) makes the contract explicit.
- **[Risk]** The new `Installer musl-input rejection` requirement
  could accidentally reject *valid* future inputs if `SUBX_LIBC` is
  ever extended to a third value. → **Mitigation**: The requirement
  scopes itself to the literal value `musl`; other values stay on
  the existing `SUBX_LIBC=bogus` rejection path (`exit 2` with a
  different diagnostic).

## Migration Plan

1. **Already shipped in v1.7.2** (the implementation half):
   - Matrix entries removed from `Cross.toml` and `release.yml`.
   - `scripts/install.sh` rejects musl inputs with `exit 2` and
     `cargo install subx-cli` guidance; six new harness cases cover
     this.
   - README install tables and zh-TW counterpart updated; CHANGELOG
     entry under `[1.7.2]` describes both the fix and the removal.
2. **This change** (the spec half):
   - Apply the delta in `openspec/changes/drop-musl-support/specs/`
     to `openspec/specs/release-distribution/spec.md` via the
     archive workflow, after this proposal is approved.
3. **Rollback strategy** — full recipe, NOT a one-PR revert:
   - Restoring musl artifacts requires either (a) upstream `ort`
     publishing musl prebuilt tarballs (out of our control) or (b)
     source-building ONNX Runtime for musl as part of `Cross.toml`'s
     `pre-build` step (a substantial change in CI complexity and
     wall-clock).
   - Then reinstate the `[target.x86_64-unknown-linux-musl]` and
     `[target.aarch64-unknown-linux-musl]` blocks in `Cross.toml`,
     re-add the matrix entries in `.github/workflows/release.yml`,
     remove the `musl_unsupported_exit` calls from
     `scripts/install.sh`, restore the musl rows in both READMEs,
     update test coverage, add a `### Added` CHANGELOG entry, and
     reverse this OpenSpec change with an inverse delta.

## Open Questions

- None. The user has explicitly approved the
  `fix_aarch64_gnu_drop_musl` direction; v1.7.2 is shipped; the
  remaining work is the spec ratification covered here.
