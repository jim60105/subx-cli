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
| Coverage report | `scripts/check_coverage.sh -T` (Linux/macOS) or `scripts/check_coverage.ps1 -Table` (Windows, PowerShell) |
| Doc build | `cargo doc --all-features --no-deps --document-private-items` |
| Doc tests | `cargo test --doc --all-features` |

### Important Notes

- **Always run `scripts/quality_check.sh`** once before every `git commit`.
  This is the single source of truth for quality validation — it runs
  formatting, linting, doc checks, and all tests **of both crates** (every
  `cargo check` / `clippy` / `nextest` invocation carries `--workspace`).
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

### CPU-Intensive Operations — Main Agent Only

**NEVER** run the following commands in sub-agents or in parallel:

- `scripts/quality_check.sh` (runs the full test suite + linting + docs)
- `scripts/check_coverage.sh` (runs the full test suite with instrumentation)
- `cargo nextest run` without a `--filter-expr` (runs all 2000+ tests)

These operations are CPU-intensive and will saturate all cores. Running them
in multiple sub-agents simultaneously will cause system overload, timeouts,
and unreliable results.

**Correct workflow:**

- Sub-agents writing tests should run only their own scoped tests using
  `cargo nextest run --filter-expr 'test(module_name)' || true`.
- The **main agent** runs `scripts/quality_check.sh` once after all
  sub-agents have finished and changes are consolidated.
- The main agent runs the quality check **before every `git commit`** to
  ensure all changes pass together.

## Repository Layout

The codebase is distributed across two git repositories:

- **`subx-cli`** (this repository) — the CLI binary and its library facade.
  Its `Cargo.toml` is the Cargo **workspace root** with
  `members = [".", "subx-core"]`.
