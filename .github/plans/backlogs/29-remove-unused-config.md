# Backlog 29: 移除未使用的配置項目

## 概覽

本 backlog 專注於移除 SubX 專案中已識別的 5 個未使用配置項目，以簡化配置結構、提升程式碼維護性，並確保配置系統的一致性。

## 背景

經過詳細的配置使用情況分析，發現以下配置項目已定義但從未在程式碼中實際使用：

### 未使用的配置項目清單

| 配置項目 | 位置 | 狀態 |
|---------|------|------|
| `general.temp_dir` | `src/config/mod.rs:31` | 未使用 |
| `general.log_level` | `src/config/mod.rs:32` | 未使用 |
| `general.cache_dir` | `src/config/mod.rs:33` | 未使用 |
| `parallel.chunk_size` | `src/config/mod.rs:75` | 未使用 |
| `parallel.enable_work_stealing` | `src/config/mod.rs:76` | 未使用 |

## 目標

1. **簡化配置結構**：移除所有未使用的配置項目
2. **更新相關文件**：同步更新所有文件和範例
3. **確保測試完整性**：移除相關測試並確保不影響現有功能
4. **保持向後相容性**：在可能的情況下提供適當的警告訊息

## 實作階段

### 階段 1：準備工作（預估時間：30 分鐘）

#### 1.1 全面程式碼搜尋
```bash
# 搜尋所有可能引用這些配置項目的地方
grep -r "temp_dir" src/ tests/ docs/ --include="*.rs" --include="*.md"
grep -r "log_level" src/ tests/ docs/ --include="*.rs" --include="*.md"
grep -r "cache_dir" src/ tests/ docs/ --include="*.rs" --include="*.md"
grep -r "chunk_size" src/ tests/ docs/ --include="*.rs" --include="*.md"
grep -r "enable_work_stealing" src/ tests/ docs/ --include="*.rs" --include="*.md"
```

#### 1.2 確認影響範圍
- 檢查配置結構體定義
- 檢查配置建構器
- 檢查配置驗證器
- 檢查測試檔案
- 檢查文件範例

### 階段 2：移除配置項目定義（預估時間：20 分鐘）

#### 2.1 更新 `src/config/mod.rs`
**位置**: `GeneralConfig` 結構體
```rust
// 移除以下欄位：
// pub temp_dir: Option<PathBuf>,      // 第 31 行
// pub log_level: Option<String>,      // 第 32 行  
// pub cache_dir: Option<PathBuf>,     // 第 33 行
```

**位置**: `ParallelConfig` 結構體
```rust
// 移除以下欄位：
// pub chunk_size: Option<usize>,           // 第 75 行
// pub enable_work_stealing: Option<bool>,  // 第 76 行
```

#### 2.2 更新配置建構器 `src/config/builder.rs`
- 移除對應的建構器方法
- 移除相關的預設值設定
- 更新建構器測試

#### 2.3 更新配置驗證器 `src/config/validator.rs`
- 移除相關的驗證邏輯（如果存在）
- 更新驗證測試

### 階段 3：更新測試檔案（預估時間：40 分鐘）

#### 3.1 識別並更新相關測試
需要檢查的測試檔案：
- `tests/config_integration_tests.rs`
- `tests/config_service_integration_tests.rs`
- `tests/config_basic_integration.rs`
- `tests/config_value_integration_tests.rs`
- `src/config/test_service.rs`
- 其他可能包含配置測試的檔案

#### 3.2 移除或更新測試案例
- 移除專門測試未使用配置項目的測試案例
- 更新包含這些配置項目的配置範例
- 確保所有測試仍能通過

#### 3.3 更新測試輔助工具
- 更新 `TestConfigService` 中的配置範例
- 移除相關的測試宏（如果存在）

### 階段 4：更新文件和範例（預估時間：30 分鐘）

#### 4.1 更新 README 檔案
- **檔案**: `README.md`、`README.zh-TW.md`
- 移除配置範例中的未使用項目
- 更新配置說明表格

