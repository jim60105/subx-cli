## ADDED Requirements

### Requirement: Reference Documentation Is Crate-Qualified and Singly-Homed

The project's reference documentation under `docs/` describes code that now lives in two repositories while itself living in one. Every source citation in it SHALL therefore name the repository that holds the file, and the documentation set SHALL NOT be duplicated to follow the code.

**Location and duplication**

- The reference documentation SHALL remain a single tree in `subx-cli/docs/`. It SHALL NOT be copied, mirrored, or partially relocated into `subx-core`, whose reference documentation for its own public interface is its rustdoc.
- A reference document whose subject matter is mostly the library SHALL be crate-qualified in place rather than moved. A document describing a procedure that spans both repositories SHALL remain one document, and SHALL name the owning repository at each step rather than being cut at the repository boundary — a procedure severed at that boundary yields two documents neither of which can be followed to completion.
- `subx-core` SHALL reach these documents by absolute URL. A relative path from inside the submodule resolves on neither the git forge, the registry, nor the documentation service.

**Citation form**

- A citation whose purpose is to send the reader to a file, including any citation carrying a line or line-range, SHALL use a repository-relative path prefixed with the owning repository's name — for example `subx-core/src/config/field_validator.rs:30-35` or `subx-cli/src/cli/config_args.rs`. Such a path is literally correct from the superproject working directory, which is where a reader following a checklist stands.
- A citation whose purpose is to name a publicly reachable API item SHALL use the crate path — for example `subx_core::core::input::InputPathHandler` or `subx_cli::cli::error_ext::SubXErrorExt`.
- An unqualified `src/…` path SHALL NOT appear in any reference document. This is mechanically checkable and SHALL be checked when a reference document is edited.

**Documents that mirror machine-readable files, and documents that are snapshots**

- A reference document SHALL NOT hand-transcribe the contents of a manifest, lockfile, or other machine-readable file. Such a transcription drifts silently, is authoritative in appearance only, and answers no question the file itself does not answer more accurately. Where the transcription carries a rule the machine-readable file cannot express — such as which dependencies are permanently confined to one crate — the rule SHALL be retained as prose and the transcription SHALL be deleted, together with the heading that invites its regrowth.
- A reference document that records a point-in-time analysis rather than current behaviour SHALL declare itself as such, naming what it describes and stating which of its contents are not maintained. It SHALL NOT be presented in the same voice as the living documents around it, and it SHALL NOT be silently re-pathed into apparent currency. Where such a document holds content recorded nowhere else, it SHALL be demoted rather than deleted.
- Where an agent-facing skill or other automation maintains such a document, that automation's instructions SHALL agree with the demotion. An instruction to refresh content the document declares unmaintained SHALL be removed, because it would reverse the demotion on its next invocation.

#### Scenario: A source citation names its repository

- **GIVEN** any reference document under `docs/`
- **WHEN** its source-path citations are enumerated
- **THEN** every one SHALL begin with `subx-cli/` or `subx-core/`, and no unqualified `src/…` path SHALL remain

#### Scenario: A mostly-core document is qualified rather than moved

- **GIVEN** a reference document the majority of whose cited files live in `subx-core`
- **WHEN** its disposition is decided
- **THEN** it SHALL stay in `subx-cli/docs/` with each citation qualified, and SHALL NOT be relocated to or duplicated in `subx-core`

#### Scenario: A cross-repository procedure stays whole

- **GIVEN** a document specifying an ordered procedure whose steps edit files in both repositories
- **WHEN** it is updated for the two-crate layout
- **THEN** it SHALL remain a single document naming the repository at each step, and SHALL NOT be split into a per-repository pair

#### Scenario: A transcribed manifest is deleted rather than re-synchronised

- **GIVEN** a reference document containing a hand-maintained copy of manifest contents
- **WHEN** that copy is found to disagree with the manifest
- **THEN** the copy SHALL be deleted along with its heading, any rule it carried that the manifest cannot express SHALL be retained as prose, and it SHALL NOT be re-synchronised in place

#### Scenario: A dated analysis declares itself

- **GIVEN** a reference document whose content describes the codebase at a past commit
- **WHEN** it is retained
- **THEN** it SHALL state what it describes and which of its contents are unmaintained, and any automation that refreshes it SHALL have its instructions brought into agreement with that statement

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
The obligation to agree with the published release matrix extends beyond documents that name an asset or an installer flag. A document that states the matrix, its size, or the set of target triples SHALL agree with it, wherever that document sits and whatever its primary subject is. A count is as falsifiable as a name and drifts more quietly: three documents in this repository have each carried a different wrong number for the same five-target matrix, and none of them named an asset.

A documentation link that crosses the repository boundary is an obligation on both ends, and only one end can detect its breakage:

- Where a document in one repository links to a document, section or anchor in the other, the change that re-authors the **target** SHALL re-verify that link and SHALL correct it in the repository where the link lives, within the same unit of work.
- This obligation SHALL NOT be delegated to a continuous-integration check. Neither repository's workflow can see both ends: the library repository's workflow has no checkout of the superproject, and the superproject's workflow does not parse the library's `README.md`. A cross-repository link therefore has none of the protections an intra-doc link has under `broken_intra_doc_links = "deny"`, and none of the visibility a relative link has when a forge renders it as a missing target.
- A change that renames a section or anchor in a linked-to document SHALL treat the rename as reaching into the other repository, and SHALL record in its own artifacts which links it verified.


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


#### Scenario: A document stating the release matrix agrees with it

- **GIVEN** any document stating the number of release targets, or enumerating the target triples, outside the two READMEs
- **WHEN** it is compared against the release workflow's matrix
- **THEN** it SHALL name the same set, and a stated count that disagrees SHALL be corrected rather than tolerated as incidental to the document's main subject

#### Scenario: Re-authoring a link target carries the obligation to fix the link

- **GIVEN** `subx-core/README.md` linking by absolute URL to a section of `subx-cli/docs/tech-architecture.md`, and a change in `subx-cli` that renames that section
- **WHEN** that change is prepared
- **THEN** it SHALL resolve the link against the post-change document and SHALL correct it in `subx-core`, and SHALL NOT rely on either repository's continuous integration to detect the breakage
