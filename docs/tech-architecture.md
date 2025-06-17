# SubX 技術架構文件

## 專案概覽

SubX 是一個基於 Rust 開發的 CLI 工具，專注於智慧字幕處理。採用模組化設計，支援多種字幕格式和 AI 驅動的匹配算法。該專案使用依賴注入模式進行配置管理，並實現了先進的音訊處理和並行處理能力。

## 整體架構

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
        │  ┌─────────────┐  ┌─────────────┐  ┌──────────── │
        │  │ OpenAI API  │  │ Audio Proc. │  │ File System │
        │  │             │  │             │  │             │
        │  │ • GPT-4o    │  │ • Symphonia │  │ • File I/O  │
        │  │ • Text      │  │ • VAD       │  │ • Path      │
        │  │   Analysis  │  │ • Dialogue  │  │   Handling  │
        │  │ • Retry     │  │   Detection │  │ • Rollback  │
        │  │   Logic     │  │             │  │   Support   │
        │  └─────────────┘  └─────────────┘  └─────────────┘
        └───────────────────────────────────────────────────┘
```

## 核心模組設計

### 1. CLI Layer (`src/cli/` and `src/commands/`)

**責任**: 用戶界面、命令解析以及命令執行邏輯。

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
    DetectEncoding(DetectEncodingArgs), // New command
    Config(ConfigArgs),
    Cache(CacheArgs),
    GenerateCompletion(GenerateCompletionArgs),
}
```

**關鍵組件**:
- `clap` - 命令行參數解析，支援自動完成
- `clap_complete` - Shell 完成腳本生成
- `indicatif` - 進度條顯示
- `colored` - 彩色輸出
- `dialoguer` - 互動式提示
- `tabled` - 表格輸出格式化

**Command Handlers (`src/commands/`)**:
- 此目錄包含每個 CLI 命令的邏輯，包括新增的 `detect_encoding_command.rs`
- 每個命令模組從 `src/cli/` 層取得已解析的參數，並透過與 `Core Engine` 和 `Services Layer` 互動來協調操作
- 支援乾燥執行模式和快取管理

### 2. Configuration Module (`src/config/`)

**責任**: 使用依賴注入模式管理應用程式的組態設定。

**架構設計**:
- **Legacy Configuration** (`config_legacy.rs`) - 配置資料結構定義
- **Service Layer** (`service.rs`) - 配置服務介面和實作
- **Builder Pattern** (`builder.rs`) - 測試配置建構器
- **Environment Provider** (`environment.rs`) - 環境變數提供者
- **Test Service** (`test_service.rs`) - 測試專用配置服務
- **Validator** (`validator.rs`) - 配置驗證邏輯

```rust
// src/config/mod.rs
pub trait ConfigService {
    fn config(&self) -> &Config;
    fn ai_config(&self) -> &AIConfig;
    fn formats_config(&self) -> &FormatsConfig;
    // ... other config getters
}

pub struct ProductionConfigService {
    config: Config,
}

pub struct TestConfigService {
    config: Config,
}
```

**配置結構**:
```rust
// src/config/config_legacy.rs
pub struct Config {
    pub ai: AIConfig,
    pub formats: FormatsConfig,
    pub sync: SyncConfig,
    pub general: GeneralConfig,
    pub parallel: ParallelConfig,
    pub loaded_from: Option<PathBuf>,
}
```

### 3. Core Engine (`src/core/`)

#### 3.1 Factory and Dependency Injection (`src/core/factory.rs` & `src/core/services.rs`)

**責任**: 元件建立和依賴注入管理

```rust
// src/core/factory.rs
pub struct ComponentFactory {
    config: Config,
}

impl ComponentFactory {
    /// Create an AI provider based on AI configuration.
    ///
    /// Currently supports OpenAI provider with API key, model, temperature,
    /// max_tokens, retry logic, and optional custom base URL configuration.
    pub fn create_ai_provider(&self) -> Result<Box<dyn AIProvider>>;
}

// src/core/services.rs
pub struct ServiceContainer {
    config_service: Arc<dyn ConfigService>,
    component_factory: ComponentFactory,
}
```

#### 3.2 Match Engine (`src/core/matcher/`)

**責任**: AI 驅動的檔案匹配邏輯

```rust
// src/core/matcher/mod.rs
pub struct MatchEngine {
    ai_client: Box<dyn AIProvider>,
    config: MatchConfig,
}

pub trait AIProvider {
    async fn analyze_content(&self, request: AnalysisRequest) -> Result<MatchResult>;
    async fn verify_match(&self, verification: VerificationRequest) -> Result<ConfidenceScore>;
}
```

**匹配算法**:
1. **Filename Analysis** - 檔名模式解析
2. **Content Sampling** - 字幕內容採樣
3. **AI Similarity** - 語義相似度分析
4. **Cache Integration** - 結果快取和重用

