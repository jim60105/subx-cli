# SubX 配置檔案使用情況分析

此文件分析 SubX 配置檔案中各項設定在程式碼中的實際| `task_timeout_seconds` | u64 | 3600 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 110, 143) → `src/core/parallel/scheduler.rs:110,143`<br>• `execute_parallel_match()` 使用於並行處理調度器<br>• 設定並行任務的執行逾時時間 | 任務執行逾時設定，用於並行處理調度器的任務執行時間上限 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `enable_progress_bar` | bool | true | **呼叫樹:**<br>• `execute_parallel_match()` (line 471) → `src/commands/match_command.rs:471`<br>• `create_progress_bar()` 控制是否顯示進度條 UI | 是否顯示進度條，控制並行處理的 UI 顯示 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `worker_idle_timeout_seconds` | u64 | 300 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 112, 145) → `src/core/parallel/scheduler.rs:112,145`<br>• `execute_parallel_match()` 使用於並行處理調度器<br>• 設定工作執行緒的閒置逾時時間 | 工作執行緒閒置逾時，用於並行處理調度器的閒置工作執行緒回收 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `temp_dir` | Option<PathBuf> | None | 無實際使用 | 處理用的暫存目錄（未實作） | 無 | ⚠️ 已定義但未使用 |
| `log_level` | String | "info" | 無實際使用，僅出現在範例和文檔中 | 應用程式輸出的日誌層級（未實作） | 無 | ⚠️ 已定義但未使用 |
| `cache_dir` | Option<PathBuf> | None | 無實際使用 | 存儲處理數據的快取目錄（未實作） | 無 | ⚠️ 已定義但未使用 |況，確保沒有多餘或未整合的配置。

## 配置設定使用分析表

### AI 配置 (`[ai]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `provider` | String | "openai" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `AIClientFactory::create_client()` (line 139) → `src/services/ai/factory.rs:139`<br>• `OpenAIClient::from_config()` 根據 provider 建立實例<br>• `AIValidator::validate()` (line 24-31) → `src/config/validator.rs:24-31` | 用於選擇 AI 提供商類型，目前支援 "openai" | `subx-cli match` | ✅ 使用中 |
| `api_key` | Option<String> | None | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `OpenAIClient::from_config()` (line 215-218) → `src/services/ai/openai.rs:215-218`<br>• `AIValidator::validate()` (line 34-41) → `src/config/validator.rs:34-41` | 用於 OpenAI API 認證，支援從環境變數 OPENAI_API_KEY 載入 | `subx-cli match` | ✅ 使用中 |
| `model` | String | "gpt-4.1-mini" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `OpenAIClient::from_config()` (line 224) → `src/services/ai/openai.rs:224`<br>• `OpenAIClient::chat_completion()` 使用模型進行 HTTP 請求 | 指定使用的 OpenAI 模型，在 HTTP 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `base_url` | String | "https://api.openai.com/v1" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `AIClientFactory::create_client()` → `src/services/ai/factory.rs:139`<br>• `OpenAIClient::from_config()` (line 222, 229) → `src/services/ai/openai.rs:222,229`<br>• `OpenAIClient::validate_base_url()` (line 234) → `src/services/ai/openai.rs:234` | 支援自訂 API 端點，完整從配置到實際 HTTP 請求的路徑 | `subx-cli match` | ✅ 使用中 |
| `max_sample_length` | usize | 2000 | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 304) → `src/commands/match_command.rs:304`<br>• `MatchEngine::create_content_preview()` 使用此限制控制傳送給 AI 的內容長度 | 控制傳送給 AI 的內容長度上限 | `subx-cli match` | ✅ 使用中 |
| `temperature` | f32 | 0.3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `OpenAIClient::from_config()` (line 225) → `src/services/ai/openai.rs:225`<br>• `OpenAIClient::chat_completion()` 在 HTTP 請求中使用<br>• `AIValidator::validate()` (line 44-49) → `src/config/validator.rs:44-49` | 控制 AI 回應的隨機性，在 HTTP 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `retry_attempts` | u32 | 3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `OpenAIClient::from_config()` (line 226) → `src/services/ai/openai.rs:226`<br>• `OpenAIClient::make_request_with_retry()` 使用重試邏輯<br>• `AIValidator::validate()` (line 52-55) → `src/config/validator.rs:52-55` | API 請求失敗時的重試次數 | `subx-cli match` | ✅ 使用中 |
| `retry_delay_ms` | u64 | 1000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `OpenAIClient::from_config()` (line 227) → `src/services/ai/openai.rs:227`<br>• `OpenAIClient::make_request_with_retry()` 重試延遲使用 | API 重試之間的延遲時間 | `subx-cli match` | ✅ 使用中 |

