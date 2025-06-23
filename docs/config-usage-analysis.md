# SubX 配置檔案使用情況分析

此文件分析 SubX 配置檔案中各項設定在程式碼中的實際使用情況，確保沒有多餘或未整合的配置。

## 配置設定使用分析表

### AI 配置 (`[ai]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `provider` | String | "openai" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 206) → `src/commands/match_command.rs:206`<br>• `ComponentFactory::create_ai_provider()` (line 101) → `src/core/factory.rs:101`<br>• `create_ai_provider()` (line 188) → `src/core/factory.rs:188`<br>• 根據 provider 類型選擇對應的 AI 客戶端實現 | 用於選擇 AI 提供商類型，目前支援 "openai" | `subx-cli match` | ✅ 使用中 |
| `api_key` | Option<String> | None | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 206) → `src/commands/match_command.rs:206`<br>• `ComponentFactory::create_ai_provider()` (line 101) → `src/core/factory.rs:101`<br>• `create_ai_provider()` (line 188) → `src/core/factory.rs:188`<br>• `OpenAIClient::from_config()` (line 251) → `src/services/ai/openai.rs:251`<br>• `OpenAIClient::new_with_base_url_and_timeout()` (line 224) → `src/services/ai/openai.rs:224`<br>• 在 HTTP 請求中作為 Authorization Bearer token (line 303) → `src/services/ai/openai.rs:303`<br>• `AIValidator::validate()` 驗證格式與前綴 (line 51) → `src/config/validator.rs:51` | 用於 OpenAI API 認證，支援從環境變數載入，並有格式驗證 | `subx-cli match` | ✅ 使用中 |
| `model` | String | "gpt-4.1-mini" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 206) → `src/commands/match_command.rs:206`<br>• `ComponentFactory::create_ai_provider()` (line 101) → `src/core/factory.rs:101`<br>• `create_ai_provider()` (line 188) → `src/core/factory.rs:188`<br>• `OpenAIClient::from_config()` (line 251) → `src/services/ai/openai.rs:251`<br>• `OpenAIClient::chat_completion()` HTTP 請求中使用 (line 294) → `src/services/ai/openai.rs:294`<br>• `AIValidator::validate_ai_model()` 驗證模型名稱 (line 71) → `src/config/validator.rs:71` | 指定使用的 OpenAI 模型，在 HTTP 請求與驗證中使用 | `subx-cli match` | ✅ 使用中 |
| `base_url` | String | "https://api.openai.com/v1" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 206) → `src/commands/match_command.rs:206`<br>• `ComponentFactory::create_ai_provider()` (line 101) → `src/core/factory.rs:101`<br>• `create_ai_provider()` (line 188) → `src/core/factory.rs:188`<br>• `OpenAIClient::from_config()` (line 250) → `src/services/ai/openai.rs:250`<br>• `OpenAIClient::validate_base_url()` 驗證 URL 格式 (line 257, 276) → `src/services/ai/openai.rs:257,276`<br>• `OpenAIClient::chat_completion()` HTTP 請求端點 (line 294) → `src/services/ai/openai.rs:294` | 支援自訂 API 端點，完整從配置到實際 HTTP 請求與驗證的路徑 | `subx-cli match` | ✅ 使用中 |
| `max_sample_length` | usize | 3000 | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 315) → `src/commands/match_command.rs:315`<br>• 透過 `MatchConfig` 結構傳遞給 `MatchEngine`<br>• `MatchEngine::create_content_preview()` 用於限制內容預覽長度 (line 758-759) → `src/core/matcher/engine.rs:758-759` | 控制傳送給 AI 的內容長度上限和內容預覽長度 | `subx-cli match` | ✅ 使用中 |
| `temperature` | f32 | 0.3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 206) → `src/commands/match_command.rs:206`<br>• `ComponentFactory::create_ai_provider()` (line 101) → `src/core/factory.rs:101`<br>• `create_ai_provider()` (line 188) → `src/core/factory.rs:188`<br>• `OpenAIClient::from_config()` (line 250) → `src/services/ai/openai.rs:250`<br>• `OpenAIClient::chat_completion()` HTTP 請求中使用 (line 296) → `src/services/ai/openai.rs:296`<br>• `AIValidator::validate_temperature()` 驗證範圍 (line 60, 74) → `src/config/validator.rs:60,74` | 控制 AI 回應的隨機性，在 HTTP 請求與驗證中使用 | `subx-cli match` | ✅ 使用中 |
| `max_tokens` | u32 | 10000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 206) → `src/commands/match_command.rs:206`<br>• `ComponentFactory::create_ai_provider()` (line 101) → `src/core/factory.rs:101`<br>• `create_ai_provider()` (line 188) → `src/core/factory.rs:188`<br>• `OpenAIClient::from_config()` (line 250) → `src/services/ai/openai.rs:250`<br>• `OpenAIClient::chat_completion()` HTTP 請求中使用 (line 297) → `src/services/ai/openai.rs:297`<br>• `AIValidator::validate_positive_number()` 驗證 (line 61) → `src/config/validator.rs:61` | 控制 AI 回應的最大 token 數量限制 | `subx-cli match` | ✅ 使用中 |
| `retry_attempts` | u32 | 3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 206) → `src/commands/match_command.rs:206`<br>• `ComponentFactory::create_ai_provider()` (line 101) → `src/core/factory.rs:101`<br>• `create_ai_provider()` (line 188) → `src/core/factory.rs:188`<br>• `OpenAIClient::from_config()` (line 250) → `src/services/ai/openai.rs:250`<br>• `OpenAIClient::make_request_with_retry()` 重試邏輯 (line 367) → `src/services/ai/openai.rs:367`<br>• `AIValidator::validate_positive_number()` 驗證 (line 85, 86) → `src/config/validator.rs:85,86` | API 請求失敗時的重試次數 | `subx-cli match` | ✅ 使用中 |
| `retry_delay_ms` | u64 | 1000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 206) → `src/commands/match_command.rs:206`<br>• `ComponentFactory::create_ai_provider()` (line 101) → `src/core/factory.rs:101`<br>• `create_ai_provider()` (line 188) → `src/core/factory.rs:188`<br>• `OpenAIClient::from_config()` (line 250) → `src/services/ai/openai.rs:250`<br>• `OpenAIClient::make_request_with_retry()` 延遲邏輯 (line 367) → `src/services/ai/openai.rs:367` | API 重試之間的延遲時間 | `subx-cli match` | ✅ 使用中 |
| `request_timeout_seconds` | u64 | 120 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 170) → `src/commands/match_command.rs:170`<br>• `MatchCommand::execute_with_config()` (line 206) → `src/commands/match_command.rs:206`<br>• `ComponentFactory::create_ai_provider()` (line 101) → `src/core/factory.rs:101`<br>• `create_ai_provider()` (line 188) → `src/core/factory.rs:188`<br>• `OpenAIClient::from_config()` (line 250) → `src/services/ai/openai.rs:250`<br>• `OpenAIClient::new_with_base_url_and_timeout()` 設定 HTTP 客戶端超時 (line 231, 234) → `src/services/ai/openai.rs:231,234`<br>• `AIValidator::validate_range()` 驗證範圍 10-600 秒 (line 91) → `src/config/validator.rs:91` | HTTP 請求超時時間，適用於慢速網路或複雜請求。完整的從配置到 HTTP 客戶端設定的路徑，包含驗證與錯誤處理 | `subx-cli match` | ✅ 使用中 |

