# SubX 技術架構文件

## 專案概覽

SubX 是一個基於 Rust 開發的 CLI 工具，專注於智慧字幕處理。採用模組化設計，支援多種字幕格式和 AI 驅動的匹配算法。

## 整體架構

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   CLI Interface │───▶│   Core Engine    │───▶│  Output Handler │
│                 │    │                  │    │                 │
│ • Argument      │    │ • Match Engine   │    │ • File Writer   │
│   Parsing       │    │ • Format Engine  │    │ • Progress      │
│ • Command       │    │ • Sync Engine    │    │   Reporting     │
│   Routing       │    │                  │    │ • Error Handler │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │
                                ▼
        ┌───────────────────────────────────────────────────┐
        │                External Services                  │
        │                                                   │
        │  ┌─────────────┐  ┌─────────────┐  ┌──────────── │
        │  │ OpenAI API  │  │ Audio Proc. │  │ File System │
        │  │             │  │             │  │             │
        │  │ • GPT-4o    │  │ • FFmpeg    │  │ • File I/O  │
        │  │ • Text      │  │ • Audio     │  │ • Path      │
        │  │   Analysis  │  │   Analysis  │  │   Handling  │
        │  └─────────────┘  └─────────────┘  └─────────────┘
        └───────────────────────────────────────────────────┘
```

## 核心模組設計

### 1. CLI Interface Layer (`src/cli/`)

**責任**: 用戶界面和命令解析

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
    Config(ConfigArgs),
}
```

**關鍵組件**:
- `clap` - 命令行參數解析
- `indicatif` - 進度條顯示
- `colored` - 彩色輸出
- `dialoguer` - 互動式提示

### 2. Core Engine (`src/core/`)

#### 2.1 Match Engine (`src/core/matcher/`)

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

#### 2.2 Format Engine (`src/core/formats/`)

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

#### 2.3 Sync Engine (`src/core/sync/`)

**責任**: 時間軸同步和校正

```rust
// src/core/sync/mod.rs
pub struct SyncEngine {
    audio_analyzer: AudioAnalyzer,
    correlation_calculator: CorrelationCalculator,
}

pub struct SyncResult {
    pub offset_seconds: f64,
    pub confidence: f32,
    pub method_used: SyncMethod,
}
```

**同步算法**:
1. **Audio Envelope Extraction** - 音訊包絡提取
2. **Dialogue Signal Generation** - 對話信號生成
3. **Cross-Correlation Analysis** - 交叉相關分析
4. **Optimal Offset Detection** - 最佳偏移檢測

### 3. External Services Integration

#### 3.1 AI Service (`src/services/ai/`)

```rust
// src/services/ai/openai.rs
pub struct OpenAIClient {
    client: reqwest::Client,
    api_key: String,
    config: OpenAIConfig,
}

impl AIProvider for OpenAIClient {
    async fn analyze_content(&self, request: AnalysisRequest) -> Result<MatchResult> {
        let prompt = self.build_analysis_prompt(&request);
        let response = self.chat_completion(prompt).await?;
        self.parse_match_result(response)
    }
}
```

#### 3.2 Audio Processing (`src/services/audio/`)

```rust
// src/services/audio/mod.rs
pub struct AudioAnalyzer {
    sample_rate: u32,
    window_size: usize,
}

impl AudioAnalyzer {
    pub fn extract_envelope(&self, audio_path: &Path) -> Result<Vec<f32>> {
        // 使用 symphonia 提取音訊特徵
    }
    
    pub fn calculate_correlation(&self, 
        audio_envelope: &[f32], 
        dialogue_signal: &[f32], 
        offset: i32
    ) -> f32 {
        // 交叉相關計算
    }
}
```

## 依賴庫選擇

### 核心依賴
```toml
[dependencies]
# CLI 框架
clap = { version = "4.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }

# HTTP 客戶端
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }

# 音訊處理
symphonia = { version = "0.5", features = ["all"] }
rustfft = "6.0"

# 文件處理
walkdir = "2.0"
regex = "1.0"
encoding_rs = "0.8"

# 用戶界面
indicatif = "0.17"
colored = "2.0"
dialoguer = "0.10"

# 錯誤處理
anyhow = "1.0"
thiserror = "1.0"
```

### 開發依賴
```toml
[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.0"
mockall = "0.11"
criterion = "0.4"
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
│ File Rename     │ ──▶ 執行檔案重命名
└─────────────────┘
```

### 2. Sync 工作流程
```
Input: Video + Subtitle
    │
    ▼
┌─────────────────┐
│ Audio Extract   │ ──▶ 提取音訊能量包絡
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Dialogue Signal │ ──▶ 生成對話時間信號
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

## 錯誤處理策略

### 錯誤類型定義
```rust
// src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum SubXError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("AI service error: {0}")]
    AIService(#[from] AIServiceError),
    
    #[error("Subtitle parsing error: {0}")]
    SubtitleParse(String),
    
    #[error("Audio processing error: {0}")]
    AudioProcessing(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
}
```

## 效能考量

### 1. 併發處理
```rust
// 批量處理使用 tokio 並發
pub async fn process_batch(files: Vec<MediaPair>) -> Result<Vec<ProcessResult>> {
    let semaphore = Arc::new(Semaphore::new(4)); // 限制並發數
    
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
- **音訊採樣** - 降採樣減少記憶體佔用
- **快取機制** - AI 分析結果快取

### 3. API 成本控制
- **內容採樣** - 限制發送給 AI 的內容長度
- **批量分析** - 合併多個請求
- **模型選擇** - 預設使用成本較低的模型

## 測試策略

### 1. 單元測試
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    
    #[tokio::test]
    async fn test_match_engine_basic_match() {
        let mut mock_ai = MockAIProvider::new();
        mock_ai.expect_analyze_content()
            .returning(|_| Ok(MatchResult::new(0.95)));
        
        let engine = MatchEngine::new(Box::new(mock_ai));
        let result = engine.match_files(&test_files).await.unwrap();
        
        assert_eq!(result.confidence, 0.95);
    }
}
```

### 2. 整合測試
- **End-to-End 測試** - 完整工作流程測試
- **AI 服務模擬** - 使用 mock 避免實際 API 調用
- **音訊處理測試** - 使用預製的測試音訊文件

### 3. 效能測試
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_audio_correlation(c: &mut Criterion) {
    c.bench_function("audio_correlation", |b| {
        b.iter(|| {
            let result = calculate_correlation(black_box(&audio_data), black_box(&dialogue_data));
            black_box(result)
        })
    });
}
```

## 部署和發佈

### 1. 編譯目標
```toml
# .cargo/config.toml
[build]
target = ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-gnu", "x86_64-apple-darwin"]
```

### 2. CI/CD Pipeline
```yaml
# .github/workflows/release.yml
- name: Build Release
  run: |
    cargo build --release --target x86_64-unknown-linux-gnu
    cargo build --release --target x86_64-pc-windows-gnu
    cargo build --release --target x86_64-apple-darwin
```

### 3. 發佈策略
- **GitHub Releases** - 預編譯二進位文件
- **Cargo.toml** - 發佈到 crates.io
