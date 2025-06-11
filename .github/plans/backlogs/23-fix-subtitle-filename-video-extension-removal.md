# 修正字幕檔案重命名中的影片副檔名移除功能

## 問題描述

目前 `match` 命令執行後，字幕檔案的重命名會包含影片檔案的副檔名，例如：
- 影片檔案：`[Noumin Kanren no Skill][01][BDRIP][1080P][H264_FLAC2].mkv`
- 字幕檔案重命名後：`[Noumin Kanren no Skill][01][BDRIP][1080P][H264_FLAC2].mkv.tc.srt`

期望的行為是字幕檔案重命名時移除影片檔案的副檔名：
- 字幕檔案重命名後：`[Noumin Kanren no Skill][01][BDRIP][1080P][H264_FLAC2].tc.srt`

## 技術分析

### 問題根源
在 `src/core/matcher/engine.rs` 中的 `generate_subtitle_name` 函式直接使用 `video.name`，而 `video.name` 包含完整的檔案名稱（包括副檔名）。

```rust
fn generate_subtitle_name(&self, video: &MediaFile, subtitle: &MediaFile) -> String {
    let detector = LanguageDetector::new();
    if let Some(code) = detector.get_primary_language(&subtitle.path) {
        format!("{}.{}.{}", video.name, code, subtitle.extension)
    } else {
        format!("{}.{}", video.name, subtitle.extension)
    }
}
```

### 預期修正
需要從 `video.name` 中移除副檔名，只保留檔案的基本名稱：

```rust
fn generate_subtitle_name(&self, video: &MediaFile, subtitle: &MediaFile) -> String {
    let detector = LanguageDetector::new();
    let video_base_name = video.name
        .strip_suffix(&format!(".{}", video.extension))
        .unwrap_or(&video.name);
    
    if let Some(code) = detector.get_primary_language(&subtitle.path) {
        format!("{}.{}.{}", video_base_name, code, subtitle.extension)
    } else {
        format!("{}.{}", video_base_name, subtitle.extension)
    }
}
```

## 實作計畫

### 第一階段：程式碼分析與測試建立

#### 任務 1.1：深入分析現有程式碼結構
- **檔案路徑**：`src/core/matcher/engine.rs`
- **分析要點**：
  - 了解 `generate_subtitle_name` 函式的完整上下文
  - 確認 `MediaFile` 結構中 `name` 和 `extension` 欄位的內容格式
  - 檢視是否有其他地方也使用了類似的命名邏輯
- **預期結果**：完整理解現有的檔案命名機制

#### 任務 1.2：建立測試案例
- **檔案路徑**：`src/core/matcher/engine.rs` 中的測試模組
- **測試內容**：
  - 測試現有的檔案命名行為（確認當前問題）
  - 測試修正後的檔案命名行為（預期結果）
  - 測試邊界情況（無副檔名、特殊字元等）
- **測試案例**：
  ```rust
  #[test]
  fn test_generate_subtitle_name_removes_video_extension() {
      // 測試移除 .mkv 副檔名
      let video = MediaFile {
          name: "movie.mkv".to_string(),
          extension: "mkv".to_string(),
          // ... 其他欄位
      };
      let subtitle = MediaFile {
          name: "subtitle.srt".to_string(),
          extension: "srt".to_string(),
          // ... 其他欄位
      };
      
      let result = engine.generate_subtitle_name(&video, &subtitle);
      assert_eq!(result, "movie.srt");
  }
  
  #[test]
  fn test_generate_subtitle_name_with_language_removes_video_extension() {
      // 測試帶語言標籤的情況
      // 預期結果：movie.tc.srt 而不是 movie.mkv.tc.srt
  }
  
  #[test]
  fn test_generate_subtitle_name_edge_cases() {
      // 測試邊界情況：檔案名稱中包含多個點、無副檔名等
  }
  ```

### 第二階段：實作檔案命名修正

#### 任務 2.1：修正 `generate_subtitle_name` 函式
- **檔案路徑**：`src/core/matcher/engine.rs`
- **修正內容**：
  ```rust
  fn generate_subtitle_name(&self, video: &MediaFile, subtitle: &MediaFile) -> String {
      let detector = LanguageDetector::new();
      
      // 從影片檔案名稱中移除副檔名
      let video_base_name = video.name
          .strip_suffix(&format!(".{}", video.extension))
          .unwrap_or(&video.name);
      
      if let Some(code) = detector.get_primary_language(&subtitle.path) {
          format!("{}.{}.{}", video_base_name, code, subtitle.extension)
      } else {
          format!("{}.{}", video_base_name, subtitle.extension)
      }
  }
  ```

#### 任務 2.2：確保邊界情況處理
- **處理情況**：
  - 檔案名稱不包含副檔名的情況
  - 檔案名稱包含多個點的情況（例如：`movie.2023.1080p.mkv`）
  - 副檔名為空的情況