### 格式配置 (`[formats]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_output` | String | "srt" | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 204) → `src/commands/convert_command.rs:204`<br>• 用於決定預設的輸出字幕格式 (line 217) → `src/commands/convert_command.rs:217`<br>• 支援 srt/ass/vtt/sub 格式 | CLI 轉換命令的預設輸出格式 | `subx-cli convert` | ✅ 使用中 |
| `preserve_styling` | bool | false | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 204) → `src/commands/convert_command.rs:204`<br>• `ConversionConfig` 中使用以控制格式轉換時的樣式保留 (line 209) → `src/commands/convert_command.rs:209`<br>• `FormatTransformer` 轉換流程判斷 (line 68, 82, 112) → `src/core/formats/transformers.rs:68,82,112` | 控制格式轉換時是否保留樣式 | `subx-cli convert` | ✅ 使用中 |
| `default_encoding` | String | "utf-8" | **呼叫樹:**<br>• `EncodingDetector::new()` (line 16) → `src/core/formats/encoding/detector.rs:16`<br>• `EncodingDetector::with_config()` (line 33) → `src/core/formats/encoding/detector.rs:33`<br>• `EncodingDetector::select_best_encoding()` (line 306, 314, 321, 329) → `src/core/formats/encoding/detector.rs:306,314,321,329`<br>• 在編碼檢測失敗或信心度不足時用作回退編碼 | 編碼檢測失敗時的預設編碼回退 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |
| `encoding_detection_confidence` | f32 | 0.8 | **呼叫樹:**<br>• `EncodingDetector::new()` (line 16) → `src/core/formats/encoding/detector.rs:16`<br>• `EncodingDetector::with_config()` (line 33) → `src/core/formats/encoding/detector.rs:33`<br>• `EncodingDetector::select_best_encoding()` (line 319) → `src/core/formats/encoding/detector.rs:319` | 編碼自動檢測的信心度閾值 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |

### 同步配置 (`[sync]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_method` | String | "auto" | **呼叫樹:**<br>• `SyncCommand::execute()` → `determine_sync_method()` (line 75) → `src/commands/sync_command.rs:75`<br>• `determine_sync_method()` 根據配置決定同步方法 (line 406, 416) → `src/commands/sync_command.rs:406,416`<br>• `SyncEngine::detect_sync_offset()` → `determine_default_method()` (line 78) → `src/core/sync/engine.rs:78`<br>• `determine_default_method()` 將配置轉換為實際的同步策略 (line 168) → `src/core/sync/engine.rs:168` | 選擇預設的同步方法 ("vad", "auto") | `subx-cli sync` | ✅ 使用中 |
| `max_offset_seconds` | f32 | 60.0 | **呼叫樹:**<br>• `SyncConfig` 結構定義 → `src/config/mod.rs:231`<br>• `validate()` 用於配置驗證 (line 215-232) → `src/config/validator.rs:215-232`<br>• `SyncEngine::apply_offset()` 驗證偏移量限制 (line 125-128) → `src/core/sync/engine.rs:125-128`<br>• `SyncEngine::detect_sync_offset()` 驗證檢測結果 (line 199-203) → `src/core/sync/engine.rs:199-203`<br>• 超過限制時進行偏移量夾緊處理 (line 213) → `src/core/sync/engine.rs:213` | 限制最大允許的時間偏移量，防止不合理的同步結果 | `subx-cli sync` | ✅ 使用中 |
| `vad.enabled` | bool | true | **呼叫樹:**<br>• `SyncCommand::execute()` (line 44) → `src/commands/sync_command.rs:44`<br>• `SyncEngine::new()` 檢查是否啟用 VAD (line 36) → `src/core/sync/engine.rs:36`<br>• 決定是否創建 `VadSyncDetector` 實例，影響同步功能可用性 | 是否啟用本地 VAD 方法 | `subx-cli sync` | ✅ 使用中 |
| `vad.sensitivity` | f32 | 0.75 | **呼叫樹:**<br>• `SyncCommand::execute()` → `SyncEngine::new()` → `VadSyncDetector::new()` → `LocalVadDetector::new()`<br>• `LocalVadDetector::detect_speech()` 使用於 VAD 檢測 (line 135) → `src/services/vad/detector.rs:135`<br>• 作為語音段的概率值 (line 137) → `src/services/vad/detector.rs:137`<br>• CLI 參數可覆蓋此設定 (line 432) → `src/commands/sync_command.rs:432` | 語音檢測敏感度，影響 VAD 算法的檢測靈敏度 | `subx-cli sync` | ✅ 使用中 |
| `vad.chunk_size` | usize | 512 | **呼叫樹:**<br>• `SyncCommand::execute()` → `SyncEngine::new()` → `VadSyncDetector::new()` → `LocalVadDetector::new()`<br>• `LocalVadDetector::detect_speech()` 創建 VAD 實例 (line 72) → `src/services/vad/detector.rs:72`<br>• `detect_speech_segments()` 計算塊持續時間 (line 131-132) → `src/services/vad/detector.rs:131-132`<br>• CLI 參數可覆蓋此設定 (line 187) → `src/commands/sync_command.rs:187` | 音訊塊大小，影響 VAD 處理精度和性能 | `subx-cli sync` | ✅ 使用中 |
| `vad.sample_rate` | u32 | 16000 | **呼叫樹:**<br>• `SyncCommand::execute()` (line 44) → `src/commands/sync_command.rs:44`<br>• `SyncEngine::new()` (line 37) → `src/core/sync/engine.rs:37`<br>• `VadSyncDetector::new()` (line 34) → `src/services/vad/sync_detector.rs:34`<br>• `LocalVadDetector::new()` (line 37) → `src/services/vad/detector.rs:37`<br>• `VadAudioProcessor::new(cfg_clone.sample_rate, 1)` 實際使用於音訊處理 | 處理採樣率 | `subx-cli sync` | ✅ 使用中 |
| `vad.padding_chunks` | u32 | 3 | **呼叫樹:**<br>• `SyncCommand::execute()` → `SyncEngine::new()` → `VadSyncDetector::new()` → `LocalVadDetector::new()`<br>• `LocalVadDetector::detect_speech_segments()` 用於語音段標記 (line 143) → `src/services/vad/detector.rs:143`<br>• 於標記語音區段時，決定前後要額外納入的非語音區塊數，提升語音段邊界穩定性 | 語音檢測前後填充塊數，影響語音段邊界檢測 | `subx-cli sync` | ✅ 使用中 |
| `vad.min_speech_duration_ms` | u32 | 100 | **呼叫樹:**<br>• `SyncCommand::execute()` → `SyncEngine::new()` → `VadSyncDetector::new()` → `LocalVadDetector::new()`<br>• `LocalVadDetector::detect_speech_segments()` 過濾短語音段 (line 165, 190) → `src/services/vad/detector.rs:165,190`<br>• 確保只保留持續時間足夠長的語音段 | 最小語音持續時間，過濾雜訊和短暫語音 | `subx-cli sync` | ✅ 使用中 |
| `vad.speech_merge_gap_ms` | u32 | 200 | **尚未實作**：目前程式碼未有任何合併語音段的邏輯或參數，僅文件與 README 有描述，實際未使用。 | 語音段合併間隔（尚未實作） | `subx-cli sync` | ❌ 未使用 |

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
| `backup_enabled` | bool | false | **呼叫樹:**<br>• `ConfigService::get_config_value()` / `set_config_value()` → `src/config/service.rs:385,566`<br>• `GeneralConfig` 結構定義 (line 346) → `src/config/mod.rs:346`<br>• 控制是否自動備份原始檔案 | 檔案匹配時是否自動備份 | `subx-cli match` | ✅ 使用中 |
| `max_concurrent_jobs` | usize | 4 | **呼叫樹:**<br>• `ConfigService::get_config_value()` / `set_config_value()` → `src/config/service.rs:389,568`<br>• `GeneralConfig` 結構定義 (line 347) → `src/config/mod.rs:347`<br>• `TaskScheduler::new()` 控制最大並發數 (line 93,127) → `src/core/parallel/scheduler.rs:93,127` | 並行任務調度器的最大並發數 | `subx-cli match` | ✅ 使用中 |
| `task_timeout_seconds` | u64 | 300 | **呼叫樹:**<br>• `ConfigService::get_config_value()` / `set_config_value()` → `src/config/service.rs:393,571`<br>• `GeneralConfig` 結構定義 (line 348) → `src/config/mod.rs:348`<br>• `TaskScheduler::new()` 任務逾時控制 (line 220) → `src/core/parallel/scheduler.rs:220` | 任務執行逾時設定 | `subx-cli match` | ✅ 使用中 |
| `enable_progress_bar` | bool | true | **呼叫樹:**<br>• `ConfigService::get_config_value()` / `set_config_value()` → `src/config/service.rs:397,574`<br>• `GeneralConfig` 結構定義 (line 350) → `src/config/mod.rs:350`<br>• 控制是否顯示進度條 UI | 是否顯示進度條 | `subx-cli match` | ✅ 使用中 |
| `worker_idle_timeout_seconds` | u64 | 60 | **呼叫樹:**<br>• `ConfigService::get_config_value()` / `set_config_value()` → `src/config/service.rs:401,577`<br>• `GeneralConfig` 結構定義 (line 351) → `src/config/mod.rs:351`<br>• 工作執行緒管理中用於回收閒置執行緒 | 工作執行緒閒置逾時 | `subx-cli match` | ✅ 使用中 |

