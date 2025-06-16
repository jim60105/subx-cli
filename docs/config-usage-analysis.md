# SubX 配置檔案使用情況分析

此文件分析 SubX 配置檔案中各項設定在程式碼中的實際使用情況，確保沒有多餘或未整合的配置。

## 配置設定使用分析表

### AI 配置 (`[ai]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `provider` | String | "openai" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 197) → `src/commands/match_command.rs:197`<br>• `AIClientFactory::create_client()` (line 131) → `src/services/ai/factory.rs:131`<br>• 根據 provider 類型選擇對應的 AI 客戶端實現 | 用於選擇 AI 提供商類型，目前支援 "openai" | `subx-cli match` | ✅ 使用中 |
| `api_key` | Option<String> | None | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 197) → `src/commands/match_command.rs:197`<br>• `AIClientFactory::create_client()` → `OpenAIClient::from_config()` (line 215-217) → `src/services/ai/openai.rs:215-217`<br>• `OpenAIClient::new_with_base_url()` (line 224) → `src/services/ai/openai.rs:224`<br>• 在 HTTP 請求中作為 Authorization Bearer token (line 273) → `src/services/ai/openai.rs:273` | 用於 OpenAI API 認證，支援從環境變數載入 | `subx-cli match` | ✅ 使用中 |
| `model` | String | "gpt-4.1-mini" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 197) → `src/commands/match_command.rs:197`<br>• `AIClientFactory::create_client()` → `OpenAIClient::from_config()` (line 225) → `src/services/ai/openai.rs:225`<br>• `OpenAIClient::chat_completion()` HTTP 請求中使用 (line 268) → `src/services/ai/openai.rs:268` | 指定使用的 OpenAI 模型，在 HTTP 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `base_url` | String | "https://api.openai.com/v1" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 197) → `src/commands/match_command.rs:197`<br>• `AIClientFactory::create_client()` → `OpenAIClient::from_config()` (line 221, 229) → `src/services/ai/openai.rs:221,229`<br>• `OpenAIClient::validate_base_url()` 驗證 URL 格式 (line 232) → `src/services/ai/openai.rs:232`<br>• `OpenAIClient::chat_completion()` HTTP 請求端點 (line 276) → `src/services/ai/openai.rs:276` | 支援自訂 API 端點，完整從配置到實際 HTTP 請求的路徑 | `subx-cli match` | ✅ 使用中 |
| `max_sample_length` | usize | 3000 | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 314) → `src/commands/match_command.rs:314`<br>• 透過 `MatchConfig` 結構傳遞給 `MatchEngine`<br>• `MatchEngine::create_content_preview()` 用於限制內容預覽長度 (line 758-759) → `src/core/matcher/engine.rs:758-759` | 控制傳送給 AI 的內容長度上限和內容預覽長度 | `subx-cli match` | ✅ 使用中 |
| `temperature` | f32 | 0.3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 197) → `src/commands/match_command.rs:197`<br>• `AIClientFactory::create_client()` → `OpenAIClient::from_config()` (line 226) → `src/services/ai/openai.rs:226`<br>• `OpenAIClient::chat_completion()` HTTP 請求中使用 (line 270) → `src/services/ai/openai.rs:270` | 控制 AI 回應的隨機性，在 HTTP 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `max_tokens` | u32 | 10000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 197) → `src/commands/match_command.rs:197`<br>• `AIClientFactory::create_client()` → `OpenAIClient::from_config()` (line 229) → `src/services/ai/openai.rs:229`<br>• `OpenAIClient::chat_completion()` HTTP 請求中使用 (line 271) → `src/services/ai/openai.rs:271` | 控制 AI 回應的最大 token 數量限制 | `subx-cli match` | ✅ 使用中 |
| `retry_attempts` | u32 | 3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 197) → `src/commands/match_command.rs:197`<br>• `AIClientFactory::create_client()` → `OpenAIClient::from_config()` (line 227) → `src/services/ai/openai.rs:227`<br>• `OpenAIClient::make_request_with_retry()` 重試邏輯 (line 347) → `src/services/ai/openai.rs:347` | API 請求失敗時的重試次數 | `subx-cli match` | ✅ 使用中 |
| `retry_delay_ms` | u64 | 1000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 197) → `src/commands/match_command.rs:197`<br>• `AIClientFactory::create_client()` → `OpenAIClient::from_config()` (line 228) → `src/services/ai/openai.rs:228`<br>• `OpenAIClient::make_request_with_retry()` 延遲邏輯 (line 349) → `src/services/ai/openai.rs:349` | API 重試之間的延遲時間 | `subx-cli match` | ✅ 使用中 |

