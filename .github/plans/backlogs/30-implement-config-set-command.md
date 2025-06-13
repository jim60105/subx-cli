# Backlog 30: 實現 Config Set 指令功能

## 概覽

本 backlog 專注於實現 SubX 專案中 `config set` 指令的完整功能，包括配置值設定、驗證、持久化存儲以及完整的測試覆蓋。此功能將允許用戶通過 CLI 動態修改配置設定，並確保設定的值經過適當的驗證和類型檢查。

## 背景

目前 `config_command.rs` 中的 `ConfigAction::Set` 操作僅返回一個錯誤訊息，提示功能尚未實現：

```rust
ConfigAction::Set { .. } => {
    // TODO: Implement setting configuration values with ConfigService
    return Err(SubXError::config(
        "Setting configuration values not yet supported with ConfigService. Use config files or environment variables instead.",
    ));
}
```

此功能的實現需要：
1. 擴展 `ConfigService` trait 以支援配置值設定
2. 實現值驗證和類型轉換
3. 添加配置持久化功能
4. 確保測試隔離和並行執行安全性

## 目標

### 主要目標
1. **完整實現 Set 功能**：允許用戶通過 CLI 設定配置值
2. **類型安全驗證**：確保所有輸入值符合期望的資料類型和約束條件
3. **持久化存儲**：將設定的值保存到適當的配置檔案中
4. **完整測試覆蓋**：提供全面的單元測試和整合測試

### 次要目標
1. **用戶體驗優化**：提供清晰的錯誤訊息和操作確認
2. **安全性保障**：確保敏感資料（如 API 金鑰）的安全處理
3. **向後相容性**：確保不影響現有的配置加載功能

## 技術規格

### 支援的配置項目

基於 `Config` 結構和實際程式碼使用分析，支援以下所有實際使用的配置項目：

#### AI 配置（8 項）
- `ai.provider` (String): AI 服務提供商，支援 "openai", "anthropic", "local"
- `ai.api_key` (Option<String>): API 金鑰，支援環境變數 OPENAI_API_KEY
- `ai.model` (String): AI 模型名稱，如 "gpt-4.1-mini"
- `ai.base_url` (String): API 基礎 URL，支援自訂端點
- `ai.max_sample_length` (usize): 傳送到 AI 的內容長度上限，預設 3000
- `ai.temperature` (f32): 溫度參數，控制回應隨機性 (0.0-1.0)
- `ai.retry_attempts` (u32): API 請求失敗重試次數，預設 3
- `ai.retry_delay_ms` (u64): 重試間隔時間（毫秒），預設 1000

#### 格式配置（4 項）
- `formats.default_output` (String): 預設輸出格式，支援 "srt", "ass", "vtt", "webvtt"
- `formats.preserve_styling` (bool): 格式轉換時是否保留樣式
- `formats.default_encoding` (String): 預設檔案編碼，支援 "utf-8", "gbk", "big5", "shift_jis"
- `formats.encoding_detection_confidence` (f32): 編碼檢測信心度閾值 (0.0-1.0)

#### 同步配置（8 項）
- `sync.max_offset_seconds` (f32): 音訊字幕同步的最大偏移範圍，預設 10.0
- `sync.correlation_threshold` (f32): 音訊相關性分析閾值，預設 0.8
- `sync.dialogue_detection_threshold` (f32): 對話片段檢測的音訊能量敏感度，預設 0.6
- `sync.min_dialogue_duration_ms` (u32): 最小對話片段持續時間，預設 500
- `sync.dialogue_merge_gap_ms` (u32): 對話片段合併間隔，預設 200
- `sync.enable_dialogue_detection` (bool): 是否啟用對話檢測功能，預設 true
- `sync.audio_sample_rate` (u32): 音訊處理採樣率，預設 44100
- `sync.auto_detect_sample_rate` (bool): 自動檢測音訊採樣率，預設 true

#### 一般配置（5 項）
- `general.backup_enabled` (bool): 檔案匹配時是否自動備份，支援環境變數
- `general.max_concurrent_jobs` (usize): 並行任務調度器的最大並發數，預設 4
- `general.task_timeout_seconds` (u64): 任務執行逾時設定，預設 300
- `general.enable_progress_bar` (bool): 是否顯示進度條，預設 true
- `general.worker_idle_timeout_seconds` (u64): 工作執行緒閒置逾時，預設 60

#### 並行配置（5 項）
- `parallel.max_workers` (usize): 並行工作執行緒池的最大執行緒數量
- `parallel.task_queue_size` (usize): 任務佇列大小限制，預設 1000
- `parallel.enable_task_priorities` (bool): 啟用任務優先級排程，預設 false
- `parallel.auto_balance_workers` (bool): 自動平衡工作負載，預設 true
- `parallel.overflow_strategy` (OverflowStrategy): 任務佇列溢出策略，預設 Block

### 值類型和驗證規則

#### 字串類型驗證
```rust
// Provider 枚舉驗證
match key {
    "ai.provider" => validate_enum(&value, &["openai", "anthropic", "local"]),
    "formats.default_output" => validate_enum(&value, &["srt", "ass", "vtt", "webvtt"]),
    "formats.default_encoding" => validate_enum(&value, &["utf-8", "gbk", "big5", "shift_jis"]),
    "ai.api_key" => validate_api_key(&value),
    "ai.base_url" => validate_url(&value),
    _ => Ok(()),
}
```

