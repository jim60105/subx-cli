# SubX 配置檔案使用情況分析

此文件分析 SubX 配置檔案中各項設定在程式碼中的實際使用情況，確保沒有多餘或未整合的配置。

## 配置設定使用分析表

### AI 配置 (`[ai]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 狀態 |
|---------|------|---------|-------------|---------|------|
| `provider` | String | "openai" | • `Config::validate()` 中驗證<br>• `ConfigValidator::validate_ai_config()` | 驗證 provider，目前只支援 "openai" | ✅ 使用中 |
| `api_key` | Option<String> | None | • env var `OPENAI_API_KEY` 載入<br>• `MatchCommand::execute()` 取得 API 金鑰<br>• `OpenAIClient::from_config()` 建立客戶端 | 用於 OpenAI API 認證 | ✅ 使用中 |
| `model` | String | "gpt-4o-mini" | • env var `SUBX_AI_MODEL` 覆寫<br>• `MatchCommand` 建立 AIClient<br>• `OpenAIClient` 構建與請求 | 指定使用的 OpenAI 模型 | ✅ 使用中 |
| `base_url` | String | "https://api.openai.com/v1" | • `OpenAIClient::from_config()` 讀取<br>• `OpenAIClient::validate_base_url()` 驗證<br>• `OpenAIClient::new_with_base_url()` 設定 | 支援自訂 API 端點 | ✅ 使用中 |
| `max_sample_length` | usize | 2000 | • `MatchCommand` 設定 `MatchConfig`<br>• `MatchEngine` 限制內容樣本長度 | 控制傳送給 AI 的內容長度上限 | ✅ 使用中 |
| `temperature` | f32 | 0.3 | • `MatchCommand` 建立 `OpenAIClient`<br>• `OpenAIClient::chat_completion()` 請求參數 | 控制 AI 回應的隨機性 | ✅ 使用中 |
| `retry_attempts` | u32 | 3 | • `MatchCommand` 建立 `OpenAIClient`<br>• `OpenAIClient` 重試邏輯 | API 請求失敗時的重試次數 | ✅ 使用中 |
| `retry_delay_ms` | u64 | 1000 | • `MatchCommand` 建立 `OpenAIClient`<br>• `OpenAIClient` 重試間隔 | API 重試之間的延遲時間 | ✅ 使用中 |

### 格式配置 (`[formats]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 狀態 |
|---------|------|---------|-------------|---------|------|
| `default_output` | String | "srt" | • `ConvertCommand::execute()` 設定預設格式<br>• `Config::get_value()` 取值方法 | CLI 轉換命令的預設輸出格式 | ✅ 使用中 |
| `preserve_styling` | bool | true | • `ConvertCommand::execute()` 設定 `ConversionConfig`<br>• `FormatConverter` 轉換配置 | 控制格式轉換時是否保留樣式 | ✅ 使用中 |
| `default_encoding` | String | "utf-8" | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 預設檔案編碼設定 | ⚠️ 已定義但未實際使用 |
| `encoding_detection_confidence` | f32 | 0.7 | • `EncodingDetector::new()` 讀取配置<br>• `EncodingDetector` 用於編碼檢測閾值 | 編碼自動檢測的信心度閾值 | ✅ 使用中 |

