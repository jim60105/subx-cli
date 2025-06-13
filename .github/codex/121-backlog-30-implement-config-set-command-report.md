---
title: "Job Report: Backlog #30 - 實現 Config Set 指令功能 (階段3 & 階段4)"
date: "2025-06-13T15:45:48Z"
---

# Backlog #30 - 實現 Config Set 指令功能 工作報告

**日期**：2025-06-13T15:45:48Z  
**任務**：階段3 - 更新 CLI `config set` 行為；階段4 - 擴展 TestConfigService 的 `set_config_value` 與驗證機制  
**類型**：Backlog  
**狀態**：已完成

## 一、任務概述

本次工作為 Backlog #30 的階段3與階段4，目標在於：
- 在 `config_command.rs` 中實作 `config set` 指令的行為，替換原先未實作的 TODO 標記，並印出執行結果。
- 在測試專用的 `TestConfigService` 中新增 `set_config_value` 方法與對應的驗證邏輯，確保測試環境能完整模擬生產端配置設定流程。

## 二、實作內容

### 2.1 在 config_command.rs 實作 Set 操作 (階段3)
- 移除原有 TODO，改為透過 `config_service.set_config_value` 設定配置值，並於成功後輸出確認訊息、顯示更新後的值與存檔路徑。
- 【F:src/commands/config_command.rs†L248-L258】【F:src/commands/config_command.rs†L303-L312】

```rust
ConfigAction::Set { key, value } => {
    config_service.set_config_value(&key, &value)?;
    println!("✓ Configuration '{}' set to '{}'", key, value);
    if let Ok(current) = config_service.get_config_value(&key) {
        println!("  Current value: {}", current);
    }
    if let Ok(path) = config_service.get_config_file_path() {
        println!("  Saved to: {}", path.display());
    }
}
```

### 2.2 擴展 TestConfigService 支援 set_config_value 與驗證 (階段4)
- 將 `TestConfigService` 結構改為 `Mutex<Config>`，以實現內部可變狀態管理。
- 更新 `get_config`、`get_config_value` 及 `reset_to_defaults`，改為操作互斥鎖中的 `Config`。
- 實作 `ConfigService::set_config_value`，重用 `validate_and_set_value` 驗證後更新內部配置。
- 新增 `validate_and_set_value` 方法，複製生產端相同的 key-path 驗證與型別轉換邏輯。
- 【F:src/config/test_service.rs†L11-L20】【F:src/config/test_service.rs†L98-L118】【F:src/config/test_service.rs†L162-L178】【F:src/config/test_service.rs†L181-L240】

```rust
pub struct TestConfigService {
    config: Mutex<Config>,
}

impl ConfigService for TestConfigService {
    fn set_config_value(&self, key: &str, value: &str) -> Result<()> {
        let mut cfg = self.get_config()?;
        self.validate_and_set_value(&mut cfg, key, value)?;
        crate::config::validator::validate_config(&cfg)?;
        *self.config.lock().unwrap() = cfg;
        Ok(())
    }
    // ... get_config, get_config_value, reset_to_defaults 皆已更新為對 Mutex 操作 ...
}

impl TestConfigService {
    fn validate_and_set_value(&self, config: &mut Config, key: &str, value: &str) -> Result<()> {
        use crate::config::validation::*;
        use crate::config::OverflowStrategy;

        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["ai", "provider"] => {
                validate_enum(value, &["openai", "anthropic", "local"])?;
                config.ai.provider = value.to_string();
            }
            // ...其餘 key-path 驗證邏輯...
            _ => return Err(SubXError::config(format!("Unknown configuration key: {}", key))),
        }
        Ok(())
    }
}
```

## 三、技術細節

### 3.1 架構變更
- CLI 層新增 `ConfigAction::Set` 處理邏輯，與 `ConfigService` 解耦。
- 測試層使用互斥鎖管理 `Config`，並與生產端共用同套型別與驗證邏輯。

### 3.2 API 變更
- `execute` / `execute_with_config` 新增 `ConfigAction::Set { key, value }` 分支。
- `ConfigService` 已於先前階段新增 `set_config_value` 方法接口。

### 3.3 配置變更
- 無需修改外部設定檔或環境變數。

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build
cargo test
```

### 4.2 功能測試
- 已執行現有單元與整合測試，確保 CLI `config set` 與 TestConfigService 行為符合預期。

### 4.3 覆蓋率測試
```bash
scripts/check_coverage.sh -T
```

## 五、影響評估

### 5.1 向後相容性
- 不影響現有 `config get` / `config list` / `config reset` 功能。

### 5.2 使用者體驗
- 成功設定後即時回饋並顯示存檔位置，提升可操作性。

## 六、問題與解決方案

### 6.1 遇到的問題
- 無重大障礙。

### 6.2 技術債務
- 待撰寫完整文件範例 (backlog 階段6)。

## 七、後續事項

### 7.1 待完成項目
- [ ] 撰寫驗證函數單元與整合測試 (階段5)
- [ ] 補充 CLI 使用文件與範例 (階段6)

### 7.2 相關任務
- Backlog #30

### 7.3 建議的下一步
- 依序完成階段5與階段6，並確保文件與測試覆蓋完整。

## 八、檔案異動清單

| 檔案路徑                           | 異動類型 | 描述                                         |
|------------------------------------|----------|----------------------------------------------|
| `src/commands/config_command.rs`   | 修改     | 實作 `ConfigAction::Set` 行為 (階段3)        |
| `src/config/test_service.rs`       | 修改     | 擴展 TestConfigService 支援 `set_config_value` 與驗證 (階段4) |
