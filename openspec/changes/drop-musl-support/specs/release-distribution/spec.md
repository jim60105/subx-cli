## ADDED Requirements

### Requirement: Installer musl-input rejection

The `scripts/install.sh` installer SHALL refuse to download any
release artifact when the user has requested a musl libc, regardless
of whether the request was explicit (`SUBX_LIBC=musl` environment
variable, `--musl` command-line flag) or implicit (auto-detection
of a musl host via `ldd --version`). On rejection, the installer
SHALL exit with status code `2` (usage / configuration error), MUST
NOT issue any HTTP request to GitHub Releases, and MUST print
diagnostic guidance that:

- States that musl artifacts are no longer published.
- Recommends `cargo install subx-cli` as the supported install path
  for musl-based distributions.

The installer SHALL continue to accept and parse `--musl` and
`SUBX_LIBC=musl` as well-formed inputs (they MUST NOT be reported as
"unknown flag" / "invalid value" errors); the rejection path is the
*only* legal handling of these values.

#### Scenario: SUBX_LIBC=musl exits with guidance

- **WHEN** the installer is invoked with `SUBX_LIBC=musl` exported
- **THEN** it exits with status `2`, prints a message stating that
  musl artifacts are not published, and recommends
  `cargo install subx-cli`

#### Scenario: --musl flag exits with guidance

- **WHEN** the installer is invoked with the `--musl` command-line flag
- **THEN** it exits with status `2`, prints a message stating that
  musl artifacts are not published, and recommends
  `cargo install subx-cli`

#### Scenario: auto-detected musl host exits with guidance

- **WHEN** the installer runs on a Linux host where `ldd --version`
  reports musl libc (e.g., Alpine, Void musl) and neither
  `SUBX_LIBC` nor `--musl` is set
- **THEN** it exits with status `2`, prints a message stating that
  musl artifacts are not published, and recommends
  `cargo install subx-cli`

#### Scenario: rejection happens before any network call

- **WHEN** any of the three musl input paths above is taken
- **THEN** the installer terminates without issuing an HTTP request
  to `api.github.com` or to the GitHub Releases CDN

## MODIFIED Requirements

### Requirement: Linux release artifact matrix

The GitHub release workflow SHALL produce, for every tagged `v*`
release, binary artifacts for the following Linux target triples:

- `x86_64-unknown-linux-gnu` (mandatory)
- `aarch64-unknown-linux-gnu` (mandatory)

Existing macOS (`x86_64-apple-darwin`, `aarch64-apple-darwin`) and
Windows (`x86_64-pc-windows-msvc`) artifacts SHALL continue to be
produced unchanged.

Linux musl artifacts (`*-unknown-linux-musl`) SHALL NOT be produced
by the release workflow. The upstream ONNX Runtime distribution
consumed by SubX-CLI's voice-activity-detection feature does not
publish musl-targeted binaries, and source-building it for musl is
out of scope for the release pipeline. Users on musl-based Linux
distributions are expected to build SubX-CLI from source against a
locally provisioned ONNX Runtime — typically by installing or
building an ONNX Runtime that targets musl, exporting
`ORT_LIB_LOCATION` (or equivalent `ort` build-time configuration) to
point at it, and then running `cargo install subx-cli`. The release
workflow itself MUST NOT promise that an unprepared
`cargo install subx-cli` will succeed on musl hosts; it will not,
because `ort`'s default `download-binaries` feature also depends on
the same upstream prebuilt manifest.

#### Scenario: tagged release publishes all required Linux artifacts

- **WHEN** a `v*` tag is pushed and the release workflow completes
  successfully
- **THEN** the GitHub Release attached to that tag contains assets
  named `subx-linux-x86_64` and `subx-linux-aarch64`

#### Scenario: existing platform artifacts are preserved

- **WHEN** a `v*` tag is pushed and the release workflow completes
  successfully
