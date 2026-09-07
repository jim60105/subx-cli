# SubX Technical Architecture

SubX is a Rust CLI tool for automated subtitle processing. It uses a
modular architecture with dependency injection, supports multiple subtitle
formats, and integrates AI-powered file matching with local Voice Activity
Detection for audio synchronization.

The codebase is distributed across two crates: `subx-cli` — this
repository — is the Cargo workspace root and owns the CLI binary and library
facade (`subx-cli/src/cli/`, `subx-cli/src/commands/`, `subx-cli/src/lib.rs`,
`subx-cli/src/main.rs`), while `subx-core`
(<https://github.com/jim60105/subx-core>) is the reusable core library
(`config`, `core`, `error`, `services`), mounted here as a git submodule at
`subx-core/` and consumed as a workspace member. The module-by-module map
below reflects that two-crate layout.

## System Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   CLI Interface │───▶│   Core Engine    │───▶│  Output Handler │
│                 │    │                  │    │                 │
│ • Argument      │    │ • Match Engine   │    │ • File Writer   │
│   Parsing       │    │ • Format Engine  │    │ • Progress      │
│ • Command       │    │ • Sync Engine    │    │   Reporting     │
│   Routing       │    │ • Factory/DI     │    │ • Error Handler │
│ • Shell         │    │ • Parallel Proc. │    │ • Cache Mgmt.   │
│   Completion    │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │
                                ▼
        ┌───────────────────────────────────────────────────┐
        │                External Services                  │
        │                                                   │
        │  ┌─────────────┐  ┌─────────────┐  ┌───────────┐ │
        │  │ AI Provider │  │ Audio Proc. │  │ File      │ │
        │  │             │  │             │  │ System    │ │
        │  │ • OpenAI    │  │ • Symphonia │  │ • File IO │ │
        │  │ • OpenRoute │  │ • VAD       │  │ • Path    │ │
        │  │ • Azure     │  │ • Speech    │  │   Resolve │ │
        │  │ • Retry     │  │   Detection │  │ • Backup  │ │
        │  └─────────────┘  └─────────────┘  └───────────┘ │
        └───────────────────────────────────────────────────┘
```

## Crate Topology

`subx-cli` is the Cargo workspace root and the binary; `subx-core` is a
library mounted as a git submodule at `subx-core/` and consumed as a
workspace member. The dependency runs one way only — `subx-cli` depends on
`subx-core`, never the reverse — and `subx-core` may not name `subx_cli` in
code or in an intra-doc link (an upward reference cannot resolve, and
`subx-cli/tests/core_cli_boundary.rs` asserts the rule). `subx-core/Cargo.toml`
carries no `[workspace]` table, no workspace inheritance, and no
`[profile.*]` table, so the crate stays resolvable standalone from its own
repository. Cloning this repository requires `--recurse-submodules` (or a
`git submodule update --init --recursive` afterwards): a plain clone leaves an
empty `subx-core/` directory and a workspace that fails to resolve.

**Dependency allocation rule.** `clap`, `clap_complete`, `colored`, `tabled`
and `indicatif` are permanently `subx-cli`-only (crate-topology spec,
decision D8): a change placing any of them in `subx-core` is a design error,
not a dependency addition. The audio, VAD and archive stack
(`symphonia`, `voice_activity_detector`, `rubato`, `zip`, `tar`,
`sevenz-rust2`, …) belongs to `subx-core`. `archive-rar` and `slow-tests` are
declared in both manifests: core owns the real gate, and the CLI declares a
pass-through feature that forwards to it. The two `Cargo.toml` files are the
source of truth for everything else — this document deliberately does not
transcribe them; `cargo tree -p subx-core --depth 1` answers the question a
transcription would be trying to answer. The normative statement of the rule
lives in `openspec/specs/crate-topology/`.

### Release Profile

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

`[profile.*]` is declared only in the workspace root. A copy inside
`subx-core/Cargo.toml` would be ignored — non-root workspace members do not
get profile sections — with a warning on every build.

## The CLI Crate (`subx-cli`)

### CLI Layer (`subx-cli/src/cli/` and `subx-cli/src/commands/`)

The CLI layer handles argument parsing, command routing, and user-facing
output. It uses `clap` with the derive API for argument definitions and
delegates execution to command modules.

```rust
// subx-cli/src/cli/mod.rs
pub struct Cli {
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Match(MatchArgs),
    Convert(ConvertArgs),
    Sync(SyncArgs),
    DetectEncoding(DetectEncodingArgs),
    Config(ConfigArgs),
    Cache(CacheArgs),
    GenerateCompletion(GenerateCompletionArgs),
    Translate(TranslateArgs),
}
```

Each command has a corresponding module in `subx-cli/src/commands/` with an
`execute()` function that receives parsed arguments and a `&dyn
ConfigService` reference. The `dispatcher` module routes `Commands` variants
to their handlers. Shell completion generation for bash, zsh, fish, and
PowerShell is handled inline via `clap_complete`.

The UI layer depends on `indicatif` for progress bars, `colored` for
terminal colors, and `tabled` for tabular output formatting.

`subx-cli/src/cli/` holds only the clap argument structs and the terminal
presentation surface: the eight `*_args.rs` modules, `output.rs`,
`reporter.rs`, `table.rs`, `ui.rs`, and `error_ext.rs` — the binary-half
`SubXErrorExt` trait carrying `exit_code()` and
`user_friendly_message()` (the library-half taxonomy, `category()`,
`machine_code()` and `hint()`, stays on `SubXError` in `subx-core/src/error.rs`).
Input collection is no longer a CLI concern: `InputPathHandler` and
`CollectedFiles` live in `subx-core/src/core/input/`, and `subx-cli/src/cli/` keeps only
legacy re-exports for out-of-tree consumers.

## The Library Crate (`subx-core`)

### Configuration Module (`subx-core/src/config/`)

The configuration system is built around the `ConfigService` trait, which
abstracts all config access behind dependency injection. Production code
uses `ProductionConfigService` (file + env var backed), while tests use
`TestConfigService` (in-memory, no filesystem).

```rust
// subx-core/src/config/service.rs
pub trait ConfigService: Send + Sync {
    fn get_config(&self) -> Result<Config>;
    fn reload(&self) -> Result<()>;
    fn save_config(&self) -> Result<()>;
    fn save_config_to_file(&self, path: &Path) -> Result<()>;
    fn get_config_file_path(&self) -> Result<PathBuf>;
    fn get_config_value(&self, key: &str) -> Result<String>;
    fn set_config_value(&self, key: &str, value: &str) -> Result<()>;
    fn reset_to_defaults(&self) -> Result<()>;
}
```

The `Config` struct holds all configuration sections:

```rust
// subx-core/src/config/mod.rs
pub struct Config {
    pub ai: AIConfig,
    pub formats: FormatsConfig,
    pub sync: SyncConfig,
    pub general: GeneralConfig,
    pub parallel: ParallelConfig,
    pub translation: TranslationConfig,
    pub loaded_from: Option<PathBuf>,
}
```

`ProductionConfigService` merges three sources in priority order:
environment variables, user config file (`~/.config/subx/config.toml`), and
built-in defaults. It stores the result behind `Arc<RwLock<Config>>` for
thread-safe shared access.

The module also provides `TestConfigBuilder` (fluent builder for test
configs), `EnvironmentProvider` trait with `SystemEnvironmentProvider` and
`TestEnvironmentProvider` implementations, and validation logic split across
`validator.rs` (section-level) and `field_validator.rs` (key-value level).

### Core Engine (`subx-core/src/core/`)

#### Factory and Dependency Injection (`subx-core/src/core/factory.rs`)

`ComponentFactory` is the central wiring point. Constructed from a
`ConfigService`, it creates all major components with proper configuration
injection.

```rust
// subx-core/src/core/factory.rs
pub struct ComponentFactory {
    config: Config,
}

impl ComponentFactory {
    pub fn new(config_service: &dyn ConfigService) -> Result<Self>;
    pub fn config(&self) -> &Config;
    pub fn create_ai_provider(&self) -> Result<Box<dyn AIProvider>>;
    pub fn create_file_manager(&self) -> FileManager;
    pub fn create_match_engine(&self) -> Result<MatchEngine>;
    pub fn create_vad_sync_detector(&self) -> Result<VadSyncDetector>;
    pub fn create_vad_detector(&self) -> Result<LocalVadDetector>;
    pub fn create_audio_processor(&self) -> Result<VadAudioProcessor>;
}
```

The `create_ai_provider` method dispatches on the canonical `ai.provider`
value to construct the appropriate client: `OpenAIClient` for `"openai"`,
`OpenRouterClient` for `"openrouter"`, `AzureOpenAIClient` for
`"azure-openai"`, or `LocalLLMClient` for `"local"` (the alias `ollama`
normalises to `local` before dispatch). All four implement the `AIProvider`
trait.

#### Match Engine (`subx-core/src/core/matcher/`)

The match engine pairs subtitle files with video files using AI analysis.
The matching pipeline follows four stages: filename analysis, content
sampling, AI similarity scoring, and result caching.

```rust
// subx-core/src/core/matcher/engine.rs
pub struct MatchEngine {
    ai_client: Box<dyn AIProvider>,
    config: MatchConfig,
}

pub struct MatchConfig {
    pub confidence_threshold: f64,
    pub max_sample_length: usize,
    pub enable_content_analysis: bool,
    pub backup_enabled: bool,
    pub relocation_mode: FileRelocationMode,
    pub conflict_resolution: ConflictResolution,
    pub ai_model: String,
}
```

`FileDiscovery` walks directories and classifies files as media or
subtitle. `FileInfo` provides normalized name helpers that strip quality
tags (`1080p`, `x264`), brackets, and parentheses for cleaner matching.

#### Format Engine (`subx-core/src/core/formats/`)

Subtitle format handling uses the `SubtitleFormat` trait as a plugin
interface. Each format (SRT, ASS, VTT, SUB) implements parsing, detection,
and serialization.

```rust
// subx-core/src/core/formats/mod.rs
pub trait SubtitleFormat {
    fn format_name(&self) -> &'static str;
    fn file_extensions(&self) -> &'static [&'static str];
    fn detect(&self, content: &str) -> bool;
    fn parse(&self, content: &str) -> Result<Subtitle>;
    fn serialize(&self, subtitle: &Subtitle) -> Result<String>;
    fn supports_styling(&self) -> bool { false }
    fn uses_frame_timing(&self) -> bool { false }
}

