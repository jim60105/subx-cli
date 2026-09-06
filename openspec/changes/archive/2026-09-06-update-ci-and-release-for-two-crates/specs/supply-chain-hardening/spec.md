## ADDED Requirements

### Requirement: Submodule Pointer Is a Supply-Chain Input

The git submodule pointer names external source code by an identifier that can be stale, unreachable, or incompatible with the version requirement declared against it. It SHALL therefore be subject to the same freshness, provenance and verification expectations as any other dependency reference, and SHALL NOT be treated as repository plumbing.

**Provenance**

- `.gitmodules` SHALL record the tracking branch for the submodule. Without it, "the pointer is behind" has no referent and an automated updater has nothing to compare against.
- The pointer SHALL be committed in the superproject whenever the intended commit changes, and SHALL NEVER be bypassed with `--allow-dirty` during packaging or publication.

**Freshness**

- An automated dependency updater SHALL be configured for the git-submodule ecosystem, so that a pointer left behind the tracking branch produces a reviewable proposal rather than going unnoticed. Its proposals SHALL pass the same verification as any other change to the pointer.

**Verification**

- CI SHALL assert that the version declared by the pinned commit satisfies the version requirement declared against the submodule crate in the consuming manifest. Nothing in Cargo checks this: inside a workspace a path dependency resolves by path and the version requirement is never consulted for resolution, so the first symptom of a mismatch would otherwise be a publish that resolves against a registry crate different from the one the pointer built against.
- Both values SHALL be read from `cargo metadata`, so that the comparison is made against Cargo's normalised requirement rather than against the manifest's surface text.
- CI SHALL assert that the pinned commit is reachable from the tracking branch, which catches a commit that was force-pushed away and a commit on a branch that was never merged.
- The reachability assertion SHALL be advisory on pull requests, where the matching commit in the other repository may legitimately still be in review, and SHALL be blocking on pushes to the default branch and in the publish job. An assertion that must routinely be ignored will be ignored when it matters.

#### Scenario: A pinned version outside the declared requirement fails CI

- **GIVEN** a consuming manifest requiring the submodule crate at a caret range
- **WHEN** the pointer is moved to a commit whose declared version falls outside that range
- **THEN** CI SHALL fail naming both values, rather than the mismatch first appearing during publication

#### Scenario: A pointer into an unreachable commit blocks the default branch

- **GIVEN** a pointer naming a commit that is not an ancestor of the tracking branch
- **WHEN** the change is pushed to the default branch
- **THEN** CI SHALL fail; and **WHEN** the same state appears on a pull request, the check SHALL report without failing

#### Scenario: A stale pointer produces a proposal rather than silence

- **GIVEN** the tracking branch has advanced beyond the pinned commit
- **WHEN** the automated updater runs on its schedule
- **THEN** it SHALL open a proposal moving the pointer, and that proposal SHALL be subject to the version and reachability assertions like any other change

### Requirement: Each Published Crate Is Audited in Its Own Repository

Every crate this project publishes SHALL have its dependency graph audited in the repository that owns and publishes it, against the lockfile that repository commits.

- A workspace lockfile records the **union** of every member's resolution. It is the correct audit surface for the shipped binary and the wrong audit surface for a library that consumers resolve on its own.
- A library published from a submodule repository SHALL therefore run its own audit against its own committed lockfile, in its own CI. Auditing only the superproject's union leaves the graph that library's consumers actually resolve audited by nobody.
- A package appearing in one lockfile and not the other SHALL NOT be treated as an inconsistency. The two resolutions legitimately differ, and both are in scope.
- The superproject's audit job SHALL check the submodule out. The audit reads the lockfile, but lockfile regeneration and any mode that distinguishes direct from transitive dependencies must load the workspace manifest, which fails outright when the member manifest is absent.

#### Scenario: The library's own graph is audited where the library lives

- **GIVEN** a library crate published from its own repository and consumed by the superproject as a submodule
- **WHEN** its CI runs
- **THEN** it SHALL audit its own committed lockfile, independently of the superproject's audit of the workspace lockfile

#### Scenario: The superproject audit requires the member manifest

