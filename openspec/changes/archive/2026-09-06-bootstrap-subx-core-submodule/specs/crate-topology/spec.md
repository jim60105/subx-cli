## ADDED Requirements

### Requirement: Repository Ownership and Submodule Mount

The SubX codebase SHALL be distributed across exactly two git repositories with a fixed containment relationship.

- `https://github.com/jim60105/subx-cli` SHALL own the `subx-cli` crate — the command-line binary and its library facade.
- `https://github.com/jim60105/subx-core` SHALL own the `subx-core` crate — the reusable library. It SHALL be licensed GPL-3.0-or-later and SHALL carry its own `LICENSE` file containing the full licence text.
- `subx-core` SHALL be mounted inside `subx-cli` as a git submodule at the repository-root-relative path `subx-core/`. It SHALL NOT be vendored, subtree-merged, or copied.
- `.gitmodules` in `subx-cli` SHALL record the submodule's `path`, its `url`, and `branch = main`. The `branch` entry SHALL be present even though it does not cause automatic updates: it defines the referent for "the pointer is behind".
- The submodule pointer SHALL be committed in `subx-cli` whenever the intended `subx-core` commit changes. A `subx-cli` commit that depends on a `subx-core` commit it does not pin is non-conforming.
- Before `cargo package` or `cargo publish` is run in either repository, the submodule pointer SHALL be committed and the submodule working tree SHALL be clean. Cargo checks submodules recursively for uncommitted changes and treats a moved pointer as dirty. `--allow-dirty` SHALL NOT be used to bypass this check, because it records `"dirty": true` in the published archive's `.cargo_vcs_info.json`.

This capability specifies repository and crate **structure**. Which individual source module belongs to which crate is specified separately and is not constrained by these requirements.

#### Scenario: Core repository is separately clonable

- **GIVEN** a developer with no copy of `subx-cli`
- **WHEN** they run `git clone https://github.com/jim60105/subx-core`
- **THEN** they SHALL obtain a complete, self-contained Rust crate that requires no other repository to build

#### Scenario: Submodule pointer accompanies a dependent change

- **GIVEN** a change to `subx-cli` that requires new behaviour from `subx-core`
- **WHEN** that change is committed
- **THEN** the commit SHALL include the moved gitlink at `subx-core/`, so that checking out the `subx-cli` commit yields the matching `subx-core` commit

#### Scenario: Publishing with a dirty submodule is refused, not bypassed

- **GIVEN** the `subx-core` submodule working tree contains uncommitted changes, or the pointer has moved without being committed
- **WHEN** `cargo package` or `cargo publish` is run
- **THEN** the command SHALL be allowed to fail, and the resolution SHALL be to commit the submodule state — `--allow-dirty` SHALL NOT be added

### Requirement: Cargo Workspace Shape

`subx-cli/Cargo.toml` SHALL be both a package manifest and the Cargo workspace root for the two crates.

- It SHALL declare `[workspace]` with `members = [".", "subx-core"]`, listing the root package explicitly.
- It SHALL declare `resolver` explicitly rather than relying on inference from the root package's edition.
- `[profile.release]` and `[profile.dev]` SHALL be declared in the workspace root manifest and SHALL NOT be declared in `subx-core/Cargo.toml`. Cargo ignores `[profile.*]` in a non-root workspace member and warns on every build when one is present; a warning-free build is a project requirement.
- The consequence SHALL be accepted that a standalone `subx-core` build uses Cargo's default profiles. In the workspace build and in any crates.io consumer's build, the governing profile is the root's or the consumer's respectively, so no shipped artifact is affected.

#### Scenario: Both crates are workspace members

- **GIVEN** a `subx-cli` checkout with the submodule initialised
- **WHEN** `cargo metadata --no-deps` is run at the repository root
- **THEN** the output SHALL list exactly the packages `subx-cli` and `subx-core` as workspace members

#### Scenario: Build emits no profile warning

- **GIVEN** the workspace as specified
- **WHEN** `cargo build` is run at the repository root
- **THEN** Cargo SHALL NOT emit a warning that profiles in a non-root package are ignored

#### Scenario: Uninitialised submodule fails at manifest load

- **GIVEN** a `subx-cli` checkout in which `subx-core/` is empty because the submodule was not initialised
- **WHEN** any Cargo command is run at the repository root
- **THEN** it SHALL fail while loading the workspace manifest, before compilation begins, naming `subx-core/Cargo.toml`