- **改進版本**：
  ```rust
  fn generate_subtitle_name(&self, video: &MediaFile, subtitle: &MediaFile) -> String {
      let detector = LanguageDetector::new();
      
      // 安全地移除影片檔案的副檔名
      let video_base_name = if !video.extension.is_empty() {
          video.name
              .strip_suffix(&format!(".{}", video.extension))
              .unwrap_or(&video.name)
      } else {
          &video.name
      };
      
      if let Some(code) = detector.get_primary_language(&subtitle.path) {
          format!("{}.{}.{}", video_base_name, code, subtitle.extension)
      } else {
          format!("{}.{}", video_base_name, subtitle.extension)
      }
  }
  ```

### 第三階段：測試驗證與品質保證

#### 任務 3.1：執行單元測試
- **命令**：`cargo test test_generate_subtitle_name`
- **驗證**：確保所有新增的測試案例都能通過
- **修正**：如果測試失敗，分析原因並修正實作

#### 任務 3.2：執行整合測試
- **測試檔案**：`tests/match_engine_id_integration_tests.rs`
- **測試場景**：
  - 使用真實的檔案結構進行測試
  - 驗證 dry-run 模式下的預期結果
  - 驗證實際執行模式下的檔案重命名結果

#### 任務 3.3：程式碼品質檢查
- **執行命令**：
  ```bash
  cargo fmt
  cargo clippy -- -D warnings
  ```
- **確保**：沒有格式問題和編譯警告

### 第四階段：文件更新與回歸測試

#### 任務 4.1：更新技術文件
- **檔案路徑**：`docs/tech-architecture.md`
- **更新內容**：
  - 記錄檔案命名邏輯的修正
  - 說明新的命名規則和行為

#### 任務 4.2：更新使用者文件
- **檔案路徑**：`README.zh-TW.md`
- **更新內容**：
  - 在使用範例中展示修正後的行為
  - 確保範例檔案命名符合新的邏輯

#### 任務 4.3：執行完整的回歸測試
- **測試範圍**：
  ```bash
  cargo test
  ```
- **確保**：所有既有的測試都能正常通過，沒有引入回歸問題

### 第五階段：效能測試與最佳化

#### 任務 5.1：效能基準測試
- **測試檔案**：`benches/file_id_generation_bench.rs`
- **測試內容**：
  - 確認字串操作的效能影響
  - 比較修正前後的效能差異

#### 任務 5.2：記憶體使用分析
- **分析**：確認新的字串操作不會造成額外的記憶體配置
- **最佳化**：如果有效能問題，考慮使用更有效率的字串處理方式

### 第六階段：部署前檢查

#### 任務 6.1：執行文件品質檢查
- **命令**：`timeout 20 scripts/check_docs.sh`
- **確保**：所有文件都符合品質標準

#### 任務 6.2：執行程式碼覆蓋率檢查
- **命令**：`scripts/check_coverage.sh -T`
- **確保**：新增的程式碼有適當的測試覆蓋率

#### 任務 6.3：建立提交
- **Git 提交**：
  ```bash
  git add .
  git commit --signoff --no-gpg-sign --author="🤖 GitHub Copilot <github-copilot[bot]@users.noreply.github.com>" -m "fix(matcher): remove video file extension from subtitle filename

  - Modified generate_subtitle_name function to strip video file extension
  - Added comprehensive test cases for edge cases
  - Updated documentation to reflect the naming behavior change
  - Ensures subtitle files no longer contain video file extensions in their names"
  ```

## 風險評估

### 高風險區域
1. **檔案重命名邏輯**：修改核心的檔案命名函式可能影響其他功能
2. **邊界情況處理**：特殊的檔案命名格式可能導致意外的行為

### 緩解策略
1. **完整的測試覆蓋**：建立全面的測試案例覆蓋各種情況
2. **分階段實作**：先實作基本功能，再處理邊界情況
3. **回歸測試**：確保現有功能不受影響

## 驗收標準

### 功能需求
- [x] 字幕檔案重命名時不包含影片檔案的副檔名
- [x] 保持語言標籤的正確位置
- [x] 處理各種邊界情況（無副檔名、多點檔名等）

### 品質需求
- [x] 所有單元測試通過
- [x] 所有整合測試通過
- [x] 程式碼品質檢查通過（clippy, fmt）
- [x] 文件品質檢查通過
- [x] 程式碼覆蓋率符合標準

### 相容性需求
- [x] 不破壞現有的匹配功能
- [x] 不影響其他檔案操作功能
- [x] 保持 API 相容性

## 預期成果

修正完成後，字幕檔案的重命名行為將符合使用者期望：

**修正前**：
```
[Noumin Kanren no Skill][01][BDRIP][1080P][H264_FLAC2].mkv.tc.srt
[Yozakura-san Chi no Daisakusen][01][BDRIP][1080P][H264_AC3].mkv.tc.ass
```

**修正後**：
```
[Noumin Kanren no Skill][01][BDRIP][1080P][H264_FLAC2].tc.srt
[Yozakura-san Chi no Daisakusen][01][BDRIP][1080P][H264_AC3].tc.ass
```

這個修正將使字幕檔案的命名更加乾淨和符合使用者的期望，同時保持所有現有功能的完整性。
