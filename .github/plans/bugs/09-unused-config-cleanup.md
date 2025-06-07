# Bug Report #09: 清理未使用的配置項目

## 問題描述

在配置檔案使用情況分析中發現，有多個配置項目在程式碼中完全未被使用，或對應的功能尚未實作。這些冗餘配置會造成：

1. 使用者誤解：認為設定這些配置會影響程式行為
2. 維護負擔：需要維護無用的程式碼和文件
3. 配置複雜度：增加配置檔案的複雜性

## 受影響的配置項目

### 完全未使用的配置項目 (需要移除)

#### 1. `formats.default_encoding`
- **狀態**: 完全未使用
- **原因**: 未在任何檔案讀取或轉換邏輯中使用
- **影響**: 使用者設定此項目無任何效果

#### 2. `sync.audio_sample_rate`
- **狀態**: 完全未使用
- **原因**: AudioAnalyzer 硬編碼為 16000
- **影響**: 配置值被忽略

#### 3. `sync.dialogue_detection_threshold`
- **狀態**: 完全未使用
- **原因**: 對話檢測功能未實作
- **影響**: 無對應功能支援

#### 4. `sync.min_dialogue_duration_ms`
- **狀態**: 完全未使用
- **原因**: 對話檢測功能未實作
- **影響**: 無對應功能支援

#### 5. `general.default_confidence`
- **狀態**: 完全未使用
- **原因**: CLI 參數有預設值但未連結配置
- **影響**: 與命令列參數重複

#### 6. `general.max_concurrent_jobs`
- **狀態**: 完全未使用
- **原因**: 無平行處理邏輯使用此設定
- **影響**: 平行處理功能未實作

#### 7. `general.log_level`
- **狀態**: 完全未使用
- **原因**: 使用 env_logger，從 RUST_LOG 環境變數讀取
- **影響**: 與環境變數機制重複

## 清理計劃

### 階段 1: 程式碼清理 (預估工時: 3 小時)

#### 1.1 移除配置結構中的無用欄位
```rust
// 修改 src/config.rs

// 移除 FormatsConfig 中的 default_encoding
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormatsConfig {
    pub default_output: String,
    pub preserve_styling: bool,
    // 移除: pub default_encoding: String,
}

// 移除 SyncConfig 中的對話檢測相關配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    pub max_offset_seconds: f32,
    // 移除: pub audio_sample_rate: u32,
    pub correlation_threshold: f32,
    // 移除: pub dialogue_detection_threshold: f32,
    // 移除: pub min_dialogue_duration_ms: u64,
}

// 移除 GeneralConfig 中的無用配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub backup_enabled: bool,
    // 移除: pub default_confidence: u8,
    // 移除: pub max_concurrent_jobs: usize,
    // 移除: pub log_level: String,
}
```

#### 1.2 更新預設配置實作
```rust
// 修改 src/config.rs 中的 Default 實作
impl Default for FormatsConfig {
    fn default() -> Self {
        Self {
            default_output: "srt".to_string(),
            preserve_styling: true,
            // 移除 default_encoding 預設值
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_offset_seconds: 30.0,
            correlation_threshold: 0.7,
            // 移除對話檢測相關預設值
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            backup_enabled: false,
            // 移除其他無用預設值
        }
    }
}
```

#### 1.3 移除相關的 config 命令處理
```rust
// 修改 src/commands/config_command.rs
// 移除對無用配置項目的 get/set 處理

pub fn handle_config_command(args: &ConfigArgs) -> crate::error::Result<()> {
    match &args.action {
        ConfigAction::Set { key, value } => {
            match key.as_str() {
                // 保留有用的配置
                "ai.model" => { /* ... */ },
                "formats.default_output" => { /* ... */ },
                "formats.preserve_styling" => { /* ... */ },
                "sync.max_offset_seconds" => { /* ... */ },
                "sync.correlation_threshold" => { /* ... */ },
                "general.backup_enabled" => { /* ... */ },
                
                // 移除無用配置的處理
                // "formats.default_encoding" => { /* 移除 */ },
                // "sync.audio_sample_rate" => { /* 移除 */ },
                // "sync.dialogue_detection_threshold" => { /* 移除 */ },
                // "sync.min_dialogue_duration_ms" => { /* 移除 */ },
                // "general.default_confidence" => { /* 移除 */ },
                // "general.max_concurrent_jobs" => { /* 移除 */ },
                // "general.log_level" => { /* 移除 */ },
                
                _ => return Err(ConfigError::InvalidKey(key.clone()).into()),
            }
        }
        // ...
    }
}
```

### 階段 2: 文件更新 (預估工時: 2 小時)

#### 2.1 更新配置檔案模板
```toml
# 更新 config.toml 模板，移除無用配置項目

[ai]
provider = "openai"
# api_key = "your-openai-api-key"  # 或使用環境變數 OPENAI_API_KEY
model = "gpt-4o-mini"
max_sample_length = 2000
temperature = 0.3
retry_attempts = 3
retry_delay_ms = 1000

[formats]
default_output = "srt"
preserve_styling = true
# 移除: default_encoding = "utf-8"

[sync]
max_offset_seconds = 30.0
correlation_threshold = 0.7
# 移除: audio_sample_rate = 16000
# 移除: dialogue_detection_threshold = 0.01
# 移除: min_dialogue_duration_ms = 500

[general]
backup_enabled = false
# 移除: default_confidence = 80
# 移除: max_concurrent_jobs = 4
# 移除: log_level = "info"
```

#### 2.2 更新說明文件
```markdown
# 更新 README.md 和配置說明文件
# 移除對無用配置項目的說明
# 簡化配置項目清單
```

