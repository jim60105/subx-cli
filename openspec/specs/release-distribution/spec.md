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

Documentation that instructs a user to invoke an installer input the installer is required to reject SHALL be treated as a defect in this requirement rather than as stale prose. Every user-facing document that names a release asset or an installer flag — not only the two READMEs — SHALL agree with the published matrix and with the installer's handling of `SUBX_LIBC` and `--musl`.

Two crates are published, and the documented install story SHALL distinguish them:

- `subx-cli` is installed as a binary — from a GitHub Release asset, from `scripts/install.sh`, or with `cargo install subx-cli`. Its documentation is the audience-facing entry point and SHALL retain the platform table, the asset names and the installer guidance.
- `subx-core` is consumed as a library dependency. Documentation SHALL state that a consumer wanting the library depends on `subx-core` and never on `subx-cli`, and SHALL NOT present a binary install path for it.
- `subx-core` SHALL carry its own `README.md`, written for a library consumer. It SHALL NOT reproduce the CLI's installer instructions, command table, or continuous-integration badges, and SHALL NOT reference repository assets that lie outside its own worktree — a relative path across the submodule boundary resolves on neither the git forge, the registry, nor the documentation service. Links into the superproject's documentation SHALL be absolute.

The build-from-source path SHALL be documented as requiring a recursive submodule checkout. A plain clone of `subx-cli` yields an empty submodule directory and a workspace that fails to resolve, so a from-source instruction that omits the recursive form documents a build that cannot succeed.

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

#### Scenario: A command reference contradicting the installer is a defect

- **GIVEN** a user-facing document listing release assets or installer flags outside the two READMEs
- **WHEN** its contents are compared against the published matrix and the installer's musl-input handling
- **THEN** any asset the workflow does not publish, and any instruction to set `SUBX_LIBC=musl` or pass `--musl` as a supported option, SHALL be corrected

#### Scenario: The library's README is not a copy of the CLI's

- **WHEN** `subx-core`'s `README.md` is read
- **THEN** it SHALL describe the crate as a library dependency with its own documentation site and continuous-integration status, SHALL NOT carry the CLI's installer instructions or command table, and SHALL reference the superproject's documents by absolute URL

#### Scenario: The from-source instruction includes the submodule

- **WHEN** a user follows the build-from-source instructions in either README
- **THEN** the instructions SHALL obtain the submodule, either by cloning recursively or by an explicit submodule initialisation step, and a plain clone SHALL NOT be presented as sufficient

### Requirement: Changelog entry for new artifacts

The project's `CHANGELOG.md` SHALL contain entries describing
material changes to the release artifact set:

- An `### Added` entry under the version that first publishes a new
  artifact, describing the new asset(s) so users can discover them
  from the changelog alone.
- A `### Removed` entry under the version that drops a previously
  published artifact, describing which asset(s) were removed and
  the supported migration path for affected users.

Each published crate SHALL carry its own changelog, and the two SHALL be independent:

- `subx-cli/CHANGELOG.md` records the CLI's releases; `subx-core/CHANGELOG.md` records the library's. Neither SHALL be a copy of the other, and neither SHALL be derived from the other, because the two crates carry independent version numbers related only by a caret requirement.
- A change confined to one crate SHALL record its entry in that crate's changelog only. A change that moves the submodule pointer and thereby alters the CLI's observable behaviour SHALL record an entry in both, because both are released.
- A crate SHALL NOT be published to the registry at a version for which its own changelog has no section. The registry page shows the changelog of the crate the reader installed; a published version with no record is a defect at the moment of publication.

A change whose effect is confined to documentation SHALL record a `### Documentation` entry. It SHALL NOT be recorded under `### Added`, `### Changed` or `### Fixed` merely because a section for it already exists, since the changelog's audience distinguishes a behavioural change from a corrected document.

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

#### Scenario: Each crate's changelog covers the version it publishes

- **GIVEN** a release publishing both crates
- **WHEN** each crate's changelog is inspected for the version being published
- **THEN** each SHALL contain a section for its own version, and neither SHALL rely on the other's record

#### Scenario: A documentation-only change is recorded as documentation

- **GIVEN** a change that alters no behaviour, flag, configuration key, output payload or public API
- **WHEN** its changelog entry is written
- **THEN** it SHALL appear under `### Documentation` in every repository it touches

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

### Requirement: Published Crate Documentation Sites

