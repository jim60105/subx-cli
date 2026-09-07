## MODIFIED Requirements

### Requirement: The Core Repository Has Its Own CI

The `subx-core` repository SHALL carry its own continuous-integration workflow, so that a commit pushed to it is validated in the repository that received it rather than in the superproject that eventually points at it.

- Without one, a core commit is compiled by nothing until somebody moves the submodule pointer. The first signal that core is broken then arrives inside a `subx-cli` pull request, attributed to the wrong repository, after the fact.
- The workflow file SHALL carry the same name as the superproject's equivalent, so the two can be reviewed by diff rather than by reading.
- It SHALL run the same checks over the same operating-system matrix as the superproject's test job. Narrowing the matrix SHALL NOT be done on the argument that the superproject covers the missing platforms: the library is where the platform-sensitive code lives, and deferring a platform's first signal to a pointer bump is the failure this workflow exists to prevent.
- Its checks SHALL be invoked through a quality script committed in the core repository, so that the set of checks has one definition rather than one in the workflow and another in the contributor's instructions.
- It SHALL audit the core repository's own committed lockfile.
- It SHALL generate coverage with the same exclusion argument the superproject uses, and SHALL upload it.

**The two permitted divergences, and only these two:**

- It SHALL omit jobs that build, smoke-test or package a binary artifact the library does not produce — a release build matrix, a cross-compilation toolchain, and an installer step. (Its `release.yml` publishes the crate and creates the matching GitHub Release; that is the library's own release surface, not an omission of one.)
- It MAY report rather than gate a measurement whose reference value was derived elsewhere. Specifically, coverage measured in a standalone clone has the same denominator and a strictly smaller numerator than the superproject's workspace-attributed measurement, because the superproject's own tests exercise library code and are credited to the library. Enforcing a workspace-derived floor against a standalone numerator would fail permanently; inventing a second floor without a measurement would be inventing evidence. The enforcing gate SHALL remain the superproject's workspace run until a standalone measurement exists to derive a standalone floor from.

Any divergence outside those two SHALL be treated as drift to be reconciled, not as a local choice.

**Publication is single-pathed per crate.**

- Each crate SHALL have exactly one publish path: `subx-core` publishes from a `v*` tag in its own repository, and `subx-cli` publishes from a `v*` tag in this one. A workflow in either repository SHALL NOT publish the other repository's crate, and no publish SHALL take the whole Cargo workspace.
- The invariant this protects is that every published version names the commit of the repository that owns it: the library version the GUI consumer at `jim60105/subx` depends on is tagged by the library's own release, and the CLI's publish job asserts (by submodule-cleanliness, gitlink ancestry from `subx-core`'s `master`, and a registry-index probe for a `subx-core` version satisfying the caret requirement) that the library it consumes is already published. A second publish path for one crate SHALL NOT exist.

#### Scenario: A broken core commit is caught before the pointer moves

- **GIVEN** a commit pushed to the `subx-core` repository that fails formatting, linting or tests
- **WHEN** its CI runs
- **THEN** it SHALL fail in that repository, before any `subx-cli` change pins it

#### Scenario: The core workflow runs the same platform matrix

- **GIVEN** the superproject's test job and the core repository's test job
- **WHEN** their operating-system matrices are compared
- **THEN** they SHALL be the same, and a narrower core matrix SHALL be treated as drift

#### Scenario: Core coverage reports without gating

- **GIVEN** a standalone coverage run in the core repository
- **WHEN** its percentage falls below the floor the superproject enforces for that crate
- **THEN** the core job SHALL still succeed, because its numerator excludes the superproject's tests, and the enforcing gate SHALL remain the superproject's workspace run

#### Scenario: Each repository publishes only its own crate

- **GIVEN** the workflows of both repositories
- **WHEN** they are enumerated
- **THEN** `subx-core`'s release workflow SHALL upload only `subx-core`, `subx-cli`'s publish job SHALL upload only `subx-cli`, and no workflow SHALL upload the other repository's crate or the whole workspace