#### 數值類型驗證
```rust
// 浮點數範圍驗證
match key {
    "ai.temperature" => validate_float_range(&value, 0.0, 1.0),
    "sync.max_offset_seconds" => validate_float_range(&value, 0.0, 300.0),
    "sync.correlation_threshold" => validate_float_range(&value, 0.0, 1.0),
    "sync.dialogue_detection_threshold" => validate_float_range(&value, 0.0, 1.0),
    "formats.encoding_detection_confidence" => validate_float_range(&value, 0.0, 1.0),
    _ => Ok(()),
}

// 整數範圍驗證
match key {
    "ai.max_sample_length" => validate_usize_range(&value, 100, 10000),
    "ai.retry_attempts" => validate_uint_range(&value, 1, 10),
    "ai.retry_delay_ms" => validate_u64_range(&value, 100, 30000),
    "sync.min_dialogue_duration_ms" => validate_uint_range(&value, 100, 5000),
    "sync.dialogue_merge_gap_ms" => validate_uint_range(&value, 50, 2000),
    "sync.audio_sample_rate" => validate_uint_range(&value, 8000, 192000),
    "general.max_concurrent_jobs" => validate_usize_range(&value, 1, 64),
    "general.task_timeout_seconds" => validate_u64_range(&value, 30, 3600),
    "general.worker_idle_timeout_seconds" => validate_u64_range(&value, 10, 3600),
    "parallel.max_workers" => validate_usize_range(&value, 1, 64),
    "parallel.task_queue_size" => validate_usize_range(&value, 100, 10000),
    _ => Ok(()),
}
```

#### 特殊類型驗證
```rust
// OverflowStrategy 枚舉驗證
match key {
    "parallel.overflow_strategy" => validate_enum(&value, &["Block", "Drop", "Expand"]),
    _ => Ok(()),
}
```

#### 布林值轉換
```rust
fn parse_bool(value: &str) -> Result<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "enabled" => Ok(true),
        "false" | "0" | "no" | "off" | "disabled" => Ok(false),
        _ => Err(SubXError::config(format!("Invalid boolean value: {}", value))),
    }
}
```

## 實作階段

### 階段 1：ConfigService Trait 擴展（預估時間：45 分鐘）

#### 1.1 添加 set_config_value 方法
在 `src/config/service.rs` 中擴展 `ConfigService` trait：

```rust
pub trait ConfigService: Send + Sync {
    // ...existing methods...
    
    /// Set a specific configuration value by key path.
    ///
    /// # Arguments
    ///
    /// - `key`: Dot-separated path to the configuration value
    /// - `value`: New value as string (will be converted to appropriate type)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the value was set successfully, or an error if:
    /// - The key is not recognized
    /// - The value cannot be converted to the expected type
    /// - The value fails validation
    /// - The configuration cannot be persisted
    ///
    /// # Errors
    ///
    /// Returns an error if validation or persistence fails.
    fn set_config_value(&self, key: &str, value: &str) -> Result<()>;
}
```

#### 1.2 實現 ProductionConfigService 的 set_config_value
```rust
impl ConfigService for ProductionConfigService {
    // ...existing methods...
    
    fn set_config_value(&self, key: &str, value: &str) -> Result<()> {
        // 1. Load current configuration
        let mut config = self.get_config()?;
        
        // 2. Validate and set the value
        self.validate_and_set_value(&mut config, key, value)?;
        
        // 3. Validate the entire configuration
        crate::config::validator::validate_config(&config)?;
        
        // 4. Save to file
        let path = self.get_config_file_path()?;
        self.save_config_to_file_with_config(&path, &config)?;
        
        // 5. Update cache
        {
            let mut cache = self.cached_config.write().unwrap();
            *cache = Some(config);
        }
        
        Ok(())
    }
}
```

### 階段 2：值驗證和設定系統（預估時間：90 分鐘）

#### 2.1 建立驗證函數模組
建立 `src/config/validation.rs`：

