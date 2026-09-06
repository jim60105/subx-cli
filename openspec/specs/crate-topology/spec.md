# Crate Topology

## Purpose

Govern the repository and crate structure of the two-repository split of SubX:
the `subx-cli` repository owns the CLI crate and mounts the separately clonable
`subx-core` library crate as a git submodule, forming a two-member Cargo
workspace with dual-form path-and-version wiring, configuration duplicated into
the core repository, and CI that checks the submodule out in every workspace
job. The module ownership boundary that allocates every source module to
exactly one of the two crates is governed here as well.

## Requirements

### Requirement: Repository Ownership and Submodule Mount

The SubX codebase SHALL be distributed across exactly two git repositories with a fixed containment relationship.

- `https://github.com/jim60105/subx-cli` SHALL own the `subx-cli` crate — the command-line binary and its library facade.
- `https://github.com/jim60105/subx-core` SHALL own the `subx-core` crate — the reusable library. It SHALL be licensed GPL-3.0-or-later and SHALL carry its own `LICENSE` file containing the full licence text.
- `subx-core` SHALL be mounted inside `subx-cli` as a git submodule at the repository-root-relative path `subx-core/`. It SHALL NOT be vendored, subtree-merged, or copied.
- `.gitmodules` in `subx-cli` SHALL record the submodule's `path`, its `url`, and `branch = main`. The `branch` entry SHALL be present even though it does not cause automatic updates: it defines the referent for "the pointer is behind".
- The submodule pointer SHALL be committed in `subx-cli` whenever the intended `subx-core` commit changes. A `subx-cli` commit that depends on a `subx-core` commit it does not pin is non-conforming.
- Before `cargo package` or `cargo publish` is run in either repository, the submodule pointer SHALL be committed and the submodule working tree SHALL be clean. Cargo checks submodules recursively for uncommitted changes and treats a moved pointer as dirty. `--allow-dirty` SHALL NOT be used to bypass this check, because it records `"dirty": true` in the published archive's `.cargo_vcs_info.json`.
- When source files are relocated from `subx-cli` into `subx-core`, the relocation SHALL preserve their per-file history in `subx-core`. A git rename cannot cross the gitlink, so the relocation SHALL be performed by rewriting the relevant paths out of `subx-cli`'s history and merging that history into `subx-core`, rather than by copying the files into a single import commit. The moved files' history SHALL also remain visible in `subx-cli`, where the code genuinely was.

Which individual source module belongs to which crate is specified by the "Module Ownership Between the Two Crates" requirement; the requirements here specify repository and crate **structure** only.

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

#### Scenario: A relocated file keeps its history

- **GIVEN** a source file that was relocated from `subx-cli` into `subx-core`
- **WHEN** `git log` is run against its path inside the `subx-core` repository
- **THEN** it SHALL show the commits that touched the file before the relocation, not a single import commit

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

### Requirement: Module Ownership Between the Two Crates

Every source module SHALL belong to exactly one crate, and the allocation SHALL follow the library/binary boundary rather than history.

- `subx-core` SHALL own `src/config/**`, `src/core/**`, `src/error.rs` and `src/services/**`. These SHALL live at those exact relative paths inside the `subx-core` repository, so that a path under `subx-core/src/` and its former path under `subx-cli/src/` differ only by the repository root.
- `subx-cli` SHALL own `src/cli/**`, `src/commands/**`, `src/lib.rs` and `src/main.rs`, and SHALL contain no other directory or file under `src/`.
- `subx-cli/src/lib.rs` SHALL contain the `App` type and the back-compatibility re-exports, and no engine, configuration, error or service implementation.
- `App` SHALL remain in `subx-cli`. It parses command-line arguments (`<cli::Cli as clap::Parser>::parse()`), accepts `cli::Commands`, and constructs clap argument structs; `clap` SHALL NOT enter `subx-core`.
- `clap` and `clap_complete` SHALL be `subx-cli`-only dependencies permanently. A module that derives `clap::Args`, `clap::Parser`, `clap::Subcommand` or `clap::ValueEnum` SHALL NOT be placed in `subx-core`.
- The terminal presentation layer — argument structs, table rendering, the process-global output mode, the `Reporter` implementation that writes to a terminal, and the error-presentation extension trait — SHALL remain in `subx-cli`.
- A module SHALL NOT be duplicated across the two crates. Where both crates need the same behaviour, `subx-core` SHALL own it and `subx-cli` SHALL call it.

