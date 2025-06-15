# Backlog 33: 完全移除 Whisper 設計

## 概覽

根據專案需求變更，決定完全移除 OpenAI Whisper API 相關功能，僅保留本地 VAD (Voice Activity Detection) 作為語音檢測解決方案。此任務將清理所有 Whisper 相關的程式碼、配置、測試和文件，並重構系統架構以純 VAD 為基礎。

## 背景

目前專案包含了完整的 Whisper API 整合功能（Backlog 32.2），但基於以下考量決定移除：
- 簡化系統架構，減少外部依賴
- 降低使用成本（避免 API 呼叫費用）
- 提升使用者隱私（所有處理皆在本地進行）
- 專注於本地處理能力，符合專案定位

## 目標

1. 完全移除所有 Whisper 相關的程式碼和配置
2. 重構 VAD 系統為唯一的語音檢測方案，處理完整音訊檔案
3. 移除音訊裁切和轉檔邏輯，利用 VAD 套件原生多格式支援
4. 更新 CLI 參數和設定選項
5. 清理相關測試檔案（**不修改歷史文件和報告**）

## 影響範圍分析

### 需要完全移除的檔案
- `src/services/whisper/` 整個目錄（4個檔案）
  - `src/services/whisper/mod.rs`
  - `src/services/whisper/client.rs`  
  - `src/services/whisper/audio_extractor.rs`
  - `src/services/whisper/sync_detector.rs`
- `tests/whisper_integration_tests.rs`
- `tests/whisper_mock_tests.rs`

### 需要修改的確切程式碼位置

#### `src/core/sync/engine.rs`
- **L14**: 移除 `use crate::services::whisper::{AudioSegmentExtractor, WhisperSyncDetector};`
- **L20**: 移除 `whisper_detector: Option<WhisperSyncDetector>,` 欄位
- **L22**: 移除 `audio_extractor: AudioSegmentExtractor,` 欄位
- **L28-36**: 移除 Whisper 檢測器初始化邏輯
- **L59**: 移除 `audio_extractor: AudioSegmentExtractor::new()?,`
- **L67-75**: 移除 `create_whisper_detector` 方法
- **L88-91**: 移除 `SyncMethod::WhisperApi => {...}` 分支
- **L173**: 移除 `"whisper" => SyncMethod::WhisperApi,` 映射
- **L179-191**: 移除 `whisper_detect_sync_offset` 方法
- **L222**: 從 `SyncMethod` 枚舉移除 `WhisperApi` 變數
- **L293**: 移除 `("whisper", SyncMethod::WhisperApi),` 項目
- **L314**: 移除 `preferred_methods: vec![SyncMethod::WhisperApi, SyncMethod::LocalVad],`

#### `src/core/services.rs` 
- **L9**: 移除 `services::whisper::WhisperSyncDetector,` 匯入
- **L60-73**: 移除整個 `create_whisper_detector` 方法

#### `src/config/mod.rs`
- **L200**: 移除 `pub whisper: WhisperConfig,` 欄位
- **L230-249**: 移除整個 `WhisperConfig` 結構定義
- **L277**: 移除 `whisper: WhisperConfig::default(),`
- **L290-304**: 移除 `WhisperConfig` 的 `Default` 實作
- **L514-535**: 移除 Whisper 相關配置建構邏輯

#### `src/config/validator.rs`
- **L9**: 移除 `WhisperConfig` 匯入
- **L104+**: 移除 `WhisperConfig` 驗證實作

#### `src/cli/sync_args.rs`
- **L169-188**: 移除 `SyncMethodArg::Whisper` 變數及相關實作
- **L376**: 移除 `crate::core::sync::SyncMethod::WhisperApi` 轉換

#### `src/commands/sync_command.rs`
- **L131**: 移除 `"whisper" => Ok(SyncMethod::WhisperApi),` 映射

#### `src/services/vad/sync_detector.rs`
- **L5**: 將 `use crate::services::whisper::AudioSegmentExtractor;` 
  改為使用本地音訊處理或移除音訊裁切功能
