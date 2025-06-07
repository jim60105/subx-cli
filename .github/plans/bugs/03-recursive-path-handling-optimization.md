# Bug Fix #03: 遞迴模式下的路徑處理優化

## 問題描述

在執行 `subx-cli match --recursive` 時，存在以下問題：
1. 提供給 LLM 的檔案資訊缺少資料夾子路徑上下文
2. 重命名時沒有正確處理資料夾子路徑
3. 遞迴搜尋的檔案在不同子目錄下時，匹配結果可能不準確

這會導致在複雜的目錄結構中，檔案匹配的準確度降低，且重命名後的檔案路徑不正確。

## 問題分析

### 現狀分析
- 目前遞迴模式下的檔案發現會找到不同子目錄的檔案
- LLM 接收的提示中只包含檔案名稱，缺少目錄上下文
- 重命名邏輯沒有考慮原始檔案的完整路徑結構

### 根本原因
1. **檔案資訊不完整**：LLM 無法獲得足夠的路徑上下文來做出準確判斷
2. **路徑處理邏輯缺陷**：重命名時沒有保持原始的目錄結構
3. **遞迴匹配策略問題**：沒有考慮目錄層級的影響

### 影響範圍
- 多層目錄結構的專案
- 包含季集資訊的影片目錄
- 語言分類的字幕目錄

## 技術方案

### 架構設計
1. **增強檔案資訊結構**
   - 在提供給 LLM 的提示中包含相對路徑資訊
   - 保持目錄結構的上下文

2. **路徑感知的匹配引擎**
   - 考慮檔案的目錄位置進行匹配
   - 優先匹配同一目錄下的檔案

3. **智慧重命名邏輯**
   - 保持原始的目錄結構
   - 正確處理子路徑的重命名

### 資料結構設計
```rust
// 增強的檔案資訊結構
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub relative_path: String,  // 相對於搜尋根目錄的路徑
    pub full_path: PathBuf,
    pub directory: String,      // 所在目錄名稱
}
```

## 實作步驟

### 第一階段：增強檔案發現引擎
1. **修改檔案發現邏輯**
   - 檔案：`src/core/matcher/file_discovery.rs`
   - 保留完整的路徑資訊
   - 記錄相對路徑和目錄層級

2. **更新檔案資訊結構**
   - 擴展 `FileInfo` 結構體
   - 增加路徑相關欄位

### 第二階段：改善 LLM 提示生成
1. **修改提示建構器**
   - 檔案：`src/core/matcher/ai_matcher.rs`
   - 在提示中包含目錄結構資訊
   - 提供更豐富的上下文

2. **優化匹配策略**
   - 增加目錄親和性權重
   - 優先考慮同目錄下的檔案

### 第三階段：完善重命名邏輯
1. **更新重命名引擎**
   - 檔案：`src/commands/match_command.rs`
   - 正確處理子路徑的重命名
   - 保持目錄結構完整性

2. **增加路徑驗證**
   - 確保重命名後的路徑有效
   - 處理路徑衝突情況

## 詳細實作指南

### 步驟 1：增強檔案資訊結構
```rust
// src/core/matcher/mod.rs
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// 檔案名稱（不含路徑）
    pub name: String,
    /// 相對於搜尋根目錄的路徑
    pub relative_path: String,
    /// 完整的絕對路徑
    pub full_path: PathBuf,
    /// 所在目錄名稱
    pub directory: String,
    /// 目錄深度（相對於根目錄）
    pub depth: usize,
}

impl FileInfo {
    pub fn new(full_path: PathBuf, root_path: &Path) -> Result<Self> {
        let relative_path = full_path
            .strip_prefix(root_path)?
            .to_string_lossy()
            .to_string();
        
        let name = full_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
            
        let directory = full_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
            
        let depth = relative_path.matches(std::path::MAIN_SEPARATOR).count();
        
        Ok(Self {
            name,
            relative_path,
            full_path,
            directory,
            depth,
        })
    }
}
```

