# Bug Fix #07: WEBVTT 格式測試增強

## 問題描述

目前 `src/core/formats/manager.rs` 缺少針對 WEBVTT 格式的具體測試，特別是需要增加一個測試來驗證：
- WEBVTT 格式的字幕（包含三句字幕）
- 通過 `parse_auto()` 方法解析後
- 第一句字幕的內容能正確提取

這個測試對於確保 WEBVTT 格式解析器的正確性非常重要。

## 問題分析

### 現狀分析
- 格式管理器可能有基本的測試，但缺少 WEBVTT 格式的詳細測試
- 沒有驗證解析後資料結構的正確性
- 缺少多句字幕的解析測試

### 根本原因
1. **測試覆蓋不完整**：WEBVTT 格式的測試用例不足
2. **驗證深度不夠**：沒有驗證解析後的具體內容
3. **邊界情況未覆蓋**：沒有測試多句字幕的情況

### 測試目標
- 確保 WEBVTT 格式能正確識別
- 驗證 `parse_auto()` 能正確解析 WEBVTT 檔案
- 確認解析後的字幕內容準確性

## 技術方案

### 架構設計
1. **WEBVTT 測試資料建立**
   - 建立標準的 WEBVTT 格式測試檔案
   - 包含三句字幕內容

2. **解析測試實作**
   - 測試自動格式檢測功能
   - 驗證解析後的資料結構

3. **內容驗證機制**
   - 檢查第一句字幕的具體內容
   - 驗證時間軸和文字的正確性

### WEBVTT 格式標準
```webvtt
WEBVTT

00:00:01.000 --> 00:00:03.000
第一句字幕內容

00:00:04.000 --> 00:00:06.000  
第二句字幕內容

00:00:07.000 --> 00:00:09.000
第三句字幕內容
```

## 實作步驟

### 第一階段：檢查現有測試結構
1. **分析現有測試**
   - 檔案：`src/core/formats/manager.rs`
   - 了解現有的測試架構和模式

2. **確認缺失的測試**
   - 檢查 WEBVTT 相關測試的覆蓋程度
   - 識別需要補強的部分

### 第二階段：建立 WEBVTT 測試資料
1. **建立測試檔案內容**
   - 設計標準的 WEBVTT 格式內容
   - 確保符合 WEBVTT 規範

2. **設計測試用例結構**
   - 規劃測試的輸入和預期輸出
   - 考慮邊界情況和錯誤處理

### 第三階段：實作測試函式
1. **撰寫測試函式**
   - 實作 WEBVTT 解析測試
   - 驗證第一句字幕內容

2. **整合到現有測試套件**
   - 確保與現有測試的一致性
   - 維持測試結構的整潔

## 詳細實作指南

### 步驟 1：建立 WEBVTT 測試資料
```rust
// 在 src/core/formats/manager.rs 的測試中增加
const SAMPLE_WEBVTT_THREE_LINES: &str = "WEBVTT

1
00:00:01.000 --> 00:00:03.000
第一句字幕內容

2
00:00:04.000 --> 00:00:06.000
第二句字幕內容

3
00:00:07.000 --> 00:00:09.000
第三句字幕內容
";
```

### 步驟 2：實作測試函式
```rust
// 在 src/core/formats/manager.rs 的測試模組中增加
#[test]
fn test_webvtt_parse_auto_first_subtitle_content() {
    let mgr = FormatManager::new();
    
    // 解析包含三句字幕的 WEBVTT 內容
    let subtitle = mgr
        .parse_auto(SAMPLE_WEBVTT_THREE_LINES)
        .expect("Failed to parse WEBVTT with auto detection");
    
    // 驗證格式識別正確
    assert_eq!(subtitle.format, SubtitleFormatType::Vtt);
    
    // 驗證解析出三句字幕
    assert_eq!(subtitle.entries.len(), 3);
    
    // 驗證第一句字幕的內容
    let first_entry = &subtitle.entries[0];
    assert_eq!(first_entry.text, "第一句字幕內容");
    assert_eq!(first_entry.index, 1);
    
    // 驗證第一句字幕的時間軸
    assert_eq!(first_entry.start_time, Duration::from_millis(1000)); // 00:00:01.000
    assert_eq!(first_entry.end_time, Duration::from_millis(3000));   // 00:00:03.000
    
    // 驗證其他句字幕也正確解析
    assert_eq!(subtitle.entries[1].text, "第二句字幕內容");
    assert_eq!(subtitle.entries[2].text, "第三句字幕內容");
}
```

