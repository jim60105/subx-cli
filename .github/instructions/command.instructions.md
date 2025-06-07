# SubX 配置檔案使用情況分析

此文件分析 SubX 配置檔案中各項設定在程式碼中的實際使用情況，確保沒有多餘的設計。

## 配置設定使用分析表

### AI 配置 (`[ai]`)

| 配置項目 | 類型 | 默認值 | 實際使用位置 | 使用方式 | 狀態 |
|---------|------|---------|-------------|---------|------|
| `provider` | String | "openai" | • `src/config.rs:246-250` - 驗證 AI provider<br>• `src/config.rs:263` - 取得配置值<br>• `src/commands/match_command.rs:19` - 透過 OpenAIClient::new 間接使用 | • 驗證只支援 "openai"<br>• 透過配置系統存取<br>• 用於建立 AI 客戶端 | ✅ **使用中** |
| `model` | String | "gpt-4o-mini" | • `src/commands/match_command.rs:19` - 傳遞給 OpenAIClient::new<br>• `src/services/ai/openai.rs:136` - 在 API 請求中使用<br>• `src/commands/config_command.rs:16` - 設定配置值<br>• `src/core/matcher/cache.rs:30` - 記錄於快取中 | • 指定 OpenAI API 使用的模型<br>• 在 chat completion 請求中傳送<br>• 支援配置變更<br>• 快取驗證依據 | ✅ **使用中** |
| `max_sample_length` | usize | 2000 | • `src/commands/match_command.rs:29` - 傳遞給 MatchConfig<br>• `src/core/matcher/engine.rs:19` - 匹配引擎配置<br>• `src/core/matcher/engine.rs:139-140` - 限制內容預覽長度 | • 控制字幕內容採樣的最大長度<br>• 防止過長的內容影響 AI 分析效率<br>• 在產生內容預覽時截斷文字 | ✅ **使用中** |
| `api_key` | Option\<String\> | None | • `src/commands/match_command.rs:13-19` - 驗證並傳遞給 OpenAIClient<br>• `src/services/ai/openai.rs:14,145` - 儲存並用於 API 授權<br>• `src/config.rs:237-238` - 從環境變數載入<br>• `src/commands/config_command.rs:15` - 設定配置值 | • OpenAI API 授權金鑰<br>• 支援環境變數 OPENAI_API_KEY<br>• 在 HTTP 請求標頭中使用<br>• 必需的配置項目 | ✅ **使用中** |
| `temperature` | f32 | 0.3 | • `src/services/ai/openai.rs:138` - **硬編碼為 0.3** | • 應該控制 AI 回應的隨機性<br>• **目前被硬編碼，未使用配置值** | ❌ **未使用** |
| `retry_attempts` | u32 | 3 | **無實際使用位置** | • 應該控制 API 重試次數<br>• **重試功能模組存在但未整合** | ❌ **未使用** |
| `retry_delay_ms` | u64 | 1000 | **無實際使用位置** | • 應該控制重試延遲時間<br>• **重試功能模組存在但未整合** | ❌ **未使用** |

### 格式配置 (`[formats]`)

| 配置項目 | 類型 | 默認值 | 實際使用位置 | 使用方式 | 狀態 |
|---------|------|---------|-------------|---------|------|
| `default_output` | String | "srt" | • `src/commands/config_command.rs:22` - 設定配置值<br>• `src/config.rs:269` - 取得配置值 | • 支援透過 config 命令設定<br>• 可透過配置系統存取<br>• **在轉換命令中未實際使用** | ⚠️ **部分使用** |
| `preserve_styling` | bool | true | • `src/core/formats/transformers.rs:42,56,86` - 格式轉換時保留樣式<br>• `src/commands/convert_command.rs:7` - **硬編碼為 true** | • 控制格式轉換時是否保留 HTML 標籤等樣式<br>• **convert 命令中硬編碼，未使用配置值** | ⚠️ **部分使用** |
| `default_encoding` | String | "utf-8" | **無實際使用位置** | • 應該控制檔案的默認編碼<br>• **未在任何轉換或讀取邏輯中使用** | ❌ **未使用** |

### 同步配置 (`[sync]`)