```rust
//! Configuration value validation utilities.
//!
//! This module provides comprehensive validation for configuration values,
//! ensuring type safety and constraint compliance.

use crate::error::{SubXError, SubXResult};

/// Validate a string value against a list of allowed values.
pub fn validate_enum(value: &str, allowed: &[&str]) -> SubXResult<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(SubXError::config(format!(
            "Invalid value '{}'. Allowed values: {}",
            value,
            allowed.join(", ")
        )))
    }
}

/// Validate a float value within a specified range.
pub fn validate_float_range(value: &str, min: f32, max: f32) -> SubXResult<f32> {
    let parsed = value.parse::<f32>().map_err(|_| {
        SubXError::config(format!("Invalid float value: {}", value))
    })?;
    
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {} is out of range [{}, {}]",
            parsed, min, max
        )));
    }
    
    Ok(parsed)
}

/// Validate an unsigned integer within a specified range.
pub fn validate_uint_range(value: &str, min: u32, max: u32) -> SubXResult<u32> {
    let parsed = value.parse::<u32>().map_err(|_| {
        SubXError::config(format!("Invalid integer value: {}", value))
    })?;
    
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {} is out of range [{}, {}]",
            parsed, min, max
        )));
    }
    
    Ok(parsed)
}

/// Validate a u64 value within a specified range.
pub fn validate_u64_range(value: &str, min: u64, max: u64) -> SubXResult<u64> {
    let parsed = value.parse::<u64>().map_err(|_| {
        SubXError::config(format!("Invalid u64 value: {}", value))
    })?;
    
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {} is out of range [{}, {}]",
            parsed, min, max
        )));
    }
    
    Ok(parsed)
}

/// Validate a usize value within a specified range.
pub fn validate_usize_range(value: &str, min: usize, max: usize) -> SubXResult<usize> {
    let parsed = value.parse::<usize>().map_err(|_| {
        SubXError::config(format!("Invalid usize value: {}", value))
    })?;
    
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {} is out of range [{}, {}]",
            parsed, min, max
        )));
    }
    
    Ok(parsed)
}

/// Validate API key format.
pub fn validate_api_key(value: &str) -> SubXResult<()> {
    if value.is_empty() {
        return Err(SubXError::config("API key cannot be empty".to_string()));
    }
    
    // Basic API key format validation
    if value.len() < 10 {
        return Err(SubXError::config("API key is too short".to_string()));
    }
    
    Ok(())
}

/// Validate URL format.
pub fn validate_url(value: &str) -> SubXResult<()> {
    if !value.starts_with("http://") && !value.starts_with("https://") {
        return Err(SubXError::config(format!(
            "Invalid URL format: {}. Must start with http:// or https://",
            value
        )));
    }
    
    Ok(())
}

/// Parse boolean value from string.
pub fn parse_bool(value: &str) -> SubXResult<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "enabled" => Ok(true),
        "false" | "0" | "no" | "off" | "disabled" => Ok(false),
        _ => Err(SubXError::config(format!("Invalid boolean value: {}", value))),
    }
}
```

#### 2.2 實現 validate_and_set_value 方法
在 `ProductionConfigService` 中添加：

```rust
impl ProductionConfigService {
    /// Validate and set a configuration value.
    fn validate_and_set_value(&self, config: &mut Config, key: &str, value: &str) -> Result<()> {
        use crate::config::validation::*;
        
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            // AI 配置（8 項）
            ["ai", "provider"] => {
                validate_enum(value, &["openai", "anthropic", "local"])?;
                config.ai.provider = value.to_string();
            }
            ["ai", "api_key"] => {
                if !value.is_empty() {
                    validate_api_key(value)?;
                    config.ai.api_key = Some(value.to_string());
                } else {
                    config.ai.api_key = None;
                }
            }
            ["ai", "model"] => {
                config.ai.model = value.to_string();
            }
            ["ai", "base_url"] => {
                validate_url(value)?;
                config.ai.base_url = value.to_string();
            }
            ["ai", "max_sample_length"] => {
                let length = validate_usize_range(value, 100, 10000)?;
                config.ai.max_sample_length = length;
            }
            ["ai", "temperature"] => {
                let temp = validate_float_range(value, 0.0, 1.0)?;
                config.ai.temperature = temp;
            }
            ["ai", "retry_attempts"] => {
                let attempts = validate_uint_range(value, 1, 10)?;
                config.ai.retry_attempts = attempts;
            }
            ["ai", "retry_delay_ms"] => {
                let delay = validate_u64_range(value, 100, 30000)?;
                config.ai.retry_delay_ms = delay;
            }
            
            // 格式配置（4 項）
            ["formats", "default_output"] => {
                validate_enum(value, &["srt", "ass", "vtt", "webvtt"])?;
                config.formats.default_output = value.to_string();
            }
            ["formats", "preserve_styling"] => {
                let preserve = parse_bool(value)?;
                config.formats.preserve_styling = preserve;
            }
            ["formats", "default_encoding"] => {
                validate_enum(value, &["utf-8", "gbk", "big5", "shift_jis"])?;
                config.formats.default_encoding = value.to_string();
            }
            ["formats", "encoding_detection_confidence"] => {
                let confidence = validate_float_range(value, 0.0, 1.0)?;
                config.formats.encoding_detection_confidence = confidence;
            }
            
            // 同步配置（8 項）
            ["sync", "max_offset_seconds"] => {
                let offset = validate_float_range(value, 0.0, 300.0)?;
                config.sync.max_offset_seconds = offset;
            }
            ["sync", "correlation_threshold"] => {
                let threshold = validate_float_range(value, 0.0, 1.0)?;
                config.sync.correlation_threshold = threshold;
            }
            ["sync", "dialogue_detection_threshold"] => {
                let threshold = validate_float_range(value, 0.0, 1.0)?;
                config.sync.dialogue_detection_threshold = threshold;
            }
            ["sync", "min_dialogue_duration_ms"] => {
                let duration = validate_uint_range(value, 100, 5000)?;
                config.sync.min_dialogue_duration_ms = duration;
            }
            ["sync", "dialogue_merge_gap_ms"] => {
                let gap = validate_uint_range(value, 50, 2000)?;
                config.sync.dialogue_merge_gap_ms = gap;
            }
            ["sync", "enable_dialogue_detection"] => {
                let enabled = parse_bool(value)?;
                config.sync.enable_dialogue_detection = enabled;
            }
            ["sync", "audio_sample_rate"] => {
                let rate = validate_uint_range(value, 8000, 192000)?;
                config.sync.audio_sample_rate = rate;
            }
            ["sync", "auto_detect_sample_rate"] => {
                let auto_detect = parse_bool(value)?;
                config.sync.auto_detect_sample_rate = auto_detect;
            }
            
            // 一般配置（5 項）
            ["general", "backup_enabled"] => {
                let enabled = parse_bool(value)?;
                config.general.backup_enabled = enabled;
            }
            ["general", "max_concurrent_jobs"] => {
                let jobs = validate_usize_range(value, 1, 64)?;
                config.general.max_concurrent_jobs = jobs;
            }
            ["general", "task_timeout_seconds"] => {
                let timeout = validate_u64_range(value, 30, 3600)?;
                config.general.task_timeout_seconds = timeout;
            }
            ["general", "enable_progress_bar"] => {
                let enabled = parse_bool(value)?;
                config.general.enable_progress_bar = enabled;
            }
            ["general", "worker_idle_timeout_seconds"] => {
                let timeout = validate_u64_range(value, 10, 3600)?;
                config.general.worker_idle_timeout_seconds = timeout;
            }
            
            // 並行配置（5 項）
            ["parallel", "max_workers"] => {
                let workers = validate_usize_range(value, 1, 64)?;
                config.parallel.max_workers = workers;
            }
            ["parallel", "task_queue_size"] => {
                let size = validate_usize_range(value, 100, 10000)?;
                config.parallel.task_queue_size = size;
            }
            ["parallel", "enable_task_priorities"] => {
                let enabled = parse_bool(value)?;
                config.parallel.enable_task_priorities = enabled;
            }
            ["parallel", "auto_balance_workers"] => {
                let enabled = parse_bool(value)?;
                config.parallel.auto_balance_workers = enabled;
            }
            ["parallel", "overflow_strategy"] => {
                validate_enum(value, &["Block", "Drop", "Expand"])?;
                // 需要實現 OverflowStrategy::from_str() 或類似的轉換
                config.parallel.overflow_strategy = match value {
                    "Block" => OverflowStrategy::Block,
                    "Drop" => OverflowStrategy::Drop,
                    "Expand" => OverflowStrategy::Expand,
                    _ => unreachable!(), // 已經通過 validate_enum 驗證
                };
            }
            
            // 未知配置項目
            _ => {
                return Err(SubXError::config(format!(
                    "Unknown configuration key: {}",
                    key
                )));
            }
        }
        
        Ok(())
    }
    
    /// Save configuration to file with specific config object.
    fn save_config_to_file_with_config(&self, path: &Path, config: &Config) -> Result<()> {
        let toml_content = toml::to_string_pretty(config)
            .map_err(|e| SubXError::config(format!("TOML serialization error: {}", e)))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SubXError::config(format!("Failed to create config directory: {}", e))
            })?;
        }

        std::fs::write(path, toml_content)
            .map_err(|e| SubXError::config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }
}
```

