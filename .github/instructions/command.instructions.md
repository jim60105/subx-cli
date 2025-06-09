# SubX 配置檔案使用情況分析

此文件分析 SubX 配置檔案中各項設定在程式碼中的實際使用情況，確保沒有多餘或未整合的配置。

## 配置設定使用分析表

### AI 配置 (`[ai]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `provider` | String | "openai" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 18) → `src/commands/match_command.rs:18`<br>• `AIClientFactory::create_client()` (line 11-15) → `src/services/ai/factory.rs:11-15`<br>• `OpenAIClient::from_config()` 根據 provider 建立實例<br>• `Config::validate()` (line 361) → `src/config.rs:361`<br>• `AIConfigValidator::validate()` (line 20) → `src/config/validator.rs:20` | 用於選擇 AI 提供商類型，目前支援 "openai" | `subx-cli match` | ✅ 使用中 |
| `api_key` | Option<String> | None | **呼叫樹:**<br>• `EnvSource::load()` (line 68) → `src/config/source.rs:68`<br>• `MatchCommand::execute()` (line 17-21) → `src/commands/match_command.rs:17-21`<br>• `OpenAIClient::from_config()` (line 175-177) → `src/services/ai/openai.rs:175-177`<br>• `AIConfigValidator::validate()` (line 19) → `src/config/validator.rs:19` | 用於 OpenAI API 認證 | `subx-cli match` | ✅ 使用中 |
| `model` | String | "gpt-4o-mini" | **呼叫樹:**<br>• `EnvSource::load()` (line 71) → `src/config/source.rs:71`<br>• `MatchCommand::execute()` (line 24) → `src/commands/match_command.rs:24`<br>• `OpenAIClient::new()` 接收參數<br>• `OpenAIClient::chat_completion()` (line 216) → `src/services/ai/openai.rs:216`<br>• `AIConfigValidator::validate()` (line 34) → `src/config/validator.rs:34` | 指定使用的 OpenAI 模型 | `subx-cli match` | ✅ 使用中 |
| `base_url` | String | "https://api.openai.com/v1" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 18) → `src/commands/match_command.rs:18`<br>• `AIClientFactory::create_client()` → `src/services/ai/factory.rs:11`<br>• `OpenAIClient::from_config()` (line 212, 223-229) → `src/services/ai/openai.rs:212,223-229`<br>• `OpenAIClient::validate_base_url()` (line 221) → `src/services/ai/openai.rs:221`<br>• `OpenAIClient::new_with_base_url()` (line 187) → `src/services/ai/openai.rs:187`<br>• `OpenAIClient::chat_completion()` 使用此 URL 發送請求 | 支援自訂 API 端點，完整從配置到實際 HTTP 請求的路徑 | `subx-cli match` | ✅ 使用中 |
| `max_sample_length` | usize | 2000 | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 38) → `src/commands/match_command.rs:38`<br>• `MatchEngine::create_content_preview()` (line 284-285) → `src/core/matcher/engine.rs:284-285` | 控制傳送給 AI 的內容長度上限 | `subx-cli match` | ✅ 使用中 |
| `temperature` | f32 | 0.3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 25) → `src/commands/match_command.rs:25`<br>• `OpenAIClient::new()` 接收參數<br>• `OpenAIClient::chat_completion()` (line 218) → `src/services/ai/openai.rs:218`<br>• `AIConfigValidator::validate()` (line 43) → `src/config/validator.rs:43` | 控制 AI 回應的隨機性 | `subx-cli match` | ✅ 使用中 |
| `retry_attempts` | u32 | 3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 26) → `src/commands/match_command.rs:26`<br>• `OpenAIClient::new()` 接收參數<br>• `OpenAIClient::make_request_with_retry()` (line 297) → `src/services/ai/openai.rs:297`<br>• `AIConfigValidator::validate()` (line 49) → `src/config/validator.rs:49` | API 請求失敗時的重試次數 | `subx-cli match` | ✅ 使用中 |
| `retry_delay_ms` | u64 | 1000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 27) → `src/commands/match_command.rs:27`<br>• `OpenAIClient::new()` 接收參數<br>• `OpenAIClient::make_request_with_retry()` (line 299) → `src/services/ai/openai.rs:299` | API 重試之間的延遲時間 | `subx-cli match` | ✅ 使用中 |

