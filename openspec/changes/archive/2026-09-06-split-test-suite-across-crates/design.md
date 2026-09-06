## Context

Six changes have run. A0 pruned five phantom dependencies and moved `hound` into `[dev-dependencies]`. A1 cut the thirteen `core`/`services` → `crate::cli` edges behind the `core::report::Reporter` seam and added `tests/core_cli_boundary.rs`. A2 relocated `input_handler`, the sync pairing logic and `create_default_output_path` into `core`, moved `exit_code`/`user_friendly_message` into `subx_cli::cli::error_ext::SubXErrorExt`, and explicitly left `tests/cli/input_handler_tests.rs` to B3. B1 created the `subx-core` repository, mounted it as a submodule, made `subx-cli` the workspace root, and shipped `.gitattributes` with the `tests/fixtures/formats/** -text` rule three changes before the fixtures it protects arrive. B2 moved 95 files and 36,687 LOC into `subx-core`, split the manifests, and proved that the D11 re-export surface lets the entire 136-file test suite keep compiling without a single edit.

So B3 inherits a green but architecturally inverted suite: 23,006 LOC of tests, all of them in `subx-cli`, most of them exercising code that now lives in another repository, reaching it through an alias that exists for the benefit of an out-of-tree GUI.

The implementation plan (§6) names `tests/common/` as the hazard that can inflate B3 from one day to three, and SDR §6 prescribes the remedy: core-facing helpers become a `subx-core` module behind a `test-support` feature; the "`assert_cmd`-based `CLITestHelper`" stays CLI-side. The instruction for this change was to verify that prescription against the code before adopting it. It was verified, and the verification changed the shape of the change in six ways that the proposal enumerates. Two of them matter enough to restate here, because every decision below rests on them:

- **`subx_cli::cli::` and `subx_cli::commands::` are `subx_cli::` too.** SDR §6's 89-file core-bound set was derived by grepping for `use subx_cli::`. Thirty-nine of those files build clap `*Args` structs and call `commands::*_command::execute`. They cannot move to a crate that SDR D8 forbids clap from ever entering. The real core-bound set is 42 files, 6,826 LOC — less than a third of the suite.
- **`CLITestHelper` is not `assert_cmd`-based.** It contains no reference to `assert_cmd`. It is a `TempDir` + `TestConfigService` fixture builder, plus one `impl` block that shells out to `cargo run --` from `env!("CARGO_MANIFEST_DIR")`. The seam SDR §6 draws around the file actually runs through it.

## Goals / Non-Goals

**Goals:**

- Give each crate the tests that exercise its own code, decided by a rule a future contributor can apply without asking.
- Give the shared helpers a home that works across a repository boundary Cargo has no mechanism for, without duplicating a line and without putting `wiremock` into every consumer's build.
- Leave `subx-core` independently testable: a standalone clone runs its own integration tests, benches, fixtures and assets with no reference to `subx-cli`.
- Remove the last in-repository consumer of the D11 back-compatibility re-exports, so their eventual deletion is a deletion.
- Repair the defects found while doing the above — the twelve never-compiled files, the unresolvable binary name, the two working-directory-relative paths B2 broke, and 493 LOC of helpers nothing references.
- Leave the combined coverage figure unmoved, since no production line changes, and document the `default-members` hazard the move creates.

**Non-Goals:**

- Triaging the twelve never-compiled test files, converting the 26 runtime binary-name lookups, or measuring per-crate coverage. All three are `harden-split-test-suite`'s (B4), for the reasons in Decision 7.
- Touching `scripts/check_coverage.sh`, `scripts/quality_check.sh`, the CI environment, `default-members`, or the two junit paths. **C1 implements**; B1 established this division and B2 honoured it.
- Replacing `CLITestHelper`'s `cargo run --` spawning with `assert_cmd`. It is a real defect (a nested Cargo invocation inside a `cargo nextest run` contends on the build lock) but it is a behaviour change to eight test files, and B3 is already over budget. Recorded in Open Questions.
- Deleting the D11 re-exports or the twelve macro re-exports. They exist for the out-of-tree GUI, which has not migrated yet.
- Touching `openspec/specs/**`. C2a and C2b own capability migration.
- Bumping `subx-cli` to 2.0.0. That is C1's release event.

## Decisions

### Decision 1: Test ownership is decided by what a test *drives*, not by what it *imports*

SDR §6 classifies with one predicate (`use subx_cli::…` → core) and one exception (`assert_cmd` → CLI). Applied to the tree, that predicate mis-files 47 of 123 non-helper files. The rule this change adopts is a four-way classification, evaluated in order, first match wins:

1. **The test spawns the binary** → `subx-cli`, regardless of what it imports. Two mechanisms qualify: `assert_cmd::Command` (17 files) and `CLITestHelper::run_command_*`, which spawns `cargo run --` (8 files). A test that spawns `subx-cli` is testing `subx-cli`.
2. **The test names `subx_cli::cli` or `subx_cli::commands`** → `subx-cli`. Those modules stay by SDR D7/D8; a test of them cannot live in a crate that cannot see them (39 files).
3. **The test names only `config`, `core`, `error`, `services` or `Result`** → `subx-core` (42 files).
4. **The file is a harness shim** → it follows its target.

The ordering is load-bearing. `tests/cli/output_format_config.rs` and `tests/cli/output_format_cross_command.rs` are SDR's two known mixed-mode files: both spawn the binary *and* name library internals (`subx_cli::config::Config` in a comment, and `use subx_cli::core::matcher::FileDiscovery;` respectively). Rule 1 fires first, they stay in `subx-cli`, and the library import is rewritten to `subx_core::core::matcher::FileDiscovery` in place. `tests/cli/output_format_sync.rs` is a third mixed-mode file SDR did not identify — it spawns via `CLITestHelper` and carries a `subx_cli::commands::sync_command::SyncPayload` doc link. Same resolution.

