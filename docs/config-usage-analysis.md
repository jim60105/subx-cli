# SubX 配置檔案使用情況分析

此文件分析 SubX 配置檔案中各項設定在程式碼中的實際使用情況，確保沒有多餘或未整合的配置。

## 配置設定使用分析表

### AI 配置 (`[ai]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `provider` | String | "openai" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 171) → `src/commands/match_command.rs:171`<br>• `ComponentFactory::create_ai_provider()` (line 102) → `src/core/factory.rs:102`<br>• `create_ai_provider()` (line 189) → `src/core/factory.rs:189`<br>• 根據 provider 類型選擇對應的 AI 客戶端實現 (line 191) → `src/core/factory.rs:191`<br>• 支援 "openai"、"openrouter"、"azure-openai" 三種提供商 | 用於選擇 AI 提供商類型，決定使用哪個 AI 服務 | `subx-cli match` | ✅ 使用中 |
| `api_key` | Option<String> | None | **呼叫樹:**<br>• `MatchCommand::execute()` (line 171) → `src/commands/match_command.rs:171`<br>• `ComponentFactory::create_ai_provider()` (line 102) → `src/core/factory.rs:102`<br>• `create_ai_provider()` (line 189) → `src/core/factory.rs:189`<br>• `OpenAIClient::from_config()` (line 252) → `src/services/ai/openai.rs:252`<br>• 檢驗 API 金鑰存在性 (line 253-255) → `src/services/ai/openai.rs:253-255`<br>• `OpenAIClient::new_with_base_url_and_timeout()` 傳遞至客戶端 (line 260) → `src/services/ai/openai.rs:260` | 用於 OpenAI API 認證，從配置或環境變數載入 | `subx-cli match` | ✅ 使用中 |
| `model` | String | "gpt-4.1-mini" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 171) → `src/commands/match_command.rs:171`<br>• `ComponentFactory::create_ai_provider()` (line 102) → `src/core/factory.rs:102`<br>• `create_ai_provider()` (line 189) → `src/core/factory.rs:189`<br>• `OpenAIClient::from_config()` (line 252) → `src/services/ai/openai.rs:252`<br>• `OpenAIClient::new_with_base_url_and_timeout()` 傳遞模型名稱 (line 260) → `src/services/ai/openai.rs:260`<br>• 在 HTTP 請求中指定使用的模型 (line 294) → `src/services/ai/openai.rs:294` | 指定使用的 AI 模型名稱，在 API 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `base_url` | String | "https://api.openai.com/v1" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 171) → `src/commands/match_command.rs:171`<br>• `ComponentFactory::create_ai_provider()` (line 102) → `src/core/factory.rs:102`<br>• `create_ai_provider()` (line 189) → `src/core/factory.rs:189`<br>• `OpenAIClient::from_config()` (line 252) → `src/services/ai/openai.rs:252`<br>• `OpenAIClient::validate_base_url()` 驗證 URL 格式 (line 269) → `src/services/ai/openai.rs:269`<br>• `OpenAIClient::new_with_base_url_and_timeout()` 設定基礎 URL (line 260) → `src/services/ai/openai.rs:260` | 支援自訂 API 端點，完整的配置到 HTTP 請求路徑 | `subx-cli match` | ✅ 使用中 |
| `max_sample_length` | usize | 3000 | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 318) → `src/commands/match_command.rs:318`<br>• 透過 `MatchConfig` 結構傳遞給 `MatchEngine` (line 326) → `src/commands/match_command.rs:326`<br>• `MatchEngine::create_content_preview()` 限制內容預覽長度 (line 758-759) → `src/core/matcher/engine.rs:758-759` | 控制傳送給 AI 的內容長度上限和內容預覽長度 | `subx-cli match` | ✅ 使用中 |
| `temperature` | f32 | 0.3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 171) → `src/commands/match_command.rs:171`<br>• `ComponentFactory::create_ai_provider()` (line 102) → `src/core/factory.rs:102`<br>• `create_ai_provider()` (line 189) → `src/core/factory.rs:189`<br>• `OpenAIClient::from_config()` (line 252) → `src/services/ai/openai.rs:252`<br>• `OpenAIClient::new_with_base_url_and_timeout()` 傳遞溫度參數 (line 260) → `src/services/ai/openai.rs:260`<br>• `OpenAIClient::chat_completion()` HTTP 請求中使用 (line 296) → `src/services/ai/openai.rs:296` | 控制 AI 回應的隨機性，在 HTTP 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `max_tokens` | u32 | 10000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 171) → `src/commands/match_command.rs:171`<br>• `ComponentFactory::create_ai_provider()` (line 102) → `src/core/factory.rs:102`<br>• `create_ai_provider()` (line 189) → `src/core/factory.rs:189`<br>• `OpenAIClient::from_config()` (line 252) → `src/services/ai/openai.rs:252`<br>• `OpenAIClient::new_with_base_url_and_timeout()` 傳遞 token 限制 (line 260) → `src/services/ai/openai.rs:260`<br>• `OpenAIClient::chat_completion()` HTTP 請求中使用 (line 297) → `src/services/ai/openai.rs:297` | 控制 AI 回應的最大 token 數量限制 | `subx-cli match` | ✅ 使用中 |
| `retry_attempts` | u32 | 3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 171) → `src/commands/match_command.rs:171`<br>• `ComponentFactory::create_ai_provider()` (line 102) → `src/core/factory.rs:102`<br>• `create_ai_provider()` (line 189) → `src/core/factory.rs:189`<br>• `OpenAIClient::from_config()` (line 252) → `src/services/ai/openai.rs:252`<br>• `OpenAIClient::new_with_base_url_and_timeout()` 傳遞重試次數 (line 260) → `src/services/ai/openai.rs:260`<br>• `OpenAIClient::make_request_with_retry()` 重試邏輯 (line 367) → `src/services/ai/openai.rs:367` | API 請求失敗時的重試次數 | `subx-cli match` | ✅ 使用中 |
| `retry_delay_ms` | u64 | 1000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 171) → `src/commands/match_command.rs:171`<br>• `ComponentFactory::create_ai_provider()` (line 102) → `src/core/factory.rs:102`<br>• `create_ai_provider()` (line 189) → `src/core/factory.rs:189`<br>• `OpenAIClient::from_config()` (line 252) → `src/services/ai/openai.rs:252`<br>• `OpenAIClient::new_with_base_url_and_timeout()` 傳遞延遲設定 (line 260) → `src/services/ai/openai.rs:260`<br>• `OpenAIClient::make_request_with_retry()` 延遲邏輯 (line 367) → `src/services/ai/openai.rs:367` | API 重試之間的延遲時間 | `subx-cli match` | ✅ 使用中 |
| `request_timeout_seconds` | u64 | 120 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 171) → `src/commands/match_command.rs:171`<br>• `ComponentFactory::create_ai_provider()` (line 102) → `src/core/factory.rs:102`<br>• `create_ai_provider()` (line 189) → `src/core/factory.rs:189`<br>• `OpenAIClient::from_config()` (line 252) → `src/services/ai/openai.rs:252`<br>• `OpenAIClient::new_with_base_url_and_timeout()` (line 260) → `src/services/ai/openai.rs:260`<br>• 設定 HTTP 客戶端超時 (line 231, 234) → `src/services/ai/openai.rs:231,234` | HTTP 請求超時時間，適用於慢速網路或複雜請求 | `subx-cli match` | ✅ 使用中 |
| `api_version` | Option<String> | None | **呼叫樹:**<br>• `AzureOpenAIClient::from_config()` (line 60) → `src/services/ai/azure_openai.rs:60`<br>• 取得 API 版本或使用預設值 (line 71-73) → `src/services/ai/azure_openai.rs:71-73`<br>• `AzureOpenAIClient::new_with_all()` 傳遞版本參數 (line 77) → `src/services/ai/azure_openai.rs:77`<br>• 用於 Azure OpenAI API 請求的版本控制 | Azure OpenAI API 版本控制，為 Azure 提供商專用 | `subx-cli match` | ✅ 使用中 |

