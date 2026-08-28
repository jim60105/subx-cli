## ADDED Requirements

### Requirement: Every Requirement Has Exactly One Owning Repository

From the moment SubX's specification tree is split, every requirement in the project SHALL be owned by exactly one repository — `subx-cli` or `subx-core` — and SHALL appear in exactly one `openspec/specs/` tree.

Ownership SHALL be decided by the following test, applied in order and stopping at the first that resolves:

1. **The changed-file test.** A requirement is owned by the repository containing the source files that must change to satisfy it. A requirement satisfiable only by editing `subx-core/src/**` is core-owned; one satisfiable only by editing `subx-cli/src/cli/**`, `subx-cli/src/commands/**`, `subx-cli/src/main.rs` or `subx-cli/src/lib.rs` is CLI-owned.
2. **The dependency test.** Where the first test is ambiguous, a requirement that can only be satisfied using a permanently CLI-only dependency — `clap`, `clap_complete`, `colored`, `tabled`, `indicatif` (SDR D8, SDR §4) — is CLI-owned.
3. **The scenario-citation test.** Where both are still ambiguous, the requirement is owned by the repository holding the files its scenarios cite as evidence.

The following SHALL NOT be used as ownership evidence, because each of them is narrative framing rather than a claim about which code must change:

- A scenario that reaches core behaviour through a user-facing invocation (`subx config set …`, `subx match -i …`) where the normative object is a core function such as `validate_ai_config`, `normalize_ai_provider` or `InputPathHandler::collect_files`.
- A prose sentence describing where a value originates (for example, "the recursion flag originates from the CLI `--recursive` option").
- A capability's title, its `## Purpose` wording, or the name of the command a user happens to run.

A requirement SHALL NOT be duplicated into both trees, and SHALL NOT be absent from both. The union of the two trees is the project's specification; neither tree alone is.

#### Scenario: A CLI-implemented requirement inside a core-looking capability stays behind

- **GIVEN** a capability whose subject matter is core, containing a requirement whose normative prose constrains files under `subx-cli/src/commands/`
- **WHEN** ownership is resolved
- **THEN** the changed-file test SHALL assign that requirement to `subx-cli`, and the capability SHALL be treated as split rather than as wholesale core-owned

#### Scenario: Narrative CLI framing does not move a core requirement

- **GIVEN** a requirement whose normative object is a function in `subx-core/src/config/field_validator.rs`, reached in its scenario through `subx config set`
- **WHEN** ownership is resolved
- **THEN** the requirement SHALL be core-owned, because the invocation is narrative framing and no `subx-cli` file must change to satisfy it

#### Scenario: A requirement is never in both trees

- **GIVEN** the two `openspec/specs/` trees at any commit
- **WHEN** the set of requirement titles in each capability of each tree is enumerated
- **THEN** no title SHALL appear under the same capability name in both trees

### Requirement: A Capability Moves Wholesale, Splits, or Stays — It Is Never Duplicated or Left as a Pointer

A capability SHALL be disposed of in exactly one of three ways, determined by where its requirements are owned.

- **Wholesale move.** Every requirement is owned by the other repository. The capability directory is removed from the source tree in its entirety and re-created in the destination tree with its full text. The source repository SHALL NOT retain a stub, a pointer, a redirect file, or an emptied capability.
- **Split.** Requirements are owned on both sides. The capability is expressed as two capabilities **of the same name**, one per repository, each with its own `## Purpose` naming the paths it actually covers. The shared name is deliberate: a reader asking "what does SubX specify about *X*" reads both files, and a name change would hide the second half.
- **Stay.** Every requirement is owned by the repository already holding it. Nothing happens.

An emptied capability SHALL NOT be left in place. `openspec validate --strict` rejects a spec with zero requirements (`Spec must have at least one requirement`), so an emptied capability directory is not a legal state and SHALL be deleted from the working tree.

A capability SHALL NOT be duplicated into both repositories in full, even where duplication would be convenient for a reader. Two copies of one requirement drift, and there is no mechanism in either repository that would detect the drift.

#### Scenario: A wholesale move leaves nothing behind

- **GIVEN** a capability every requirement of which is core-owned
- **WHEN** it is moved to `subx-core`
- **THEN** its directory SHALL be absent from `subx-cli/openspec/specs/`, with no stub file and no pointer, and its full text SHALL be present in `subx-core/openspec/specs/` under the same directory name

#### Scenario: An emptied capability is deleted, not kept

- **GIVEN** a delta that removes every requirement of a capability
- **WHEN** the change is applied
- **THEN** the capability directory SHALL be deleted, because `openspec validate --strict` reports `Spec must have at least one requirement` for a spec with an empty `## Requirements` section

