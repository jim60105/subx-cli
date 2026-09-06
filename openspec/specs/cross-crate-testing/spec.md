# Cross-Crate Testing

## Purpose

Govern how the test suite is allocated across the `subx-cli` / `subx-core` crate boundary: which crate owns which test file, how shared test helpers cross the repository boundary through the `test-support` feature in `subx-core`, how test files under `tests/` subdirectories are reached by Cargo through harness shims, how the CLI binary under test is named at compile time, how fixtures and media assets are resolved from `CARGO_MANIFEST_DIR`, where test feature gates (`slow-tests`) are declared, how `[dev-dependencies]` follow their use sites after the split, and how coverage is measured once across the workspace and gated by per-crate floors.

## Requirements

### Requirement: Test Ownership Follows the Crate Under Test

Every test file SHALL belong to exactly one crate, and the allocation SHALL be decided by what the test exercises, not by which crate name appears in its imports.

- The classification SHALL be applied in order, with the first matching rule deciding:
  1. A test that spawns the `subx-cli` binary — through `assert_cmd`, through `std::process::Command`, or through a shared helper that does either — SHALL belong to `subx-cli`, regardless of which library items it also imports.
  2. A test that names `subx_cli::cli` or `subx_cli::commands` SHALL belong to `subx-cli`, because those modules never enter `subx-core`.
  3. A test that names only library items — configuration, engines, errors, services, or the crate-root `Result` alias — SHALL belong to `subx-core`.
  4. A harness shim SHALL belong to the crate that owns the file it points at.
- A test that matches rule 1 **and** rule 3 is a mixed-mode test. It SHALL stay in `subx-cli`, and its library imports SHALL be rewritten to name `subx_core` directly rather than being used as a reason to relocate the file.
- A test SHALL NOT be relocated in order to follow a helper it uses. Where a helper is needed on both sides of the boundary, the helper SHALL move to the shared mechanism and the test SHALL stay where its subject is.
- Every test file that lives in `subx-core` SHALL import from `subx_core` directly. No test in either crate SHALL resolve a library item through `subx-cli`'s back-compatibility re-exports, including the crate-root macros.
- The absence of such resolution SHALL be enforced by an automated check that resolves the directories it walks from `CARGO_MANIFEST_DIR` and never from the working directory.

#### Scenario: A binary-driving test stays with the binary

- **GIVEN** a test file that spawns the `subx-cli` binary and also imports a library type
- **WHEN** its owning crate is determined
- **THEN** it SHALL be assigned to `subx-cli`, and the library import SHALL be rewritten to name `subx_core`

#### Scenario: A test of a command module cannot move to the library crate

- **GIVEN** a test file that constructs a clap argument struct and calls a command entry point
- **WHEN** relocation to `subx-core` is proposed
- **THEN** it SHALL be rejected, because `subx-core` contains neither the argument struct nor the command module

#### Scenario: The back-compatibility re-exports have no test consumers

- **GIVEN** the test suites of both crates after the split
- **WHEN** every file under each crate's `tests/` and `benches/` is searched for the re-exported module names and macro names prefixed by the CLI crate name
- **THEN** no match SHALL be found

#### Scenario: A helper does not decide where a test lives

- **GIVEN** a test that belongs to `subx-core` by rule 3 but uses a helper that also serves `subx-cli` tests
- **WHEN** the allocation is reviewed
- **THEN** the test SHALL be placed in `subx-core` and the helper SHALL be made reachable from both crates

### Requirement: Shared Test Helpers Live in `subx-core` Behind a `test-support` Feature

Test helper code needed by both crates SHALL live in `subx-core` behind a `test-support` feature, and SHALL NOT be duplicated across the two repositories.

- `subx-core` SHALL expose the shared helpers from a single module gated by `#[cfg(feature = "test-support")]`. The feature SHALL be off by default in both crates.
- Dependencies that exist only to serve the shared helpers — an HTTP mock server, an audio fixture writer — SHALL be declared `optional = true` and activated by the feature, so that a build without the feature does not resolve them.
- `subx-cli` SHALL enable the feature through a second declaration of `subx-core` under `[dev-dependencies]` carrying `features = ["test-support"]`, alongside its ordinary `[dependencies]` declaration. It SHALL NOT declare `test-support` as a feature of its own, so the feature can never be enabled by a `--features` flag on a release build.
- `subx-core` SHALL reach its own gated module from its own integration tests through a path-only self declaration under `[dev-dependencies]`. That declaration carries no version requirement, so `cargo package` removes it from the published manifest.
- A normal build of either crate SHALL NOT enable the feature. This SHALL be verified by inspecting the compiler invocations of a release build, not by inspection of the manifest.
- Helper code needed by only one crate SHALL stay in that crate's own `tests/` tree and SHALL NOT be placed behind the feature. The feature is for what crosses the boundary.
- A helper whose correctness depends on `CARGO_MANIFEST_DIR` expanding to the binary's crate SHALL stay in the crate that owns the binary, because the macro expands at compile time to the crate being compiled.
- A helper module with no consumer SHALL be deleted rather than relocated. Relocating unreferenced code moves a maintenance cost across a repository boundary without moving any value.
- The gated module MAY be exempted from the crate's missing-documentation lint, provided the module itself carries documentation stating what the gate is for. Intra-doc link resolution SHALL still apply to it.

