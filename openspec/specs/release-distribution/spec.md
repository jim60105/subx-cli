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

Once the library is a separate crate, `archive-rar` is declared in **two** manifests and only one of them owns the real gate:

- `subx-core` SHALL own the effective gate, activating the optional `unrar` dependency.
- `subx-cli` SHALL declare `archive-rar` as a pass-through that enables `subx-core`'s feature and nothing else.
- A release build SHALL therefore continue to pass a single bare `--features archive-rar`, which applies to every selected package that declares the feature. Cargo rejects a bare feature name only when **no** selected package declares it, so the pass-through arrangement is what keeps the existing release command correct.
- Because the feature that actually enables `.rar` support lives in the other repository, a `subx-core` change that alters or removes that gate SHALL be treated as a change to this requirement. The failure mode is silent: the release command still succeeds and the published Linux binaries simply lack `.rar` support.
- The release workflow SHALL therefore verify feature parity by an observable property of the built artifact rather than by the presence of the flag alone.

#### Scenario: every Linux artifact is built with archive-rar enabled

- **WHEN** the release workflow builds either of the two Linux
  artifacts (`subx-linux-x86_64`, `subx-linux-aarch64`)
- **THEN** the `cargo build` invocation for that target includes
  `--features archive-rar`, and the build step exits successfully

#### Scenario: The pass-through reaches the crate that owns the gate

- **GIVEN** `subx-cli` declaring `archive-rar` as a pass-through to `subx-core`'s feature
- **WHEN** a release build passes a bare `--features archive-rar`
- **THEN** `subx-core` SHALL be compiled with its `archive-rar` feature active and the optional `unrar` dependency present

#### Scenario: Removing the gate in the library repository is a spec change

- **GIVEN** a change in `subx-core` that removes or renames the feature the pass-through targets
- **WHEN** that change is proposed
- **THEN** it SHALL be treated as amending this requirement, because the release command would continue to succeed while producing Linux binaries without `.rar` support

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

### Requirement: Two-Crate Publication Order and Preconditions

The project publishes two coupled crates to crates.io from one repository, and `subx-cli` cannot be published before `subx-core` exists on the registry at a version satisfying its declared requirement. The release workflow's publish job SHALL therefore treat publication as an ordered operation with checked preconditions rather than as a single command.

**Ordering and mechanism**

- Publication SHALL be performed with `cargo publish --workspace`, which packages every selected member, orders them topologically, verifies each against the packaged artifacts of its already-selected dependencies rather than against the registry, and uploads in that order.
- `cargo publish --workspace` requires Cargo 1.90 or later. The publish job SHALL assert the available Cargo version against that floor as its first step, and SHALL fail with a message naming the floor when it is not met. The assertion SHALL NOT be replaced by pinning the toolchain to the floor, and the floor SHALL NOT be expressed as the package's `rust-version`: `rust-version` is a promise to consumers about compiling the crate, whereas this floor is a property of the publishing tool.
- Where `cargo publish --workspace` is unavailable, the fallback SHALL be a two-stage publish — `subx-core` first, then `subx-cli` — relying on Cargo's index-propagation wait. It SHALL be documented as a fallback and SHALL NOT be adopted as the default flow, because its propagation window is a failure mode the workspace form does not have.

**Preconditions, all checked before the first upload**

- The submodule pointer SHALL be committed and the submodule working tree SHALL be clean. The job SHALL verify this explicitly — no `-`, `+` or `U` prefix from `git submodule status --recursive`, and no output from a porcelain status inside the submodule — so that the failure names the submodule rather than naming a dirty working directory.
- `--allow-dirty` SHALL NOT be used, under any circumstance, to satisfy the preceding bullet. It records `"dirty": true` in the published archive's `.cargo_vcs_info.json`, permanently, asserting that the artifact corresponds to no commit. Its absence from the workflow files SHALL be enforced mechanically, not by review.
- A full `cargo publish --workspace --dry-run` SHALL succeed before any upload is attempted, and SHALL additionally have been run locally before the release tag is pushed.
- The job SHALL determine which members actually require publication by querying the registry index for each member's `name@version`, and SHALL exclude members already present. A member whose version is unchanged since the previous release SHALL NOT be re-uploaded, and its version SHALL NOT be bumped merely to make a workspace publish succeed.
- If the tagged version of `subx-cli` is already present on the registry, the job SHALL fail rather than publish anything.

**Recovery from a partial publication**

- A crates.io upload is irreversible: a published `name@version` can be yanked but never unpublished and never re-uploaded. The workflow SHALL NOT retry a failed publish automatically.
- Where one member has uploaded and the other has failed, the remaining member SHALL be published on its own with `cargo publish -p <name>` after the cause is fixed. The already-uploaded member SHALL NOT be re-uploaded, and its version SHALL NOT be bumped to permit a whole-workspace retry.
- Where the already-uploaded artifact is itself defective, it SHALL be yanked and superseded by a new version with a moved submodule pointer — that is a new release, not a repair of the failed one.

**Future tooling**