pub struct Subtitle {
    pub entries: Vec<SubtitleEntry>,
    pub metadata: SubtitleMetadata,
}
```

`FormatManager` holds a registry of `Box<dyn SubtitleFormat>` and provides
auto-detection via `parse_auto()`, format lookup by name or extension, and
encoding-aware file reading. `FormatConverter` handles cross-format
conversion, and `encoding/` provides `EncodingDetector` for automatic
character encoding detection using `encoding_rs`.

Format handlers are `Send + Sync` by supertrait (`SubtitleFormat: Send + Sync`),
which makes `FormatManager`, `FormatConverter` and `TranslationEngine`
thread-safe; the guarantee is asserted at compile time in
`subx-core/src/core/mod.rs`.

#### Sync Engine (`subx-core/src/core/sync/`)

The sync engine computes timing offsets between audio and subtitles using
local Voice Activity Detection. Since v0.6.0, the architecture focuses
exclusively on local VAD processing — no network-based analysis is
performed.

```rust
// subx-core/src/core/sync/engine.rs
pub struct SyncEngine {
    config: SyncConfig,
    vad_detector: Option<VadSyncDetector>,
}

pub enum SyncMethod {
    Auto,
    LocalVad,
    Manual,
}

pub struct SyncResult {
    pub offset_seconds: f32,
    pub confidence: f32,
    pub method_used: SyncMethod,
    pub correlation_peak: f32,
    pub additional_info: Option<serde_json::Value>,
    pub processing_duration: Duration,
    pub warnings: Vec<String>,
}
```

The `LocalVad` method loads the audio file directly via `DirectAudioLoader`,
extracts the first channel, resamples to 16 kHz when the source sample rate
is not 8 kHz or 16 kHz, runs VAD analysis with dynamically calculated chunk
sizes, and compares detected speech segments against subtitle timestamps to
compute the optimal offset. The `Auto` method currently resolves to
`LocalVad`. The `Manual` method applies a user-specified offset directly.

Alongside the engine, `subx-core/src/core/sync/mod.rs` owns the parser-agnostic sync
pairing contract that used to be implicit behaviour of the CLI's `SyncArgs`:
`resolve_sync_pairing(&SyncPairingRequest) -> Result<SyncMode, SubXError>`
(single-pair auto-pairing and batch selection over `BatchRequest`),
`create_default_output_path`, and the `SYNC_VIDEO_EXTENSIONS` /
`SYNC_SUBTITLE_EXTENSIONS` lists. `SyncArgs::get_sync_mode` is a thin
adapter that fills a `SyncPairingRequest` from its clap fields.

#### Input Path Handling (`subx-core/src/core/input/`)

`InputPathHandler` merges positional, `-i`, and string path sources,
validates them, scans directories (flat or recursive) with symlink
skipping and case-insensitive extension filtering, and transparently
extracts archive inputs into `TempDir`s owned by the returned
`CollectedFiles` handle. It depends only on `std`, `log`, `tempfile`,
`crate::core::archive` and `crate::error` — never on `clap` or
`crate::cli`; each command's `*Args::get_input_handler` is a thin adapter.

#### Translation Engine (`subx-core/src/core/translation/`)

`TranslationEngine` implements the two-pass translation behind `subx-cli
translate`: a terminology pass (glossary entries parsed by
`parse_glossary_text` into a `TerminologyMap` that steers cue translation)
followed by a cue-translation pass that batches cues into
`TranslationRequest`s and validates the AI responses. It rides the existing
`subx-core/src/core/formats/` pipeline — parse, translate, re-apply — so
timing, cue ordering, cue counts and supported metadata survive the round
trip. Cue IDs are UUIDv7 (from `subx-core/src/core/uuidv7.rs`, re-exported
here for path compatibility), spaced ≥1 ms apart so batch logs and retries
carry strictly increasing, order-encoding IDs. The `[translation]`
configuration section lives in `subx-core/src/config/mod.rs` as
`TranslationConfig`.

#### Parallel Processing (`subx-core/src/core/parallel/`)

The parallel processing module implements a producer-consumer task
scheduler. `TaskScheduler` manages a worker pool, task queue, and load
balancer. The `ParallelConfig` section controls pool size (defaults to CPU
core count), queue capacity, overflow strategy, and priority-based ordering.

Workers specialize by operation type: format conversion, AI analysis, audio
processing, and file operations. The overflow strategy determines behavior
when the queue is full (`Block`, `DropOldest`, `Reject`, `Drop`, or
`Expand`).

#### File Manager (`subx-core/src/core/file_manager.rs`)

`FileManager` provides batch file operations with backup support. It
records creations and moves so that `rollback()` can undo them if a later
operation fails. Removed files are backed up before deletion when
`backup_enabled` is true. Rollback restores recorded creations and moves but
cannot recover removed files.

#### Reporting Seam (`subx-core/src/core/report/`)

Core engines and service clients never print to the terminal and never read
the CLI's output mode or `--quiet` flag. They report through the
transport-agnostic `core::report::Reporter` seam — four channels
(`diagnostic`, `warn`, `ai_usage`, `progress`). Every attachable type
(`MatchEngine`, `TranslationEngine`, `SyncEngine`, `ComponentFactory`,
`FileManager`, `WorkerPool`, the four AI clients) defaults to
`NoopReporter` — library consumers see silence — and accepts a sink via the
`with_reporter` builder. The CLI owns the only terminal implementation,
`cli::reporter::TerminalReporter`, which applies the JSON/`--quiet`
suppression matrix and is attached at command boundaries. Layering rule:
`subx-core` — its core and service modules alike — must not reference the
`subx-cli` crate at all (the dependency is one-way and an upward reference
cannot resolve); enforced by `tests/core_cli_boundary.rs`.

Batch progress and cooperative cancellation cross the crate boundary
through the same seam: engines stream structured
`core::report::ProgressEvent::{Started, Advanced, Finished}` values and
poll `Reporter::cancelled()`, and no engine knows what transport renders
or answers them. Constructors stay compatible with pre-seam code:
`MatchEngine::new`, `SyncEngine::new`, `TranslationEngine::new` and
`ComponentFactory::new` keep their signatures and default to
`NoopReporter`; attaching a sink is the opt-in `with_reporter` step.

### External Services

#### AI Service (`subx-core/src/services/ai/`)

The AI service layer provides four provider implementations behind the
`AIProvider` trait:

```rust
// subx-core/src/services/ai/mod.rs
#[async_trait]
pub trait AIProvider: Send + Sync {
    async fn analyze_content(&self, request: AnalysisRequest) -> Result<MatchResult>;
    async fn verify_match(&self, request: VerificationRequest) -> Result<ConfidenceScore>;
}
```

Provider clients: `OpenAIClient` (`openai.rs`), `OpenRouterClient`
(`openrouter.rs`), `AzureOpenAIClient` (`azure_openai.rs`), and
`LocalLLMClient` (`local.rs`, the OpenAI-compatible local-runtime client).
All four use shared infrastructure from the module:

- `prompts.rs` — `PromptBuilder` and `ResponseParser` traits with base
  implementations for constructing analysis prompts and parsing AI responses.
  The match prompt uses an XML-tagged structure (`<role>`,
  `<instructions>`, `<video_files>`, `<subtitle_files>`,
  `<content_samples>`, `<output_schema>`, `<example>`) per Anthropic's
  Claude prompt-engineering guidelines, and asks the model to return
  optional `language` and `target_filename_suffix` fields per match. The
  matcher's `apply_unique_target_paths` allocator then enforces globally
  unique final target paths across the entire batch.
- `retry.rs` — `RetryConfig` struct and `retry_with_backoff()` async
  function for exponential backoff retry logic
- `cache.rs` — `AICache` for in-memory TTL caching of analysis results

Each provider reports token accounting by calling
`crate::core::report::Reporter::ai_usage` on its attached reporter after
receiving a response, passing a `core::report::AiUsage` value (legacy alias
`AiUsageStats`) — the seam is documented under the Reporting Seam subsection
of the Core Engine section; a provider never decides whether to print.

#### VAD Service (`subx-core/src/services/vad/`)

The VAD module handles local voice activity detection using the
`voice_activity_detector` crate and `symphonia` for audio decoding.

```rust
pub struct DirectAudioLoader { /* loads audio files via symphonia */ }
pub struct VadAudioProcessor { /* preprocesses audio for VAD */ }
pub struct LocalVadDetector { /* runs VAD analysis */ }
pub struct VadSyncDetector { /* computes sync offset from VAD results */ }
```

The processing pipeline:

```
Audio file → DirectAudioLoader → first channel extraction
                                       ↓
                               Resample to 16 kHz if source ≠ 8/16 kHz
                                       ↓
                               Dynamic chunk_size calculation → VAD analysis
