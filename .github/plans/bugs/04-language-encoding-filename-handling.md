# Bug Fix #04: 語言編碼檔名處理

## 問題描述

當檔名或路徑中包含語言編碼時，系統需要將語言資訊附加到字幕檔名後面，但目前的實作未正確處理這種情況。

具體問題場景：
- 影片檔案：`movie01.mp4`
- 字幕檔案：`tc/subtitle01.ass` → 應重命名為 `tc/movie01.tc.ass`
- 字幕檔案：`subtitle01.sc.ass` → 應重命名為 `movie01.sc.ass`

目前系統沒有：
1. 識別路徑中的語言編碼資訊
2. 正確提取檔名中的語言標記
3. 將語言標記附加到新檔名中

## 問題分析

### 現狀分析
- 重命名邏輯只考慮影片檔案的基本名稱
- 沒有解析字幕檔案路徑中的語言資訊
- 缺乏語言編碼的標準化處理

### 根本原因
1. **語言識別缺失**：沒有語言編碼識別機制
2. **檔名建構邏輯不完整**：重命名時未考慮語言標記
3. **路徑分析不足**：未分析目錄名稱中的語言資訊

### 語言編碼模式
常見的語言編碼出現位置：
- 目錄名稱：`tc/`, `sc/`, `en/`, `jp/`
- 檔名中：`.tc.`, `.sc.`, `.en.`, `.jp.`
- 檔名結尾：`_tc`, `_sc`, `_en`, `_jp`

## 技術方案

### 架構設計
1. **語言編碼識別引擎**
   - 建立語言編碼字典和規則
   - 支援多種識別模式（目錄、檔名、後綴）

2. **智慧檔名建構器**
   - 整合語言資訊到新檔名
   - 處理複雜的語言標記組合

3. **路徑分析器**
   - 分析完整路徑中的語言資訊
   - 提取和標準化語言編碼

### 語言編碼標準
建立統一的語言編碼對應表：
```
tc/繁中 -> tc
sc/簡中 -> sc  
en/英文 -> en
jp/日文 -> jp
kr/韓文 -> kr
```

## 實作步驟

### 第一階段：建立語言識別模組
1. **建立語言編碼模組**
   - 檔案：`src/core/language.rs`
   - 定義語言編碼字典和識別規則

2. **實作識別邏輯**
   - 目錄名稱識別
   - 檔名模式識別
   - 優先級規則

### 第二階段：增強路徑分析
1. **擴展檔案資訊結構**
   - 在 `FileInfo` 中增加語言資訊欄位
   - 記錄語言編碼來源

2. **實作路徑解析**
   - 分析完整路徑的語言資訊
   - 處理多重語言標記

### 第三階段：更新重命名邏輯
1. **修改檔名建構器**
   - 檔案：`src/commands/match_command.rs`
   - 整合語言資訊到新檔名

2. **處理語言標記優先級**
   - 路徑中的語言資訊優先
   - 檔名中的語言資訊次之

## 詳細實作指南