### 階段 3：更新 config_command.rs（預估時間：30 分鐘）

#### 3.1 實現 Set 操作
更新 `src/commands/config_command.rs` 中的 `execute` 函數：

```rust
pub async fn execute(args: ConfigArgs, config_service: &dyn ConfigService) -> SubXResult<()> {
    match args.action {
        ConfigAction::Set { key, value } => {
            config_service.set_config_value(&key, &value)?;
            println!("✓ Configuration '{}' set to '{}'", key, value);
            
            // Display the updated value to confirm
            match config_service.get_config_value(&key) {
                Ok(current_value) => {
                    println!("  Current value: {}", current_value);
                }
                Err(_) => {
                    // This shouldn't happen, but handle gracefully
                }
            }
            
            if let Ok(path) = config_service.get_config_file_path() {
                println!("  Saved to: {}", path.display());
            }
        }
        // ...existing Get, List, Reset cases...
    }
    Ok(())
}
```

#### 3.2 更新 execute_with_config 函數
```rust
pub async fn execute_with_config(
    args: ConfigArgs,
    config_service: std::sync::Arc<dyn ConfigService>,
) -> SubXResult<()> {
    match args.action {
        ConfigAction::Set { key, value } => {
            config_service.set_config_value(&key, &value)?;
            println!("✓ Configuration '{}' set to '{}'", key, value);
            
            // Display the updated value to confirm
            match config_service.get_config_value(&key) {
                Ok(current_value) => {
                    println!("  Current value: {}", current_value);
                }
                Err(_) => {
                    // This shouldn't happen, but handle gracefully
                }
            }
            
            if let Ok(path) = config_service.get_config_file_path() {
                println!("  Saved to: {}", path.display());
            }
        }
        // ...existing cases...
    }
    Ok(())
}
```

### 階段 4：TestConfigService 實現（預估時間：60 分鐘）

#### 4.1 擴展 TestConfigService
在 `src/config/test_service.rs` 中添加 `set_config_value` 實現：

```rust
impl ConfigService for TestConfigService {
    // ...existing methods...
    
    fn set_config_value(&self, key: &str, value: &str) -> Result<()> {
        // Load current config
        let mut config = self.get_config()?;
        
        // Use the same validation logic as ProductionConfigService
        self.validate_and_set_value(&mut config, key, value)?;
        
        // Validate the entire configuration
        crate::config::validator::validate_config(&config)?;
        
        // Update the internal config
        *self.config.lock().unwrap() = config;
        
        Ok(())
    }
}

impl TestConfigService {
    /// Validate and set a configuration value (same logic as ProductionConfigService).
    fn validate_and_set_value(&self, config: &mut Config, key: &str, value: &str) -> Result<()> {
        // Same implementation as ProductionConfigService::validate_and_set_value
        // This ensures consistency between test and production behavior
        
        use crate::config::validation::*;
        
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["ai", "provider"] => {
                validate_enum(value, &["openai", "anthropic", "local"])?;
                config.ai.provider = value.to_string();
            }
            // ...same validation logic as ProductionConfigService...
            _ => {
                return Err(SubXError::config(format!(
                    "Unknown configuration key: {}",
                    key
                )));
            }
        }
        
        Ok(())
    }
}
```