#### Scenario: The feature is enabled for tests and absent from a release build

- **GIVEN** both crates with the `test-support` declarations in place
- **WHEN** a release build and a test build are each compiled with verbose output
- **THEN** the release build SHALL contain no compiler invocation enabling `test-support`, and the test build SHALL contain one

#### Scenario: The core crate's own integration tests reach the gated module

- **GIVEN** an integration test under `subx-core/tests/` that names an item from the gated module
- **WHEN** `cargo test` is run for `subx-core`, in the workspace and in a standalone clone
- **THEN** it SHALL compile and run in both, with no `--features` flag supplied on the command line

#### Scenario: The self declaration does not reach consumers

- **GIVEN** `subx-core` carrying a path-only self declaration under `[dev-dependencies]`
- **WHEN** `cargo package` is run
- **THEN** the manifest inside the produced archive SHALL NOT declare that dependency

#### Scenario: An optional helper dependency is absent without the feature

- **GIVEN** a consumer that depends on `subx-core` without enabling `test-support`
- **WHEN** its dependency graph is resolved
- **THEN** the HTTP mock server crate SHALL NOT appear in it

#### Scenario: A single-crate helper is not gated

- **GIVEN** a helper used only by tests that assert the binary's stdout envelope
- **WHEN** its home is decided
- **THEN** it SHALL be placed in `subx-cli`'s own test tree and SHALL NOT be added to the gated module

#### Scenario: An unreferenced helper is deleted

- **GIVEN** a helper module with no consumer in either crate's `src/`, `tests/` or `benches/`
- **WHEN** the split is performed
- **THEN** it SHALL be deleted, and the deletion SHALL be recorded in the changelog

### Requirement: Test Files Are Reached Through Harness Shims

Every `.rs` file under a crate's `tests/` tree SHALL be compiled by some test target, and the absence of a target SHALL be treated as a defect rather than as a way of disabling a test.

- Cargo auto-discovers only `tests/*.rs`. A file in a subdirectory of `tests/` SHALL therefore be reached by exactly one top-level harness shim declaring `#[path = "<subdir>/<file>.rs"] mod <name>;`, or SHALL be relocated to the top level.
- A shim SHALL contain only module declarations and documentation. Test functions SHALL NOT be added to a shim.
- Test crate roots SHALL include shared helpers with a plain `mod common;`. A `#[path]` attribute SHALL NOT be used where a plain `mod` declaration resolves to the same file.
- `#[path]` MAY additionally be used to include a single helper module without its siblings, where compiling the siblings would produce unused-code warnings. Such a use SHALL carry a comment stating that reason.
- Where a crate's test files can all sit at the top level, the crate SHALL have no shims and no `tests/` subdirectories, because a shim is overhead that exists only to work around discovery.
- A file under a non-auto-discovered subdirectory with no shim pointing at it SHALL be treated as a defect. It SHALL be wired up, relocated, or deleted — never left in place.
- The absence of such files SHALL be enforced by an automated check rather than by review. The check SHALL compare the set of `.rs` files under the non-auto-discovered subdirectories against the set of `#[path]` targets declared by the top-level files, and SHALL fail naming every file in the first set that is absent from the second.
- The check SHALL resolve the directories it walks from `CARGO_MANIFEST_DIR` and never from the working directory, and SHALL cover both crates' test trees. It SHALL live in the crate that is permitted to see the other, because the dependency direction between the two crates is one-way.
- When an undiscovered file is triaged, it SHALL be revived if its imports already resolve or resolve after a rewrite the same change is performing anyway, and deleted if reviving it would require authoring code that does not exist. A helper function that has never been written is not a repair.
- A revived test that fails on an assertion about behaviour that has legitimately changed SHALL have its assertion updated to characterise current behaviour. Production behaviour SHALL NOT be changed to satisfy a test that has never executed; where a revived test cannot pass without such a change, it SHALL be deleted and the finding recorded.