One file crosses the line because of A2. `tests/archive_input_extraction_tests.rs:16` is `use subx_cli::cli::InputPathHandler;` and nothing else; once A2 has moved `InputPathHandler` to `subx_core::core::input`, rule 2 stops matching and rule 3 applies. Final counts: **43 core-bound, 80 CLI-bound.** `tests/unified_path_handling_tests.rs` looks like the same case but is not — it also imports `ConvertArgs`, `MatchArgs`, `SyncArgs`, `DetectEncodingArgs`, `OutputSubtitleFormat` and `SyncMethodArg`, so rule 2 still fires and it stays.

**Why not move the CLI-library tests to core anyway, by testing the engines directly instead of the commands?** Because that is 8,605 LOC of test rewriting disguised as a file move, and because the coverage those tests provide for `src/commands/**` — 6,283 LOC that stay in `subx-cli` — would evaporate. It is also not obviously desirable: `match_command::execute` composes discovery, AI matching, conflict resolution and file relocation, and testing that composition is the point.

### Decision 2: `subx-core` gains a `test-support` feature — and here is the module-by-module verdict on SDR §6

SDR §6's prescription is adopted in its shape and corrected in its contents. The correction is not editorial: measured by consumer class, **five of the thirteen `tests/common/` modules need no home at all**, and the one SDR names as the CLI-side anchor has to be cut in half.

| Module | LOC | Core consumers | CLI consumers | SDR §6 says | Verdict |
|---|---|---|---|---|---|
| `mod.rs` | 12 | — | — | — | Rewritten on both sides; it is a `pub mod` list, not code |
| `cli_helpers.rs` | 503 | 2 | 9 | stays CLI-side entire | **Split through the middle** (Decision 3) |
| `file_managers.rs` | 259 | 4 | 0 | core-facing | `test_support::file_managers` |
| `mock_openai_helper.rs` | 324 | 1 | 11 | core-facing | `test_support::mock_openai` |
| `mock_azure_openai_helper.rs` | 404 | 4 | 0 | core-facing | `test_support::mock_azure_openai` |
| `mock_generators.rs` | 467 | 1 | 1 | core-facing | `test_support::mock_generators` |
| `test_data_generators.rs` | 75 | 5 | 8 | core-facing | `test_support::responses` |
| `json_output.rs` | 133 | 0 | 4 | core-facing | **CLI-side.** Asserts the `--output json` stdout envelope of a spawned process; it is `machine-readable-output`'s test surface and has no core meaning |
| `validators.rs` | 274 | 0 | 0 | core-facing | **Deleted.** Zero consumers, and its `OutputValidator`/`ValidationResult` duplicate the copies in `cli_helpers.rs` that *are* used |
| `parallel_helpers.rs` | 97 | 0 | 0 | core-facing | **Deleted.** Zero consumers |
| `sync_helpers.rs` | 78 | 0 | 0 | core-facing | **Deleted.** Zero consumers |
| `integration_test_macros.rs` | 41 | 0 | 0 | core-facing | **Deleted.** Zero consumers; its two `macro_rules!` are never `#[macro_use]`d |
| `command_helpers.rs` | 3 | 0 | 4 | core-facing | **Deleted.** Three lines of `//!` and no items; the four files that import three functions from it are among the twelve that have never compiled |

That is **493 LOC deleted, 1,529 LOC to `test_support`, 133 LOC staying CLI-side, and one 503-LOC file cut in two.** The cross-boundary set is not a stylistic preference — it is structural and irreducible: the AI-mocking helpers (`mock_openai`, `responses`, `mock_generators`) are needed by core-level provider tests *and* by CLI-level `match_command` tests, because both drive the same `AIProvider` trait through the same wiremock server.

**Is a different split better?** Two were considered and rejected on evidence.

*Split by "does more than one crate need it", leaving core-only helpers in `subx-core/tests/common/`.* This would keep `file_managers` (259) and `mock_azure_openai` (404) out of the feature — 663 LOC of ordinary Cargo-native test module instead of published-crate surface. It is genuinely tempting. It is rejected because it creates **two mechanisms** for the same thing in the same crate: `subx-core`'s own tests would reach some helpers as `subx_core::test_support::…` and others as `mod common;`, with the boundary determined by a fact (how many crates use it today) that changes whenever a test is added. The next contributor who needs `TestFileManager` from `subx-cli` would have to move it between mechanisms. One mechanism, applied to everything shared, is worth 663 LOC of gated surface.

*Relocate the outlier tests so the overlap collapses.* `mock_openai_helper` has exactly one core consumer (`tests/wiremock_basic_integration.rs`, 977 bytes) and `mock_generators` has exactly one on each side. Moving three files would shrink the shared set to `responses.rs` (75 LOC) and the `TestWorkspace` half of `cli_helpers.rs`. Rejected: it files tests by where their helpers live rather than by what they test, which is precisely the inversion Decision 1 exists to end, and it does not eliminate the mechanism — 75 LOC still has to cross the boundary, so the feature is built either way.

### Decision 2a: the mechanism is verified, not assumed — including the two parts that look like they should not work

Two Cargo behaviours carry this decision, and both were verified empirically in a scratch workspace shaped exactly like the target (edition 2024, `resolver = "3"`, a root package plus a member, the member declared in both `[dependencies]` and `[dev-dependencies]`):