The project publishes two crates and therefore two documentation sites, `docs.rs/subx-cli` and `docs.rs/subx-core`. Both crates set `broken_intra_doc_links = "deny"`, and the dependency edge between them runs in exactly one direction. The two sites SHALL therefore be governed by an asymmetric link rule, and the rule SHALL be verified by a documentation build that generates dependency documentation rather than by one that suppresses it.

**The asymmetric link rule**

- `subx-cli`'s rustdoc MAY use intra-doc links naming `subx_core::…`, because `subx-core` is a declared dependency and the path resolves.
- `subx-core`'s rustdoc SHALL NOT contain an intra-doc link naming `subx_cli` or any path beneath it, in any form. `subx-cli` is not a dependency of `subx-core`; such a link does not resolve; and under `broken_intra_doc_links = "deny"` it is a hard build failure whose only manifestation is in a standalone `subx-core` checkout.
- Where `subx-core`'s documentation must refer to CLI behaviour — including the CLI-flavoured prose of `SubXError::hint()` and the fact that only the CLI constructs the `OutputModeUnsupported` variant — the reference SHALL be plain prose with a backticked crate or item name and SHALL NOT be a bracketed intra-doc link. A bare URL is not a substitute, because `bare_urls` is a warning-level lint in both manifests; a Markdown link is.
- The back-compatibility re-exports in `subx-cli`'s `lib.rs` cause `subx-cli`'s documentation site to present `subx-core`'s modules as its own. The re-export block's rustdoc SHALL state in prose that those paths exist for compatibility and that `docs.rs/subx-core` is the canonical documentation for the re-exported items. It SHALL NOT use `#[deprecated]`, which the project's conventions prohibit for new items.

**Verification**

- The documentation build in the quality gate SHALL select every workspace member, so that both crates' documentation is generated into one output tree and cross-crate links resolve locally. A `--no-deps` build SHALL NOT be treated as sufficient verification of the boundary: `--no-deps` suppresses generation of dependency documentation but not resolution of cross-crate intra-doc paths, so a link across the boundary satisfies the deny lint while producing an `href` into a directory that was never generated. The build therefore exits zero while the rendered link is dead.
- At least one cross-crate link SHALL be exercised by inspecting the generated output rather than by relying on the build's exit status.

**Documentation-site configuration**

- Each crate's `[package.metadata.docs.rs]` block SHALL activate only the features whose documentation belongs on a public API reference. It SHALL NOT activate a feature that exists to serve the project's own test suite. In particular, `subx-core`'s `test-support` feature gates relocated test scaffolding and pulls test-only dependencies into the documentation build; activating it on the published site presents that scaffolding as part of the crate's documented public surface.
- The items the project deliberately exposes as unconditional public API for consumer test suites SHALL remain documented and SHALL NOT be hidden from the documentation site in order to solve a feature-activation problem.

#### Scenario: A link from the library to the CLI fails the library's own build

- **GIVEN** a doc comment in `subx-core` containing an intra-doc link to a `subx_cli` path
- **WHEN** `subx-core` is built in a standalone checkout
- **THEN** the build SHALL fail on `broken_intra_doc_links`, and the correction SHALL be to restate the reference as plain prose rather than to relax the lint

#### Scenario: CLI documentation reaches into the library and resolves

- **GIVEN** a doc comment in `subx-cli` containing an intra-doc link to a `subx_core` path
- **WHEN** the documentation build selects every workspace member
- **THEN** the link SHALL resolve and the generated page for the target item SHALL exist in the same output tree

#### Scenario: A dead cross-crate link is not hidden by a green exit status

- **GIVEN** a documentation build that suppresses dependency documentation
- **WHEN** the boundary is verified
- **THEN** the exit status alone SHALL NOT be accepted as evidence, and the generated output SHALL be inspected for at least one resolved cross-crate link

#### Scenario: The published site does not present test scaffolding as public API

- **GIVEN** `subx-core`'s documentation-site metadata
- **WHEN** the crate is published and its documentation is built by the registry's documentation service
- **THEN** the feature gating the project's relocated test helpers SHALL NOT be active, and those helpers SHALL NOT appear on the published documentation site

#### Scenario: The re-exported modules name their canonical documentation

- **WHEN** a reader lands on `subx-cli`'s documentation site at one of the modules re-exported from `subx-core`
- **THEN** the module's documentation SHALL state that the path exists for compatibility and SHALL name `subx-core`'s documentation site as canonical