- **THEN** the GitHub Release also contains `subx-macos-x86_64`,
  `subx-macos-aarch64`, and `subx-windows-x86_64.exe`, and the
  existing asset names, target triples, executable names, and
  feature set for those four artifacts plus the two Linux gnu
  artifacts are preserved relative to releases produced before this
  change

#### Scenario: musl artifacts are not produced

- **WHEN** a `v*` tag is pushed and the release workflow completes
  successfully
- **THEN** the GitHub Release does NOT contain any asset whose name
  ends in `-musl`

### Requirement: Asset naming convention

Release asset names SHALL follow the pattern
`subx-<platform>-<arch>[.<ext>]` where:

- `<platform>` is one of `linux`, `macos`, `windows`.
- `<arch>` is one of `x86_64`, `aarch64`.
- `<ext>` is `.exe` for Windows and absent for Linux/macOS.

Linux assets SHALL NOT carry a libc suffix; the gnu libc is
implicit for every published Linux asset. Names ending in `-musl`,
`-static`, or any other libc/abi suffix SHALL NOT be produced.

#### Scenario: Linux asset omits libc suffix

- **WHEN** a Linux artifact is published
- **THEN** its asset name is `subx-linux-<arch>` with no libc suffix

#### Scenario: Windows asset uses .exe extension

- **WHEN** a Windows artifact is published
- **THEN** its asset name ends with `.exe`

#### Scenario: macOS asset omits any suffix

- **WHEN** a macOS artifact is published
- **THEN** its asset name is `subx-macos-<arch>` with no extension
  and no libc suffix

### Requirement: Cross-compilation for ARM64 Linux

The release workflow SHALL build the `aarch64-unknown-linux-gnu`
artifact via cross-compilation on `ubuntu-latest` runners (using
`cross`, or a native cross toolchain such as `gcc-aarch64-linux-gnu`
when `cross` is unavailable). It SHALL NOT depend on
GitHub-hosted ARM Linux runners.

#### Scenario: aarch64 build runs on x86_64 runner

- **WHEN** the workflow's `aarch64-unknown-linux-gnu` matrix job runs
- **THEN** it executes on `ubuntu-latest` (x86_64) using a
  cross-compilation toolchain

#### Scenario: cross toolchain failure aborts release

- **WHEN** the cross toolchain step fails to build the
  `aarch64-unknown-linux-gnu` artifact
- **THEN** the release workflow fails and the corresponding asset
  is not uploaded

### Requirement: Smoke test of Linux artifacts

Every Linux release artifact (the two gnu artifacts) SHALL be
executed with `--version` (or equivalent no-op invocation) before
being uploaded. Cross-compiled aarch64 artifacts SHALL be executed
under QEMU user-mode emulation (`qemu-user-static` or an equivalent
action), with a sysroot available to the emulator (e.g., the
`gcc-aarch64-linux-gnu` package's `/usr/aarch64-linux-gnu` tree
exported via `QEMU_LD_PREFIX`) so that the dynamic loader and
runtime libraries (`ld-linux-aarch64.so.1`, `libstdc++.so.6`,
`libgcc_s.so.1`) resolve correctly. A non-zero exit status,
missing-output condition, or timeout SHALL fail the workflow.

#### Scenario: x86_64 artifact runs successfully

- **WHEN** the x86_64 Linux artifact is built
- **THEN** the workflow runs `./<asset> --version` natively and the
  command exits 0 with version output

#### Scenario: aarch64 artifact runs under QEMU with a sysroot

- **WHEN** the aarch64 Linux artifact is built
- **THEN** the workflow runs `./<asset> --version` under QEMU
  user-mode emulation with `QEMU_LD_PREFIX` pointing at an aarch64
  sysroot, and the command exits 0 with version output

#### Scenario: broken artifact blocks release

- **WHEN** the smoke test for any Linux artifact fails (non-zero
  exit, segfault, or timeout)
- **THEN** the workflow fails and that artifact is not attached to
  the GitHub Release

### Requirement: Installer asset selection

The `scripts/install.sh` installer SHALL map the detected host to a
release asset URL using the following rules:

- Operating system: Linux → `linux`, macOS (`darwin`) → `macos`.
  Other systems SHALL exit with a non-zero status and a clear error.
- Architecture: `x86_64` → `x86_64`, `aarch64`/`arm64` → `aarch64`.
  Other architectures SHALL exit with a non-zero status and a clear
  error.
- libc selection on Linux: gnu is the only supported libc.
  `SUBX_LIBC` MAY be set explicitly to `gnu` (no-op); any musl
  request (whether via `SUBX_LIBC=musl`, `--musl`, or auto-detection
  on a musl host) SHALL be handled by the *Installer musl-input
  rejection* requirement and SHALL NOT result in any download
  attempt.
- The constructed asset name SHALL conform to the asset-naming
  convention requirement.

#### Scenario: aarch64 Linux host installs ARM64 gnu binary

- **WHEN** the installer runs on a Linux host where `uname -m`
  returns `aarch64` and `SUBX_LIBC` is not set
- **THEN** it downloads the `subx-linux-aarch64` asset

#### Scenario: arm64 macOS host installs ARM64 macOS binary

- **WHEN** the installer runs on a macOS host where `uname -m`
  returns `arm64`
- **THEN** it downloads the `subx-macos-aarch64` asset

#### Scenario: x86_64 Linux host installs x86_64 gnu binary

- **WHEN** the installer runs on a Linux host where `uname -m`
  returns `x86_64` and `SUBX_LIBC` is not set
- **THEN** it downloads the `subx-linux-x86_64` asset

#### Scenario: unsupported OS exits with error

- **WHEN** the installer runs on an OS other than Linux or macOS
- **THEN** it prints an error identifying the unsupported OS and
  exits non-zero

#### Scenario: unsupported architecture exits with error

- **WHEN** the installer runs on an architecture other than x86_64
  or aarch64
- **THEN** it prints an error identifying the unsupported
  architecture and exits non-zero

### Requirement: Exact asset-name matching in installer

The `scripts/install.sh` installer SHALL select the download URL
using exact asset-name matching against the GitHub Releases JSON
response. The installer MUST NOT use substring or prefix matching,
as a precaution against future asset-name collisions (for example,
a hypothetical `subx-linux-x86_64-static` asset would be a substring
of `subx-linux-x86_64` under loose matching).

Implementations SHALL satisfy this requirement by either:

- Using `jq` with an exact-string filter such as
  `.assets[] | select(.name == "<expected>") | .browser_download_url`,
  or
- Comparing the basename of each `browser_download_url` to the
  expected asset name with a string-equality test (no globbing, no
  `grep` substring).

The expected asset name SHALL be the value produced by the asset
naming convention requirement for the current host.

#### Scenario: x86_64 Linux selects exact gnu asset

- **WHEN** the installer runs on x86_64 Linux and the latest
  release contains `subx-linux-x86_64`
- **THEN** it selects the URL whose asset name equals
  `subx-linux-x86_64` exactly

#### Scenario: aarch64 Linux selects exact gnu asset

- **WHEN** the installer runs on aarch64 Linux
- **THEN** it selects the URL whose asset name equals
  `subx-linux-aarch64` exactly

### Requirement: archive-rar feature parity across Linux artifacts

The release workflow SHALL build every published Linux artifact
with the `archive-rar` Cargo feature enabled. Both targets
(`subx-linux-x86_64`, `subx-linux-aarch64`) MUST ship `.rar`
extraction support, and the workflow SHALL configure the toolchain
so the optional `unrar` C dependency compiles for each target.

If a future build environment makes `archive-rar` infeasible for a
specific Linux target, the change that drops the feature SHALL
update this requirement (and the changelog) to document the
divergence explicitly; until then, parity across both Linux
artifacts is the contract.

#### Scenario: every Linux artifact is built with archive-rar enabled

- **WHEN** the release workflow builds either of the two Linux
  artifacts (`subx-linux-x86_64`, `subx-linux-aarch64`)
