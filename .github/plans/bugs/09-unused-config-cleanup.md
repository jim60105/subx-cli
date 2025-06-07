# Bug Report #09: 清理未使用的配置項目

## 問題描述

在配置檔案使用情況分析中發現，有多個配置項目在程式碼中完全未被使用，或對應的功能尚未實作。這些冗餘配置會造成：

1. 使用者誤解：認為設定這些配置會影響程式行為
2. 維護負擔：需要維護無用的程式碼和文件
3. 配置複雜度：增加配置檔案的複雜性

## 受影響的配置項目

### 計劃保留的配置項目 (未來實作功能)

> **重要**: 以下配置項目雖然目前未使用，但在 Backlog #14 中計劃實作對應功能，因此應予以保留：

#### 保留項目清單
1. `formats.default_encoding` - 計劃實作檔案編碼自動檢測功能
2. `sync.audio_sample_rate` - 計劃實作音訊採樣率動態配置功能
3. `sync.dialogue_detection_threshold` - 計劃實作對話檢測功能
4. `sync.min_dialogue_duration_ms` - 計劃實作對話檢測功能
5. `general.max_concurrent_jobs` - 計劃實作平行處理系統

### 確認需要移除的配置項目

#### 1. `general.default_confidence`
- **狀態**: 完全未使用且無實作計劃
- **原因**: CLI 參數有預設值但未連結配置
- **影響**: 與命令列參數重複
- **決定**: 移除

#### 2. `general.log_level`
- **狀態**: 完全未使用且無實作計劃
- **原因**: 使用 env_logger，從 RUST_LOG 環境變數讀取
- **影響**: 與環境變數機制重複
- **決定**: 移除

## 清理計劃

### 階段 1: 程式碼清理 (預估工時: 3 小時)

#### 1.1 移除配置結構中的無用欄位
```rust
// 修改 src/config.rs

// 移除 GeneralConfig 中的無用配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub backup_enabled: bool,
    // 保留計劃實作的配置項目:
    pub max_concurrent_jobs: usize,  // 保留 - Backlog #14
    // 移除: pub default_confidence: u8,
    // 移除: pub log_level: String,
}

// FormatsConfig 和 SyncConfig 維持不變，因為它們的所有項目都計劃實作
```

#### 1.2 更新預設配置實作
```rust
// 修改 src/config.rs 中的 Default 實作

// FormatsConfig 和 SyncConfig 維持現有預設值，因為所有項目都計劃實作

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            backup_enabled: false,
            max_concurrent_jobs: 4,  // 保留 - Backlog #14
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
                "formats.default_encoding" => { /* ... */ },  // 保留 - Backlog #14
                "sync.max_offset_seconds" => { /* ... */ },
                "sync.correlation_threshold" => { /* ... */ },
                "sync.audio_sample_rate" => { /* ... */ },  // 保留 - Backlog #14
                "sync.dialogue_detection_threshold" => { /* ... */ },  // 保留 - Backlog #14
                "sync.min_dialogue_duration_ms" => { /* ... */ },  // 保留 - Backlog #14
                "general.backup_enabled" => { /* ... */ },
                "general.max_concurrent_jobs" => { /* ... */ },  // 保留 - Backlog #14
                
                // 移除無用配置的處理
                // "general.default_confidence" => { /* 移除 */ },
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
# 更新 config.toml 模板，僅移除確認無用的配置項目

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
default_encoding = "utf-8"  # 保留 - Backlog #14 計劃實作

[sync]
max_offset_seconds = 30.0
correlation_threshold = 0.7
audio_sample_rate = 16000  # 保留 - Backlog #14 計劃實作
dialogue_detection_threshold = 0.01  # 保留 - Backlog #14 計劃實作
min_dialogue_duration_ms = 500  # 保留 - Backlog #14 計劃實作

[general]
backup_enabled = false
max_concurrent_jobs = 4  # 保留 - Backlog #14 計劃實作
# 移除: default_confidence = 80
# 移除: log_level = "info"
```

#### 2.2 更新說明文件
```markdown
# 更新 README.md 和配置說明文件
# 僅移除對確認無用配置項目的說明 (default_confidence, log_level)
# 為計劃實作的配置項目添加 "功能開發中" 標記
```

### 階段 3: 測試更新 (預估工時: 2 小時)

#### 3.1 移除相關測試
```rust
// 移除 tests/ 中對已確認無用配置的測試 (default_confidence, log_level)
// 確保移除配置後現有測試仍然通過
// 保留對計劃實作配置項目的測試或添加待實作標記
```