### 並行處理配置 (`[parallel]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `max_workers` | usize | num_cpus::get() | **呼叫樹:**<br>• `ConfigService::get_config_value()` / `set_config_value()` → `src/config/service.rs`<br>• `ParallelConfig` 結構定義 (line 396) → `src/config/mod.rs:396`<br>• `WorkerPool::new()` 控制最大工作執行緒數 | 並行工作執行緒池的最大執行緒數量 | `subx-cli match` | ✅ 使用中 |
| `task_queue_size` | usize | 1000 | **呼叫樹:**<br>• `ConfigService::get_config_value()` / `set_config_value()` → `src/config/service.rs`<br>• `ParallelConfig` 結構定義 (line 398) → `src/config/mod.rs:398`<br>• `TaskScheduler` 控制任務佇列容量 (line 296,300,392,396) → `src/core/parallel/scheduler.rs:296,300,392,396` | 任務佇列大小限制 | `subx-cli match` | ✅ 使用中 |
| `enable_task_priorities` | bool | false | **呼叫樹:**<br>• `ConfigService::get_config_value()` / `set_config_value()` → `src/config/service.rs`<br>• `ParallelConfig` 結構定義 (line 400) → `src/config/mod.rs:400`<br>• `TaskScheduler` 任務優先級排程 (line 192,328,421) → `src/core/parallel/scheduler.rs:192,328,421` | 啟用任務優先級排程 | `subx-cli match` | ✅ 使用中 |
| `auto_balance_workers` | bool | true | **呼叫樹:**<br>• `ConfigService::get_config_value()` / `set_config_value()` → `src/config/service.rs`<br>• `ParallelConfig` 結構定義 (line 401) → `src/config/mod.rs:401`<br>• `TaskScheduler` 自動平衡工作負載 (line 105,138) → `src/core/parallel/scheduler.rs:105,138` | 自動平衡工作負載 | `subx-cli match` | ✅ 使用中 |
| `overflow_strategy` | OverflowStrategy | Block | **呼叫樹:**<br>• `ConfigService::get_config_value()` / `set_config_value()` → `src/config/service.rs`<br>• `ParallelConfig` 結構定義 (line 402) → `src/config/mod.rs:402`<br>• `TaskScheduler` 佇列滿時的處理策略 (line 297,393) → `src/core/parallel/scheduler.rs:297,393` | 任務佇列溢出策略 | `subx-cli match` | ✅ 使用中 |

