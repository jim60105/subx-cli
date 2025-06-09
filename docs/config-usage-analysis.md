# SubX 配置檔案使用情況分析

此文件分析 SubX 配置檔案中各項設定在程式碼中的實際使用情況，確保沒有多餘或未整合的配置。

## 配置設定使用分析表

### AI 配置 (`[ai]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `provider` | String | "openai" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 19) → `src/commands/match_command.rs:19`<br>• `AIClientFactory::create_client()` (line 11-13) → `src/services/ai/factory.rs:11-13`<br>• `OpenAIClient::from_config()` 根據 provider 建立實例<br>• `Config::validate()` (line 409) → `src/config.rs:409`<br>• `EnvSource::load()` (line 97-98) → `src/config/source.rs:97-98` | 用於選擇 AI 提供商類型，目前支援 "openai" | `subx-cli match` | ✅ 使用中 |
| `api_key` | Option<String> | None | **呼叫樹:**<br>• `EnvSource::load()` (line 91-92) → `src/config/source.rs:91-92`<br>• `MatchCommand::execute()` (line 19) → `src/commands/match_command.rs:19`<br>• `OpenAIClient::from_config()` (line 215-218) → `src/services/ai/openai.rs:215-218`<br>• `AIConfigValidator::validate()` (line 29-33) → `src/config/validator.rs:29-33` | 用於 OpenAI API 認證，支援從環境變數 OPENAI_API_KEY 載入 | `subx-cli match` | ✅ 使用中 |
| `model` | String | "gpt-4o-mini" | **呼叫樹:**<br>• `EnvSource::load()` (line 94-95) → `src/config/source.rs:94-95`<br>• `MatchCommand::execute()` (line 19) → `src/commands/match_command.rs:19`<br>• `OpenAIClient::from_config()` (line 225) → `src/services/ai/openai.rs:225`<br>• `OpenAIClient::chat_completion()` (line 256) → `src/services/ai/openai.rs:256`<br>• `AIConfigValidator::validate()` (line 44-49) → `src/config/validator.rs:44-49` | 指定使用的 OpenAI 模型，在 HTTP 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `base_url` | String | "https://api.openai.com/v1" | **呼叫樹:**<br>• `EnvSource::load()` (line 100-101) → `src/config/source.rs:100-101`<br>• `MatchCommand::execute()` (line 19) → `src/commands/match_command.rs:19`<br>• `AIClientFactory::create_client()` → `src/services/ai/factory.rs:11`<br>• `OpenAIClient::from_config()` (line 222, 229) → `src/services/ai/openai.rs:222,229`<br>• `OpenAIClient::validate_base_url()` (line 231) → `src/services/ai/openai.rs:231`<br>• `OpenAIClient::new_with_base_url()` (line 205) → `src/services/ai/openai.rs:205`<br>• `AIConfigValidator::validate()` (line 66-68) → `src/config/validator.rs:66-68` | 支援自訂 API 端點，完整從配置到實際 HTTP 請求的路徑 | `subx-cli match` | ✅ 使用中 |
| `max_sample_length` | usize | 2000 | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 28) → `src/commands/match_command.rs:28`<br>• `MatchEngine::create_content_preview()` (line 284-285) → `src/core/matcher/engine.rs:284-285` | 控制傳送給 AI 的內容長度上限 | `subx-cli match` | ✅ 使用中 |
| `temperature` | f32 | 0.3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 19) → `src/commands/match_command.rs:19`<br>• `OpenAIClient::from_config()` (line 226) → `src/services/ai/openai.rs:226`<br>• `OpenAIClient::chat_completion()` (line 258) → `src/services/ai/openai.rs:258`<br>• `AIConfigValidator::validate()` (line 53-55) → `src/config/validator.rs:53-55` | 控制 AI 回應的隨機性，在 HTTP 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `retry_attempts` | u32 | 3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 19) → `src/commands/match_command.rs:19`<br>• `OpenAIClient::from_config()` (line 227) → `src/services/ai/openai.rs:227`<br>• `OpenAIClient::make_request_with_retry()` (line 337) → `src/services/ai/openai.rs:337`<br>• `AIConfigValidator::validate()` (line 59-61) → `src/config/validator.rs:59-61` | API 請求失敗時的重試次數 | `subx-cli match` | ✅ 使用中 |
| `retry_delay_ms` | u64 | 1000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 19) → `src/commands/match_command.rs:19`<br>• `OpenAIClient::from_config()` (line 228) → `src/services/ai/openai.rs:228`<br>• `OpenAIClient::make_request_with_retry()` (line 339) → `src/services/ai/openai.rs:339` | API 重試之間的延遲時間 | `subx-cli match` | ✅ 使用中 |