- **L12**: 移除 `audio_extractor: AudioSegmentExtractor,` 欄位  
- **L19**: 移除 `audio_extractor: AudioSegmentExtractor::new()?,`
- **L34-37**: 移除音訊片段提取邏輯，改為直接處理完整音訊檔案
- **L42-45**: 移除臨時檔案清理邏輯

### 需要重構的 VAD 系統

根據程式碼分析，目前的 VAD 實作有以下問題需要解決：

1. **移除音訊裁切依賴**: `VadSyncDetector` 目前依賴 `AudioSegmentExtractor` 進行音訊裁切
2. **移除 WAV 轉檔需求**: `VadAudioProcessor.load_wav_file()` 只支援 WAV 格式
3. **重新設計同步邏輯**: 不再基於 30 秒窗口，而是分析整個音訊檔案

## 技術規格

### 新的 VAD 架構設計

#### 移除音訊裁切，直接處理完整音訊檔案

```rust
// src/services/vad/sync_detector.rs (重構後)
use super::{LocalVadDetector, VadResult};
use crate::config::VadConfig;
use crate::core::formats::{Subtitle, SubtitleEntry};
use crate::core::sync::{SyncMethod, SyncResult};
use crate::{Result, error::SubXError};
use serde_json::json;
use std::path::Path;

pub struct VadSyncDetector {
    vad_detector: LocalVadDetector,
}

impl VadSyncDetector {
    pub fn new(config: VadConfig) -> Result<Self> {
        Ok(Self {
            vad_detector: LocalVadDetector::new(config)?,
        })
    }

    pub async fn detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
        _analysis_window_seconds: u32, // 忽略此參數，處理完整檔案
    ) -> Result<SyncResult> {
        // 1. 獲取第一句字幕的預期開始時間
        let first_entry = self.get_first_subtitle_entry(subtitle)?;

        // 2. 直接對完整音訊檔案進行 VAD 分析
        let vad_result = self.vad_detector.detect_speech(audio_path).await?;

        // 3. 分析結果：比較第一個語音片段與第一句字幕的時間差
        let analysis_result = self.analyze_vad_result(&vad_result, first_entry)?;

        Ok(analysis_result)
    }

    fn analyze_vad_result(
        &self,
        vad_result: &VadResult,
        first_entry: &SubtitleEntry,
    ) -> Result<SyncResult> {
        // 檢測第一個顯著的語音片段
        let first_speech_time = self.find_first_significant_speech(vad_result)?;
        
        // 計算偏移量：實際語音開始時間 - 預期字幕開始時間
        let expected_start = first_entry.start_time.as_secs_f64();
        let offset_seconds = first_speech_time - expected_start;

        // 計算信心度
        let confidence = self.calculate_confidence(vad_result);

        Ok(SyncResult {
            offset_seconds: offset_seconds as f32,
            confidence,
            method_used: SyncMethod::LocalVad,
            correlation_peak: 0.0,
            additional_info: Some(json!({
                "speech_segments_count": vad_result.speech_segments.len(),
                "first_speech_start": first_speech_time,
                "expected_subtitle_start": expected_start,
                "processing_time_ms": vad_result.processing_duration.as_millis(),
                "audio_duration": vad_result.audio_info.duration_seconds,
            })),
            processing_duration: vad_result.processing_duration,
            warnings: Vec::new(),
        })
    }
}
```

#### 移除 WAV 轉檔需求，利用 VAD 套件多格式支援

由於 `voice_activity_detector` 套件可以支援多種音訊格式，需要移除目前 `VadAudioProcessor` 中的 WAV 限制：

```rust
// src/services/vad/audio_processor.rs (需要重構的部分)
impl VadAudioProcessor {
    // 移除 load_wav_file 方法，改為 load_audio_file
    pub async fn load_and_prepare_audio(&self, audio_path: &Path) -> Result<ProcessedAudioData> {
        // 直接使用 VAD 套件的音訊載入能力，支援多格式
        // 移除 WAV 特定的處理邏輯
    }
}
```

### 簡化的同步方法枚舉