```

This design processes only the first channel (reducing computation),
resamples only when necessary (preserving native quality for 8/16 kHz
sources), and dynamically computes the chunk size based on the actual sample
rate.

## Data Flow

### Match Workflow

```
Input: Media folder
    │
    ▼
┌─────────────────┐
│ File Discovery  │ ──▶ Scan for video and subtitle files
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Cache Check     │ ──▶ Look up cached results
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ AI Analysis     │ ──▶ Call AI provider for matching
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Confidence      │ ──▶ Evaluate match confidence scores
│ Evaluation      │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Dry-run         │ ──▶ Preview results (if --dry-run)
│ Preview         │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ File Rename     │ ──▶ Rename/copy/move files (with backup)
└─────────────────┘
```

Subtitle files are renamed to match the video's base name, dropping the
video file extension. For example, `movie.mkv` produces `movie.tc.srt`,
not `movie.mkv.tc.srt`.

### Sync Workflow

```
Input: Video + Subtitle
    │
    ▼
┌─────────────────┐
│ Audio Loading   │ ──▶ Load audio via DirectAudioLoader
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ VAD Analysis    │ ──▶ Detect speech segments
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Offset          │ ──▶ Compare speech timing with subtitles
│ Calculation     │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Subtitle        │ ──▶ Apply timing correction
│ Adjustment      │
└─────────────────┘
```

### Convert Workflow

```
Input: Source subtitle file
    │
    ▼
