---
title: "Job Report: Refactor #81 - 單元測試重新定位到實作檔案"
date: "2025-06-10T04:12:32Z"
---

# Refactor #81 - 單元測試重新定位到實作檔案 工作報告

**日期**：2025-06-10T04:12:32Z  
**任務**：將 tests 資料夾中的單元測試移動到對應的實作檔案中，遵循 Rust 慣例  
**類型**：Refactor  
**狀態**：已完成

## 一、任務概述

根據 Rust 專案慣例，單元測試應該與實作程式碼放在同一個檔案中的 `#[cfg(test)]` 模組內，而不是放在單獨的 `tests/` 資料夾中。`tests/` 資料夾應該只包含整合測試。

在檢查 `tests/` 資料夾時發現了四個檔案包含的是單元測試而非整合測試：
- `ai_retry_tests.rs` - 測試 AI 重試機制的單元測試
- `audio_analyzer_tests.rs` - 測試音訊分析器的單元測試
- `encoding_analyzer_tests.rs` - 測試編碼分析器的單元測試
- `encoding_detector_tests.rs` - 測試編碼檢測器的單元測試

這些測試需要移動到對應的實作檔案中，並修復潛在的競爭條件問題。

## 二、實作內容

### 2.1 AI Retry 測試重新定位
- 將 `tests/ai_retry_tests.rs` 移動到 `src/services/ai/retry.rs`
- 移動了 6 個測試函數，包括重試機制、指數退避、延遲上限等測試
- 【F:src/services/ai/retry.rs†L64-L235】
- 【F:tests/ai_retry_tests.rs†刪除】

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SubXError;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    /// 測試基本重試機制
    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        // ...測試實作
    }
    // ...其他測試
}
```

### 2.2 Encoding Analyzer 測試重新定位
- 將 `tests/encoding_analyzer_tests.rs` 移動到 `src/core/formats/encoding/analyzer.rs`
- 移動了 10 個測試函數，涵蓋位元組分析、中文文字編碼、熵值計算等
- 【F:src/core/formats/encoding/analyzer.rs†L269-L429】
- 【F:tests/encoding_analyzer_tests.rs†刪除】

### 2.3 Encoding Detector 測試重新定位與競爭條件修復
- 將 `tests/encoding_detector_tests.rs` 移動到 `src/core/formats/encoding/detector.rs`
- 修復了嚴重的競爭條件問題：原測試中每個測試都調用 `init_config_manager()` 導致並行測試時產生競爭條件
- 使用 `create_test_detector()` 輔助函數建立測試用的 detector，避免依賴全局配置
- 移動了 12 個測試函數，包括 UTF-8/UTF-16 BOM 檢測、編碼信心值排序等
- 【F:src/core/formats/encoding/detector.rs†L353-L560】
- 【F:tests/encoding_detector_tests.rs†刪除】

```rust
fn create_test_detector() -> EncodingDetector {
    EncodingDetector {
        confidence_threshold: 0.7,
        max_sample_size: 8192,
        supported_charsets: EncodingDetector::default_charsets(),
    }
}
```

### 2.4 Audio Analyzer 測試重新定位
- 將 `tests/audio_analyzer_tests.rs` 移動到 `src/services/audio/analyzer.rs`
- 移動了 8 個測試函數，所有測試都標記為 `#[ignore]` 因為需要音訊處理依賴
- 包含音訊檔案載入、格式轉換、特徵分析等測試
- 【F:src/services/audio/analyzer.rs†L154-L364】
- 【F:tests/audio_analyzer_tests.rs†刪除】

## 三、技術細節

### 3.1 競爭條件問題解決
原來的 encoding detector 測試存在嚴重的競爭條件：
- 每個測試都調用 `init_config_manager().unwrap()`
- 並行測試時會同時修改同一個全局配置管理器
- 導致測試可能無限運行或死鎖

**解決方案**：
- 建立 `create_test_detector()` 函數直接建立測試用的 detector 實例
- 避免依賴全局配置管理器
- 使用預設配置值進行測試