#### Scenario: A redundant path attribute is replaced

- **GIVEN** a test crate root declaring `#[path = "common/mod.rs"] mod common;`
- **WHEN** the declaration is reviewed
- **THEN** it SHALL be replaced by `mod common;`, because the plain declaration resolves to the same file

#### Scenario: A flat test tree needs no shims

- **GIVEN** a crate whose test files all sit directly under `tests/`
- **WHEN** its test targets are enumerated
- **THEN** every file SHALL be its own target and no `#[path]` shim SHALL exist

#### Scenario: A subdirectory file is reached exactly once

- **GIVEN** a test file under a subdirectory of `tests/`
- **WHEN** the top-level files are searched for `#[path]` attributes naming it
- **THEN** exactly one SHALL be found

#### Scenario: An orphan test file is detected

- **GIVEN** a `.rs` file under `tests/cli/` that no top-level file names in a `#[path]` attribute
- **WHEN** the discovery check is run
- **THEN** it SHALL fail and name the file, and the resolution SHALL be to wire it up, relocate it, or delete it

#### Scenario: A file whose repair would require authoring is deleted

- **GIVEN** an undiscovered test file that imports helper functions from a module which defines none
- **WHEN** it is triaged
- **THEN** it SHALL be deleted rather than wired up, because supplying the missing helpers would be authoring new tests rather than restoring existing ones

#### Scenario: A revived test with a stale assertion is characterised, not repaired

- **GIVEN** a newly compiled test that fails because the behaviour it asserts has legitimately changed since it was written
- **WHEN** the failure is addressed
- **THEN** the assertion SHALL be updated to describe current behaviour, and the production code SHALL NOT be changed

### Requirement: Fixtures and Media Assets Are Resolved From `CARGO_MANIFEST_DIR`

A test that reads a file from the repository SHALL resolve it from `CARGO_MANIFEST_DIR`, and SHALL NOT depend on the process working directory.

- This applies to integration tests, to `#[cfg(test)]` modules inside `src/`, and to benches alike. A path literal relative to the working directory SHALL be treated as a defect even where it currently happens to resolve.
- The reason SHALL be recorded where the rule is documented: Cargo sets a test binary's working directory to the *package* root, so a path that resolved before a module moved between packages silently stops resolving, and coverage runners, IDE runners and nested process spawns do not all agree on the working directory in the first place.
- Byte-exact parser fixtures SHALL live in the repository of the crate whose parsers they exercise, and SHALL be accompanied there by a `.gitattributes` rule disabling text normalisation for their directory.
- That rule SHALL be present in the destination repository **before** the first fixture is added to its index. An attributes rule added afterwards does not undo a normalisation that has already occurred, and the corrupted bytes are not recoverable from the destination repository alone.
- A move of byte-exact fixtures SHALL be verified by comparing file sizes before and after from a fresh checkout, not from the working tree that performed the move.
- Media assets read by tests SHALL live in the repository of the crate whose tests read them. Where tests in both repositories read the same asset, a small text asset MAY be duplicated; a large binary asset SHALL be placed with the crate that needs it and the other crate's tests SHALL NOT reach across the submodule boundary to find it.
- Assets and fixtures SHALL remain excluded from both crates' published archives.

#### Scenario: A working-directory-relative asset path is rejected

- **GIVEN** a test that opens a media file through a path literal relative to the working directory
- **WHEN** the test is reviewed
- **THEN** it SHALL be changed to join the path onto `CARGO_MANIFEST_DIR`

#### Scenario: A module that moved between crates still finds its fixtures

- **GIVEN** a `#[cfg(test)]` module that reads a fixture and has been relocated from one crate to another
- **WHEN** its tests are run in the new crate
- **THEN** the fixture SHALL be found, because it moved with the module and is resolved from the new crate's manifest directory

#### Scenario: The normalisation rule precedes the fixtures

- **GIVEN** a repository that will receive byte-exact fixtures
- **WHEN** the first such fixture is added to its index
- **THEN** the `.gitattributes` rule covering that directory SHALL already be committed

#### Scenario: Byte-exactness is verified from a fresh checkout

- **GIVEN** fixtures that carry CRLF line endings or a byte-order mark, moved between repositories
- **WHEN** the destination repository is cloned afresh and the files are measured
- **THEN** every file SHALL have the same byte size it had in the source repository

### Requirement: Test Feature Gates Are Owned Where the Gated Tests Live

A feature that gates tests SHALL be declared as a real feature by every crate that contains tests gated on it, and forwarded to the other crate where both are involved.