┌─────────────────┐
│ Encoding        │ ──▶ Auto-detect character encoding
│ Detection       │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Format          │ ──▶ Parse source format
│ Parsing         │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Content         │ ──▶ Transform and convert content
│ Transformation  │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Output          │ ──▶ Write target format file
│ Generation      │
└─────────────────┘
```

## Error Handling

All errors flow through `SubXError`, defined in `subx-core/src/error.rs` using
`thiserror`. The core crate owns the typed variants, their `Display`
rendering and the machine-readable contract (`category()`, `machine_code()`,
`hint()`) as inherent methods; the exit-code mapping (1–6) and the
user-friendly terminal prose — `exit_code()` and `user_friendly_message()` —
are methods of the `SubXErrorExt` extension trait in the CLI half
(`subx_cli::cli::error_ext::SubXErrorExt`; also re-exported as
`subx_cli::cli::SubXErrorExt`).

One variant defies the split: `OutputModeUnsupported` stays in the core
enum even though only the CLI constructs it, so `category()` and
`machine_code()` keep their wildcard-free exhaustive matches. Its
`category()` is the generic `"command_execution"` while its `machine_code()`
is the specific `"E_OUTPUT_MODE_UNSUPPORTED"` — a deliberate asymmetry
locked by the `error-handling` capability spec.

```rust
#[derive(thiserror::Error, Debug)]
pub enum SubXError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),                        // exit code 1

    #[error("Configuration error: {message}")]
    Config { message: String },                        // exit code 2

    #[error("AI service error: {0}")]
    AiService(String),                                 // exit code 3
    #[error("API error: {message}")]
    Api { message: String, source: ApiErrorSource },   // exit code 3

    #[error("Subtitle format error [{format}]: {message}")]
    SubtitleFormat { format: String, message: String }, // exit code 4

    #[error("Audio processing error: {message}")]
    AudioProcessing { message: String },               // exit code 5

    #[error("File matching error: {message}")]
    FileMatching { message: String },                  // exit code 6

    // Additional variants: FileAlreadyExists, FileNotFound,
    // InvalidFileName, FileOperationFailed, CommandExecution,
    // NoInputSpecified, InvalidPath, PathNotFound, etc.
}
```

Constructor helpers like `SubXError::config()`, `SubXError::ai_service()`,
and `SubXError::audio_processing()` simplify error creation. `From` impls
automatically convert `std::io::Error`, `reqwest::Error`,
`walkdir::Error`, `symphonia` errors, `config::ConfigError`, and
`serde_json::Error` into the appropriate `SubXError` variant.

## Performance Design

### Concurrency

Batch processing uses `tokio::spawn` with `Arc<Semaphore>` to limit
concurrent operations. The semaphore permit count defaults to
`num_cpus::get().min(8)`, and `futures::future::try_join_all` collects
results.

```rust
pub async fn process_batch(files: Vec<MediaPair>) -> Result<Vec<ProcessResult>> {
    let semaphore = Arc::new(Semaphore::new(num_cpus::get().min(8)));
    let tasks: Vec<_> = files.into_iter().map(|file| {
        let sem = semaphore.clone();
        tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            process_single_file(file).await
        })
    }).collect();
    futures::future::try_join_all(tasks).await
}
```

### Memory Efficiency

Large files are read with streaming where possible. Audio processing uses
only the first channel to reduce memory usage. The AI cache stores analysis
results to avoid redundant API calls, and the parallel scheduler dynamically
adjusts concurrency based on system resources.

### API Cost Control

Content sampling limits the amount of text sent to AI providers
(`max_sample_length`). Batch analysis combines multiple files into single
requests where the provider supports it. Exponential backoff retry avoids
flooding the API on transient failures, and persistent caching prevents
re-analysis of previously matched files within the same session.

## Testing Strategy

Unit tests live inline in source files as `#[cfg(test)] mod tests`. They
use `TestConfigService` for configuration and `mockall` for trait mocking.
Every test is parallel-safe — no global state mutation.