### 步驟 3：增加邊界情況測試
```rust
#[test]
fn test_webvtt_parse_auto_with_complex_content() {
    let complex_webvtt = "WEBVTT

NOTE 這是註解，應該被忽略

STYLE
::cue {
  background-color: black;
  color: white;
}

1
00:00:01.000 --> 00:00:03.500
第一句字幕內容
包含多行文字

2  
00:00:04.200 --> 00:00:07.800
第二句字幕內容

cue-with-id
00:00:08.000 --> 00:00:10.000
第三句字幕內容
";

    let mgr = FormatManager::new();
    let subtitle = mgr
        .parse_auto(complex_webvtt)
        .expect("Failed to parse complex WEBVTT");
    
    // 驗證正確解析三句字幕（忽略 NOTE 和 STYLE）
    assert_eq!(subtitle.entries.len(), 3);
    
    // 驗證第一句字幕內容（包含多行）
    let first_entry = &subtitle.entries[0];
    assert_eq!(first_entry.text, "第一句字幕內容\n包含多行文字");
    
    // 驗證時間解析正確
    assert_eq!(first_entry.start_time, Duration::from_millis(1000));
    assert_eq!(first_entry.end_time, Duration::from_millis(3500));
}
```

### 步驟 4：測試錯誤處理
```rust
#[test]
fn test_webvtt_parse_auto_invalid_format() {
    let mgr = FormatManager::new();
    
    // 測試不是 WEBVTT 格式的內容
    let invalid_content = "這不是有效的字幕格式";
    let result = mgr.parse_auto(invalid_content);
    assert!(result.is_err());
    
    // 測試缺少 WEBVTT 標頭的內容
    let no_header = "1\n00:00:01.000 --> 00:00:03.000\n字幕內容";
    let result = mgr.parse_auto(no_header);
    // 這應該不會被識別為 WEBVTT 格式
    assert!(result.is_err() || result.unwrap().format != SubtitleFormatType::Vtt);
}
```

### 步驟 5：完整的測試實作
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::formats::SubtitleFormatType;
    use std::time::Duration;

    // ...existing constants...

    const SAMPLE_WEBVTT_THREE_LINES: &str = "WEBVTT

1
00:00:01.000 --> 00:00:03.000
第一句字幕內容

2
00:00:04.000 --> 00:00:06.000
第二句字幕內容

3
00:00:07.000 --> 00:00:09.000
第三句字幕內容
";

    // ...existing tests...

    #[test]
    fn test_webvtt_parse_auto_first_subtitle_content() {
        let mgr = FormatManager::new();
        
        // 使用 parse_auto 解析 WEBVTT 格式的字幕
        let subtitle = mgr
            .parse_auto(SAMPLE_WEBVTT_THREE_LINES)
            .expect("Failed to parse WEBVTT format using parse_auto");
        
        // 驗證自動檢測正確識別為 WEBVTT 格式
        assert_eq!(
            subtitle.format, 
            SubtitleFormatType::Vtt,
            "Auto detection should identify as WEBVTT format"
        );
        
        // 驗證解析出正確數量的字幕條目
        assert_eq!(
            subtitle.entries.len(), 
            3,
            "Should parse exactly 3 subtitle entries"
        );
        
        // 驗證第一句字幕的內容
        let first_entry = &subtitle.entries[0];
        assert_eq!(
            first_entry.text, 
            "第一句字幕內容",
            "First subtitle content should be correctly parsed"
        );
        
        // 驗證第一句字幕的索引
        assert_eq!(
            first_entry.index, 
            1,
            "First subtitle should have index 1"
        );
        
        // 驗證第一句字幕的開始時間 (00:00:01.000)
        assert_eq!(
            first_entry.start_time,
            Duration::from_millis(1000),
            "First subtitle start time should be 1 second"
        );
        
        // 驗證第一句字幕的結束時間 (00:00:03.000)
        assert_eq!(
            first_entry.end_time,
            Duration::from_millis(3000),
            "First subtitle end time should be 3 seconds"
        );
        
        // 額外驗證：確保其他字幕也正確解析
        assert_eq!(subtitle.entries[1].text, "第二句字幕內容");
        assert_eq!(subtitle.entries[2].text, "第三句字幕內容");
    }
}
```

## 測試計劃

### 單元測試策略
1. **基本功能測試**
   - 測試 WEBVTT 格式的自動檢測
   - 測試三句字幕的正確解析
   - 驗證第一句字幕內容的準確性

2. **邊界情況測試**
   - 包含 NOTE 和 STYLE 的 WEBVTT 檔案
   - 多行字幕內容
   - 帶有 cue ID 的字幕

3. **錯誤處理測試**
   - 無效的 WEBVTT 格式
   - 缺少標頭的內容
   - 時間格式錯誤

### 測試資料設計
```rust
// 基本的三句字幕測試資料
const BASIC_THREE_LINES: &str = "WEBVTT