#### 3.3 Format Engine (`src/core/formats/`)

**責任**: 字幕格式解析和轉換

```rust
// src/core/formats/mod.rs
pub trait SubtitleFormat {
    fn parse(&self, content: &str) -> Result<Subtitle>;
    fn serialize(&self, subtitle: &Subtitle) -> Result<String>;
    fn detect(content: &str) -> bool;
}

pub struct Subtitle {
    pub entries: Vec<SubtitleEntry>,
    pub metadata: SubtitleMetadata,
}
```

**支援格式**:
- **SRT Parser** (`srt.rs`) - SubRip 格式
- **ASS Parser** (`ass.rs`) - Advanced SSA
- **VTT Parser** (`vtt.rs`) - WebVTT
- **SUB Parser** (`sub.rs`) - 多種 SUB 變體
- **Encoding Detection** (`encoding/`) - 自動編碼檢測
- **Style Management** (`styling.rs`) - 樣式處理
- **Format Conversion** (`converter.rs`) - 格式轉換邏輯
- **Content Transformers** (`transformers.rs`) - 內容轉換器

#### 3.4 Sync Engine (`src/core/sync/`)

**責任**: 多方法音訊字幕同步與智慧方法選擇

SubX v0.6.0 引入了簡潔高效的同步引擎，專注於本地 VAD 語音檢測：

```rust
// src/core/sync/engine.rs
pub struct SyncEngine {
    config: SyncConfig,
    vad_detector: Option<VadSyncDetector>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncMethod {
    LocalVad,       // 本地 Voice Activity Detection
    Manual,         // 手動指定偏移量
}

pub struct SyncResult {
    pub offset_seconds: f32,
    pub confidence: f32,      
    pub method_used: SyncMethod,
    pub processing_duration: Duration,
    pub warnings: Vec<String>,
    pub additional_info: Option<serde_json::Value>,
}
```

**同步處理架構**:

SubX 採用直接且高效的 VAD 處理架構：

```
SyncEngine
└── VAD Detector (VadSyncDetector)
    └── 直接處理完整音訊檔案

SyncMethod: LocalVad | Manual
```

**同步方法詳細說明**:

##### 本地 VAD (`src/services/vad/`)
- **語音活動檢測**: 使用信號處理技術檢測語音段
- **本地處理**: 無需網路連接，保護隱私
- **可調參數**: 靈敏度、塊大小、採樣率等
- **高效能**: 適用於大批量處理

```rust
// src/services/vad/sync_detector.rs
pub struct VadSyncDetector {
    detector: VadDetector,
    config: VadConfig,
}

// src/services/vad/detector.rs
pub struct VadDetector {
    config: VadConfig,
}

// src/services/vad/audio_processor.rs
pub struct AudioProcessor {
    target_sample_rate: u32,
}
```

**VAD 處理流程**（已重構）:
1. **直接檔案處理**: 不再限制於時間窗口，直接分析完整音訊檔案
2. **語音檢測**: 使用 voice_activity_detector crate 檢測整個檔案的語音活動
3. **時間對應**: 比較檢測到的第一個語音段與第一句字幕的開始時間
4. **偏移計算**: 直接計算時間差異，無需音訊片段提取
5. **隱私保護**: 所有處理都在本地進行，無需網路連接

##### 手動偏移
- **用戶指定**: 直接提供時間偏移量
- **精確控制**: 適用於已知偏移量的情況
- **快速處理**: 無需分析，直接應用偏移

**引擎配置系統**:

```rust
// 簡化的配置結構
pub struct SyncConfig {
    pub max_offset_seconds: f32,
    pub vad: VadConfig,
}

pub struct VadConfig {
    pub enabled: bool,
    pub sensitivity: f32,
    pub chunk_size: usize,
    pub sample_rate: u32,
    pub padding_chunks: u32,
    pub min_speech_duration_ms: u32,
    pub speech_merge_gap_ms: u32,
}
```

#### 3.5 Parallel Processing (`src/core/parallel/`)

**責任**: 並行任務調度和執行

- 支援多核心並行處理
- 任務佇列管理
- 資源限制控制

#### 3.6 File Manager (`src/core/file_manager.rs`)

**責任**: 安全的檔案操作和回滾支援

- 檔案操作的事務性支援
- 自動備份和復原機制
- 路徑解析和驗證

### 4. External Services Integration

#### 4.1 AI Service (`src/services/ai/`)

**完整的 AI 服務架構**:

```rust
// src/services/ai/openai.rs
pub struct OpenAIClient {
    client: reqwest::Client,
    api_key: String,
    config: OpenAIConfig,
}

// src/services/ai/cache.rs - AI 結果快取
pub struct AICache {
    // Cache implementation for AI responses
}

// src/services/ai/retry.rs - 重試邏輯
pub struct RetryHandler {
    // Retry logic for failed AI requests
}

// src/services/ai/prompts.rs - 提示管理
pub struct PromptManager {
    // AI prompt templates and management
}

// src/services/ai/factory.rs - AI 服務工廠
pub struct AIServiceFactory {
    // Factory for creating AI service instances
}
```

