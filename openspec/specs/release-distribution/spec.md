# release-distribution Specification

## Purpose

Defines how SubX-CLI release artifacts are produced, named, validated,
documented, and consumed by the `scripts/install.sh` installer. This
capability covers the GitHub Actions release workflow (target matrix,
cross-compilation, smoke tests, feature parity) as well as the
installer-side asset selection rules and user-facing documentation that
together guarantee users on every supported platform can reliably
download and run a compatible binary.

## Requirements

### Requirement: Linux release artifact matrix

The GitHub release workflow SHALL produce, for every tagged `v*` release,
binary artifacts for the following Linux target triples:

- `x86_64-unknown-linux-gnu` (mandatory)
- `aarch64-unknown-linux-gnu` (mandatory)
- `x86_64-unknown-linux-musl` (mandatory)
- `aarch64-unknown-linux-musl` (mandatory)

Existing macOS (`x86_64-apple-darwin`, `aarch64-apple-darwin`) and Windows
(`x86_64-pc-windows-msvc`) artifacts SHALL continue to be produced unchanged.

#### Scenario: tagged release publishes all required Linux artifacts

- **WHEN** a `v*` tag is pushed and the release workflow completes successfully
- **THEN** the GitHub Release attached to that tag contains assets named
  `subx-linux-x86_64`, `subx-linux-aarch64`, `subx-linux-x86_64-musl`, and
  `subx-linux-aarch64-musl`

#### Scenario: existing platform artifacts are preserved

- **WHEN** a `v*` tag is pushed and the release workflow completes successfully
- **THEN** the GitHub Release also contains `subx-macos-x86_64`,
  `subx-macos-aarch64`, and `subx-windows-x86_64.exe`, and the existing
  asset names, target triples, executable names, and feature set are
  preserved relative to releases produced before this change

### Requirement: Asset naming convention

Release asset names SHALL follow the pattern `subx-<platform>-<arch>[-<libc>][.<ext>]` where:

- `<platform>` is one of `linux`, `macos`, `windows`.
- `<arch>` is one of `x86_64`, `aarch64`.
- `<libc>` is omitted for the platform default (gnu on Linux) and set to
  `musl` for musl-linked Linux builds.
- `<ext>` is `.exe` for Windows and absent for Linux/macOS.

#### Scenario: gnu Linux asset omits libc suffix

- **WHEN** a Linux gnu artifact is published
- **THEN** its asset name is `subx-linux-<arch>` with no libc suffix

#### Scenario: musl Linux asset includes libc suffix

- **WHEN** a Linux musl artifact is published
- **THEN** its asset name ends with `-musl` (e.g., `subx-linux-aarch64-musl`)

#### Scenario: Windows asset uses .exe extension

- **WHEN** a Windows artifact is published
- **THEN** its asset name ends with `.exe`

### Requirement: Cross-compilation for ARM64 Linux

The release workflow SHALL build `aarch64-unknown-linux-gnu` and
`aarch64-unknown-linux-musl` artifacts via cross-compilation on
`ubuntu-latest` runners (using `cross`, or a native cross toolchain such as
`gcc-aarch64-linux-gnu` when `cross` is unavailable). It SHALL NOT depend on
GitHub-hosted ARM Linux runners.

#### Scenario: aarch64 build runs on x86_64 runner

- **WHEN** the workflow's `aarch64-unknown-linux-gnu` matrix job runs
- **THEN** it executes on `ubuntu-latest` (x86_64) using a cross-compilation
  toolchain

#### Scenario: cross toolchain failure aborts release

- **WHEN** the cross toolchain step fails to build a required Linux artifact
- **THEN** the release workflow fails and the corresponding asset is not
  uploaded

### Requirement: Smoke test of Linux artifacts

Every Linux release artifact SHALL be executed with `--version` (or
equivalent no-op invocation) before being uploaded. Cross-compiled aarch64
artifacts SHALL be executed under QEMU user-mode emulation
(`qemu-user-static` or an equivalent action). A non-zero exit status,
missing-output condition, or timeout SHALL fail the workflow.

#### Scenario: x86_64 artifact runs successfully

- **WHEN** the x86_64 Linux artifact is built
- **THEN** the workflow runs `./<asset> --version` natively and the command
  exits 0 with version output

#### Scenario: aarch64 artifact runs under QEMU

- **WHEN** the aarch64 Linux artifact is built
- **THEN** the workflow runs `./<asset> --version` under QEMU user-mode
  emulation and the command exits 0 with version output

#### Scenario: broken artifact blocks release