### Requirement: No Nested Workspace Root and No Workspace Inheritance in `subx-core`

`subx-core/Cargo.toml` SHALL NOT contain a `[workspace]` table, and SHALL NOT use any form of workspace inheritance.

- A `[workspace]` table in `subx-core/Cargo.toml` is a hard Cargo error while that directory is nested inside the `subx-cli` workspace, not a warning. `subx-core` is a member; a member cannot also be a root.
- `subx-core/Cargo.toml` SHALL NOT use `version.workspace = true`, `authors.workspace = true`, or any other `<field>.workspace = true` inheritance.
- It SHALL NOT reference `[workspace.dependencies]` via `<dep>.workspace = true`.
- It SHALL NOT reference `[workspace.lints]` via `[lints] workspace = true`.
- Every inherited form resolves against the workspace the package is currently a member of. Inside `subx-cli` all of them resolve successfully, which is precisely why the prohibition is normative: the breakage is invisible in the workspace build and appears only in a standalone clone, which is how crates.io consumers, `docs.rs`, and the downstream Tauri GUI see the crate.
- `subx-core` SHALL therefore build, format, lint and test correctly both as a workspace member and as a standalone clone. The standalone check SHALL be performed from a directory that is **not** inside the `subx-cli` tree, because Cargo walks upward and would otherwise find the parent workspace.

#### Scenario: Standalone clone builds without a parent workspace

- **GIVEN** `subx-core` cloned on its own into a directory outside any `subx-cli` checkout
- **WHEN** `cargo build` and `cargo fmt -- --check` are run inside it
- **THEN** both SHALL succeed, with no manifest error about an inherited field and no missing configuration

#### Scenario: Nested workspace table is rejected

- **GIVEN** a `[workspace]` table added to `subx-core/Cargo.toml`
- **WHEN** any Cargo command is run from the `subx-cli` workspace root
- **THEN** Cargo SHALL fail with a nested-workspace error, and the resolution SHALL be to delete the table rather than to add an `exclude` entry or a `package.workspace` key

#### Scenario: Inherited field breaks only the standalone case

- **GIVEN** `subx-core/Cargo.toml` declaring `version.workspace = true`
- **WHEN** `cargo build` is run from the `subx-cli` workspace root, and then again from a standalone clone of `subx-core`
- **THEN** the workspace build SHALL succeed and the standalone build SHALL fail — and that asymmetry SHALL be treated as a violation of this requirement, not as an acceptable state

### Requirement: Dual Path-and-Version Dependency Declaration

`subx-cli` SHALL depend on `subx-core` using both a `path` and a `version` key in a single declaration: `subx-core = { path = "subx-core", version = "1.0" }`.

- `path` SHALL point at the submodule mount and SHALL drive resolution for every local, workspace and CI build, so that a change to `subx-core` is visible to `subx-cli` without any publication step.
- `version` SHALL carry the caret requirement that survives publication. `cargo package` strips `path` from the manifest written into the `.crate` archive and leaves only the registry requirement.
- A `path`-only declaration SHALL NOT be used. A crate whose dependency is specified with `path` alone cannot be published to crates.io at all, because the stripped manifest would name a dependency with no source.
- A registry-only declaration SHALL NOT be used, because it would force a crates.io release between every core change and its consumption by the CLI.
- The same dual form SHALL be used for any `[dev-dependencies]` entry on `subx-core` that is added later.

#### Scenario: Local build resolves through the path

- **GIVEN** an edit to a file under `subx-core/src/`
- **WHEN** `cargo build` is run at the `subx-cli` workspace root
- **THEN** the edit SHALL be compiled into the resulting binary without any crates.io interaction

#### Scenario: Packaged manifest carries only the version requirement

- **GIVEN** the dual-form declaration
- **WHEN** `cargo package` is run for `subx-cli`
- **THEN** the manifest inside the produced `.crate` SHALL declare `subx-core` with its version requirement and SHALL NOT declare a `path`

#### Scenario: Path-only declaration is rejected before it reaches a release

- **GIVEN** a declaration reduced to `subx-core = { path = "subx-core" }`
- **WHEN** publication is attempted
- **THEN** it SHALL fail, and the resolution SHALL be to restore the `version` key rather than to work around the failure

### Requirement: Independent Version Lines and Their Caret Relationship

The two crates SHALL carry independent version numbers related only by the caret requirement in the dependency declaration.

