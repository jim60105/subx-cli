# Backlog 42: 移除不必要的 AudioTranscoder 並實現 VAD 直接音訊格式支援

## 概述

本項目發現了目前 VAD（Voice Activity Detection）實作中的一個架構性問題：系統錯誤地使用 `AudioTranscoder` 將所有音訊檔案轉換為 WAV 格式，然後才進行 VAD 處理。然而，我們使用的 `voice_activity_detector` 套件實際上可以直接處理原始音訊樣本數據（`&[i16]`），並不需要特定的檔案格式。

這種不必要的轉碼步驟導致：
- 額外的處理時間和 CPU 使用
- 額外的磁碟空間佔用（臨時 WAV 檔案）
- 架構複雜度增加
- 可能的音質損失

## 問題分析

### 目前的錯誤架構
```
音訊檔案 (MP4/MKV/OGG/etc) 
    ↓ 
AudioTranscoder.transcode_to_wav() 
    ↓ 
臨時 WAV 檔案 
    ↓ 
VadAudioProcessor.load_wav_file() 
    ↓ 
hound::WavReader 
    ↓ 
audio samples (&[i16]) 
    ↓ 
VoiceActivityDetector
```

### 目標架構
```
音訊檔案 (任何格式) 
    ↓ 
Symphonia 直接解碼 
    ↓ 
audio samples (&[i16]) 
    ↓ 
VoiceActivityDetector
```

## 技術分析

### 檔案位置分析
- **AudioTranscoder**: `src/services/audio/transcoder.rs`
- **VadAudioProcessor**: `src/services/vad/audio_processor.rs`
- **LocalVadDetector**: `src/services/vad/detector.rs`
- **使用點**: 主要在文件範例中，生產環境中尚未被直接使用

### 依賴分析
- `voice_activity_detector` 套件接受 `&[i16]` 樣本數據
- 套件內部使用 ONNX 模型，不依賴特定音訊格式
- 我們已經有 Symphonia 來處理多種音訊格式

## 實作計畫

### 階段一：分析和準備 (預估: 2 工作日)

#### 任務 1.1: 深度分析 AudioTranscoder 使用情況
```bash
# 搜尋所有對 AudioTranscoder 的引用
grep -r "AudioTranscoder" src/
grep -r "transcode" src/
```

**產出**:
- 完整的使用情況報告
- 確認目前沒有生產環境依賴

#### 任務 1.2: 研究 Symphonia 直接音訊解碼能力
```rust
// 研究如何使用 Symphonia 直接獲取音訊樣本
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
```

**產出**:
- Symphonia 音訊解碼範例程式碼
- 支援格式清單確認

#### 任務 1.3: 分析 VadAudioProcessor 重構需求
- 檢查 `load_wav_file()` 方法
- 分析依賴於 `hound::WavReader` 的程式碼
- 設計新的音訊載入介面

### 階段二：新增直接音訊處理能力 (預估: 3 工作日)

#### 任務 2.1: 實作新的音訊載入器
建立 `src/services/vad/audio_loader.rs`:

```rust
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};
use std::fs::File;
use std::path::Path;

pub struct DirectAudioLoader {
    probe: &'static Probe,
    codecs: &'static CodecRegistry,
}

impl DirectAudioLoader {
    pub fn new() -> Result<Self> {
        Ok(Self {
            probe: get_probe(),
            codecs: get_codecs(),
        })
    }

    /// 直接從音訊檔案載入樣本數據，支援多種格式
    pub fn load_audio_samples<P: AsRef<Path>>(
        &self, 
        path: P
    ) -> Result<(Vec<i16>, AudioInfo)> {
        // 使用 Symphonia 直接解碼音訊
        // 支援 MP4, MKV, OGG, WAV, 等格式
        // 返回 i16 樣本和音訊資訊
    }
}
```

#### 任務 2.2: 重構 VadAudioProcessor
更新 `src/services/vad/audio_processor.rs`:

```rust
impl VadAudioProcessor {
    /// 新方法：直接載入各種格式的音訊檔案
    pub async fn load_and_prepare_audio_direct(&self, audio_path: &Path) -> Result<ProcessedAudioData> {
        // 1. 使用 DirectAudioLoader 載入音訊
        let loader = DirectAudioLoader::new()?;
        let (samples, info) = loader.load_audio_samples(audio_path)?;

        // 2. 建立 ProcessedAudioData
        let audio_data = ProcessedAudioData { samples, info };

        // 3. 進行必要的重採樣和格式轉換
        let resampled_data = if audio_data.info.sample_rate != self.target_sample_rate {
            self.resample_audio(&audio_data)?
        } else {
            audio_data
        };

        // 4. 轉換為單聲道（如需要）
        let mono_data = if resampled_data.info.channels > 1 {
            self.convert_to_mono(&resampled_data)?
        } else {
            resampled_data
        };

        Ok(mono_data)
    }

    /// 保留舊方法以向後相容（標記為已廢棄）
    #[deprecated(note = "Use load_and_prepare_audio_direct instead")]
    pub async fn load_and_prepare_audio(&self, audio_path: &Path) -> Result<ProcessedAudioData> {
        self.load_and_prepare_audio_direct(audio_path).await
    }
}
```

#### 任務 2.3: 建立完整的測試套件
建立 `tests/vad_direct_audio_loading_tests.rs`:

```rust
#[tokio::test]
async fn test_direct_mp4_loading() {
    // 測試直接載入 MP4 檔案
}

#[tokio::test]
async fn test_direct_mkv_loading() {
    // 測試直接載入 MKV 檔案
}

#[tokio::test]
async fn test_format_comparison() {
    // 比較直接載入和轉碼載入的結果
}

#[tokio::test]
async fn test_performance_improvement() {
    // 測試效能改善
}
```

### 階段三：更新 VAD 檢測器 (預估: 2 工作日)

#### 任務 3.1: 更新 LocalVadDetector
更新 `src/services/vad/detector.rs`:

```rust
impl LocalVadDetector {
    /// 更新的語音檢測方法，直接支援多種音訊格式
    pub async fn detect_speech(&self, audio_path: &Path) -> Result<VadResult> {
        let start_time = Instant::now();

        // 使用新的直接音訊載入方法
        let audio_data = self
            .audio_processor
            .load_and_prepare_audio_direct(audio_path)
            .await?;

        // 其餘邏輯保持不變
        let vad = VoiceActivityDetector::builder()
            .sample_rate(self.config.sample_rate)
            .chunk_size(self.config.chunk_size)
            .build()
            .map_err(|e| SubXError::audio_processing(format!("Failed to create VAD: {}", e)))?;

        let speech_segments = self.detect_speech_segments(vad, &audio_data.samples)?;
        let processing_duration = start_time.elapsed();

        Ok(VadResult {
            speech_segments,
            processing_duration,
            audio_info: audio_data.info,
        })
    }
}
```

### 階段四：移除 AudioTranscoder 依賴 (預估: 2 工作日)

#### 任務 4.1: 驗證沒有生產環境依賴
執行完整的程式碼搜尋和測試：

```bash
# 搜尋所有可能的依賴
grep -r "AudioTranscoder" src/ --exclude-dir=tests
grep -r "transcode_to_wav" src/ --exclude-dir=tests

# 執行所有測試確保沒有破壞
cargo test
```

#### 任務 4.2: 清理 AudioTranscoder 相關程式碼
1. 移除 `src/services/audio/transcoder.rs`
2. 更新 `src/services/audio/mod.rs` 移除 AudioTranscoder 匯出
3. 清理所有相關的測試檔案
4. 更新文件中的範例程式碼

#### 任務 4.3: 更新 Cargo.toml 依賴
檢查並移除不再需要的依賴項：
```toml
# 檢查這些依賴是否還需要
hound = "3.5"  # 可能只在其他地方需要
tempfile = "3.8"  # AudioTranscoder 使用的臨時檔案
```

### 階段五：效能測試和驗證 (預估: 2 工作日)