### 階段 5：全面測試實現（預估時間：120 分鐘）

#### 5.1 單元測試 - 驗證函數
建立 `tests/config_validation_tests.rs`：

```rust
//! Tests for configuration value validation functions.

use subx_cli::config::validation::*;
use subx_cli::error::SubXError;

#[test]
fn test_validate_enum_success() {
    let result = validate_enum("openai", &["openai", "anthropic", "local"]);
    assert!(result.is_ok());
}

#[test]
fn test_validate_enum_failure() {
    let result = validate_enum("invalid", &["openai", "anthropic", "local"]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid value 'invalid'"));
}

#[test]
fn test_validate_float_range_success() {
    let result = validate_float_range("0.5", 0.0, 1.0);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0.5);
}

#[test]
fn test_validate_float_range_out_of_bounds() {
    let result = validate_float_range("1.5", 0.0, 1.0);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of range"));
}

#[test]
fn test_validate_float_range_invalid_format() {
    let result = validate_float_range("not_a_number", 0.0, 1.0);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid float value"));
}

#[test]
fn test_validate_uint_range_success() {
    let result = validate_uint_range("32", 1, 64);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 32);
}

#[test]
fn test_validate_uint_range_out_of_bounds() {
    let result = validate_uint_range("128", 1, 64);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of range"));
}

#[test]
fn test_validate_api_key_success() {
    let result = validate_api_key("sk-1234567890abcdef");
    assert!(result.is_ok());
}

#[test]
fn test_validate_api_key_too_short() {
    let result = validate_api_key("short");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too short"));
}

#[test]
fn test_validate_api_key_empty() {
    let result = validate_api_key("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
}

#[test]
fn test_validate_url_success() {
    let result = validate_url("https://api.openai.com/v1");
    assert!(result.is_ok());
    
    let result = validate_url("http://localhost:8080");
    assert!(result.is_ok());
}

#[test]
fn test_validate_url_invalid_format() {
    let result = validate_url("ftp://example.com");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid URL format"));
}

#[test]
fn test_parse_bool_true_values() {
    let true_values = ["true", "1", "yes", "on", "enabled", "TRUE", "Yes", "ON"];
    for value in &true_values {
        let result = parse_bool(value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }
}

#[test]
fn test_parse_bool_false_values() {
    let false_values = ["false", "0", "no", "off", "disabled", "FALSE", "No", "OFF"];
    for value in &false_values {
        let result = parse_bool(value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }
}

#[test]
fn test_parse_bool_invalid_value() {
    let result = parse_bool("maybe");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid boolean value"));
}
```

#### 5.2 整合測試 - ConfigService Set 操作
建立 `tests/config_set_integration_tests.rs`：

```rust
//! Integration tests for config set operations.

use subx_cli::config::{Config, ConfigService, TestConfigService};
use subx_cli::config::test_macros::*;

#[test]
fn test_set_ai_provider_success() {
    test_with_config_service!(config_service, {
        // Set AI provider
        let result = config_service.set_config_value("ai.provider", "anthropic");
        assert!(result.is_ok());
        
        // Verify the value was set
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.provider, "anthropic");
        
        // Verify it can be retrieved
        let value = config_service.get_config_value("ai.provider").unwrap();
        assert_eq!(value, "anthropic");
    });
}

#[test]
fn test_set_ai_provider_invalid_value() {
    test_with_config_service!(config_service, {
        let result = config_service.set_config_value("ai.provider", "invalid_provider");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid value"));
    });
}

#[test]
fn test_set_ai_temperature_success() {
    test_with_config_service!(config_service, {
        let result = config_service.set_config_value("ai.temperature", "0.7");
        assert!(result.is_ok());
        
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.temperature, 0.7);
    });
}

#[test]
fn test_set_ai_temperature_out_of_range() {
    test_with_config_service!(config_service, {
        let result = config_service.set_config_value("ai.temperature", "1.5");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of range"));
    });
}

#[test]
fn test_set_ai_api_key_success() {
    test_with_config_service!(config_service, {
        let result = config_service.set_config_value("ai.api_key", "sk-1234567890abcdef");
        assert!(result.is_ok());
        
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.api_key, Some("sk-1234567890abcdef".to_string()));
    });
}

#[test]
fn test_set_ai_api_key_empty_clears_value() {
    test_with_config_service!(config_service, {
        // First set a value
        config_service.set_config_value("ai.api_key", "sk-1234567890abcdef").unwrap();
        
        // Then clear it
        let result = config_service.set_config_value("ai.api_key", "");
        assert!(result.is_ok());
        
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.api_key, None);
    });
}

#[test]
fn test_set_boolean_values() {
    test_with_config_service!(config_service, {
        // Test various boolean representations
        let boolean_tests = vec![
            ("true", true),
            ("false", false),
            ("1", true),
            ("0", false),
            ("yes", true),
            ("no", false),
            ("on", true),
            ("off", false),
            ("enabled", true),
            ("disabled", false),
        ];
        
        for (input, expected) in boolean_tests {
            let result = config_service.set_config_value("general.backup_enabled", input);
            assert!(result.is_ok(), "Failed to set boolean value: {}", input);
            
            let config = config_service.get_config().unwrap();
            assert_eq!(config.general.backup_enabled, expected, "Wrong boolean value for input: {}", input);
        }
    });
}

#[test]
fn test_set_integer_values() {
    test_with_config_service!(config_service, {
        let result = config_service.set_config_value("general.max_concurrent_jobs", "8");
        assert!(result.is_ok());
        
        let config = config_service.get_config().unwrap();
        assert_eq!(config.general.max_concurrent_jobs, 8);
    });
}

#[test]
fn test_set_unknown_key() {
    test_with_config_service!(config_service, {
        let result = config_service.set_config_value("unknown.key", "value");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown configuration key"));
    });
}

#[test]
fn test_set_preserves_other_values() {
    test_with_config_service!(config_service, {
        // Set initial values
        config_service.set_config_value("ai.provider", "openai").unwrap();
        config_service.set_config_value("ai.temperature", "0.3").unwrap();
        
        // Change one value
        config_service.set_config_value("ai.provider", "anthropic").unwrap();
        
        // Verify the other value is preserved
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.provider, "anthropic");
        assert_eq!(config.ai.temperature, 0.3);
    });
}

#[test]
fn test_set_validates_entire_config() {
    test_with_config_service!(config_service, {
        // This test ensures that setting a value doesn't break configuration validation
        let result = config_service.set_config_value("ai.provider", "openai");
        assert!(result.is_ok());
        
        // The configuration should still be valid after the change
        let config = config_service.get_config().unwrap();
        assert!(crate::config::validator::validate_config(&config).is_ok());
    });
}
```