### 步驟 2：改善 LLM 提示生成
```rust
// src/core/matcher/ai_matcher.rs
impl AiMatcher {
    fn build_enhanced_prompt(&self, videos: &[FileInfo], subtitles: &[FileInfo]) -> String {
        let mut prompt = String::from("請協助匹配以下影片檔案和字幕檔案。");
        prompt.push_str("請特別注意檔案的目錄結構和相對路徑。\n\n");
        
        prompt.push_str("影片檔案：\n");
        for video in videos {
            prompt.push_str(&format!(
                "- {} (路徑: {}, 目錄: {})\n", 
                video.name, 
                video.relative_path,
                video.directory
            ));
        }
        
        prompt.push_str("\n字幕檔案：\n");
        for subtitle in subtitles {
            prompt.push_str(&format!(
                "- {} (路徑: {}, 目錄: {})\n", 
                subtitle.name, 
                subtitle.relative_path,
                subtitle.directory
            ));
        }
        
        prompt.push_str("\n匹配規則：\n");
        prompt.push_str("1. 優先匹配同一目錄下的檔案\n");
        prompt.push_str("2. 考慮檔案名稱的相似度\n");
        prompt.push_str("3. 注意季集資訊的對應\n");
        prompt.push_str("4. 保持目錄結構的一致性\n");
        
        prompt
    }
}
```

### 步驟 3：實作路徑感知匹配
```rust
// src/core/matcher/ai_matcher.rs
impl AiMatcher {
    async fn match_with_path_awareness(&self, videos: &[FileInfo], subtitles: &[FileInfo]) -> Result<Vec<MatchPair>> {
        // 按目錄分組
        let video_groups = self.group_by_directory(videos);
        let subtitle_groups = self.group_by_directory(subtitles);
        
        let mut results = Vec::new();
        
        // 優先處理同目錄下的匹配
        for (dir, dir_videos) in video_groups {
            if let Some(dir_subtitles) = subtitle_groups.get(&dir) {
                let matches = self.match_in_directory(&dir_videos, &dir_subtitles).await?;
                results.extend(matches);
            }
        }
        
        // 處理跨目錄的匹配
        // ...
        
        Ok(results)
    }
    
    fn group_by_directory(&self, files: &[FileInfo]) -> HashMap<String, Vec<FileInfo>> {
        let mut groups = HashMap::new();
        for file in files {
            groups.entry(file.directory.clone())
                  .or_insert_with(Vec::new)
                  .push(file.clone());
        }
        groups
    }
}
```

### 步驟 4：完善重命名邏輯
```rust
// src/commands/match_command.rs
impl MatchCommand {
    fn execute_rename(&self, match_pair: &MatchPair) -> Result<()> {
        let video_path = &match_pair.video.full_path;
        let subtitle_path = &match_pair.subtitle.full_path;
        
        // 建構新的字幕檔案路徑
        let new_subtitle_path = self.build_new_subtitle_path(video_path, subtitle_path)?;
        
        // 確保目標目錄存在
        if let Some(parent) = new_subtitle_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // 執行重命名
        std::fs::rename(subtitle_path, &new_subtitle_path)?;
        
        println!("✅ 重命名: {} -> {}", 
                 subtitle_path.display(), 
                 new_subtitle_path.display());
        
        Ok(())
    }
    
    fn build_new_subtitle_path(&self, video_path: &Path, subtitle_path: &Path) -> Result<PathBuf> {
        let video_stem = video_path.file_stem().unwrap().to_string_lossy();
        let subtitle_ext = subtitle_path.extension().unwrap().to_string_lossy();
        
        // 保持字幕檔案的目錄結構
        let subtitle_dir = subtitle_path.parent().unwrap();
        
        // 處理語言標記
        let new_name = if let Some(lang_code) = self.extract_language_code(&subtitle_path) {
            format!("{}.{}.{}", video_stem, lang_code, subtitle_ext)
        } else {
            format!("{}.{}", video_stem, subtitle_ext)
        };
        
        Ok(subtitle_dir.join(new_name))
    }
}
```

## 測試計劃