### 格式配置 (`[formats]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_output` | String | "srt" | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 204) → `src/commands/convert_command.rs:204`<br>• 決定預設的輸出字幕格式 (line 217) → `src/commands/convert_command.rs:217`<br>• 支援 srt/ass/vtt/sub 格式轉換 (line 228) → `src/commands/convert_command.rs:228` | CLI 轉換命令的預設輸出格式 | `subx-cli convert` | ✅ 使用中 |
| `preserve_styling` | bool | false | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 204) → `src/commands/convert_command.rs:204`<br>• `ConversionConfig` 中使用控制樣式保留 (line 209) → `src/commands/convert_command.rs:209`<br>• `FormatConverter::new()` 傳遞配置 (line 213) → `src/commands/convert_command.rs:213` | 控制格式轉換時是否保留樣式 | `subx-cli convert` | ✅ 使用中 |
| `default_encoding` | String | "utf-8" | **呼叫樹:**<br>• `EncodingDetector::new()` (line 21) → `src/core/formats/encoding/detector.rs:21`<br>• `EncodingDetector::with_config()` (line 41) → `src/core/formats/encoding/detector.rs:41`<br>• `EncodingDetector::select_best_encoding()` 回退編碼 (line 306, 314, 321, 329) → `src/core/formats/encoding/detector.rs:306,314,321,329` | 編碼檢測失敗時的預設編碼回退 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |
| `encoding_detection_confidence` | f32 | 0.8 | **呼叫樹:**<br>• `EncodingDetector::new()` (line 18) → `src/core/formats/encoding/detector.rs:18`<br>• `EncodingDetector::with_config()` (line 38) → `src/core/formats/encoding/detector.rs:38`<br>• `EncodingDetector::select_best_encoding()` (line 320) → `src/core/formats/encoding/detector.rs:320` | 編碼自動檢測的信心度閾值 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |

### 同步配置 (`[sync]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_method` | String | "auto" | **呼叫樹:**<br>• `SyncCommand::execute()` → `determine_sync_method()` (line 75) → `src/commands/sync_command.rs:75`<br>• `determine_sync_method()` 根據配置決定同步方法 (line 406, 416) → `src/commands/sync_command.rs:406,416`<br>• `SyncEngine::detect_sync_offset()` → `determine_default_method()` (line 78) → `src/core/sync/engine.rs:78`<br>• `determine_default_method()` 將配置轉換為實際的同步策略 (line 168) → `src/core/sync/engine.rs:168` | 選擇預設的同步方法 ("vad", "auto") | `subx-cli sync` | ✅ 使用中 |
| `max_offset_seconds` | f32 | 60.0 | **呼叫樹:**<br>• `SyncConfig` 結構定義 → `src/config/mod.rs:231`<br>• `validate()` 用於配置驗證 (line 215-232) → `src/config/validator.rs:215-232`<br>• `SyncEngine::apply_offset()` 驗證偏移量限制 (line 125-128) → `src/core/sync/engine.rs:125-128`<br>• `SyncEngine::detect_sync_offset()` 驗證檢測結果 (line 199-203) → `src/core/sync/engine.rs:199-203`<br>• 超過限制時進行偏移量夾緊處理 (line 213) → `src/core/sync/engine.rs:213` | 限制最大允許的時間偏移量，防止不合理的同步結果 | `subx-cli sync` | ✅ 使用中 |
| `vad.enabled` | bool | true | **呼叫樹:**<br>• `SyncCommand::execute()` (line 44) → `src/commands/sync_command.rs:44`<br>• `SyncEngine::new()` 檢查是否啟用 VAD (line 36) → `src/core/sync/engine.rs:36`<br>• 決定是否創建 `VadSyncDetector` 實例，影響同步功能可用性 | 是否啟用本地 VAD 方法 | `subx-cli sync` | ✅ 使用中 |
| `vad.sensitivity` | f32 | 0.25 | **呼叫樹:**<br>• `SyncCommand::execute()` → `SyncEngine::new()` → `VadSyncDetector::new()` → `LocalVadDetector::new()`<br>• `LocalVadDetector::detect_speech()` 使用於 VAD 檢測 (line 135) → `src/services/vad/detector.rs:135`<br>• 作為語音段的概率值 (line 137) → `src/services/vad/detector.rs:137`<br>• CLI 參數可覆蓋此設定 (line 432) → `src/commands/sync_command.rs:432` | 語音檢測敏感度，影響 VAD 算法的檢測靈敏度 | `subx-cli sync` | ✅ 使用中 |
| `vad.min_speech_duration_ms` | u32 | 300 | **呼叫樹:**<br>• `SyncCommand::execute()` → `SyncEngine::new()` → `VadSyncDetector::new()` → `LocalVadDetector::new()`<br>• `LocalVadDetector::detect_speech_segments()` 過濾短語音段 (line 165, 190) → `src/services/vad/detector.rs:165,190`<br>• 確保只保留持續時間足夠長的語音段 | 最小語音持續時間，過濾雜訊和短暫語音 | `subx-cli sync` | ✅ 使用中 |

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
| `backup_enabled` | bool | false | **呼叫樹:**<br>• `ComponentFactory::create_file_manager()` (line 75) → `src/core/factory.rs:75`<br>• `MatchCommand::execute_with_client()` (line 321) → `src/commands/match_command.rs:321`<br>• `MatchEngine::apply_single_match()` 控制檔案備份 (line 816) → `src/core/matcher/engine.rs:816`<br>• 決定在移動檔案時是否創建備份 | 檔案匹配時是否自動備份 | `subx-cli match` | ✅ 使用中 |
| `max_concurrent_jobs` | usize | 4 | **呼叫樹:**<br>• `TaskScheduler::new_with_config()` (line 93) → `src/core/parallel/scheduler.rs:93`<br>• 創建信號量限制並發任務數量 (line 127) → `src/core/parallel/scheduler.rs:127`<br>• `active_task_count()` 計算活躍任務數量 (line 462) → `src/core/parallel/scheduler.rs:462` | 並行任務調度器的最大並發數 | `subx-cli match` | ✅ 使用中 |
| `task_timeout_seconds` | u64 | 300 | **呼叫樹:**<br>• `TaskScheduler::new_with_config()` (line 105) → `src/core/parallel/scheduler.rs:105`<br>• 設定任務執行逾時時間 (line 105) → `src/core/parallel/scheduler.rs:105`<br>• 從一般配置讀取超時設定 | 任務執行逾時設定 | `subx-cli match` | ✅ 使用中 |
| `workspace` | PathBuf | "." | **呼叫樹:**<br>• `cli::run()` 切換工作目錄 (line 163) → `src/cli/mod.rs:163`<br>• 支援透過 `SUBX_WORKSPACE` 環境變數覆蓋 (line 154) → `src/cli/mod.rs:154`<br>• 設定 CLI 命令的工作目錄 | 工作目錄設定，影響檔案路徑解析 | 所有子命令 | ✅ 使用中 |
| `enable_progress_bar` | bool | true | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 494) → `src/commands/match_command.rs:494`<br>• 控制是否顯示進度條 UI | 是否顯示進度條 | `subx-cli match` | ✅ 使用中 |
| `worker_idle_timeout_seconds` | u64 | 60 | **呼叫樹:**<br>• `TaskScheduler::new_with_config()` (line 112, 145) → `src/core/parallel/scheduler.rs:112,145`<br>• 設定工作執行緒閒置逾時時間 | 工作執行緒閒置逾時 | `subx-cli match` | ✅ 使用中 |