```rust
// src/core/sync/engine.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncMethod {
    /// 本地 VAD 檢測
    LocalVad,
    /// 手動指定偏移量
    Manual,
    /// 自動選擇（目前只有 VAD）
    Auto,
}

impl Default for SyncMethod {
    fn default() -> Self {
        Self::Auto
    }
}
```

### 更新的同步配置

```rust
// src/config/mod.rs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    /// 預設同步方法 ("vad", "manual", "auto")
    pub default_method: String,
    /// 最大允許的時間偏移量（秒）
    pub max_offset_seconds: f32,
    /// 本地 VAD 配置
    pub vad: VadConfig,

    // 移除 whisper: WhisperConfig 欄位
    // 移除 analysis_window_seconds 欄位（不再需要）
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            default_method: "auto".to_string(),
            max_offset_seconds: 60.0,
            vad: VadConfig::default(),
        }
    }
}
```

### 簡化的同步引擎

```rust
// src/core/sync/engine.rs
pub struct SyncEngine {
    config: SyncConfig,
    vad_detector: Option<VadSyncDetector>,
}

impl SyncEngine {
    pub fn new(config: SyncConfig) -> Result<Self> {
        let vad_detector = if config.vad.enabled {
            Some(VadSyncDetector::new(config.vad.clone())?)
        } else {
            None
        };

        Ok(Self {
            config,
            vad_detector,
        })
    }

    pub async fn detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
        method: Option<SyncMethod>,
    ) -> Result<SyncResult> {
        let method = method.unwrap_or_else(|| self.parse_method(&self.config.default_method));
        
        match method {
            SyncMethod::LocalVad | SyncMethod::Auto => {
                self.vad_detect_sync_offset(audio_path, subtitle).await
            }
            SyncMethod::Manual => {
                Err(SubXError::config("Manual method requires explicit offset"))
            }
        }
    }

    async fn vad_detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
    ) -> Result<SyncResult> {
        let detector = self
            .vad_detector
            .as_ref()
            .ok_or_else(|| SubXError::audio_processing("VAD detector not available"))?;
        
        detector
            .detect_sync_offset(audio_path, subtitle, 0) // analysis_window_seconds 不再使用
            .await
    }
}
```

## 實作步驟

### 步驟 1: 移除音訊裁切依賴並重構 VAD 同步檢測器

**目標**: 重構 `VadSyncDetector` 以處理完整音訊檔案，移除對 Whisper `AudioSegmentExtractor` 的依賴

**確切的程式碼修改**:

**檔案**: `src/services/vad/sync_detector.rs`
- **L5**: 移除 `use crate::services::whisper::AudioSegmentExtractor;`
- **L12**: 移除 `audio_extractor: AudioSegmentExtractor,` 欄位
- **L19**: 移除 `audio_extractor: AudioSegmentExtractor::new()?,` 初始化
- **L29-52**: 重構 `detect_sync_offset` 方法：
  ```rust
  // 原：提取音訊片段 + VAD 分析
  let audio_segment_path = self.audio_extractor.extract_segment(
      audio_path, first_entry.start_time, analysis_window_seconds
  ).await?;
  let vad_result = self.vad_detector.detect_speech(&audio_segment_path).await?;
  
  // 改為：直接對完整音訊檔案進行 VAD 分析
  let vad_result = self.vad_detector.detect_speech(audio_path).await?;
  ```
- **L64-95**: 重構 `analyze_vad_result` 偏移量計算：
  ```rust
  // 原：基於分析窗口的相對偏移計算
  let half_window = analysis_window_seconds as f64 / 2.0;
  let expected_position_in_segment = half_window;
  let offset_seconds = actual_position_in_segment - expected_position_in_segment;
  
  // 改為：直接比較語音開始時間與字幕時間
  let expected_start = first_entry.start_time.as_secs_f64();
  let offset_seconds = first_speech_time - expected_start;
  ```

### 步驟 2: 移除音訊轉檔限制，支援多格式輸入

**目標**: 修改 `VadAudioProcessor` 以支援多種音訊格式，而非僅限 WAV