### 格式配置 (`[formats]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_output` | String | "srt" | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 204) → `src/commands/convert_command.rs:204`<br>• 用於決定預設的輸出字幕格式 (line 217) → `src/commands/convert_command.rs:217`<br>• 支援 srt/ass/vtt/sub 格式 | CLI 轉換命令的預設輸出格式 | `subx-cli convert` | ✅ 使用中 |
| `preserve_styling` | bool | false | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 204) → `src/commands/convert_command.rs:204`<br>• `ConversionConfig` 中使用以控制格式轉換時的樣式保留 (line 209) → `src/commands/convert_command.rs:209` | 控制格式轉換時是否保留樣式 | `subx-cli convert` | ✅ 使用中 |
| `default_encoding` | String | "utf-8" | **呼叫樹:**<br>• `EncodingDetector::new()` (line 16) → `src/core/formats/encoding/detector.rs:16`<br>• `EncodingDetector::with_config()` (line 33) → `src/core/formats/encoding/detector.rs:33`<br>• `EncodingDetector::select_best_encoding()` (line 306, 314, 321, 329) → `src/core/formats/encoding/detector.rs:306,314,321,329`<br>• 在編碼檢測失敗或信心度不足時用作回退編碼 | 編碼檢測失敗時的預設編碼回退 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |
| `encoding_detection_confidence` | f32 | 0.8 | **呼叫樹:**<br>• `EncodingDetector::new()` (line 16) → `src/core/formats/encoding/detector.rs:16`<br>• `EncodingDetector::with_config()` (line 33) → `src/core/formats/encoding/detector.rs:33`<br>• `EncodingDetector::select_best_encoding()` (line 319) → `src/core/formats/encoding/detector.rs:319` | 編碼自動檢測的信心度閾值 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |

### 同步配置 (`[sync]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_method` | String | "auto" | **呼叫樹:**<br>• `SyncCommand::execute()` → `determine_sync_method()` (line 75) → `src/commands/sync_command.rs:75`<br>• `determine_sync_method()` 根據配置決定同步方法 (line 168) → `src/commands/sync_command.rs:168`<br>• `SyncEngine::detect_sync_offset()` → `determine_default_method()` (line 78) → `src/core/sync/engine.rs:78`<br>• `determine_default_method()` 將配置轉換為實際的同步策略 (line 168) → `src/core/sync/engine.rs:168` | 選擇預設的同步方法 ("vad", "auto") | `subx-cli sync` | ✅ 使用中 |
| `max_offset_seconds` | f32 | 60.0 | **呼叫樹:**<br>• `SyncConfig` 結構定義 → `src/config/mod.rs:202`<br>• `validate()` 用於配置驗證 (line 98) → `src/config/validator.rs:98`<br>• **⚠️ 未發現實際業務邏輯使用**：配置可設定和驗證，但未在同步引擎中實際使用 | 設計用於最大允許時間偏移量，但**未實現限制邏輯** | `subx-cli sync` | ⚠️ 已定義但未使用 |
| `vad.enabled` | bool | true | **呼叫樹:**<br>• `SyncCommand::execute()` (line 44) → `src/commands/sync_command.rs:44`<br>• `SyncEngine::new()` 檢查是否啟用 VAD (line 36) → `src/core/sync/engine.rs:36`<br>• 決定是否創建 `VadSyncDetector` 實例，影響同步功能可用性 | 是否啟用本地 VAD 方法 | `subx-cli sync` | ✅ 使用中 |
| `vad.sensitivity` | f32 | 0.75 | **呼叫樹:**<br>• `SyncCommand::execute()` → `SyncEngine::new()` → `VadSyncDetector::new()` → `LocalVadDetector::new()`<br>• `LocalVadDetector::detect_speech()` 使用於 VAD 檢測 (line 102) → `src/services/vad/detector.rs:102`<br>• 作為語音段的概率值 (line 129) → `src/services/vad/detector.rs:129`<br>• CLI 參數可覆蓋此設定 (line 184) → `src/commands/sync_command.rs:184` | 語音檢測敏感度，影響 VAD 算法的檢測靈敏度 | `subx-cli sync` | ✅ 使用中 |
| `vad.chunk_size` | usize | 512 | **呼叫樹:**<br>• `SyncCommand::execute()` → `SyncEngine::new()` → `VadSyncDetector::new()` → `LocalVadDetector::new()`<br>• `LocalVadDetector::detect_speech()` 創建 VAD 實例 (line 72) → `src/services/vad/detector.rs:72`<br>• `detect_speech_segments()` 計算塊持續時間 (line 94) → `src/services/vad/detector.rs:94`<br>• CLI 參數可覆蓋此設定 (line 187) → `src/commands/sync_command.rs:187` | 音訊塊大小，影響 VAD 處理精度和性能 | `subx-cli sync` | ✅ 使用中 |
| `vad.sample_rate` | u32 | 16000 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 44) → `src/commands/sync_command.rs:44`<br>• `SyncEngine::new()` (line 37) → `src/core/sync/engine.rs:37`<br>• `VadSyncDetector::new()` (line 34) → `src/services/vad/sync_detector.rs:34`<br>• `LocalVadDetector::new()` (line 37) → `src/services/vad/detector.rs:37`<br>• `VadAudioProcessor::new(cfg_clone.sample_rate, 1)` 實際使用於音訊處理 | 處理採樣率 | `subx-cli sync` | ✅ 使用中 |
| `vad.padding_chunks` | u32 | 3 | **呼叫樹:**<br>• `SyncCommand::execute()` → `SyncEngine::new()` → `VadSyncDetector::new()` → `LocalVadDetector::new()`<br>• `LocalVadDetector::detect_speech_segments()` 用於語音段標記 (line 103) → `src/services/vad/detector.rs:103`<br>• 在語音檢測前後添加填充塊，提高檢測穩定性 | 語音檢測前後填充塊數，影響語音段邊界檢測 | `subx-cli sync` | ✅ 使用中 |
| `vad.min_speech_duration_ms` | u32 | 100 | **呼叫樹:**<br>• `SyncCommand::execute()` → `SyncEngine::new()` → `VadSyncDetector::new()` → `LocalVadDetector::new()`<br>• `LocalVadDetector::detect_speech_segments()` 過濾短語音段 (line 125, 145) → `src/services/vad/detector.rs:125,145`<br>• 確保只保留持續時間足夠長的語音段 | 最小語音持續時間，過濾雜訊和短暫語音 | `subx-cli sync` | ✅ 使用中 |
| `vad.speech_merge_gap_ms` | u32 | 200 | **呼叫樹:**<br>• `SyncCommand::execute()` → `SyncEngine::new()` → `VadSyncDetector::new()` → `LocalVadDetector::new()`<br>• `LocalVadDetector::merge_close_segments()` 合併相近語音段 (line 166) → `src/services/vad/detector.rs:166`<br>• 決定相近語音段的合併閾值，影響最終語音段數量 | 語音段合併間隔，合併時間間隔短的語音段 | `subx-cli sync` | ✅ 使用中 |

**注意**: 以下配置項目已被棄用，保留僅為向後相容性：
<!-- 已移除舊項目: correlation_threshold, dialogue_detection_threshold -->
- `min_dialogue_duration_ms` (已棄用)
- `dialogue_merge_gap_ms` (已棄用)
- `enable_dialogue_detection` (已棄用)
- `audio_sample_rate` (已棄用)
- `auto_detect_sample_rate` (已棄用)