**`subx-core` reaches its own gated module through a path-only self dev-dependency.**

```toml
# subx-core/Cargo.toml
[dev-dependencies]
subx-core = { path = ".", features = ["test-support"] }
```

An integration test under `subx-core/tests/` links the library as an *external* crate compiled without `cfg(test)`, so `#[cfg(any(test, feature = "test-support"))]` would not reach it and `--features test-support` on every invocation is not a contract, it is a convention people forget. The self dev-dependency is the documented workaround, and it works: `cargo test` compiled `subx_core::test_support::helper()` and ran it. It is also invisible to consumers — `cargo package` emits an **empty** `[dev-dependencies]` table, because Cargo strips path-only dev-dependencies when normalising the published manifest.

**`subx-cli` enables the feature for tests without leaking it into release builds.**

```toml
# subx-cli/Cargo.toml
[dependencies]
subx-core = { path = "subx-core", version = "1.0" }

[dev-dependencies]
subx-core = { path = "subx-core", version = "1.0", features = ["test-support"] }
```

Under resolver 2/3 a dev-dependency's features are not unified into a build that has no dev target. Verified by counting `--cfg feature="test-support"` in `cargo build --release -v` (**0** occurrences) versus `cargo test --no-run -v` (present). This is the exact hazard B2 Decision 5 cites as its reason for *not* gating `TestConfigService` — there, the consumer is the out-of-tree GUI, whose `[dev-dependencies]` really would enable the feature for its own normal dependency in test builds. Here the consumer is `subx-cli`, which does not ship a library that anyone links, so the same mechanism is benign.

**What `test-support` adds beyond D10.** SDR D10 already makes `TestConfigService`, `TestConfigBuilder` and `TestEnvironmentProvider` unconditional public API, and B2 Decision 5 records why. `test-support` therefore adds **nothing at the configuration layer** — `TestWorkspace` constructs a `TestConfigService` through the same ungated path any test uses. What it adds is the layer above: HTTP mock servers (`MockOpenAITestHelper`, `MockAzureOpenAITestHelper`), canned AI responses (`MatchResponseGenerator`), generated media and subtitle fixtures (`AudioMockGenerator`, `SubtitleGenerator`), a temp-directory file manager (`TestFileManager`), and the workspace builder. The distinction that justifies the gate is dependency weight, not test-ness: the D10 types need nothing that `subx-core` does not already depend on, whereas the mock helpers need `wiremock` (a full hyper-based HTTP server) and `hound`. Those two become **optional** dependencies activated by the feature — `wiremock = { version = "0.6", optional = true }`, `hound = { version = "3.5", optional = true }`, `test-support = ["dep:wiremock", "dep:hound"]` — which is the whole reason the gate is worth its complexity.

`pub mod test_support` carries `#[allow(missing_docs)]`. `subx-core`'s crate root declares `#![warn(missing_docs)]` (B2 Decision 3) and the project runs `cargo clippy -- -D warnings`, which would promote every undocumented `pub` item in 1,929 LOC of relocated scaffolding into a build failure. Writing `# Arguments` / `# Returns` / `# Errors` / `# Examples` for mock helpers would consume most of a day and dilute the contract the lint exists to protect. The module keeps its `//!` header explaining the gate; `broken_intra_doc_links = "deny"` still applies to everything inside it.

### Decision 2b: not a separate dev-only helper crate, despite it being the better engineering

The alternative shape is a third package — `subx-core/testkit/`, `publish = false`, depended on by both crates through path-only `[dev-dependencies]`. It is cleaner on every axis that matters technically: no feature gate, no unification rules to reason about, no self-dependency, `wiremock` never appears in `subx-core`'s manifest at all, and zero semver surface on a published crate. The dev-dependency cycle it creates (`subx-core` → `testkit` → `subx-core`) is explicitly permitted by Cargo, unlike a direct self-dependency.

**It is rejected because it is not this change's to make.** It requires editing B1's `Cargo Workspace Shape` requirement, which normatively fixes `members = [".", "subx-core"]`; it adds a third package to `cargo publish --workspace`, which is C1's flow and which would need `publish = false` handled; it adds a third manifest to SDR §5's duplicated-configuration list; and it gives the `subx-core` repository a second package, which C1's core-repo CI and C3's documentation both have to account for. That is a change to three other people's lanes to save one feature flag whose two tricky behaviours have already been verified to work. SDR D3's prohibition on workspace inheritance is not directly violated — `testkit` would carry literal values — but the arrangement runs against the same grain: a standalone `subx-core` clone would contain a package that only exists to serve a repository it cannot see.

If the feature gate proves painful in practice, this is the migration to make, and it is a clean isolated change at any later point.

### Decision 3: `CLITestHelper` splits through the middle into `TestWorkspace` + a CLI-side `CliRun` extension trait

`tests/common/cli_helpers.rs` is 503 LOC in two halves that share only a struct name:

- **The fixture builder** (`new`, `with_config_service`, `with_ai_settings`, `with_sync_settings`, `temp_dir_path`, `config_service`, `create_isolated_test_workspace`, `create_media_files`, `create_subtitle_files`, `create_config_file`, `create_subtitle_file`, `create_video_file`, `test_files`, `cleanup`, `Default`, `Drop`, plus `OutputValidator` and `ValidationResult`). It imports `subx_cli::Result` and `subx_cli::config::{ConfigService, TestConfigService}` — all D10 public API — plus `tempfile` and `tokio::fs`. Nothing about it is CLI-specific.
- **The process-spawning `impl` block** (`run_command_with_config`, `run_command_expect_success`, `assert_command_success`, `assert_command_failure`, `CommandResult`). It runs `Command::new("cargo").arg("run").arg("--")` with `.current_dir(env!("CARGO_MANIFEST_DIR"))`.

