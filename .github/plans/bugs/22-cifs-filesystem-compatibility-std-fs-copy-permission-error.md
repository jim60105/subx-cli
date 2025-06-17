# Bug 22: CIFS 檔案系統相容性 - std::fs::copy 操作權限錯誤

## 問題描述

### 核心問題
當 SubX 在 CIFS（SMB/網路共享）檔案系統上執行檔案複製操作時，`std::fs::copy` 函式會拋出 "Operation not permitted (os error 1)" 錯誤，即使在 SELinux Permissive 模式下也會發生此問題。

### 問題根本原因
`std::fs::copy` 函式的實作包含兩個階段：
1. **檔案內容複製**：將來源檔案的資料位元組完整寫入目標檔案
2. **元資料複製**：嘗試複製檔案權限（POSIX permissions）

CIFS 檔案系統基於 Windows 的存取控制清單（ACL）模型，與 Unix/Linux 的 POSIX 權限模型不相容。當 `std::fs::copy` 嘗試使用 `fchmod` 系統呼叫設定 POSIX 權限時，CIFS 檔案系統或 Linux 核心的 CIFS 模組無法正確處理此操作，導致權限被拒絕的錯誤。

### 錯誤發生時序
1. 檔案建立成功（產生空檔案）
2. 檔案內容複製開始或準備開始
3. 元資料（權限）設定嘗試
4. CIFS 檔案系統拒絕權限設定操作
5. `std::fs::copy` 回傳錯誤，操作中止

### 影響範圍
此問題影響所有需要將檔案複製到 CIFS 掛載點的 SubX 功能：
- **Match 指令的 Copy 模式**：複製字幕檔案到視訊檔案目錄
- **Backup 操作**：建立備份檔案
- **檔案重新定位**：跨目錄的檔案複製操作

## 技術分析

### 問題檔案分析
根據程式碼分析，以下檔案中使用了 `std::fs::copy`：

#### 1. src/core/matcher/engine.rs
- **第 871 行**：`std::fs::copy(&final_target, backup_path)?;` - 備份操作
- **第 875 行**：`std::fs::copy(&source_path, &final_target)?;` - 檔案重新定位複製
- **第 914 行**：`std::fs::copy(&source_path, backup_path)?;` - Move 操作前備份
- **第 926 行**：`std::fs::copy(&final_target, backup_path)?;` - 目標檔案備份
- **第 986 行**：`std::fs::copy(&final_target, backup_path)?;` - 執行複製操作前備份
- **第 990 行**：`std::fs::copy(&op.subtitle_file.path, &final_target)?;` - 主要複製操作
- **第 1021 行**：`std::fs::copy(&final_target, backup_path)?;` - 本地複製備份
- **第 1025 行**：`std::fs::copy(&op.subtitle_file.path, &final_target)?;` - 本地複製操作
- **第 1098 行**：`std::fs::copy(old_path, backup_path)?;` - 重新命名前備份

#### 2. src/core/parallel/task.rs
- **第 306 行**：`std::fs::copy(source, &final_target)?;` - 平行處理中的複製操作

### 架構問題分析

根據進一步的程式碼分析，發現了更深層的架構問題：

#### 當前架構問題
1. **MatchEngine 直接處理檔案操作**：違反了單一職責原則
2. **重複的檔案操作實作**：MatchEngine 和 FileProcessingTask 都有自己的檔案複製邏輯
3. **無法利用平行處理**：MatchEngine 的檔案操作是序列的
4. **維護困難**：相同的 Bug 需要在兩個地方修復

#### 正確的架構設計
```
┌─────────────────────────────────────┐
│           MatchEngine               │  ← 高層邏輯（AI分析、配對決策）
│  - AI content analysis              │
│  - Match operations planning        │
│  - Cache management                 │
└─────────────┬───────────────────────┘
              │ delegates to
              ▼
┌─────────────────────────────────────┐
│        FileProcessingTask           │  ← 底層檔案操作（統一、平行化）
│  - File copy operations             │
│  - File move operations             │
│  - Conflict resolution              │
│  - CIFS compatibility handling     │
└─────────────────────────────────────┘
```

### 解決方案設計

#### 主要方案：架構重構 + CIFS 相容性修復
將檔案操作從 MatchEngine 重構到 FileProcessingTask，同時解決 CIFS 相容性問題。

**核心策略**：
1. **統一檔案操作**：所有檔案操作都透過 FileProcessingTask 執行
2. **CIFS 安全的複製**：在 FileProcessingTask 中使用 `std::io::copy` 替代 `std::fs::copy`
3. **保持 API 相容性**：MatchEngine 的公開介面保持不變，內部委託給 FileProcessingTask

**CIFS 安全複製實作**：
```rust
// 在 FileProcessingTask 中實作 CIFS 相容的複製
async fn copy_file_cifs_safe(source: &Path, target: &Path) -> Result<u64> {
    let mut source_file = File::open(source)?;
    let mut dest_file = File::create(target)?;
    std::io::copy(&mut source_file, &mut dest_file)
}
```

## 實作計畫

### 階段 1：擴充 FileProcessingTask 支援所有檔案操作 (4-5 小時)

**目標**：讓 FileProcessingTask 能夠處理 MatchEngine 所需的所有檔案操作類型

