## ADDED Requirements

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

## MODIFIED Requirements

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