`env!("CARGO_MANIFEST_DIR")` is the decisive fact. It expands at compile time to the manifest directory of *the crate being compiled*. If this block moved to `subx-core`, it would expand to `…/subx-core` and `cargo run --` would fail — `subx-core` is a library with no binary. The half that must stay CLI-side is identified by that macro, not by `assert_cmd` (which the file never uses) and not by the type's name.

**Resolution.** The fixture builder becomes `subx_core::test_support::workspace::TestWorkspace`, and `subx-cli/tests/common/cli_helpers.rs` retains `CommandResult` plus:

```rust
pub trait CliRun {
    fn run_command_with_config(&self, args: &[&str]) -> impl Future<Output = CommandResult>;
    fn run_command_expect_success(&self, args: &[&str]) -> impl Future<Output = CommandResult>;
    fn assert_command_success(&self, result: &CommandResult);
    fn assert_command_failure(&self, result: &CommandResult);
}
impl CliRun for TestWorkspace { /* the block, verbatim */ }
```

Nine CLI-side files add `use common::cli_helpers::CliRun;`; two core-side files (`tests/dependency_injection_integration_tests.rs`, `tests/vad_detector_tests.rs`) do not, because they only ever call fixture methods.

**Why rename `CLITestHelper` → `TestWorkspace`.** Eleven call sites is a cheap edit, and a type called `CLITestHelper` exported from a crate that contains no CLI is the kind of name that makes the next reader distrust the whole split. The name also stops being accurate the moment the spawning half leaves. `subx-cli`'s copies of `OutputValidator` and `ValidationResult` travel with the fixture builder; `validators.rs`'s unreferenced duplicates are deleted, which resolves a duplication that has existed since before this series.

### Decision 4: `mod common;` at crate roots; `#[path]` reserved for harness shims

Two inclusion mechanisms exist today and they are not equivalent in *purpose*, only in effect at a crate root:

- `mod common;` — used by 40 top-level test files. For a crate root at `tests/foo.rs`, rustc resolves `common` against `tests/`, finding `tests/common/mod.rs`. Correct, attribute-free, and what a Rust reader expects.
- `#[path = "common/mod.rs"] mod common;` — used by six harness shims. At a crate root it resolves identically, so on those six files it is **redundant**. Its one non-redundant use is selective inclusion: `tests/config_service_file_integration_tests.rs:3` writes `#[path = "common/file_managers.rs"] mod file_managers;` to pull in one helper without compiling the other eleven and collecting `dead_code` warnings for them.

`#[path]` is genuinely required in exactly one place: pointing a top-level crate root at a file in a subdirectory Cargo does not auto-discover. `tests/cli/`, `tests/commands/`, `tests/parallel/` and `tests/sync/` are not scanned; only `tests/*.rs` is.

**Standardisation.** Moved files use plain `mod common;`. `#[path]` survives only in harness shims (`#[path = "cli/<file>.rs"] mod <file>;`) and in the one selective-inclusion site, which keeps its form and gains a comment saying why. The six redundant `#[path = "common/mod.rs"]` lines in shims become `mod common;`.

**Consequences for the moved files.** `subx-core/tests/` ends up flat — all 40 files B3 moves sit at `tests/*.rs` — so **`subx-core` needs no harness shims at all**, and B4 keeps it that way by flattening the three core-bound orphans rather than shimming them. `subx-cli` keeps `tests/cli/` and its shims (15 today, 17 after A1). `tests/commands/`, `tests/parallel/` and `tests/sync/` still exist when B3 lands, holding only never-compiled files; B4 empties and deletes all three.

The rule that prevents this class of defect recurring is normative in the spec: a file under a non-auto-discovered directory with no shim pointing at it is a defect, and the check is a directory listing compared against the set of `#[path]` targets.

### Decision 5: media assets move to `subx-core`, the 3.4 KB subtitle is duplicated, and every reader resolves from `CARGO_MANIFEST_DIR`

`assets/` holds four files: `SubX - The Subtitle Revolution.{srt,mp4,mp3}` (3.4 KB / 7.1 MB / 4.2 MB) and `logo.svg`. Three groups read them:

1. **Six binary-driving `sync_*` tests** (`sync_batch_new_logic`, `sync_batch_processing`, `sync_batch_subtitle_only_skip`, `sync_comprehensive`, `sync_input_path_handling`, `sync_parameter_combinations`) use `PathBuf::from("assets/SubX - The Subtitle Revolution.{srt,mp4}")`. They stay in `subx-cli`.
2. **One core-bound test** (`tests/sync_first_sentence_offset_integration_tests.rs:21-22`) reads the `.mp3` and `.srt`. It moves to `subx-core`.
3. **One in-`src` unit test** (`src/services/vad/audio_loader.rs:33`) reads the `.mp4`. B2 already moved it to `subx-core`, where the path no longer resolves — Cargo sets a test binary's working directory to the *package* root, so `assets/…` now means `subx-core/assets/…`, which does not exist.

Group 3 makes the decision for us: `subx-core` must have the media, or a test B2 moved stays broken. Group 1 makes duplication unavoidable in one direction or another, because `subx-cli` cannot reach `subx-core/assets/` without either a `../` traversal into a submodule (fragile, and meaningless in a published crate) or an environment variable set by a script (which C1 owns and which would make `cargo nextest run` alone insufficient).