**實作內容**:
根據 `voice_activity_detector` 文件，它支援 LPCM 編碼音訊（8/16位整數或32位浮點），需要修改：

**檔案**: `src/services/vad/audio_processor.rs`
- **L50-101**: 重構 `load_wav_file` 方法為 `load_audio_file`，支援多格式
- **L29-43**: 更新 `load_and_prepare_audio` 方法調用

**注意**: 可能需要整合 Symphonia 或其他音訊解碼庫來支援多格式。

### 步驟 3: 移除 Whisper 相關程式碼檔案

**目標**: 刪除所有 Whisper 相關的程式碼檔案

**實作內容**:
- 刪除 `src/services/whisper/` 整個目錄：
  - `src/services/whisper/mod.rs`
  - `src/services/whisper/client.rs`
  - `src/services/whisper/audio_extractor.rs` 
  - `src/services/whisper/sync_detector.rs`

### 步驟 4: 重構同步引擎，移除 Whisper 邏輯

**目標**: 從 `SyncEngine` 中移除所有 Whisper 相關邏輯

**確切的程式碼修改**:

**檔案**: `src/core/sync/engine.rs`
- **L14**: 移除 `use crate::services::whisper::{AudioSegmentExtractor, WhisperSyncDetector};`
- **L20**: 移除 `whisper_detector: Option<WhisperSyncDetector>,` 欄位
- **L22**: 移除 `audio_extractor: AudioSegmentExtractor,` 欄位
- **L28-36**: 移除 Whisper 檢測器初始化邏輯
- **L59**: 移除 `audio_extractor: AudioSegmentExtractor::new()?,`
- **L67-75**: 移除整個 `create_whisper_detector` 方法
- **L88-91**: 移除 `SyncMethod::WhisperApi => {...}` 分支
- **L173**: 移除 `"whisper" => SyncMethod::WhisperApi,` 映射
- **L179-191**: 移除整個 `whisper_detect_sync_offset` 方法
- **L293**: 移除 `("whisper", SyncMethod::WhisperApi),` 項目
- **L314**: 從 `preferred_methods` 移除 `SyncMethod::WhisperApi`

### 步驟 5: 簡化同步方法枚舉

**目標**: 從 `SyncMethod` 枚舉中移除 `WhisperApi` 選項

**確切的程式碼修改**:

**檔案**: `src/core/sync/engine.rs` (L222 附近)
```rust
// 原枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncMethod {
    LocalVad,
    WhisperApi,  // ← 移除此項
    Manual,
    Auto,
}

// 修改後
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncMethod {
    LocalVad,
    Manual, 
    Auto,
}
```

### 步驟 6: 更新配置系統，移除 Whisper 配置

**目標**: 從配置系統中完全移除 Whisper 相關配置

**確切的程式碼修改**:

**檔案**: `src/config/mod.rs`
- **L200**: 移除 `pub whisper: WhisperConfig,` 欄位
- **L230-249**: 移除整個 `WhisperConfig` 結構定義
- **L277**: 移除 `whisper: WhisperConfig::default(),`
- **L290-304**: 移除 `WhisperConfig` 的 `Default` 實作
- **L514-535**: 移除 Whisper 配置建構邏輯

**檔案**: `src/config/validator.rs`
- **L9**: 移除 `WhisperConfig` 匯入
- **L104+**: 移除 `WhisperConfig` 驗證實作

### 步驟 7: 更新服務工廠

**目標**: 從服務建立邏輯中移除 Whisper 服務

**確切的程式碼修改**:

**檔案**: `src/core/services.rs`
- **L9**: 移除 `services::whisper::WhisperSyncDetector,` 匯入
- **L60-73**: 移除整個 `create_whisper_detector` 方法

### 步驟 8: 更新 CLI 介面

**目標**: 移除 CLI 中的 Whisper 相關參數

**確切的程式碼修改**:

**檔案**: `src/cli/sync_args.rs`
- **L169-188**: 移除 `SyncMethodArg::Whisper` 變數及相關實作
- **L376**: 移除 `crate::core::sync::SyncMethod::WhisperApi` 轉換