#### Scenario: The CLI crate holds only CLI code

- **GIVEN** the `subx-cli` repository after the source migration
- **WHEN** the entries directly under `src/` are enumerated
- **THEN** they SHALL be exactly `cli/`, `commands/`, `lib.rs` and `main.rs`

#### Scenario: Relative paths are preserved across the boundary

- **GIVEN** a module that was located at `subx-cli/src/core/matcher/engine.rs` before the migration
- **WHEN** it is located after the migration
- **THEN** it SHALL be at `subx-core/src/core/matcher/engine.rs`, with the same file name and the same parent module chain

#### Scenario: A clap-deriving module cannot be moved into core

- **GIVEN** a proposal to relocate a module that derives a `clap` trait into `subx-core`
- **WHEN** the allocation is reviewed
- **THEN** it SHALL be rejected, and the resolution SHALL be to extract the clap-free logic into `subx-core` and leave the argument struct in `subx-cli`

### Requirement: `subx-core` Never References `subx-cli`

`subx-core` SHALL contain no reference to `subx-cli` of any kind, in code or in documentation.

- No file under `subx-core/src/` SHALL contain a `use`, a path expression, an attribute, or a `#[cfg]` naming `subx_cli` or `crate::cli`.
- No rustdoc comment in `subx-core` SHALL contain an intra-doc link resolving into `subx-cli` — including `` [`crate::cli::…`] `` and `` [`subx_cli::…`] ``. Because there is no dependency edge from `subx-core` to `subx-cli`, such a link is unresolvable rather than merely stale, and `broken_intra_doc_links = "deny"` makes it a build failure with no repair short of deleting the link.
- No doctest in `subx-core` SHALL name a `subx-cli` type. A doctest that can only be written in terms of the binary's types SHALL be deleted rather than adapted.
- `subx-cli` MAY reference `subx-core` freely in code and in intra-doc links, in both directions of its own module tree. The prohibition is one-way by construction.
- Rustdoc prose in `subx-core` MAY mention the `subx-cli` binary by name where a returned string or a documented behaviour is written for that binary's terminal, provided the mention is plain text and not a link.
- The prohibition SHALL be enforced by an automated check rather than by review. That check SHALL resolve the directory it walks from `CARGO_MANIFEST_DIR` and never from the working directory, and SHALL match on the offending token anywhere in a line, comments included.

#### Scenario: An upward code reference fails the boundary check

- **GIVEN** a file under `subx-core/src/` that adds `use subx_cli::cli::output;`
- **WHEN** the boundary check runs
- **THEN** it SHALL fail and name the offending file and line

#### Scenario: An upward doc link fails the build

- **GIVEN** a rustdoc comment in `subx-core` containing `` [`crate::cli::error_ext::SubXErrorExt`] ``
- **WHEN** `cargo doc` is run for `subx-core`
- **THEN** the build SHALL fail on the unresolved intra-doc link, because `broken_intra_doc_links` is `deny` in `subx-core`'s own manifest

#### Scenario: The downward direction stays permitted

- **GIVEN** a module under `subx-cli/src/commands/` that links to `` [`subx_core::core::matcher::MatchEngine`] ``
- **WHEN** `cargo doc` is run for `subx-cli`
- **THEN** the link SHALL resolve, because `subx-cli` depends on `subx-core`

### Requirement: Public API Path Stability for the Library Surface

`subx-core`'s public module paths SHALL be identical to the paths the same items had inside `subx-cli`, with only the crate name changed.