### 格式配置 (`[formats]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_output` | String | "srt" | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 19, 26) → `src/commands/convert_command.rs:19,26`<br>• `FormatsConfigValidator::validate()` (line 151) → `src/config/validator.rs:151`<br>• `Config::get_value()` (line 433) → `src/config.rs:433` | CLI 轉換命令的預設輸出格式 | `subx-cli convert` | ✅ 使用中 |
| `preserve_styling` | bool | true | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 11) → `src/commands/convert_command.rs:11`<br>• `SrtToAssTransformer::transform()` (line 42) → `src/core/formats/transformers.rs:42`<br>• 類似在其他轉換器中使用 (line 56, 86) | 控制格式轉換時是否保留樣式 | `subx-cli convert` | ✅ 使用中 |
| `default_encoding` | String | "utf-8" | **呼叫樹:**<br>• `EncodingDetector::detect_file()` (line 302) → `src/core/formats/encoding/detector.rs:302`<br>• `FormatsConfigValidator::validate()` (line 157) → `src/config/validator.rs:157`<br>• 當檢測信心度低於閾值時作為回退編碼 | 預設檔案編碼設定 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |
| `encoding_detection_confidence` | f32 | 0.7 | **呼叫樹:**<br>• `EncodingDetector::new()` (line 19) → `src/core/formats/encoding/detector.rs:19`<br>• `EncodingDetector::detect_file()` (line 294) → `src/core/formats/encoding/detector.rs:294`<br>• 被 `DetectEncodingCommand` (line 8) → `src/commands/detect_encoding_command.rs:8`<br>• 被 `FormatConverter` (line 152) → `src/core/formats/converter.rs:152`<br>• 被 `FileManager` (line 61, 81) → `src/core/formats/manager.rs:61,81`<br>• `FormatsConfigValidator::validate()` (line 163-167) → `src/config/validator.rs:163-167` | 編碼自動檢測的信心度閾值 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |

### 同步配置 (`[sync]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `max_offset_seconds` | f32 | 30.0 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 16) → `src/commands/sync_command.rs:16`<br>• `SyncEngine::find_best_offset()` (line 95) → `src/core/sync/engine.rs:95`<br>• `SyncConfigValidator::validate()` (line 126) → `src/config/validator.rs:126` | 音訊字幕同步的最大偏移範圍 | `subx-cli sync` | ✅ 使用中 |
| `correlation_threshold` | f32 | 0.7 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 17-19) → `src/commands/sync_command.rs:17-19`<br>• `SyncEngine::find_best_offset()` (line 112) → `src/core/sync/engine.rs:112`<br>• `SyncConfigValidator::validate()` (line 132) → `src/config/validator.rs:132` | 音訊相關性分析的閾值 | `subx-cli sync` | ✅ 使用中 |
| `dialogue_detection_threshold` | f32 | 0.01 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 20) → `src/commands/sync_command.rs:20`<br>• `DialogueDetector::new()` (line 18) → `src/core/sync/dialogue/detector.rs:18`<br>• `EnergyAnalyzer::new()` (line 15) → `src/core/sync/dialogue/analyzer.rs:15`<br>• `EnergyAnalyzer::analyze()` 使用閾值進行能量檢測 | 對話片段檢測的音訊能量敏感度閾值 | `subx-cli sync` | ✅ 使用中 |
| `min_dialogue_duration_ms` | u64 | 500 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 21) → `src/commands/sync_command.rs:21`<br>• `DialogueDetector::new()` (line 19) → `src/core/sync/dialogue/detector.rs:19`<br>• `EnergyAnalyzer::new()` (line 15) → `src/core/sync/dialogue/analyzer.rs:15`<br>• `EnergyAnalyzer::filter_short_segments()` (line 72) → `src/core/sync/dialogue/analyzer.rs:72` | 最小對話片段持續時間，用於過濾短於此時間的檢測結果 | `subx-cli sync` | ✅ 使用中 |
| `enable_dialogue_detection` | bool | true | **呼叫樹:**<br>• `SyncCommand::execute()` (line 25) → `src/commands/sync_command.rs:25`<br>• `DialogueDetector::detect_dialogue()` (line 30) → `src/core/sync/dialogue/detector.rs:30`<br>• 控制是否執行對話檢測和語音片段分析 | 是否啟用對話檢測功能 | `subx-cli sync` | ✅ 使用中 |
| `audio_sample_rate` | u32 | 16000 | **呼叫樹:**<br>• `DialogueDetector::load_audio()` (line 45, 48) → `src/core/sync/dialogue/detector.rs:45,48`<br>• `AusAdapter::new()` (line 45) → 作為回退採樣率<br>• `AudioAnalyzer::new()` (line 51) → 用於音訊分析初始化 | 音訊處理的目標採樣率，用於對話檢測 | `subx-cli sync`（透過 DialogueDetector） | ✅ 使用中 |
| `dialogue_merge_gap_ms` | u64 | 500 | **呼叫樹:**<br>• `DialogueDetector::optimize_segments()` (line 57) → `src/core/sync/dialogue/detector.rs:57`<br>• 用於計算相鄰對話片段是否應該合併的時間間隔閾值 | 對話片段合併間隔，控制相鄰對話合併邏輯 | `subx-cli sync`（透過 DialogueDetector） | ✅ 使用中 |
| `auto_detect_sample_rate` | bool | true | **呼叫樹:**<br>• `DialogueDetector::load_audio()` (line 44) → `src/core/sync/dialogue/detector.rs:44`<br>• `AusAdapter::new()` + `AusAdapter::read_audio_file()` (line 45-46) → 檢測音訊檔案採樣率<br>• 決定是否自動檢測音訊檔案採樣率，或使用配置值 | 自動檢測音訊採樣率，失敗時回退到配置值 | `subx-cli sync` | ✅ 使用中 |

### 一般配置 (`[general]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `backup_enabled` | bool | false | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 31) → `src/commands/match_command.rs:31`<br>• `MatchEngine::apply_operations()` (line 324) → `src/core/matcher/engine.rs:324`<br>• `EnvSource::load()` (line 103-104) → `src/config/source.rs:103-104` | 檔案匹配時是否自動備份，支援環境變數 SUBX_BACKUP_ENABLED | `subx-cli match` | ✅ 使用中 |
| `max_concurrent_jobs` | usize | 4 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 76, 82) → `src/core/parallel/scheduler.rs:76,82`<br>• `ParallelConfig::from_app_config()` (line 82) → `src/core/parallel/config.rs:82`<br>• `execute_parallel_match()` (line 59) → `src/commands/match_command.rs:59`<br>• `GeneralConfigValidator::validate()` (line 184) → `src/config/validator.rs:184` | 並行任務調度器的最大並發數，控制同時執行的工作執行緒數量 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `task_timeout_seconds` | u64 | 3600 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 96, 129) → `src/core/parallel/scheduler.rs:96,129`<br>• `execute_parallel_match()` (line 59) → `src/commands/match_command.rs:59`<br>• 設定並行任務的執行逾時時間 | 任務執行逾時設定，用於並行處理調度器的任務執行時間上限 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `enable_progress_bar` | bool | true | **呼叫樹:**<br>• `execute_parallel_match()` (line 84) → `src/commands/match_command.rs:84`<br>• `create_progress_bar()` (line 23, 27) → `src/cli/ui.rs:23,27`<br>• 控制是否顯示進度條 UI | 是否顯示進度條，控制並行處理的 UI 顯示 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `worker_idle_timeout_seconds` | u64 | 300 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 97-98, 130-131) → `src/core/parallel/scheduler.rs:97-98,130-131`<br>• `execute_parallel_match()` (line 59) → `src/commands/match_command.rs:59`<br>• 設定工作執行緒的閒置逾時時間 | 工作執行緒閒置逾時，用於並行處理調度器的閒置工作執行緒回收 | `subx-cli match`（並行處理模式） | ✅ 使用中 |

