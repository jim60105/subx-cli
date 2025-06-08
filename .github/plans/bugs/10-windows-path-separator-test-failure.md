# Bug Fix #10: Windows 平台路徑分隔符測試失敗

## 問題描述

在 Windows 平台上執行測試時，`core::matcher::tests::test_file_info_creation` 測試會失敗，錯誤訊息如下：

```
thread 'core::matcher::tests::test_file_info_creation' panicked at src\core\matcher\mod.rs:76:9:
assertion `left == right` failed
  left: "season1\\episode1.mp4"
 right: "season1/episode1.mp4"
```

這個問題導致跨平台相容性受損，影響 Windows 使用者的開發體驗。

## 問題分析

### 現狀分析
- 測試程式碼在第76行硬編碼了 Unix 風格的路徑分隔符 `/`
- Windows 系統使用反斜線 `\` 作為路徑分隔符
- `FileInfo::new()` 函式正確使用了系統原生路徑分隔符，但測試期望值錯誤

### 根本原因
1. **測試資料硬編碼問題**：測試中直接使用 `"season1/episode1.mp4"` 作為期望值
2. **跨平台路徑處理不一致**：實際實作使用系統路徑分隔符，但測試期望值固定使用 Unix 格式
3. **缺乏平台相關的測試策略**：沒有考慮不同作業系統的路徑差異

### 影響範圍
- 所有涉及路徑處理的 `FileInfo` 相關功能
- Windows 平台的持續整合測試
- 開發者在 Windows 環境下的測試體驗

## 技術方案

### 架構設計
1. **統一路徑表示法**
   - 在內部處理中統一使用 Unix 風格路徑分隔符 `/`
   - 確保跨平台一致性和可預測性

2. **改進測試策略**
   - 使用平台無關的路徑處理方法
   - 建立可移植的測試資料建構邏輯

### 解決方案選項

#### 選項一：統一內部路徑格式（推薦）
- 修改 `FileInfo::new()` 函式，將 `relative_path` 統一轉換為 Unix 風格
- 保持對外 API 的一致性
- 確保內部邏輯的可預測性

#### 選項二：平台特定測試
- 根據當前平台動態產生期望值
- 使用 `std::path::MAIN_SEPARATOR` 建構期望的路徑字串
- 保持平台原生行為

## 實作步驟

### 第一階段：修復核心路徑處理邏輯

1. **修改 FileInfo 實作**
   ```rust
   // 在 src/core/matcher/mod.rs 的 FileInfo::new() 中
   let relative_path = full_path
       .strip_prefix(root_path)
       .map_err(|e| SubXError::Other(e.into()))?
       .to_string_lossy()
       .replace('\\', "/");  // 統一轉換為 Unix 風格路徑
   ```

2. **更新深度計算邏輯**
   ```rust
   let depth = relative_path.matches('/').count();  // 使用統一的分隔符
   ```

### 第二階段：增強測試覆蓋率

1. **建立跨平台測試輔助函式**
   ```rust
   #[cfg(test)]
   mod test_helpers {
       use std::path::Path;
       
       pub fn create_test_path_string(segments: &[&str]) -> String {
           segments.join("/")  // 統一使用 Unix 風格
       }
   }
   ```

2. **擴展現有測試**
   - 新增更多路徑深度的測試案例
   - 測試特殊字元和 Unicode 路徑
   - 驗證邊界條件

### 第三階段：文件更新

1. **API 文件說明**
   - 明確說明 `relative_path` 欄位始終使用 Unix 風格分隔符
   - 補充跨平台相容性說明

2. **測試文件**
   - 記錄路徑處理的設計決策
   - 說明跨平台測試策略

## 測試策略

### 單元測試
```rust
#[test]
fn test_file_info_cross_platform() -> Result<()> {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    
    // 建立深層目錄結構
    let file_path = root.join("season1").join("episode1.mp4");
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    std::fs::write(&file_path, b"").unwrap();

    let info = FileInfo::new(file_path.clone(), root)?;
    
    // 驗證統一的路徑格式
    assert_eq!(info.name, "episode1.mp4");
    assert_eq!(info.relative_path, "season1/episode1.mp4");  // 始終使用 Unix 風格
    assert_eq!(info.directory, "season1");
    assert_eq!(info.depth, 1);
    
    Ok(())
}