- For every public item the downstream consumers reach, `subx_core::<path>` SHALL name the same item that `subx_cli::<path>` named before the migration. Migrating a consumer SHALL therefore be a substitution of the crate name, with no per-item path edits.
- The module tree SHALL NOT be flattened. `subx_core::core::matcher::…`, `subx_core::core::formats::…`, `subx_core::core::sync::…`, `subx_core::core::translation::…`, `subx_core::core::input::…`, `subx_core::config::…`, `subx_core::error::…` and `subx_core::services::…` SHALL keep their present shape, including the `core::` segment, even though that segment reads redundantly in the crate's own name.
- The reason for the redundancy SHALL be recorded in the crate's own crate-level rustdoc, so that it reads as a deliberate compatibility decision rather than an oversight.
- An item SHALL NOT be reachable by two public paths. In particular, no module SHALL be re-exported at the crate root in addition to its canonical path, because a second path creates ambiguous intra-doc links under `broken_intra_doc_links = "deny"` and renders the module twice in the generated documentation.
- The crate root SHALL additionally expose only the names that were already at `subx-cli`'s crate root for these items: `Config`, `ConfigService`, `EnvironmentProvider`, `ProductionConfigService`, `SystemEnvironmentProvider`, `TestConfigBuilder`, `TestConfigService`, `TestEnvironmentProvider`, the `Result<T>` alias, and `VERSION`.
- Flattening or otherwise reshaping these paths SHALL be treated as a breaking change requiring a major version of `subx-core`.

#### Scenario: A consumer migrates by substituting the crate name

- **GIVEN** a downstream consumer importing library items as `subx_cli::core::matcher::MatchEngine`, `subx_cli::config::ProductionConfigService` and `subx_cli::error::SubXError`
- **WHEN** every occurrence of `subx_cli::` is replaced by `subx_core::` and the dependency is switched
- **THEN** the consumer SHALL compile with no further path changes

#### Scenario: A flattening proposal is rejected

- **GIVEN** a proposal to expose `subx_core::matcher::MatchEngine` in place of `subx_core::core::matcher::MatchEngine`
- **WHEN** it is evaluated against this requirement
- **THEN** it SHALL be rejected for the current major version, and the resolution SHALL NOT be to add the shorter path alongside the longer one

#### Scenario: No second path is introduced for a module

- **GIVEN** a module reachable as `subx_core::core::report`
- **WHEN** a crate-root alias such as `pub use core::report;` is proposed
- **THEN** it SHALL be rejected, because it would give one module two public paths

### Requirement: Crate-Root Items Belong to the Owning Crate

Items whose location is the crate root rather than a module SHALL be accounted for explicitly when a module moves between crates, because a module-level re-export does not reach them.

- A `#[macro_export]` macro SHALL be reachable at the root of the crate that compiles it, regardless of which module declares it. When the declaring module moves to `subx-core`, the macro SHALL become `subx_core::<name>`.
- A module re-export such as `pub use subx_core::config;` SHALL NOT be relied upon to make such macros reachable, because they were never within that module's path.
- Where `subx-cli` keeps a macro reachable at its own root for back-compatibility, it SHALL re-export the complete set declared by the moved module, not a subset. A partially re-exported macro set gives consumers no rule they can infer.
- Crate-level inner attributes — the `#![allow(…)]`, `#![warn(…)]` and `#![deny(…)]` that governed the moved code inside `subx-cli` — SHALL be reproduced in `subx-core`'s crate root. They do not travel with a file, and their absence changes how the moved code is linted without any source having changed.
- Module-level inner attributes travel with their files and SHALL NOT be duplicated at the crate root.
- The two crates' crate-level attribute blocks SHALL be kept in agreement by review, for the same reason and under the same terms as their duplicated `[lints.*]` tables.

#### Scenario: A moved macro is reachable at the new crate root

