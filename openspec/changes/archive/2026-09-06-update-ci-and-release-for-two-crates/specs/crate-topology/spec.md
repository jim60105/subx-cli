## ADDED Requirements

### Requirement: The Core Repository Has Its Own CI

The `subx-core` repository SHALL carry its own continuous-integration workflow, so that a commit pushed to it is validated in the repository that received it rather than in the superproject that eventually points at it.

- Without one, a core commit is compiled by nothing until somebody moves the submodule pointer. The first signal that core is broken then arrives inside a `subx-cli` pull request, attributed to the wrong repository, after the fact.
- The workflow file SHALL carry the same name as the superproject's equivalent, so the two can be reviewed by diff rather than by reading.
- It SHALL run the same checks over the same operating-system matrix as the superproject's test job. Narrowing the matrix SHALL NOT be done on the argument that the superproject covers the missing platforms: the library is where the platform-sensitive code lives, and deferring a platform's first signal to a pointer bump is the failure this workflow exists to prevent.
- Its checks SHALL be invoked through a quality script committed in the core repository, so that the set of checks has one definition rather than one in the workflow and another in the contributor's instructions.
- It SHALL audit the core repository's own committed lockfile.
- It SHALL generate coverage with the same exclusion argument the superproject uses, and SHALL upload it.

**The two permitted divergences, and only these two:**

- It SHALL omit jobs that build, smoke-test, package or publish an artifact the library does not produce — a release matrix, a cross-compilation toolchain, an installer step, and a publish job.
- It MAY report rather than gate a measurement whose reference value was derived elsewhere. Specifically, coverage measured in a standalone clone has the same denominator and a strictly smaller numerator than the superproject's workspace-attributed measurement, because the superproject's own tests exercise library code and are credited to the library. Enforcing a workspace-derived floor against a standalone numerator would fail permanently; inventing a second floor without a measurement would be inventing evidence. The enforcing gate SHALL remain the superproject's workspace run until a standalone measurement exists to derive a standalone floor from.

Any divergence outside those two SHALL be treated as drift to be reconciled, not as a local choice.

**Publication stays single-pathed.**

- The core repository SHALL NOT contain a publish job. The library is published by the superproject's workspace publish, because the submodule pointer is what defines which library commit a given superproject release corresponds to.
- Two publish paths for one crate SHALL NOT exist, because they permit a library version to reach the registry that no superproject release pins.

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

#### Scenario: The core repository does not publish

- **GIVEN** the core repository's workflows
- **WHEN** they are enumerated
- **THEN** none SHALL upload the crate to the registry, and publication SHALL happen only through the superproject's workspace publish

## MODIFIED Requirements

### Requirement: Configuration Duplicated into the Core Repository

Because a git submodule has its own index and worktree, and because a standalone clone of `subx-core` has no parent to inherit from, the following SHALL be present in the `subx-core` repository in its own right rather than relied upon from `subx-cli`:

- `LICENSE` — the full GPL-3.0-or-later text.
- `.gitignore` — the parent's ignore rules never reach inside a submodule, and the parent's `/target/` is root-anchored and would not match `subx-core/target/` even if they did.
- `.gitattributes` — including the `tests/fixtures/formats/** -text` rule. This rule SHALL be present before any byte-exact fixture is committed: an attributes rule added after the fact does not undo a line-ending normalisation that already occurred at `git add` time.
- `rustfmt.toml` — `edition` and `max_width`. Without it a standalone clone falls back to rustfmt's defaults, including edition 2015, and 2024-edition sources fail to parse.
- `.config/nextest.toml` — the `default`, `ci`, `quick` and `full` profiles. In a workspace build nextest reads the workspace root's copy and never the member's, so the member's copy exists solely for the standalone case and for the core repository's own CI.
- `.github/workflows/` — the core repository's own CI. A workflow in the superproject never runs for a commit pushed to the submodule's repository, so CI is per-repository in exactly the way the files above are.
- `scripts/quality_check.sh` — the core repository's own quality gate, invoked by its CI and available to a contributor working in a standalone clone. It SHALL be authored for this repository rather than ported from the superproject's: duplicating a large script creates two implementations of one contract, in two repositories, with no mechanism that notices when they drift.
- `.codegraph/.gitignore` — the self-ignoring file, so the core repository can be indexed independently.
- `[lints.rustdoc]`, `[lints.clippy]` and `[lints.rust]` — written out literally in `subx-core/Cargo.toml`. They SHALL NOT be inherited, per the workspace-inheritance prohibition. Their contents SHALL be kept in agreement with `subx-cli`'s by review; a drift between the two blocks is a violation of this requirement.
- `[package.metadata.docs.rs]` — per-package by definition.
- `Cargo.lock` — committed, so a standalone clone and the core repository's own CI resolve reproducibly. Inside the workspace this file is inert, because the workspace root's lockfile is the only one Cargo reads or writes. It SHALL NOT be used as a cache key for a workspace build, which resolves against the root lockfile alone.

A coverage-tool configuration file SHALL NOT appear in this list, in either repository. A file that no script, workflow or manifest reads is not configuration, and duplicating one "for parity" propagates the mistaken belief that its contents are in force. Coverage exclusions SHALL be expressed as an argument the coverage command actually receives, at every invocation site, with a single normative value.

All comments in these files SHALL be written in English, per the project's language rule. This applies to both repositories' copies: a file duplicated with translated comments while the original keeps untranslated ones is a divergence to be closed, not a permitted difference.

#### Scenario: Core repository formats itself correctly when cloned alone

- **GIVEN** a standalone clone of `subx-core`
- **WHEN** `cargo fmt -- --check` is run inside it
- **THEN** it SHALL apply the crate's own `rustfmt.toml` settings and SHALL NOT fall back to rustfmt defaults

#### Scenario: Byte-exact fixtures are protected from the first commit

- **GIVEN** the `subx-core` repository before any test fixture has been added
- **WHEN** its initial commit is created
- **THEN** `.gitattributes` SHALL already contain the `-text` rule covering the fixture directory

#### Scenario: Lint configuration is enforced in the core repository

- **GIVEN** a rustdoc intra-doc link in `subx-core` that does not resolve
- **WHEN** the crate is built in a standalone clone
- **THEN** the build SHALL fail, because `broken_intra_doc_links = "deny"` is declared in `subx-core`'s own manifest rather than inherited

#### Scenario: A standalone clone can run its own gate

- **GIVEN** a contributor working in a standalone clone of `subx-core`
- **WHEN** they look for the project's quality gate
- **THEN** `scripts/quality_check.sh` SHALL be present in that repository and SHALL run the crate's formatting, lint, documentation and test checks

#### Scenario: An unread coverage configuration file is not duplicated

- **GIVEN** a coverage configuration file that no script, workflow or manifest in either repository reads
- **WHEN** the per-repository configuration set is determined
- **THEN** the file SHALL be deleted from both repositories rather than duplicated, and its intent SHALL be expressed as an argument passed to the coverage command