### 單元測試
1. **檔案資訊測試**
   - 測試 FileInfo 結構體的建立
   - 測試相對路徑計算
   - 測試目錄層級計算

2. **提示生成測試**
   - 測試增強的提示包含路徑資訊
   - 測試不同目錄結構的提示生成

### 整合測試
1. **遞迴匹配測試**
   - 建立多層目錄結構測試資料
   - 測試同目錄優先匹配
   - 測試跨目錄匹配

2. **重命名測試**
   - 測試子路徑的正確處理
   - 測試目錄結構保持

### 測試資料結構
```
test_data/
├── season1/
│   ├── episode1.mp4
│   ├── episode2.mp4
│   └── subtitles/
│       ├── ep1.srt
│       └── ep2.srt
├── season2/
│   ├── s02e01.mp4
│   └── subs/
│       └── s02e01.zh.ass
└── movies/
    ├── movie1.mp4
    └── movie1.tc.srt
```

### 測試用例
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_file_info_creation() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let file_path = root.join("season1").join("episode1.mp4");
        
        let info = FileInfo::new(file_path, root).unwrap();
        assert_eq!(info.name, "episode1.mp4");
        assert_eq!(info.relative_path, "season1/episode1.mp4");
        assert_eq!(info.directory, "season1");
        assert_eq!(info.depth, 1);
    }

    #[tokio::test]
    async fn test_recursive_matching() {
        // 建立測試目錄結構
        // 執行遞迴匹配
        // 驗證結果
    }
}
```

## 品質保證

### 程式碼品質檢查
```bash
# 格式化程式碼
cargo fmt

# 靜態分析
cargo clippy -- -D warnings

# 執行測試
cargo test test_recursive

# 整合測試
cargo test --test integration_tests
```

### 效能考量
- 避免對大量檔案進行不必要的路徑操作
- 快取目錄分組結果
- 優化遞迴搜尋的效率

## 預期成果

### 功能改善
- LLM 獲得更豐富的檔案上下文資訊
- 遞迴模式下的匹配準確度提升
- 重命名時正確保持目錄結構

### 使用案例改善
```bash
# 執行前的目錄結構
project/
├── season1/
│   ├── episode1.mp4
│   └── subs/
│       └── ep1.srt
└── season2/
    ├── episode1.mp4
    └── subs/
        └── ep1.srt

# 執行命令
subx-cli match --recursive project/

# 執行後的目錄結構
project/
├── season1/
│   ├── episode1.mp4
│   └── subs/
│       └── episode1.srt  # 正確重命名
└── season2/
    ├── episode1.mp4
    └── subs/
        └── episode1.srt  # 正確重命名，沒有衝突
```

## 額外功能

### 進階匹配策略
1. **目錄親和性評分**
   - 同目錄檔案給予更高權重
   - 考慮目錄名稱的語義相關性

2. **層級感知匹配**
   - 優先匹配相同層級的檔案
   - 避免跨度過大的匹配

### 智慧路徑處理
1. **語言子目錄處理**
   - 識別語言相關的子目錄
   - 正確處理多語言字幕結構

2. **季集資訊保持**
   - 識別季集相關的目錄結構
   - 保持季集資訊的一致性

## 注意事項

### 相容性
- 確保與現有的非遞迴模式相容
- 保持 API 介面的一致性

### 錯誤處理
- 處理路徑過長的情況
- 處理特殊字元和 Unicode 路徑
- 處理權限不足的目錄

### 安全性
- 驗證路徑的合法性
- 防止路徑遍歷攻擊
- 確保重命名操作的安全性

## 驗收標準

- [ ] LLM 提示包含完整的路徑上下文資訊
- [ ] 遞迴模式下優先匹配同目錄檔案
- [ ] 重命名時正確保持目錄結構
- [ ] 支援多層目錄結構的正確處理
- [ ] 跨目錄匹配時考慮路徑關係
- [ ] 所有路徑處理測試通過
- [ ] 程式碼品質檢查無警告
- [ ] 與現有功能保持相容性