Integration tests follow the ownership rule: a test belongs to the crate
whose code it drives, not the crate it imports. Tests that spawn the binary
or exercise the CLI surface live in `subx-cli/tests/`; tests that drive
engines, config or services live in `subx-core/tests/` even when they import
both crates. Shared fixtures live in `subx-core/src/test_support/` behind
the `test-support` feature, which `subx-cli` enables only through
`[dev-dependencies]` — never `--features`. AI interactions
use `wiremock` via `MockOpenAITestHelper` (in `subx_core::test_support`), which stubs
HTTP endpoints and verifies request expectations. File-system tests use
`tempfile::TempDir` with RAII cleanup.

Two environment contracts keep the suites relocatable: every fixture or
asset read resolves from `env!("CARGO_MANIFEST_DIR")` (never a path relative
to the process working directory, which differs between the two crates'
`cargo test` invocations), and CLI tests spawn the binary through
`env!("CARGO_BIN_EXE_subx-cli")` rather than a hand-built path. Nested test
directories need a harness: Cargo only auto-discovers top-level
`tests/*.rs`, so every `.rs` file under a `tests/` subdirectory must be
named by exactly one top-level `#[path = "..."] mod` shim —
`tests/core_cli_boundary.rs` fails the build on any orphan or duplicate
(`subx-core/tests/` is flat and therefore has none).

