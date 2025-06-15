# Backlog 34: 清理文檔中的過時內容並簡化同步配置

## 概覽

SubX 在發布前已決定移除 OpenAI Whisper API 相關功能，採用純 VAD (Voice Activity Detection) 本地處理架構。同時，`sync.default_method` 配置已確定為冗餘設計需要移除。此任務需要全面清理文檔中的過時內容，簡化配置結構，並確保文檔反映最終的產品設計。

## 重要變更聲明

**OpenAI Whisper API 從未發布**: Whisper 功能在正式發布前已完全撤回，應被視為從未存在於 SubX 專案中。所有文檔和配置都應反映純 VAD 架構。

**移除冗餘配置**: `sync.default_method` 被確認為不必要的設計複雜性，將從所有文檔、配置示例和使用指南中完全移除。SubX 將採用直接的 VAD 處理，無需複雜的方法選擇邏輯。

## 背景

SubX 採用簡潔的設計理念，專注於本地 VAD 處理：

1. **Whisper 功能從未發布** - 在正式發布前已決定移除，無需任何向後兼容考慮
2. **移除冗餘配置** - `sync.default_method` 是冗餘設計，應完全移除
3. **純 VAD 架構** - 專注於隱私友好的本地音訊處理，VAD 是唯一自動同步方法

需要清理的文檔：
1. **技術架構文檔** (`docs/tech-architecture.md`) - 移除多方法描述，純化 VAD 架構說明
2. **配置指南** (`docs/configuration-guide.md`) - 移除 `sync.default_method` 配置，簡化同步配置
3. **README 檔案** (`README.md` 和 `README.zh-TW.md`) - 清理功能描述，突出 VAD 特色

## 目標

1. ✅ 清理所有文檔中的過時內容
2. ✅ 移除冗餘配置 `sync.default_method`，簡化同步配置結構
3. ✅ 統一 VAD 為唯一同步方法的文檔描述
4. ✅ 更新使用範例以反映簡化的 CLI 參數
5. ✅ 確保中英文檔案內容的一致性
6. ✅ 突出 VAD 本地處理的優勢和特色

## 當前代碼庫狀況分析

SubX 採用簡潔的純 VAD 架構，專注於本地音訊處理：

### 實際同步引擎架構

```rust
// src/core/sync/engine.rs
pub struct SyncEngine {
    config: SyncConfig,
    vad_detector: Option<VadSyncDetector>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncMethod {
    LocalVad,  // 本地 VAD 檢測（主要方法）
    Manual,    // 手動指定偏移量
}
```

### 簡化的配置結構

```rust
// src/config/mod.rs
pub struct SyncConfig {
    pub max_offset_seconds: f32,         // 預設：60.0
    pub vad: VadConfig,
    
    // 注意：移除冗餘的 default_method 欄位
}

pub struct VadConfig {
    pub enabled: bool,                   // 預設：true
    pub sensitivity: f32,               // 預設：0.75
    pub chunk_size: usize,              // 預設：512
    pub sample_rate: u32,               // 預設：16000
    pub padding_chunks: u32,            // 預設：3
    pub min_speech_duration_ms: u32,    // 預設：100
    pub speech_merge_gap_ms: u32,       // 預設：200
}
```

### 簡化的 CLI 參數

```rust
// src/cli/sync_args.rs
#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum SyncMethodArg {
    Vad,     // 使用本地 VAD（預設且主要方法）
    Manual,  // 手動偏移
}
```

### VAD 同步檢測器

```rust
// src/services/vad/sync_detector.rs
pub struct VadSyncDetector {
    vad_detector: LocalVadDetector,
}

impl VadSyncDetector {
    // 直接處理完整音訊檔案，無需時間窗口配置
    pub async fn detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
        _analysis_window_seconds: u32, // 已廢棄參數
    ) -> Result<SyncResult>
}
```

## 文檔修改範圍分析

### 1. 技術架構文檔 (`docs/tech-architecture.md`)

**需要移除的內容**：
- **多方法同步描述** - 移除任何關於方法選擇的複雜邏輯
- **過時的架構圖** - 簡化為單一 VAD 處理流程

**需要更新的內容**：
- **同步方法說明** - 簡化為 VAD 和手動兩種方法
- **VAD 處理流程** - 強調直接處理完整音訊檔案的優勢
- **架構圖** - 展示簡潔的 VAD 處理架構

### 2. 配置指南 (`docs/configuration-guide.md`)