### 格式配置 (`[formats]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_output` | String | "srt" | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 19, 26) → `src/commands/convert_command.rs:19,26`<br>• `FormatsConfigValidator::validate()` (line 141) → `src/config/validator.rs:141`<br>• `Config::get_value()` (line 385) → `src/config.rs:385` | CLI 轉換命令的預設輸出格式 | `subx-cli convert` | ✅ 使用中 |
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
| `dialogue_merge_gap_ms` | u64 | 500 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 26) → `src/commands/sync_command.rs:26`<br>• `DialogueDetector::new()` (line 19) → `src/core/sync/dialogue/detector.rs:19`<br>• `DialogueDetector::optimize_segments()` (line 56) → `src/core/sync/dialogue/detector.rs:56`<br>• 用於對話片段合併的間隔時間計算 | 對話片段合併間隔，控制相鄰對話合併邏輯 | `subx-cli sync`（透過 DialogueDetector） | ✅ 使用中 |
| `resample_quality` | String | "high" | **呼叫樹:**<br>• `ResampleConfig::from_config()` (line 64) → `src/services/audio/resampler/converter.rs:64`<br>• `ResampleQuality::from_string()` (line 28) → `src/services/audio/resampler/converter.rs:28`<br>• `AudioResampler::create_interpolator()` 使用品質設定 | 音訊重採樣品質設定 | `subx-cli sync`（透過 DialogueDetector） | ✅ 使用中 |
| `auto_detect_sample_rate` | bool | true | **呼叫樹:**<br>• `SyncCommand::execute()` → 載入配置<br>• `DialogueDetector::new()` (line 15) → `src/core/sync/dialogue/detector.rs:15`<br>• `DialogueDetector::load_audio()` (line 44) → `src/core/sync/dialogue/detector.rs:44`<br>• `AusSampleRateDetector::auto_detect_if_enabled()` (line 94) → `src/services/audio/resampler/detector.rs:94`<br>• 決定是否自動檢測音訊檔案的採樣率 | 自動檢測音訊採樣率，失敗時回退到配置值 | `subx-cli sync` | ✅ 使用中 |
| `enable_smart_resampling` | bool | true | • `PartialConfig` 定義與合併<br>• `SyncConfig::enable_smart_resampling()` 方法 | 啟用智慧重採樣 | 無（功能未實作） | ⚠️ 已定義但功能未實作 |

### 一般配置 (`[general]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `backup_enabled` | bool | false | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 41) → `src/commands/match_command.rs:41`<br>• `MatchEngine::apply_operations()` (line 324) → `src/core/matcher/engine.rs:324`<br>• `EnvSource::load()` (line 79-80) → `src/config/source.rs:79-80` | 檔案匹配時是否自動備份 | `subx-cli match` | ✅ 使用中 |
| `max_concurrent_jobs` | usize | 4 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 68) → `src/core/parallel/scheduler.rs:68`<br>• `MatchCommand::batch_match_directory()` (line 69) → `src/commands/match_command.rs:69`<br>• `GeneralConfigValidator::validate()` (line 174) → `src/config/validator.rs:174` | 並行任務調度器的最大並發數 | `subx-cli match`（批次處理模式） | ✅ 使用中 |
| `task_timeout_seconds` | u64 | 3600 | **呼叫樹:**<br>• `MatchCommand::batch_match_directory()` (line 59) → `src/commands/match_command.rs:59`<br>• `TaskScheduler::new()` (line 96) → `src/core/parallel/scheduler.rs:96`<br>• 設定並行任務的執行逾時時間 | 任務執行逾時設定，用於並行處理調度器 | `subx-cli match`（批次處理模式） | ✅ 使用中 |
| `enable_progress_bar` | bool | true | **呼叫樹:**<br>• `MatchCommand::batch_match_directory()` (line 84) → `src/commands/match_command.rs:84`<br>• `create_progress_bar()` (line 27) → `src/cli/ui.rs:27`<br>• 控制是否顯示進度條 UI | 是否顯示進度條，控制批次處理的 UI 顯示 | `subx-cli match`（批次處理模式） | ✅ 使用中 |
| `worker_idle_timeout_seconds` | u64 | 300 | **呼叫樹:**<br>• `MatchCommand::batch_match_directory()` (line 59) → `src/commands/match_command.rs:59`<br>• `TaskScheduler::new()` (line 98) → `src/core/parallel/scheduler.rs:98`<br>• 設定工作執行緒的閒置逾時時間 | 工作執行緒閒置逾時，用於並行處理調度器 | `subx-cli match`（批次處理模式） | ✅ 使用中 |

