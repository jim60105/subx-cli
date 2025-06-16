# Bug 21: Match Command Copy Mode Cache 目標目錄錯誤

## 問題描述

### 核心問題
在 `match` 指令的 `copy` 模式中，當執行 `dry-run` 操作並產生快取後，第二次執行時（不使用 `dry-run`）會將字幕檔案複製到字幕目錄而非視訊檔案目錄。這表明快取系統儲存了錯誤的新位置，或者沒有正確儲存目標視訊檔案目錄。

### 問題重現步驟
1. 執行 `subx match /path/to/media --copy --dry-run` 並產生快取
2. 執行 `subx match /path/to/media --copy`（不使用 dry-run）
3. 觀察到字幕檔案被複製到字幕目錄而非視訊檔案目錄

### 問題影響
- **檔案位置錯誤**：字幕檔案沒有被複製到與視訊檔案相同的目錄
- **使用者體驗**：違反了使用者的預期行為
- **快取可靠性**：快取系統提供了不正確的操作資訊

## 根本原因分析

### 1. 快取加載時缺少重新計算邏輯
在 `src/core/matcher/engine.rs` 的 `check_file_list_cache` 函數中，從快取加載操作時存在關鍵問題：

```rust
// 問題：relocation_target_path 和 requires_relocation 沒有被正確計算
ops.push(MatchOperation {
    video_file,
    subtitle_file,
    new_subtitle_name: item.new_subtitle_name,
    confidence: item.confidence,
    reasoning: item.reasoning,
    relocation_mode: self.config.relocation_mode.clone(), // 正確：使用當前的 relocation_mode
    relocation_target_path: None, // 問題：需要根據當前 relocation_mode 重新計算
    requires_relocation: false,   // 問題：需要根據當前 relocation_mode 重新計算
});
```

### 2. 重新定位資訊計算邏輯缺失
問題的核心在於快取加載時：
- `relocation_mode` 正確使用當前執行時的設定
- 但 `relocation_target_path` 和 `requires_relocation` 沒有被正確重新計算
- 這導致即使指定了 copy 模式，系統仍然認為不需要重新定位（`requires_relocation: false`）

### 3. 快取資料結構實際上是完整的
快取系統已經正確儲存了所有必要資訊：
- `video_file` 和 `subtitle_file` 包含完整路徑
- `original_relocation_mode` 儲存了快取產生時的重新定位模式
- `new_subtitle_name` 包含了目標檔案名稱

問題在於讀取時沒有正確利用這些資訊。

## 技術解決方案

### 方案：在快取加載時重新計算重新定位資訊（推薦）

#### 核心修復
問題的解決方案很直接：在 `check_file_list_cache` 函數中，從快取加載操作後，需要根據當前的 `relocation_mode` 重新計算 `relocation_target_path` 和 `requires_relocation`。

#### 修復步驟

##### 1. 修改 `check_file_list_cache` 函數
在 `src/core/matcher/engine.rs` 中更新快取加載邏輯：

```rust
// 原始問題程式碼
ops.push(MatchOperation {
    video_file,
    subtitle_file,
    new_subtitle_name: item.new_subtitle_name,
    confidence: item.confidence,
    reasoning: item.reasoning,
    relocation_mode: self.config.relocation_mode.clone(),
    relocation_target_path: None, // 問題：固定為 None
    requires_relocation: false,   // 問題：固定為 false
});

// 修復後的程式碼
// 重新計算是否需要重新定位
let requires_relocation = self.config.relocation_mode != FileRelocationMode::None
    && subtitle_file.path.parent() != video_file.path.parent();

// 重新計算目標路徑
let relocation_target_path = if requires_relocation {
    let video_dir = video_file.path.parent().unwrap();
    Some(video_dir.join(&item.new_subtitle_name))
} else {
    None
};

ops.push(MatchOperation {
    video_file,
    subtitle_file,
    new_subtitle_name: item.new_subtitle_name,
    confidence: item.confidence,
    reasoning: item.reasoning,
    relocation_mode: self.config.relocation_mode.clone(),
    relocation_target_path,
    requires_relocation,
});
```

##### 2. 確保重新計算邏輯的一致性
使用與 `match_file_list` 函數中相同的邏輯來計算重新定位資訊：

```rust
// 與原始邏輯保持一致（來自第 686-693 行）
let requires_relocation = self.config.relocation_mode
    != FileRelocationMode::None
    && subtitle_file.path.parent() != video_file.path.parent();

let relocation_target_path = if requires_relocation {
    let video_dir = video_file.path.parent().unwrap();
    Some(video_dir.join(&item.new_subtitle_name))
} else {
    None
};
```

## 實作計劃