### 步驟 1：建立語言識別模組
```rust
// src/core/language.rs
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum LanguageSource {
    Directory,  // 來自目錄名稱
    Filename,   // 來自檔案名稱
    Extension,  // 來自副檔名前
}

#[derive(Debug, Clone)]
pub struct LanguageInfo {
    pub code: String,
    pub source: LanguageSource,
    pub confidence: f32,  // 識別信心度
}

pub struct LanguageDetector {
    // 語言編碼字典
    language_codes: HashMap<String, String>,
    // 目錄名稱模式
    directory_patterns: Vec<String>,
    // 檔名模式
    filename_patterns: Vec<regex::Regex>,
}

impl LanguageDetector {
    pub fn new() -> Self {
        let mut language_codes = HashMap::new();
        language_codes.insert("tc".to_string(), "tc".to_string());
        language_codes.insert("繁中".to_string(), "tc".to_string());
        language_codes.insert("繁體".to_string(), "tc".to_string());
        language_codes.insert("cht".to_string(), "tc".to_string());
        
        language_codes.insert("sc".to_string(), "sc".to_string());
        language_codes.insert("簡中".to_string(), "sc".to_string());
        language_codes.insert("簡體".to_string(), "sc".to_string());
        language_codes.insert("chs".to_string(), "sc".to_string());
        
        language_codes.insert("en".to_string(), "en".to_string());
        language_codes.insert("英文".to_string(), "en".to_string());
        language_codes.insert("english".to_string(), "en".to_string());
        
        // 建立檔名模式正規表達式
        let filename_patterns = vec![
            regex::Regex::new(r"\.([a-z]{2,3})\.").unwrap(),  // .tc., .sc., .en.
            regex::Regex::new(r"_([a-z]{2,3})\.").unwrap(),   // _tc., _sc., _en.
            regex::Regex::new(r"-([a-z]{2,3})\.").unwrap(),   // -tc., -sc., -en.
        ];
        
        Self {
            language_codes,
            directory_patterns: vec!["tc".to_string(), "sc".to_string(), "en".to_string()],
            filename_patterns,
        }
    }
    
    pub fn detect_from_path(&self, path: &Path) -> Option<LanguageInfo> {
        // 優先檢查目錄名稱
        if let Some(lang) = self.detect_from_directory(path) {
            return Some(lang);
        }
        
        // 檢查檔案名稱
        if let Some(lang) = self.detect_from_filename(path) {
            return Some(lang);
        }
        
        None
    }
    
    fn detect_from_directory(&self, path: &Path) -> Option<LanguageInfo> {
        for component in path.components() {
            if let Some(dir_name) = component.as_os_str().to_str() {
                let dir_lower = dir_name.to_lowercase();
                if let Some(code) = self.language_codes.get(&dir_lower) {
                    return Some(LanguageInfo {
                        code: code.clone(),
                        source: LanguageSource::Directory,
                        confidence: 0.9,
                    });
                }
            }
        }
        None
    }
    
    fn detect_from_filename(&self, path: &Path) -> Option<LanguageInfo> {
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            for pattern in &self.filename_patterns {
                if let Some(captures) = pattern.captures(filename) {
                    if let Some(lang_match) = captures.get(1) {
                        let lang_code = lang_match.as_str();
                        if let Some(code) = self.language_codes.get(lang_code) {
                            return Some(LanguageInfo {
                                code: code.clone(),
                                source: LanguageSource::Filename,
                                confidence: 0.8,
                            });
                        }
                    }
                }
            }
        }
        None
    }
}
```

### 步驟 2：擴展檔案資訊結構
```rust
// src/core/matcher/mod.rs
use crate::core::language::{LanguageInfo, LanguageDetector};

#[derive(Debug, Clone)]
pub struct FileInfo {
    // ...existing fields...
    pub language: Option<LanguageInfo>,
}

impl FileInfo {
    pub fn new(full_path: PathBuf, root_path: &Path) -> Result<Self> {
        // ...existing code...
        
        let detector = LanguageDetector::new();
        let language = detector.detect_from_path(&full_path);
        
        Ok(Self {
            // ...existing fields...
            language,
        })
    }
}
```

### 步驟 3：更新重命名邏輯
```rust
// src/commands/match_command.rs
use crate::core::language::LanguageDetector;

impl MatchCommand {
    fn build_new_subtitle_path(&self, video_path: &Path, subtitle_path: &Path) -> Result<PathBuf> {
        let video_stem = video_path.file_stem().unwrap().to_string_lossy();
        let subtitle_ext = subtitle_path.extension().unwrap().to_string_lossy();
        
        // 保持字幕檔案的目錄結構
        let subtitle_dir = subtitle_path.parent().unwrap();
        
        // 檢測語言編碼
        let detector = LanguageDetector::new();
        let language_info = detector.detect_from_path(subtitle_path);
        
        // 建構新檔名
        let new_name = match language_info {
            Some(lang) => {
                // 包含語言編碼的檔名
                format!("{}.{}.{}", video_stem, lang.code, subtitle_ext)
            },
            None => {
                // 不包含語言編碼的檔名
                format!("{}.{}", video_stem, subtitle_ext)
            }
        };
        
        Ok(subtitle_dir.join(new_name))
    }
}
```

### 步驟 4：處理複雜語言標記
```rust
// src/core/language.rs
impl LanguageDetector {
    pub fn detect_all_languages(&self, path: &Path) -> Vec<LanguageInfo> {
        let mut languages = Vec::new();
        
        // 收集所有可能的語言標記
        if let Some(dir_lang) = self.detect_from_directory(path) {
            languages.push(dir_lang);
        }
        
        if let Some(file_lang) = self.detect_from_filename(path) {
            languages.push(file_lang);
        }
        
        // 去重並排序（按信心度）
        languages.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        languages.dedup_by(|a, b| a.code == b.code);
        
        languages
    }
    
    pub fn get_primary_language(&self, path: &Path) -> Option<String> {
        let languages = self.detect_all_languages(path);
        languages.first().map(|lang| lang.code.clone())
    }
}
```

