# 36 - Cargo套件大小最佳化：排除不必要的檔案

## 概述

本計劃旨在透過配置 `Cargo.toml` 的 `package.exclude` 欄位來排除專案中不必要的檔案，以減少發布套件的大小。目前專案中的 `assets` 資料夾佔用約 12MB 空間，`.github` 資料夾佔用約 2.5MB 空間，`tests` 資料夾佔用約 376KB 空間，這些檔案在發布的 crate 中並非必要，應予以排除。

## 問題描述

### 當前狀況
- `assets` 資料夾包含：
  - `SubX - The Subtitle Revolution.mp4` (7.5MB) - 示範影片
  - `SubX - The Subtitle Revolution.mp3` (4.4MB) - 示範音訊
  - `SubX - The Subtitle Revolution.srt` (3.4KB) - 示範字幕檔
  - `logo.svg` (7.7KB) - 專案標誌
- `.github` 資料夾包含：
  - 工作流程設定檔
  - 專案規劃文件
  - 提示檔案
  - 問題和 PR 模板
- `tests` 資料夾包含：
  - 整合測試檔案（約 40 個檔案，376KB）
  - 測試輔助模組
  - 測試數據檔案

### 影響評估
- 總計約 15MB 的不必要檔案會被包含在發布的 crate 中
- 增加下載時間和儲存空間需求
- 對終端使用者無實際價值

## 技術需求

### 主要目標
1. 配置 `Cargo.toml` 的 `package.exclude` 欄位
2. 排除 `assets/` 資料夾中的多媒體檔案
3. 排除 `.github/` 資料夾
4. 排除 `tests/` 資料夾（整合測試）
5. 保留必要的專案檔案
6. 驗證排除設定的有效性

### 技術規格
- 使用 Cargo 的 `package.exclude` 功能
- 遵循 glob 模式語法
- 確保不影響本地開發和測試
- 維持 CI/CD 流程正常運作

## 實作計劃

### 階段 1：分析現有檔案結構
**預估時間：30分鐘**

1. **檔案大小分析**
   ```bash
   # 檢查各資料夾大小
   du -sh assets/ .github/ target/ docs/
   
   # 列出大檔案
   find . -type f -size +1M -exec ls -lh {} \;
   ```

2. **建立排除清單**
   - 識別應排除的檔案類型
   - 確認哪些檔案對 crate 功能必要
   - 記錄檔案大小和用途

### 階段 2：配置 Cargo.toml
**預估時間：45分鐘**

1. **修改 `Cargo.toml`**
   在 `[package]` 區段中添加 `exclude` 欄位：
   ```toml
   [package]
   name = "subx-cli"
   version = "0.8.0"
   # ... 其他現有欄位 ...
   exclude = [
       "assets/",
       ".github/",
       "tests/",
       "target/",
       "*.mp4",
       "*.mp3",
       "*.mov",
       "*.avi",
       "*.mkv",
       "plans/",
       "scripts/test_*.sh",
       "benches/",
       "**/*.log",
       "**/*.tmp",
       "**/.DS_Store",
       "Cargo.lock"  # 考慮排除，因為執行檔不需要
   ]
   ```

2. **排除模式說明**
   - `assets/` - 排除整個 assets 資料夾
   - `.github/` - 排除 GitHub 特定檔案
   - `tests/` - 排除整合測試（使用者不需要執行 crate 的測試）
   - `target/` - 排除建置輸出
   - `*.mp4`, `*.mp3` 等 - 排除多媒體檔案
   - `plans/` - 排除專案規劃文件
   - `scripts/test_*.sh` - 排除測試腳本
   - `benches/` - 排除效能測試
   - 臨時檔案和系統檔案

### 階段 3：測試和驗證
**預估時間：30分鐘**

1. **本地測試**
   ```bash
   # 執行本地測試確保功能正常
   cargo test
   
   # 檢查程式碼品質
   timeout 30 scripts/quality_check.sh
   
   # 驗證建置正常
   cargo build --release
   ```

2. **套件大小驗證**
   ```bash
   # 建立測試套件
   cargo package --dry-run
   
   # 檢查套件內容
   cargo package --list
   
   # 比較套件大小（之前和之後）
   ls -lh target/package/
   ```