**需要移除的內容**：
- **方法選擇策略** - 移除複雜的多方法選擇邏輯
- **冗餘配置項** - 移除 `default_method` 等不必要的配置

**需要更新的內容**：
- **同步配置** - 簡化為 VAD 配置和基本設定
- **配置範例** - 提供清晰簡潔的 VAD 配置範例
- **調優指南** - 專注於 VAD 參數調優

### 3. README 檔案 (`README.md`)

**需要移除的內容**：
- **複雜的功能描述** - 移除關於多種同步方法的描述
- **過時的配置範例** - 清理不再相關的配置設定

**需要更新的內容**：
- **功能特色** - 突出 VAD 本地處理的優勢
- **使用範例** - 簡化 CLI 使用範例
- **配置指南** - 專注於 VAD 相關配置

### 4. 繁體中文 README (`README.zh-TW.md`)

**需要進行與英文版對等的修改**，同時確保：
- 技術術語翻譯一致：VAD = "語音活動檢測"
- 保持中文表達的自然性和簡潔性
- 配置範例和使用指南完全對等

## 實作步驟（更新）

### 步驟 1: 更新技術架構文檔

**目標**: 完全移除 Whisper 描述，準確反映當前的純 VAD 架構

**具體修改**:

**檔案**: `docs/tech-architecture.md`

1. **更新同步方法說明** (L198-210 附近):
```rust
// 修改前
#[derive(Debug, Clone)]
pub enum SyncMethod {
    WhisperAPI,     // OpenAI Whisper API 雲端語音識別
    Manual,         // 手動指定偏移量
}

// 修改後
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncMethod {
    Vad,       // 本地 VAD 檢測
    Manual,    // 手動指定偏移量
}
```

2. **移除 Whisper API 整個章節** (L233-280 附近)

3. **更新 VAD 處理流程說明**:
```markdown
**VAD 處理流程**（已重構）:
1. **直接檔案處理**: 不再限制於時間窗口，直接分析完整音訊檔案
2. **語音檢測**: 使用 voice_activity_detector crate 檢測整個檔案的語音活動
3. **時間對應**: 比較檢測到的第一個語音段與第一句字幕的開始時間
4. **偏移計算**: 直接計算時間差異，無需音訊片段提取
5. **隱私保護**: 所有處理都在本地進行，無需網路連接
```

4. **更新引擎架構說明**:
```markdown
**重構後架構**:
```
SyncEngine
└── VAD Detector (VadSyncDetector)
    └── 直接處理完整音訊檔案

SyncMethod: Vad | Manual
```
```

### 步驟 2: 更新配置指南

**目標**: 完全移除 Whisper 配置，更新為純 VAD 配置系統

**具體修改**:

**檔案**: `docs/configuration-guide.md`

1. **更新同步方法概述** (L75-85 附近):
```markdown
<!-- 修改前 -->
SubX supports three main synchronization methods:

1. **OpenAI Whisper API** - Cloud-based speech recognition with high accuracy
2. **Local VAD (Voice Activity Detection)** - On-device speech detection
3. **Manual Offset** - User-specified time adjustment

<!-- 修改後 -->
SubX supports two main synchronization methods:

1. **Local VAD (Voice Activity Detection)** - Privacy-focused on-device speech detection with full audio file processing
2. **Manual Offset** - User-specified time adjustment for precise control
```

2. **更新基礎配置示例** (L85-105 附近):
```toml
# 修改前
[sync]
default_method = "whisper"           # Default sync method ("whisper", "vad", "manual")
analysis_window_seconds = 30         # Analysis window: seconds before/after first subtitle (u32)
max_offset_seconds = 60.0

# OpenAI Whisper API configuration
[sync.whisper]
enabled = true
model = "whisper-1"
# ... 其他 Whisper 設定

# 修改後
[sync]
max_offset_seconds = 60.0            # Maximum allowed time offset in seconds (f32)

# Local VAD configuration
[sync.vad]
enabled = true                       # Enable local VAD method (bool)
sensitivity = 0.75                   # Speech detection sensitivity (0.0-1.0) (f32)
chunk_size = 512                     # Audio chunk size in samples (usize)
sample_rate = 16000                  # Processing sample rate in Hz (u32)
padding_chunks = 3                   # Padding chunks before and after speech detection (u32)
min_speech_duration_ms = 100         # Minimum speech duration in milliseconds (u32)
speech_merge_gap_ms = 200            # Speech segment merge gap in milliseconds (u32)
```