### 並行處理配置 (`[parallel]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `task_queue_size` | usize | 100 | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 83) → `src/core/parallel/config.rs:83`<br>• `ParallelConfig::validate()` (line 97-98) → `src/core/parallel/config.rs:97-98`<br>• `TaskScheduler::submit_task()` (line 275, 279) → `src/core/parallel/scheduler.rs:275,279`<br>• `TaskScheduler::submit_prioritized_task()` (line 359, 363) → `src/core/parallel/scheduler.rs:359,363`<br>• 用於控制任務佇列最大長度 | 任務佇列大小限制，控制記憶體使用和佇列溢出策略 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `enable_task_priorities` | bool | true | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 84) → `src/core/parallel/config.rs:84`<br>• `TaskScheduler::start_scheduler_loop()` (line 172) → `src/core/parallel/scheduler.rs:172`<br>• `TaskScheduler::submit_task()` (line 295) → `src/core/parallel/scheduler.rs:295`<br>• `TaskScheduler::submit_prioritized_task()` (line 380) → `src/core/parallel/scheduler.rs:380`<br>• 控制任務佇列中的優先級排序邏輯 | 啟用任務優先級排程，影響任務執行順序和佇列插入位置 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `auto_balance_workers` | bool | true | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 85) → `src/core/parallel/config.rs:85`<br>• `TaskScheduler::new()` (line 91) → `src/core/parallel/scheduler.rs:91`<br>• `TaskScheduler::new_with_defaults()` (line 124) → `src/core/parallel/scheduler.rs:124`<br>• 決定是否啟用 LoadBalancer | 自動平衡工作負載，啟用負載平衡器來分配任務 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `queue_overflow_strategy` | OverflowStrategy | "block" | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 86) → `src/core/parallel/config.rs:86`<br>• `TaskScheduler::submit_task()` (line 276) → `src/core/parallel/scheduler.rs:276`<br>• `TaskScheduler::submit_prioritized_task()` (line 360) → `src/core/parallel/scheduler.rs:360`<br>• 控制佇列滿時的處理策略（block/drop_oldest/reject） | 任務佇列溢出策略，處理佇列滿時的行為（阻塞、丟棄最舊任務或拒絕） | `subx-cli match`（並行處理模式） | ✅ 使用中 |

## 狀態說明

- ✅ **使用中**: 配置項目已完全整合並在程式碼中實際使用
- ⚠️ **已定義但未使用**: 配置項目已定義並可設定，但核心功能未實作或未讀取此設定
- 🗑️ **待移除**: 配置項目為死代碼，完全未被使用，應移除以避免混淆
- ❌ **未使用**: 配置項目完全未在程式碼中使用（已移除此類別）

## 總結

### 完全整合的配置 (29 項) - 含詳細呼叫樹
- **AI 配置**: 8/8 項已使用，包含完整的從環境變數載入到實際 API 呼叫的路徑，包括 provider 選擇和自訂 base_url
- **格式配置**: 4/4 項已使用，包含編碼檢測、格式轉換流程
- **同步配置**: 8/8 項已使用，主要在 SyncCommand 和相關引擎中使用，包含音訊處理、對話檢測和自動採樣率檢測
- **一般配置**: 5/5 項已使用，包含備份、並行任務調度、進度條顯示和逾時設定
- **並行處理配置**: 4/4 項已完全使用（task_queue_size, enable_task_priorities, auto_balance_workers, queue_overflow_strategy）