## 狀態說明

- ✅ **使用中**: 配置項目已完全整合並在程式碼中實際使用
- ⚠️ **已定義但未使用**: 配置項目已定義並可設定，但核心功能未實作或未讀取此設定

## 總結

### 完全整合的配置（31 項，含詳細呼叫樹）
- **AI 配置**：10/10 項已使用，包含完整的從配置載入到實際 API 呼叫的路徑，包括 provider 選擇、自訂 base_url 及 request_timeout_seconds
- **格式配置**：4/4 項已使用，涵蓋編碼檢測、格式轉換流程和預設編碼回退
- **同步配置**：9/9 項已使用，VAD 配置結構完整整合，僅 `vad.speech_merge_gap_ms` 為「尚未實作」：
  - `default_method`、`max_offset_seconds`、`vad.enabled`、`vad.sample_rate`、`vad.sensitivity`、`vad.chunk_size`、`vad.padding_chunks`、`vad.min_speech_duration_ms` 均有完整實作
  - `vad.speech_merge_gap_ms` 僅文件描述，程式碼未實作，狀態為「❌ 未使用」
- **一般配置**：5/5 項已使用，包含備份、並行任務調度、進度條顯示和逾時設定
- **並行處理配置**：5/5 項已使用，涵蓋工作執行緒池管理、任務佇列和優先級系統

### 已棄用但保留的配置（7 項）
- **同步配置**：`correlation_threshold`、`dialogue_detection_threshold`、`min_dialogue_duration_ms`、`dialogue_merge_gap_ms`、`enable_dialogue_detection`、`audio_sample_rate`、`auto_detect_sample_rate` — 已標記為 `#[deprecated]` 並確認未在業務邏輯中被使用，僅保留以維持向後相容性

**最後更新**：2025-06-21 — 完成所有配置項目審查、補全與狀態標註，`vad.speech_merge_gap_ms` 明確標示為未實作，並同步 README 配置說明。

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