**Resolution:**

- The `.mp4` and `.mp3` (11.3 MB, the expensive ones) **move** to `subx-core/assets/`. Only groups 2 and 3 read them, and both are core-bound.
- The `.srt` (3.4 KB) is **copied**: `subx-core/assets/` gets it for group 2, and `subx-cli/assets/` keeps it for group 1. Duplicating 3.4 KB of static text is cheaper than any mechanism that avoids it, and the file has not changed in the repository's history.
- `logo.svg` stays in `subx-cli`. It is a README asset with no test consumer.
- **Every reader resolves from `CARGO_MANIFEST_DIR`, never from the working directory.** Both a bare `PathBuf::from("assets/…")` and a `std::env::current_dir()` join are prohibited. This is not merely tidier: `cargo nextest run` and `cargo test` set the CWD to the package root, but `cargo llvm-cov`, an IDE test runner and a nested `cargo run` do not all agree, and the failure mode is a file-not-found in a test that reads a 7 MB video.

Both `subx-cli/Cargo.toml` and `subx-core/Cargo.toml` already exclude `assets/` and `tests/` from the published archive, so none of this reaches crates.io. `subx-core/.gitattributes` needs no new rule for the media — they are binary and Git will not normalise them — but the `.srt` is text and its line endings are not asserted byte-exactly by any test, so it needs none either.

**Alternative considered — move the assets and have `subx-cli`'s tests resolve `CARGO_MANIFEST_DIR/../subx-core/assets/`.** Rejected. It hard-codes the submodule mount path into eight test files, breaks if anyone ever consumes `subx-cli`'s tests outside the superproject, and gains 3.4 KB.

**Alternative considered — move the six group-1 tests to core as well.** Rejected by Decision 1: they spawn the binary.

### Decision 6: `tests/fixtures/formats/**` moves whole, and the rule that protects it is already in place

22 files across `srt/`, `ass/`, `vtt/` and `sub/`, each an input plus a `.expected` companion, three of which per format deliberately carry CRLF line endings or a UTF-8 BOM. `.gitattributes` protects them with `tests/fixtures/formats/** -text`.

Both consumers already resolve correctly and neither needs an edit to its *mechanism*:

- `src/core/formats/tests_support.rs:63-70` builds `CARGO_MANIFEST_DIR/tests/fixtures/formats/<rel>`. It is in `subx-core` since B2, and it is broken there today — invisibly, because `#[cfg(all(test, feature = "slow-tests"))]` (`src/core/formats/mod.rs:149`) keeps it out of the default run. Moving the fixtures fixes it.
- `tests/format_roundtrip_tests.rs:31-40` does the same and is core-bound, so it moves with them.

The one thing that could go wrong is a normalisation at `git add` time, which is not reversible by adding the rule afterwards. **B1 already shipped `.gitattributes` into `subx-core`'s initial commit precisely for this** (B1 Decision 5). B3's obligation is therefore a verification, not an authorship: confirm the rule is present *before* the first `git add` of a fixture, and confirm afterwards with `git diff --stat` on a fresh checkout that no byte moved. The task list checks the three CRLF fixtures and the three BOM fixtures explicitly, by size, on both sides of the move.

### Decision 7: three bodies of work are handed to B4, and the seam is chosen so neither change can break the other

The census that produced Decision 1's classification also produced three findings that are defects rather than relocations, and together they are ~5.5 h of the ~14.5 h this change measured. Carrying them would make B3 a two-day change wearing a one-day label. They are lifted into `harden-split-test-suite` (B4), which runs between B3 and C1:

| Handed to B4 | Size | Why it is separable from the move |
|---|---|---|
| Triage of the twelve never-compiled files — revive eight, delete four | 1,104 LOC | They are compiled by nothing, so B3's build is byte-identical whether they are present or absent |
| The 26 `Command::cargo_bin(…)` → `env!("CARGO_BIN_EXE_subx-cli")` rewrites | 17 files | Every one of those files stays in `subx-cli` unmoved; B3 edits their library imports, B4 edits their spawn calls, in different parts of each file |
| Per-crate coverage measurement and floor derivation | — | The floors cannot be derived before the suite reaches its final composition, and B4's eight revivals change it |

**The one place the two changes interlock, and how it is resolved.** Three of the twelve orphans are core-bound: `tests/cli/input_handler_tests.rs`, `tests/parallel/integration_tests.rs` and `tests/sync/integration_tests.rs`. Flattening a file into `subx-core/tests/*.rs` *is* reviving it, because Cargo auto-discovers everything there — so B3 cannot move them without doing B4's work, and cannot move them unflattened without importing dead code into a fresh repository and failing its own `grep -rn 'subx_cli' subx-core/tests/` check. **B3 therefore moves the 40 live core-bound files and leaves those three; B4 moves and revives them together.** The ownership census stays 43/80 — that is the classification, and SDR §6 records it — and only the physical move is split 40/3. Both proposals say so, so neither reads as if it moved 43.

**Three consequences inside B3.**