**檔案**: `src/commands/sync_command.rs`
- **L131**: 移除 `"whisper" => Ok(SyncMethod::WhisperApi),` 映射

### 步驟 9: 清理依賴套件

**目標**: 檢查並移除僅用於 Whisper 的依賴套件

**實作內容**:
檢查 `Cargo.toml` 中的依賴，重點關注：
- `tokio-util` - 檢查是否僅用於 Whisper multipart 上傳
- 其他 HTTP 相關套件是否僅為 Whisper 添加
- **保留**: `voice_activity_detector`, `hound`, `rubato` 等 VAD 相關套件

### 步驟 10: 移除測試檔案

**目標**: 清理所有 Whisper 相關的測試檔案

**實作內容**:
- 刪除 `tests/whisper_integration_tests.rs`
- 刪除 `tests/whisper_mock_tests.rs`
- 修改其他測試檔案，移除 Whisper 相關測試案例：
  - `tests/sync_cli_integration_tests.rs` (L233: Whisper 方法測試)
  - `tests/sync_new_architecture_tests.rs` (L217: Whisper 方法測試)
  - `tests/config_*` 系列測試檔案中的 Whisper 配置測試

### 步驟 11: 最終驗證和清理

**目標**: 確保系統完整性和功能正確性

**實作內容**:
1. 執行完整的程式碼檢查：
   - `cargo build --release`
   - `cargo test`
   - `cargo clippy -- -D warnings`
   - `cargo fmt --check`

