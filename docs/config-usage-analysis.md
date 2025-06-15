# SubX 配置檔案使用情況分析

此文件分析 SubX 配置檔案中各項設定在程式碼中的實際使用情況，確保沒有多餘或未整合的配置。

## 配置設定使用分析表

### AI 配置 (`[ai]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `provider` | String | "openai" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `MatchCommand::execute_with_config()` (line 202) → `src/commands/match_command.rs:202`<br>• `AIClientFactory::create_client()` → `src/services/ai/factory.rs`<br>• `OpenAIClient::from_config()` 根據 provider 建立實例<br>• `AIValidator::validate()` (line 24-31) → `src/config/validator.rs:24-31` | 用於選擇 AI 提供商類型，目前支援 "openai" | `subx-cli match` | ✅ 使用中 |
| `api_key` | Option<String> | None | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `MatchCommand::execute_with_config()` (line 202) → `src/commands/match_command.rs:202`<br>• `OpenAIClient::from_config()` (line 213-216) → `src/services/ai/openai.rs:213-216`<br>• `AIValidator::validate()` (line 33-39) → `src/config/validator.rs:33-39`<br>• 環境變數 OPENAI_API_KEY 載入邏輯 | 用於 OpenAI API 認證，支援從環境變數載入 | `subx-cli match` | ✅ 使用中 |
| `model` | String | "gpt-4.1-mini" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `MatchCommand::execute_with_config()` (line 202) → `src/commands/match_command.rs:202`<br>• `OpenAIClient::from_config()` (line 218) → `src/services/ai/openai.rs:218`<br>• `MatchConfig` 結構中使用 (line 320) → `src/commands/match_command.rs:320`<br>• `OpenAIClient::chat_completion()` 使用模型進行 HTTP 請求 | 指定使用的 OpenAI 模型，在 HTTP 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `base_url` | String | "https://api.openai.com/v1" | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `MatchCommand::execute_with_config()` (line 202) → `src/commands/match_command.rs:202`<br>• `AIClientFactory::create_client()` → `src/services/ai/factory.rs`<br>• `OpenAIClient::from_config()` (line 221, 224) → `src/services/ai/openai.rs:221,224`<br>• `OpenAIClient::validate_base_url()` (line 233) → `src/services/ai/openai.rs:233` | 支援自訂 API 端點，完整從配置到實際 HTTP 請求的路徑 | `subx-cli match` | ✅ 使用中 |
| `max_sample_length` | usize | 3000 | **呼叫樹:**<br>• `MatchCommand::execute_with_client()` (line 314) → `src/commands/match_command.rs:314`<br>• 透過 `MatchConfig` 結構傳遞給 `MatchEngine` | 控制傳送給 AI 的內容長度上限 | `subx-cli match` | ✅ 使用中 |
| `temperature` | f32 | 0.3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `MatchCommand::execute_with_config()` (line 202) → `src/commands/match_command.rs:202`<br>• `OpenAIClient::from_config()` (line 219) → `src/services/ai/openai.rs:219`<br>• `OpenAIClient::chat_completion()` 在 HTTP 請求中使用<br>• `AIValidator::validate()` (line 42-47) → `src/config/validator.rs:42-47` | 控制 AI 回應的隨機性，在 HTTP 請求中使用 | `subx-cli match` | ✅ 使用中 |
| `retry_attempts` | u32 | 3 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `MatchCommand::execute_with_config()` (line 202) → `src/commands/match_command.rs:202`<br>• `OpenAIClient::from_config()` (line 220) → `src/services/ai/openai.rs:220`<br>• `OpenAIClient::make_request_with_retry()` (line 332) → `src/services/ai/openai.rs:332`<br>• `AIValidator::validate()` (line 50-52) → `src/config/validator.rs:50-52` | API 請求失敗時的重試次數 | `subx-cli match` | ✅ 使用中 |
| `retry_delay_ms` | u64 | 1000 | **呼叫樹:**<br>• `MatchCommand::execute()` (line 173) → `src/commands/match_command.rs:173`<br>• `MatchCommand::execute_with_config()` (line 202) → `src/commands/match_command.rs:202`<br>• `OpenAIClient::from_config()` (line 221) → `src/services/ai/openai.rs:221`<br>• `OpenAIClient::make_request_with_retry()` (line 337) → `src/services/ai/openai.rs:337` | API 重試之間的延遲時間 | `subx-cli match` | ✅ 使用中 |