## 測試計劃

### 單元測試
1. **語言識別測試**
   - 測試目錄名稱識別
   - 測試檔名模式識別
   - 測試信心度評分

2. **檔名建構測試**
   - 測試各種語言編碼組合
   - 測試邊界情況

### 整合測試
1. **端到端語言處理測試**
   - 建立包含語言編碼的測試目錄
   - 驗證重命名結果

### 測試資料結構
```
test_data/
├── tc/
│   └── subtitle01.ass
├── sc/
│   └── subtitle02.srt
├── subtitle03.tc.vtt
├── subtitle04_en.ass
└── movie01.mp4
```

### 測試用例
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directory_language_detection() {
        let detector = LanguageDetector::new();
        let path = Path::new("tc/subtitle.srt");
        
        let lang = detector.detect_from_path(path).unwrap();
        assert_eq!(lang.code, "tc");
        assert_eq!(lang.source, LanguageSource::Directory);
    }

    #[test]
    fn test_filename_language_detection() {
        let detector = LanguageDetector::new();
        let path = Path::new("subtitle.tc.srt");
        
        let lang = detector.detect_from_path(path).unwrap();
        assert_eq!(lang.code, "tc");
        assert_eq!(lang.source, LanguageSource::Filename);
    }

    #[test]
    fn test_new_subtitle_name_with_language() {
        let command = MatchCommand::new();
        let video_path = Path::new("movie01.mp4");
        let subtitle_path = Path::new("tc/subtitle01.ass");
        
        let new_path = command.build_new_subtitle_path(video_path, subtitle_path).unwrap();
        assert_eq!(new_path.file_name().unwrap(), "movie01.tc.ass");
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

# 執行語言模組測試
cargo test language

# 執行整合測試
cargo test test_language_integration
```

### 效能考量
- 語言識別應該快速且輕量
- 避免重複解析相同路徑
- 快取常用的語言編碼結果

## 預期成果

### 功能改善
- 正確識別和處理檔案路徑中的語言編碼
- 重命名時保留語言資訊
- 支援多種語言編碼格式

### 使用案例展示
```bash
# 案例 1：目錄中的語言編碼
# 輸入檔案：
# - movie01.mp4
# - tc/subtitle01.ass
# 
# 執行後：
# - movie01.mp4
# - tc/movie01.tc.ass

# 案例 2：檔名中的語言編碼
# 輸入檔案：
# - movie02.mp4
# - subtitle02.sc.ass
#
# 執行後：
# - movie02.mp4  
# - movie02.sc.ass

# 案例 3：複合語言編碼
# 輸入檔案：
# - movie03.mp4
# - tc/subtitle03.en.srt
#
# 執行後：
# - movie03.mp4
# - tc/movie03.tc.srt  # 目錄語言優先
```

## 額外功能

### 進階語言處理
1. **多語言支援**
   - 同時處理多種語言編碼
   - 支援複合語言標記

2. **語言標準化**
   - 統一不同形式的語言編碼
   - 支援自定義語言對應表

### 智慧語言識別
1. **上下文分析**
   - 考慮檔案內容的語言特徵
   - 基於檔案大小和編碼推測語言

2. **學習機制**
   - 記錄使用者的語言偏好
   - 適應專案特定的語言模式

## 注意事項

### 相容性
- 確保不影響無語言編碼的檔案處理
- 保持與現有重命名邏輯的相容性

### 錯誤處理
- 處理無法識別的語言編碼
- 處理衝突的語言標記
- 處理無效的檔名字元

### 國際化
- 支援不同地區的語言編碼習慣
- 考慮 Unicode 字元的處理

## 驗收標準

- [ ] 正確識別目錄名稱中的語言編碼
- [ ] 正確識別檔名中的語言標記
- [ ] 重命名時將語言資訊附加到檔名
- [ ] 處理多重語言標記的優先級
- [ ] 支援常見的語言編碼格式
- [ ] 所有語言處理測試通過
- [ ] 程式碼品質檢查無警告
- [ ] 不影響現有功能的正常運作