#### 任務 5.1: 建立效能基準測試
建立 `benches/vad_performance_comparison.rs`:

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_old_transcoding_method(c: &mut Criterion) {
    // 測試舊的轉碼方法
}

fn benchmark_new_direct_method(c: &mut Criterion) {
    // 測試新的直接方法
}

criterion_group!(
    vad_performance,
    benchmark_old_transcoding_method,
    benchmark_new_direct_method
);
criterion_main!(vad_performance);
```

#### 任務 5.2: 執行全面測試
```bash
# 效能測試
cargo bench

# 功能測試
cargo test

# 整合測試
timeout 30 scripts/quality_check.sh
```

### 階段六：文件更新和部署準備 (預估: 1 工作日)

#### 任務 6.1: 更新技術文件
更新以下檔案：
- `docs/tech-architecture.md` - 移除 AudioTranscoder 相關內容
- `docs/configuration-guide.md` - 更新 VAD 處理說明
- `README.md` - 更新音訊格式支援說明

#### 任務 6.2: 更新範例程式碼
更新 `src/services/mod.rs` 和 `src/services/audio/mod.rs` 中的範例：

```rust
//! ## Audio Synchronization
//! ```rust,ignore
//! use subx_cli::services::vad::LocalVadDetector;
//! use subx_cli::config::VadConfig;
//!
//! async fn synchronize_audio() -> subx_cli::Result<()> {
//!     let vad_config = VadConfig::default();
//!     let detector = LocalVadDetector::new(vad_config)?;
//!     
//!     // 直接處理各種音訊格式，無需轉碼
//!     let result = detector.detect_speech("video.mp4").await?;
//!     
//!     println!("Detected {} speech segments", result.speech_segments.len());
//!     Ok(())
//! }
//! ```
```

## 驗收標準

### 功能驗收
- [ ] VAD 系統可以直接處理 MP4, MKV, OGG, WAV 等格式
- [ ] 移除所有 AudioTranscoder 相關程式碼
- [ ] 所有現有測試繼續通過
- [ ] 新增的直接音訊載入測試通過

### 效能驗收
- [ ] 處理時間減少至少 30%（無轉碼開銷）
- [ ] 磁碟空間使用減少（無臨時 WAV 檔案）
- [ ] 記憶體使用更有效率

### 程式碼品質驗收
- [ ] `cargo fmt` 通過
- [ ] `cargo clippy -- -D warnings` 無警告
- [ ] `timeout 30 scripts/quality_check.sh` 通過
- [ ] 程式碼覆蓋率保持或提升

## 風險評估

### 低風險
- **相容性問題**: 使用 Symphonia 確保廣泛的格式支援
- **效能退化**: 直接解碼比轉碼更有效率

### 中等風險
- **音訊品質**: 需要確保直接解碼的品質與轉碼相同
- **錯誤處理**: 需要處理更多種類的音訊格式錯誤

### 緩解策略
- 完整的測試覆蓋，包括各種音訊格式
- 逐步遷移，保留舊方法作為備用
- 詳細的錯誤日誌和處理

## 專案里程碑

- **第 1 週**: 完成階段一和二（分析和新功能實作）
- **第 2 週**: 完成階段三和四（整合和清理）
- **第 3 週**: 完成階段五和六（測試和文件）

## 預期效益

### 效能改善
- **處理速度**: 提升 30-50%（移除轉碼開銷）
- **磁碟使用**: 減少臨時檔案佔用
- **記憶體效率**: 更直接的資料流

### 架構簡化
- **減少依賴**: 移除不必要的 AudioTranscoder
- **程式碼簡潔**: 更直接的音訊處理流程
- **維護性**: 更少的元件和複雜度

### 使用者體驗
- **更快的同步**: 減少等待時間
- **更廣泛的格式支援**: 無需額外轉碼步驟
- **更穩定的效能**: 減少臨時檔案操作風險

---

**注意**: 這是一個重要的架構改善項目，需要仔細的測試和驗證。建議在實作過程中保持與現有功能的向後相容性，直到完全驗證新實作的正確性。