1
00:00:01.000 --> 00:00:03.000
第一句字幕內容

2
00:00:04.000 --> 00:00:06.000
第二句字幕內容

3
00:00:07.000 --> 00:00:09.000
第三句字幕內容
";

// 複雜格式測試資料
const COMPLEX_WEBVTT: &str = "WEBVTT

NOTE 測試註解

STYLE
::cue {
  color: white;
}

subtitle-1
00:00:01.500 --> 00:00:04.200
第一句字幕內容
這是多行文字

subtitle-2
00:00:05.000 --> 00:00:07.500
第二句字幕內容

subtitle-3
00:00:08.000 --> 00:00:10.000
第三句字幕內容
";
```

## 品質保證

### 程式碼品質檢查
```bash
# 執行特定的格式管理器測試
cargo test formats::manager

# 執行 WEBVTT 相關測試
cargo test vtt

# 檢查測試覆蓋率
cargo llvm-cov --all-features --workspace --html

# 格式化程式碼
cargo fmt

# 靜態分析
cargo clippy -- -D warnings
```

### 測試驗證重點
1. **解析正確性**：確保 WEBVTT 內容被正確解析
2. **內容準確性**：驗證第一句字幕的文字和時間
3. **格式識別**：確認自動檢測能正確識別 WEBVTT
4. **邊界處理**：測試各種複雜情況

## 預期成果

### 測試覆蓋增強
- 新增針對 WEBVTT 格式的具體測試
- 驗證 `parse_auto()` 方法的正確性
- 確保第一句字幕內容的準確解析

### 測試執行結果
```bash
$ cargo test test_webvtt_parse_auto_first_subtitle_content
running 1 test
test core::formats::manager::tests::test_webvtt_parse_auto_first_subtitle_content ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 功能驗證
- 確保 WEBVTT 格式檔案能正確被 `parse_auto()` 識別
- 驗證三句字幕都能正確解析
- 確認第一句字幕的內容、時間軸都準確無誤

## 額外測試案例

### 進階 WEBVTT 功能測試
```rust
#[test]
fn test_webvtt_with_metadata_and_styling() {
    let webvtt_with_features = "WEBVTT
Kind: subtitles
Language: zh-TW

NOTE 這是測試檔案

STYLE
::cue {
  background-color: rgba(0,0,0,0.8);
  color: white;
}

1
00:00:01.000 --> 00:00:03.000 align:middle line:90%
第一句字幕內容

2
00:00:04.000 --> 00:00:06.000
第二句字幕內容

3
00:00:07.000 --> 00:00:09.000
第三句字幕內容
";

    let mgr = FormatManager::new();
    let subtitle = mgr.parse_auto(webvtt_with_features)
        .expect("Failed to parse WEBVTT with metadata");
    
    assert_eq!(subtitle.format, SubtitleFormatType::Vtt);
    assert_eq!(subtitle.entries.len(), 3);
    assert_eq!(subtitle.entries[0].text, "第一句字幕內容");
}
```

## 注意事項

### 測試設計原則
- **明確的測試目標**：專注於驗證第一句字幕內容
- **完整的驗證**：包括文字、時間、索引等所有欄位
- **清晰的錯誤訊息**：提供有用的斷言訊息

### 維護性考量
- 測試資料應該易於理解和修改
- 測試函式應該專注且獨立
- 考慮未來格式變更的相容性

### 效能考量
- 測試應該快速執行
- 避免過大的測試資料
- 考慮記憶體使用效率

### 相容性確認
- 確保新增的測試不影響現有測試
- 驗證測試在不同環境下的穩定性
- 保持與現有測試風格的一致性

## 驗收標準

- [ ] 新增 WEBVTT 格式的三句字幕測試
- [ ] 測試使用 `parse_auto()` 方法進行解析
- [ ] 驗證第一句字幕的內容正確
- [ ] 驗證第一句字幕的時間軸正確
- [ ] 驗證格式自動檢測功能
- [ ] 所有新增測試通過
- [ ] 程式碼品質檢查無警告
- [ ] 測試覆蓋率符合要求
- [ ] 不影響現有測試的正常執行
- [ ] 測試資料符合 WEBVTT 標準規範