### 格式配置 (`[formats]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_output` | String | "srt" | **呼叫樹:**<br>• `ConvertCommand::execute()` 使用預設輸出格式<br>• `FormatsValidator::validate()` (line 83) → `src/config/validator.rs:83` | CLI 轉換命令的預設輸出格式 | `subx-cli convert` | ✅ 使用中 |
| `preserve_styling` | bool | true | **呼叫樹:**<br>• `ConvertCommand::execute()` 使用於格式轉換<br>• 在格式轉換器中控制樣式保留 | 控制格式轉換時是否保留樣式 | `subx-cli convert` | ✅ 使用中 |
| `default_encoding` | String | "utf-8" | **呼叫樹:**<br>• `EncodingDetector::detect_file()` 作為回退編碼<br>• `FormatsValidator::validate()` (line 88) → `src/config/validator.rs:88` | 預設檔案編碼設定 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |
| `encoding_detection_confidence` | f32 | 0.7 | **呼叫樹:**<br>• `EncodingDetector::new()` 和 `detect_file()` 使用此閾值<br>• `DetectEncodingCommand`, `FormatConverter`, `FileManager` 中使用<br>• `FormatsValidator::validate()` (line 93-98) → `src/config/validator.rs:93-98` | 編碼自動檢測的信心度閾值 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |

### 同步配置 (`[sync]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `max_offset_seconds` | f32 | 30.0 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 278) → `src/commands/sync_command.rs:278`<br>• `SyncEngine::find_best_offset()` 使用最大偏移範圍<br>• `SyncValidator::validate()` (line 64-68) → `src/config/validator.rs:64-68` | 音訊字幕同步的最大偏移範圍 | `subx-cli sync` | ✅ 使用中 |
| `correlation_threshold` | f32 | 0.7 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 280-282) → `src/commands/sync_command.rs:280-282`<br>• `SyncEngine::find_best_offset()` 使用相關性閾值<br>• `SyncValidator::validate()` (line 71-75) → `src/config/validator.rs:71-75` | 音訊相關性分析的閾值 | `subx-cli sync` | ✅ 使用中 |
| `dialogue_detection_threshold` | f32 | 0.01 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 283) → `src/commands/sync_command.rs:283`<br>• `DialogueDetector::new()` 和 `EnergyAnalyzer::new()` 使用<br>• `EnergyAnalyzer::analyze()` 使用閾值進行能量檢測 | 對話片段檢測的音訊能量敏感度閾值 | `subx-cli sync` | ✅ 使用中 |
| `min_dialogue_duration_ms` | u64 | 500 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 284) → `src/commands/sync_command.rs:284`<br>• `DialogueDetector::new()` 和 `EnergyAnalyzer::new()` 使用<br>• `EnergyAnalyzer::filter_short_segments()` 過濾短片段 | 最小對話片段持續時間，用於過濾短於此時間的檢測結果 | `subx-cli sync` | ✅ 使用中 |
| `enable_dialogue_detection` | bool | true | **呼叫樹:**<br>• `SyncCommand::execute()` 控制是否執行對話檢測<br>• `DialogueDetector::detect_dialogue()` 條件執行對話檢測和語音片段分析 | 是否啟用對話檢測功能 | `subx-cli sync` | ✅ 使用中 |
| `audio_sample_rate` | u32 | 16000 | **呼叫樹:**<br>• `DialogueDetector::load_audio()` 使用作為目標採樣率<br>• `AusAdapter::new()` 作為回退採樣率<br>• `AudioAnalyzer::new()` 用於音訊分析初始化 | 音訊處理的目標採樣率，用於對話檢測 | `subx-cli sync`（透過 DialogueDetector） | ✅ 使用中 |
| `dialogue_merge_gap_ms` | u64 | 500 | **呼叫樹:**<br>• `DialogueDetector::optimize_segments()` 使用於計算相鄰對話片段合併<br>• 用於計算相鄰對話片段是否應該合併的時間間隔閾值 | 對話片段合併間隔，控制相鄰對話合併邏輯 | `subx-cli sync`（透過 DialogueDetector） | ✅ 使用中 |
| `auto_detect_sample_rate` | bool | true | **呼叫樹:**<br>• `DialogueDetector::load_audio()` 決定是否自動檢測音訊採樣率<br>• `AusAdapter::new()` + `AusAdapter::read_audio_file()` 檢測音訊檔案採樣率<br>• 失敗時回退到配置值 | 自動檢測音訊採樣率，失敗時回退到配置值 | `subx-cli sync` | ✅ 使用中 |