- `subx-core` SHALL start at `1.0.0`. It is not new code: it is the library surface that has been shipping inside `subx-cli` and is already consumed in production by the downstream GUI. A `0.x` line would additionally forbid consumers from adopting a subsequent minor release automatically, because Cargo treats every `0.x` bump as potentially breaking.
- `subx-cli` SHALL move to `2.0.0` when the split is released, because its library surface changes shape — its modules become re-exports of `subx-core` — even though the binary's observable behaviour does not change. The bump is a release event and SHALL be performed together with the release-pipeline work, not with the creation of the skeleton.
- Neither crate's version SHALL be inherited from the other or from a workspace table.
- The version of the `subx-core` commit pinned by the submodule pointer SHALL satisfy the caret requirement declared in `subx-cli`'s dependency line. Cargo does not check this inside the workspace, where `path` wins outright and the `version` key is not consulted for resolution; the agreement SHALL therefore be asserted explicitly.

#### Scenario: Pinned core version satisfies the declared requirement

- **GIVEN** `subx-cli` declaring `subx-core = { path = "subx-core", version = "1.0" }`
- **WHEN** the pinned `subx-core` commit declares `version = "1.0.0"`
- **THEN** the requirement SHALL be satisfied, and any pinned version outside `>=1.0.0, <2.0.0` SHALL be treated as a build-breaking mismatch

#### Scenario: Version drift is detected without waiting for a publish

- **GIVEN** the pointer moved to a `subx-core` commit whose major version is no longer `1`
- **WHEN** the `subx-cli` test suite runs
- **THEN** the version-agreement assertion SHALL fail, rather than the mismatch surfacing for the first time during `cargo publish`

#### Scenario: The two version lines move independently

- **GIVEN** `subx-core` at `1.0.0` and `subx-cli` at `1.9.1`
- **WHEN** either crate is released
- **THEN** its version SHALL be chosen from its own change history, and SHALL NOT be forced to match the other crate's number

### Requirement: Configuration Duplicated into the Core Repository

Because a git submodule has its own index and worktree, and because a standalone clone of `subx-core` has no parent to inherit from, the following SHALL be present in the `subx-core` repository in its own right rather than relied upon from `subx-cli`:

- `LICENSE` — the full GPL-3.0-or-later text.
- `.gitignore` — the parent's ignore rules never reach inside a submodule, and the parent's `/target/` is root-anchored and would not match `subx-core/target/` even if they did.
- `.gitattributes` — including the `tests/fixtures/formats/** -text` rule. This rule SHALL be present before any byte-exact fixture is committed: an attributes rule added after the fact does not undo a line-ending normalisation that already occurred at `git add` time.
- `rustfmt.toml` — `edition` and `max_width`. Without it a standalone clone falls back to rustfmt's defaults, including edition 2015, and 2024-edition sources fail to parse.
- `.config/nextest.toml` — the `default`, `ci`, `quick` and `full` profiles. In a workspace build nextest reads the workspace root's copy and never the member's, so the member's copy exists solely for the standalone case and for the core repository's own CI.
- `.llvm-cov.toml` — carrying the same exclusions as the parent's, minus any entry that names a binary entry point, which does not exist in a library.
- `.codegraph/.gitignore` — the self-ignoring file, so the core repository can be indexed independently.
- `[lints.rustdoc]`, `[lints.clippy]` and `[lints.rust]` — written out literally in `subx-core/Cargo.toml`. They SHALL NOT be inherited, per the workspace-inheritance prohibition. Their contents SHALL be kept in agreement with `subx-cli`'s by review; a drift between the two blocks is a violation of this requirement.
- `[package.metadata.docs.rs]` — per-package by definition.
- `Cargo.lock` — committed, so a standalone clone and the core repository's own CI resolve reproducibly. Inside the workspace this file is inert, because the workspace root's lockfile is the only one Cargo reads or writes.

All comments in these files SHALL be written in English, per the project's language rule.

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

### Requirement: Submodule Checkout in Every CI Job

Every CI job that loads the Cargo workspace SHALL check the submodule out.

