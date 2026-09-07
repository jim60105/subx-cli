# AGENTS.md

Instructions for AI coding agents working on **SubX-CLI**.

## Project Overview

SubX-CLI is an AI-powered command-line tool for automated subtitle
processing, written in Rust (edition 2024). It matches subtitle files to
videos using AI, converts between subtitle formats (SRT, ASS, VTT, SUB),
and synchronizes subtitle timing via Voice Activity Detection (VAD).

- **Repository:** <https://github.com/jim60105/subx-cli>
- **License:** GPL-3.0-or-later
- **Binary name:** `subx-cli`
- **Library half:** `subx-core` (<https://github.com/jim60105/subx-core>),
  mounted here as a git submodule at `subx-core/`

## Build, Test, and Quality Commands

Trust these instructions — only search the codebase if they are incomplete
or produce errors.

| Task | Command |
|---|---|
| Build | `cargo build` |
| Release build | `cargo build --release` |
| Format | `cargo fmt` |
| Lint | `cargo clippy -- -D warnings` |
| Run tests | `cargo nextest run --workspace \|\| true` |
| **Full quality check** | `scripts/quality_check.sh` (Linux/macOS) or `scripts/quality_check.ps1` (Windows, PowerShell) |
| Full QA (verbose) | `scripts/quality_check.sh -v` or `scripts/quality_check.ps1 -VerboseOutput` |
| Spec-governance check only | `scripts/quality_check.sh --check-spec-governance` or `scripts/quality_check.ps1 -CheckSpecGovernance` |
| Coverage report | `scripts/check_coverage.sh -T` (Linux/macOS) or `scripts/check_coverage.ps1 -Table` (Windows, PowerShell) |
| Doc build | `cargo doc --workspace --all-features --no-deps --document-private-items` |
| Doc tests | `cargo test --doc --all-features` |

### Important Notes

- **Always run `scripts/quality_check.sh`** once before every `git commit`.
  This is the single source of truth for quality validation — it runs
  formatting, linting, doc checks, the spec-governance drift check, and all
  tests **of both crates** (every `cargo check` / `clippy` / `nextest`
  invocation carries `--workspace`).
  For work inside a standalone `subx-core` clone, `subx-core/scripts/quality_check.sh`
  is the local gate — a strict subset; the authoritative run is still this one,
  in the superproject, at the moment the submodule pointer is bumped. Use `-v` for verbose
  output if debugging failures. On Unix, you may optionally wrap it with
  `timeout 240` to prevent hangs.
- **Use `cargo nextest run --workspace || true` for tests**, never bare
  `cargo test` (except for doc tests). The `|| true` prevents shell abort due
  to a known nextest issue in this project — **you must still inspect the
  output and treat any test failure as a real failure**. The manifest sets
  `default-members = [".", "subx-core"]`, so a bare `cargo nextest run`
  covers both crates; the scripts still pass `--workspace` explicitly, so the
  manifest key protects the human and the flag protects the gate.
- **Which crate does my test belong to?** Decide in this order:
  1. It spawns the binary (`assert_cmd` / `run_command_*`) → `subx-cli/tests/`.
  2. It names `subx_cli::cli`, `subx_cli::commands` or `subx_cli::App` →
     `subx-cli/tests/`.
  3. It names only library segments (`config`, `core`, `error`, `services`,
     the crate-root macros) → `subx-core/tests/`.
  4. Shared helpers with a cross-boundary consumer live in
     `subx_core::test_support` behind the `test-support` feature; helpers only
     one crate's tests want belong in that crate's `tests/`.
- **The `test-support` mechanism:** `subx_core::test_support` (workspace
  builder, file managers, mock OpenAI/Azure helpers, response generators) is
  gated by the `test-support` feature. `subx-cli` enables it only through
  `[dev-dependencies]` — never `--features` — and `subx-core` reaches it via a
  path-only self dev-dependency, so `cargo build --release` can never see the
  module or its optional `wiremock`/`hound` dependencies.
- **Harness shims:** Cargo auto-discovers only `tests/*.rs`; a test file under
  `tests/cli/` etc. needs exactly one top-level `#[path = "..."] mod` shim.
  `subx-core/tests/` is flat and therefore has none. A shim's target must not
  declare `mod common;` itself (the shim crate root does); a plain `mod
  common;` never needs a `#[path = "common/mod.rs"]` attribute. The
  `every_subdirectory_test_file_has_exactly_one_harness_shim` guard in
  `tests/core_cli_boundary.rs` walks both crates' test trees and **fails the
  build** if any subdirectory `.rs` file has zero shims (an orphan that is
  never compiled) or more than one (the same tests running twice); module
  directories with their own `mod.rs` are exempt.
- **Revive or delete — never leave an orphan:** revive a dead test file when
  its imports already resolve, or resolve after rewriting to `subx_core::…`
  paths; delete it when reviving would require authoring production code that
  does not exist (record the deletion and what it asserted in `CHANGELOG.md`).
  When a revived test's assertion fails because behaviour legitimately
  changed, update the **assertion** to characterise current behaviour — never
  change production code to satisfy a test that has never executed.
- **Spawning the binary in tests:** use
  `assert_cmd::Command::new(env!("CARGO_BIN_EXE_subx-cli"))`.
  `Command::cargo_bin("…")` is prohibited: a wrong binary name is a runtime
  panic that only fires when the test runs (and silently never fires in an
  orphan), while the `CARGO_BIN_EXE_<name>` env var is resolved by Cargo at
  compile time — a typo becomes a build error, and the value names the
  artefact actually built for this package in the two-package workspace.
- **Every fixture/asset read resolves from `env!("CARGO_MANIFEST_DIR")`**,
  never the working directory: Cargo sets a test binary's CWD to the *package*
  root, so a relative path that resolved before a module moved between
  packages silently stops resolving. See `subx-core/AGENTS.md` for the
  mirrored rules.
- **Always run `cargo fmt` and `cargo clippy -- -D warnings`** and fix every
  warning before submitting code.
- **Coverage floors** (line coverage, measured once on the split suite —
  workspace 91.77%, `subx-core` 93.54%, `subx-cli` 85.93%): workspace
  **75%**, `subx-core` **90%**, `subx-cli` **82%**. Floors are derived as
  `floor(measured − 3)` clamped to ≥75 (core) / ≥65 (cli); they may be
  **raised** as coverage improves and may only be **lowered through a
  proposal**. All three are enforced by `scripts/check_coverage.sh` / `.ps1`
  (flags `--threshold-core` / `--threshold-cli`, environment
  `COVERAGE_THRESHOLD_CORE` / `COVERAGE_THRESHOLD_CLI`, combined gate stays
  `COVERAGE_THRESHOLD`) and by the CI coverage matrix env. The JSON file list
  is partitioned by `.filename` (`/subx-core/src/` vs the workspace root's
  `src/`); each group's percentage comes from summed line counts — never a
  mean of per-file percentages — and the two groups must account for the
  reported total.
- Required tooling: Rust stable, `rustfmt`, `clippy`, `cargo-nextest`.
  For coverage: `cargo-llvm-cov`, plus `jq` and `bc` on Linux/macOS (the
  Windows port `scripts/check_coverage.ps1` parses JSON natively in
  PowerShell and needs neither `jq` nor `bc`).

## Repository Layout

The codebase is distributed across two git repositories:

- **`subx-cli`** (this repository) — the CLI binary and its library facade.
  Its `Cargo.toml` is the Cargo **workspace root** with
  `members = [".", "subx-core"]`.
- **`subx-core`** (<https://github.com/jim60105/subx-core>) — the reusable
  core library, mounted here as a **git submodule** at `subx-core/`.

Rules:

- Library changes are committed in `subx-core` first, then the submodule
  pointer (gitlink) is bumped in `subx-cli`.
- The `subx-core/Cargo.toml` prohibitions (no `[workspace]` table, no
  workspace inheritance, no `[profile.*]` tables) are recorded in the shared
  conventions region below, verbatim in both repositories' `AGENTS.md`.

Cloning and updating:

```bash
git clone --recurse-submodules https://github.com/jim60105/subx-cli   # fresh clone
git submodule update --init --recursive                              # repair an existing clone
git config submodule.recurse true       # keep pulls/checkout in sync (per-clone setting)
```

## Architecture

### Execution Flow

Everything above the arrow into `subx-core` runs in `subx-cli`:

```
subx-cli/src/main.rs → cli::run() → cli::run_with_config()        [subx-cli]
  → commands::dispatcher::dispatch_command_with_ref()             [subx-cli]
    → *_command::execute(args, &dyn ConfigService)                [subx-cli]
      → subx-core/src/core/* and subx-core/src/services/*         [subx-core]
```

### Layer Overview

```
CLI Layer     (subx-cli/src/cli/)      → Argument parsing, user interface
Command Layer (subx-cli/src/commands/) → Business logic per command
Core Layer    (subx-core/src/core/)    → Processing engines, formats, matching
Report Seam   (subx-core/src/report/)  → Reporter trait: structured progress/usage
                                          events a host (the CLI) renders
Service Layer (subx-core/src/services/)→ External integrations (AI, audio, VAD)
Config Layer  (subx-core/src/config/)  → DI-based configuration system
```

### Key Design Patterns

- **Dependency injection** — All components receive `&dyn ConfigService` or
  `Arc<dyn ConfigService>`. Never use global state. `ComponentFactory`
  (in `subx-core/src/core/factory.rs`) centralizes component construction from config.
- **Trait objects for polymorphism** — `SubtitleFormat` (format plugins),
  `AIProvider` (AI backends), `ConfigService` (prod vs test),
  `EnvironmentProvider` (system vs test env).
- **Async throughout** — `tokio` runtime; `async_trait` for trait methods;
  semaphore-limited concurrency.
- **Error handling** — `SubXError` enum via `thiserror` with typed variants,
  exit codes (1–6), and user-friendly messages. Use `crate::Result<T>`
  alias. Propagate with `?`; use `From` impls for automatic conversion.
- **File operations** — `FileManager` provides batch file operations with
  backup support. Rollback covers recorded creations and moves but cannot
  restore removed files.

### Module Guide

| Module | Crate | Purpose | Key Types |
|---|---|---|---|
| `src/cli/` | `subx-cli` | Argument parsing via clap derive + terminal presentation | `Cli`, `Commands`, `*Args`, `SubXErrorExt` (`error_ext.rs`), `TerminalReporter` |
| `src/commands/` | `subx-cli` | Command implementations | `dispatcher`, `execute()` functions |
| `src/config/` | `subx-core` | Configuration with DI | `ConfigService`, `Config`, `TestConfigService`, `TestConfigBuilder` |
| `src/core/factory.rs` | `subx-core` | Component wiring | `ComponentFactory` |
| `src/core/archive/` | `subx-core` | Archive extraction (`.zip`, `.7z`, `.tar.gz`, optional `.rar`) | `ArchiveFormat`, `extract_archive()` |
| `src/core/formats/` | `subx-core` | Subtitle parsing/conversion | `SubtitleFormat` trait, `FormatManager`, `FormatConverter` |
| `src/core/matcher/` | `subx-core` | AI-powered file matching | `MatchEngine`, `FileDiscovery`, `MatchConfig` |
| `src/core/report/` | `subx-core` | Reporter seam for batch progress/usage events | `Reporter`, `ProgressEvent` |
| `src/core/sync/` | `subx-core` | Subtitle synchronization; core-owned sync pairing and default output-path derivation | `SyncEngine`, `SyncMethod`, `SyncMode`, `SyncPairingRequest`, `BatchRequest`, `resolve_sync_pairing`, `create_default_output_path`, `SYNC_VIDEO_EXTENSIONS`, `SYNC_SUBTITLE_EXTENSIONS` |
| `src/core/uuidv7.rs` | `subx-core` | Shared UUIDv7 generator with strict ≥1 ms spacing; canonical home of `Uuidv7Generator` and `generate_ids`. Used for matcher file IDs, translation cue IDs, and parallel worker/task IDs. | `Uuidv7Generator`, `generate_ids`, `unix_time_ms` |
| `src/core/translation/` | `subx-core` | AI-assisted subtitle translation (two-pass terminology + cue translation, UUIDv7 cue IDs re-exported from `core::uuidv7`) | `TranslationEngine`, `TerminologyMap`, `CueIdGenerator` (alias of `Uuidv7Generator`) |
| `src/core/parallel/` | `subx-core` | Task scheduling | `TaskScheduler`, `Task`, `WorkerPool` |
| `src/core/file_manager.rs` | `subx-core` | File operations with backup | `FileManager` |
| `src/core/input/` | `subx-core` | Core-owned input collection: path merging, directory scanning, extension filtering, archive extraction | `InputPathHandler`, `CollectedFiles` |
| `src/services/ai/` | `subx-core` | AI provider clients | `AIProvider` trait, `OpenAIClient`, `OpenRouterClient`, `AzureOpenAIClient`, `LocalLLMClient` |
| `src/services/audio/` | `subx-core` | Audio analysis data structures and helpers | `AudioData`, `AudioEnvelope`, `DialogueSegment` |
| `src/services/vad/` | `subx-core` | Voice Activity Detection | `LocalVadDetector`, `VadSyncDetector` |
| `src/error.rs` | `subx-core` | Error taxonomy and machine contracts (library half) | `SubXError` (`category()`, `machine_code()`, `hint()`), `SubXResult<T>` |
| `src/cli/error_ext.rs` | `subx-cli` | Binary-half error presentation (exit codes, terminal prose) | `SubXErrorExt` (`exit_code()`, `user_friendly_message()`) |

In this table, `src/…` rows prefixed `subx-core` are paths inside the
`subx-core/` submodule mount; the crate's canonical paths are
`subx_core::…`.

### Common Edit Targets

| Task | Files to Edit |
|---|---|
| Add/change CLI arguments | `subx-cli/src/cli/*_args.rs` (the collection algorithm lives in `subx-core/src/core/input/mod.rs`) |
| Add/change command logic | `subx-cli/src/commands/*_command.rs` |
| Change command routing | `subx-cli/src/commands/dispatcher.rs` |
| Add/change config keys | `subx-core/src/config/mod.rs`, `subx-core/src/config/service.rs`, `subx-core/src/config/field_validator.rs`, `subx-core/src/config/validator.rs` |
| Add AI provider | `subx-core/src/services/ai/`, `subx-core/src/core/factory.rs`, `subx-core/src/config/` |
| Add subtitle format | `subx-core/src/core/formats/`, register in `FormatManager::new()` |
| Add/change translation behavior | `subx-cli/src/cli/translate_args.rs`, `subx-cli/src/commands/translate_command.rs`, `subx-core/src/core/translation/`, AI prompt helpers in `subx-core/src/services/ai/` |

### CLI Naming

- Modules: commands as `<verb>_command.rs`, args as `<verb>_args.rs`.
- Command entry points: `pub async fn execute(args, &dyn ConfigService)`.

## Error Handling

- Use `SubXError` variants from `subx-core/src/error.rs` — never invent
  ad-hoc error types. The exit-code range 1–6 and this no-ad-hoc-types rule
  are unchanged by the crate split.
- The machine contracts `category()`, `machine_code()` and `hint()` stay on
  `SubXError` in `subx-core/src/error.rs` (library half). The process/terminal
  half lives in `subx-cli/src/cli/error_ext.rs`: each variant maps to an exit
  code (1–6) via `SubXErrorExt::exit_code` and provides user-facing guidance
  via `SubXErrorExt::user_friendly_message` — callers must
  `use crate::cli::error_ext::SubXErrorExt;`. Code under
  `subx-core/src/core/` and `subx-core/src/services/` must not name the
  trait or either method — the crate may never reference `subx-cli` at all
  (enforced by `tests/core_cli_boundary.rs`); render messages through
  `Display` there.

## File Organization

```
subx-cli/                    This repository (workspace root, CLI binary)
├── .github/                 GitHub Actions workflows and project-scoped skills
├── assets/                  Project logo, media samples, test assets
├── docs/                    Technical documentation (all six stay here)
│   ├── ai-provider-integration-guide.md   AI provider integration guide
│   ├── command-reference.md               CLI command reference
│   ├── config-usage-analysis.md           Configuration usage analysis
│   ├── configuration-guide.md             Configuration reference
│   ├── machine-readable-output.md         JSON output contract
│   └── tech-architecture.md               Technical architecture overview
├── openspec/                subx-cli's OpenSpec changes, specs, and workflow config
│   └── split-capabilities.txt             the twelve split-capability names
├── scripts/                 Build, quality, and CI shell scripts (both crates' gate)
│   ├── quality_check.sh                   Full QA (lint, format, tests, governance; Linux/macOS)
│   ├── quality_check.ps1                  Full QA (Windows / PowerShell)
│   ├── check_coverage.sh                  Coverage report (floors 75% combined / 90% core / 82% cli; Linux/macOS)
│   ├── check_coverage.ps1                 Coverage report (same floors, Windows / PowerShell)
│   ├── install.sh                         End-user binary installer
│   ├── test_parallel_stability.sh         Parallel test isolation check
│   └── test_unified_paths.sh              Path handling tests (⚠️ uses real AI API)
├── src/                     CLI half of the Rust sources
│   ├── cli/                 CLI argument parsing and UI modules
│   ├── commands/            Command implementations
│   ├── lib.rs               Library facade (compatibility re-exports of subx-core)
│   └── main.rs              Binary entry point
├── tests/                   Integration tests organized by feature
│   ├── cli/                 CLI-focused test modules
│   ├── commands/            Command-focused test modules
│   ├── parallel/            Parallel execution test modules
│   ├── sync/                Synchronization test modules
│   └── common/              Shared test infrastructure (helpers, mocks, generators)
└── subx-core/               The library crate — git submodule, own repository below

subx-core/                   Library repository (its own CI, openspec root, configs)
├── .config/nextest.toml     Its own nextest profile
├── .github/                 Its own build-test-audit-coverage workflow
├── assets/                  Media assets its tests read
├── benches/                 Criterion benchmarks (retry_performance, file_id_generation_bench)
├── openspec/                Its own, independent OpenSpec root (changes, specs, config)
├── scripts/quality_check.sh Standalone-clone local gate (takes an optional profile)
├── src/                     Library half of the Rust sources
│   ├── config/              Configuration management (DI-based)
│   ├── core/                Core processing engines (incl. report/ seam)
│   ├── services/            External service integrations
│   ├── error.rs             Error type definitions
│   ├── test_support/        Shared test fixtures (test-support feature gated)
│   └── lib.rs               Library entry point
└── tests/                   Flat integration tests (+ tests/fixtures/)
```

### Cargo Features

Which manifest owns each gate:

- `default = []` in **both** manifests — no optional features are enabled by
  default anywhere.
- `archive-rar` — the real gate lives in `subx-core/Cargo.toml`
  (`archive-rar = ["dep:unrar"]`); `subx-cli/Cargo.toml` declares a
  pass-through (`archive-rar = ["subx-core/archive-rar"]`).
- `slow-tests` — same shape: real gate in `subx-core` (`slow-tests = []`),
  pass-through in `subx-cli` (`slow-tests = ["subx-core/slow-tests"]`); CI
  uses it through `scripts/quality_check.sh -v -p ci --full`.
- `test-support` — exists **only** in `subx-core/Cargo.toml`
  (`test-support = ["dep:wiremock", "dep:hound"]`). `subx-cli` never declares
  it as a feature; it enables it exclusively through `[dev-dependencies]`.

<!-- SHARED_CONVENTIONS_START -->
<!-- The text between these two markers (inclusive) is byte-identical in
     subx-cli/AGENTS.md and subx-core/AGENTS.md. The superproject's quality
     gate enforces it: scripts/quality_check.sh --check-spec-governance
     (quality_check.ps1 -CheckSpecGovernance), exit code 2 on failure.
     Edit this region in BOTH repositories in the same change or the gate
     turns red. The tool-managed CODEGRAPH block lives outside the region
     in both files. -->

## Coding Conventions

### General Rules

- All code comments and rustdoc must be written in **English**.
- Do not introduce new `#[deprecated]` attributes. When removing
  functionality, delete the item and update all call sites. Some legacy
  fields in `SyncConfig` still carry `#[deprecated]` for backward
  compatibility — leave those as-is unless actively cleaning them up.
- Unimplemented code must be marked with `// TODO`. Unless requirements
  explicitly permit phased implementation, all TODOs must be resolved
  before submitting.
- Never parse or hand-edit `Cargo.lock` — it is managed by Cargo.
- Formatting: `rustfmt.toml` sets edition 2024 with max width 100 columns.

### Naming Conventions

- Modules: `snake_case`.
- Factory methods: `create_*` on `ComponentFactory`.

## Documentation Conventions

- Write rustdoc in **English** for all public APIs.
- Required sections for public functions: `# Arguments`, `# Returns`,
  `# Errors`, `# Examples`.
- Include `# Panics` and `# Safety` sections when applicable.
- All doc examples must compile — verified by `cargo test --doc --all-features`.
- Use intra-doc links: `` [`crate::module::Type`] ``. Broken links are
  denied (`broken_intra_doc_links = "deny"` in `Cargo.toml`).
- **Cross-crate rustdoc links are one-way**: the CLI's rustdoc may link into
  the core with absolute `subx_core::...` paths; core rustdoc SHALL NOT
  contain any bracketed `[subx_cli::...]` link. The CLI is not a core
  dependency, so the deny'd broken intra-doc link turns the standalone core
  documentation build into a build failure. Core documentation may mention
  CLI behaviour only as backticked prose — the hyphen form `subx-cli`; the
  underscore spelling `subx_cli` is rejected by the layering guard even
  inside core comments and doctests — or a plain GitHub URL, never as an
  intra-doc link.
- **Verify the shared documentation boundary with
  `cargo doc --workspace --all-features`** — not `--no-deps`.
  `cargo doc --no-deps` documents only the local crates without building
  registry dependencies, so it can report success while generating no
  `subx_core` pages at all and leaving every re-export link pointing at a
  page that does not exist.

### Changelog Convention

Follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) with
[Semantic Versioning](https://semver.org/). Use sections: `### Added`,
`### Changed`, `### Fixed`, `### Removed`, `### Documentation`. Every
user-facing change (including CI/release behaviour) gets an entry in this
repository's `CHANGELOG.md` under the top `## [Unreleased]` header, and every
released entry lives under its own `## [VERSION]` header. `subx-cli`'s release
workflow parses that repository's `## [VERSION]` headers to generate release
notes — always add a properly formatted entry for every release.

## CPU-Intensive Operations — Main Agent Only

Running the full test suite or coverage check hogs CPU and interferes with
parallel work. Subagents and worker sessions MUST NOT run these themselves:

- `scripts/quality_check.sh` (every profile; the `.ps1` port exists in the
  `subx-cli` repository) — the full check suite
- `scripts/check_coverage.sh` / `.ps1` — the instrumented full-suite coverage
  run (`subx-cli` repository only)
- bare `cargo nextest run` without a `--filter-expr`

**Correct workflow:** subagents run only their own scoped tests with
`cargo nextest run --filter-expr 'test(module_name)' || true`. When a task
needs full-suite or coverage validation, request it from the main agent, who
runs it once after all sub-agent work is consolidated and before every
`git commit`. `cargo check` is fine for quick validation. CI generates
coverage reports when its workflow runs (note both workflows' `paths-ignore`
skips documentation-only changes), so local coverage is usually unnecessary.

## subx-core Manifest Prohibitions

`subx-core/Cargo.toml` SHALL NOT contain a `[workspace]` table, workspace
inheritance (`version.workspace = true`, `authors.workspace = true`,
`<dep>.workspace = true`, `[lints] workspace = true`), or `[profile.*]`
tables. The superproject root manifest owns the `[profile.release]` settings
and the shared dependency versions; a workspace table or inheritance in the
member would stop `cargo build` from working in a standalone clone of the
library repository, which is a supported workflow — the Tauri GUI consumes
it through a git dependency today, and after the first publication crates.io
and docs.rs will consume exactly that same standalone tree.

## Testing Conventions

### Critical Rules

- **Always use `TestConfigService`** for configuration in tests. Never use
  `ProductionConfigService`.
- **Never modify global state** — no `std::env::set_var`, no `static mut`,
  no `Lazy<Mutex<_>>`, no writes outside `TempDir`.
- **All tests must be parallel-safe** and deterministic.
- **Async tests** use `#[tokio::test]`.

Repository-specific test infrastructure, test patterns, and test
organisation are the Test Infrastructure, Test Patterns, and Test
Organization sections immediately below this shared region.

<!-- SHARED_CONVENTIONS_END -->

### Test Infrastructure

| Helper | Location | Purpose |
|---|---|---|
| `TestConfigService` | `subx-core/src/config/test_service.rs` | Isolated config without filesystem I/O |
| `TestConfigBuilder` | `subx-core/src/config/builder.rs` | Fluent builder for test configs |
| `TestEnvironmentProvider` | `subx-core/src/config/environment.rs` | In-memory env vars for isolated testing |
| `CLITestHelper` | `tests/common/cli_helpers.rs` | TempDir + config; auto-cleanup via `Drop` |
| `MockOpenAITestHelper` | `tests/common/mock_openai_helper.rs` | Wiremock-based AI mock server |
| `MockAzureOpenAITestHelper` | `tests/common/mock_azure_openai_helper.rs` | Wiremock-based Azure OpenAI mock server |
| `MatchResponseGenerator` | `tests/common/test_data_generators.rs` | AI response fixture generator |

See `tests/common/mod.rs` for the complete shared helper list, including
command, sync, parallel, file-manager, validator, and mock-generator helpers.

### Test Patterns

```rust
// Unit test with config
#[tokio::test]
async fn test_feature() {
    let config_service = TestConfigBuilder::new()
        .with_ai_provider("openai")
        .with_ai_model("gpt-4.1-mini")
        .build_service();
    let result = some_function(&*config_service).await;
    assert!(result.is_ok());
}

// Integration test with wiremock
#[tokio::test]
async fn test_with_mock_ai() {
    let mock = MockOpenAITestHelper::new().await;
    mock.mock_chat_completion_success(
        &MatchResponseGenerator::successful_single_match(),
    ).await;
    let config = TestConfigBuilder::new()
        .with_mock_ai_server(&mock.base_url())
        .build_service();
    // ... invoke command ...
    mock.verify_expectations().await;
}
```

### Test Organization

- **Unit tests:** Inline `#[cfg(test)] mod tests` in source files.
- **Integration tests:** `tests/*.rs`, one file per feature area. Import
  shared helpers via `mod common;` at the top.
- **Nested test modules:** directories under `tests/cli/`, `tests/commands/`,
  `tests/parallel/`, and `tests/sync/` are not automatically discovered by
  Cargo as standalone integration tests. Wire new nested tests through an
  existing top-level test harness or add an explicit top-level `tests/*.rs`
  entry so `cargo nextest` runs them.
- **Shared helpers:** `tests/common/` — mocks, generators, CLI helpers.
- **Benchmarks:** `benches/` using Criterion with `criterion_group!` and
  `criterion_main!`; registered benches are `retry_performance` and
  `file_id_generation_bench`.

## Configuration System

The configuration system uses dependency injection. Components receive
`&dyn ConfigService` — never read config files directly.

### Config Priority (highest → lowest)

1. Environment variables
2. User config file (`~/.config/subx/config.toml` on Linux/macOS,
   `%APPDATA%\subx\config.toml` on Windows)
3. Built-in defaults

### Supported Environment Variables

Provider-specific variables (checked first):

- `OPENAI_API_KEY`, `OPENAI_BASE_URL` — OpenAI provider
- `OPENROUTER_API_KEY` — OpenRouter provider
- `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`,
  `AZURE_OPENAI_API_VERSION` — Azure OpenAI provider
- `LOCAL_LLM_API_KEY`, `LOCAL_LLM_BASE_URL` — Local LLM provider (only
  honored when `ai.provider = "local"`).

General overrides with `SUBX_` prefix (e.g., `SUBX_AI_MODEL`,
`SUBX_GENERAL_WORKSPACE`). Note that env-var handling has special cases
in `subx-core/src/config/service.rs` — check the implementation if a specific
override doesn't work as expected.

Workspace override: `SUBX_WORKSPACE` or `general.workspace` config changes
the working directory before command dispatch.

### Config Sections

- `[ai]` — Provider, API key, model, base URL, retry, timeout (default
  provider: `openai`, default model: `gpt-4.1-mini`)
- `[formats]` — Output format, encoding, styling preservation
- `[sync]` / `[sync.vad]` — Sync method, VAD sensitivity, padding
- `[general]` — Backup, concurrency, timeout, workspace, progress bar
- `[parallel]` — Worker pool, overflow strategy, task queue

See `docs/configuration-guide.md` for the full reference.

### Adding New Config Keys

New configuration keys must be added to all of the following:

1. `subx-core/src/config/mod.rs` — struct field with serde attributes
2. `subx-core/src/config/service.rs` — both `get_config_value()` and
   `set_config_value()`
3. `subx-core/src/config/field_validator.rs` — field-level validation
4. `subx-core/src/config/validator.rs` — section-level validation
5. `docs/configuration-guide.md` — user-facing documentation

## AI Provider System

Four providers are supported: `openai`, `openrouter`, `azure-openai`, and
`local`. The string `ollama` is accepted as an alias for `local` and is
normalized to the canonical value at config write time. All implement the
`AIProvider` trait (defined in `subx-core/src/services/ai/mod.rs`) with two async
methods: `analyze_content()` and `verify_match()`.

Hosted providers (`openai`, `openrouter`, `azure-openai`) require
`https://` for any user-set `ai.base_url`; non-HTTPS values are rejected
by `validate_ai_config` with a hint pointing to `ai.provider = "local"`.
The `local` provider is endpoint-agnostic — it accepts loopback, LAN,
VPN/tailnet, and remote OpenAI-compatible endpoints over either `http://`
or `https://`.

To add a new provider, follow the step-by-step guide in
`docs/ai-provider-integration-guide.md`. Key touchpoints: create the client
in `subx-core/src/services/ai/`, register in `subx-core/src/core/factory.rs` →
`create_ai_provider()`, add validation in `subx-core/src/config/field_validator.rs`
and `subx-core/src/config/validator.rs`, and update both README files plus
`docs/configuration-guide.md`.

AI-provider changes should also review supporting modules in
`subx-core/src/services/ai/`: `cache.rs`, `error_sanitizer.rs`, `prompts.rs`,
`retry.rs`, and `security.rs`.

## OpenSpec and Project Skills

This repository includes OpenSpec artifacts under `openspec/` and
project-scoped skills under `.github/skills/`. Use the OpenSpec skills for
proposal/change workflows when the user asks to propose, apply, verify, sync,
or archive changes. Use the `update-config-document` skill when configuration
items or configuration documentation need to be audited or refreshed.

**Two OpenSpec roots.** `openspec/` at this root specifies `subx-cli`;
`subx-core/openspec/` is a second, independent root that specifies the
`subx-core` crate. The parent's tooling never descends into the submodule:
`openspec validate`/`list` at this root reports only this root's specs, and the
same commands run inside `subx-core/` resolve the nested root. A change resolves
against exactly one root — run its commands from inside that repository. Work
spanning both repositories is authored as one change per repository, each
change's `## Why` naming the other as its other half (see
`openspec/specs/spec-governance/` for the capability-assignment and citation
rules).

**Split capabilities.** Twelve capability names — `cache-management`,
`component-factory`, `configuration-management`, `encoding-detection`,
`error-handling`, `format-conversion`, `input-path-handling`,
`parallel-processing`, `secrets-protection`, `subtitle-matching`,
`subtitle-translation`, `timeline-sync` — exist in **both**
`openspec/specs/` and `subx-core/openspec/specs/` by design: each is one half
of one capability, split along the crate boundary. The authoritative record of
which names are split is `openspec/split-capabilities.txt`; a name in both
trees and not in that file is a duplication defect, and a name in the file and
one tree only is a split that lost a half. When adding a requirement to a split
capability, decide its owning repository first, by `spec-governance`'s
ownership test, and add it to that repository's half — never to whichever half
happened to be open; a capability that grows a requirement on the other side is
split, not moved. A change spanning both trees is archived with `--skip-specs`
in the sending repository even when no capability is emptied, because
`openspec archive` does not rewrite `## Purpose` paragraphs and a stale Purpose
passes `validate --strict`.

**Spec-governance drift check.** `scripts/quality_check.sh
--check-spec-governance` (and the `.ps1` port's `-CheckSpecGovernance`)
asserts three predicates over the two roots and exits **2** on any failure,
distinct from the generic failure exit 1:

1. No capability directory name appears in both `openspec/specs/` and
   `subx-core/openspec/specs/` unless it is recorded in
   `openspec/split-capabilities.txt`.
2. Every name in `openspec/split-capabilities.txt` is present in **both**
   trees — a name in one tree only is a split that lost a half.
3. The `SHARED_CONVENTIONS` marker-inclusive region of this file and
   `subx-core/AGENTS.md` is byte-identical (the tool-managed CODEGRAPH block
   lives outside the region in both files). On failure the check prints both
   file paths and a diff of the two extracted regions.

All three predicates skip with a warning when `subx-core/openspec/specs/`
does not exist (a missing submodule already fails the build at
manifest-parse time). The full `scripts/quality_check.sh` run includes the
check. **Known residual gap (deliberately open):** the check catches a
requirement title present in both trees, but nothing catches two
differently-titled requirements in the two halves of one split capability
that contradict each other.

**Manual-archive procedure.** A delta that removes every requirement of a
capability makes `openspec archive` refuse atomically
(`archive_spec_validation_failed` — a rebuilt spec with zero requirements fails
validation), and that refusal skips the change's other deltas. Such a change is
archived with `--skip-specs`, after which **every** delta — removals, additions
and modifications alike — is applied by hand (deleting the emptied capability
directories, writing new-capability main specs, and substituting MODIFIED
requirement blocks). Corollary: a newly created capability's H1 and `## Purpose`
must be replaced after any archive, because `openspec archive` writes
`# <capability> Specification` plus a `TBD` Purpose that passes
`validate --strict` while being wrong.

## CI/CD Pipeline

CI runs on push/PR to `master` across Ubuntu, Windows, and macOS with Rust
stable. It executes `scripts/quality_check.sh -v -p ci --full` (workspace-wide),
runs `cargo audit` for security, and enforces the three coverage floors via
`scripts/check_coverage.sh` on Linux/macOS and the PowerShell port
`scripts/check_coverage.ps1` on Windows. Results are uploaded to Codecov.
A `submodule-pointer` job guards the supply chain: the `subx-core` version must
match `subx-cli`'s caret requirement, the gitlink commit must be reachable from
`subx-core`'s `master` (advisory on PRs, blocking on `master` pushes), and
`.gitmodules` must keep recording `branch = master` — the referent
Dependabot's `gitsubmodules` entry (`.github/dependabot.yml`, weekly) uses to
keep the pointer moving. `subx-core` additionally runs its own CI in its own
repository (`subx-core/.github/workflows/build-test-audit-coverage.yml`:
three-OS test job, own-lockfile audit, ungated standalone coverage).

Each crate publishes **from its own repository's `v*` tag** — `subx-cli` is
not an umbrella project (the crate has a second consumer, the Tauri GUI at
`jim60105/subx`), so `subx-core` releases from `subx-core`'s own
`release.yml` and this repository's tag publishes `subx-cli` alone.
Ordering contract: tag `jim60105/subx-core` first; this repository's
`publish-crates` job probes the sparse crates.io index for a published
`subx-core` version satisfying the caret requirement in `Cargo.toml` and
refuses the tag until one exists, then asserts submodule cleanliness, the
absence of any `--allow-dirty` publish flag, gitlink ancestry from
`subx-core`'s `master`, tag-versus-manifest version agreement, and a
`cargo publish -p subx-cli --dry-run` before the single real upload
(`cargo publish -p subx-cli`; no retry — see the recovery comment in the
job).

Releases are triggered by `v*` tags. The workflow extracts notes from
`CHANGELOG.md`, cross-compiles for **five** targets — Linux x86_64 (gnu),
Linux aarch64 (gnu), Windows x86_64, macOS x86_64, and macOS aarch64 —
and publishes them to GitHub Releases and crates.io. There are **no musl
artifacts**: musl support was dropped (archived change
`2026-04-28-drop-musl-support`), and the companion `scripts/install.sh`
rejects both `SUBX_LIBC=musl` and the `--musl` flag with exit code 2 and
no HTTP request. musl users build from source against a locally
provisioned ONNX Runtime.

<!-- CODEGRAPH_START -->
## CodeGraph

In repositories indexed by CodeGraph (a `.codegraph/` directory exists at the repo root), reach for it BEFORE grep/find or reading files when you need to understand or locate code:

- **MCP tool** (when available): `codegraph_explore` answers most code questions in one call — the relevant symbols' verbatim source plus the call paths between them, including dynamic-dispatch hops grep can't follow. Name a file or symbol in the query to read its current line-numbered source. If it's listed but deferred, load it by name via tool search.
- **Shell** (always works): `codegraph explore "<symbol names or question>"` prints the same output.

If there is no `.codegraph/` directory, skip CodeGraph entirely — indexing is the user's decision.
<!-- CODEGRAPH_END -->