#### 5.3 Command 整合測試
建立 `tests/config_command_set_tests.rs`：

```rust
//! Integration tests for the config command set functionality.

use subx_cli::cli::{ConfigArgs, ConfigAction};
use subx_cli::commands::config_command;
use subx_cli::config::{ConfigService, TestConfigService};
use subx_cli::config::test_macros::*;
use std::sync::Arc;

#[tokio::test]
async fn test_config_command_set_success() {
    test_with_config_service!(config_service, {
        let args = ConfigArgs {
            action: ConfigAction::Set {
                key: "ai.provider".to_string(),
                value: "anthropic".to_string(),
            },
        };
        
        let result = config_command::execute(args, config_service.as_ref()).await;
        assert!(result.is_ok());
        
        // Verify the value was set
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.provider, "anthropic");
    });
}

#[tokio::test]
async fn test_config_command_set_invalid_key() {
    test_with_config_service!(config_service, {
        let args = ConfigArgs {
            action: ConfigAction::Set {
                key: "invalid.key".to_string(),
                value: "value".to_string(),
            },
        };
        
        let result = config_command::execute(args, config_service.as_ref()).await;
        assert!(result.is_err());
    });
}

#[tokio::test]
async fn test_config_command_set_invalid_value() {
    test_with_config_service!(config_service, {
        let args = ConfigArgs {
            action: ConfigAction::Set {
                key: "ai.temperature".to_string(),
                value: "invalid_number".to_string(),
            },
        };
        
        let result = config_command::execute(args, config_service.as_ref()).await;
        assert!(result.is_err());
    });
}

#[tokio::test]
async fn test_config_command_set_with_arc() {
    test_with_config_service!(config_service, {
        let args = ConfigArgs {
            action: ConfigAction::Set {
                key: "ai.model".to_string(),
                value: "gpt-4".to_string(),
            },
        };
        
        let result = config_command::execute_with_config(args, config_service.clone()).await;
        assert!(result.is_ok());
        
        // Verify the value was set
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.model, "gpt-4");
    });
}

#[tokio::test]
async fn test_config_command_set_multiple_values() {
    test_with_config_service!(config_service, {
        // Set multiple values
        let settings = vec![
            ("ai.provider", "openai"),
            ("ai.temperature", "0.7"),
            ("general.backup_enabled", "true"),
            ("parallel.max_workers", "8"),
        ];
        
        for (key, value) in settings {
            let args = ConfigArgs {
                action: ConfigAction::Set {
                    key: key.to_string(),
                    value: value.to_string(),
                },
            };
            
            let result = config_command::execute(args, config_service.as_ref()).await;
            assert!(result.is_ok(), "Failed to set {}: {}", key, value);
        }
        
        // Verify all values were set correctly
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.temperature, 0.7);
        assert_eq!(config.general.backup_enabled, true);
        assert_eq!(config.parallel.max_workers, 8);
    });
}
```

### 階段 6：文件更新和範例（預估時間：45 分鐘）

#### 6.1 更新 CLI 文件
在 `src/cli/config_args.rs` 中添加更多範例：