#### 4.2 Audio Processing (`src/services/audio/`)

**音訊處理系統**:

```rust
// src/services/audio/analyzer.rs
pub struct AudioAnalyzer {
    sample_rate: u32,
    window_size: usize,
}

// src/services/audio/dialogue_detector.rs - 對話檢測
pub struct DialogueDetector {
    // Voice activity detection and dialogue segmentation
}
```

**音訊處理流程**:
1. **Audio Loading** - 使用 Symphonia 載入音訊
2. **VAD Integration** - 使用 Voice Activity Detection 進行語音檢測
3. **Dialogue Detection** - 自動檢測對話段落
4. **Feature Extraction** - 提取音訊特徵

## 依賴庫選擇

### 核心依賴
```toml
[dependencies]
# CLI 框架
clap = { version = "4.5.40", features = ["derive", "cargo"] }
clap_complete = "4.5.54"  # Shell completion support
tokio = { version = "1.0", features = ["full"] }

# HTTP 客戶端
reqwest = { version = "0.12.20", features = ["json", "rustls-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# 音訊處理
symphonia = { version = "0.5", features = ["all"] }

# 文件處理
walkdir = "2.3"
regex = "1.0"
encoding_rs = "0.8"

# 配置管理
config = "0.15"
dirs = "6.0"

# 並行處理
futures = "0.3"
async-trait = "0.1"
uuid = { version = "1.3", features = ["v4"] }
num_cpus = "1.0"

# 文件監控
notify = "8.0"

# URL 處理
url = "2"

# 用戶界面
indicatif = "0.17"
colored = "3.0"
tabled = "0.20"
dialoguer = "0.11"

# 實用工具
md5 = "0.7"
log = "0.4"
env_logger = "0.11"

# 錯誤處理
anyhow = "1.0"
thiserror = "2.0"

# 跨平台支援
[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["winuser"] }

[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

### 開發依賴
```toml
[dev-dependencies]
# 測試框架
tokio-test = "0.4"
assert_cmd = "2.0"
predicates = "3.0"
tempfile = "3.10"

# Mock 和測試工具
mockall = "0.13"
rstest = "0.25"
test-case = "3.0"
wiremock = "0.6"

# 效能測試
criterion = { version = "0.6.0", features = ["html_reports"] }
```

## 資料流設計

### 1. Match 工作流程
```
Input: Media Folder
    │
    ▼
┌─────────────────┐
│ File Discovery  │ ──▶ 掃描影片和字幕文件
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Cache Check     │ ──▶ 檢查快取結果
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ AI Analysis     │ ──▶ 調用 AI 進行匹配分析
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Confidence      │ ──▶ 評估匹配信心度
│ Evaluation      │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Dry-run         │ ──▶ 預覽匹配結果
│ Preview         │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ File Rename     │ ──▶ 執行檔案重命名（含備份）
└─────────────────┘
```

> **注意**：字幕檔案重命名時會移除原影片檔案的副檔名，僅保留檔案基礎名稱與字幕副檔名。例如，若影片為 `movie.mkv`，匹配後的字幕檔將命名為 `movie.tc.srt` 而非 `movie.mkv.tc.srt`。

### 2. Sync 工作流程
```
Input: Video + Subtitle
    │
    ▼
┌─────────────────┐
│ Audio Extract   │ ──▶ 提取音訊能量包絡（VAD）
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Dialogue        │ ──▶ 檢測對話段落
│ Detection       │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Signal          │ ──▶ 生成對話時間信號
│ Generation      │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Correlation     │ ──▶ 交叉相關分析
│ Analysis        │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Offset          │ ──▶ 確定最佳偏移量
│ Detection       │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Subtitle        │ ──▶ 應用時間校正
│ Adjustment      │
└─────────────────┘
```

### 3. Convert 工作流程
```
Input: Source Subtitle File
    │
    ▼
┌─────────────────┐
│ Encoding        │ ──▶ 自動檢測字元編碼
│ Detection       │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Format          │ ──▶ 解析來源格式
│ Parsing         │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Content         │ ──▶ 應用格式轉換
│ Transformation │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Output          │ ──▶ 生成目標格式檔案
│ Generation      │
└─────────────────┘
```

## 錯誤處理策略

### 錯誤類型定義
```rust
// src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum SubXError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    #[error("Subtitle format error [{format}]: {message}")]
    SubtitleFormat { format: String, message: String },
    
    #[error("AI service error: {0}")]
    AiService(String),
    
    #[error("Audio processing error: {message}")]
    AudioProcessing { message: String },
    
    #[error("File matching error: {message}")]
    FileMatching { message: String },
    
    #[error("File already exists: {0}")]
    FileAlreadyExists(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Invalid file name: {0}")]
    InvalidFileName(String),
    
    #[error("File operation failed: {0}")]
    FileOperationFailed(String),
}