### 格式配置 (`[formats]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_output` | String | "srt" | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 216) → `src/commands/convert_command.rs:216`<br>• `ConvertCommand::execute_with_config()` (line 264) → `src/commands/convert_command.rs:264`<br>• 用於決定預設的輸出字幕格式 | CLI 轉換命令的預設輸出格式 | `subx-cli convert` | ✅ 使用中 |
| `preserve_styling` | bool | false | **呼叫樹:**<br>• `ConvertCommand::execute()` (line 208) → `src/commands/convert_command.rs:208`<br>• `ConvertCommand::execute_with_config()` (line 256) → `src/commands/convert_command.rs:256`<br>• `ConversionConfig` 中使用以控制格式轉換時的樣式保留 | 控制格式轉換時是否保留樣式 | `subx-cli convert` | ✅ 使用中 |
| `default_encoding` | String | "utf-8" | **呼叫樹:**<br>• `EncodingDetector::new()` (line 21) → `src/core/formats/encoding/detector.rs:21`<br>• `EncodingDetector::with_config()` (line 41) → `src/core/formats/encoding/detector.rs:41`<br>• `EncodingDetector::select_best_encoding()` (line 306, 314, 321, 329) → `src/core/formats/encoding/detector.rs:306,314,321,329`<br>• 在編碼檢測失敗或信心度不足時用作回退編碼 | 編碼檢測失敗時的預設編碼回退 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |
| `encoding_detection_confidence` | f32 | 0.8 | **呼叫樹:**<br>• `EncodingDetector::new()` (line 18) → `src/core/formats/encoding/detector.rs:18`<br>• `EncodingDetector::with_config()` (line 38) → `src/core/formats/encoding/detector.rs:38`<br>• `EncodingDetector::select_best_encoding()` (line 319) → `src/core/formats/encoding/detector.rs:319` | 編碼自動檢測的信心度閾值 | `subx-cli detect-encoding`, `subx-cli convert` | ✅ 使用中 |

### 同步配置 (`[sync]`)