### 並行處理配置 (`[parallel]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `task_queue_size` | usize | 100 | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 72) → `src/core/parallel/config.rs:72`<br>• `TaskScheduler::new()` (line 68) → `src/core/parallel/scheduler.rs:68`<br>• `TaskScheduler::submit_task()` (line 276) → `src/core/parallel/scheduler.rs:276`<br>• 用於控制任務佇列最大長度 | 任務佇列大小限制，控制記憶體使用 | `subx-cli match`（批次處理模式） | ✅ 使用中 |
| `enable_task_priorities` | bool | true | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 73) → `src/core/parallel/config.rs:73`<br>• `TaskScheduler::new()` (line 68) → `src/core/parallel/scheduler.rs:68`<br>• `TaskScheduler::submit_task()` (line 292) → `src/core/parallel/scheduler.rs:292`<br>• 控制任務佇列中的優先級排序邏輯 | 啟用任務優先級排程，影響任務執行順序 | `subx-cli match`（批次處理模式） | ✅ 使用中 |
| `auto_balance_workers` | bool | true | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 74) → `src/core/parallel/config.rs:74`<br>• `TaskScheduler::new()` (line 87) → `src/core/parallel/scheduler.rs:87`<br>• 決定是否啟用 LoadBalancer | 自動平衡工作負載，啟用負載平衡器 | `subx-cli match`（批次處理模式） | ✅ 使用中 |
| `queue_overflow_strategy` | OverflowStrategy | "block" | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 75) → `src/core/parallel/config.rs:75`<br>• `TaskScheduler::new()` (line 68) → `src/core/parallel/scheduler.rs:68`<br>• `TaskScheduler::submit_task()` (line 277) → `src/core/parallel/scheduler.rs:277`<br>• 控制佇列滿時的處理策略（block/drop_oldest/reject） | 任務佇列溢出策略，處理佇列滿時的行為 | `subx-cli match`（批次處理模式） | ✅ 使用中 |

## 狀態說明

- ✅ **使用中**: 配置項目已完全整合並在程式碼中實際使用
- ⚠️ **已定義但未使用**: 配置項目已定義並可設定，但核心功能未實作或未讀取此設定
- 🗑️ **待移除**: 配置項目為死代碼，完全未被使用，應移除以避免混淆
- ❌ **未使用**: 配置項目完全未在程式碼中使用（已移除此類別）

## 總結

### 完全整合的配置 (25 項) - 含詳細呼叫樹
- **AI 配置**: 8/8 項已使用，包含完整的從環境變數載入到實際 API 呼叫的路徑，包括 provider 選擇和自訂 base_url
- **格式配置**: 4/4 項已使用，包含編碼檢測、格式轉換流程
- **同步配置**: 8/10 項已使用，主要在 SyncCommand 和相關引擎中使用，包含音訊處理、對話檢測和自動採樣率檢測
- **一般配置**: 5/5 項已使用，包含備份、並行任務調度、進度條顯示和逾時設定
- **並行處理配置**: 4/6 項已完全使用（task_queue_size, enable_task_priorities, auto_balance_workers, queue_overflow_strategy）

### 需要進一步整合的配置 (1 項)
主要集中在：
1. **音訊處理功能**: `enable_smart_resampling` （智慧重採樣功能未實作）

### 待移除的死代碼配置 (2 項)
主要集中在：

這些配置項目都在配置系統中正確定義並可設定，但對應的功能實作尚未完成或未完全使用配置。