### 同步配置 (`[sync]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 狀態 |
|---------|------|---------|-------------|---------|------|
| `max_offset_seconds` | f32 | 30.0 | • `SyncCommand::execute()` 設定 `SyncConfig`<br>• `SyncEngine` 同步演算法 | 音訊字幕同步的最大偏移範圍 | ✅ 使用中 |
| `correlation_threshold` | f32 | 0.7 | • `SyncCommand::execute()` 設定 `SyncConfig`<br>• `SyncEngine` 相關性判斷 | 音訊相關性分析的閾值 | ✅ 使用中 |
| `dialogue_detection_threshold` | f32 | 0.01 | • `SyncCommand::execute()` 設定對話閾值<br>• `DialogueDetector` 語音檢測 | 對話片段檢測的敏感度 | ✅ 使用中 |
| `min_dialogue_duration_ms` | u64 | 500 | • `SyncCommand::execute()` 轉換為秒<br>• `DialogueDetector` 最小對話長度 | 最小對話片段持續時間 | ✅ 使用中 |
| `enable_dialogue_detection` | bool | true | • `SyncCommand::execute()` 控制是否啟用<br>• `DialogueDetector` 流程控制 | 是否啟用對話檢測功能 | ✅ 使用中 |
| `audio_sample_rate` | u32 | 16000 | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 音訊處理的採樣率 | ⚠️ 已定義但 `AusAudioAnalyzer` 硬編碼採樣率 |
| `dialogue_merge_gap_ms` | u64 | 500 | • `PartialConfig` 定義與合併<br>• `SyncConfig::resample_quality()` 方法存取 | 對話片段合併間隔 | ⚠️ 已定義但功能未實作 |
| `resample_quality` | String | "high" | • `PartialConfig` 定義與合併<br>• `SyncConfig::resample_quality()` 方法 | 音訊重採樣品質設定 | ⚠️ 已定義但重採樣器未使用 |
| `auto_detect_sample_rate` | bool | true | • `PartialConfig` 定義與合併<br>• `SyncConfig::auto_detect_sample_rate()` 方法 | 自動檢測音訊採樣率 | ⚠️ 已定義但功能未實作 |
| `enable_smart_resampling` | bool | true | • `PartialConfig` 定義與合併<br>• `SyncConfig::enable_smart_resampling()` 方法 | 啟用智慧重採樣 | ⚠️ 已定義但功能未實作 |

### 一般配置 (`[general]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 狀態 |
|---------|------|---------|-------------|---------|------|
| `backup_enabled` | bool | false | • `MatchCommand::execute_with_client()` 設定 `MatchConfig`<br>• `MatchEngine` 備份邏輯 | 檔案匹配時是否自動備份 | ✅ 使用中 |
| `max_concurrent_jobs` | usize | 4 | • `TaskScheduler::new()` 讀取配置<br>• `ConfigValidator` 驗證不為零 | 並行任務調度器的最大並發數 | ✅ 使用中 |
| `task_timeout_seconds` | u64 | 3600 | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 任務執行逾時設定 | ⚠️ 已定義但調度器未使用 |
| `enable_progress_bar` | bool | true | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 是否顯示進度條 | ⚠️ 已定義但未在 UI 中使用 |
| `worker_idle_timeout_seconds` | u64 | 300 | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 工作執行緒閒置逾時 | ⚠️ 已定義但調度器未使用 |

### 並行處理配置 (`[parallel]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 狀態 |
|---------|------|---------|-------------|---------|------|
| `cpu_intensive_limit` | usize | 2 | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | CPU 密集型任務限制 | ⚠️ 已定義但調度器未使用 |
| `io_intensive_limit` | usize | 8 | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | I/O 密集型任務限制 | ⚠️ 已定義但調度器未使用 |
| `task_queue_size` | usize | 100 | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 任務佇列大小限制 | ⚠️ 已定義但調度器未使用 |
| `enable_task_priorities` | bool | true | • `PartialConfig` 定義與合併<br>• `TaskScheduler` 內建優先級邏輯 | 啟用任務優先級排程 | ⚠️ 已定義，調度器有優先級但不讀取此設定 |
| `auto_balance_workers` | bool | true | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 自動平衡工作負載 | ⚠️ 已定義但負載平衡功能未實作 |

## 狀態說明

- ✅ **使用中**: 配置項目已完全整合並在程式碼中實際使用
- ⚠️ **已定義但未使用**: 配置項目已定義並可設定，但核心功能未實作或未讀取此設定
- ❌ **未使用**: 配置項目完全未在程式碼中使用（已移除此類別）

## 總結

### 完全整合的配置 (16 項)
- AI 配置: 全部 8 項都已使用
- 格式配置: 3/4 項已使用 (`encoding_detection_confidence` 已整合)
- 同步配置: 5/10 項已使用  
- 一般配置: 2/5 項已使用
- 並行配置: 0/5 項已使用

### 需要進一步整合的配置 (14 項)
主要集中在：
1. **音訊處理功能**: `audio_sample_rate`, `resample_quality`, `auto_detect_sample_rate`, `enable_smart_resampling`
2. **任務管理功能**: `task_timeout_seconds`, `enable_progress_bar`, `worker_idle_timeout_seconds`
3. **並行處理功能**: 所有 5 項並行配置都需要整合
4. **對話處理功能**: `dialogue_merge_gap_ms` 需要實作

這些配置項目都在配置系統中正確定義並可設定，但對應的功能實作尚未完成或未讀取配置。
