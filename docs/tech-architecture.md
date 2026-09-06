# SubX Technical Architecture

SubX is a Rust CLI tool for automated subtitle processing. It uses a
modular architecture with dependency injection, supports multiple subtitle
formats, and integrates AI-powered file matching with local Voice Activity
Detection for audio synchronization.

The codebase is being distributed across two crates: `subx-cli` — this
repository — is the Cargo workspace root and owns the CLI binary and library
facade (`src/cli/`, `src/commands/`, `src/lib.rs`, `src/main.rs`), while
`subx-core` (<https://github.com/jim60105/subx-core>) is the
reusable core library (`config`, `core`, `error`, `services`), mounted here
as a git submodule at `subx-core/` and consumed as a workspace member. The
module-by-module map below reflects that two-crate layout.

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

## Core Modules

### CLI Layer (`src/cli/` and `src/commands/`)

The CLI layer handles argument parsing, command routing, and user-facing
output. It uses `clap` with the derive API for argument definitions and
delegates execution to command modules.

```rust
// src/cli/mod.rs
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
}
```

Each command has a corresponding module in `src/commands/` with an
`execute()` function that receives parsed arguments and a `&dyn
ConfigService` reference. The `dispatcher` module routes `Commands` variants
to their handlers. Shell completion generation for bash, zsh, fish, and
PowerShell is handled inline via `clap_complete`.

The UI layer depends on `indicatif` for progress bars, `colored` for
terminal colors, `dialoguer` for interactive prompts, and `tabled` for
tabular output formatting.

`src/cli/` holds only the clap argument structs and the terminal
presentation surface: the eight `*_args.rs` modules, `output.rs`,
`reporter.rs`, `table.rs`, `ui.rs`, and `error_ext.rs` — the binary-half
`SubXErrorExt` trait carrying `exit_code()` and
`user_friendly_message()` (the library-half taxonomy, `category()`,
`machine_code()` and `hint()`, stays on `SubXError` in `subx-core/src/error.rs`).
Input collection is no longer a CLI concern: `InputPathHandler` and
`CollectedFiles` live in `subx-core/src/core/input/`, and `src/cli/` keeps only
legacy re-exports for out-of-tree consumers.

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

The `create_ai_provider` method dispatches on `ai.provider` to construct
the appropriate client: `OpenAIClient` for `"openai"`, `OpenRouterClient`
for `"openrouter"`, or `AzureOpenAIClient` for `"azure-openai"`. All three
implement the `AIProvider` trait.

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
`subx-core/src/core/mod.rs`. *(C3: this sentence may relocate to `subx-core`'s
own documentation when that file is rewritten for the two-crate world.)*

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

#### Input Collection (`subx-core/src/core/input/`)

`InputPathHandler` merges positional, `-i`, and string path sources,
validates them, scans directories (flat or recursive) with symlink
skipping and case-insensitive extension filtering, and transparently
extracts archive inputs into `TempDir`s owned by the returned
`CollectedFiles` handle. It depends only on `std`, `log`, `tempfile`,
`crate::core::archive` and `crate::error` — never on `clap` or
`crate::cli`; each command's `*Args::get_input_handler` is a thin adapter.

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

### External Services

#### AI Service (`subx-core/src/services/ai/`)

The AI service layer provides three provider implementations behind the
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
(`openrouter.rs`), and `AzureOpenAIClient` (`azure_openai.rs`). All three
use shared infrastructure from the module:

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
`AiUsageStats`).

##### Reporting seam (`subx-core/src/core/report/`)

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
`hint()`); the exit-code mapping (1–6) and the user-friendly terminal prose
are supplied by the `SubXErrorExt` trait in the CLI half
(`subx_cli::cli::SubXErrorExt`).

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

## Dependencies

### Runtime Dependencies

Dependencies are declared across two manifests. `subx-cli/Cargo.toml` keeps
only what the CLI facade itself uses; everything the engines and services
need belongs to `subx-core/Cargo.toml` (`subx-core = { path = "subx-core",
version = "1.0" }` is how the CLI reaches it, and the crate must stay
buildable standalone — depending on the CLI's feature graph is a defect, not
a shortcut).