Performance benchmarks live in `subx-core/benches/` and use Criterion. Each
benchmark function creates a `tokio::runtime::Runtime` for async operations
and wraps inputs with `std::hint::black_box`.

The testing toolchain splits with the crates: `assert_cmd` and `predicates`
(plus `regex` for output validation) are `subx-cli`'s; `mockall`,
`wiremock`, `hound`, `criterion` and `pretty_assertions` are core's.
Parameterized cases are plain `#[test]` functions over helper loops — there
is no `rstest`/`test-case` dependency in either manifest.

## Build and Release

### CI/CD Pipeline

GitHub Actions runs on every push and pull request to `master`. The
`build-test-audit-coverage` workflow tests across Ubuntu, Windows, and macOS
with Rust stable. It runs `scripts/quality_check.sh`, `cargo audit` for
dependency security, and the coverage check — one instrumented
`cargo llvm-cov nextest` run over the whole workspace
(`scripts/check_coverage.sh` on Linux/macOS, the PowerShell port
`scripts/check_coverage.ps1` on Windows), with per-crate line-coverage
floors derived as `floor(measured − 3)` from the split-suite baseline:
workspace 75%, `subx-core` 90%, `subx-cli` 82% (measured 91.77 / 93.54 /
85.93). Floors ratchet upward with coverage and are only lowered through a
proposal; the plumbing landed with 2.0.0: `--threshold-core` /
`--threshold-cli` flags and `COVERAGE_THRESHOLD_CORE` /
`COVERAGE_THRESHOLD_CLI` environment variables in both scripts, and the env
values on every CI coverage matrix entry. Reporting exclusions are a single
`--ignore-filename-regex` passed to both report invocations — the
`.llvm-cov.toml` files that used to sit in both repositories were deleted
with 2.0.0 (nothing ever read them). A `submodule-pointer` job asserts the
version/requirement agreement, the gitlink's reachability from `subx-core`'s
`master`, and `.gitmodules`' `branch = master` referent (kept fresh by
Dependabot's `gitsubmodules` entry). `subx-core` also runs its own CI in its
own repository — three-OS test job, own-lockfile audit, and an ungated
standalone coverage upload (workspace-derived floors do not transfer to a
standalone run). Results are uploaded to Codecov.

