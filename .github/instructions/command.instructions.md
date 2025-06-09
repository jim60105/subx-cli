# SubX 配置檔案使用情況分析

此文件分析 SubX 配置檔案中各項設定在程式碼中的實際使用情況，確保沒有多餘或未整合的配置。

## 配置設定使用分析表

### AI 配置 (`[ai]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `provider` | String | "openai" | **呼叫樹:**<br>• `Config::validate()` (line 361) → `src/config.rs:361`<br>• 測試中在 `test_config_validation_invalid_provider()` (line 122) → `src/config.rs:122`<br><br>**注意：** `validate()` 函式存在但未在實際配置載入流程中被呼叫，僅在測試中使用。所有 command 硬編碼建立 `OpenAIClient`，不會根據 provider 切換實作。 | 僅用於驗證 provider 必須為 "openai"，不影響實際 command 行為 | 無 | ⚠️ 已定義但未被 command 實際使用 |
| `api_key` | Option<String> | None | **呼叫樹:**<br>• `EnvSource::load()` (line 68) → `src/config/source.rs:68`<br>• `MatchCommand::execute()` (line 17-21) → `src/commands/match_command.rs:17-21`<br>• `OpenAIClient::from_config()` (line 175-177) → `src/services/ai/openai.rs:175-177`<br>• `AIConfigValidator::validate()` (line 19) → `src/config/validator.rs:19` | 用於 OpenAI API 認證 | `subx-cli match` | ✅ 使用中 |
| `model` | String | "gpt-4o-mini" | **呼叫樹:**<br>• `EnvSource::load()` (line 71) → `src/config/source.rs:71`<br>• `MatchCommand::execute()` (line 24) → `src/commands/match_command.rs:24`<br>• `OpenAIClient::new()` 接收參數<br>• `OpenAIClient::chat_completion()` (line 216) → `src/services/ai/openai.rs:216`<br>• `AIConfigValidator::validate()` (line 34) → `src/config/validator.rs:34` | 指定使用的 OpenAI 模型 | `subx-cli match`, `subx-cli config` | ✅ 使用中 |
| `base_url` | String | "https://api.openai.com/v1" | **呼叫樹:**<br>• `EnvSource::load()` (line 77) → `src/config/source.rs:77`<br>• `OpenAIClient::from_config()` (line 181, 187) → `src/services/ai/openai.rs:181,187`<br>• `OpenAIClient::validate_base_url()` (line 181) → `src/services/ai/openai.rs:181`<br>• `OpenAIClient::new_with_base_url()` (line 167) → `src/services/ai/openai.rs:167`<br>• `OpenAIClient::chat_completion()` (line 224) → `src/services/ai/openai.rs:224`<br>• `AIConfigValidator::validate()` (line 56) → `src/config/validator.rs:56`<br><br>**注意：** `from_config()` 方法雖然支援 base_url，但實際的 match command 使用 `new()` 方法，該方法使用硬編碼的預設值。 | 支援自訂 API 端點 | 無（未被任何命令實際使用） | ⚠️ 已定義但未被 command 實際使用 |
| `max_sample_length` | usize | 2000 | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 38) → `src/commands/match_command.rs:38`<br>• `MatchEngine::create_content_preview()` (line 284-285) → `src/core/matcher/engine.rs:284-285` | 控制傳送給 AI 的內容長度上限 | `subx-cli match` | ✅ 使用中 |
| `temperature` | f32 | 0.3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 25) → `src/commands/match_command.rs:25`<br>• `OpenAIClient::new()` 接收參數<br>• `OpenAIClient::chat_completion()` (line 218) → `src/services/ai/openai.rs:218`<br>• `AIConfigValidator::validate()` (line 43) → `src/config/validator.rs:43` | 控制 AI 回應的隨機性 | `subx-cli match` | ✅ 使用中 |
| `retry_attempts` | u32 | 3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 26) → `src/commands/match_command.rs:26`<br>• `OpenAIClient::new()` 接收參數<br>• `OpenAIClient::make_request_with_retry()` (line 297) → `src/services/ai/openai.rs:297`<br>• `AIConfigValidator::validate()` (line 49) → `src/config/validator.rs:49` | API 請求失敗時的重試次數 | `subx-cli match` | ✅ 使用中 |
| `retry_delay_ms` | u64 | 1000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 27) → `src/commands/match_command.rs:27`<br>• `OpenAIClient::new()` 接收參數<br>• `OpenAIClient::make_request_with_retry()` (line 299) → `src/services/ai/openai.rs:299` | API 重試之間的延遲時間 | `subx-cli match` | ✅ 使用中 |