```toml
# ── subx-cli/Cargo.toml ──
# CLI framework
clap = { version = "4.5.40", features = ["derive", "cargo"] }
clap_complete = "4.5.54"

# Async runtime
tokio = { version = "1.0", features = [
    "rt-multi-thread",
    "macros",
    "time",
] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "1"

# Config-directory resolution
dirs = "6.0"

# UI
indicatif = "0.18"
colored = "3.0"
tabled = "0.20"

# Logging
log = "0.4"
env_logger = "0.11"

# ── subx-core/Cargo.toml ──
# Async runtime
tokio = { version = "1.0", features = ["rt", "sync", "time", "fs", "macros"] }

# HTTP client
reqwest = { version = "0.13", features = ["json", "stream", "rustls"] }

# Audio processing
symphonia = { version = "0.5", default-features = false, features = [
    "isomp4",
    "mkv",
    "ogg",
    "wav",
    "aac",
    "flac",
    "mp3",
    "pcm",
    "vorbis",
] }

# VAD
voice_activity_detector = { version = "0.2.1", features = ["async"] }
rubato = "2.0"
audioadapter-buffers = "3"

# Subtitle processing
regex = "1.0"
encoding_rs = "0.8"
walkdir = "2.3"

# Configuration
config = "0.15"
dirs = "6.0"
num_cpus = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Concurrency
futures = "0.3"
async-trait = "0.1"

# Utilities
uuid = { version = "1.3", features = ["v7"] }
url = "2"
log = "0.4"

# Archive extraction
zip = "8"
unrar = { version = "0.5", optional = true }
tar = "0.4"
flate2 = "1"
tempfile = "3.10"
sevenz-rust2 = "0.22.2"
```

### Dev Dependencies

```toml
# WAV fixture generation for VAD and sync integration tests
hound = "3.5"

# testing frameworks and utilities
assert_cmd = "2.0"
predicates = "3.0"

# mock and test case generation
mockall = "0.13"
rstest = "0.25"
test-case = "3.0"
regex = "1.10" # For test output validation

# coverage and HTTP mocking
wiremock = "0.6"
criterion = { version = "0.6.0", features = ["html_reports"] }

# diff-friendly assertions for fixture round-trip harness
pretty_assertions = "1.4"
```

## Testing Strategy

Unit tests live inline in source files as `#[cfg(test)] mod tests`. They
use `TestConfigService` for configuration and `mockall` for trait mocking.
Every test is parallel-safe — no global state mutation.

Integration tests are split across the two crates by what each drives: tests
that spawn the binary or exercise the CLI surface live in `subx-cli/tests/`,
while the 40 core-bound files live in `subx-core/tests/` and shared fixtures
in `subx-core/src/test_support/` (behind the `test-support` feature, enabled
only via `[dev-dependencies]`). AI interactions
use `wiremock` via `MockOpenAITestHelper` (in `subx_core::test_support`), which stubs
HTTP endpoints and verifies request expectations. File-system tests use
`tempfile::TempDir` with RAII cleanup.

Performance benchmarks in `benches/` use Criterion. Each benchmark function
creates a `tokio::runtime::Runtime` for async operations and wraps inputs
with `std::hint::black_box`.

The testing toolchain: `rstest` for parameterized tests, `test-case` for
data-driven scenarios, `assert_cmd` for CLI integration tests, and
`wiremock` for HTTP mocking.

## Build and Release

### Release Profile

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

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
`main`, and `.gitmodules`' `branch = main` referent (kept fresh by
Dependabot's `gitsubmodules` entry). `subx-core` also runs its own CI in its
own repository — three-OS test job, own-lockfile audit, and an ungated
standalone coverage upload (workspace-derived floors do not transfer to a
standalone run). Results are uploaded to Codecov.

The `release` workflow triggers on `v*` tags. It cross-compiles for the
matrix targets, creates a GitHub Release with notes extracted from
`CHANGELOG.md`, and publishes **both crates** to crates.io from one
`cargo publish --workspace` run, after asserting Cargo ≥ 1.90, submodule
cleanliness, a `--workspace --dry-run`, and the registry's current state
(already-published members are excluded; an already-published tagged
`subx-cli` version fails the job).

### Distribution

Pre-compiled binaries are available from GitHub Releases. The
`scripts/install.sh` script automates download and installation on Linux and
macOS. The crate is also published to crates.io for `cargo install
subx-cli`.

## System Requirements

SubX runs on Linux (x86_64), Windows (x86_64), and macOS (x86_64, ARM64).
Recommended: 4 GB RAM, 100 MB disk space (excluding cache). An AI provider
API key is required for the `match` command. FFmpeg is optional — Symphonia
handles most audio formats natively.