| 配置項目 | 類型 | 默認值 | 實際使用位置 | 使用方式 | 狀態 |
|---------|------|---------|-------------|---------|------|
| `max_offset_seconds` | f32 | 30.0 | • `src/core/sync/engine.rs:95` - 計算最大偏移樣本數<br>• `src/commands/sync_command.rs:13` - **使用命令列參數而非配置** | • 限制音訊同步的最大偏移範圍<br>• **sync 命令使用 args.range，忽略配置值** | ⚠️ **部分使用** |
| `audio_sample_rate` | u32 | 16000 | **無實際使用位置** | • 應該控制音訊分析的採樣率<br>• **AudioAnalyzer 硬編碼為 16000** | ❌ **未使用** |
| `correlation_threshold` | f32 | 0.7 | • `src/core/sync/engine.rs:112` - 判斷相關性信心度<br>• `src/commands/sync_command.rs:14` - **硬編碼為 0.3** | • 判斷音訊相關性的閾值<br>• **sync 命令硬編碼，與配置值不符** | ⚠️ **部分使用** |
| `dialogue_detection_threshold` | f32 | 0.01 | **無實際使用位置** | • 應該控制對話檢測的敏感度<br>• **音訊分析中未實作對話檢測** | ❌ **未使用** |
| `min_dialogue_duration_ms` | u64 | 500 | **無實際使用位置** | • 應該控制最小對話持續時間<br>• **音訊分析中未實作對話檢測** | ❌ **未使用** |

### 一般配置 (`[general]`)

| 配置項目 | 類型 | 默認值 | 實際使用位置 | 使用方式 | 狀態 |
|---------|------|---------|-------------|---------|------|
| `backup_enabled` | bool | false | • `src/commands/match_command.rs:32` - 與命令列參數 OR 運算<br>• `src/core/matcher/engine.rs:174` - 決定是否備份檔案 | • 控制匹配操作時是否自動備份<br>• 與命令列 --backup 參數結合使用<br>• 實際影響檔案操作行為 | ✅ **使用中** |
| `default_confidence` | u8 | 80 | **無實際使用位置** | • 應該作為信心度閾值的預設值<br>• **CLI 參數有相同預設值，但未連結配置** | ❌ **未使用** |
| `max_concurrent_jobs` | usize | `num_cpus::get_physical()` | **無實際使用位置** | • 應該控制平行處理的最大任務數<br>• **目前沒有平行處理邏輯使用此設定** | ❌ **未使用** |
| `log_level` | String | "info" | **無實際使用位置** | • 應該控制日誌輸出級別<br>• **實際使用 env_logger::init()，從 RUST_LOG 環境變數讀取** | ❌ **未使用** |

## 未使用或部分使用的配置項目

以下配置項目在程式碼中定義但未被完全使用：

### ❌ 完全未使用的配置項目：
1. **`ai.temperature`** - 在 OpenAI 客戶端中硬編碼為 0.3，未讀取配置值
2. **`ai.retry_attempts`** - 重試功能模組存在但未整合到 AI 客戶端
3. **`ai.retry_delay_ms`** - 重試功能模組存在但未整合到 AI 客戶端
4. **`formats.default_encoding`** - 未在任何檔案讀取或轉換邏輯中使用
5. **`sync.audio_sample_rate`** - AudioAnalyzer 硬編碼為 16000
6. **`sync.dialogue_detection_threshold`** - 對話檢測功能未實作
7. **`sync.min_dialogue_duration_ms`** - 對話檢測功能未實作
8. **`general.default_confidence`** - CLI 參數有預設值但未連結配置
9. **`general.max_concurrent_jobs`** - 無平行處理邏輯使用此設定
10. **`general.log_level`** - 使用 env_logger，從環境變數讀取

### ⚠️ 部分使用的配置項目：
1. **`formats.default_output`** - 支援配置設定但轉換命令未使用
2. **`formats.preserve_styling`** - 轉換邏輯支援但命令中硬編碼
3. **`sync.max_offset_seconds`** - sync 命令使用命令列參數而非配置
4. **`sync.correlation_threshold`** - sync 命令硬編碼值與配置不符

## 建議

### 立即修復建議：
1. **修復 temperature 配置** - 在 OpenAIClient 中使用配置值而非硬編碼
2. **整合重試配置** - 在 OpenAI API 呼叫中使用 retry_attempts 和 retry_delay_ms
3. **連結同步配置** - 讓 sync 命令讀取配置檔案的預設值
4. **連結格式配置** - 讓 convert 命令使用配置中的 preserve_styling

### 移除建議：
1. **移除未實作功能的配置** - dialogue_detection_threshold, min_dialogue_duration_ms
2. **移除重複配置** - default_confidence（與 CLI 預設值重複）
3. **移除無用配置** - max_concurrent_jobs（無平行處理）, log_level（使用環境變數）

### 優化建議：
1. **統一配置管理** - 建立統一的配置載入機制，讓所有命令都能存取配置
2. **配置驗證** - 加強配置值的驗證和錯誤處理
3. **配置文件更新** - 移除多餘配置項目，更新文件說明