2. 功能驗證：
   - 測試 VAD 同步功能與完整音訊檔案
   - 驗證不同音訊格式的支援（如果步驟2實作完成）
   - 確認 CLI 參數正確性：`subx sync --method vad` 和 `subx sync --method auto`
  let expected_position_in_segment = half_window;
  let actual_position_in_segment = first_speech_time;
  let offset_seconds = actual_position_in_segment - expected_position_in_segment;
  
  // 改為：直接比較絕對時間
  let expected_start = first_entry.start_time.as_secs_f64();
  let offset_seconds = first_speech_time - expected_start;
  ```

### 步驟 3: 更新 VAD 音訊處理器 - 支援多格式

**目標**: 移除 WAV 格式限制，利用 VAD 套件的多格式支援能力

**確切的程式碼修改**:

**檔案**: `src/services/vad/audio_processor.rs`
- **L48-99**: 重構 `load_wav_file` 方法，更名為 `load_audio_file`
- 移除 WAV 特定的格式檢查和 `WavReader` 依賴
- 研究 `voice_activity_detector` 套件的音訊載入 API，使用其原生多格式支援

### 步驟 4: 更新同步引擎 - 移除 Whisper 支援

**目標**: 從同步引擎中完全移除 Whisper 相關邏輯

**確切的程式碼修改**:

**檔案**: `src/core/sync/engine.rs`
- **L14**: 移除 `use crate::services::whisper::{AudioSegmentExtractor, WhisperSyncDetector};`
- **L20**: 移除 `whisper_detector: Option<WhisperSyncDetector>,` 欄位
- **L22**: 移除 `audio_extractor: AudioSegmentExtractor,` 欄位  
- **L28-36**: 移除 Whisper 檢測器初始化邏輯
- **L59**: 移除 `audio_extractor: AudioSegmentExtractor::new()?,`
- **L67-75**: 移除 `create_whisper_detector` 方法
- **L88-91**: 移除 `SyncMethod::WhisperApi => {...}` 分支
- **L179-191**: 移除 `whisper_detect_sync_offset` 方法
- **L222**: 從 `SyncMethod` 枚舉移除 `WhisperApi,` 變數
- **L293**: 移除 `("whisper", SyncMethod::WhisperApi),` 映射
- **L314**: 更新為 `preferred_methods: vec![SyncMethod::LocalVad],`

### 步驟 5: 移除 Whisper 配置

**目標**: 清理配置系統中的所有 Whisper 相關項目

**確切的程式碼修改**:

**檔案**: `src/config/mod.rs`
- **L200**: 移除 `pub whisper: WhisperConfig,` 欄位
- **L230-249**: 移除整個 `WhisperConfig` 結構定義
- **L277**: 移除 `whisper: WhisperConfig::default(),`
- **L290-304**: 移除 `WhisperConfig` 的 `Default` 實作
- **L514-535**: 移除 Whisper 相關配置建構邏輯

**檔案**: `src/config/validator.rs`
- **L9**: 移除 `use crate::config::{SyncConfig, VadConfig, WhisperConfig};` 中的 `WhisperConfig`
- **L104+**: 移除 `impl WhisperConfig` 整個實作區塊

### 步驟 6: 更新 CLI 介面

**目標**: 移除 CLI 中的 Whisper 相關參數和選項

**確切的程式碼修改**:

**檔案**: `src/cli/sync_args.rs`
- **L169-188**: 從 `SyncMethodArg` 枚舉移除 `Whisper,` 變數及相關實作
- **L376**: 移除 `crate::core::sync::SyncMethod::WhisperApi` 轉換案例

**檔案**: `src/commands/sync_command.rs`
- **L131**: 移除 `"whisper" => Ok(SyncMethod::WhisperApi),` 映射

### 步驟 7: 移除服務工廠中的 Whisper 建立邏輯

**目標**: 從服務建立邏輯中移除 Whisper 服務

**確切的程式碼修改**:

**檔案**: `src/core/services.rs`
- **L9**: 移除 `services::whisper::WhisperSyncDetector,` 匯入
- **L60-73**: 移除整個 `create_whisper_detector` 方法

### 步驟 8: 清理測試檔案

**目標**: 移除 Whisper 相關的測試檔案和測試案例

**實作內容**:
1. 刪除 Whisper 專用測試檔案
2. 從其他測試中移除 Whisper 相關的測試案例

**檔案異動**:
- 刪除: `tests/whisper_integration_tests.rs`
- 刪除: `tests/whisper_mock_tests.rs`
- 修改: `tests/sync_cli_integration_tests.rs` - 移除 L30, L47, L143, L232-233 等 Whisper 測試
- 修改: `tests/sync_new_architecture_tests.rs` - 移除 L217 等 Whisper 測試  
- 修改: `tests/sync_engine_integration_tests.rs` - 移除 Whisper 相關測試案例

### 步驟 9: 清理依賴套件

**目標**: 從 Cargo.toml 中移除不再需要的 Whisper 相關套件

**確切的程式碼修改**:

**檔案**: `Cargo.toml`
檢查以下依賴是否僅用於 Whisper 功能，如是則移除：
- **L56**: `tokio-util` (如果僅用於 Whisper multipart 上傳)
- 檢查其他 HTTP 相關套件的使用情況

### 步驟 10: 最終驗證和清理

**目標**: 確保系統完整性和功能正確性

**實作內容**:
1. 執行完整的編譯檢查
2. 運行測試套件確保沒有 Whisper 殘留引用
3. 檢查程式碼中是否有 TODO 或 FIXME 標記
4. 驗證 VAD 同步功能正常運作

**驗證命令**:
```bash
cargo check
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## 測試策略

### 編譯和基本驗證
- 確保所有 Whisper 引用完全移除，程式碼能夠編譯
- 驗證沒有 dead code 警告
- 確保 clippy 檢查通過

### VAD 功能測試
- 測試新的整個檔案處理邏輯
- 驗證多種音訊格式支援（MP3, MP4, WAV, OGG 等）
- 測試同步偏移計算的準確性
- 驗證錯誤處理機制

### CLI 介面測試
- 測試 `--method vad` 和 `--method auto` 參數
- 確保移除 `--method whisper` 後的錯誤處理
- 驗證配置檔案載入正常

### 回歸測試
- 確保其他音訊處理功能未受影響
- 確保配置系統完整性
- 驗證其他命令（match, convert）正常運作

## 潛在風險和緩解措施