#### Scenario: A split capability keeps one name in two repositories

- **GIVEN** a capability with requirements owned on both sides
- **WHEN** it is split
- **THEN** both repositories SHALL hold a capability directory of the same name, each with its own `## Purpose`, and neither SHALL contain a requirement the other contains

### Requirement: A Capability That Grows a Requirement on the Other Side of the Line Is Split, Not Moved

When a capability that lives wholly in one repository acquires a requirement owned by the other, the change that adds that requirement SHALL split the capability rather than relocate it or restate the requirement out of place.

- The adding change SHALL create the capability of the same name in the other repository, with a `## Purpose` of its own, and SHALL add the new requirement there.
- The adding change SHALL NOT move the existing requirements to follow the new one, and SHALL NOT weaken the new requirement's prose so that it appears to belong where the capability already is.
- Because such a change spans both repositories, it SHALL follow the two-repository change protocol specified below.
- Where the new requirement is genuinely satisfiable in either repository, it SHALL be placed in `subx-core`, because `subx-core` has consumers `subx-cli` does not — the Tauri GUI at `../subx` depends on `subx-core` only.

#### Scenario: A new CLI requirement lands in a core-owned capability

- **GIVEN** `vad-speech-detection`, wholly owned by `subx-core`, and a proposal adding a requirement for a `--vad-report` terminal table
- **WHEN** the proposal is written
- **THEN** it SHALL add a `vad-speech-detection` capability to `subx-cli/openspec/specs/` containing only the new requirement, and SHALL leave `subx-core`'s ten requirements untouched

#### Scenario: An ambiguous requirement defaults to core

- **GIVEN** a requirement that could be satisfied by a function in either crate with equal effort
- **WHEN** its owner is chosen
- **THEN** it SHALL be assigned to `subx-core`, so that the GUI consumer is not excluded from the behaviour it specifies

### Requirement: Capabilities Governing the Repository Relationship Live in `subx-cli`

A capability whose subject is the relationship between the two repositories SHALL live in `subx-cli/openspec/specs/` and SHALL NOT be moved, split, or mirrored into `subx-core`.

- `crate-topology`, `cross-crate-testing` and `spec-governance` are such capabilities and SHALL stay in `subx-cli`.
- The reason is that each of them is a statement about both repositories at once. Splitting one across the boundary would leave a reader of either half without the rule: "every job checks the submodule out" and "the core repository has its own CI" are one contract, not two.
- `subx-cli` is the correct home rather than `subx-core` because `subx-cli` is the parent: it holds the workspace root, `.gitmodules`, the submodule pointer, and the release pipeline that publishes both crates. A rule about the pair is enforceable from the parent and only partly visible from the child.
- A standalone clone of `subx-core` therefore does not carry these rules. That is accepted: a standalone clone is a build target and a crates.io source, not the place where the two-repository contract is decided.

#### Scenario: The relationship capabilities are not mirrored

- **GIVEN** `subx-core/openspec/specs/`
- **WHEN** its capability directories are enumerated
- **THEN** none SHALL be named `crate-topology`, `cross-crate-testing` or `spec-governance`

#### Scenario: A new relationship rule is added in the parent

- **GIVEN** a proposal adding a rule about how the submodule pointer is verified
- **WHEN** its delta specs are written
- **THEN** the rule SHALL be added to `crate-topology` in `subx-cli`, even if the mechanism it describes runs inside `subx-core`'s CI

### Requirement: Citation Paths Are Resolved Against the Owning Repository

Every path citation in a spec SHALL be repository-root-relative to the repository owning that spec, and any citation naming a file in the *other* repository SHALL be qualified with that repository's name.

- The qualified form SHALL be `subx-cli:<path>` or `subx-core:<path>`, written inside the same backticks as the path.
- An unqualified path SHALL be read as belonging to the owning repository. An unqualified path that does not exist in the owning repository is a defect, not a stale citation to be tolerated.
- Because B2 relocated `src/core/**`, `src/services/**`, `src/config/**` and `src/error.rs` at **identical relative paths**, a citation of one of those paths in a moved spec SHALL be carried over verbatim and SHALL NOT be rewritten. A spec that moves to `subx-core` and cites `src/core/matcher/engine.rs` is correct as written.
- A `## Purpose` paragraph SHALL end with "Implemented in `…`" naming paths that exist in the owning repository. Where the pre-split Purpose named paths on both sides, the moved Purpose SHALL name only its own and SHALL state the other side's role in prose without a path.
- A `file.rs:line-line` citation SHALL be re-verified against the destination tree whenever a spec crosses a repository boundary, and corrected where an earlier change in the series shifted it. A line range carried across a boundary unverified is a defect.

