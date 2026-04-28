## Why

The Linux musl release artifacts (`subx-linux-x86_64-musl` and
`subx-linux-aarch64-musl`) introduced in v1.7.0 cannot be produced with
the current toolchain. SubX-CLI's voice-activity-detection feature links
ONNX Runtime via `ort` 2.0.0-rc.10, whose binary distribution manifest
(`ort-sys`'s `dist.txt`) ships **no** `*-unknown-linux-musl` rows;
the upstream Pyke CDN simply does not publish musl builds of ONNX
Runtime. Source-building ONNX Runtime for musl inside the release
pipeline is out of scope (it requires a different toolchain, hours of
build time, and ongoing maintenance against upstream churn).

The v1.7.0 musl artifacts therefore never built successfully; the v1.7.1
release confirmed this by failing both musl matrix jobs at link time.
The v1.7.2 hot-fix already removed the musl matrix entries from
`Cross.toml` and `release.yml`, taught `scripts/install.sh` to reject
musl with a clear error, and dropped the README install rows — but
`openspec/specs/release-distribution/spec.md` still mandates musl
artifacts as part of the contract. This change formalizes the removal in
the spec layer so that the spec, the implementation, and the user-facing
documentation agree, and so that future contributors do not reintroduce
musl artifacts under the assumption that the spec still requires them.

## What Changes

- **BREAKING**: Remove the contractual requirement to publish
  `subx-linux-x86_64-musl` and `subx-linux-aarch64-musl` from the
  release artifact matrix. Linux release artifacts now consist of
  `subx-linux-x86_64` (gnu) and `subx-linux-aarch64` (gnu) only.
- **BREAKING**: Remove the `--musl` installer flag and the
  `SUBX_LIBC=musl` override as supported install modes. Both inputs
  remain *parsed* but are now contractually required to exit non-zero
  with actionable guidance pointing users at a source-build with a
  locally provisioned ONNX Runtime (typically by exporting
  `ORT_LIB_LOCATION` before `cargo install subx-cli`). Plain
  `cargo install subx-cli` is intentionally NOT promised to succeed
  on musl hosts because `ort`'s default `download-binaries` feature
  hits the same upstream gap that blocked the release pipeline.
- **BREAKING**: Remove the `archive-rar` feature-parity requirement
  for musl Linux artifacts (the parity contract is reduced to the two
  remaining gnu artifacts).
- Tighten the asset-naming convention: the optional `<libc>` suffix is
  removed from the supported grammar; Linux assets always omit the
  libc suffix. The exact-asset-name installer-matching requirement is
  preserved (defense in depth against future asset name collisions),
  but its musl-specific scenarios are deleted.
- Add a new requirement covering installer rejection of musl inputs so
  that the contract for `--musl` / `SUBX_LIBC=musl` / auto-detected
  musl hosts is explicit rather than implicit-by-omission.
- Update the README requirement: `README.md` and `README.zh-TW.md`
  MUST document that Linux musl users install via `cargo install
  subx-cli` rather than the script installer.
- Add a CHANGELOG `### Removed` requirement for the musl artifacts in
  the release that drops them, mirroring the existing `### Added`
  requirement that announced them.

## Capabilities

### New Capabilities

<!-- None: this change only modifies an existing capability. -->

### Modified Capabilities

- `release-distribution`: removes musl from the Linux artifact matrix,
  the asset naming grammar, the cross-compilation target list, the
  smoke-test target list, and the archive-rar feature-parity contract;
  adds an explicit installer-rejection contract for musl inputs;
  updates the README documentation requirement to point musl users at
  `cargo install`; tightens the changelog requirement to also cover
  the removal of artifacts.

## Impact

- **Specs**: `openspec/specs/release-distribution/spec.md` is the only
  modified spec. The change is recorded as a delta under
  `openspec/changes/drop-musl-support/specs/release-distribution/spec.md`.
- **Code (already shipped in v1.7.2, ratified by this change)**:
  - `Cross.toml` — musl `[target.*]` entries removed.
  - `.github/workflows/release.yml` — musl matrix entries removed; smoke
    test scope reduced to two Linux artifacts.
  - `scripts/install.sh` — `musl_unsupported_exit` helper introduced;
    `--musl`, `SUBX_LIBC=musl`, and auto-detected musl hosts all exit 2
    with `cargo install subx-cli` guidance.
  - `scripts/test_install.sh` — six new harness cases (`g.4`, `g.5`)
    cover the rejection contract.
  - `README.md`, `README.zh-TW.md` — install tables drop musl rows;
    musl opt-in section replaced with a `cargo install` note.
  - `CHANGELOG.md` — `[1.7.2]` `### Removed` entry documents the
    artifact removal and installer change.
- **Users on Alpine / Void musl / other musl distros**:
  `curl … | bash` no longer works; they MUST build from source with a
  locally provisioned, musl-compatible ONNX Runtime (typically by
  installing or building ONNX Runtime themselves and exporting
  `ORT_LIB_LOCATION` before `cargo install subx-cli`). The installer
  now tells them so. Note: bare `cargo install subx-cli` is NOT
  guaranteed to work because `ort`'s default `download-binaries`
  feature hits the same upstream prebuilt-manifest gap that blocked
  the release pipeline.
- **No source-code (`src/`) changes** are required by this change; the
  `subx-cli` crate itself remains musl-buildable when a musl-compatible
  ONNX Runtime is supplied via `ORT_LIB_LOCATION`.
- **Cargo features**: `archive-rar` is unaffected — it still ships in
  the two remaining Linux gnu artifacts, in the macOS and Windows
  artifacts, and in source-built binaries (musl users get it for free
  if their host has the `unrar` C toolchain installed).
- **Reversibility**: the removed `[target.*-unknown-linux-musl]`
  pre-build blocks and the `--features archive-rar` musl matrix entries
  remain in the v1.7.0/v1.7.1 git history, but restoring them is
  **not** a one-PR revert — it requires either upstream `ort` to
  publish musl prebuilts (out of our control) or a source-build of
  ONNX Runtime added to `Cross.toml`'s `pre-build`, plus reverting
  `install.sh`'s `musl_unsupported_exit` paths and reinstating the
  README rows. See `design.md`'s rollback section for the full recipe.