- **GIVEN** the superproject's audit job
- **WHEN** its checkout step is inspected
- **THEN** it SHALL fetch submodules, so that every Cargo operation the audit depends on can load the workspace manifest

#### Scenario: Divergent resolutions are both in scope

- **GIVEN** a package present in the workspace lockfile but absent from the library's own lockfile
- **WHEN** the two audit surfaces are compared
- **THEN** the difference SHALL be accepted as the expected consequence of two independent resolutions, and neither audit SHALL be narrowed to match the other

## MODIFIED Requirements

### Requirement: CI cargo audit gate

The project's existing `cargo audit` CI step SHALL be verified to fail the build pipeline on any direct dependency with a known vulnerability advisory. If the current configuration allows advisory-only warnings without failing, it SHALL be tightened to enforce failure.

The surface that `cargo audit` examines SHALL be the dependency graph resolved from `Cargo.lock`, and that graph SHALL contain only packages reachable from a manifest entry that satisfies the "Every Declared Dependency Has a Use Site" requirement. A package that enters the resolved graph solely through a declaration with no use site SHALL be treated as an audit-surface defect and removed by deleting the declaration, so that every audit failure is attributable to code the crate actually builds against.

Once the project spans more than one crate, the lockfile named above is the **workspace** lockfile at the superproject root, which resolves the union of every member's manifest. It SHALL be the audited surface for the superproject, and it SHALL be resolvable — meaning the submodule providing any member manifest is checked out before the audit runs. It SHALL NOT be the only audited surface: each separately published crate additionally carries its own, per the "Each Published Crate Is Audited in Its Own Repository" requirement.

`Cargo.lock` SHALL be regenerated by Cargo — by running a build or `cargo update` — and SHALL NEVER be hand-edited. After a manifest change, the regenerated lockfile SHALL be reviewed against the expected package delta before being committed. This applies equally to a lockfile regenerated by a version bump.

Removing a direct declaration does not imply the package leaves the lockfile: a package retained as a transitive dependency of another crate legitimately remains in the resolved graph and remains in scope for the audit.

#### Scenario: vulnerable dependency fails CI

- **WHEN** a direct dependency has a RUSTSEC advisory
- **THEN** the CI pipeline fails

#### Scenario: clean dependencies pass CI

- **WHEN** no direct dependencies have advisories
- **THEN** the CI pipeline succeeds

#### Scenario: Unused declaration is not allowed to widen the audit surface

- **GIVEN** `notify` is declared in `Cargo.toml` with no use site, pulling `notify-types`, `inotify`, `inotify-sys`, `fsevent-sys`, `kqueue`, and `kqueue-sys` into the resolved graph
- **WHEN** the audit surface is reviewed
- **THEN** the declaration SHALL be deleted so that those seven packages leave the graph, rather than an advisory against any of them being suppressed or ignored

#### Scenario: Transitively-required package legitimately stays in the graph

- **GIVEN** the direct `once_cell`, `tokio-util`, `winapi`, and `libc` declarations are deleted for having no use site
- **WHEN** `Cargo.lock` is regenerated
- **THEN** those packages MAY still appear in the lockfile as transitive dependencies of crates that require them (for example `rustls` and `tempfile` for `once_cell`, `reqwest` and `h2` for `tokio-util`, `unrar_sys` for `winapi`, and `tokio` for `libc`), and their continued presence SHALL NOT be treated as a violation

#### Scenario: Lockfile is regenerated, never hand-edited

- **GIVEN** a dependency has been added, removed, or moved between manifest tables
- **WHEN** `Cargo.lock` is updated
- **THEN** the update SHALL be produced by running Cargo (for example `cargo build`, including once with `--features archive-rar` so the optional `unrar` subtree is re-resolved), and the resulting diff SHALL be reviewed rather than authored by hand

#### Scenario: The audited workspace lockfile requires every member manifest

- **GIVEN** a workspace whose members include a crate provided by a git submodule
- **WHEN** the audit job runs without that submodule checked out
- **THEN** the job SHALL be treated as non-conforming, because the lockfile it audits cannot be resolved against a workspace manifest that is missing a member