**實作內容**：
1. **擴充 ProcessingOperation 枚舉**：
   - `CopyWithRename`：複製並重新命名（本地複製操作）
   - `CreateBackup`：建立備份檔案
   - `RenameFile`：檔案重新命名
   - 所有操作都使用 CIFS 安全的 `std::io::copy`

2. **實作新的檔案操作方法**：
   - `execute_copy_with_rename_operation()`
   - `execute_create_backup_operation()` 
   - `execute_rename_file_operation()`

3. **實作 CIFS 相容的核心複製函式**：
   ```rust
   async fn copy_file_cifs_safe(&self, source: &Path, target: &Path) -> Result<u64>
   ```

### 階段 2：重構 MatchEngine 以委託檔案操作 (5-6 小時)

**目標**：移除 MatchEngine 中的直接檔案操作，改為建立 FileProcessingTask

**修改方法**：
1. **修改 execute_operations 方法**：
   ```rust
   // 原本：直接呼叫 self.execute_copy_operation(op)
   // 改為：建立並執行 FileProcessingTask
   ```

2. **建立檔案操作工廠方法**：
   ```rust
   fn create_copy_task(&self, op: &MatchOperation) -> FileProcessingTask
   fn create_backup_task(&self, file_path: &Path) -> FileProcessingTask 
   fn create_rename_task(&self, op: &MatchOperation) -> FileProcessingTask
   ```

3. **保持向後相容性**：MatchEngine 的公開 API 保持不變

### 階段 3：移除 MatchEngine 中的重複檔案操作程式碼 (2-3 小時)

**目標**：清理 MatchEngine 中不再需要的檔案操作方法

**移除的方法**：
- `execute_copy_operation()`
- `execute_local_copy()`
- `execute_relocation_operation()` 中的檔案操作部分
- 備份相關的直接 `std::fs::copy` 呼叫

### 階段 4：更新測試以驗證架構重構 (3-4 小時)

**測試更新**：
1. **單元測試**：確保 FileProcessingTask 的新操作正確運作
2. **整合測試**：驗證 MatchEngine 透過 FileProcessingTask 的檔案操作
3. **CIFS 相容性測試**：模擬 CIFS 環境測試
4. **回歸測試**：確保所有現有功能正常運作

### 階段 5：文件更新 (1 小時)

**更新內容**：
- 架構文件：說明新的檔案操作架構
- 技術文件：FileProcessingTask 的新功能說明

## 測試策略

### 單元測試
1. **工具函式測試**：驗證 `copy_file_cifs_safe` 函式的正確性
2. **功能測試**：確保所有複製操作仍然正常工作
3. **錯誤處理測試**：驗證錯誤情況的適當處理

### 整合測試
1. **Match 指令測試**：驗證 copy 模式在修改後仍正常工作
2. **Backup 功能測試**：確保備份操作不受影響
3. **跨目錄複製測試**：驗證檔案重新定位功能

### 回歸測試
執行完整的測試套件，確保沒有破壞現有功能。

## 風險評估

### 低風險
- **功能相同性**：`std::io::copy` 提供相同的核心功能（檔案內容複製）
- **效能影響**：效能差異微乎其微
- **相容性**：在所有檔案系統上都能正常工作

### 注意事項
- **權限繼承**：新建立的檔案將使用目標目錄的預設權限，而非來源檔案的權限
- **時間戳記**：不會複製檔案的修改時間等元資料

## 驗證標準

### 功能驗證
- [ ] 所有原有的檔案複製功能仍正常工作
- [ ] CIFS 檔案系統上的複製操作不再出現權限錯誤
- [ ] 備份功能正常運作
- [ ] 檔案重新定位功能正常運作

### 測試覆蓋率
- [ ] 新增的工具函式有完整的單元測試
- [ ] 修改後的程式碼通過所有現有測試
- [ ] 新增 CIFS 相容性相關測試

### 文件完整性
- [ ] 程式碼變更有適當的註解說明
- [ ] 技術文件已更新
- [ ] CHANGELOG 記錄了相關變更

## 預期效益

### 直接效益
- **解決 CIFS 相容性問題**：消除網路檔案系統上的權限錯誤
- **提升使用者體驗**：支援更多檔案系統環境
- **增強可靠性**：減少檔案操作失敗的情況

### 長期效益
- **更好的跨平台支援**：為未來的跨平台擴展奠定基礎
- **程式碼簡化**：統一的檔案複製介面
- **維護性提升**：集中化的檔案操作錯誤處理

## 完成時間估計

**總預計時間**：8-12 小時

- 階段 1：2-3 小時
- 階段 2：3-4 小時  
- 階段 3：1-2 小時
- 階段 4：2-3 小時
- 階段 5：30 分鐘

## 實作注意事項

### 程式碼品質
- 使用一致的錯誤處理模式
- 保持現有的程式碼風格
- 新增適當的文件註解

### 相容性保證
- 確保在所有支援的作業系統上測試
- 驗證與現有 API 的相容性
- 保持向後相容性

### 測試需求
- 在實際的 CIFS 環境中測試（如果可能）
- 使用模擬的檔案系統錯誤進行測試
- 執行完整的回歸測試套件