#### Scenario: A core path needs no rewrite

- **GIVEN** a requirement in a moved spec citing `src/core/language.rs`
- **WHEN** the spec is read in `subx-core`
- **THEN** the citation SHALL resolve, because B2 preserved the relative path, and it SHALL NOT have been rewritten

#### Scenario: A cross-repository citation is qualified

- **GIVEN** a `subx-core` spec citing a test that stays in `subx-cli`
- **WHEN** the citation is written
- **THEN** it SHALL read `subx-cli:tests/<file>.rs`, so that it cannot be mistaken for a path inside `subx-core`

#### Scenario: A line range is re-verified at the boundary

- **GIVEN** a citation of the form `src/core/matcher/engine.rs:766-782` in a spec crossing into `subx-core`
- **WHEN** the import is prepared
- **THEN** the range SHALL be checked against `subx-core/src/core/matcher/engine.rs` and corrected if it no longer covers the cited construct

#### Scenario: A Purpose names only the owning repository's paths

- **GIVEN** a capability whose pre-split `## Purpose` named both `src/core/factory.rs` and `src/commands/match_command.rs`
- **WHEN** it is split
- **THEN** the `subx-core` half's Purpose SHALL name `src/core/factory.rs` and SHALL NOT cite a `subx-cli` path, and the `subx-cli` half's Purpose SHALL do the converse

### Requirement: Cross-Repository Capability References Are Qualified and Re-Qualified When They Move

A spec that refers to another capability by name SHALL name the repository holding it whenever that capability is in the other repository, and a change that moves a capability SHALL update every reference to it.

- The reference form SHALL be "the `<capability>` capability in `subx-core`" or "… in `subx-cli`". An unqualified capability reference SHALL be read as a reference within the same repository.
- A change that moves or splits a capability SHALL, as part of that change, re-qualify every surviving reference to it in both trees. Leaving a reference pointing at a capability that is no longer in the reader's tree is a defect introduced by the moving change, not by the referring spec.
- The referring spec SHALL be amended through a `## MODIFIED Requirements` delta, restating the affected requirement in full. Editing a main spec directly, outside the delta mechanism, SHALL NOT be used to repair a reference.

#### Scenario: A reference is qualified when its target leaves

- **GIVEN** `configuration-management` in `subx-cli` referring to "the `ai-provider-integration` capability", and a change moving `ai-provider-integration` to `subx-core`
- **WHEN** that change is written
- **THEN** it SHALL carry a `## MODIFIED Requirements` delta restating the referring requirement with the reference qualified as being in `subx-core`

#### Scenario: A reference is re-qualified when its target splits later

- **GIVEN** a `subx-core` spec referring to "the `configuration-management` capability in `subx-cli`", and a later change splitting `configuration-management` so that the referenced requirement's half becomes core-owned
- **WHEN** that later change is written
- **THEN** it SHALL re-qualify the reference to `subx-core`

### Requirement: A Change Spanning Both Repositories Is Authored as Two Changes and Archived by Hand

A single unit of work that alters specs in both repositories SHALL be expressed as one OpenSpec change per repository, and the pair SHALL be archived under the procedure below rather than by `openspec archive` alone.

- Each half SHALL be a complete change in its own repository's `openspec/changes/`, with its own `.openspec.yaml`, `proposal.md`, `design.md`, delta specs and `tasks.md`.
- Each half's `## Why` SHALL name the other half by change name and repository, so that neither can be read as self-contained when it is not.
- The two halves SHALL NOT be merged into one change living in one repository with deltas addressed at the other. OpenSpec resolves a change against exactly one root, and a delta cannot name a capability outside it.
- Where a delta removes every requirement of a capability, `openspec archive <change> -y` SHALL be expected to refuse, reporting `Rebuilt spec for '<capability>' failed validation. No files were changed.` The refusal is whole-change atomic: no capability in that change is updated, including capabilities whose deltas would have applied cleanly.
- That change SHALL therefore be archived with `openspec archive <change> -y --skip-specs`, and every delta in it SHALL then be applied by hand: capability directories removed with `git rm -r`, added capabilities written out as main specs with a real H1 and `## Purpose`, and modified requirements edited into their main specs. The change's `tasks.md` SHALL enumerate those hand steps individually.
- The commits SHALL be paired: the `subx-cli` commit SHALL carry the moved submodule gitlink for the matching `subx-core` commit, so that checking out either repository at that commit yields a consistent pair of specification trees.
- `openspec archive` in the *receiving* repository SHALL be expected to succeed, and SHALL be expected to write `# <capability> Specification` as the H1 and `TBD - created by archiving change <name>. Update Purpose after archive.` as the `## Purpose` for every newly created capability. Both SHALL be replaced by hand — with the Title Case capability name and the real Purpose paragraph — before the change is committed. `openspec validate --specs --strict` passes on the `TBD` text, so this step SHALL be a task rather than a check.

