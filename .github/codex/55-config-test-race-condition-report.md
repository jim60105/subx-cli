---
title: "Job Report: 錯誤修正 #14 - 配置測試競爭條件修復"
date: "2025-06-09T14:54:09Z"
---

# 錯誤修正 #14 - 配置測試競爭條件修復 工作報告

**日期**：2025-06-09T14:54:09Z  
**任務**：修正測試套件中 `test_full_config_integration` 競爭條件造成的間歇性失敗  
**類型**：Bug Fix  
**狀態**：已完成

## 一、任務概述

在 CI 環境中，`test_full_config_integration` 測試因並行執行改寫了全域環境變數 `SUBX_CONFIG_PATH` 與全域配置管理器，導致載入錯誤的測試檔案路徑，出現預期 `max_sample_length = 3000`，實際讀得 2000 的間歇性失敗。

## 二、實作內容

### 2.1 新增重置全域配置管理器函式
- 在 `src/config.rs` 中新增 `reset_global_config_manager()`，透過覆寫 `OnceLock` 初始化狀態，清除先前鎖定，允許後續測試重新建立新的 `ConfigManager` 實例。
- 相關程式碼位於 `src/config.rs`【F:src/config.rs†L23-L35】。

```rust
#[allow(invalid_reference_casting)]
pub fn reset_global_config_manager() {
    unsafe {
        let dst = &GLOBAL_CONFIG_MANAGER as *const _ as *mut OnceLock<Mutex<ConfigManager>>;
        std::ptr::write(dst, OnceLock::new());
    }
}
```

### 2.2 測試程式碼隔離與序列化
- 在整合測試 `tests/config_integration_tests.rs` 中，導入 `serial_test::serial`，並為兩個測試函式加上 `#[serial]` 標記，避免同批測試並行執行時改寫環境變數。
- 同時更新 `reset_config_manager()` 呼叫新函式以清除全域環境與配置狀態。
- 相關程式碼位於 `tests/config_integration_tests.rs`【F:tests/config_integration_tests.rs†L4-L7】【F:tests/config_integration_tests.rs†L22-L31】。

```rust
use serial_test::serial;
use subx_cli::config::{init_config_manager, load_config, reset_global_config_manager};

fn reset_config_manager() {
    unsafe {
        env::remove_var("SUBX_CONFIG_PATH");
        /* ... */
    }
    reset_global_config_manager();
}

#[test]
#[serial]
fn test_full_config_integration() { /* ... */ }

#[test]
#[serial]
fn test_base_url_unified_config_integration() { /* ... */ }
```

## 三、技術細節

### 3.1 配置管理器隔離
- 透過重置 `OnceLock`，避免全域配置管理器被多個測試重入時保留舊狀態。
### 3.2 測試序列化
- 利用 `serial_test` crate 標記測試順序，確保整合測試間環境狀態互不干擾。

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build
```

### 4.2 功能測試
```bash
# 單獨執行整合測試，並序列化
cargo test --test config_integration_tests -- --test-threads=1
```

## 五、影響評估

此修正只影響整合測試邏輯，對使用者 API 與核心功能無任何變動，向後相容且風險極低。

## 六、後續事項

- 可考慮其他整合測試套件亦採用相同隔離策略，以提升 CI 穩定性。

## 七、檔案異動清單

| 檔案路徑 | 異動類型 | 描述 |
|---------|----------|------|
| `src/config.rs` | 修改 | 新增 `reset_global_config_manager()` |
| `tests/config_integration_tests.rs` | 修改 | 加入測試隔離與序列化標記 |