### 格式配置 (`[formats]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_output` | String | "srt" | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 19, 26) → `src/commands/convert_command.rs:19,26`<br>• `FormatsConfigValidator::validate()` (line 141) → `src/config/validator.rs:141`<br>• `Config::get_value()` (line 385) → `src/config.rs:385` | CLI 轉換命令的預設輸出格式 | `subx-cli convert`, `subx-cli config` | ✅ 使用中 |
| `preserve_styling` | bool | true | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 11) → `src/commands/convert_command.rs:11`<br>• `SrtToAssTransformer::transform()` (line 42) → `src/core/formats/transformers.rs:42`<br>• 類似在其他轉換器中使用 (line 56, 86) | 控制格式轉換時是否保留樣式 | `subx-cli convert` | ✅ 使用中 |
| `default_encoding` | String | "utf-8" | **呼叫樹:**<br>• `EncodingDetector::detect_file()` (line 302) → `src/core/formats/encoding/detector.rs:302`<br>• `FormatsConfigValidator::validate()` (line 147) → `src/config/validator.rs:147`<br>• 當檢測信心度低於閾值時作為回退編碼 | 預設檔案編碼設定 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |
| `encoding_detection_confidence` | f32 | 0.7 | **呼叫樹:**<br>• `EncodingDetector::new()` (line 19) → `src/core/formats/encoding/detector.rs:19`<br>• `EncodingDetector::detect_file()` (line 294) → `src/core/formats/encoding/detector.rs:294`<br>• 被 `DetectEncodingCommand` (line 8) → `src/commands/detect_encoding_command.rs:8`<br>• 被 `FormatConverter` (line 152) → `src/core/formats/converter.rs:152`<br>• 被 `FileManager` (line 61, 81) → `src/core/formats/manager.rs:61,81` | 編碼自動檢測的信心度閾值 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |

### 同步配置 (`[sync]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `max_offset_seconds` | f32 | 30.0 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 16) → `src/commands/sync_command.rs:16`<br>• `SyncEngine::find_best_offset()` (line 95) → `src/core/sync/engine.rs:95`<br>• `SyncConfigValidator::validate()` (line 116) → `src/config/validator.rs:116` | 音訊字幕同步的最大偏移範圍 | `subx-cli sync` | ✅ 使用中 |
| `correlation_threshold` | f32 | 0.7 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 17-19) → `src/commands/sync_command.rs:17-19`<br>• `SyncEngine::find_best_offset()` (line 112) → `src/core/sync/engine.rs:112`<br>• `SyncConfigValidator::validate()` (line 122) → `src/config/validator.rs:122` | 音訊相關性分析的閾值 | `subx-cli sync` | ✅ 使用中 |
| `dialogue_detection_threshold` | f32 | 0.01 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 20) → `src/commands/sync_command.rs:20`<br>• `DialogueDetector::new()` (line 18) → `src/core/sync/dialogue/detector.rs:18`<br>• `EnergyAnalyzer::new()` 接收參數 | 對話片段檢測的敏感度 | `subx-cli sync` | ✅ 使用中 |
| `min_dialogue_duration_ms` | u64 | 500 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 21) → `src/commands/sync_command.rs:21`<br>• `DialogueDetector::new()` (line 19) → `src/core/sync/dialogue/detector.rs:19`<br>• `EnergyAnalyzer::new()` 接收參數 | 最小對話片段持續時間 | `subx-cli sync` | ✅ 使用中 |
| `enable_dialogue_detection` | bool | true | **呼叫樹:**<br>• `SyncCommand::execute()` (line 25) → `src/commands/sync_command.rs:25`<br>• `DialogueDetector::detect_dialogue()` (line 30) → `src/core/sync/dialogue/detector.rs:30` | 是否啟用對話檢測功能 | `subx-cli sync` | ✅ 使用中 |
| `audio_sample_rate` | u32 | 16000 | **呼叫樹:**<br>• `ResampleConfig::from_config()` (line 63) → `src/services/audio/resampler/converter.rs:63`<br>• `AudioResampler::from_config()` 接收參數<br>• `DialogueDetector::load_audio()` (line 46) → `src/core/sync/dialogue/detector.rs:46` | 音訊處理的採樣率 | `subx-cli sync`（透過 DialogueDetector） | ✅ 使用中 |
| `dialogue_merge_gap_ms` | u64 | 500 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 26) → `src/commands/sync_command.rs:26`<br>• `DialogueDetector::new()` 載入配置<br>• `DialogueDetector::optimize_segments()` (line 49) → `src/core/sync/dialogue/detector.rs:49`<br>• 用於對話片段合併的間隔時間計算 | 對話片段合併間隔 | `subx-cli sync`（透過 DialogueDetector） | ✅ 使用中 |
| `resample_quality` | String | "high" | **呼叫樹:**<br>• `ResampleConfig::from_config()` (line 64) → `src/services/audio/resampler/converter.rs:64`<br>• `ResampleQuality::from_string()` (line 28) → `src/services/audio/resampler/converter.rs:28`<br>• `AudioResampler::create_interpolator()` 使用品質設定 | 音訊重採樣品質設定 | `subx-cli sync`（透過 DialogueDetector） | ✅ 使用中 |
| `auto_detect_sample_rate` | bool | true | • `PartialConfig` 定義與合併<br>• `SyncConfig::auto_detect_sample_rate()` 方法 | 自動檢測音訊採樣率 | 無（功能未實作） | ⚠️ 已定義但功能未實作 |
| `enable_smart_resampling` | bool | true | • `PartialConfig` 定義與合併<br>• `SyncConfig::enable_smart_resampling()` 方法 | 啟用智慧重採樣 | 無（功能未實作） | ⚠️ 已定義但功能未實作 |