```rust
/// # Advanced Usage Examples
///
/// ## Setting Complex Values
/// ```bash
/// # Set AI provider with API key
/// subx-cli config set ai.provider openai
/// subx-cli config set ai.api_key "sk-1234567890abcdef"
/// subx-cli config set ai.base_url "https://api.openai.com/v1"
/// 
/// # Configure audio processing
/// subx-cli config set sync.max_offset_seconds 15.0
/// subx-cli config set sync.correlation_threshold 0.85
/// 
/// # Set performance options
/// subx-cli config set parallel.max_workers 4
/// subx-cli config set general.max_concurrent_jobs 8
/// ```
///
/// ## Boolean Value Formats
/// ```bash
/// # All of these set the value to true
/// subx-cli config set general.backup_enabled true
/// subx-cli config set general.backup_enabled 1
/// subx-cli config set general.backup_enabled yes
/// subx-cli config set general.backup_enabled on
/// subx-cli config set general.backup_enabled enabled
/// 
/// # All of these set the value to false
/// subx-cli config set general.backup_enabled false
/// subx-cli config set general.backup_enabled 0
/// subx-cli config set general.backup_enabled no
/// subx-cli config set general.backup_enabled off
/// subx-cli config set general.backup_enabled disabled
/// ```
///
/// ## Clearing Optional Values
/// ```bash
/// # Clear API key (set to None)
/// subx-cli config set ai.api_key ""
/// ```
```

#### 6.2 建立設定指南文件
建立 `docs/configuration-guide.md`：

```markdown
# SubX Configuration Guide

## Setting Configuration Values

The `subx-cli config set` command allows you to modify configuration settings with comprehensive validation and type safety.

### Basic Syntax

```bash
subx-cli config set <key> <value>
```

### AI Configuration

Configure AI service providers and their settings:

```bash
# Set AI provider
subx-cli config set ai.provider openai

# Set API key
subx-cli config set ai.api_key "your-api-key-here"

# Set model
subx-cli config set ai.model "gpt-4.1-mini"

# Set temperature (creativity level)
subx-cli config set ai.temperature 0.7

# Set custom API endpoint
subx-cli config set ai.base_url "https://api.openai.com/v1"
```

### Format Configuration

Control subtitle format handling:

```bash
# Set default output format
subx-cli config set formats.default_output srt

# Set default encoding
subx-cli config set formats.default_encoding utf-8

# Enable/disable style preservation
subx-cli config set formats.preserve_styling true
```

### Synchronization Settings

Configure audio-subtitle synchronization:

```bash
# Set maximum sync offset
subx-cli config set sync.max_offset_seconds 10.0

# Set correlation threshold
subx-cli config set sync.correlation_threshold 0.8

# Set audio sample rate
subx-cli config set sync.audio_sample_rate 44100
```

### General Options

Control general application behavior:

```bash
# Enable/disable backup creation
subx-cli config set general.backup_enabled true

# Set maximum concurrent jobs
subx-cli config set general.max_concurrent_jobs 4
```

### Performance Settings

Configure parallel processing:

```bash
# Set maximum worker threads
subx-cli config set parallel.max_workers 8
```

## Value Types and Validation

### String Values

Certain string values are validated against allowed options:

- `ai.provider`: Must be one of `openai`, `anthropic`, `local`
- `formats.default_output`: Must be one of `srt`, `ass`, `vtt`, `webvtt`
- `formats.default_encoding`: Must be one of `utf-8`, `gbk`, `big5`, `shift_jis`

### Numeric Values

Numeric values are validated for range and type:

- `ai.temperature`: Float between 0.0 and 1.0
- `sync.max_offset_seconds`: Float between 0.0 and 300.0
- `sync.correlation_threshold`: Float between 0.0 and 1.0
- `general.max_concurrent_jobs`: Integer between 1 and 64
- `parallel.max_workers`: Integer between 1 and 64
- `sync.audio_sample_rate`: Integer between 8000 and 192000

### Boolean Values

Boolean values accept multiple formats:

- **True**: `true`, `1`, `yes`, `on`, `enabled`
- **False**: `false`, `0`, `no`, `off`, `disabled`

### Special Values

- **Empty String**: Use `""` to clear optional string values
- **API Keys**: Must be at least 10 characters long
- **URLs**: Must start with `http://` or `https://`

## Error Handling

The system provides detailed error messages for various validation failures:

- **Invalid Key**: The configuration key doesn't exist
- **Type Mismatch**: The value cannot be converted to the expected type
- **Range Error**: Numeric values are outside acceptable ranges
- **Format Error**: String values don't match required patterns
- **Validation Error**: The value fails specific validation rules

## Examples

### Complete AI Setup

```bash
# Configure OpenAI
subx-cli config set ai.provider openai
subx-cli config set ai.api_key "sk-1234567890abcdef"
subx-cli config set ai.model "gpt-4.1-mini"
subx-cli config set ai.temperature 0.3

# Configure Anthropic
subx-cli config set ai.provider anthropic
subx-cli config set ai.api_key "anthropic-key-here"
subx-cli config set ai.model "claude-3-haiku"
```

### Performance Optimization

```bash
# Optimize for faster processing
subx-cli config set parallel.max_workers 8
subx-cli config set general.max_concurrent_jobs 4

# Optimize for accuracy
subx-cli config set sync.correlation_threshold 0.9
subx-cli config set ai.temperature 0.1
```

### Viewing Changes

After setting values, you can verify the changes:

```bash
# View specific setting
subx-cli config get ai.provider

# View all settings
subx-cli config list
```
```

### 階段 7：程式碼品質檢查（預估時間：30 分鐘）

#### 7.1 執行程式碼格式化和檢查
```bash
# Format code
cargo fmt

# Check for warnings
cargo clippy -- -D warnings

# Run tests
cargo test

# Check documentation
timeout 30 scripts/check_docs.sh