### 並行處理配置 (`[parallel]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `max_workers` | usize | num_cpus::get() | **呼叫樹:**<br>• `ConfigValidator::validate()` 驗證工作執行緒數量 (line 189-190, 218-221) → `src/config/validator.rs:189-190,218-221`<br>• 與 `max_concurrent_jobs` 進行相容性檢查<br>• 用於建立配置時的參考值 | 並行工作執行緒池的最大執行緒數量（主要用於驗證） | `subx-cli match` | ✅ 使用中 |
| `task_queue_size` | usize | 1000 | **呼叫樹:**<br>• `TaskScheduler::submit_task()` 控制佇列容量 (line 296, 392) → `src/core/parallel/scheduler.rs:296,392`<br>• `TaskScheduler::submit_batch()` 批次任務提交檢查 (line 392) → `src/core/parallel/scheduler.rs:392`<br>• 與 `queue_overflow_strategy` 結合控制佇列溢出行為 | 任務佇列大小限制 | `subx-cli match` | ✅ 使用中 |
| `enable_task_priorities` | bool | false | **呼叫樹:**<br>• `TaskScheduler::submit_task()` 任務優先級排程 (line 328) → `src/core/parallel/scheduler.rs:328`<br>• `TaskScheduler::submit_batch()` 批次任務優先級處理 (line 421) → `src/core/parallel/scheduler.rs:421`<br>• `ParallelConfig::from_app_config()` 負載平衡器初始化 (line 192) → `src/core/parallel/scheduler.rs:192` | 啟用任務優先級排程 | `subx-cli match` | ✅ 使用中 |
| `auto_balance_workers` | bool | true | **呼叫樹:**<br>• `TaskScheduler::new_with_config()` 負載平衡器初始化 (line 105) → `src/core/parallel/scheduler.rs:105`<br>• 決定是否創建 `LoadBalancer` 實例來分散工作負載 | 自動平衡工作負載 | `subx-cli match` | ✅ 使用中 |
| `overflow_strategy` | OverflowStrategy | Block | **呼叫樹:**<br>• `TaskScheduler::submit_task()` 佇列滿時的處理策略 (line 297) → `src/core/parallel/scheduler.rs:297`<br>• `TaskScheduler::submit_batch()` 批次任務溢出處理 (line 393) → `src/core/parallel/scheduler.rs:393`<br>• 支援 Block、DropOldest、Reject、Drop、Expand 五種策略 | 任務佇列溢出策略 | `subx-cli match` | ✅ 使用中 |