- `slow-tests` SHALL be declared in `subx-core` as the gate over the long-running tests and `#[cfg(test)]` modules that live there.
- `subx-cli` SHALL declare `slow-tests` as a pass-through that enables the core feature. The declaration SHALL remain a real `subx-cli` feature, because `subx-cli` tests carry their own `#[cfg(feature = "slow-tests")]` annotations permanently — not transitionally.
- Enabling the feature at the workspace root SHALL enable the gated tests in both crates in a single invocation, so that the existing quality and coverage scripts need no per-crate flag.
- A feature that exists to make helper code available to another crate's tests SHALL NOT be declared as a feature of the consuming crate. It SHALL be enabled through the dependency declaration alone, so that no command-line flag can turn it on for a release build.
- Neither crate's `default` feature SHALL contain a test-only feature.

#### Scenario: One flag enables the gated tests in both crates

- **GIVEN** the workspace built with the slow-tests feature enabled at the root
- **WHEN** the test suite runs across the workspace
- **THEN** the gated tests in both crates SHALL be compiled and executed

#### Scenario: A CLI-side gated test survives the split

- **GIVEN** a `subx-cli` test annotated with the slow-tests gate that will never move to `subx-core`
- **WHEN** `subx-cli` is built with the feature
- **THEN** the test SHALL be compiled, because the pass-through is a declared feature of `subx-cli` and not merely a forwarder

#### Scenario: The helper feature cannot be enabled from the command line

- **GIVEN** the crate that consumes the shared test helpers
- **WHEN** its declared features are enumerated
- **THEN** the helper feature SHALL NOT be among them, and enabling it SHALL be possible only through the dev-dependency declaration

### Requirement: Test-Only Dependencies Follow Their Use Sites

Each crate's `[dev-dependencies]` SHALL be derived from the test files that crate actually owns after the split.

- A test-only dependency SHALL be declared in every crate that has at least one use site for it across that crate's `src/`, `tests/` and `benches/`, and in no crate that has none.
- A test-only dependency with no use site in either crate SHALL be deleted, not relocated. A planned future use is not a use site.
- A dependency used both by feature-gated helper code under `src/` and by that crate's own tests SHALL be declared twice — once as an optional entry activated by the feature, once under `[dev-dependencies]`. That is the ordinary shape for such a dependency and SHALL NOT be treated as duplication.
- Bench targets SHALL be declared in the crate whose sources their benchmark subject lives in, and their harness dependency SHALL follow them.
- A dependency that was a normal dependency of the single crate and becomes test-only for one of the two crates SHALL move into that crate's `[dev-dependencies]` rather than staying in `[dependencies]` unused.

#### Scenario: An unused test-only dependency is deleted rather than moved

- **GIVEN** a `[dev-dependencies]` entry with no use site anywhere in either crate
- **WHEN** the test suite is split
- **THEN** the entry SHALL be deleted, and it SHALL NOT be added to the other crate's manifest

#### Scenario: A bench moves with its subject

- **GIVEN** a benchmark whose subject module now lives in `subx-core`
- **WHEN** the benches are allocated
- **THEN** both the bench source and its `[[bench]]` declaration SHALL be in `subx-core`, and the harness dependency SHALL be declared there

#### Scenario: A dependency serving both the gated helpers and the crate's own tests

- **GIVEN** an HTTP mock server crate used by the feature-gated helper module and by the crate's own integration tests
- **WHEN** the manifest is written
- **THEN** it SHALL appear as an optional entry activated by the feature and as a `[dev-dependencies]` entry, and neither SHALL be removed as redundant

### Requirement: The Binary Under Test Is Named at Compile Time

An integration test that spawns the CLI binary SHALL locate it through the compile-time environment variable Cargo defines for that binary target.

- Tests SHALL use `env!("CARGO_BIN_EXE_subx-cli")` to obtain the executable path.
- A runtime lookup by binary name — `assert_cmd`'s `Command::cargo_bin("…")` or any equivalent that takes the target name as a string — SHALL NOT be used. A wrong name in such a call is a runtime panic inside a test; the same mistake in the compile-time form is a build failure.
- The compile-time form SHALL be preferred additionally because it names the exact artefact built for the package under test. Once a repository contains more than one package sharing a target directory, a runtime search of that directory is no longer unambiguous.
- The conversion SHALL be applied to every spawn site, including those whose name is currently correct. Converting only the incorrect ones would leave in place the mechanism that made them possible.
- A test that is currently skipped SHALL NOT be treated as exempt. A skipped test with an unresolvable binary name is a latent defect and SHALL be corrected when found, and the skip SHALL be lifted so the correction is exercised.
- Where a lifted skip reveals a test that cannot pass, the skip MAY be restored, but SHALL then carry a comment naming the reason.