- **GIVEN** a `#[macro_export]` macro declared in a module that moves into `subx-core`
- **WHEN** a consumer imports it
- **THEN** it SHALL be reachable as `subx_core::<macro_name>` and SHALL NOT be reachable as `subx_core::config::<macro_name>`

#### Scenario: The macro back-compatibility set is complete

- **GIVEN** `subx-cli` re-exporting moved macros at its crate root
- **WHEN** the re-exported names are compared against the moved module's declared macros
- **THEN** every declared macro SHALL be present

#### Scenario: Moved code is linted identically after the move

- **GIVEN** source that passed `cargo clippy -- -D warnings` inside `subx-cli`
- **WHEN** the same source is compiled as part of `subx-core` with no edit
- **THEN** it SHALL still pass, because the crate-level attribute block was reproduced

### Requirement: Feature Flags Are Gated in Core and Forwarded by the CLI

A feature SHALL be declared with its real effect in the crate whose sources it gates, and forwarded from the other crate.

- `archive-rar` SHALL be declared in `subx-core` as the optional-dependency gate that actually enables RAR extraction, and in `subx-cli` as a pass-through that enables the core feature and nothing else.
- `slow-tests` SHALL be declared in `subx-core` as the gate over the long-running tests that live in core, and in `subx-cli` as a pass-through.
- A pass-through feature SHALL remain a real, declared feature of `subx-cli`. `subx-cli` sources and tests may write `#[cfg(feature = "…")]` against it, and a pass-through declaration satisfies both roles: it is a `subx-cli` feature *and* it enables the core feature.
- Both crates' `default` feature SHALL be empty.
- The optional dependency that a feature gates SHALL be declared only in the crate that owns the gate. `subx-cli` SHALL NOT declare an optional dependency it does not itself use in order to mirror a core feature.
- `subx-cli` SHALL NOT rely on a feature being enabled by a sibling crate through Cargo's feature unification. Every feature a `subx-cli` source file requires SHALL be enabled by `subx-cli`'s own dependency declaration.

#### Scenario: Enabling the CLI feature enables the core gate

- **GIVEN** `subx-cli` built with `--features archive-rar`
- **WHEN** the RAR extraction path in `subx-core` is reached
- **THEN** the real implementation SHALL run rather than the disabled-feature stub

#### Scenario: A CLI test can still gate on a pass-through feature

- **GIVEN** a `subx-cli` test annotated `#[cfg(feature = "slow-tests")]`
- **WHEN** `subx-cli` is built with `--features slow-tests`
- **THEN** the test SHALL be compiled, because the pass-through is a declared feature of `subx-cli` and not merely a forwarder

#### Scenario: A feature relied on only through unification is a defect

- **GIVEN** a `subx-cli` source file using an API that requires a dependency feature which only `subx-core`'s declaration enables
- **WHEN** the arrangement is reviewed
- **THEN** it SHALL be treated as a defect, and the resolution SHALL be for `subx-cli` to declare the feature itself

### Requirement: Back-Compatibility Re-Exports in `subx-cli`

`subx-cli` SHALL re-export `subx-core`'s public surface so that consumers written against the pre-split paths keep compiling.

- `subx-cli`'s crate root SHALL re-export the modules `config`, `core`, `error` and `services` from `subx_core`, together with `Config`, the configuration-service types that were already at its root, and a `Result<T>` alias denoting the same type as before.
- `subx_cli::VERSION` SHALL NOT be aliased to `subx_core::VERSION`. Each constant SHALL continue to report its own crate's version, and the constant that reports the linked core version SHALL remain the separately named one introduced when the dependency was first wired. The two version lines are independent and diverge at the release that bumps `subx-cli`, so an alias would make `subx_cli::VERSION` report the wrong crate's version.
- These re-exports SHALL be marked legacy in rustdoc prose: each SHALL state where the item now lives, that the canonical path is the `subx_core` one, and that the alias exists for consumers that have not yet migrated.
- `#[deprecated]` SHALL NOT be used for this purpose. The project's rule is to delete an item and update its call sites; because the consumers are out of tree, the compromise is documentation, not an attribute.
- Code **inside** `subx-cli` SHALL NOT resolve through these re-exports. Every `use` and every path in `src/cli/` and `src/commands/` SHALL name `subx_core::…` directly, so that removing the re-exports later is a deletion rather than a crate-wide rewrite.
- After the migration, a search of `subx-cli/src/` for the re-exported module names prefixed by `crate::` SHALL return only the re-export declarations themselves.
- The re-export surface SHALL be complete with respect to what already compiled against it. Completeness SHALL be demonstrated by the existing test suite continuing to build without import edits, not asserted.