- **`subx-core`** (<https://github.com/jim60105/subx-core>) — the reusable
  core library, mounted here as a **git submodule** at `subx-core/`.

Rules:

- `subx-core/Cargo.toml` must **never** contain a `[workspace]` table (a
  nested workspace root is a hard Cargo error) and must **never** use
  workspace inheritance (`version.workspace = true`,
  `<dep>.workspace = true`, `[lints] workspace = true`, ...): those resolve
  inside this workspace but break a standalone clone of `subx-core`, which is
  how crates.io, `docs.rs`, and the Tauri GUI consume the crate.
- `[profile.*]` tables belong to the workspace root manifest only; Cargo
  ignores them in a member and warns on every build when present.
- Library changes are committed in `subx-core` first, then the submodule
  pointer (gitlink) is bumped in `subx-cli`.

Cloning and updating:

```bash
git clone --recurse-submodules https://github.com/jim60105/subx-cli   # fresh clone
git submodule update --init --recursive                              # repair an existing clone
git config submodule.recurse true       # keep pulls/checkout in sync (per-clone setting)
```

## Architecture

### Execution Flow

```
main.rs → cli::run() → cli::run_with_config()
  → commands::dispatcher::dispatch_command_with_ref()
    → *_command::execute(args, &dyn ConfigService)
      → subx-core/src/core/* and subx-core/src/services/*
```

### Layer Overview

```
CLI Layer (src/cli/)          → Argument parsing, user interface
Command Layer (src/commands/) → Business logic per command
Core Layer (subx-core/src/core/)        → Processing engines, formats, matching
Service Layer (subx-core/src/services/) → External integrations (AI, audio, VAD)
Config Layer (subx-core/src/config/)    → DI-based configuration system
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

| Module | Purpose | Key Types |
|---|---|---|
| `src/cli/` | Argument parsing via clap derive + terminal presentation | `Cli`, `Commands`, `*Args`, `SubXErrorExt` (`error_ext.rs`), `TerminalReporter` |
| `src/commands/` | Command implementations | `dispatcher`, `execute()` functions |
| `subx-core/src/config/` | Configuration with DI | `ConfigService`, `Config`, `TestConfigService`, `TestConfigBuilder` |
| `subx-core/src/core/factory.rs` | Component wiring | `ComponentFactory` |
| `subx-core/src/core/archive/` | Archive extraction (`.zip`, `.7z`, `.tar.gz`, optional `.rar`) | `ArchiveFormat`, `extract_archive()` |
| `subx-core/src/core/formats/` | Subtitle parsing/conversion | `SubtitleFormat` trait, `FormatManager`, `FormatConverter` |
| `subx-core/src/core/matcher/` | AI-powered file matching | `MatchEngine`, `FileDiscovery`, `MatchConfig` |
| `subx-core/src/core/sync/` | Subtitle synchronization; core-owned sync pairing and default output-path derivation | `SyncEngine`, `SyncMethod`, `SyncMode`, `SyncPairingRequest`, `BatchRequest`, `resolve_sync_pairing`, `create_default_output_path`, `SYNC_VIDEO_EXTENSIONS`, `SYNC_SUBTITLE_EXTENSIONS` |
| `subx-core/src/core/uuidv7.rs` | Shared UUIDv7 generator with strict ≥1 ms spacing; canonical home of `Uuidv7Generator` and `generate_ids`. Used for matcher file IDs, translation cue IDs, and parallel worker/task IDs. | `Uuidv7Generator`, `generate_ids`, `unix_time_ms` |
| `subx-core/src/core/translation/` | AI-assisted subtitle translation (two-pass terminology + cue translation, UUIDv7 cue IDs re-exported from `core::uuidv7`) | `TranslationEngine`, `TerminologyMap`, `CueIdGenerator` (alias of `Uuidv7Generator`) |
| `subx-core/src/core/parallel/` | Task scheduling | `TaskScheduler`, `Task`, `WorkerPool` |
| `subx-core/src/core/file_manager.rs` | File operations with backup | `FileManager` |
| `subx-core/src/core/input/` | Core-owned input collection: path merging, directory scanning, extension filtering, archive extraction | `InputPathHandler`, `CollectedFiles` |
| `subx-core/src/services/ai/` | AI provider clients | `AIProvider` trait, `OpenAIClient`, `OpenRouterClient`, `AzureOpenAIClient`, `LocalLLMClient` |
| `subx-core/src/services/audio/` | Audio analysis data structures and helpers | `AudioData`, `AudioEnvelope`, `DialogueSegment` |
| `subx-core/src/services/vad/` | Voice Activity Detection | `LocalVadDetector`, `VadSyncDetector` |
| `subx-core/src/error.rs` | Error taxonomy and machine contracts (library half) | `SubXError` (`category()`, `machine_code()`, `hint()`), `SubXResult<T>` |
| `src/cli/error_ext.rs` | Binary-half error presentation (exit codes, terminal prose) | `SubXErrorExt` (`exit_code()`, `user_friendly_message()`) |

### Common Edit Targets

| Task | Files to Edit |
|---|---|
| Add/change CLI arguments | `src/cli/*_args.rs` (the collection algorithm lives in `subx-core/src/core/input/mod.rs`) |
| Add/change command logic | `src/commands/*_command.rs` |
| Change command routing | `src/commands/dispatcher.rs` |
| Add/change config keys | `subx-core/src/config/mod.rs`, `service.rs`, `field_validator.rs`, `validator.rs` |
| Add AI provider | `subx-core/src/services/ai/`, `subx-core/src/core/factory.rs`, `subx-core/src/config/` |
| Add subtitle format | `subx-core/src/core/formats/`, register in `FormatManager::new()` |
| Add/change translation behavior | `src/cli/translate_args.rs`, `src/commands/translate_command.rs`, `subx-core/src/core/translation/`, AI prompt helpers in `subx-core/src/services/ai/` |

## File Organization

```
.github/            GitHub Actions workflows and project-scoped skills
assets/             Project logo, media samples, test assets
benches/            Criterion performance benchmarks
docs/               Technical documentation
  ├── ai-provider-integration-guide.md   AI provider integration guide
  ├── command-reference.md                CLI command reference
  ├── config-usage-analysis.md           Configuration usage analysis
  ├── configuration-guide.md             Configuration reference
  └── tech-architecture.md               Technical architecture overview
openspec/           subx-cli's OpenSpec changes, specs, and workflow config
subx-core/openspec/  subx-core's own, independent OpenSpec root (changes, specs, config)
scripts/            Build, quality, and CI shell scripts (both crates' gate)
subx-core/scripts/  quality_check.sh — standalone-clone local gate for subx-core
  ├── quality_check.sh                   Full QA (lint, format, tests; Linux/macOS)
  ├── quality_check.ps1                  Full QA (Windows / PowerShell)
  ├── check_coverage.sh                  Coverage report (floors 75% combined / 90% core / 82% cli; Linux/macOS)
  ├── check_coverage.ps1                 Coverage report (same floors, Windows / PowerShell)
  ├── install.sh                         End-user binary installer
  ├── test_parallel_stability.sh         Parallel test isolation check
  └── test_unified_paths.sh              Path handling tests (⚠️ uses real AI API)
src/                Rust source code
  ├── cli/          CLI argument parsing and UI modules
  ├── commands/     Command implementations
  ├── lib.rs        Library entry point
  └── main.rs       Binary entry point
subx-core/          The subx-core library crate (git submodule):
  ├── config/       Configuration management (DI-based)
  ├── core/         Core processing engines
  ├── services/     External service integrations
  └── error.rs      Error type definitions
tests/              Integration tests organized by feature
  ├── cli/          CLI-focused test modules
  ├── commands/     Command-focused test modules
  ├── parallel/     Parallel execution test modules
  ├── sync/         Synchronization test modules
  └── common/       Shared test infrastructure (helpers, mocks, generators)
```

### Cargo Features

- `default = []` — no optional features are enabled by default.
- `slow-tests` — enables intentionally slower tests; CI uses this through
  `scripts/quality_check.sh -v -p ci --full`.
- `archive-rar` — enables `.rar` extraction via the optional `unrar`
  dependency.

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

### Error Handling

- Use `SubXError` variants from `subx-core/src/error.rs` — never invent
  ad-hoc error types.
- The machine contracts `category()`, `machine_code()` and `hint()` stay on
  `SubXError` in `subx-core/src/error.rs` (library half). The process/terminal
  half lives in `src/cli/error_ext.rs`: each variant maps to an exit code
  (1–6) via `SubXErrorExt::exit_code` and provides user-facing guidance via
  `SubXErrorExt::user_friendly_message` — callers must
  `use crate::cli::error_ext::SubXErrorExt;`. Code under
  `subx-core/src/core/` and `subx-core/src/services/` must not name the
  trait or either method — the crate may never reference `subx-cli` at all
  (enforced by `tests/core_cli_boundary.rs`); render messages through
  `Display` there.

### Naming Conventions

- Modules: `snake_case` — commands as `<verb>_command.rs`, args as
  `<verb>_args.rs`.
- Command entry points: `pub async fn execute(args, &dyn ConfigService)`.
- Factory methods: `create_*` on `ComponentFactory`.

## Testing Conventions

### Critical Rules

- **Always use `TestConfigService`** for configuration in tests. Never use
  `ProductionConfigService`.
- **Never modify global state** — no `std::env::set_var`, no `static mut`,
  no `Lazy<Mutex<_>>`, no writes outside `TempDir`.
- **All tests must be parallel-safe** and deterministic.
- **Async tests** use `#[tokio::test]`.

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

## Documentation Conventions

- Write rustdoc in **English** for all public APIs.
- Required sections for public functions: `# Arguments`, `# Returns`,
  `# Errors`, `# Examples`.
- Include `# Panics` and `# Safety` sections when applicable.
- All doc examples must compile — verified by `cargo test --doc --all-features`.
- Use intra-doc links: `` [`crate::module::Type`] ``. Broken links are
  denied (`broken_intra_doc_links = "deny"` in `Cargo.toml`).

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
`subx-core`'s `main` (advisory on PRs, blocking on `master` pushes), and
`.gitmodules` must keep recording `branch = main` — the referent
Dependabot's `gitsubmodules` entry (`.github/dependabot.yml`, weekly) uses to
keep the pointer moving. `subx-core` additionally runs its own CI in its own
repository (`subx-core/.github/workflows/build-test-audit-coverage.yml`:
three-OS test job, own-lockfile audit, ungated standalone coverage).

Releases publish **both crates**: the tag's `publish-crates` job asserts
Cargo ≥ 1.90, submodule cleanliness, the absence of any `--allow-dirty`
publish flag, and a `cargo publish --workspace --dry-run`, probes the sparse
crates.io index to pick `--workspace` vs `--workspace --exclude subx-core`,
then runs the real `cargo publish --workspace` (topological order; the
half-published recovery is documented in the job).

Releases are triggered by `v*` tags. The workflow extracts notes from
`CHANGELOG.md`, cross-compiles for 7 targets — Linux x86_64 (gnu),
Linux aarch64 (gnu), Linux x86_64 (musl), Linux aarch64 (musl),
Windows x86_64, macOS x86_64, and macOS aarch64 — and publishes them
to GitHub Releases and crates.io. The companion `scripts/install.sh`
auto-detects the host and libc; users on Alpine or other
glibc-incompatible distros can opt into the static musl artifacts via
`SUBX_LIBC=musl` or the `--musl` flag.

### Changelog Convention

Follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) with
[Semantic Versioning](https://semver.org/). Use sections: `### Added`,
`### Changed`, `### Fixed`, `### Removed`, `### Documentation`. The release
workflow parses the `## [VERSION]` header to generate release notes — always
add a properly formatted entry for every release.

<!-- CODEGRAPH_START -->
## CodeGraph

In repositories indexed by CodeGraph (a `.codegraph/` directory exists at the repo root), reach for it BEFORE grep/find or reading files when you need to understand or locate code:

- **MCP tool** (when available): `codegraph_explore` answers most code questions in one call — the relevant symbols' verbatim source plus the call paths between them, including dynamic-dispatch hops grep can't follow. Name a file or symbol in the query to read its current line-numbered source. If it's listed but deferred, load it by name via tool search.
- **Shell** (always works): `codegraph explore "<symbol names or question>"` prints the same output.

If there is no `.codegraph/` directory, skip CodeGraph entirely — indexing is the user's decision.
<!-- CODEGRAPH_END -->
