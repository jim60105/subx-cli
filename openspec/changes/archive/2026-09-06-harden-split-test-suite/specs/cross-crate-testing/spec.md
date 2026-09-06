## ADDED Requirements

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

## MODIFIED Requirements

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
