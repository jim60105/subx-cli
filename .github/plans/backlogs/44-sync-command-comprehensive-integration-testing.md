# Backlog 44: Sync 命令綜合整合測試實作

## 概述

本計劃旨在為 `sync` 子命令創建一個完整的整合測試套件，確保所有文檔中記錄的參數組合都得到適當的測試覆蓋。當前的測試雖然涵蓋了一些基本功能，但缺乏對所有參數組合的系統性測試。

## 背景分析

### 當前已有的測試
根據代碼庫分析，目前已存在以下 sync 相關測試檔案：

1. **基礎功能測試**
   - `sync_command_comprehensive_tests.rs` - 基本同步命令功能
   - `sync_cli_integration_tests.rs` - CLI 參數解析
   - `sync_new_architecture_tests.rs` - 新架構測試
   - `sync_argument_flexibility_tests.rs` - 參數靈活性測試

2. **特定功能測試**
   - `sync_max_offset_integration_tests.rs` - 最大偏移量測試
   - `sync_first_sentence_offset_integration_tests.rs` - 首句偏移測試
   - `sync_engine_integration_tests.rs` - 同步引擎測試

3. **命令模組測試**
   - `commands/sync_command_tests.rs` - 同步命令模組測試
   - `commands/sync_command_manual_offset_tests.rs` - 手動偏移測試

### 支援的參數組合（文檔中）

根據 README.md，sync 命令支援以下使用模式：

```bash
# 自動 VAD 同步（需要音視頻檔案）
subx-cli sync video.mp4 subtitle.srt

# 手動同步（僅需字幕檔案）
subx-cli sync --offset 2.5 subtitle.srt

# VAD 自訂敏感度
subx-cli sync --vad-sensitivity 0.8 video.mp4 subtitle.srt

# 批次處理模式（處理整個目錄）
subx-cli sync --batch /path/to/media/folder

# 使用 -i 參數進行多目錄批次處理
subx-cli sync -i ./movies_directory --batch

# 批次處理與遞歸目錄掃描
subx-cli sync -i ./movies_directory --batch --recursive

# 進階：多目錄與特定同步方法
subx-cli sync -i ./movies1 -i ./movies2 -i ./tv_shows --recursive --batch --method vad

# 批次模式與詳細輸出及試運行
subx-cli sync -i ./media --batch --recursive --dry-run --verbose
subx-cli sync movie.mkv
subx-cli sync subtitles.ass
subx-cli sync -b media_folder
```

### 測試覆蓋缺口分析

通過對現有測試的分析，發現以下測試覆蓋缺口：

#### 1. 參數組合缺口
- **批次模式 + 遞歸 + 多輸入源**：缺少完整的組合測試
- **VAD 參數組合**：缺少不同 VAD 敏感度設定的測試
- **輸出選項組合**：缺少 `--dry-run` + `--verbose` 的組合測試
- **方法選擇組合**：缺少 `--method vad` 與其他參數的組合測試

#### 2. 輸入路徑處理缺口
- **多 -i 參數**：缺少多個 `-i` 參數的測試
- **-i 與位置參數混合**：缺少混合使用的測試
- **檔案與目錄混合輸入**：缺少混合輸入類型的測試

#### 3. 邊界條件缺口
- **空目錄處理**：缺少空目錄的測試
- **無效路徑處理**：缺少路徑錯誤處理的測試
- **權限問題**：缺少檔案權限相關的測試

#### 4. 整合測試缺口
- **端到端工作流程**：缺少完整工作流程的測試
- **配置整合**：缺少與配置系統的整合測試
- **錯誤恢復**：缺少錯誤恢復機制的測試

## 技術設計

### 測試架構設計

```rust
// 新的整合測試檔案結構
tests/
├── sync_comprehensive_integration_tests.rs          // 主要整合測試檔案
├── sync_parameter_combinations_tests.rs             // 參數組合測試
├── sync_input_path_handling_tests.rs               // 輸入路徑處理測試
├── sync_batch_processing_integration_tests.rs      // 批次處理整合測試
└── sync_edge_cases_integration_tests.rs            // 邊界條件測試
```

### 測試類別設計

#### 1. 基本參數組合測試
```rust
#[tokio::test]
async fn test_sync_basic_parameter_combinations() {
    // 測試所有基本參數組合
}

#[tokio::test]
async fn test_sync_manual_offset_combinations() {
    // 測試手動偏移的各種組合
}

#[tokio::test]
async fn test_sync_vad_parameter_combinations() {
    // 測試 VAD 相關參數組合
}
```

#### 2. 輸入路徑處理測試
```rust
#[tokio::test]
async fn test_sync_multiple_input_paths() {
    // 測試多個 -i 參數
}

#[tokio::test]
async fn test_sync_mixed_input_types() {
    // 測試檔案與目錄混合輸入
}

#[tokio::test]
async fn test_sync_positional_and_input_paths() {
    // 測試位置參數與 -i 參數混合使用
}
```

