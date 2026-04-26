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
| Run tests | `cargo nextest run \|\| true` |
| **Full quality check** | `scripts/quality_check.sh` |
| Full QA (verbose) | `scripts/quality_check.sh -v` |
| Coverage report | `scripts/check_coverage.sh -T` |
| Doc build | `cargo doc --all-features --no-deps --document-private-items` |
| Doc tests | `cargo test --doc --all-features` |

### Important Notes

- **Always run `scripts/quality_check.sh`** once before every `git commit`.
  This is the single source of truth for quality validation — it runs
  formatting, linting, doc checks, and all tests. Use `-v` for verbose
  output if debugging failures. On Unix, you may optionally wrap it with
  `timeout 240` to prevent hangs.
- **Use `cargo nextest run || true` for tests**, never `cargo test` (except
  for doc tests). The `|| true` prevents shell abort due to a known nextest
  issue in this project — **you must still inspect the output and treat any
  test failure as a real failure**.
- **Always run `cargo fmt` and `cargo clippy -- -D warnings`** and fix every
  warning before submitting code.
- Coverage threshold is **75%** line coverage.
- Required tooling: Rust stable, `rustfmt`, `clippy`, `cargo-nextest`.
  For coverage: `cargo-llvm-cov`, `jq`, `bc`.

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

## Architecture

### Execution Flow

```
main.rs → cli::run() → cli::run_with_config()
  → commands::dispatcher::dispatch_command_with_ref()
    → *_command::execute(args, &dyn ConfigService)
      → core/* and services/*
```

### Layer Overview

```
CLI Layer (src/cli/)          → Argument parsing, user interface
Command Layer (src/commands/) → Business logic per command
Core Layer (src/core/)        → Processing engines, formats, matching
Service Layer (src/services/) → External integrations (AI, audio, VAD)
Config Layer (src/config/)    → DI-based configuration system
```

### Key Design Patterns

- **Dependency injection** — All components receive `&dyn ConfigService` or
  `Arc<dyn ConfigService>`. Never use global state. `ComponentFactory`
  (in `src/core/factory.rs`) centralizes component construction from config.
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
| `src/cli/` | Argument parsing via clap derive | `Cli`, `Commands`, `*Args`, `InputPathHandler` |
| `src/commands/` | Command implementations | `dispatcher`, `execute()` functions |
| `src/config/` | Configuration with DI | `ConfigService`, `Config`, `TestConfigService`, `TestConfigBuilder` |
| `src/core/factory.rs` | Component wiring | `ComponentFactory` |
| `src/core/archive/` | Archive extraction (`.zip`, `.7z`, `.tar.gz`, optional `.rar`) | `ArchiveFormat`, `extract_archive()` |
| `src/core/formats/` | Subtitle parsing/conversion | `SubtitleFormat` trait, `FormatManager`, `FormatConverter` |
| `src/core/matcher/` | AI-powered file matching | `MatchEngine`, `FileDiscovery`, `MatchConfig` |
| `src/core/sync/` | Subtitle synchronization | `SyncEngine`, `SyncMethod` |
| `src/core/translation/` | AI-assisted subtitle translation (two-pass terminology + cue translation, UUIDv7 cue IDs) | `TranslationEngine`, `TerminologyMap`, `CueIdGenerator` |
| `src/core/parallel/` | Task scheduling | `TaskScheduler`, `Task`, `WorkerPool` |
| `src/core/file_manager.rs` | File operations with backup | `FileManager` |
| `src/services/ai/` | AI provider clients | `AIProvider` trait, `OpenAIClient`, `OpenRouterClient`, `AzureOpenAIClient`, `LocalLLMClient` |
| `src/services/audio/` | Audio analysis data structures and helpers | `AudioData`, `AudioEnvelope`, `DialogueSegment` |
| `src/services/vad/` | Voice Activity Detection | `LocalVadDetector`, `VadSyncDetector` |
| `src/error.rs` | Error definitions | `SubXError`, `SubXResult<T>` |

### Common Edit Targets