### 階段 3: 測試更新 (預估工時: 2 小時)

#### 3.1 移除相關測試
```rust
// 移除 tests/ 中對無用配置的測試
// 確保移除配置後現有測試仍然通過
```

#### 3.2 加入向後相容性測試
```rust
#[cfg(test)]
mod backward_compatibility_tests {
    use super::*;

    #[test]
    fn test_old_config_file_still_works() {
        // 測試包含舊配置項目的檔案仍能正常載入
        // 舊配置項目應被忽略而不報錯
        let config_content = r#"
[ai]
model = "gpt-4"

[formats]
default_output = "srt"
default_encoding = "utf-8"  # 舊配置項目

[sync]
max_offset_seconds = 30.0
dialogue_detection_threshold = 0.01  # 舊配置項目

[general]
backup_enabled = true
log_level = "debug"  # 舊配置項目
"#;

        // 確保能成功解析且不報錯
        let config: Result<Config, _> = toml::from_str(config_content);
        assert!(config.is_ok(), "舊配置檔案應該能正常載入");
    }
}
```

### 階段 4: 向後相容性處理 (預估工時: 1 小時)

#### 4.1 實作配置遷移提示
```rust
// 在 src/config.rs 中加入向後相容性處理
impl Config {
    pub fn load_with_migration_warning(path: &Path) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        
        // 檢查是否包含已移除的配置項目
        let deprecated_keys = [
            "formats.default_encoding",
            "sync.audio_sample_rate",
            "sync.dialogue_detection_threshold",
            "sync.min_dialogue_duration_ms",
            "general.default_confidence",
            "general.max_concurrent_jobs",
            "general.log_level",
        ];
        
        for key in &deprecated_keys {
            if content.contains(key) {
                eprintln!(
                    "⚠️  警告: 配置項目 '{}' 已被移除，將被忽略", 
                    key
                );
            }
        }
        
        // 使用寬鬆解析，忽略未知欄位
        let config: Config = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        
        Ok(config)
    }
}
```

## 特殊情況處理

### audio_sample_rate 的特殊處理
由於 AudioAnalyzer 硬編碼為 16000，需要決定：
1. **選項 A**: 移除配置，保持硬編碼
2. **選項 B**: 實作配置支援 (移至 Backlog)

**建議選擇選項 A**，因為：
- 音訊分析通常有最佳採樣率
- 避免使用者錯誤配置導致分析失敗
- 簡化配置檔案

### dialogue_detection 相關配置
這些配置對應未實作的功能，應該：
1. 移除配置項目
2. 在未來實作對話檢測功能時重新加入

## 測試計劃

### 單元測試
```rust
#[cfg(test)]
mod config_cleanup_tests {
    use super::*;

    #[test]
    fn test_config_serialization_without_removed_fields() {
        let config = Config::default();
        let serialized = toml::to_string(&config).unwrap();
        
        // 確保序列化後不包含已移除的欄位
        assert!(!serialized.contains("default_encoding"));
        assert!(!serialized.contains("audio_sample_rate"));
        assert!(!serialized.contains("dialogue_detection_threshold"));
        assert!(!serialized.contains("min_dialogue_duration_ms"));
        assert!(!serialized.contains("default_confidence"));
        assert!(!serialized.contains("max_concurrent_jobs"));
        assert!(!serialized.contains("log_level"));
    }

    #[test]
    fn test_config_deserialization_ignores_unknown_fields() {
        // 測試包含未知欄位的配置能正常解析
        let config_with_unknown = r#"
[ai]
model = "gpt-4"
unknown_field = "should be ignored"

[formats]
default_output = "srt"
"#;
        
        let result: Result<Config, _> = toml::from_str(config_with_unknown);
        assert!(result.is_ok());
    }
}
```

### 整合測試
```bash
# 測試移除配置後的命令功能
cargo test --test integration_tests

# 測試配置命令不接受已移除的配置項目
subx-cli config set formats.default_encoding utf-8  # 應該報錯
subx-cli config set general.log_level debug  # 應該報錯
```

## 驗收標準

### 功能驗收
- [ ] 移除的配置項目不再出現在程式碼中
- [ ] 舊配置檔案仍能正常載入（向後相容）
- [ ] config 命令不再接受已移除的配置項目
- [ ] 所有現有功能正常運作

### 程式碼品質驗收
- [ ] 通過所有現有測試
- [ ] 新增的測試涵蓋向後相容性
- [ ] 程式碼通過 `cargo clippy` 和 `cargo fmt` 檢查
- [ ] 移除了所有相關的死程式碼

### 文件品質驗收
- [ ] 配置檔案模板已更新
- [ ] README 和說明文件已更新
- [ ] 移除了對無用配置的說明

## 風險評估

### 向後相容性風險
- **中等風險**: 使用者可能依賴舊配置項目
- **緩解措施**: 實作遷移警告和寬鬆解析

### 功能回歸風險
- **低風險**: 移除未使用功能不影響現有邏輯
- **緩解措施**: 完整的回歸測試

### 使用者體驗風險
- **低風險**: 移除無效配置改善使用者體驗
- **緩解措施**: 清楚的遷移指南和警告訊息

## 實作順序

1. **程式碼清理** (最高優先級)
2. **測試更新和向後相容性** (高優先級)
3. **文件更新** (中等優先級)

## 完成後效益

- **簡化配置**: 移除 7 個無用配置項目
- **減少困惑**: 使用者不會被無效配置誤導
- **降低維護成本**: 減少需要維護的程式碼量
- **提升程式碼品質**: 移除死程式碼和無用依賴