3. **功能測試**
   ```bash
   # 確保 CLI 功能正常
   ./target/release/subx-cli --help
   ./target/release/subx-cli --version
   
   # 測試基本命令
   ./target/release/subx-cli config get
   ```

### 階段 4：文件更新
**預估時間：20分鐘**

1. **更新 README.md**
   - 如果有提及 assets 檔案的部分，需要更新說明
   - 確保安裝和使用說明仍然準確

2. **更新 CHANGELOG.md**
   - 記錄套件大小最佳化的變更
   - 說明排除的檔案類型

## 測試策略

### 自動化測試
1. **套件完整性測試**
   ```bash
   # 驗證套件可以正常建立
   cargo package --dry-run
   
   # 檢查套件內容
   cargo package --list | grep -E "(assets|\.github|tests)" && echo "ERROR: Excluded files found" || echo "OK: No excluded files"
   ```

2. **功能測試**
   ```bash
   # 確保所有測試通過
   cargo test --all-features
   
   # 整合測試
   cargo test --test cli_integration_tests
   ```

### 手動測試
1. **套件大小比較**
   - 記錄變更前後的套件大小
   - 驗證減少的檔案大小符合預期

2. **本地安裝測試**
   ```bash
   # 從本地套件安裝
   cargo install --path .
   
   # 測試安裝的版本
   subx-cli --version
   ```

## 效能影響

### 正面影響
- **套件大小減少**：預計減少約 15MB
- **下載時間縮短**：特別是在慢速網路環境下
- **儲存空間節省**：減少 crate 快取佔用空間

### 潛在風險
- **文件遺失**：確保重要的使用者文件不被排除
- **CI/CD 影響**：驗證 GitHub Actions 不依賴排除的檔案
- **開發體驗**：確保本地開發不受影響

## 品質保證

### 程式碼品質檢查
```bash
# 格式化程式碼
cargo fmt

# Clippy 檢查
cargo clippy -- -D warnings

# 品質檢查腳本
timeout 30 scripts/quality_check.sh
```

### 測試覆蓋率
```bash
# 檢查測試覆蓋率
scripts/check_coverage.sh -T
```

## 部署準備

### 發布前檢查清單
- [ ] 所有測試通過
- [ ] 套件大小確實減少
- [ ] 功能完整性驗證
- [ ] 文件更新完成
- [ ] CI/CD 流程正常

### 發布後驗證
- [ ] 從 crates.io 下載大小合理
- [ ] 安裝和使用正常
- [ ] 社群回饋正面

## 相關資源

### 官方文件
- [Cargo Book - The Manifest Format](https://doc.rust-lang.org/cargo/reference/manifest.html#the-exclude-and-include-fields)
- [Cargo Book - Publishing](https://doc.rust-lang.org/cargo/reference/publishing.html)

### 最佳實務
- 排除所有不必要的檔案
- 使用 glob 模式進行靈活配置
- 定期檢查套件內容
- 監控套件大小變化

## 成功標準

### 主要指標
- 套件大小減少至少 14.5MB
- 所有現有功能正常運作
- CI/CD 流程無中斷
- 無功能迴歸

### 次要指標
- 本地開發體驗無影響
- 文件準確性維持
- 社群滿意度

## 風險管控

### 高風險項目
1. **意外排除重要檔案**
   - 緩解措施：仔細檢查排除清單
   - 應急計劃：快速回復配置

2. **CI/CD 流程中斷**
   - 緩解措施：在多個環境測試
   - 應急計劃：暫時恢復原始配置

### 中風險項目
1. **文件不一致**
   - 緩解措施：同步更新相關文件
   - 監控措施：定期檢查文件準確性

## 後續最佳化

### 未來改進方向
1. **動態排除**：根據建置類型動態調整
2. **自動化監控**：定期檢查套件大小
3. **進階壓縮**：考慮額外的壓縮選項

### 維護計劃
- 定期檢查排除清單的有效性
- 監控新增檔案是否需要排除
- 根據使用者回饋調整配置

---

**預估完成時間：2-3小時**  
**優先級：中**  
**複雜度：低**  
**影響範圍：套件發布、下載體驗**