1. **B3 does not add the orphan-file check.** Twelve files are still unwired when it lands, so the assertion would ship red. The check belongs on `tests/core_cli_boundary.rs` alongside B3's own re-export guard, and B4 adds it there after the triage. B3's `cross-crate-testing` delta correspondingly states the harness-shim *mechanism* and stops short of claiming orphan-freedom; B4 restates that requirement in full with the enforcement half attached.
2. **B3 still deletes `tests/common/command_helpers.rs`.** It is a `tests/common/` module and `tests/common/` is B3's to split; `mod.rs` must lose the declaration in any case, and leaving the file behind would create precisely the undeclared orphan A0 deleted from `src/cli/validation.rs`. Its four dead consumers under `tests/commands/` are triage, and go with B4. The sequence reads correctly in the history: the helper goes when its directory is reorganised, the tests go when the never-compiled class is triaged.
3. **B3 documents the `default-members` hazard even though B4 owns the coverage contract.** With `default-members` unset, a bare `cargo nextest run` acts on the root package alone, and the moment B3's move lands that silently skips 43 test files. The hazard is created here, so the `--workspace` warning is documented here; the normative requirement and the numbers are B4's, and the script and CI plumbing stays C1's. The overlap is deliberate and is noted in both changes.

### Decision 8: the D11 re-exports survive B3, and lose their last in-repository consumer

B2's `crate-topology` delta already requires that code inside `subx-cli/src/` never resolve through the D11 re-exports, and demonstrates completeness by the test suite continuing to build. That leaves a gap: the test suite itself resolves through them, in 89 files.

B3 closes it in both directions. The 43 moving files become `subx_core::` because they must. The 80 staying files are rewritten too, wherever they name `config`, `core`, `error`, `services` or `Result` — even though those imports would keep resolving. The one macro consumer, `tests/config_set_integration_tests.rs:4`'s `use subx_cli::test_with_config;`, is core-bound and becomes `use subx_core::test_with_config;`.

**The re-exports are not deleted.** They exist for the out-of-tree Tauri GUI, which the implementation plan (§7) says migrates in an independent PR that is not gated on B3. Deleting them here would break a consumer that has not been given the chance to move, and AGENTS.md's "delete the item and update all call sites" rule cannot be honoured when the call sites are in another repository. What B3 changes is their *status*: after this change they have zero in-repository consumers, so their eventual deletion is a deletion rather than a crate-wide rewrite — which is exactly the property B2's spec asked for and could not yet deliver.

Enforcement is a widened guard: A1's `tests/core_cli_boundary.rs`, already repointed by B2 to walk `subx-core/src/`, gains an assertion that no file under `subx-cli/tests/` or `subx-cli/benches/` contains `subx_cli::config`, `subx_cli::core`, `subx_cli::error`, `subx_cli::services`, `subx_cli::Result`, or any of the twelve re-exported macro names. It resolves both directories from `CARGO_MANIFEST_DIR`, matching the mechanism B2's spec already requires of it.

### Decision 9: both crates keep a real `slow-tests` gate; core owns the sources, and B2's pass-through arrangement is unchanged

After the split, `#[cfg(feature = "slow-tests")]` sites land on both sides:

- **`subx-core`:** five sites in `src/core/formats/{srt,ass,vtt,sub}/tests.rs` and `src/core/formats/mod.rs:149`, plus two moving test files — `sync_first_sentence_offset_integration_tests.rs:3` and `vad_detector_tests.rs:20,52,90`.
- **`subx-cli`:** three staying test files — `sync_comprehensive_integration_tests.rs` (2 sites), `sync_parameter_combinations_tests.rs` (7), `sync_batch_subtitle_only_skip_tests.rs` (1).

B2 already declared `slow-tests = ["subx-core/slow-tests"]` in `subx-cli` and `slow-tests = []` in `subx-core`, and its spec records that a pass-through is simultaneously a real `subx-cli` feature and a forwarder. **B3 changes nothing about that** — it only removes the reason B2 gave for the CLI side needing it ("the four `tests/sync_*` files ... until B3 moves them") and replaces it with a permanent one: three of those files never move. `scripts/quality_check.sh:223,326` and `scripts/check_coverage.sh:358` pass `--features slow-tests` at the workspace root, which forwards to core through the pass-through, so no script changes.

`test-support` is deliberately *not* forwarded as a `subx-cli` feature. It is enabled through the `[dev-dependencies]` declaration only, so it can never be turned on by `cargo build --features …` and cannot reach a release binary.

### Decision 10: dev-dependencies are re-derived from the post-split composition, and three of them are deleted rather than moved

SDR §4 says `subx-core`'s `[dev-dependencies]` are `hound`, `mockall`, `wiremock`, `rstest`, `test-case`, `pretty_assertions`, `tokio-test` and `criterion`, with B2 taking `mockall` and `wiremock` and B3 taking the rest. A use-site grep over `src/`, `tests/` and `benches/` finds:

| Crate | Use sites | Allocation |
|---|---|---|
| `criterion` | 2 (both benches) | `subx-core` dev-dep; delete from `subx-cli` |
| `pretty_assertions` | 1 (`format_roundtrip_tests.rs`) | `subx-core` dev-dep; delete from `subx-cli` |
| `hound` | 5 (all core-bound tests) | `subx-core` — **optional** dep behind `test-support` *and* dev-dep; delete from `subx-cli` |
| `wiremock` | 6 in `subx-core/src`, 2 helpers, 4 CLI-side tests | `subx-core` optional dep behind `test-support` *and* dev-dep; **also** `subx-cli` dev-dep |
| `mockall` | 2 (in `subx-core/src`) | already `subx-core` dev-dep (B2); delete from `subx-cli` |
| `assert_cmd` | 17 (all CLI-side) | `subx-cli` only |
| `predicates` | 3 (all CLI-side) | `subx-cli` only |
| `regex` | 2 CLI-side tests + `TestWorkspace`'s `OutputValidator` | `subx-cli` dev-dep; already a `subx-core` normal dep |
| `tokio-test` | **0** anywhere | **deleted** |
| `rstest` | **0** anywhere | **deleted** |
| `test-case` | **0** anywhere | **deleted** |