- **WHEN** the smoke test for any Linux artifact fails (non-zero exit,
  segfault, or timeout)
- **THEN** the workflow fails and that artifact is not attached to the
  GitHub Release

### Requirement: Installer asset selection

The `scripts/install.sh` installer SHALL map the detected host to a release
asset URL using the following rules:

- Operating system: Linux → `linux`, macOS (`darwin`) → `macos`. Other
  systems SHALL exit with a non-zero status and a clear error.
- Architecture: `x86_64` → `x86_64`, `aarch64`/`arm64` → `aarch64`. Other
  architectures SHALL exit with a non-zero status and a clear error.
- libc selection on Linux: default to gnu (no libc suffix). The user MAY
  override selection via:
  - `SUBX_LIBC=musl` environment variable, or
  - a `--musl` command-line flag, or
  - automatic detection: when `ldd --version` output indicates musl, the
    installer MAY prefer the musl asset.
- The constructed asset name SHALL conform to the asset naming convention
  requirement.

#### Scenario: aarch64 Linux host installs ARM64 gnu binary

- **WHEN** the installer runs on a Linux host where `uname -m` returns
  `aarch64` and `SUBX_LIBC` is not set
- **THEN** it downloads the `subx-linux-aarch64` asset

#### Scenario: arm64 macOS host installs ARM64 macOS binary

- **WHEN** the installer runs on a macOS host where `uname -m` returns
  `arm64`
- **THEN** it downloads the `subx-macos-aarch64` asset

#### Scenario: SUBX_LIBC=musl forces musl asset

- **WHEN** the installer runs on a Linux host with `SUBX_LIBC=musl` exported
- **THEN** it downloads `subx-linux-<arch>-musl`

#### Scenario: --musl flag forces musl asset

- **WHEN** the installer is invoked with `--musl`
- **THEN** it downloads `subx-linux-<arch>-musl` regardless of auto-detection

#### Scenario: explicit override beats auto-detection

- **WHEN** auto-detection would pick gnu but `SUBX_LIBC=musl` is set
- **THEN** the installer downloads the musl asset

#### Scenario: unsupported OS exits with error

- **WHEN** the installer runs on an OS other than Linux or macOS
- **THEN** it prints an error identifying the unsupported OS and exits non-zero

#### Scenario: unsupported architecture exits with error

- **WHEN** the installer runs on an architecture other than x86_64 or aarch64
- **THEN** it prints an error identifying the unsupported architecture and
  exits non-zero

### Requirement: Exact asset-name matching in installer

The `scripts/install.sh` installer SHALL select the download URL using
exact asset-name matching against the GitHub Releases JSON response.
The installer MUST NOT use substring or prefix matching, because
`subx-linux-x86_64` is a substring of `subx-linux-x86_64-musl` and a
loose match would silently install the wrong libc variant.

Implementations SHALL satisfy this requirement by either:

- Using `jq` with an exact-string filter such as
  `.assets[] | select(.name == "<expected>") | .browser_download_url`,
  or
- Comparing the basename of each `browser_download_url` to the expected
  asset name with a string-equality test (no globbing, no `grep`
  substring).

The expected asset name SHALL be the value produced by the Asset
naming convention requirement for the current host and libc selection.

#### Scenario: gnu host does not match musl asset

- **WHEN** the installer runs on x86_64 Linux with no libc override and
  the latest release contains both `subx-linux-x86_64` and
  `subx-linux-x86_64-musl`
- **THEN** it selects the URL whose asset name equals
  `subx-linux-x86_64` exactly, and never downloads
  `subx-linux-x86_64-musl`

#### Scenario: musl override does not match gnu asset

- **WHEN** the installer runs with `SUBX_LIBC=musl` (or `--musl`) on
  x86_64 Linux and the latest release contains both
  `subx-linux-x86_64` and `subx-linux-x86_64-musl`
- **THEN** it selects the URL whose asset name equals
  `subx-linux-x86_64-musl` exactly, and never falls back to
  `subx-linux-x86_64`

#### Scenario: aarch64 gnu does not match aarch64 musl

- **WHEN** the installer runs on aarch64 Linux with no libc override and
  both `subx-linux-aarch64` and `subx-linux-aarch64-musl` are published
- **THEN** it selects the URL whose asset name equals
  `subx-linux-aarch64` exactly

### Requirement: archive-rar feature parity across Linux artifacts