### 3.2 測試組織架構
- 所有移動的測試都放在各自實作檔案的 `#[cfg(test)]` 模組中
- 保持原有的測試邏輯和斷言不變
- 調整必要的 import 路徑

### 3.3 測試標記處理
- Audio analyzer 測試保持 `#[ignore]` 標記，因為需要外部音訊處理依賴
- 其他測試正常運行，無需特殊標記

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
# 格式化檢查
cargo fmt

# Clippy 警告檢查
cargo clippy -- -D warnings
✅ 通過，無警告

# 建置測試
cargo check
✅ 通過
```

### 4.2 單元測試驗證
```bash
# 使用 timeout 防止無限運行
timeout 20 cargo test services::ai::retry::tests --lib
✅ 6 個測試全部通過

timeout 20 cargo test core::formats::encoding::analyzer::tests --lib
✅ 10 個測試全部通過

timeout 20 cargo test core::formats::encoding::detector::tests --lib  
✅ 12 個測試全部通過

timeout 20 cargo test services::audio::analyzer::tests --lib
✅ 8 個測試被正確忽略

# 完整庫測試
timeout 60 cargo test --lib
✅ 135 個測試通過，9 個被忽略
```

### 4.3 特定問題驗證
驗證競爭條件問題已解決：
```bash
# 測試原本會無限運行的測試
timeout 20 cargo test core::formats::encoding::detector::tests::test_encoding_confidence_ranking --lib
✅ 在 1 秒內完成，無競爭條件
```

## 五、影響評估

### 5.1 向後相容性
- ✅ 無 API 變更，完全向後相容
- ✅ 所有測試邏輯保持不變
- ✅ 測試覆蓋率維持不變

### 5.2 程式碼組織改善
- ✅ 遵循 Rust 專案慣例
- ✅ 單元測試與實作程式碼同處一檔，便於維護
- ✅ `tests/` 資料夾現在只包含真正的整合測試
- ✅ 修復了嚴重的測試並行運行競爭條件問題

## 六、問題與解決方案

### 6.1 遇到的問題
- **問題描述**：`test_encoding_confidence_ranking` 測試無限運行
- **根本原因**：多個測試並行調用 `init_config_manager()` 造成競爭條件
- **解決方案**：改用直接建立 detector 實例的方式，避免全局狀態依賴

### 6.2 技術債務
- ✅ **解決**：移除了測試中的全局狀態依賴
- ✅ **解決**：統一了測試組織架構
- ✅ **解決**：修復了潛在的測試穩定性問題

## 七、後續事項

### 7.1 待完成項目
- [x] AI retry 測試移動
- [x] Encoding analyzer 測試移動  
- [x] Encoding detector 測試移動和競爭條件修復
- [x] Audio analyzer 測試移動
- [x] 刪除原始測試檔案
- [x] 驗證所有測試正常運行

### 7.2 相關任務
無直接相關的 Backlog 任務，本次為程式碼品質改善工作。

### 7.3 建議的下一步
- 考慮檢查其他可能存在的全局狀態依賴問題
- 繼續改善測試覆蓋率和測試品質
- 定期檢查測試運行時間，確保無效能問題

## 八、檔案異動清單

| 檔案路徑 | 異動類型 | 描述 |
|---------|----------|------|
| `src/services/ai/retry.rs` | 修改 | 新增 6 個單元測試到 tests 模組 |
| `src/core/formats/encoding/analyzer.rs` | 修改 | 新增 10 個單元測試到 tests 模組 |
| `src/core/formats/encoding/detector.rs` | 修改 | 新增 12 個單元測試，修復競爭條件問題 |
| `src/services/audio/analyzer.rs` | 修改 | 新增 8 個被忽略的單元測試 |
| `tests/ai_retry_tests.rs` | 刪除 | 移動到對應實作檔案 |
| `tests/audio_analyzer_tests.rs` | 刪除 | 移動到對應實作檔案 |
| `tests/encoding_analyzer_tests.rs` | 刪除 | 移動到對應實作檔案 |
| `tests/encoding_detector_tests.rs` | 刪除 | 移動到對應實作檔案 |