The last three are the interesting ones. Moving them to `subx-core` as SDR §4 instructs would plant three entries with no use site, which B2's own `crate-topology` requirement calls a defect in as many words ("A manifest entry with no use site at the commit that introduces it is a defect, whether or not a future change would have given it one") and which A0 established as a `supply-chain-hardening` violation. They are deleted from `subx-cli` and never declared in `subx-core`.

`wiremock` and `hound` appear twice in `subx-core`'s manifest — once as `optional = true` under `[dependencies]` so the feature can activate them for `src/test_support/`, and once under `[dev-dependencies]` so core's own tests can name them directly. That is the ordinary Cargo shape for a dependency used both behind a feature and by tests, not a duplication.

## Risks / Trade-offs

- **Risk: the 42/81 reclassification is wrong somewhere, and a file that names `subx_cli::commands` is moved to a crate that cannot see it.** → Mitigation: the classification is mechanical and re-runnable — first path segment after `subx_cli::` over every file — and the task list re-runs it as a positive check after the move rather than trusting the pre-move census. The compiler is the backstop: a mis-filed file fails to resolve immediately, in the crate it was moved to.
- **Risk: the self dev-dependency does not behave as expected in the real workspace and `subx-core`'s own tests cannot see `test_support`.** → Mitigation: the mechanism was verified end-to-end in a scratch workspace of the same shape (edition 2024, resolver 3, root package + member, dual declaration) before any of this was written, and the task list re-verifies it in the real tree **as the first task of the manifest phase**, before a single helper is moved. Named fallback: run core's tests with `--features test-support`, which is the same shape as the existing `--features slow-tests` the scripts already pass.
- **Risk: `test-support` leaks into a release build and ships `wiremock` inside the binary.** → Mitigation: verified absent by counting `--cfg feature="test-support"` in `cargo build --release -v` (0) versus `cargo test --no-run -v` (present); the task list repeats that exact count in the real tree, and additionally builds `subx-cli` with `--no-default-features`.
- **Risk: a CRLF or BOM fixture is normalised during the move and the round-trip tests fail with a diff nobody can read.** → Mitigation: B1 shipped the `-text` rule in `subx-core`'s initial commit, three changes early, for this reason. B3 verifies the rule is present *before* the first `git add`, records the six sensitive files' byte sizes before and after, and re-checks from a fresh clone rather than from the working tree.
- **Risk: deleting 493 LOC of helpers and 104 LOC of tests removes something that turns out to be used.** → Mitigation: each deletion is justified by a grep over `src/`, `tests/` and `benches/` that the task list re-runs immediately before the deletion, and the deletions happen in their own phase so a revert is one commit. `command_helpers.rs` is the strongest case — it defines nothing at all — and `validators.rs`'s types survive as the copies in `TestWorkspace` that are actually used.
- **Risk: the seam is drawn wrongly and B4 turns out to depend on something B3 did not finish, or vice versa.** → Mitigation: Decision 7 states the one place they interlock (the three core-bound orphans) and resolves it explicitly, and the three handed-over bodies of work each touch files or moments B3 does not: files compiled by nothing, spawn calls in files B3 leaves in place, and a measurement that cannot be taken until after B4's own revivals.
- **Risk: the combined coverage figure moves even though no production line changed, and the 75% gate fails on a pure relocation.** → Mitigation: the instrumented run stays a single `--workspace` invocation over the same 49,069 lines, so the figure should be stable within noise; if it is not, that is evidence something was lost in the move and is investigated rather than absorbed by adjusting the floor. Per-crate floors are B4's and are not introduced here.
- **Risk: `default-members` being unset means a bare `cargo nextest run` skips all 43 core test files, and someone reads a green terminal as a green suite.** → Mitigation: documented in both `AGENTS.md` files with the `--workspace` workaround, called out in the CHANGELOG, and handed to C1 as a named item. B3's own verification always passes `--workspace` explicitly.
- **Trade-off: 1,929 LOC of test scaffolding becomes public API of a published crate, gated but visible.** → Accepted (Decision 2). It is the price of the only mechanism Cargo offers for sharing test code across a repository boundary, and D10 already set the precedent at the configuration layer. The dependency weight — the part that would actually cost consumers something — is kept optional.
- **Trade-off: `subx_core::test_support` is exempt from `missing_docs`.** → Accepted (Decision 2a). The alternative is most of a day writing `# Examples` for mock builders, and it would weaken rather than strengthen the signal the lint gives on the real API.
- **Trade-off: a 3.4 KB subtitle file exists in two repositories.** → Accepted (Decision 5). Every mechanism that avoids it costs more than 3.4 KB.
- **Trade-off: eleven `CLITestHelper` call sites are renamed in a change that is otherwise a move.** → Accepted (Decision 3). A `CLITestHelper` exported from a crate with no CLI is a name that makes the split unreadable.

## Migration Plan

Each step leaves the workspace either building or failing loudly, and the `subx-core` commit plus the `subx-cli` commit that moves the gitlink must be pushed together.