#### Scenario: A pre-split import still resolves

- **GIVEN** a consumer writing `use subx_cli::core::matcher::MatchEngine;`
- **WHEN** it is compiled against the post-migration `subx-cli`
- **THEN** it SHALL resolve to the same type as `subx_core::core::matcher::MatchEngine`

#### Scenario: The two version constants still report different crates

- **GIVEN** `subx-core` and `subx-cli` at different versions
- **WHEN** `subx_cli::VERSION` is read
- **THEN** it SHALL report `subx-cli`'s version, and the core version SHALL be readable only through the separately named constant

#### Scenario: In-crate code does not use the legacy paths

- **GIVEN** the post-migration `subx-cli` sources
- **WHEN** `src/cli/` and `src/commands/` are searched for `crate::core`, `crate::config`, `crate::error` and `crate::services`
- **THEN** no match SHALL be found outside `src/lib.rs`'s re-export declarations

#### Scenario: Removing a re-export is a one-line change

- **GIVEN** the day the last out-of-tree consumer has migrated
- **WHEN** a re-export is deleted from `subx-cli`'s crate root
- **THEN** no other file in `subx-cli` SHALL need editing

### Requirement: Dependency Allocation Across the Two Manifests

Each crate's manifest SHALL declare the dependencies its own sources use, and only those.

- A dependency SHALL be declared in every crate that has at least one use site for it, and in no crate that has none. Two crates declaring the same registry crate is normal and SHALL NOT be avoided by relying on a re-export or on feature unification.
- The feature list on a shared dependency SHALL be derived from each crate's own use sites independently. A crate SHALL NOT omit a feature on the grounds that the other crate enables it.
- A dependency used only by `#[cfg(test)]` modules within a crate's own `src/**` SHALL be declared in that crate's `[dev-dependencies]`.
- A dependency SHALL NOT be declared in anticipation of a later change. A manifest entry with no use site at the commit that introduces it is a defect, whether or not a future change would have given it one.
- The conversions declared on the library's error type determine which crate owns the crates those conversions name. Because the error type lives in `subx-core`, every crate appearing in a `From` implementation for it SHALL be a `subx-core` dependency.
- `[profile.release]` and `[profile.dev]` SHALL remain declared only in the workspace root manifest, unchanged by the migration.
- `[[bin]]` and `[[bench]]` targets SHALL remain declared in `subx-cli` while their sources remain in `subx-cli`.

#### Scenario: A shared dependency is declared twice

- **GIVEN** a registry crate used by production code in both `subx-core` and `subx-cli`
- **WHEN** the two manifests are inspected
- **THEN** both SHALL declare it, each with the feature list its own use sites require

#### Scenario: An anticipatory declaration is rejected

- **GIVEN** a dependency added to `subx-core`'s `[dev-dependencies]` because a later change will move tests that need it
- **WHEN** the manifest is reviewed at that commit
- **THEN** the entry SHALL be removed, and it SHALL be added by the change that brings its use sites

#### Scenario: A missing feature is not masked by unification

- **GIVEN** `subx-cli` source using an API behind a dependency feature that only `subx-core`'s declaration enables
- **WHEN** `subx-cli` is built
- **THEN** the arrangement SHALL be corrected by adding the feature to `subx-cli`'s own declaration, rather than left to resolve through the shared build graph

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