| 配置項目 | 類型 | 預設值 | 實際使用位置 | 使用方式 | 使用的子命令 | 狀態 |
|---------|------|---------|-------------|---------|-------------|------|
| `default_method` | String | "auto" | **呼叫樹:**<br>• `SyncCommand::execute()` (line 76) → `src/commands/sync_command.rs:76`<br>• `determine_sync_method()` 用於選擇同步方法 | 選擇預設的同步方法 ("vad", "auto") | `subx-cli sync` | ✅ 使用中 |
| `max_offset_seconds` | f32 | 60.0 | **呼叫樹:**<br>• `SyncConfig` 結構定義 (line 199) → `src/config/mod.rs:199`<br>• `SyncEngine::new()` 中載入配置 | 最大允許時間偏移量 | `subx-cli sync` | ✅ 使用中 |
| `vad.enabled` | bool | true | **呼叫樹:**<br>• `VadConfig` 結構定義 (line 243) → `src/config/mod.rs:243`<br>• 本地 VAD 方法啟用控制 | 是否啟用本地 VAD 方法 | `subx-cli sync` | ✅ 使用中 |
| `vad.sensitivity` | f32 | 0.75 | **呼叫樹:**<br>• `apply_cli_overrides()` (line 177) → `src/commands/sync_command.rs:177`<br>• CLI 參數可覆蓋此設定 | 語音檢測敏感度 | `subx-cli sync` | ✅ 使用中 |
| `vad.chunk_size` | usize | 512 | **呼叫樹:**<br>• `apply_cli_overrides()` (line 180) → `src/commands/sync_command.rs:180`<br>• CLI 參數可覆蓋此設定 | 音訊塊大小 | `subx-cli sync` | ✅ 使用中 |
| `vad.sample_rate` | u32 | 16000 | **呼叫樹:**<br>• `VadConfig` 結構定義 (line 247) → `src/config/mod.rs:247` | 處理採樣率 | `subx-cli sync` | ✅ 使用中 |
| `vad.padding_chunks` | u32 | 3 | **呼叫樹:**<br>• `VadConfig` 結構定義 (line 249) → `src/config/mod.rs:249` | 語音檢測前後填充塊數 | `subx-cli sync` | ✅ 使用中 |
| `vad.min_speech_duration_ms` | u32 | 100 | **呼叫樹:**<br>• `VadConfig` 結構定義 (line 251) → `src/config/mod.rs:251` | 最小語音持續時間 | `subx-cli sync` | ✅ 使用中 |
| `vad.speech_merge_gap_ms` | u32 | 200 | **呼叫樹:**<br>• `VadConfig` 結構定義 (line 253) → `src/config/mod.rs:253` | 語音段合併間隔 | `subx-cli sync` | ✅ 使用中 |

**注意**: 以下配置項目已被棄用，保留僅為向後相容性：
- `correlation_threshold` (已棄用)
- `dialogue_detection_threshold` (已棄用)  
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

### 完全整合的配置 (29 項) - 含詳細呼叫樹
- **AI 配置**: 8/8 項已使用，包含完整的從配置載入到實際 API 呼叫的路徑，包括 provider 選擇和自訂 base_url
- **格式配置**: 4/4 項已使用，包含編碼檢測、格式轉換流程和預設編碼回退
- **同步配置**: 9/9 項已使用，包含新的 VAD 配置結構，舊配置已棄用但保留向後相容性
- **一般配置**: 5/5 項已使用，包含備份、並行任務調度、進度條顯示和逾時設定
- **並行處理配置**: 5/5 項已使用，包含工作執行緒池管理、任務佇列和優先級系統

### 已棄用但保留的配置 (7 項)
- **同步配置**: `correlation_threshold`, `dialogue_detection_threshold`, `min_dialogue_duration_ms`, `dialogue_merge_gap_ms`, `enable_dialogue_detection`, `audio_sample_rate`, `auto_detect_sample_rate` - 已標記為 `#[deprecated]` 但保留以維持向後相容性

**最後更新**: 2025-06-15 - 基於實際程式碼使用情況完成配置分析，更新了行號並確認所有配置項目的實際使用狀態

## 配置一致性問題

### ⚠️ get_config_value 方法支援不完整

目前 `ProductionConfigService::get_config_value()` 方法只支援有限的配置鍵，但實際程式碼中使用了更多配置項目：

**get_config_value 支援的配置鍵 (15 項)**：
- AI: provider, model, api_key, base_url, temperature (缺少: max_sample_length, retry_attempts, retry_delay_ms)
- 格式: default_output, default_encoding, preserve_styling (缺少: encoding_detection_confidence)
- 同步: max_offset_seconds, correlation_threshold, audio_sample_rate (缺少: default_method 和 VAD 相關配置)
- 一般: backup_enabled, max_concurrent_jobs (缺少: task_timeout_seconds, enable_progress_bar, worker_idle_timeout_seconds)
- 並行: max_workers (缺少: task_queue_size, enable_task_priorities, auto_balance_workers, overflow_strategy)

**建議修復**：
1. 擴展 `get_config_value` 方法以支援所有實際使用的配置項目
2. 或者移除未使用的配置項目以保持一致性
3. 確保 `config set` 命令能夠設定所有實際使用的配置項目