### 一般配置 (`[general]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `backup_enabled` | bool | false | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 41) → `src/commands/match_command.rs:41`<br>• `MatchEngine::apply_operations()` (line 324) → `src/core/matcher/engine.rs:324`<br>• `EnvSource::load()` (line 79-80) → `src/config/source.rs:79-80` | 檔案匹配時是否自動備份 | `subx-cli match` | ✅ 使用中 |
| `max_concurrent_jobs` | usize | 4 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 68) → `src/core/parallel/scheduler.rs:68`<br>• `MatchCommand::batch_match_directory()` (line 69) → `src/commands/match_command.rs:69`<br>• `GeneralConfigValidator::validate()` (line 174) → `src/config/validator.rs:174` | 並行任務調度器的最大並發數 | `subx-cli match`（批次處理模式） | ✅ 使用中 |
| `task_timeout_seconds` | u64 | 3600 | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 任務執行逾時設定 | 無（調度器未使用） | ⚠️ 已定義但調度器未使用 |
| `enable_progress_bar` | bool | true | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 是否顯示進度條 | 無（UI 未使用） | ⚠️ 已定義但未在 UI 中使用 |
| `worker_idle_timeout_seconds` | u64 | 300 | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 工作執行緒閒置逾時 | 無（調度器未使用） | ⚠️ 已定義但調度器未使用 |

### 並行處理配置 (`[parallel]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `cpu_intensive_limit` | usize | 2 | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | CPU 密集型任務限制 | 無（調度器未使用） | ⚠️ 已定義但調度器未使用 |
| `io_intensive_limit` | usize | 8 | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | I/O 密集型任務限制 | 無（調度器未使用） | ⚠️ 已定義但調度器未使用 |
| `task_queue_size` | usize | 100 | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 任務佇列大小限制 | 無（調度器未使用） | ⚠️ 已定義但調度器未使用 |
| `enable_task_priorities` | bool | true | • `PartialConfig` 定義與合併<br>• `TaskScheduler` 內建優先級邏輯 | 啟用任務優先級排程 | 無（調度器有優先級但不讀取此設定） | ⚠️ 已定義，調度器有優先級但不讀取此設定 |
| `auto_balance_workers` | bool | true | • `PartialConfig` 定義與合併<br>• 可透過配置管理系統設定 | 自動平衡工作負載 | 無（功能未實作） | ⚠️ 已定義但負載平衡功能未實作 |

## 狀態說明

- ✅ **使用中**: 配置項目已完全整合並在程式碼中實際使用
- ⚠️ **已定義但未使用**: 配置項目已定義並可設定，但核心功能未實作或未讀取此設定
- ❌ **未使用**: 配置項目完全未在程式碼中使用（已移除此類別）

## 總結

### 完全整合的配置 (18 項) - 含詳細呼叫樹
- **AI 配置**: 7/8 項已使用，包含完整的從環境變數載入到實際 API 呼叫的路徑（ai.provider 僅用於驗證）
- **格式配置**: 3/4 項已使用，包含編碼檢測、格式轉換流程
- **同步配置**: 6/10 項已使用，主要在 SyncCommand 和相關引擎中使用，包含音訊處理和對話檢測
- **一般配置**: 2/5 項已使用，包含備份和並行任務調度

### 需要進一步整合的配置 (12 項)
主要集中在：
1. **AI 配置**: `provider` （僅用於驗證，實際 command 硬編碼 OpenAI）
2. **音訊處理功能**: `auto_detect_sample_rate`, `enable_smart_resampling` （audio_sample_rate, resample_quality 已整合）
3. **任務管理功能**: `task_timeout_seconds`, `enable_progress_bar`, `worker_idle_timeout_seconds`
4. **並行處理功能**: 所有 5 項並行配置都需要整合

這些配置項目都在配置系統中正確定義並可設定，但對應的功能實作尚未完成或未讀取配置。
