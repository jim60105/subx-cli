---
title: "Job Report: Bug Fix #113 - Wiremock 快取重用修復"
date: "2025-06-12T14:10:37Z"
---

# Bug Fix #113 - Wiremock 快取重用修復工作報告

**日期**：2025-06-12T14:10:37Z  
**任務**：修復 Wiremock 整合測試框架中的快取重用問題，確保測試穩定性和快取功能正確性  
**類型**：Bug Fix  
**狀態**：已完成

## 一、任務概述

SubX 專案中的 Wiremock 整合測試存在快取重用功能問題。主要表現為 `test_cache_reuse_preserves_copy_mode` 和 `test_cache_reuse_preserves_move_mode` 測試在並行執行時失敗，影響持續整合的穩定性。問題根源包括測試間環境變數競爭、檔案 ID 不匹配，以及快取路徑不一致等。此任務旨在診斷並修復這些問題，確保快取機制在真實場景中的正確運作。

## 二、實作內容

### 2.1 測試隔離機制改善
- 實作靜態互斥鎖以序列化測試執行，避免環境變數競爭
- 【F:tests/match_cache_reuse_tests.rs†L1-L15】新增測試序列化機制

```rust
static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tokio::test]
async fn test_cache_reuse_preserves_copy_mode() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let test_root = std::path::Path::new("/tmp/subx_cache_test");
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }
    // ...
}
```

### 2.2 快取路徑一致性修復
- 修改快取路徑邏輯，優先使用 XDG_CONFIG_HOME 環境變數
- 【F:src/core/matcher/engine.rs†L200-L220】更新快取路徑取得邏輯

```rust
fn get_cache_file_path(&self) -> Result<std::path::PathBuf> {
    let dir = if let Some(xdg_config) = std::env::var_os("XDG_CONFIG_HOME") {
        std::path::PathBuf::from(xdg_config)
    } else {
        dirs::config_dir()
            .ok_or_else(|| SubXError::config("Unable to determine cache directory"))?
    };
    Ok(dir.join("subx").join("match_cache.json"))
}
```

### 2.3 動態檔案 ID 處理
- 修改測試邏輯，使用實際掃描產生的檔案 ID 設定 Mock 期望
- 【F:tests/match_cache_reuse_tests.rs†L50-L100】更新測試檔案 ID 處理邏輯

### 2.4 編譯警告處理
- 為測試輔助方法加入 `#[allow(dead_code)]` 屬性消除編譯警告
- 【F:tests/common/mock_openai_helper.rs†L1-L10】、【F:tests/common/test_data_generators.rs†L1-L10】

## 三、技術細節

### 3.1 架構變更
- 快取系統現在支援測試環境的獨立配置路徑
- 測試執行機制從並行改為序列化，確保環境變數隔離

### 3.2 API 變更
- `get_cache_file_path` 方法內部邏輯調整，但對外介面保持不變
- 快取檔案路徑計算邏輯增強，支援測試環境的特殊需求

### 3.3 配置變更
- 測試環境使用 `XDG_CONFIG_HOME` 環境變數指定快取路徑
- 生產環境維持原有的系統配置目錄邏輯

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
# 格式化檢查
cargo fmt -- --check

# Clippy 警告檢查
cargo clippy -- -D warnings

# 建置測試
cargo build

# 單元測試
cargo test
```

### 4.2 功能測試
- **快取重用測試**：`test_cache_reuse_preserves_copy_mode` 和 `test_cache_reuse_preserves_move_mode` 測試通過
- **測試結果**：
```
running 2 tests
test test_cache_reuse_preserves_copy_mode ... ok
test test_cache_reuse_preserves_move_mode ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 23 filtered out; finished in 0.12s
```

### 4.3 整合測試驗證
- 驗證快取在第一次執行時正確建立
- 確認第二次執行時成功重用快取，跳過 AI API 調用
- 測試不同檔案 ID 和配置變更時的快取失效機制

## 五、影響評估

### 5.1 向後相容性
- 對生產環境完全向後相容，不影響現有快取行為
- 測試環境增強了環境隔離能力，提升測試可靠性

### 5.2 使用者體驗
- 提升測試執行的穩定性和可預測性
- 減少因測試失敗導致的開發流程中斷

## 六、問題與解決方案

### 6.1 遇到的問題
- **問題描述**：並行測試執行時 `XDG_CONFIG_HOME` 環境變數被其他測試覆蓋，導致快取路徑混亂
- **解決方案**：使用靜態互斥鎖序列化測試執行，確保環境變數操作的原子性

### 6.2 技術債務
- **解決的債務**：移除了測試中的不穩定因素，提升了測試套件的整體品質
- **新增的債務**：測試序列化可能略微增加測試執行時間，但在可接受範圍內

## 七、後續事項

### 7.1 待完成項目
- [ ] 考慮實作更細粒度的測試隔離機制
- [ ] 增加更多快取邊界條件的測試案例
- [ ] 評估快取壓縮和版本管理的必要性

### 7.2 相關任務
- 與整體測試框架改善計畫相關
- 可能影響未來的快取系統優化任務

### 7.3 建議的下一步
- 監控此修復在持續整合環境中的長期穩定性
- 考慮將測試隔離機制標準化應用到其他測試模組
- 評估是否需要為快取系統增加更多監控和除錯功能