3. **移除複雜的方法選擇策略** (L114-125 附近):
```markdown
<!-- 移除前 -->
The sync engine can automatically select the best method based on availability and configuration:

- **Auto Selection**: Tries Whisper first, falls back to VAD if confidence is too low
- **Forced Method**: Uses only the specified method without fallback
- **Hybrid Approach**: Combines results from multiple methods for better accuracy

<!-- 修改後：簡化為 -->
The sync engine uses optimized local VAD processing for reliable speech detection:

- **VAD Processing**: Uses Voice Activity Detection with optimized parameters for local processing
- **Manual Offset**: Applies user-specified time adjustments directly without analysis
```

4. **添加 VAD 調優指南**:
```markdown
#### VAD Fine-tuning

```toml
[sync.vad]
# For quiet speech or background noise
sensitivity = 0.8              # Higher sensitivity for difficult audio
chunk_size = 1024             # Larger chunks for better accuracy
sample_rate = 22050           # Higher sample rate for quality
padding_chunks = 5            # More padding for complex transitions

# For clear speech with minimal noise
sensitivity = 0.6             # Lower sensitivity to avoid false positives
chunk_size = 256              # Smaller chunks for faster processing
min_speech_duration_ms = 50   # Shorter minimum for rapid speech
speech_merge_gap_ms = 300     # Larger gaps for natural pauses
```
```

5. **移除 Whisper 相關內容** (L122-140 和 L227-235):
- 完全移除 "Whisper API Options" 章節
- 移除所有 Whisper 環境變數示例

### 步驟 3: 更新英文 README

**目標**: 準確反映當前的純 VAD 功能，移除所有 Whisper 相關內容

**具體修改**:

**檔案**: `README.md`

1. **更新功能特色** (L8-12 附近):
```markdown
<!-- 修改前 -->
- 🔊 **Audio Transcoding** - Auto-transcode diverse audio container formats (MP4, MKV, WebM, OGG) to WAV for synchronization analysis
- ⏰ **Timeline Correction** - Automatically detects and corrects subtitle timing offset issues

<!-- 修改後 -->
- 🔊 **Audio Processing** - Direct processing of various audio formats using advanced Voice Activity Detection
- ⏰ **Timeline Correction** - Privacy-focused local VAD automatically detects and corrects subtitle timing offset issues
```

2. **更新時間軸校正使用範例**:
```bash
# 修改前
**Timeline Correction**
```bash
# Automatic synchronization (requires video file)
subx sync video.mp4 subtitle.srt

# Manual synchronization (only subtitle file required)  
subx sync --offset 2.5 subtitle.srt

# Batch processing mode (video directory required)
subx sync --batch /path/to/media/folder

# Backward compatibility (legacy format still supported)
subx sync video.mp4 subtitle.srt --offset 2.5
```

# 修改後
**Timeline Correction**
```bash
# Automatic VAD synchronization (requires audio/video file)
subx sync video.mp4 subtitle.srt

# Manual synchronization (only subtitle file required)  
subx sync --offset 2.5 subtitle.srt

# VAD with custom sensitivity
subx sync --vad-sensitivity 0.8 video.mp4 subtitle.srt

# Batch processing mode (processes entire directories)
subx sync --batch /path/to/media/folder
```

3. **更新快速配置** (L35-45 附近):
```bash
# 修改前
# Set OpenAI API Key (for AI matching functionality)
export OPENAI_API_KEY="your-api-key-here"

# Optional: Set custom OpenAI Base URL (for OpenAI API or private deployment)
export OPENAI_BASE_URL="https://api.openai.com/v1"

# Or set through configuration commands
subx config set ai.api_key "your-api-key-here"
subx config set ai.base_url "https://api.openai.com/v1"
subx config set ai.model "gpt-4.1-mini"

# 修改後
# Set OpenAI API Key (for AI matching functionality only)
export OPENAI_API_KEY="your-api-key-here"

# Configure VAD settings
subx config set sync.vad.sensitivity 0.8
subx config set sync.vad.enabled true

# Enable general backup feature
subx config set general.backup_enabled true
```

4. **更新疑難排解** (L180-200 附近):
```markdown
<!-- 修改前 -->
### Q: Timeline synchronization failed?

A: Ensure the video file is accessible and check if the file format is supported. If automatic sync doesn't work well, try:
- Manual offset: `subx sync --offset <seconds> subtitle.srt`
- Backward compatibility: `subx sync --offset <seconds> video.mp4 subtitle.srt`
- Adjust sync configuration: `subx config set sync.correlation_threshold 0.6`
- Enable dialogue detection: `subx config set sync.enable_dialogue_detection true`