# Check test coverage
scripts/check_coverage.sh -T
```

#### 7.2 修復任何發現的問題
根據 clippy 和測試結果修復程式碼品質問題。

## 驗收標準

### 功能需求
- [ ] `subx config set` 指令可以成功設定所有支援的配置項目
- [ ] 值驗證能正確捕獲和報告錯誤
- [ ] 設定的值能正確持久化到配置檔案
- [ ] 配置變更能正確反映到後續的指令執行中

### 品質需求
- [ ] 所有新增的程式碼都有完整的測試覆蓋
- [ ] 所有測試都通過並且能並行執行
- [ ] 程式碼符合 clippy 標準，沒有警告
- [ ] 文件完整且格式正確

### 安全需求
- [ ] 敏感資料（如 API 金鑰）在錯誤訊息中不會洩漏
- [ ] 配置檔案具有適當的權限設定
- [ ] 測試不會修改全域狀態或影響其他測試

### 使用體驗需求
- [ ] 錯誤訊息清晰且提供建設性的解決方案
- [ ] 成功操作有明確的確認訊息
- [ ] 支援所有合理的值格式（如布林值的多種表示法）

## 重要注意事項

### ConfigService get_config_value 方法擴展需求

根據配置使用分析，目前 `ProductionConfigService::get_config_value()` 方法只支援有限的配置鍵（16 項），但實際程式碼中使用了 30 項配置。本 backlog 實現後，需要同時擴展 `get_config_value` 方法以支援所有新增的配置項目：

**當前 get_config_value 缺少支援的配置項目**：
- AI: `max_sample_length`, `retry_attempts`, `retry_delay_ms`
- 格式: `encoding_detection_confidence`
- 同步: `dialogue_detection_threshold`, `min_dialogue_duration_ms`, `dialogue_merge_gap_ms`, `enable_dialogue_detection`, `auto_detect_sample_rate`
- 一般: `task_timeout_seconds`, `enable_progress_bar`, `worker_idle_timeout_seconds`
- 並行: `task_queue_size`, `enable_task_priorities`, `auto_balance_workers`, `overflow_strategy`

### 實現一致性要求

為確保 `config set` 和 `config get` 功能的一致性，此 backlog 必須：

1. **擴展 get_config_value 方法**：在 `ProductionConfigService` 和 `TestConfigService` 中添加對所有 30 項配置的支援
2. **更新驗證測試**：確保所有可以 `set` 的配置項目都可以透過 `get` 檢索
3. **保持方法同步**：`validate_and_set_value` 和 `get_config_value` 必須支援相同的配置鍵集合

### 階段調整

基於此需求，**階段 1** 的時間預估需要從 45 分鐘調整為 **60 分鐘**，以包含 `get_config_value` 方法的擴展工作。

## 風險和緩解措施

### 風險 1：配置檔案格式破壞
**描述**：錯誤的值可能導致 TOML 檔案格式損壞
**緩解措施**：
- 在寫入前驗證完整配置的序列化
- 建立自動備份機制
- 提供配置修復指令

### 風險 2：並行存取衝突
**描述**：多個進程同時修改配置檔案可能導致資料丟失
**緩解措施**：
- 使用檔案鎖定機制
- 實現原子性寫入操作
- 提供衝突檢測和解決機制

### 風險 3：驗證邏輯不一致
**描述**：生產環境和測試環境的驗證邏輯可能不同步
**緩解措施**：
- 共享驗證函數
- 全面的整合測試
- 明確的程式碼審查檢查清單

## 後續改進

### 版本 1.1 增強功能
- 添加配置範本和預設設定組合
- 實現配置匯入/匯出功能
- 添加配置值的歷史記錄和回滾功能

### 版本 1.2 增強功能
- 添加配置值的互動式編輯
- 實現配置檔案的即時重載
- 添加配置變更的通知機制

## 估算總時間

| 階段 | 預估時間 | 說明 |
|------|----------|------|
| ConfigService 擴展 | 60 分鐘 | Trait、基本實現和 get_config_value 擴展 |
| 驗證系統 | 120 分鐘 | 30項配置的驗證函數和設定邏輯 |
| Command 更新 | 30 分鐘 | 更新指令處理函數 |
| TestConfigService | 60 分鐘 | 測試實現 |
| 測試撰寫 | 150 分鐘 | 30項配置的全面測試覆蓋 |
| 文件更新 | 45 分鐘 | 使用指南和範例 |
| 品質檢查 | 30 分鐘 | 格式化和修復 |
| **總計** | **495 分鐘** | **約 8.25 小時** |

## 結論

此 backlog 將完整實現 SubX 的 `config set` 指令功能，支援所有 **30 項實際使用的配置項目**，提供安全、可靠且使用者友善的配置管理體驗。通過階段性的實現和全面的測試，確保功能的穩定性和可維護性。實現完成後，用戶將能夠通過 CLI 直接管理所有 SubX 配置項目，而無需手動編輯配置檔案。

### 配置項目支援範圍
- **AI 配置**：8 項（provider, api_key, model, base_url, max_sample_length, temperature, retry_attempts, retry_delay_ms）
- **格式配置**：4 項（default_output, preserve_styling, default_encoding, encoding_detection_confidence）
- **同步配置**：8 項（包含完整的對話檢測和音訊處理配置）
- **一般配置**：5 項（包含備份、並行調度、逾時設定）
- **並行配置**：5 項（包含高級並行處理配置）

此實現將提供與現有程式碼完全一致的配置管理功能，確保所有實際使用的配置項目都能通過 `subx-cli config set` 指令進行設定。