#### Scenario: A removing change refuses to archive normally

- **GIVEN** a change whose delta removes every requirement of at least one capability
- **WHEN** `openspec archive <change> -y` is run
- **THEN** it SHALL report `archive_spec_validation_failed` and change no file, and the resolution SHALL be `--skip-specs` plus the hand steps, not editing the delta to keep a placeholder requirement

#### Scenario: `--skip-specs` skips the additions too

- **GIVEN** a change carrying both a `## REMOVED Requirements` delta for one capability and an `## ADDED Requirements` delta creating another
- **WHEN** it is archived with `--skip-specs`
- **THEN** neither delta SHALL be applied, and the added capability's main spec SHALL be written by hand in the same commit

#### Scenario: The receiving repository's generated Purpose is replaced

- **GIVEN** a change in `subx-core` that adds capabilities and has just been archived
- **WHEN** each new `openspec/specs/<capability>/spec.md` is inspected
- **THEN** its H1 SHALL have been changed from `# <capability> Specification` to the Title Case capability name and its `## Purpose` SHALL have been changed from the `TBD` placeholder to a real paragraph ending in "Implemented in `…`"

#### Scenario: The two halves reference each other

- **GIVEN** a unit of work moving specs from `subx-cli` to `subx-core`
- **WHEN** both changes' `proposal.md` files are read
- **THEN** each `## Why` SHALL name the other change and the repository it lives in

### Requirement: The Two Specification Trees Are Isolated by Construction and Checked for Drift

The two `openspec/` roots SHALL remain independent, and their independence SHALL be relied upon rather than worked around.

- `subx-cli/openspec/` and `subx-core/openspec/` are separate OpenSpec roots. The parent's tooling does not descend into the submodule: `openspec list --specs` and `openspec validate --all` run at the `subx-cli` root enumerate only `subx-cli`'s capabilities, and the same commands run from inside `subx-core/` resolve the nearest root and enumerate only `subx-core`'s. No configuration is required to obtain this, and none SHALL be added to defeat it.
- `subx-core/openspec/config.yaml` SHALL be byte-identical to `subx-cli/openspec/config.yaml`. Both SHALL remain the unmodified `openspec init` template — `schema: spec-driven` plus the commented-out `context:` and `rules:` examples. Project context belongs in each repository's `AGENTS.md`, which the OpenSpec skills already read; a `context:` block would be a second home for the same facts and the two would drift. If a `rules:` block is ever added, it SHALL be added to both files in the same change.
- The two trees SHALL NOT be linked through `openspec store` registration or a `references:` key in either `config.yaml`. Such a reference resolves only through a per-machine store registration, so `openspec doctor` would emit `reference_unresolved` for every developer and every CI runner that has not registered the store, and the registration itself is not committable. Cross-tree links SHALL be the qualified prose forms specified above.
- Drift SHALL be checked, not assumed. A check SHALL enumerate the capability directory names in both trees and assert that no name appears in both **except** where that capability is deliberately split, and that the deliberately-split set is recorded in one place. A name in both trees that is not in that set is a duplication defect.
- The check SHALL additionally assert that every capability name in either tree is reachable from the split-ownership record, so that a capability created in `subx-core` without a corresponding entry is surfaced rather than silently accumulating.

#### Scenario: The parent root ignores the submodule's tree

- **GIVEN** a `subx-cli` checkout with the submodule initialised and `subx-core/openspec/specs/` populated
- **WHEN** `openspec list --specs` is run at the `subx-cli` repository root
- **THEN** it SHALL list only `subx-cli`'s capabilities, and `openspec validate --all` SHALL validate only those

#### Scenario: The submodule resolves its own root

- **GIVEN** the same checkout
- **WHEN** `openspec list --specs` is run from inside `subx-core/`
- **THEN** it SHALL list only `subx-core`'s capabilities

#### Scenario: An accidental duplicate is caught

- **GIVEN** a capability name present in both trees that is not recorded as a deliberate split
- **WHEN** the drift check runs
- **THEN** it SHALL fail and name the capability

#### Scenario: The two config files stay identical

- **GIVEN** the two `openspec/config.yaml` files
- **WHEN** they are compared
- **THEN** `diff` SHALL report no difference, and a change adding a `rules:` block to one SHALL add it to the other