#### 3. 批次處理整合測試
```rust
#[tokio::test]  
async fn test_sync_batch_recursive_combinations() {
    // 測試批次 + 遞歸組合
}

#[tokio::test]
async fn test_sync_batch_multiple_directories() {
    // 測試批次處理多個目錄
}

#[tokio::test]
async fn test_sync_batch_with_method_selection() {
    // 測試批次處理與方法選擇
}
```

#### 4. 輸出選項測試
```rust
#[tokio::test]
async fn test_sync_dry_run_verbose_combinations() {
    // 測試試運行與詳細輸出組合
}

#[tokio::test]
async fn test_sync_output_file_handling() {
    // 測試輸出檔案處理
}

#[tokio::test]
async fn test_sync_force_overwrite_scenarios() {
    // 測試強制覆寫場景
}
```

## 測試資料設計

### 測試資料來源

> **重要：本計劃禁止實作測試資料產生器，所有測試必須僅使用現有的 assets/ 目錄下的檔案。**

請僅使用下列檔案作為測試資料來源：

- `assets/SubX - The Subtitle Revolution.mp3`
- `assets/SubX - The Subtitle Revolution.mp4`
- `assets/SubX - The Subtitle Revolution.srt`

請勿在測試中產生或寫入新的影音或字幕檔案，所有測試必須以這些現有檔案為基礎進行組合與驗證。

### 測試目錄結構

```
assets/
├── SubX - The Subtitle Revolution.mp3
├── SubX - The Subtitle Revolution.mp4
└── SubX - The Subtitle Revolution.srt
```

---

## 其他注意事項

- **所有測試必須僅使用 assets/ 目錄下的現有檔案，不得產生、寫入或依賴其他測試資料。**
- **請勿實作任何測試資料產生器或自動產生檔案的邏輯。**
- **如需不同檔案組合，請以 assets/ 目錄下的檔案進行排列組合。**
- 將測試資料複製到 Tempdir 中進行測試，確保測試環境的隔離性。

---

## 實作細节

### 第一階段：基礎整合測試檔案

創建 `tests/sync_comprehensive_integration_tests.rs`，包含：

1. **基本功能驗證**
   - 單檔案同步測試
   - 手動偏移測試
   - VAD 同步測試

2. **參數驗證測試**
   - 參數解析正確性
   - 無效參數組合處理
   - 預設值應用測試

### 第二階段：參數組合測試

創建 `tests/sync_parameter_combinations_tests.rs`，涵蓋：

1. **所有文檔記錄的組合**
   ```rust
   // 測試: subx-cli sync video.mp4 subtitle.srt
   #[tokio::test]
   async fn test_basic_video_subtitle_sync()
   
   // 測試: subx-cli sync --offset 2.5 subtitle.srt  
   #[tokio::test]
   async fn test_manual_offset_sync()
   
   // 測試: subx-cli sync --vad-sensitivity 0.8 video.mp4 subtitle.srt
   #[tokio::test] 
   async fn test_vad_sensitivity_sync()
   
   // 測試: subx-cli sync --batch /path/to/media/folder
   #[tokio::test]
   async fn test_batch_directory_sync()
   
   // 測試: subx-cli sync -i ./movies_directory --batch
   #[tokio::test]
   async fn test_batch_input_path_sync()
   
   // 測試: subx-cli sync -i ./movies_directory --batch --recursive  
   #[tokio::test]
   async fn test_batch_recursive_sync()
   
   // 測試: subx-cli sync -i ./movies1 -i ./movies2 -i ./tv_shows --recursive --batch --method vad
   #[tokio::test]
   async fn test_multiple_input_batch_recursive_vad_sync()
   
   // 測試: subx-cli sync -i ./media --batch --recursive --dry-run --verbose
   #[tokio::test]
   async fn test_batch_recursive_dry_run_verbose_sync()
   ```

2. **邊界組合測試**
   ```rust
   // 測試最小參數組合
   #[tokio::test]
   async fn test_minimal_parameter_combinations()
   
   // 測試最大參數組合
   #[tokio::test]
   async fn test_maximal_parameter_combinations()
   
   // 測試衝突參數處理
   #[tokio::test]
   async fn test_conflicting_parameter_handling()
   ```

### 第三階段：輸入路徑處理測試

創建 `tests/sync_input_path_handling_tests.rs`，包含：

1. **多輸入源測試**
   ```rust
   // 測試多個 -i 參數
   #[tokio::test]
   async fn test_multiple_input_flag_usage()
   
   // 測試 -i 與位置參數混合
   #[tokio::test]
   async fn test_input_flag_with_positional_args()
   
   // 測試檔案與目錄混合輸入
   #[tokio::test]
   async fn test_mixed_file_directory_inputs()
   ```

2. **路徑解析測試**
   ```rust
   // 測試相對路徑處理
   #[tokio::test]
   async fn test_relative_path_handling()
   
   // 測試絕對路徑處理
   #[tokio::test]
   async fn test_absolute_path_handling()
   
   // 測試路徑標準化
   #[tokio::test]
   async fn test_path_normalization()
   ```

### 第四階段：批次處理整合測試

創建 `tests/sync_batch_processing_integration_tests.rs`，涵蓋：