- Release automation introduced later SHALL be verified to handle a workspace member that is a git submodule before being adopted. `cargo-release` refuses submodule workspace members and `release-please`'s Rust strategy does not model submodules; neither SHALL be adopted on the basis that it works for ordinary Cargo workspaces.

#### Scenario: Both crates are published in dependency order at the split release

- **GIVEN** a `v*` tag at which `subx-core` has never been published and `subx-cli`'s version has changed
- **WHEN** the publish job runs
- **THEN** `subx-core` SHALL be uploaded before `subx-cli`, and `subx-cli`'s verification build SHALL resolve `subx-core` from the packaged workspace artifact rather than waiting on registry index propagation

#### Scenario: An uncommitted submodule pointer stops the release before any upload

- **GIVEN** a checkout in which the `subx-core` gitlink has moved without being committed, or the submodule worktree has uncommitted changes
- **WHEN** the publish job runs
- **THEN** it SHALL fail in its precondition step with a message naming the submodule, and SHALL NOT reach `cargo publish`

#### Scenario: `--allow-dirty` is refused rather than used

- **GIVEN** a publish that fails Cargo's recursive submodule dirty check
- **WHEN** the failure is addressed
- **THEN** the resolution SHALL be to commit the submodule state, and adding `--allow-dirty` to the workflow SHALL fail the mechanical check that asserts its absence

#### Scenario: An unchanged member is not re-uploaded

- **GIVEN** a release in which `subx-cli`'s version has changed and `subx-core`'s has not
- **WHEN** the publish job determines which members require publication
- **THEN** `subx-core` SHALL be excluded from the publish, and its version SHALL NOT be bumped in order to make the command succeed

#### Scenario: A Cargo too old to publish a workspace fails at the first step

- **GIVEN** a publish job whose toolchain provides a Cargo older than the required floor
- **WHEN** the job runs
- **THEN** it SHALL fail in its version-assertion step naming the required floor, rather than failing later on an unrecognised `--workspace` argument

#### Scenario: A half-published release is completed, not retried wholesale

- **GIVEN** `subx-core` uploaded successfully and `subx-cli` then failed
- **WHEN** the release is completed after the cause is fixed
- **THEN** only `subx-cli` SHALL be published, and neither a whole-workspace retry nor a version bump of the already-published member SHALL be used

### Requirement: Release Version Bump and Changelog Header

A release SHALL carry version numbers that are chosen per crate and a changelog section the release workflow can actually parse.

- The two crates SHALL carry independent version numbers. Neither SHALL be derived from, inherited from, or forced to match the other; they are related only by the caret requirement in `subx-cli`'s dependency declaration.
- The release that ships the crate split SHALL bump `subx-cli` to a new **major** version. Its library surface changes shape — its modules become re-exports of another crate and two error methods become extension-trait methods — even though the binary's observable behaviour, flags, configuration keys and output envelopes are unchanged. The changelog entry SHALL state that distinction in its first line, because the binary's users will otherwise read a major bump as a behavioural break.
- `subx-core` SHALL be released at `1.0.0` and SHALL carry its own changelog with a corresponding section from the release at which it is first published. A crate published at 1.0.0 with no release record is a defect at the moment of publication, not a documentation gap to be tidied afterwards.
- `Cargo.lock` SHALL be regenerated by running Cargo after a version bump, and SHALL NEVER be hand-edited.
- Each release's changelog section SHALL begin with a heading of the exact form `## [<version>]` at the start of a line, optionally followed by a date, and SHALL be terminated by the next `## [` heading. The release workflow extracts release notes by matching that shape; a missing or misspelled heading does **not** fail the release — the workflow silently substitutes a generic one-line body — so the heading SHALL be verified by running the workflow's own extraction against the edited file and asserting non-empty output, rather than by inspection.
- The changelog SHALL use the project's Keep a Changelog section names (`### Added`, `### Changed`, `### Fixed`, `### Removed`, `### Migration`, `### Documentation`).

#### Scenario: Major bump is explained in terms of the library, not the binary

- **WHEN** the release that ships the crate split is documented
- **THEN** the changelog entry SHALL state that the major bump is a library-surface change and that the command-line behaviour is unchanged

#### Scenario: Release notes are extracted successfully from the changelog

- **GIVEN** a changelog section written for the version being tagged
- **WHEN** the release workflow's extraction expression is run against `CHANGELOG.md` with that version
- **THEN** it SHALL produce non-empty output, and that check SHALL be performed before the tag is pushed

#### Scenario: A missing changelog heading is caught rather than silently substituted

- **GIVEN** a tag whose version has no matching `## [<version>]` heading
- **WHEN** the changelog is verified before tagging
- **THEN** the omission SHALL be detected, rather than the release shipping with the workflow's generic fallback body

#### Scenario: The two crates' versions move independently

- **GIVEN** `subx-cli` bumping to a new major version
- **WHEN** `subx-core`'s version is considered
- **THEN** it SHALL be chosen from `subx-core`'s own change history, and SHALL NOT be bumped merely because the other crate was
