## Why

The `scripts/install.sh` installer already maps `aarch64`/`arm64` Linux hosts to a
`subx-linux-aarch64` asset, but the GitHub release workflow
(`.github/workflows/release.yml`) only builds and uploads `x86_64-unknown-linux-gnu`.
ARM64 Linux users — including Raspberry Pi 4/5, AWS Graviton, Oracle Ampere, and
many container hosts — therefore see the installer fail with
`Could not find download file for linux-aarch64`. The release matrix needs to
match the installer's contract, and we want to take this opportunity to make Linux
artifacts more portable (musl static builds) and to formalize installer/asset
mapping so it cannot drift again.

## What Changes

- Extend the release build matrix in `.github/workflows/release.yml` to produce
  `aarch64-unknown-linux-gnu` artifacts using cross-compilation
  (`cross` or `cargo` with the `aarch64-linux-gnu` GCC toolchain) on
  `ubuntu-latest`.
- Add optional musl static artifacts for both architectures
  (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`) so the installer can
  serve a self-contained binary on glibc-incompatible distros (Alpine, minimal
  containers).
- Standardize the asset naming contract documented for the installer:
  - `subx-linux-x86_64` → `x86_64-unknown-linux-gnu` (default Linux x86_64)
  - `subx-linux-aarch64` → `aarch64-unknown-linux-gnu` (default Linux ARM64)
  - `subx-linux-x86_64-musl` → `x86_64-unknown-linux-musl` (optional)
  - `subx-linux-aarch64-musl` → `aarch64-unknown-linux-musl` (optional)
  - macOS (`subx-macos-x86_64`, `subx-macos-aarch64`) and Windows
    (`subx-windows-x86_64.exe`) names remain unchanged.
- Update `scripts/install.sh` to:
  - Retain its current platform/architecture detection logic (no breaking
    behavior for x86_64 Linux/macOS users).
  - Add an opt-in `SUBX_LIBC=musl` (or `--musl` flag) selection so users on
    Alpine/musl distros can request the musl asset; otherwise the gnu asset is
    used.
  - Produce a clearer error message that lists available assets when the
    expected asset is missing, instead of failing silently with a single line.
- Run a post-build smoke test on each Linux artifact in the release workflow:
  the matrix job for native targets executes `./<asset> --version`; the
  cross-compiled aarch64 artifacts are exercised via QEMU user-mode emulation
  (`qemu-user-static`) or the `run-on-arch-action` to confirm the binary
  actually runs.
- Update README installation sections (`README.md`, `README.zh-TW.md`) and
  `docs/command-reference.md` to document supported release targets and the
  optional musl/aarch64 install paths.
- Add changelog entry under `## [Unreleased]` describing the new release
  artifacts.

This change is **not** a breaking change for end users: existing asset names
(`subx-linux-x86_64`, `subx-macos-*`, `subx-windows-*`) are preserved. It is a
backward-compatible expansion of the release artifact set plus a documented
installer contract.

## Capabilities

### New Capabilities

- `release-distribution`: Defines the official release artifact matrix
  (target triples, asset names, packaging rules), the contract between the
  GitHub release workflow and `scripts/install.sh`, and the validation
  (smoke testing, fallback messaging) that must hold for every tagged
  release.

### Modified Capabilities

<!-- None: supply-chain-hardening covers dependency advisories and is unrelated
     to release artifact production. No existing spec governs release/install
     artifact mapping today. -->

## Impact

- **Affected files**:
  - `.github/workflows/release.yml` (build matrix, cross toolchain, smoke
    tests).
  - `scripts/install.sh` (libc selection, error messaging).
  - `README.md`, `README.zh-TW.md`, `docs/command-reference.md` (install docs).
  - `CHANGELOG.md` (Unreleased entry).
- **New CI dependencies**: `cross` (or system `gcc-aarch64-linux-gnu` +
  `qemu-user-static`) and optionally `uraimo/run-on-arch-action` for emulation.
  No new runtime dependencies are introduced for users.
- **Release time**: the release workflow gains 2–4 additional matrix jobs;
  with parallel matrix execution the wall-clock release time should rise by
  roughly the duration of one cross-compile (~5–10 minutes), not by N×.
- **Backward compatibility**: existing asset URLs are preserved; users on
  x86_64 Linux/macOS/Windows experience no change. The installer continues to
  default to glibc Linux assets.
- **Risk surface**: cross-compilation reproducibility, QEMU smoke-test
  flakiness, and potential dynamic-link issues on older glibc versions —
  mitigated by the optional musl artifacts and the documented fallback message.
