## MODIFIED Requirements

### Requirement: Two-Crate Publication Order and Preconditions

The project publishes two coupled crates, each from the repository that owns it: `subx-core` is published by a `v*` tag in `jim60105/subx-core`, and `subx-cli` is published by a `v*` tag in `jim60105/subx-cli`. `subx-cli` cannot be published before `subx-core` exists on the registry at a version satisfying its declared caret requirement, so the two tags form an ordered operation — library first — and each repository's publish job SHALL treat its upload as the last, most-guarded step rather than the first.

**Ordering and mechanism**

- Each repository's release workflow SHALL publish exactly one crate: core's workflow SHALL run `cargo publish -p subx-core`, and the CLI's `publish-crates` job SHALL run `cargo publish -p subx-cli`. A workflow SHALL NOT publish a crate owned by the other repository, and no publish command SHALL take the whole workspace.
- The consumer's job SHALL enforce the ordering before packaging: it SHALL read the caret requirement of the dependency from `cargo metadata --no-deps` (never from manifest text) and query the sparse crates.io index for a published version satisfying it (same major, and not below the requirement, ignoring pre-release versions). When none exists, the job SHALL fail with a message naming the remedy — publish the library first by tagging its version in `jim60105/subx-core` — and SHALL NOT reach `cargo publish`.
- A consumer-side version floor that exists only because a particular publish *flag* requires it (such as the retired `--workspace` Cargo ≥ 1.90 assertion) SHALL be deleted together with the flag, not left as an assertion the chosen command does not need.

**Preconditions, all checked before the first upload, in both repositories**

- The working tree the publish runs against SHALL be committed and clean. In `subx-cli` this SHALL be verified explicitly for the submodule — no `-`, `+` or `U` prefix from `git submodule status --recursive`, and no output from a porcelain status inside the submodule — so that the failure names the submodule rather than naming a dirty working directory. In `subx-core` the standalone equivalent SHALL be asserted: empty porcelain status of its own tree.
- `--allow-dirty` SHALL NOT be used, under any circumstance, to satisfy the preceding bullet. It records `"dirty": true` in the published archive's `.cargo_vcs_info.json`, permanently, asserting that the artifact corresponds to no commit. Its absence from the workflow files of both repositories SHALL be enforced mechanically, not by review.
- The tag SHALL name the version the manifest declares, and the tagged `name@version` SHALL NOT already be present on the registry index; either mismatch fails the job rather than publishing anything.
- A `cargo publish -p <crate> --dry-run` SHALL succeed before any upload is attempted, and SHALL additionally have been run locally before the release tag is pushed.
- In `subx-cli`, the hard submodule-pointer assertions stay: the gitlink commit SHALL be reachable from `subx-core`'s `master` (blocking, no exceptions — tag pushes never run the CI job whose equivalent is advisory on pull requests).

**Recovery from a failed publication**

- A crates.io upload is irreversible: a published `name@version` can be yanked but never unpublished and never re-uploaded. Neither workflow SHALL retry a failed publish automatically.
- Because each job performs a single upload guarded by every probe above, the half-published state the shared workspace publish could create no longer exists. Where a published artifact is defective, it SHALL be yanked and superseded by a new version — a new release in the owning repository, not a repair of the failed one.

**Future tooling**

- Release automation introduced later SHALL be verified to handle a workspace member that is a git submodule before being adopted. `cargo-release` refuses submodule workspace members and `release-please`'s Rust strategy does not model submodules; neither SHALL be adopted on the basis that it works for ordinary Cargo workspaces.

#### Scenario: Each crate publishes from its own repository's tag

- **GIVEN** `subx-core`'s `v<X>` tag has published `subx-core@X`, satisfying `subx-cli`'s caret requirement
- **WHEN** `subx-cli`'s `v<Y>` tag runs the publish job
- **THEN** the index probe SHALL pass, the job SHALL upload `subx-cli@Y` alone with `cargo publish -p subx-cli`, and no step SHALL attempt to upload `subx-core`

#### Scenario: The library must be tagged first, and saying so is part of the contract

- **GIVEN** a `subx-cli` `v*` tag pushed while no published `subx-core` version satisfies the caret requirement in the manifest
- **WHEN** the publish job runs
- **THEN** it SHALL fail at the index-probe step with a message naming `jim60105/subx-core` as the repository to tag first, before any package is built or uploaded

#### Scenario: An uncommitted submodule pointer stops the CLI release before any upload

- **GIVEN** a checkout in which the `subx-core` gitlink has moved without being committed, or the submodule worktree has uncommitted changes
- **WHEN** the CLI publish job runs
- **THEN** it SHALL fail in its precondition step with a message naming the submodule, and SHALL NOT reach `cargo publish`

#### Scenario: `--allow-dirty` is refused rather than used

- **GIVEN** a publish in either repository that fails Cargo's dirty check
- **WHEN** the failure is addressed
- **THEN** the resolution SHALL be to commit the tree state, and adding `--allow-dirty` to either repository's workflow SHALL fail the mechanical check that asserts its absence

#### Scenario: A tag that names an already-published version fails loudly

- **GIVEN** a `v*` tag in either repository whose `name@version` is already present on the registry index
- **WHEN** the publish job runs
- **THEN** it SHALL fail at the already-published probe rather than publishing or half-publishing anything

#### Scenario: A defective publication is superseded, not retried

- **GIVEN** a crate uploaded to the registry whose artifact is found defective after publication
- **WHEN** the release is corrected
- **THEN** the version SHALL be yanked and a new version released from the owning repository's tag, and neither workflow SHALL re-run or auto-retry the failed publish
