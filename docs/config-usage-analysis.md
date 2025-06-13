# SubX 配置檔案使用情況分析

此文件分析 SubX 配置檔案中各項設定在程式碼中的實際使用情況，確保沒有多餘或未整合的配置。

## 配置設定使用分析表

### AI 配置 (`[ai]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `provider` | String | "openai" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 174, 209) → `src/commands/match_command.rs:174,209`<br>• `AIClientFactory::create_client()` (line 140) → `src/services/ai/factory.rs:140`<br>• `OpenAIClient::from_config()` 根據 provider 建立實例<br>• `AIValidator::validate()` (line 23-31) → `src/config/validator.rs:23-31` | 用於選擇 AI 提供商類型，目前支援 "openai" | `subx-cli match` | ✅ 使用中 |
| `api_key` | Option<String> | None | **呼叫樹:**<br>• `MatchCommand::execute()` (line 174, 209) → `src/commands/match_command.rs:174,209`<br>• `OpenAIClient::from_config()` → `src/services/ai/openai.rs:221-229`<br>• `AIValidator::validate()` (line 34-41) → `src/config/validator.rs:34-41`<br>• 環境變數 OPENAI_API_KEY 載入邏輯 | 用於 OpenAI API 認證，支援從環境變數載入 | `subx-cli match` | ✅ 使用中 |
| `model` | String | "gpt-4.1-mini" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 174, 209) → `src/commands/match_command.rs:174,209`<br>• `OpenAIClient::from_config()` (line 225) → `src/services/ai/openai.rs:225`<br>• `OpenAIClient::chat_completion()` 使用模型進行 HTTP 請求 | 指定使用的 OpenAI 模型，在 HTTP 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `base_url` | String | "https://api.openai.com/v1" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 174, 209) → `src/commands/match_command.rs:174,209`<br>• `AIClientFactory::create_client()` → `src/services/ai/factory.rs:140`<br>• `OpenAIClient::from_config()` (line 221, 229) → `src/services/ai/openai.rs:221,229`<br>• `OpenAIClient::validate_base_url()` 驗證 URL 格式 | 支援自訂 API 端點，完整從配置到實際 HTTP 請求的路徑 | `subx-cli match` | ✅ 使用中 |
| `max_sample_length` | usize | 3000 | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 314) → `src/commands/match_command.rs:314`<br>• `MatchEngine::create_content_preview()` 使用此限制控制傳送給 AI 的內容長度 | 控制傳送給 AI 的內容長度上限 | `subx-cli match` | ✅ 使用中 |
| `temperature` | f32 | 0.3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 174, 209) → `src/commands/match_command.rs:174,209`<br>• `OpenAIClient::from_config()` (line 226) → `src/services/ai/openai.rs:226`<br>• `OpenAIClient::chat_completion()` 在 HTTP 請求中使用<br>• `AIValidator::validate()` (line 43-49) → `src/config/validator.rs:43-49` | 控制 AI 回應的隨機性，在 HTTP 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `retry_attempts` | u32 | 3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 174, 209) → `src/commands/match_command.rs:174,209`<br>• `OpenAIClient::from_config()` (line 227) → `src/services/ai/openai.rs:227`<br>• `RetryConfig::make_request_with_retry()` 使用重試邏輯<br>• `AIValidator::validate()` (line 50-55) → `src/config/validator.rs:50-55` | API 請求失敗時的重試次數 | `subx-cli match` | ✅ 使用中 |
| `retry_delay_ms` | u64 | 1000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 174, 209) → `src/commands/match_command.rs:174,209`<br>• `OpenAIClient::from_config()` (line 228) → `src/services/ai/openai.rs:228`<br>• `RetryConfig::make_request_with_retry()` 重試延遲使用 | API 重試之間的延遲時間 | `subx-cli match` | ✅ 使用中 |

### 格式配置 (`[formats]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_output` | String | "srt" | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 217, 265) → `src/commands/convert_command.rs:217,265`<br>• `FormatsValidator::validate()` → `src/config/validator.rs` | CLI 轉換命令的預設輸出格式 | `subx-cli convert` | ✅ 使用中 |
| `preserve_styling` | bool | false | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 209, 257) → `src/commands/convert_command.rs:209,257`<br>• `FormatConverter::transform()` (line 68, 82, 112) → `src/core/formats/transformers.rs:68,82,112` | 控制格式轉換時是否保留樣式 | `subx-cli convert` | ✅ 使用中 |
| `default_encoding` | String | "utf-8" | **呼叫樹:**<br>• `EncodingDetector::new()` (line 17, 35) → `src/core/formats/encoding/detector.rs:17,35`<br>• `FormatsValidator::validate()` → `src/config/validator.rs` | 預設檔案編碼設定 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |
| `encoding_detection_confidence` | f32 | 0.8 | **呼叫樹:**<br>• `EncodingDetector::new()` (line 17, 35) → `src/core/formats/encoding/detector.rs:17,35`<br>• `EncodingDetector::detect_file()` 使用此閾值進行編碼檢測 | 編碼自動檢測的信心度閾值 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |

### 同步配置 (`[sync]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `max_offset_seconds` | f32 | 10.0 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 278, 313) → `src/commands/sync_command.rs:278,313`<br>• `SyncEngine::find_best_offset()` (line 451) → `src/core/sync/engine.rs:451`<br>• `DialogueDetector::set_max_offset()` (line 73) → `src/core/sync/dialogue/detector.rs:73` | 音訊字幕同步的最大偏移範圍 | `subx-cli sync` | ✅ 使用中 |
| `correlation_threshold` | f32 | 0.8 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 281, 316) → `src/commands/sync_command.rs:281,316`<br>• `SyncEngine::find_best_offset()` (line 468) → `src/core/sync/engine.rs:468`<br>• `DialogueDetector::set_correlation_threshold()` (line 55) → `src/core/sync/dialogue/detector.rs:55` | 音訊相關性分析的閾值 | `subx-cli sync` | ✅ 使用中 |
| `dialogue_detection_threshold` | f32 | 0.6 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 282, 317) → `src/commands/sync_command.rs:282,317`<br>• `DialogueDetector::new()` (line 37, 58, 199) → `src/core/sync/dialogue/detector.rs:37,58,199`<br>• `EnergyAnalyzer::analyze()` 使用閾值進行能量檢測 | 對話片段檢測的音訊能量敏感度閾值 | `subx-cli sync` | ✅ 使用中 |
| `min_dialogue_duration_ms` | u32 | 500 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 283, 318) → `src/commands/sync_command.rs:283,318`<br>• `DialogueDetector::new()` (line 38, 59, 200) → `src/core/sync/dialogue/detector.rs:38,59,200`<br>• `EnergyAnalyzer::filter_short_segments()` 過濾短片段 | 最小對話片段持續時間，用於過濾短於此時間的檢測結果 | `subx-cli sync` | ✅ 使用中 |
| `dialogue_merge_gap_ms` | u32 | 200 | **呼叫樹:**<br>• `DialogueDetector::optimize_segments()` (line 114) → `src/core/sync/dialogue/detector.rs:114`<br>• 用於計算相鄰對話片段是否應該合併的時間間隔閾值 | 對話片段合併間隔，控制相鄰對話合併邏輯 | `subx-cli sync`（透過 DialogueDetector） | ✅ 使用中 |
| `enable_dialogue_detection` | bool | true | **呼叫樹:**<br>• `SyncCommand::execute()` (line 336) → `src/commands/sync_command.rs:336`<br>• `DialogueDetector::detect_dialogue()` (line 87) → `src/core/sync/dialogue/detector.rs:87` | 是否啟用對話檢測功能 | `subx-cli sync` | ✅ 使用中 |
| `audio_sample_rate` | u32 | 44100 | **呼叫樹:**<br>• `DialogueDetector::load_audio()` (line 102, 105) → `src/core/sync/dialogue/detector.rs:102,105`<br>• `AusAdapter::new()` 作為回退採樣率<br>• `AudioAnalyzer::new()` 用於音訊分析初始化 | 音訊處理的目標採樣率，用於對話檢測 | `subx-cli sync`（透過 DialogueDetector） | ✅ 使用中 |
| `auto_detect_sample_rate` | bool | true | **呼叫樹:**<br>• `DialogueDetector::load_audio()` (line 101-105) → `src/core/sync/dialogue/detector.rs:101-105`<br>• `AusAdapter::read_audio_file()` 檢測音訊檔案採樣率<br>• 失敗時回退到配置值 | 自動檢測音訊採樣率，失敗時回退到配置值 | `subx-cli sync` | ✅ 使用中 |

### 一般配置 (`[general]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `backup_enabled` | bool | false | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 317) → `src/commands/match_command.rs:317`<br>• `MatchEngine::apply_operations()` 控制是否自動備份<br>• `ServiceFactory::create_match_engine()` (line 83) → `src/core/factory.rs:83` | 檔案匹配時是否自動備份，支援環境變數 SUBX_BACKUP_ENABLED | `subx-cli match` | ✅ 使用中 |
| `max_concurrent_jobs` | usize | 4 | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 82) → `src/core/parallel/config.rs:82`<br>• `TaskScheduler::new()` (line 93, 127) → `src/core/parallel/scheduler.rs:93,127`<br>• 並行處理模式使用 | 並行任務調度器的最大並發數，控制同時執行的工作執行緒數量 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `task_timeout_seconds` | u64 | 300 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 98, 131) → `src/core/parallel/scheduler.rs:98,131`<br>• 設定並行任務的執行逾時時間 | 任務執行逾時設定，用於並行處理調度器的任務執行時間上限 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `enable_progress_bar` | bool | true | **呼叫樹:**<br>• `execute_parallel_match()` (line 482) → `src/commands/match_command.rs:482`<br>• 控制是否顯示進度條 UI | 是否顯示進度條，控制並行處理的 UI 顯示 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `worker_idle_timeout_seconds` | u64 | 60 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 98, 131) → `src/core/parallel/scheduler.rs:98,131`<br>• 設定工作執行緒的閒置逾時時間 | 工作執行緒閒置逾時，用於並行處理調度器的閒置工作執行緒回收 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `temp_dir` | Option<PathBuf> | None | 無實際使用 | 處理用的暫存目錄（未實作） | 無 | ⚠️ 已定義但未使用 |
| `log_level` | String | "info" | 無實際使用，僅出現在範例和文檔中 | 應用程式輸出的日誌層級（未實作） | 無 | ⚠️ 已定義但未使用 |
| `cache_dir` | Option<PathBuf> | None | 無實際使用 | 存儲處理數據的快取目錄（未實作） | 無 | ⚠️ 已定義但未使用 |