1. **Baseline.** Confirm B2 landed: `subx-cli/src/` holds only `cli/`, `commands/`, `lib.rs`, `main.rs`; `cargo nextest run --workspace` is green. Re-run the classification census and record the four counts, the `tests/common/` consumer table, the twelve never-compiled files (handed to B4) and the workspace coverage percentage. Confirm `subx-core/.gitattributes` carries the `-text` rule.
2. **Manifests and the mechanism, before any file moves.** Add `test-support` to `subx-core` with `wiremock`/`hound` optional, add the self dev-dependency, add `subx-cli`'s `[dev-dependencies]` declaration, and prove the mechanism with a one-line throwaway `test_support` item and a one-line test in each crate. Verify the release-build non-leak. Only then delete the throwaway.
3. **Delete what has no home.** The five `tests/common/` modules with no live consumer — `validators.rs`, `parallel_helpers.rs`, `sync_helpers.rs`, `integration_test_macros.rs` and `command_helpers.rs`. Re-grep each immediately before deleting. `cargo nextest run --workspace` stays green.
4. **Build `subx-core/src/test_support/`** from the seven surviving helper modules, splitting `cli_helpers.rs` and renaming `CLITestHelper` → `TestWorkspace`. Rewrite `subx-cli/tests/common/` down to `cli_helpers.rs` (`CommandResult` + `CliRun`) and `json_output.rs`. Update all consumers on both sides. This is the step where the suite is briefly red; it closes when `cargo nextest run --workspace` is green again with every test still in `subx-cli`.
5. **Move the 40 live core-bound files, the 22 fixtures, the two benches and the media assets**, with `subx_cli::` → `subx_core::` applied to the moved files only. Move the `[[bench]]` tables. Fix `src/services/vad/audio_loader.rs:33` and confirm `src/core/formats/tests_support.rs:63` now resolves.
6. **Rewrite the staying files' library imports** to `subx_core::…` and widen `tests/core_cli_boundary.rs`. After this, no test in either repository resolves through a D11 re-export.
7. **Verify:** `cargo nextest run --workspace`, `--features slow-tests`, `cargo bench --no-run --workspace`, `cargo doc --no-deps --all-features` in both crates, `cargo test --doc --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, a standalone `subx-core` clone outside the tree running `cargo test`, and `cargo build --release` with the feature-leak count.
8. **Documentation** and `[Unreleased]` CHANGELOG entries in both repositories, then the quality gate on the main agent only.

**Rollback** is `git revert` in `subx-cli` (restoring `tests/`, `benches/`, `assets/` and the old gitlink) plus `git revert` in `subx-core`. Nothing persists to disk, no public non-gated API changes, and the D11 re-exports were deliberately not deleted, so a revert of B3 alone leaves a working tree.

## Sizing

**B3 is ~9 h**, which is one long workday and is the residual after the seam. The change as originally scoped measured **~14.5 h**, and rather than overflow silently it hands ~5.5 h to `harden-split-test-suite` (B4) — see Decision 7 for what moves and why each piece is separable.

| Phase | Estimate |
|---|---|
| Classification census and baseline | 0.5 h |
| `test-support` feature, optional deps, self dev-dependency, mechanism proof | 1.5 h |
| Deleting the five unreferenced helper modules, with re-greps | 0.25 h |
| Building `src/test_support/`, splitting `cli_helpers.rs`, rewriting both `common/` trees and all consumers | 2.5 h |
| Moving 40 files + 22 fixtures + 2 benches + assets, with the import rewrite | 2 h |
| Rewriting the 71 live staying files' library imports; widening the boundary guard | 1 h |
| Verification across both crates, standalone clone, slow-tests, benches, docs | 0.75 h |
| Documentation and CHANGELOG in both repositories | 0.5 h |
| **Total** | **9 h** |

The LOC counts are misleading in both directions. The 40-file move is mechanical and its import rewrite is a substitution; what actually costs time is the `tests/common/` split, because it is the one part of this change that is a redesign rather than a relocation — one file cut in two, one type renamed across eleven call sites, five modules deleted, and a feature whose two tricky Cargo behaviours had to be verified before they could be relied on.

The item most likely to overrun is the `test-support` mechanism, and it is front-loaded deliberately: phase 2 proves it with a throwaway module before phase 4 moves 1,529 LOC onto it, so a failure is discovered when the fallback is still cheap.

## Open Questions

- **Should `CLITestHelper`'s `cargo run --` spawning become `assert_cmd`?** A nested `cargo run` inside a `cargo nextest run` serialises on Cargo's build lock and rebuilds the binary from inside a test, which is both slow and a source of flakiness under parallel scheduling. `Command::new(env!("CARGO_BIN_EXE_subx-cli"))` would fix it, and B4 establishes that pattern for the 26 `assert_cmd` sites. It is a behaviour change to eight test files and is out of scope for both B3 and B4. Worth its own small change, after C1.
- **Do the two `.llvm-cov.toml` files stay?** They are read by nothing in either repository (verified by grep over `scripts/`, `.github/` and both manifests), so their `exclude-from-report` patterns have never been in force. B3 records the finding and takes no action; B4 specifies the `--ignore-filename-regex` that replaces them, and C1 decides whether the files are deleted or made load-bearing.
- **Should `subx-core` grow its own `tests/common/` later?** Today it does not need one: all 43 core-bound files sit flat at `tests/*.rs` and reach shared code through `subx_core::test_support`. If a helper is ever needed that only core's tests want and that should not be public even behind a feature, `subx-core/tests/common/` is the right home for it, and Decision 2's "one mechanism" argument does not forbid it — it forbids splitting the *shared* set across two mechanisms.
- **Does the GUI need `test-support`?** SDR §8 lists `TestConfigService`, `TestConfigBuilder` and `TestEnvironmentProvider` in the GUI's consumed set; all three are ungated by D10, so the GUI's current test suite needs nothing from `test-support`. If it ever wants `MockOpenAITestHelper`, B2 Decision 5's feature-unification concern applies to it in full and should be re-read first.