#### 4.2 更新技術文件
- **檔案**: `docs/tech-architecture.md`
- 移除對未使用配置項目的提及

#### 4.3 更新配置範例檔案
- 檢查是否存在獨立的配置範例檔案
- 移除未使用的配置項目

### 階段 5：程式碼品質檢查（預估時間：20 分鐘）

#### 5.1 格式化和檢查
```bash
cargo fmt
cargo clippy -- -D warnings
```

#### 5.2 測試執行
```bash
cargo test
cargo test --release
scripts/test_parallel_stability.sh
```

#### 5.3 文件檢查
```bash
timeout 30 scripts/check_docs.sh
```

#### 5.4 覆蓋率檢查
```bash
scripts/check_coverage.sh -T
```

### 階段 6：最終更新和驗證（預估時間：20 分鐘）

#### 6.1 更新配置使用情況分析
- **檔案**: `docs/config-usage-analysis.md`
- 移除已刪除配置項目的條目
- 更新統計資料

#### 6.2 最終驗證
- 確保所有測試通過
- 確保文件一致性
- 檢查是否有任何遺漏的引用

## 風險評估與緩解策略

### 高風險項目
1. **意外的隱藏依賴**
   - **風險**: 某些配置項目可能在動態或反射程式碼中被使用
   - **緩解**: 進行全域搜尋，包括字串形式的配置名稱

2. **測試失敗**
   - **風險**: 移除配置項目可能導致測試失敗
   - **緩解**: 逐步移除並在每個步驟後執行測試

### 中風險項目
1. **文件不一致**
   - **風險**: 某些文件可能仍然引用已移除的配置項目
   - **緩解**: 使用自動化檢查腳本驗證文件一致性

## 成功標準

1. **功能完整性**: 所有現有功能正常運作
2. **測試通過率**: 100% 的測試通過
3. **文件一致性**: 所有文件與實際程式碼保持一致
4. **程式碼品質**: 通過所有 linting 和格式化檢查
5. **覆蓋率維持**: 測試覆蓋率不低於移除前的水準

## 預期結果

完成此 backlog 後，SubX 專案將：

1. **簡化的配置結構**: 移除 6 個未使用的配置項目
2. **更清潔的程式碼庫**: 減少不必要的程式碼複雜性
3. **一致的文件**: 所有文件準確反映實際的配置選項
4. **維護性提升**: 減少維護負擔和潛在的混淆點

## 後續工作

完成此 backlog 後，建議：

1. **建立配置項目生命週期管理流程**: 防止未來出現未使用的配置項目
2. **實施自動化檢查**: 在 CI/CD 中加入配置一致性檢查
3. **考慮配置項目的棄用策略**: 為未來可能需要移除的配置項目建立更平滑的過渡機制

## 時程估算

| 階段 | 預估時間 | 累計時間 |
|------|----------|----------|
| 準備工作 | 30 分鐘 | 30 分鐘 |
| 移除配置定義 | 20 分鐘 | 50 分鐘 |
| 更新測試檔案 | 40 分鐘 | 90 分鐘 |
| 更新文件範例 | 30 分鐘 | 120 分鐘 |
| 程式碼品質檢查 | 20 分鐘 | 140 分鐘 |
| 最終更新驗證 | 20 分鐘 | 160 分鐘 |

**總預估時間**: 約 2.5-3 小時

## 驗收標準

- [ ] 所有 5 個未使用的配置項目已從程式碼中移除
- [ ] 所有相關測試已更新或移除
- [ ] 所有文件已更新以反映新的配置結構
- [ ] `cargo test` 100% 通過
- [ ] `cargo clippy -- -D warnings` 無警告
- [ ] `scripts/check_docs.sh` 檢查通過
- [ ] `docs/config-usage-analysis.md` 已更新並準確反映當前狀態
- [ ] 測試覆蓋率維持在可接受水準