### 一般配置 (`[general]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `backup_enabled` | bool | false | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 308) → `src/commands/match_command.rs:308`<br>• `MatchEngine::apply_operations()` 控制是否自動備份 | 檔案匹配時是否自動備份，支援環境變數 SUBX_BACKUP_ENABLED | `subx-cli match` | ✅ 使用中 |
| `max_concurrent_jobs` | usize | 4 | **呼叫樹:**<br>• `TaskScheduler::new()` 並行任務調度器使用<br>• `ParallelConfig::from_app_config()` (line 82) → `src/core/parallel/config.rs:82`<br>• `execute_parallel_match()` 並行處理模式使用 | 並行任務調度器的最大並發數，控制同時執行的工作執行緒數量 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `task_timeout_seconds` | u64 | 3600 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 96, 129) → `src/core/parallel/scheduler.rs:96,129`<br>• `execute_parallel_match()` (line 59) → `src/commands/match_command.rs:59`<br>• 設定並行任務的執行逾時時間 | 任務執行逾時設定，用於並行處理調度器的任務執行時間上限 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `enable_progress_bar` | bool | true | **呼叫樹:**<br>• `execute_parallel_match()` (line 84) → `src/commands/match_command.rs:84`<br>• `create_progress_bar()` (line 23, 27) → `src/cli/ui.rs:23,27`<br>• 控制是否顯示進度條 UI | 是否顯示進度條，控制並行處理的 UI 顯示 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `worker_idle_timeout_seconds` | u64 | 300 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 97-98, 130-131) → `src/core/parallel/scheduler.rs:97-98,130-131`<br>• `execute_parallel_match()` (line 59) → `src/commands/match_command.rs:59`<br>• 設定工作執行緒的閒置逾時時間 | 工作執行緒閒置逾時，用於並行處理調度器的閒置工作執行緒回收 | `subx-cli match`（並行處理模式） | ✅ 使用中 |

### 並行處理配置 (`[parallel]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `max_workers` | usize | num_cpus::get() | **呼叫樹:**<br>• `WorkerPool::new()` → `src/core/parallel/worker.rs:42`<br>• `ParallelValidator::validate()` (line 115) → `src/config/validator.rs:115`<br>• 控制工作執行緒池的最大執行緒數量 | 並行工作執行緒池的最大執行緒數量 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `chunk_size` | usize | 1000 | 無實際使用 | 平行處理的區塊大小（未實作） | 無 | ⚠️ 已定義但未使用 |
| `enable_work_stealing` | bool | true | 無實際使用 | 是否啟用工作竊取（未實作） | 無 | ⚠️ 已定義但未使用 |
| `task_queue_size` | usize | 100 | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 83) → `src/core/parallel/config.rs:83`<br>• `ParallelConfig::validate()` (line 97-98) → `src/core/parallel/config.rs:97-98`<br>• `TaskScheduler::submit_task()` 和 `submit_prioritized_task()` 使用<br>• 用於控制任務佇列最大長度 | 任務佇列大小限制，控制記憶體使用和佇列溢出策略 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `enable_task_priorities` | bool | true | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 84) → `src/core/parallel/config.rs:84`<br>• `TaskScheduler::start_scheduler_loop()` 控制優先級排序邏輯<br>• `TaskScheduler::submit_task()` 和 `submit_prioritized_task()` 使用 | 啟用任務優先級排程，影響任務執行順序和佇列插入位置 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `auto_balance_workers` | bool | true | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 85) → `src/core/parallel/config.rs:85`<br>• `TaskScheduler::new()` 決定是否啟用 LoadBalancer | 自動平衡工作負載，啟用負載平衡器來分配任務 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `queue_overflow_strategy` | OverflowStrategy | "block" | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 86) → `src/core/parallel/config.rs:86`<br>• `TaskScheduler::submit_task()` 和 `submit_prioritized_task()` 使用<br>• 控制佇列滿時的處理策略（block/drop_oldest/reject） | 任務佇列溢出策略，處理佇列滿時的行為（阻塞、丟棄最舊任務或拒絕） | `subx-cli match`（並行處理模式） | ✅ 使用中 |

## 狀態說明

- ✅ **使用中**: 配置項目已完全整合並在程式碼中實際使用
- ⚠️ **已定義但未使用**: 配置項目已定義並可設定，但核心功能未實作或未讀取此設定
- 🗑️ **待移除**: 配置項目為死代碼，完全未被使用，應移除以避免混淆
- ❌ **未使用**: 配置項目完全未在程式碼中使用（已移除此類別）

## 總結

### 完全整合的配置 (30 項) - 含詳細呼叫樹
- **AI 配置**: 8/8 項已使用，包含完整的從配置載入到實際 API 呼叫的路徑，包括 provider 選擇和自訂 base_url
- **格式配置**: 4/4 項已使用，包含編碼檢測、格式轉換流程
- **同步配置**: 8/8 項已使用，主要在 SyncCommand 和相關引擎中使用，包含音訊處理、對話檢測和自動採樣率檢測
- **一般配置**: 5/9 項已使用，包含備份、並行任務調度、進度條顯示和逾時設定
- **並行處理配置**: 5/7 項已使用（task_queue_size, enable_task_priorities, auto_balance_workers, queue_overflow_strategy, max_workers）

### 已定義但未使用的配置 (6 項)
- **一般配置**: temp_dir, log_level, cache_dir - 這些配置項目已定義但在實際程式碼中未使用
- **並行配置**: chunk_size, enable_work_stealing - 這些配置項目已定義但功能未實作