### 一般配置 (`[general]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `backup_enabled` | bool | false | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 317) → `src/commands/match_command.rs:317`<br>• 在 `MatchConfig` 中用於控制是否自動備份，支援環境變數 SUBX_BACKUP_ENABLED | 檔案匹配時是否自動備份 | `subx-cli match` | ✅ 使用中 |
| `max_concurrent_jobs` | usize | 4 | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 82) → `src/core/parallel/config.rs:82`<br>• `TaskScheduler::new()` (line 110, 143) → `src/core/parallel/scheduler.rs:110,143`<br>• 並行處理模式使用 | 並行任務調度器的最大並發數 | `subx-cli match` | ✅ 使用中 |
| `task_timeout_seconds` | u64 | 300 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 110, 143) → `src/core/parallel/scheduler.rs:110,143`<br>• 在任務執行時用作逾時控制 (line 220) → `src/core/parallel/scheduler.rs:220` | 任務執行逾時設定 | `subx-cli match` | ✅ 使用中 |
| `enable_progress_bar` | bool | true | **呼叫樹:**<br>• `execute_parallel_match()` (line 483) → `src/commands/match_command.rs:483`<br>• 控制是否顯示進度條 UI | 是否顯示進度條 | `subx-cli match` | ✅ 使用中 |
| `worker_idle_timeout_seconds` | u64 | 60 | **呼叫樹:**<br>• `TaskScheduler::new()` (line 111-112, 144-145) → `src/core/parallel/scheduler.rs:111-112,144-145`<br>• 在工作執行緒管理中用於回收閒置執行緒 (line 184) → `src/core/parallel/scheduler.rs:184` | 工作執行緒閒置逾時 | `subx-cli match` | ✅ 使用中 |

### 並行處理配置 (`[parallel]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `max_workers` | usize | num_cpus::get() | **呼叫樹:**<br>• `ParallelConfig` 結構定義 (line 346) → `src/config/mod.rs:346`<br>• `WorkerPool::new()` 中使用控制最大工作執行緒數 | 並行工作執行緒池的最大執行緒數量 | `subx-cli match` | ✅ 使用中 |
| `task_queue_size` | usize | 1000 | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 83) → `src/core/parallel/config.rs:83`<br>• 用於控制任務佇列容量 | 任務佇列大小限制 | `subx-cli match` | ✅ 使用中 |
| `enable_task_priorities` | bool | false | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 84) → `src/core/parallel/config.rs:84`<br>• `TaskScheduler::start_scheduler_loop()` (line 192) → `src/core/parallel/scheduler.rs:192`<br>• `TaskScheduler::submit_task()` (line 328) → `src/core/parallel/scheduler.rs:328` | 啟用任務優先級排程 | `subx-cli match` | ✅ 使用中 |
| `auto_balance_workers` | bool | true | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 85) → `src/core/parallel/config.rs:85`<br>• 用於工作負載平衡策略 | 自動平衡工作負載 | `subx-cli match` | ✅ 使用中 |
| `overflow_strategy` | OverflowStrategy | Block | **呼叫樹:**<br>• `ParallelConfig::from_app_config()` (line 86) → `src/core/parallel/config.rs:86`<br>• 佇列滿時的處理策略 | 任務佇列溢出策略 | `subx-cli match` | ✅ 使用中 |

## 狀態說明

- ✅ **使用中**: 配置項目已完全整合並在程式碼中實際使用
- ⚠️ **已定義但未使用**: 配置項目已定義並可設定，但核心功能未實作或未讀取此設定

## 總結

### 完全整合的配置 (30 項) - 含詳細呼叫樹
- **AI 配置**: 9/9 項已使用，包含完整的從配置載入到實際 API 呼叫的路徑，包括 provider 選擇和自訂 base_url
- **格式配置**: 4/4 項已使用，包含編碼檢測、格式轉換流程和預設編碼回退
- **同步配置**: 8/9 項已使用，包含新的 VAD 配置結構的完整實現路徑：
  - `default_method` - 實際用於同步方法選擇和引擎邏輯決策
  - `vad.enabled` - 控制 VAD 檢測器的創建和可用性
  - `vad.sample_rate` - 實際用於音訊處理器的採樣率設定
  - `vad.sensitivity` - 直接用於 VAD 算法的檢測靈敏度和概率計算
  - `vad.chunk_size` - 用於 VAD 實例創建和塊持續時間計算
  - `vad.padding_chunks` - 用於語音段標記的填充設定
  - `vad.min_speech_duration_ms` - 過濾短語音段的實際閾值
  - `vad.speech_merge_gap_ms` - 語音段合併邏輯的實際閾值
  - `max_offset_seconds` - **已定義但未使用**：配置可設定但未在同步邏輯中實際使用