The `release` workflow triggers on `v*` tags. It cross-compiles for the
matrix targets, creates a GitHub Release with notes extracted from
`CHANGELOG.md`, and publishes **`subx-cli` alone** with
`cargo publish -p subx-cli`, after asserting submodule cleanliness, gitlink
ancestry from `subx-core`'s `master`, the absence of any `--allow-dirty`
publish flag, a published `subx-core` version satisfying the caret
requirement in `Cargo.toml` (probed against the sparse crates.io index —
the ordering precondition below), tag-versus-manifest version agreement, an
already-published check for `subx-cli` itself, and a
`cargo publish -p subx-cli --dry-run`. The matrix is five targets:
`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
`x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, and
`aarch64-apple-darwin` — there are no musl artifacts.

`subx-core` publishes from **its own repository's** `v*` tag
(`subx-core/.github/workflows/release.yml`): GitHub Release notes extracted
from the core repository's `CHANGELOG.md`, then
`cargo publish -p subx-core` after the same shape of guards over its
standalone tree. `subx-cli` is not an umbrella project — the library has a
second consumer (the Tauri GUI at `jim60105/subx`) — so the two tags form an
ordered operation: tag the library repository first; the CLI's index probe
fails any CLI tag pushed before its `subx-core` dependency is on the
registry.

### Distribution

The two crates are designed for two install stories, and the release
topology now matches: each crate publishes to crates.io from its own
repository's `v*` tag. No release tag has been cut in either repository
since the split, so until each tag is pushed and its publish job runs,
crates.io carries only the legacy single-crate `subx-cli` 1.x releases,
`subx-core` has no registry entry, and neither docs.rs site is live;
meanwhile `subx-core` is consumable from its git URL. Once each tag is cut:
`subx-cli` is the binary — pre-compiled binaries from GitHub Releases,
`scripts/install.sh` automating download and installation on Linux and
macOS, and `cargo install subx-cli` installing from crates.io. `subx-core`
is never installed — a consumer wanting the processing engine depends on
`subx-core = "1"` as a library and never on `subx-cli`, whose library
surface exists only as compatibility re-exports — and documentation lives
on two sites: `docs.rs/subx-cli` for the CLI facade, `docs.rs/subx-core`
for the library.

## System Requirements

SubX runs on Linux (x86_64, aarch64), Windows (x86_64), and macOS (x86_64, ARM64).
Recommended: 4 GB RAM, 100 MB disk space (excluding cache). An AI provider
API key is required for the `match` command. FFmpeg is optional — Symphonia
handles most audio formats natively.