1. **批次模式測試**
   ```rust
   // 測試基本批次處理
   #[tokio::test]
   async fn test_basic_batch_processing()
   
   // 測試遞歸批次處理
   #[tokio::test]
   async fn test_recursive_batch_processing()
   
   // 測試大型目錄批次處理
   #[tokio::test]
   async fn test_large_directory_batch_processing()
   ```

2. **批次處理與其他選項組合**
   ```rust
   // 測試批次 + 試運行
   #[tokio::test]
   async fn test_batch_dry_run_combination()
   
   // 測試批次 + 詳細輸出
   #[tokio::test]
   async fn test_batch_verbose_combination()
   
   // 測試批次 + 方法選擇
   #[tokio::test]
   async fn test_batch_method_selection_combination()
   ```

### 第五階段：邊界條件與錯誤處理測試

創建 `tests/sync_edge_cases_integration_tests.rs`，包含：

1. **邊界條件測試**
   ```rust
   // 測試空目錄處理
   #[tokio::test]
   async fn test_empty_directory_handling()
   
   // 測試不存在的路徑
   #[tokio::test]
   async fn test_nonexistent_path_handling()
   
   // 測試檔案權限問題
   #[tokio::test]
   async fn test_file_permission_handling()
   ```

2. **錯誤恢復測試**
   ```rust
   // 測試部分失敗恢復
   #[tokio::test]
   async fn test_partial_failure_recovery()
   
   // 測試中斷處理
   #[tokio::test]
   async fn test_interruption_handling()
   
   // 測試資源清理
   #[tokio::test]
   async fn test_resource_cleanup()
   ```

## 測試執行策略

### 測試分組
1. **快速測試組** (< 1分鐘)
   - 基本參數解析測試
   - 單檔案處理測試
   - 試運行模式測試

2. **中等測試組** (1-5分鐘)
   - 小規模批次處理測試
   - 參數組合測試
   - 輸入路徑處理測試

3. **耗時測試組** (5-15分鐘)
   - 大規模批次處理測試
   - 完整整合測試
   - 效能測試

### 並行執行設計
```rust
// 使用 tokio::test 支援並行執行
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_batch_processing_parallel() {
    // 批次處理測試
}
```

### 測試工具整合
```bash
# 執行完整測試套件
cargo nextest run sync_comprehensive_integration_tests

# 執行特定測試組
cargo nextest run --test sync_parameter_combinations_tests

# 執行效能測試
cargo nextest run --test sync_batch_processing_integration_tests --test-threads 1
```

## 品質保證

### 測試覆蓋率目標
- **參數組合覆蓋率**: 100% (所有文檔記錄的組合)
- **代碼路徑覆蓋率**: > 95% (sync_command 模組)
- **邊界條件覆蓋率**: > 90% (錯誤處理路徑)

### 測試穩定性要求
- **測試成功率**: > 99.5% (在 CI 環境中)
- **測試執行時間**: < 15分鐘 (完整測試套件)
- **記憶體使用**: < 500MB (單一測試進程)

### 程式碼品質檢查
```bash
# 執行 clippy 檢查
cargo clippy --tests -- -D warnings

# 執行格式檢查
cargo fmt --check

# 執行測試覆蓋率檢查
timeout 240 scripts/check_coverage.sh -T
```
## 驗收標準

### 功能性驗收標準
1. **完整參數組合覆蓋**
   - ✅ 所有 README 中記錄的 sync 命令用法都有對應測試
   - ✅ 每個參數組合都能正確執行並產生預期結果
   - ✅ 無效參數組合能正確回報錯誤

2. **輸入處理完整性**
   - ✅ 支援單一檔案、多檔案、目錄、混合輸入的所有組合
   - ✅ `-i` 參數與位置參數的所有組合都能正確處理
   - ✅ 遞歸處理在各種目錄結構下都能正常運作

### 技術性驗收標準
1. **測試品質**
   - ✅ 所有測試都使用 `TestConfigService` 進行配置管理
   - ✅ 測試之間保持完全隔離，無相互影響
   - ✅ 測試命名清晰，易於理解和維護

3. **程式碼品質**
   - ✅ 通過 `cargo clippy -- -D warnings` 檢查
   - ✅ 通過 `cargo fmt --check` 格式檢查
   - ✅ 測試覆蓋率 > 95% (sync_command 相關程式碼)

### 維護性驗收標準
1. **測試可讀性**
   - ✅ 每個測試都有清晰的文檔說明其測試目的
   - ✅ 測試失敗時能提供有用的錯誤訊息
   - ✅ 測試程式碼結構清晰，易於擴展

2. **測試可靠性**
   - ✅ 測試在不同環境下都能穩定執行
   - ✅ 測試結果具有重現性
   - ✅ 測試能正確清理產生的暫存檔案

## 總結

本計劃通過系統性的方法，為 sync 命令建立完整的整合測試覆蓋。通過分階段實作、明確的驗收標準和風險緩解措施，確保測試品質和專案成功。實作完成後，sync 命令將擁有業界標準的測試覆蓋，為後續功能開發和維護提供堅實基礎。