- **THEN** the `cargo build` invocation for that target includes
  `--features archive-rar`, and the build step exits successfully

### Requirement: Release documentation

The repository SHALL document the supported release targets, the
asset naming convention, and the install path for musl-based
distributions in user-facing documentation. At minimum, `README.md`
and `README.zh-TW.md` SHALL list the available installer-supported
platforms and SHALL explain that musl users build from source with
a locally provisioned ONNX Runtime (and SHOULD reference
`ORT_LIB_LOCATION` or the equivalent `ort` configuration) — they
SHALL NOT promise that an unprepared `cargo install subx-cli`
succeeds on musl hosts.

#### Scenario: README lists supported platforms

- **WHEN** a user reads the installation section of `README.md` or
  `README.zh-TW.md`
- **THEN** the section names every supported `(platform, arch)`
  combination available via the installer (`linux x86_64`,
  `linux aarch64`, `macos x86_64`, `macos aarch64`,
  `windows x86_64`)

#### Scenario: README documents the musl source-build path

- **WHEN** a user reads the installation section of `README.md` or
  `README.zh-TW.md`
- **THEN** the section explains that musl-based Linux distributions
  (e.g., Alpine, Void musl) are not served by the script installer
  and that users on those distributions need to build from source
  with a locally provisioned ONNX Runtime (with a reference to
  `ORT_LIB_LOCATION` or the equivalent `ort` build-time
  configuration)

### Requirement: Backward-compatible installer behavior

The `scripts/install.sh` installer SHALL preserve existing behavior
on hosts that were already supported before this change (x86_64
Linux gnu, aarch64 Linux gnu, x86_64 macOS, aarch64 macOS).
Specifically, the installer MUST use the same asset names for those
hosts, MUST install to the same path (`/usr/local/bin/subx-cli`),
and MUST NOT introduce any new mandatory flags or environment
variables for those hosts.

The installer MAY introduce new exit-code behavior for previously
musl-detected or musl-requested hosts (covered by the *Installer
musl-input rejection* requirement); such hosts were never
backward-compatibility-protected, since musl artifacts only ever
existed for one release (v1.7.0) and the v1.7.1 release of those
artifacts failed at link time.

#### Scenario: x86_64 Linux gnu install is unchanged

- **WHEN** the installer runs on x86_64 Linux with no environment
  overrides and no flags
- **THEN** it downloads `subx-linux-x86_64` and installs it to
  `/usr/local/bin/subx-cli`, matching pre-change behavior

#### Scenario: aarch64 Linux gnu install is unchanged

- **WHEN** the installer runs on aarch64 Linux with no environment
  overrides and no flags
- **THEN** it downloads `subx-linux-aarch64` and installs it to
  `/usr/local/bin/subx-cli`, matching pre-change behavior

#### Scenario: macOS install is unchanged

- **WHEN** the installer runs on macOS (x86_64 or aarch64) with no
  environment overrides and no flags
- **THEN** it downloads the corresponding `subx-macos-<arch>`
  asset and installs it to `/usr/local/bin/subx-cli`, matching
  pre-change behavior

### Requirement: Changelog entry for new artifacts

The project's `CHANGELOG.md` SHALL contain entries describing
material changes to the release artifact set:

- An `### Added` entry under the version that first publishes a new
  artifact, describing the new asset(s) so users can discover them
  from the changelog alone.
- A `### Removed` entry under the version that drops a previously
  published artifact, describing which asset(s) were removed and
  the supported migration path for affected users.

#### Scenario: changelog announces ARM64 Linux artifact

- **WHEN** the release that introduces ARM64 Linux artifacts is cut
- **THEN** `CHANGELOG.md` contains an `### Added` line referencing the
  new `subx-linux-aarch64` asset under that release's version header

#### Scenario: changelog announces a removed artifact

- **WHEN** a release drops a previously published platform/arch
  combination
- **THEN** `CHANGELOG.md` contains a `### Removed` line referencing
  the dropped asset(s) and naming the supported migration path
  under that release's version header
