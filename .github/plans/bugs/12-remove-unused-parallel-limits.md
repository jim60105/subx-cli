# Bug #12: 移除未使用的並行限制配置項目

## 問題描述

通過對 `src/core/parallel` 目錄下程式碼的全面分析，發現 `cpu_intensive_limit` 和 `io_intensive_limit` 配置項目雖然被定義和驗證，但在實際的任務執行系統中完全沒有被使用。這些配置項目成為了死代碼，可能會誤導使用者以為配置會生效。

## 技術分析

### 當前實現狀況

1. **配置定義**: 在 `src/config.rs` 和 `src/core/parallel/config.rs` 中定義
2. **驗證邏輯**: 在 `ParallelConfig::validate()` 中有完整的驗證
3. **實際使用**: 調度器 (`TaskScheduler`) 完全沒有使用這些限制：
   - 只使用 `max_concurrent_jobs` 進行總並發控制
   - 直接使用 `tokio::spawn` 執行任務
   - 沒有針對 CPU/IO 類型進行分別限制

### 程式碼證據

- `src/core/parallel/scheduler.rs`: 調度器未引用這些限制
- `src/core/parallel/worker.rs`: 工作者池雖然分類工作者類型，但未被調度器使用
- 任務執行邏輯完全繞過了工作者類型限制

## 影響範圍

- **配置檔案**: `cpu_intensive_limit` 和 `io_intensive_limit` 設定項目
- **文檔**: 相關配置文檔和範例
- **使用者體驗**: 避免對未實現功能的混淆

## 解決方案

### 建議行動：完全移除

基於以下理由，建議完全移除這兩個配置項目：

1. **死代碼清理**: 移除未使用的代碼提高程式碼品質
2. **避免混淆**: 防止使用者誤以為配置會生效
3. **簡化配置**: 減少不必要的配置複雜度
4. **當前設計足夠**: `max_concurrent_jobs` 已能滿足並行控制需求

### 需要修改的檔案

1. **配置結構**:
   - `src/config.rs` - 移除 `ParallelConfig` 中的欄位
   - `src/config/partial.rs` - 移除 `PartialParallelConfig` 中的欄位

2. **並行處理模組**:
   - `src/core/parallel/config.rs` - 移除相關欄位和驗證邏輯

3. **文檔更新**:
   - `.github/instructions/command.instructions.md` - 移除相關文檔
   - `README.md` - 更新配置範例

4. **測試檔案**:
   - 更新相關測試以反映配置變更

## 實作步驟

1. **移除配置定義**:
   ```rust
   // 從 ParallelConfig 中移除
   // pub cpu_intensive_limit: usize,
   // pub io_intensive_limit: usize,
   ```

2. **移除驗證邏輯**:
   ```rust
   // 從 validate() 方法中移除相關檢查
   ```

3. **更新預設值**:
   ```rust
   // 移除 default() 實作中的對應欄位
   ```

4. **更新文檔**:
   - 從配置表格中移除這兩個項目
   - 更新狀態統計資訊
   - 更新 README.md 中的配置範例

5. **執行測試**:
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt
   ```

## 驗證標準

- [ ] 所有測試通過
- [ ] 無編譯警告
- [ ] 配置載入正常運作
- [ ] 文檔準確反映實際功能
- [ ] 並行處理功能不受影響

## 預期結果

- 配置系統更加簡潔和準確
- 移除死代碼提高程式碼品質
- 避免使用者對未實現功能的混淆
- 並行處理仍通過 `max_concurrent_jobs` 正常運作

## 優先級

**中等** - 雖然不影響核心功能，但對程式碼品質和使用者體驗有正面影響

## 標籤

- `cleanup`
- `config`
- `documentation`
- `parallel-processing`

## 相關問題

- Bug #11.2: 並行處理配置實作 (此問題是對該 bug 的進一步澄清)
- 配置審核任務 (此問題是審核過程中發現的)