- **一般配置**: 5/5 項已使用，包含備份、並行任務調度、進度條顯示和逾時設定
- **並行處理配置**: 5/5 項已使用，包含工作執行緒池管理、任務佇列和優先級系統

### 已棄用但保留的配置 (7 項)
- **同步配置**: `correlation_threshold`, `dialogue_detection_threshold`, `min_dialogue_duration_ms`, `dialogue_merge_gap_ms`, `enable_dialogue_detection`, `audio_sample_rate`, `auto_detect_sample_rate` - 已標記為 `#[deprecated]` 並且確認沒有在業務邏輯中被使用，但保留以維持向後相容性

**最後更新**: 2025-06-15 - 逐項驗證配置項目的實際使用情況，更新了詳細的呼叫樹和行號，識別出重大配置支援不一致性問題

## 配置一致性問題

### ⚠️ get_config_value 方法支援不完整

目前 `ProductionConfigService::get_config_value()` 方法只支援有限的配置鍵 (15 項)，但 `set_config_value()` 方法支援更多配置項目 (31 項)：

**get_config_value 支援的配置鍵 (15 項)** → `src/config/service.rs:522-542`：
- AI: provider, model, api_key, base_url, temperature, max_tokens (缺少: max_sample_length, retry_attempts, retry_delay_ms)
- 格式: default_output, default_encoding, preserve_styling (缺少: encoding_detection_confidence)
- 同步: max_offset_seconds, correlation_threshold, audio_sample_rate (缺少: default_method 和所有 VAD 相關配置)
- 一般: backup_enabled, max_concurrent_jobs (缺少: task_timeout_seconds, enable_progress_bar, worker_idle_timeout_seconds)
- 並行: max_workers (缺少: task_queue_size, enable_task_priorities, auto_balance_workers, overflow_strategy)

**set_config_value 支援的配置鍵 (31 項)** → `src/config/service.rs:281-416`：
- AI: provider, model, api_key, base_url, max_sample_length, temperature, max_tokens, retry_attempts, retry_delay_ms (9 項)
- 格式: default_output, preserve_styling, default_encoding, encoding_detection_confidence (4 項)
- 同步（含舊項目）: max_offset_seconds, correlation_threshold, dialogue_detection_threshold, min_dialogue_duration_ms, dialogue_merge_gap_ms, enable_dialogue_detection, audio_sample_rate, auto_detect_sample_rate (8 項)
- 一般: backup_enabled, max_concurrent_jobs, task_timeout_seconds, enable_progress_bar, worker_idle_timeout_seconds (5 項)
- 並行: max_workers, task_queue_size, enable_task_priorities, auto_balance_workers, overflow_strategy (5 項)

### ⚠️ VAD 配置項目完全未實作

**嚴重問題**: 所有 VAD 相關配置 (7 項) 在 `get_config_value` 和 `set_config_value` 中都**完全缺失**：
- `sync.vad.enabled`, `sync.vad.sensitivity`, `sync.vad.chunk_size`, `sync.vad.sample_rate`
- `sync.vad.padding_chunks`, `sync.vad.min_speech_duration_ms`, `sync.vad.speech_merge_gap_ms`
- `sync.default_method` 也未在任何配置方法中實作

**影響**: 使用者無法透過 `subx config set/get` 命令操作任何 VAD 配置，這些配置只能透過配置檔案或環境變數設定

**建議修復**：
1. 擴展 `get_config_value` 方法以支援所有 `set_config_value` 支援的配置項目
2. **緊急**：新增對所有 VAD 相關配置的 `get_config_value` 和 `set_config_value` 支援：
   - `sync.vad.enabled`, `sync.vad.sensitivity`, `sync.vad.chunk_size`
   - `sync.vad.sample_rate`, `sync.vad.padding_chunks`
   - `sync.vad.min_speech_duration_ms`, `sync.vad.speech_merge_gap_ms`
3. 新增 `sync.default_method` 的配置方法支援
4. 確保 `config get` 和 `config set` 命令的一致性

**註**: 同步配置 CLI 現已完整支援 `sync.default_method` 及所有 VAD 相關配置，並已移除對已棄用項目（correlation_threshold, dialogue_detection_threshold）的 `set_config_value` 支援。