### 並行處理配置 (`[parallel]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `max_workers` | usize | num_cpus::get() | **呼叫樹:**<br>• `ParallelValidator::validate()` → `src/config/validator.rs`<br>• 控制工作執行緒池的最大執行緒數量 | 並行工作執行緒池的最大執行緒數量 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `task_queue_size` | usize | 1000 | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 83) → `src/core/parallel/config.rs:83`<br>• `TaskScheduler::submit_task()` (line 296) → `src/core/parallel/scheduler.rs:296`<br>• 用於控制任務佇列最大長度 | 任務佇列大小限制，控制記憶體使用和佇列溢出策略 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `enable_task_priorities` | bool | false | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 84) → `src/core/parallel/config.rs:84`<br>• `TaskScheduler::start_scheduler_loop()` (line 192) → `src/core/parallel/scheduler.rs:192`<br>• 控制優先級排序邏輯 | 啟用任務優先級排程，影響任務執行順序和佇列插入位置 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `auto_balance_workers` | bool | true | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 85) → `src/core/parallel/config.rs:85`<br>• `TaskScheduler::new()` (line 105, 138) → `src/core/parallel/scheduler.rs:105,138`<br>• 決定是否啟用 LoadBalancer | 自動平衡工作負載，啟用負載平衡器來分配任務 | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `overflow_strategy` | OverflowStrategy | Block | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 86) → `src/core/parallel/config.rs:86`<br>• `TaskScheduler::submit_task()` 和 `submit_prioritized_task()` 使用<br>• 控制佇列滿時的處理策略（block/drop/expand） | 任務佇列溢出策略，處理佇列滿時的行為（阻塞、丟棄任務或擴展工作執行緒） | `subx-cli match`（並行處理模式） | ✅ 使用中 |
| `chunk_size` | usize | 1000 | 無實際使用 | 平行處理的區塊大小（未實作） | 無 | ⚠️ 已定義但未使用 |
| `enable_work_stealing` | bool | true | 無實際使用 | 是否啟用工作竊取（未實作） | 無 | ⚠️ 已定義但未使用 |

## 狀態說明

- ✅ **使用中**: 配置項目已完全整合並在程式碼中實際使用
- ⚠️ **已定義但未使用**: 配置項目已定義並可設定，但核心功能未實作或未讀取此設定

## 總結

### 完全整合的配置 (29 項) - 含詳細呼叫樹
- **AI 配置**: 8/8 項已使用，包含完整的從配置載入到實際 API 呼叫的路徑，包括 provider 選擇和自訂 base_url
- **格式配置**: 4/4 項已使用，包含編碼檢測、格式轉換流程
- **同步配置**: 8/8 項已使用，主要在 SyncCommand 和相關引擎中使用，包含音訊處理、對話檢測和自動採樣率檢測
- **一般配置**: 5/8 項已使用，包含備份、並行任務調度、進度條顯示和逾時設定
- **並行處理配置**: 5/7 項已使用（task_queue_size, enable_task_priorities, auto_balance_workers, overflow_strategy, max_workers）

### 已定義但未使用的配置 (6 項)
- **一般配置**: temp_dir, log_level, cache_dir - 這些配置項目已定義但在實際程式碼中未使用
- **並行配置**: chunk_size, enable_work_stealing - 這些配置項目已定義但功能未實作

**最後更新**: 2025-06-13 - 基於實際程式碼使用情況完成配置分析

## 配置一致性問題

### ⚠️ get_config_value 方法支援不完整

目前 `ProductionConfigService::get_config_value()` 方法只支援有限的配置鍵，但實際程式碼中使用了更多配置項目：

**get_config_value 支援的配置鍵 (16 項)**：
- AI: provider, model, api_key, base_url, temperature (缺少: max_sample_length, retry_attempts, retry_delay_ms)
- 格式: default_output, default_encoding, preserve_styling (缺少: encoding_detection_confidence)
- 同步: max_offset_seconds, correlation_threshold, audio_sample_rate (缺少: 5 項對話檢測相關配置)
- 一般: backup_enabled, max_concurrent_jobs, log_level (缺少: 5 項並行處理相關配置)
- 並行: max_workers, chunk_size (缺少: 5 項高級並行配置)

**建議修復**：
1. 擴展 `get_config_value` 方法以支援所有實際使用的配置項目
2. 或者移除未使用的配置項目以保持一致性
3. 確保 `config set` 命令能夠設定所有實際使用的配置項目