### 階段 1：修復快取加載邏輯
1. **修改 `check_file_list_cache` 函數**
   - 在 `src/core/matcher/engine.rs` 中找到 `check_file_list_cache` 函數
   - 在建立 `MatchOperation` 之前加入重新計算邏輯
   - 確保使用與原始邏輯相同的計算方式

2. **實作重新計算邏輯**
   - 根據當前 `relocation_mode` 和檔案路徑計算 `requires_relocation`
   - 根據視訊檔案目錄和新檔案名稱計算 `relocation_target_path`
   - 確保邏輯與 `match_file_list` 函數中的原始邏輯一致

### 階段 2：測試和驗證
1. **單元測試**
   - 測試快取加載時的重新計算邏輯
   - 測試不同 `relocation_mode` 的情況

2. **整合測試**
   - 測試 dry-run 和實際執行的一致性
   - 測試不同目錄結構的情況

### 階段 3：回歸測試
1. **現有測試**
   - 確保所有現有的 copy 和 move 模式測試仍然通過
   - 驗證快取功能在所有模式下正常工作

2. **效能測試**
   - 確保修復不會影響快取系統的效能

## 測試策略

### 單元測試
```rust
#[tokio::test]
async fn test_cache_preserves_relocation_info() {
    // 測試快取儲存和加載保持重新定位資訊
}

#[tokio::test]
async fn test_copy_mode_cache_target_directory() {
    // 測試 copy 模式下快取的目標目錄正確性
}
```

### 整合測試
```rust
#[tokio::test]
async fn test_dry_run_then_execute_copy_mode() {
    // 1. 執行 dry-run 並產生快取
    // 2. 執行實際操作
    // 3. 驗證檔案被複製到正確的目錄
}
```

### 回歸測試
- 確保現有的 copy 和 move 模式測試仍然通過
- 驗證快取功能在所有模式下正常工作

## 風險評估

### 低風險
- **程式碼變更範圍小**：只需要修改一個函數中的邏輯
- **現有快取結構完整**：不需要修改快取資料結構
- **邏輯重用**：使用現有的計算邏輯，降低引入新 bug 的風險

### 中風險
- **測試覆蓋**：需要確保新的重新計算邏輯被充分測試
- **邊界情況**：需要處理檔案路徑不存在等邊界情況

## 相容性考慮

### 向後相容性
- **快取格式不變**：現有快取檔案格式保持不變
- **無破壞性變更**：修復不會破壞現有功能

### 向前相容性
- **邏輯穩定**：重新計算邏輯基於現有的成熟邏輯
- **可維護性**：程式碼變更簡單，易於維護

## 驗收標準

### 功能驗收
1. ✅ 從快取加載時正確重新計算 `requires_relocation` 和 `relocation_target_path`
2. ✅ copy 模式下檔案被複製到正確的目錄（視訊檔案目錄）
3. ✅ dry-run 和實際執行的行為一致
4. ✅ 快取系統保持高效能

### 品質驗收
1. ✅ 所有現有測試繼續通過
2. ✅ 新增的測試覆蓋率 > 90%
3. ✅ 沒有記憶體洩漏或效能回歸
4. ✅ 程式碼符合專案風格指南

## 完成時間估算

- **階段 1**：1 工作日（修復快取加載邏輯）
- **階段 2-3**：2-3 工作日（測試和驗證）
- **總計**：3-4 工作日
- **總計**：5-8 工作日

## 後續追蹤

### 監控指標
- 快取命中率
- 快取檔案大小
- 操作執行時間

### 文件更新
- 更新快取系統文件
- 更新故障排除指南
- 更新 API 文件

### 使用者溝通
- 在 CHANGELOG 中記錄修復
- 更新使用者文件
- 提供升級指南

## 相關議題

- 參考 `tests/match_copy_behavior_tests.rs` 中的現有測試
- 考慮 `tests/match_cache_reuse_tests.rs` 中的快取重用測試
- 檢查 `src/commands/cache_command.rs` 中的快取管理命令

## 結論

這個錯誤的根本原因是快取系統在加載操作時沒有正確重新計算 `relocation_target_path` 和 `requires_relocation` 欄位。雖然快取中儲存了完整的檔案路徑資訊，但在重建 `MatchOperation` 時，這些關鍵欄位被硬編碼為 `None` 和 `false`。

解決方案非常直接：在 `check_file_list_cache` 函數中，使用與原始邏輯相同的方式重新計算這些欄位。這樣可以確保 dry-run 和實際執行之間的一致性，讓使用者獲得預期的行為。

這個修復的優點是：
- **變更範圍小**：只需要修改一個函數
- **風險低**：重用現有的成熟邏輯
- **相容性好**：不需要修改快取資料結構
- **效果明確**：直接解決問題根源
