## ADDED Requirements

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

## MODIFIED Requirements

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