impl SubXError {
    /// Get the exit code for this error type
    pub fn exit_code(&self) -> i32 {
        match self {
            SubXError::Io(_) => 1,
            SubXError::Config { .. } => 2,
            SubXError::SubtitleFormat { .. } => 3,
            SubXError::AiService(_) => 4,
            SubXError::AudioProcessing { .. } => 5,
            SubXError::FileMatching { .. } => 6,
            // ... other mappings
        }
    }
}
```

## 效能考量

### 1. 併發處理
```rust
// 批量處理使用 tokio 並發
pub async fn process_batch(files: Vec<MediaPair>) -> Result<Vec<ProcessResult>> {
    let max_concurrent = num_cpus::get().min(8); // 限制並發數
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    
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

### 2. 記憶體優化
- **流式處理** - 大文件採用流式讀取
- **音訊採樣** - 使用 VAD 進行高效音訊處理
- **快取機制** - AI 分析結果快取，減少重複請求
- **並行控制** - 根據系統資源動態調整並行度

### 3. API 成本控制
- **內容採樣** - 限制發送給 AI 的內容長度
- **批量分析** - 合併多個請求
- **智慧重試** - 指數退避重試策略
- **快取優化** - 長期快取 AI 分析結果

## 測試策略

### 1. 單元測試
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{TestConfigService, ConfigService};
    use mockall::predicate::*;
    
    #[tokio::test]
    async fn test_match_engine_basic_match() {
        let config_service = TestConfigService::with_defaults();
        let mut mock_ai = MockAIProvider::new();
        mock_ai.expect_analyze_content()
            .returning(|_| Ok(MatchResult::new(0.95)));
        
        let engine = MatchEngine::new(Box::new(mock_ai), &config_service);
        let result = engine.match_file_list(&test_files).await.unwrap();
        
        assert_eq!(result.confidence, 0.95);
    }
}
```

### 2. 整合測試
- **End-to-End 測試** - 完整工作流程測試
- **AI 服務模擬** - 使用 wiremock 進行 HTTP 模擬
- **音訊處理測試** - 使用預製的測試音訊文件
- **配置測試** - 使用 TestConfigService 進行隔離測試

### 3. 效能測試
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_audio_correlation(c: &mut Criterion) {
    c.bench_function("audio_correlation", |b| {
        b.iter(|| {
            let result = calculate_correlation(
                black_box(&audio_data), 
                black_box(&dialogue_data)
            );
            black_box(result)
        })
    });
}

criterion_group!(benches, bench_audio_correlation);
criterion_main!(benches);
```

**測試工具**:
- **rstest** - 參數化測試
- **test-case** - 測試案例生成
- **assert_cmd** - CLI 測試
- **tempfile** - 臨時檔案管理

## 部署和發佈

### 1. 編譯目標
```toml
# 支援多平台編譯
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### 2. CI/CD Pipeline
- **GitHub Actions** - 自動化建構、測試、覆蓋率檢查
- **Cross-compilation** - 支援 Linux、Windows、macOS
- **自動發佈** - 自動生成 GitHub Releases 和 crates.io 發佈

### 3. 發佈策略
- **GitHub Releases** - 預編譯二進位文件
- **crates.io** - Rust 套件發佈
- **安裝腳本** - 自動化安裝腳本 (`scripts/install.sh`)
- **Shell 完成** - 自動生成 bash/zsh/fish 完成腳本

## 品質保證

### 1. 程式碼品質
- **rustfmt** - 程式碼格式化
- **clippy** - 靜態分析和 linting
- **rustdoc** - 文件品質檢查
- **audit** - 安全漏洞掃描

### 2. 測試覆蓋率
- **llvm-cov** - 程式碼覆蓋率分析
- **codecov** - 覆蓋率報告和追蹤
- **並行測試** - 測試穩定性驗證

### 3. 程式碼品質和文件品質檢查
- **檢查腳本** - `scripts/quality_check.sh`
- **內連結驗證** - 確保文件連結有效
- **API 文件** - 完整的 rustdoc 文件

## 系統需求

### 最低需求
- **作業系統**: Linux (x86_64), Windows (x86_64), macOS (x86_64, ARM64)
- **記憶體**: 建議 4GB 以上
- **硬碟空間**: 100MB （不含快取和臨時檔案）

### 外部依賴
- **FFmpeg** - 音訊處理 (可選，由 symphonia 處理大部分格式)
- **OpenAI API** - AI 分析功能
- **網路連線** - 用於 AI 服務和更新檢查