#### 3.2 加入向後相容性測試
```rust
#[cfg(test)]
mod backward_compatibility_tests {
    use super::*;

    #[test]
    fn test_old_config_file_still_works() {
        // 測試包含舊配置項目的檔案仍能正常載入
        // 已移除的配置項目應被忽略而不報錯
        let config_content = r#"
[ai]
model = "gpt-4"

[formats]
default_output = "srt"
default_encoding = "utf-8"  # 計劃實作的配置項目，應正常載入

[sync]
max_offset_seconds = 30.0
dialogue_detection_threshold = 0.01  # 計劃實作的配置項目，應正常載入

[general]
backup_enabled = true
default_confidence = 80  # 已移除的配置項目，應被忽略
log_level = "debug"  # 已移除的配置項目，應被忽略
max_concurrent_jobs = 4  # 計劃實作的配置項目，應正常載入
"#;

        // 確保能成功解析且不報錯
        let config: Result<Config, _> = toml::from_str(config_content);
        assert!(config.is_ok(), "包含舊配置項目的檔案應該能正常載入");
        
        let config = config.unwrap();
        // 驗證計劃實作的配置項目被正確載入
        assert_eq!(config.general.max_concurrent_jobs, 4);
        assert_eq!(config.formats.default_encoding, "utf-8");
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
        
        // 檢查是否包含已確認移除的配置項目
        let removed_keys = [
            "general.default_confidence",
            "general.log_level",
        ];
        
        // 檢查是否包含計劃實作的配置項目
        let planned_keys = [
            "formats.default_encoding",
            "sync.audio_sample_rate", 
            "sync.dialogue_detection_threshold",
            "sync.min_dialogue_duration_ms",
            "general.max_concurrent_jobs",
        ];
        
        for key in &removed_keys {
            if content.contains(key) {
                eprintln!(
                    "⚠️  警告: 配置項目 '{}' 已被移除，將被忽略", 
                    key
                );
            }
        }
        
        for key in &planned_keys {
            if content.contains(key) {
                eprintln!(
                    "ℹ️  資訊: 配置項目 '{}' 功能開發中，目前設定將被忽略", 
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

### 重要決策：保留計劃實作的配置項目

**依據 Backlog #14 的實作計劃**，以下配置項目雖然目前未使用，但已規劃實作對應功能，因此**不予移除**：

1. `formats.default_encoding` - 檔案編碼自動檢測功能
2. `sync.audio_sample_rate` - 音訊採樣率動態配置功能
3. `sync.dialogue_detection_threshold` - 對話檢測功能
4. `sync.min_dialogue_duration_ms` - 對話檢測功能  
5. `general.max_concurrent_jobs` - 平行處理系統

### 確認移除的配置項目處理

#### general.default_confidence
- **原因**: 與 CLI 參數功能重複
- **處理方式**: 完全移除，使用 CLI 參數的預設值機制

#### general.log_level  
- **原因**: 與環境變數 RUST_LOG 機制重複
- **處理方式**: 完全移除，維持標準的日誌配置方式

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
        
        // 確保序列化後不包含已確認移除的欄位
        assert!(!serialized.contains("default_confidence"));
        assert!(!serialized.contains("log_level"));
        
        // 確保序列化後包含計劃實作的欄位
        assert!(serialized.contains("default_encoding"));
        assert!(serialized.contains("audio_sample_rate"));
        assert!(serialized.contains("dialogue_detection_threshold"));
        assert!(serialized.contains("min_dialogue_duration_ms"));
        assert!(serialized.contains("max_concurrent_jobs"));
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

# 測試配置命令不接受已確認移除的配置項目
subx-cli config set general.default_confidence 80  # 應該報錯
subx-cli config set general.log_level debug  # 應該報錯

# 測試配置命令接受計劃實作的配置項目
subx-cli config set formats.default_encoding utf-8  # 應該成功
subx-cli config set general.max_concurrent_jobs 8  # 應該成功
```

## 驗收標準

### 功能驗收
- [ ] 確認移除的配置項目不再出現在程式碼中 (僅 default_confidence, log_level)
- [ ] 計劃實作的配置項目被保留且可正常設定
- [ ] 舊配置檔案仍能正常載入（向後相容）
- [ ] config 命令不再接受已確認移除的配置項目
- [ ] config 命令接受計劃實作的配置項目
- [ ] 所有現有功能正常運作

### 程式碼品質驗收
- [ ] 通過所有現有測試
- [ ] 新增的測試涵蓋向後相容性
- [ ] 程式碼通過 `cargo clippy` 和 `cargo fmt` 檢查
- [ ] 移除了確認無用的死程式碼
- [ ] 保留了計劃實作功能的配置結構

### 文件品質驗收
- [ ] 配置檔案模板已更新 (僅移除確認無用的項目)
- [ ] README 和說明文件已更新
- [ ] 移除了對確認無用配置的說明
- [ ] 為計劃實作的配置添加了適當標記

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

- **精準清理**: 僅移除 2 個確認無用的配置項目 (default_confidence, log_level)
- **保護未來功能**: 保留 5 個計劃實作的配置項目，避免重複工作
- **減少困惑**: 使用者不會被確認無效的配置誤導
- **平衡維護成本**: 在清理冗餘與保留未來功能間取得平衡
- **提升程式碼品質**: 移除確認的死程式碼，保持未來擴展性

## 與其他計劃的協調

- **Backlog #14**: 協調確保計劃實作的配置項目不被移除
- **Bug #08**: 硬編碼配置值修復，專注於已使用但硬編碼的配置
- **Backlog #13**: 統一配置管理，等待此清理完成後進行重構