<!-- 修改後 -->
### Q: Timeline synchronization failed?

A: Ensure the audio/video file is accessible and check if the file format is supported. If VAD sync doesn't work well, try:
- Adjust VAD sensitivity: `subx config set sync.vad.sensitivity 0.8` (higher for quiet audio)
- Use manual offset for difficult cases: `subx sync --offset <seconds> subtitle.srt`
- Check VAD configuration: `subx config set sync.vad.enabled true`
- For very noisy audio: `subx config set sync.vad.min_speech_duration_ms 200`
- For rapid speech: `subx config set sync.vad.speech_merge_gap_ms 100`
```

### 步驟 4: 更新繁體中文 README

**目標**: 與英文版保持對等，確保中文表達自然流暢

**具體修改**:

**檔案**: `README.zh-TW.md`

1. **更新功能特色** (L8-12 附近):
```markdown
<!-- 修改前 -->
- 🔊 **音訊轉碼** - 自動將多種音訊容器格式 (MP4、MKV、WebM、OGG) 轉為 WAV 以進行同步分析
- ⏰ **時間軸校正** - 自動檢測並修正字幕時間偏移問題

<!-- 修改後 -->
- 🔊 **音訊處理** - 使用先進的語音活動檢測技術直接處理多種音訊格式
- ⏰ **時間軸校正** - 採用隱私保護的本地 VAD 技術自動檢測並修正字幕時間偏移問題
```

2. **更新時間軸校正使用範例**:
```bash
# 修改前
**時間軸校正**
```bash
# 自動同步（需要視頻檔案）
subx sync video.mp4 subtitle.srt

# 手動同步（僅需字幕檔案）
subx sync --offset 2.5 subtitle.srt

# 批量處理模式（需要視頻資料夾）
subx sync --batch /path/to/media/folder

# 向後相容（舊格式仍然支援）
subx sync video.mp4 subtitle.srt --offset 2.5
```

# 修改後
**時間軸校正**
```bash
# 自動 VAD 同步（需要音訊/視頻檔案）
subx sync video.mp4 subtitle.srt

# 手動同步（僅需字幕檔案）
subx sync --offset 2.5 subtitle.srt

# 明確指定 VAD 方法並自訂靈敏度
subx sync --method vad --vad-sensitivity 0.8 video.mp4 subtitle.srt

# 批量處理模式（處理整個資料夾）
subx sync --batch /path/to/media/folder
```

3. **更新快速配置** (L35-45 附近):
```bash
# 修改前
# 設定 OpenAI API Key (用於 AI 匹配功能)
export OPENAI_API_KEY="your-api-key-here"

# 可選：設定自訂 OpenAI Base URL (用於 OpenAI API 或私有部署)
export OPENAI_BASE_URL="https://api.openai.com/v1"

# 或通過配置檔案指令設定
subx config set ai.api_key "your-api-key-here"
subx config set ai.base_url "https://api.openai.com/v1"
subx config set ai.model "gpt-4.1-mini"

# 修改後
# 設定 OpenAI API Key (僅用於 AI 匹配功能)
export OPENAI_API_KEY="your-api-key-here"

# 配置 VAD 設定
subx config set sync.vad.sensitivity 0.8
subx config set sync.vad.enabled true

# 啟用一般備份功能
subx config set general.backup_enabled true
```

4. **更新疑難排解**:
```markdown
<!-- 修改前 -->
### Q: 時間軸同步失敗？

A: 確保影片檔案可存取，並檢查檔案格式是否支援。如果自動同步不理想，可以嘗試：
- 手動指定偏移量：`subx sync --offset <seconds> subtitle.srt`
- 向後相容：`subx sync --offset <seconds> video.mp4 subtitle.srt`
- 調整同步配置：`subx config set sync.correlation_threshold 0.6`
- 啟用對話檢測：`subx config set sync.enable_dialogue_detection true`

<!-- 修改後 -->
### Q: 時間軸同步失敗？

A: 確保音訊/視頻檔案可存取，並檢查檔案格式是否支援。如果 VAD 同步不理想，可以嘗試：
- 調整 VAD 靈敏度：`subx config set sync.vad.sensitivity 0.8`（較高值適用於安靜音訊）
- 針對困難案例使用手動偏移：`subx sync --offset <seconds> subtitle.srt`
- 檢查 VAD 配置：`subx config set sync.vad.enabled true`
- 針對非常嘈雜的音訊：`subx config set sync.vad.min_speech_duration_ms 200`
- 針對快速語音：`subx config set sync.vad.speech_merge_gap_ms 100`
```