| Task | Files to Edit |
|---|---|
| Add/change CLI arguments | `src/cli/*_args.rs`, `src/cli/input_handler.rs`, `src/cli/validation.rs` |
| Add/change command logic | `src/commands/*_command.rs` |
| Change command routing | `src/commands/dispatcher.rs` |
| Add/change config keys | `src/config/mod.rs`, `service.rs`, `field_validator.rs`, `validator.rs` |
| Add AI provider | `src/services/ai/`, `src/core/factory.rs`, `src/config/` |
| Add subtitle format | `src/core/formats/`, register in `FormatManager::new()` |
| Add/change translation behavior | `src/cli/translate_args.rs`, `src/commands/translate_command.rs`, `src/core/translation/`, AI prompt helpers in `src/services/ai/` |

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
openspec/           OpenSpec changes, specs, and workflow config
scripts/            Build, quality, and CI shell scripts
  ├── quality_check.sh                   Full QA (lint, format, tests)
  ├── check_coverage.sh                  Coverage report (threshold 75%)
  ├── install.sh                         End-user binary installer
  ├── test_parallel_stability.sh         Parallel test isolation check
  └── test_unified_paths.sh              Path handling tests (⚠️ uses real AI API)
src/                Rust source code
  ├── cli/          CLI argument parsing and UI modules
  ├── commands/     Command implementations
  ├── config/       Configuration management (DI-based)
  ├── core/         Core processing engines
  ├── services/     External service integrations
  ├── error.rs      Error type definitions
  ├── lib.rs        Library entry point
  └── main.rs       Binary entry point
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

- Use `SubXError` variants from `src/error.rs` — never invent ad-hoc error
  types.
- Each variant maps to an exit code (1–6) via `exit_code()` and provides
  user-facing guidance via `user_friendly_message()`.

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
| `TestConfigService` | `src/config/test_service.rs` | Isolated config without filesystem I/O |
| `TestConfigBuilder` | `src/config/builder.rs` | Fluent builder for test configs |
| `TestEnvironmentProvider` | `src/config/environment.rs` | In-memory env vars for isolated testing |
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
in `src/config/service.rs` — check the implementation if a specific
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

1. `src/config/mod.rs` — struct field with serde attributes
2. `src/config/service.rs` — both `get_config_value()` and
   `set_config_value()`
3. `src/config/field_validator.rs` — field-level validation
4. `src/config/validator.rs` — section-level validation
5. `docs/configuration-guide.md` — user-facing documentation

## AI Provider System

Four providers are supported: `openai`, `openrouter`, `azure-openai`, and
`local`. The string `ollama` is accepted as an alias for `local` and is
normalized to the canonical value at config write time. All implement the
`AIProvider` trait (defined in `src/services/ai/mod.rs`) with two async
methods: `analyze_content()` and `verify_match()`.

Hosted providers (`openai`, `openrouter`, `azure-openai`) require
`https://` for any user-set `ai.base_url`; non-HTTPS values are rejected
by `validate_ai_config` with a hint pointing to `ai.provider = "local"`.
The `local` provider is endpoint-agnostic — it accepts loopback, LAN,
VPN/tailnet, and remote OpenAI-compatible endpoints over either `http://`
or `https://`.

To add a new provider, follow the step-by-step guide in
`docs/ai-provider-integration-guide.md`. Key touchpoints: create the client
in `src/services/ai/`, register in `src/core/factory.rs` →
`create_ai_provider()`, add validation in `src/config/field_validator.rs`
and `src/config/validator.rs`, and update both README files plus
`docs/configuration-guide.md`.

AI-provider changes should also review supporting modules in
`src/services/ai/`: `cache.rs`, `error_sanitizer.rs`, `prompts.rs`,
`retry.rs`, and `security.rs`.

## OpenSpec and Project Skills

This repository includes OpenSpec artifacts under `openspec/` and
project-scoped skills under `.github/skills/`. Use the OpenSpec skills for
proposal/change workflows when the user asks to propose, apply, verify, sync,
or archive changes. Use the `update-config-document` skill when configuration
items or configuration documentation need to be audited or refreshed.

## CI/CD Pipeline

CI runs on push/PR to `master` across Ubuntu, Windows, and macOS with Rust
stable. It executes `scripts/quality_check.sh -v -p ci --full`, runs
`cargo audit` for security, and enforces 75% coverage via
`scripts/check_coverage.sh`. Results are uploaded to Codecov.

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