- Every `actions/checkout` step in `.github/workflows/build-test-audit-coverage.yml` (jobs `test`, `archive-rar`, `security`, `coverage`) and in `.github/workflows/release.yml` (jobs `create-release`, `build`, `publish-crates`) SHALL set `submodules: recursive`.
- `recursive` SHALL be used rather than `true`, so the setting remains correct if `subx-core` ever gains a submodule of its own.
- The security-audit job SHALL be included, because dependency auditing resolves against the workspace lockfile and therefore requires the member manifest to exist.
- Jobs that read git tags or generate release notes from `CHANGELOG.md` SHALL additionally set `fetch-depth: 0`, so that tag ancestry and the recursive submodule state used for VCS metadata are complete. A shallow parent and a shallow submodule make Cargo's publish-time dirty check unreliable.
- Jobs that do not read tags SHALL keep the default fetch depth, so PR runs are not slowed for no benefit.

#### Scenario: A checkout without the submodule fails loudly

- **GIVEN** a CI job whose `actions/checkout` step omits `submodules: recursive`
- **WHEN** the job runs any Cargo command
- **THEN** it SHALL fail at workspace-manifest load rather than silently building a partial tree

#### Scenario: Every workflow job checks the submodule out

- **GIVEN** the two workflow files
- **WHEN** their `actions/checkout` steps are enumerated
- **THEN** every one of them SHALL carry `submodules: recursive`

#### Scenario: Release jobs fetch full history

- **GIVEN** the `create-release` and `publish-crates` jobs
- **WHEN** their checkout steps are inspected
- **THEN** both SHALL carry `fetch-depth: 0` in addition to `submodules: recursive`

### Requirement: Contributor Clone and Update Procedure

The submodule requirement SHALL be documented wherever a contributor is told how to obtain or build the source.

- A fresh clone SHALL be documented as `git clone --recurse-submodules https://github.com/jim60105/subx-cli`.
- An existing clone SHALL be repaired with `git submodule update --init --recursive`.
- `git config submodule.recurse true` SHALL be recommended so that subsequent `git pull` and `git checkout` operations carry the submodule working tree forward. It SHALL be documented as a recommendation rather than a guarantee: it is per-clone configuration and it does not apply to `git clone`, which is why `--recurse-submodules` is documented separately.
- The `subx-core` repository's own documentation SHALL state that it is normally consumed as a submodule of `subx-cli` and as a crates.io dependency, so that a reader who arrives there directly understands the relationship.

#### Scenario: Documented clone command yields a buildable tree

- **GIVEN** a machine with no prior checkout
- **WHEN** the documented clone command is run and `cargo build` follows
- **THEN** the build SHALL succeed without any further submodule command

#### Scenario: Existing clone is repairable from the documentation alone

- **GIVEN** a checkout created before the submodule existed, or cloned without `--recurse-submodules`
- **WHEN** the contributor follows the documented repair step
- **THEN** `subx-core/` SHALL be populated at the pinned commit and the build SHALL succeed

### Requirement: The Wiring Is Proven by a Compile-Time Reference

The dependency from `subx-cli` on `subx-core` SHALL be exercised by the code, not merely declared in the manifest.

- `subx-core` SHALL expose `pub const VERSION: &str = env!("CARGO_PKG_VERSION");` from its crate root.
- `subx-cli`'s crate root SHALL reference it through a distinctly named public item whose value is derived from `subx_core::VERSION`, so that `subx-core` is compiled and linked on every build of `subx-cli`.
- That item SHALL NOT shadow, replace or alias `subx_cli::VERSION`, which reports the CLI's own version and SHALL keep doing so once the two crates' version lines diverge.
- The reference SHALL NOT be a dependency that only the manifest knows about. An unreferenced dependency compiles whether or not the declaration is correct, whether or not the version requirement is satisfiable, and whether or not the crate itself builds — so it proves nothing.
- The `subx-cli` test suite SHALL assert that the linked core version is non-empty and that its major component matches the major admitted by the declared caret requirement.
- The reference SHALL NOT change the binary's `--version` output or any other observable CLI surface.

#### Scenario: A missing submodule is caught by the compiler

- **GIVEN** a checkout whose `subx-core/` directory is empty or holds a crate that does not compile
- **WHEN** `cargo build` is run
- **THEN** the build SHALL fail, and it SHALL fail on this change rather than on a later change that moves source files

#### Scenario: The two version constants stay distinct

- **GIVEN** `subx-core` at `1.0.0` and `subx-cli` at a different version
- **WHEN** each crate's version constant is read
- **THEN** each SHALL report its own crate's version, and neither SHALL report the other's

#### Scenario: CLI surface is unchanged by the wiring

- **GIVEN** the compile-time reference in place
- **WHEN** the binary is invoked with `--version`
- **THEN** the output SHALL be identical to the output produced before the reference was added