### 步驟 5: 文檔一致性和品質檢查

**目標**: 確保所有文檔內容準確、一致且可執行

**檢查項目**:

1. **術語一致性檢查**:
   - 搜索確認無 "whisper", "Whisper" 殘留引用（AI 配置除外）
   - 確保 VAD 全稱統一為 "Voice Activity Detection" / "語音活動檢測"
   - 確認同步方法名稱一致：`vad`, `manual`

2. **配置範例驗證**:
   - 確認 VAD 配置參數與 `VadConfig` 結構對應
   - 檢查配置範例的 TOML 語法正確性
   - 驗證移除了所有 `sync.default_method` 配置引用

3. **CLI 範例驗證**:
   - 確認所有 CLI 範例使用有效參數
   - 驗證移除了 `--method` 參數的複雜選擇
   - 確認不再包含 `--method whisper` 等無效選項

4. **中英文對等性檢查**:
   - 逐章節對比中英文版本內容結構
   - 確認技術術語翻譯一致
   - 驗證配置範例和使用指南完全對等

## 驗證檢查清單（更新）

### 代碼一致性驗證
- [ ] 確認 `SyncMethod` 枚舉只包含 `Vad`, `Manual`
- [ ] 確認完全移除了 `sync.default_method` 配置
- [ ] 確認 VAD 配置參數與代碼一致
- [ ] 檢查 CLI 參數只保留必要的 VAD 和手動選項

### 文檔完整性檢查
- [ ] 搜索確認無 "whisper" 字串殘留（除 .github/ 資料夾為歷史文件 ）
- [ ] 搜索確認無 "Whisper" 字串殘留（除 .github/ 資料夾為歷史文件 ）
- [ ] 驗證所有配置範例語法正確且可載入

### 功能準確性驗證
- [ ] 測試所有文檔中的 CLI 使用範例
- [ ] 驗證 VAD 配置範例可正常載入
- [ ] 確認錯誤訊息和疑難排解有效
- [ ] 檢查 VAD 方法描述與實際行為一致

### 使用者體驗檢查
- [ ] 文檔結構清晰易懂
- [ ] 使用範例實用且完整
- [ ] 配置指南詳細且準確
- [ ] 疑難排解覆蓋常見 VAD 相關問題

## 完成標準（更新）

### 技術準確性標準
1. ✅ 所有同步方法描述與當前 `SyncMethod` 枚舉一致
2. ✅ VAD 處理流程描述準確反映直接檔案處理實作
3. ✅ 配置參數說明與 `SyncConfig` 和 `VadConfig` 結構對應
4. ✅ CLI 參數範例與 `SyncMethodArg` 枚舉一致
5. ✅ 錯誤處理和疑難排解針對 VAD 相關問題

### 文檔品質標準
1. ✅ 完全移除所有 Whisper 相關內容和配置範例
2. ✅ 所有配置範例與當前代碼實作一致且可執行
3. ✅ 技術術語使用一致且準確
4. ✅ 中英文版本內容完全對等
5. ✅ VAD 功能描述詳細且實用

### 向後相容性標準
1. ✅ 錯誤訊息清楚指出 Whisper 功能從未發布
2. ✅ 提供 VAD 作為唯一自動同步方案的清晰指導
3. ✅ 移除所有 `sync.default_method` 配置引用
4. ✅ 保持配置載入的穩定性和友善性

---

**預估工時**: 8 小時  
**優先等級**: 高  
**複雜度**: 中等  
**依賴項目**: Backlog 33 (已完成 - Whisper 完全移除)  
**驗證需求**: 深度代碼分析和文檔一致性檢查  
**品質標準**: 文檔必須準確反映當前純 VAD 架構實作

## 重要注意事項

1. **代碼驗證優先**: 所有文檔修改必須基於對當前代碼庫的準確理解
2. **實作一致性**: 確保文檔描述與實際 `SyncEngine`, `SyncMethod`, `VadSyncDetector` 實作完全一致
3. **配置準確性**: 所有配置範例必須與 `SyncConfig` 和 `VadConfig` 結構對應
4. **CLI 參數正確性**: 確保所有 CLI 範例使用當前有效的 `SyncMethodArg` 選項
5. **VAD 特色突出**: 強調本地處理、隱私保護、直接檔案處理等 VAD 優勢

此計劃完成後，SubX 的文檔將完全準確地反映當前的純 VAD 架構，為使用者提供可靠且實用的使用指南。