The release workflow SHALL build every published Linux artifact with the `archive-rar` Cargo feature enabled. All four targets (`subx-linux-x86_64`, `subx-linux-aarch64`, `subx-linux-x86_64-musl`, and `subx-linux-aarch64-musl`) MUST ship `.rar` extraction support, and the workflow SHALL configure the toolchain (gnu or musl, native or cross) so the optional `unrar` C dependency compiles for each target.

If a future build environment makes `archive-rar` infeasible for a
specific Linux target, the change that drops the feature SHALL update
this requirement (and the changelog) to document the divergence
explicitly; until then, parity across all four Linux artifacts is the
contract.

#### Scenario: every Linux artifact is built with archive-rar enabled

- **WHEN** the release workflow builds any of the four Linux artifacts
  (`subx-linux-x86_64`, `subx-linux-aarch64`, `subx-linux-x86_64-musl`,
  `subx-linux-aarch64-musl`)
- **THEN** the `cargo build` invocation for that target includes
  `--features archive-rar`, and the build step exits successfully

#### Scenario: musl build uses a toolchain that compiles unrar

- **WHEN** the workflow builds `x86_64-unknown-linux-musl` or
  `aarch64-unknown-linux-musl`
- **THEN** the build step uses a toolchain (e.g., `cross` image with
  the appropriate musl C toolchain) that successfully compiles the
  `unrar` C dependency with `--features archive-rar`

### Requirement: Installer fallback diagnostics

The `scripts/install.sh` installer SHALL emit actionable diagnostics and
SHALL exit with a non-zero status when it cannot locate the expected
asset URL in the GitHub Releases JSON response. The diagnostic output
MUST include all of the following:

- The detected platform and architecture.
- The asset name it searched for.
- The list of asset names that ARE available in the latest release.
- A link to `https://github.com/jim60105/subx-cli/releases` for manual
  download.

The installer SHALL also exit with a non-zero status when the GitHub API
request fails or returns an empty response, and SHALL print a clear
network/availability error in that case.

#### Scenario: missing asset prints actionable diagnostics

- **WHEN** the requested asset is not present in the latest release
- **THEN** the installer prints the detected platform, the searched asset
  name, the list of available assets, and a link to the releases page, and
  exits non-zero

#### Scenario: network failure exits with error

- **WHEN** the GitHub API request fails or returns an empty response
- **THEN** the installer prints a network/availability error and exits non-zero

### Requirement: Backward-compatible installer behavior

The `scripts/install.sh` installer SHALL preserve existing behavior on
hosts that were already supported before this change (x86_64 Linux,
x86_64 macOS, aarch64 macOS). Specifically, the installer MUST use the
same asset names, MUST install to the same path
(`/usr/local/bin/subx-cli`), and MUST NOT introduce any new mandatory
flags or environment variables for those hosts.

#### Scenario: x86_64 Linux install is unchanged

- **WHEN** the installer runs on x86_64 Linux with no environment overrides
  and no flags
- **THEN** it downloads `subx-linux-x86_64` and installs it to
  `/usr/local/bin/subx-cli`, matching pre-change behavior

#### Scenario: macOS install is unchanged

- **WHEN** the installer runs on macOS (x86_64 or aarch64) with no
  environment overrides and no flags
- **THEN** it downloads the corresponding `subx-macos-<arch>` asset and
  installs it to `/usr/local/bin/subx-cli`, matching pre-change behavior

### Requirement: Release documentation

The repository SHALL document the supported release targets, asset naming
convention, and installer libc selection mechanism in user-facing
documentation. At minimum, `README.md` and `README.zh-TW.md` SHALL list
the available installer-supported platforms and explain how to opt into
musl artifacts.

#### Scenario: README lists supported platforms

- **WHEN** a user reads the installation section of `README.md` or
  `README.zh-TW.md`
- **THEN** the section names every supported `(platform, arch, libc)`
  combination available via the installer

#### Scenario: README documents musl opt-in

- **WHEN** a user reads the installation section of `README.md` or
  `README.zh-TW.md`
- **THEN** the section explains how to opt into the musl artifact (for
  example via `SUBX_LIBC=musl` or `--musl`)

### Requirement: Changelog entry for new artifacts

The project's `CHANGELOG.md` SHALL contain an `### Added` entry under
the version header of the release that first publishes the new
artifacts, and that entry MUST describe the new aarch64 Linux and musl
artifacts so users can discover them from the changelog alone.

#### Scenario: changelog announces ARM64 Linux artifact

- **WHEN** the release that introduces ARM64 Linux artifacts is cut
- **THEN** `CHANGELOG.md` contains an `### Added` line referencing the new
  `subx-linux-aarch64` (and musl) assets under that release's version
  header