#### Scenario: A renamed binary breaks the build rather than a test run

- **GIVEN** integration tests that resolve the binary through the compile-time environment variable
- **WHEN** the `[[bin]]` target is renamed without updating the tests
- **THEN** compilation SHALL fail, and no test SHALL panic at run time on an unresolved name

#### Scenario: A stale binary-name string is corrected even in a skipped test

- **GIVEN** a skipped test whose spawn call names a binary target that does not exist
- **WHEN** the defect is found
- **THEN** the call SHALL be converted to the compile-time form, and the test SHALL NOT be left skipped as a substitute for the fix

#### Scenario: A correct name is converted too

- **GIVEN** a spawn site that names the binary correctly today
- **WHEN** the binary-name contract is applied
- **THEN** it SHALL also be converted, so that no runtime name lookup remains anywhere in either crate's test suite

### Requirement: Per-Crate Coverage Floors Over a Single Instrumented Run

Coverage SHALL be measured once across the whole workspace and reported per crate, and each crate SHALL carry its own floor.

- Instrumentation SHALL run once, across the workspace, so that a test in one crate exercising the other crate's lines is credited to the crate that owns those lines. Per-package instrumented runs SHALL NOT be used, because they would discard that attribution and turn a coverage floor into a statement about packaging rather than about testing.
- The report SHALL be split by source path prefix, and each crate's percentage SHALL be compared against its own floor.
- The combined workspace floor SHALL remain the value in force before the split, and SHALL remain the gate that may not regress. A change that moves code between crates without changing behaviour SHALL NOT move the combined figure; if it does, the discrepancy SHALL be investigated rather than absorbed.
- Each crate's floor SHALL be derived from a measurement taken once the test suite has reached its final composition, set below the measured value by a fixed margin identical for both crates, and SHALL never be lowered without a proposal that states why. It MAY be raised.
- The measurement SHALL be taken after any change that alters which test files compile, because a newly compiled test file moves the numerator.
- Generated coverage SHALL exclude test trees, bench trees, the binary entry point, and the shared test-support module, whose own coverage is not meaningful and would otherwise inflate the owning crate's numerator.
- The exclusion SHALL be applied through the mechanism the coverage tool actually reads. A configuration file that no script, workflow or manifest references SHALL NOT be relied upon, and its presence SHALL NOT be taken as evidence that its contents are in force.
- The tooling that enforces the per-crate floors — script arguments, CI environment variables, and the default set of workspace members the test and lint commands act on — SHALL be specified by the change that measures, and implemented by the change that owns the CI and release pipeline.
- Where the default set of workspace members excludes a crate, the commands that run the test suite SHALL name the workspace explicitly, and the hazard SHALL be documented, so that a green result from a default invocation is not mistaken for a green suite.

#### Scenario: Cross-crate coverage is retained

- **GIVEN** a `subx-cli` test that exercises a `subx-core` engine
- **WHEN** coverage is generated for the workspace and reported per crate
- **THEN** the lines it executed in `subx-core` SHALL count toward `subx-core`'s percentage

#### Scenario: The combined floor is unchanged by a pure relocation

- **GIVEN** a change that moves tests between crates without changing production behaviour
- **WHEN** the combined workspace coverage is measured before and after
- **THEN** it SHALL be unchanged within measurement noise, and the combined floor SHALL NOT be adjusted

#### Scenario: A floor is a ratchet

- **GIVEN** a crate whose measured coverage has risen well above its floor
- **WHEN** a later change reduces it below the floor
- **THEN** the gate SHALL fail, and the resolution SHALL be to restore coverage rather than to lower the floor

#### Scenario: The measurement follows the last change to the suite's composition

- **GIVEN** a change that causes previously uncompiled test files to be compiled
- **WHEN** the per-crate floors are derived
- **THEN** the measurement SHALL be taken after that change, not before it

#### Scenario: An inert configuration file is not treated as configuration

- **GIVEN** a coverage configuration file that no script, workflow or manifest reads
- **WHEN** exclusions are specified
- **THEN** they SHALL be expressed through an argument the coverage command actually receives, and the inert file SHALL NOT be cited as the mechanism

#### Scenario: A default invocation that skips a crate is documented

- **GIVEN** a workspace whose default member set is the root package alone
- **WHEN** a contributor runs the test suite without naming the workspace
- **THEN** the documentation SHALL warn that the other crate's tests were not run, and the project's own verification SHALL name the workspace explicitly
