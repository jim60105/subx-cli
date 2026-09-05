## ADDED Requirements

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

## MODIFIED Requirements

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