## 狀態說明

- ✅ **使用中**: 配置項目已完全整合並在程式碼中實際使用
- ⚠️ **已定義但未使用**: 配置項目已定義並可設定，但核心功能未實作或未讀取此設定

## 總結

### 完全整合的配置（32 項，含詳細呼叫樹）
- **AI 配置**：11/11 項已使用，包含完整的從配置載入到實際 API 呼叫的路徑，包括 provider 選擇、自訂 base_url、request_timeout_seconds 及新增的 api_version
- **格式配置**：4/4 項已使用，涵蓋編碼檢測、格式轉換流程和預設編碼回退
- **同步配置**：6/6 項已使用，VAD 配置結構完整整合：
  - `default_method`、`max_offset_seconds`、`vad.enabled`、`vad.sensitivity`、`vad.padding_chunks`、`vad.min_speech_duration_ms` 均有完整實作
  - 移除不存在的配置項目：`vad.chunk_size`、`vad.sample_rate`、`vad.speech_merge_gap_ms`
- **一般配置**：6/6 項已使用，包含備份、並行任務調度、逾時設定、工作目錄和進度條顯示
- **並行處理配置**：5/5 項已使用，涵蓋工作執行緒池管理、任務佇列和優先級系統

### 已棄用但保留的配置（7 項）
- **同步配置**：`correlation_threshold`、`dialogue_detection_threshold`、`min_dialogue_duration_ms`、`dialogue_merge_gap_ms`、`enable_dialogue_detection`、`audio_sample_rate`、`auto_detect_sample_rate` — 已標記為 `#[deprecated]` 並確認未在業務邏輯中被使用，僅保留以維持向後相容性

**最後更新**：2025-07-07 — 完成所有配置項目全面審查，更新行號、添加 `api_version` 配置、修正 VAD 配置預設值、移除不存在的配置項目、補完一般配置項目，確保所有呼叫樹準確無誤。

## 配置一致性問題

### ✅ 配置方法支援已完整

`ProductionConfigService::get_config_value()` 和 `set_config_value()` 方法現已支援所有配置項目：

**get_config_value 支援的配置鍵 (32 項)** → `src/config/service.rs:523-572`：
- AI: provider, model, api_key, base_url, max_sample_length, temperature, max_tokens, retry_attempts, retry_delay_ms, request_timeout_seconds (10 項)
- 格式: default_output, default_encoding, preserve_styling, encoding_detection_confidence (4 項)
- 同步: default_method, max_offset_seconds, vad.enabled, vad.sensitivity, vad.chunk_size, vad.sample_rate, vad.padding_chunks, vad.min_speech_duration_ms, vad.speech_merge_gap_ms (9 項)
- 一般: backup_enabled, max_concurrent_jobs, task_timeout_seconds, enable_progress_bar, worker_idle_timeout_seconds (5 項)
- 並行: max_workers, task_queue_size, enable_task_priorities, auto_balance_workers, overflow_strategy (5 項)

**set_config_value 支援的配置鍵 (32 項)** → `src/config/service.rs:283-416`：
- AI: provider, model, api_key, base_url, max_sample_length, temperature, max_tokens, retry_attempts, retry_delay_ms, request_timeout_seconds (10 項)
- 格式: default_output, preserve_styling, default_encoding, encoding_detection_confidence (4 項)
- 同步: default_method, max_offset_seconds, vad.enabled, vad.sensitivity, vad.chunk_size, vad.sample_rate, vad.padding_chunks, vad.min_speech_duration_ms, vad.speech_merge_gap_ms (9 項)
- 一般: backup_enabled, max_concurrent_jobs, task_timeout_seconds, enable_progress_bar, worker_idle_timeout_seconds (5 項)
- 並行: max_workers, task_queue_size, enable_task_priorities, auto_balance_workers, overflow_strategy (5 項)

**已修復**: 之前存在的配置方法不一致性問題已完全解決：
1. ✅ `get_config_value` 現已支援所有 `set_config_value` 支援的配置項目
2. ✅ 所有 VAD 相關配置現已完整支援 `get_config_value` 和 `set_config_value`：
   - `sync.vad.enabled`, `sync.vad.sensitivity`, `sync.vad.chunk_size`
   - `sync.vad.sample_rate`, `sync.vad.padding_chunks`
   - `sync.vad.min_speech_duration_ms`, `sync.vad.speech_merge_gap_ms`
3. ✅ `sync.default_method` 現已支援配置方法
4. ✅ `config get` 和 `config set` 命令現已完全一致