#[test]
fn test_file_info_deep_path() -> Result<()> {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    
    // 測試多層目錄
    let file_path = root.join("series").join("season1").join("episodes").join("ep01.mp4");
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    std::fs::write(&file_path, b"").unwrap();

    let info = FileInfo::new(file_path.clone(), root)?;
    
    assert_eq!(info.relative_path, "series/season1/episodes/ep01.mp4");
    assert_eq!(info.depth, 3);
    
    Ok(())
}
```

### 整合測試
**注意：跨平台測試主要透過 CI 自動化執行，無需在本地環境進行跨平台測試**

- CI 環境會自動在不同作業系統上驗證路徑處理一致性
- CI 自動化測試涵蓋實際檔案操作的跨平台行為

## 驗證計劃

### 自動化驗證（主要方法）
1. **CI/CD 跨平台測試**
   - **主要驗證方式**：透過 GitHub Actions 或類似 CI 系統自動化測試
   - **測試矩陣**：確保在 Linux、macOS、Windows 三個平台上的測試都通過
   - **專門的 Windows 測試環境**：CI 中設定 Windows runner 執行完整測試套件
   - **自動化觸發**：每次程式碼提交和 Pull Request 都會觸發跨平台測試
   - **測試報告**：CI 會自動產生各平台的測試結果報告

2. **本地開發驗證**
   - **開發者責任**：在本地進行基本功能測試和程式碼品質檢查
   - **單元測試**：確保本地環境下單元測試通過
   - **程式碼格式化**：執行 `cargo fmt` 和 `cargo clippy` 確保程式碼品質

3. **迴歸測試**
   - **CI 自動執行**：驗證現有功能不受影響
   - **跨平台一致性**：確認路徑處理邏輯在所有平台上的一致性

### 手動驗證（輔助方法）
**注意：跨平台相容性主要透過 CI 驗證，手動驗證僅作為補充**

1. **本地功能測試**
   - 在開發環境下執行基本的檔案匹配流程
   - 驗證程式碼修改不會破壞現有功能

2. **程式碼品質檢查**
   - 執行 `cargo fmt` 確保程式碼格式一致
   - 執行 `cargo clippy -- -D warnings` 修復所有警告
   - 確保單元測試在本地通過

### CI 流程整合
1. **測試流水線設計**
   - Pull Request 觸發：自動執行所有平台測試
   - 主分支合併前：確保所有 CI 檢查通過
   - 失敗處理：CI 失敗時阻止合併，要求修復後重新測試

2. **測試覆蓋範圍**
   - 單元測試：所有平台執行完整測試套件
   - 整合測試：驗證實際檔案操作在各平台的行為

## 風險評估

### 技術風險
- **低風險**：路徑處理邏輯相對簡單，影響範圍可控
- **相容性**：統一路徑格式可能影響依賴原生路徑格式的程式碼

### 緩解措施
1. **向後相容性**
   - 確保對外 API 行為保持一致
   - 建立完整的測試覆蓋

2. **段落式部署**
   - 先修復測試，再優化實作
   - 逐步驗證各個功能模組

## 後續改進

### 長期規劃
1. **路徑處理標準化**
   - 建立統一的路徑處理工具函式
   - 制定跨平台開發指南

2. **測試基礎設施完善**
   - 建立更全面的跨平台測試框架
   - 自動化多平台驗證流程

### 技術債務清理
- 檢查其他模組的路徑處理邏輯
- 統一專案中的路徑處理模式
- 完善錯誤處理和邊界條件檢查