### 風險 1: VAD 多格式支援實作困難
**描述**: `voice_activity_detector` 套件的多格式支援 API 可能與預期不符
**緩解措施**: 
- 詳細研究 VAD 套件文檔和範例
- 必要時保留音訊轉檔邏輯，但使用 `AudioTranscoder` 而非 Whisper 的擷取器
- 漸進式移除，先移除 Whisper 再優化 VAD

### 風險 2: 同步準確性下降
**描述**: 移除音訊片段分析可能影響同步檢測準確性
**緩解措施**:
- 完整測試不同類型的音訊檔案
- 保留原有的信心度計算邏輯
- 必要時增加音訊預處理邏輯

### 風險 3: 測試覆蓋不足
**描述**: 移除大量測試檔案後可能遺漏重要測試案例
**緩解措施**:
- 仔細審查每個被移除的測試，確保核心功能有對應的 VAD 測試
- 增加 VAD 邊界案例測試
- 保留錯誤處理測試

### 風險 4: 配置遷移問題
**描述**: 舊配置檔案包含 Whisper 設定可能導致載入失敗
**緩解措施**:
- 實作向後相容的配置讀取邏輯
- 提供清晰的錯誤訊息指導用戶更新配置
- 支援自動配置清理功能

## 完成標準

1. ✅ 所有 Whisper 相關程式碼檔案已完全移除
2. ✅ VAD 系統能夠處理完整音訊檔案，支援多種格式
3. ✅ 配置系統不再包含任何 Whisper 相關項目
4. ✅ CLI 介面只提供 VAD、Manual 和 Auto 選項
5. ✅ 同步引擎完全基於 VAD 運作
6. ✅ 所有測試通過，包括新的 VAD 完整檔案處理測試
7. ✅ 程式碼編譯無警告，通過 clippy 檢查
8. ✅ 依賴套件已清理，移除不必要的套件
9. ✅ 手動測試確認 VAD 同步功能正常運作
10. ✅ 不修改歷史文件和報告（保持完整性）

## 驗證檢查清單

### 程式碼檢查
- [ ] `src/services/whisper/` 目錄已完全刪除
- [ ] 所有檔案中無 `whisper` 相關匯入或引用
- [ ] `SyncMethod` 只包含 `LocalVad`、`Manual` 和 `Auto`
- [ ] `SyncConfig` 不包含 `WhisperConfig` 欄位
- [ ] CLI 參數不包含 `--whisper-*` 或 `--method whisper` 選項
- [ ] VAD 系統能直接處理多種音訊格式

### 測試檢查
- [ ] `tests/whisper_*.rs` 檔案已刪除
- [ ] 所有測試檔案中無 Whisper 相關測試案例
- [ ] VAD 整個檔案處理測試完整且通過
- [ ] 多格式音訊檔案測試通過
- [ ] 配置測試反映新的結構

### 功能檢查
- [ ] `subx sync --method vad` 正常運作
- [ ] `subx sync --method auto` 使用 VAD
- [ ] `subx sync --method manual` 提供適當錯誤訊息
- [ ] 支援 MP3, MP4, WAV, OGG 等格式的同步檢測
- [ ] 配置檔案載入正常
- [ ] 錯誤訊息正確且有意義

### 品質檢查
- [ ] `cargo check` 通過
- [ ] `cargo test` 全部通過
- [ ] `cargo clippy -- -D warnings` 無警告
- [ ] `cargo fmt --check` 格式正確
- [ ] 無 dead code 警告

---

**預估工時**: 12 小時  
**優先等級**: 高  
**複雜度**: 中等  
**依賴項目**: 無（可獨立執行）  
**後續項目**: 可能需要進一步優化 VAD 效能和準確性

## 注意事項

1. **保留歷史文件**: 不修改 `.github/plans/backlogs/32.2-whisper-api-integration.md` 和相關報告檔案
2. **漸進式移除**: 建議先移除 Whisper 程式碼，確保編譯通過，再優化 VAD 系統
3. **測試驗證**: 每個步驟完成後都要進行編譯檢查，確保沒有破壞其他功能
4. **配置向後相容**: 考慮舊配置檔案的處理，提供友善的遷移體驗
