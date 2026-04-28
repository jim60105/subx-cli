# SubX Technical Architecture

SubX is a Rust CLI tool for automated subtitle processing. It uses a
modular architecture with dependency injection, supports multiple subtitle
formats, and integrates AI-powered file matching with local Voice Activity
Detection for audio synchronization.

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

### Configuration Module (`src/config/`)

The configuration system is built around the `ConfigService` trait, which
abstracts all config access behind dependency injection. Production code
uses `ProductionConfigService` (file + env var backed), while tests use
`TestConfigService` (in-memory, no filesystem).

```rust
// src/config/service.rs
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
// src/config/mod.rs
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

### Core Engine (`src/core/`)

#### Factory and Dependency Injection (`src/core/factory.rs`)

`ComponentFactory` is the central wiring point. Constructed from a
`ConfigService`, it creates all major components with proper configuration
injection.

```rust
// src/core/factory.rs
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

#### Match Engine (`src/core/matcher/`)

The match engine pairs subtitle files with video files using AI analysis.
The matching pipeline follows four stages: filename analysis, content
sampling, AI similarity scoring, and result caching.

```rust
// src/core/matcher/engine.rs
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

#### Format Engine (`src/core/formats/`)

Subtitle format handling uses the `SubtitleFormat` trait as a plugin
interface. Each format (SRT, ASS, VTT, SUB) implements parsing, detection,
and serialization.

```rust
// src/core/formats/mod.rs
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

#### Sync Engine (`src/core/sync/`)

The sync engine computes timing offsets between audio and subtitles using
local Voice Activity Detection. Since v0.6.0, the architecture focuses
exclusively on local VAD processing — no network-based analysis is
performed.

```rust
// src/core/sync/engine.rs
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

#### Parallel Processing (`src/core/parallel/`)

The parallel processing module implements a producer-consumer task
scheduler. `TaskScheduler` manages a worker pool, task queue, and load
balancer. The `ParallelConfig` section controls pool size (defaults to CPU
core count), queue capacity, overflow strategy, and priority-based ordering.

Workers specialize by operation type: format conversion, AI analysis, audio
processing, and file operations. The overflow strategy determines behavior
when the queue is full (`Block`, `DropOldest`, `Reject`, `Drop`, or
`Expand`).

#### File Manager (`src/core/file_manager.rs`)

`FileManager` provides batch file operations with backup support. It
records creations and moves so that `rollback()` can undo them if a later
operation fails. Removed files are backed up before deletion when
`backup_enabled` is true. Rollback restores recorded creations and moves but
cannot recover removed files.

### External Services

#### AI Service (`src/services/ai/`)

The AI service layer provides three provider implementations behind the
`AIProvider` trait:

```rust
// src/services/ai/mod.rs
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

Each provider calls `display_ai_usage` (from `src/cli/`) after receiving a
response, reporting token counts via `AiUsageStats`.

#### VAD Service (`src/services/vad/`)

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

All errors flow through `SubXError`, defined in `src/error.rs` using
`thiserror`. Each variant maps to a specific exit code and provides a
user-friendly message.

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

```toml
# CLI framework
clap = { version = "4.5.40", features = ["derive", "cargo"] }
clap_complete = "4.5.54"

# Async runtime
tokio = { version = "1.0", features = ["full"] }

# HTTP client
reqwest = { version = "0.13", features = ["json", "stream", "rustls"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "1"

# Audio processing
symphonia = { version = "0.5", features = ["all"] }

# VAD
voice_activity_detector = { version = "0.2.1", features = ["async"] }
hound = "3.5"
rubato = "0.16.2"

# Subtitle processing
regex = "1.0"
encoding_rs = "0.8"
walkdir = "2.3"

# Configuration
config = "0.15"
dirs = "6.0"

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Concurrency
futures = "0.3"
async-trait = "0.1"
num_cpus = "1.0"

# UI
indicatif = "0.18"
colored = "3.0"
tabled = "0.20"
dialoguer = "0.11"

# Utilities
uuid = { version = "1.3", features = ["v7"] }
url = "2"
notify = "8.0"
md5 = "0.7"
log = "0.4"
env_logger = "0.11"
once_cell = "1.19"

# Platform-specific
[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["winuser"] }

[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

### Dev Dependencies

```toml
tokio-test = "0.4"
assert_cmd = "2.0"
predicates = "3.0"
tempfile = "3.10"
mockall = "0.13"
rstest = "0.25"
test-case = "3.0"
wiremock = "0.6"
criterion = { version = "0.6.0", features = ["html_reports"] }
```

## Testing Strategy

Unit tests live inline in source files as `#[cfg(test)] mod tests`. They
use `TestConfigService` for configuration and `mockall` for trait mocking.
Every test is parallel-safe — no global state mutation.

Integration tests in `tests/` cover full command workflows. AI interactions
use `wiremock` via `MockOpenAITestHelper` (in `tests/common/`), which stubs
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
dependency security, and the coverage check with a 75% line coverage
threshold — `scripts/check_coverage.sh` on Linux/macOS and the PowerShell
port `scripts/check_coverage.ps1` on Windows. Results are uploaded to
Codecov.

The `release` workflow triggers on `v*` tags. It cross-compiles for four
targets (Linux x86_64, Windows x86_64, macOS x86_64, macOS ARM64), creates
a GitHub Release with notes extracted from `CHANGELOG.md`, and publishes to
crates.io.

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
