# Bug Fix #02: 美化對映表格顯示

## 問題描述

當執行 `subx-cli match` 命令後，檔案對映結果的顯示格式不夠美觀和直觀，無論是 dry-run 模式還是實際執行模式，都缺乏清晰的表格展示。

目前的問題包括：
- 對映結果顯示不整齊
- 缺乏表格框線和對齊
- 資訊層次不清楚
- 不容易快速識別對映關係

## 問題分析

### 現狀分析
- 目前 match 命令的結果輸出比較簡陋
- 檔案對映關係不容易一眼看出
- 缺乏視覺化的表格展示
- 不同模式（dry-run/實際執行）的顯示不一致

### 根本原因
- CLI UI 模組缺乏專門的表格顯示功能
- 對映結果的資料結構未考慮顯示需求
- 沒有統一的格式化標準

## 技術方案

### 架構設計
1. **表格顯示引擎**
   - 建立專門的表格顯示模組
   - 支援動態欄位寬度調整
   - 提供美觀的框線和對齊

2. **對映結果格式化**
   - 設計統一的對映結果顯示格式
   - 支援不同的顯示模式
   - 增加顏色和圖示增強可讀性

### 依賴套件
選用 `tabled` 或 `prettytable-rs` 來實現表格顯示功能。

## 實作步驟

### 第一階段：增加表格顯示依賴
1. **更新 Cargo.toml**
   - 增加表格顯示相關依賴
   - 選擇合適的表格渲染函式庫

2. **建立表格顯示模組**
   - 檔案：`src/cli/table.rs`
   - 實作表格渲染功能

### 第二階段：設計對映結果結構
1. **定義顯示資料結構**
   - 檔案：`src/cli/ui.rs`
   - 建立專門用於顯示的資料結構

2. **實作格式化邏輯**
   - 將內部對映結果轉換為顯示格式
   - 處理路徑截斷和美化

### 第三階段：更新 match 命令顯示
1. **修改 match 命令**
   - 檔案：`src/commands/match_command.rs`
   - 使用新的表格顯示功能

2. **統一顯示格式**
   - 確保 dry-run 和實際執行模式的一致性
   - 增加必要的狀態指示

## 詳細實作指南

### 步驟 1：更新依賴
```toml
# Cargo.toml
[dependencies]
tabled = "0.15"
colored = "2.0"
```

### 步驟 2：建立表格顯示模組
```rust
// src/cli/table.rs
use tabled::{Table, Tabled, Style, Modify, object::Rows, Alignment};
use colored::*;

#[derive(Tabled)]
pub struct MatchDisplayRow {
    #[tabled(rename = "狀態")]
    pub status: String,
    #[tabled(rename = "影片檔案")]
    pub video_file: String,
    #[tabled(rename = "字幕檔案")]
    pub subtitle_file: String,
    #[tabled(rename = "新檔名")]
    pub new_name: String,
}

pub fn create_match_table(rows: Vec<MatchDisplayRow>) -> String {
    let mut table = Table::new(rows);
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()));
    
    table.to_string()
}
```

### 步驟 3：實作顯示邏輯
```rust
// src/cli/ui.rs
use crate::cli::table::{MatchDisplayRow, create_match_table};
use colored::*;

pub fn display_match_results(results: &[MatchResult], is_dry_run: bool) {
    if results.is_empty() {
        println!("{}", "沒有找到匹配的檔案對映".yellow());
        return;
    }

    println!("\n{}", "📋 檔案對映結果".bold().blue());
    if is_dry_run {
        println!("{}", "🔍 預覽模式 (不會實際修改檔案)".yellow());
    }
    println!();

    let display_rows: Vec<MatchDisplayRow> = results
        .iter()
        .map(|result| MatchDisplayRow {
            status: if is_dry_run {
                "🔍 預覽".yellow().to_string()
            } else {
                "✅ 完成".green().to_string()
            },
            video_file: truncate_path(&result.video_file, 30),
            subtitle_file: truncate_path(&result.subtitle_file, 30),
            new_name: truncate_path(&result.new_name, 30),
        })
        .collect();

    println!("{}", create_match_table(display_rows));
    
    println!("\n{}", format!("總共處理了 {} 個檔案對映", results.len()).bold());
}

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}
```

### 步驟 4：更新 match 命令
```rust
// src/commands/match_command.rs
use crate::cli::ui::display_match_results;

impl MatchCommand {
    pub async fn execute(&self) -> Result<()> {
        // ... 現有的匹配邏輯 ...

        // 顯示結果
        display_match_results(&results, self.args.dry_run);

        if !self.args.dry_run {
            // 實際執行重命名操作
            // ...
        }

        Ok(())
    }
}
```

## 測試計劃

### 單元測試
1. **表格顯示測試**
   - 測試不同數量的對映結果顯示
   - 測試路徑截斷功能
   - 測試空結果的處理

2. **格式化測試**
   - 測試不同長度檔名的顯示
   - 測試特殊字元的處理

### 視覺測試
1. **顯示效果測試**
   - 在終端中執行實際命令查看效果
   - 測試不同終端寬度下的顯示

### 測試用例
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_table_display() {
        let rows = vec![
            MatchDisplayRow {
                status: "✅ 完成".to_string(),
                video_file: "movie1.mp4".to_string(),
                subtitle_file: "subtitle1.srt".to_string(),
                new_name: "movie1.srt".to_string(),
            },
        ];
        
        let table = create_match_table(rows);
        assert!(table.contains("movie1.mp4"));
    }

    #[test]
    fn test_path_truncation() {
        let long_path = "/very/long/path/to/some/movie/file.mp4";
        let truncated = truncate_path(long_path, 20);
        assert!(truncated.len() <= 20);
        assert!(truncated.starts_with("..."));
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
cargo test

# 檢查依賴更新
cargo update
```

### 效能考量
- 表格渲染不應顯著影響命令執行時間
- 大量檔案對映時考慮分頁顯示

## 預期成果

### 功能改善
- 提供美觀的表格式對映結果顯示
- 增強使用者體驗和可讀性
- 統一 dry-run 和實際執行的顯示格式

### 顯示範例
```
📋 檔案對映結果
🔍 預覽模式 (不會實際修改檔案)

╭────────┬─────────────────────┬─────────────────────┬─────────────────────╮
│ 狀態   │ 影片檔案            │ 字幕檔案            │ 新檔名              │
├────────┼─────────────────────┼─────────────────────┼─────────────────────┤
│ 🔍 預覽 │ movie1.mp4          │ subtitle1.srt       │ movie1.srt          │
│ 🔍 預覽 │ movie2.mp4          │ subtitle2.ass       │ movie2.ass          │
│ 🔍 預覽 │ ...very/long/pa.mp4 │ ...long/subtitle.vtt│ ...long/movie.vtt   │
╰────────┴─────────────────────┴─────────────────────┴─────────────────────╯

總共處理了 3 個檔案對映
```

## 額外功能

### 進階顯示選項
1. **詳細模式**
   - 顯示完整路徑
   - 增加檔案大小資訊

2. **簡潔模式**
   - 只顯示核心對映資訊
   - 適合腳本化使用

### 互動功能
1. **確認提示**
   - 在非 dry-run 模式下顯示確認提示
   - 讓使用者確認對映結果

## 注意事項

### 相容性
- 確保在不同終端環境下的顯示效果
- 考慮終端寬度限制

### 國際化
- 支援不同語言的欄位標題
- 考慮文字寬度計算的準確性

### 錯誤處理
- 當表格渲染失敗時提供降級顯示
- 處理特殊字元和 Unicode 字符

## 驗收標準

- [ ] 對映結果以美觀的表格形式顯示
- [ ] 支援 dry-run 和實際執行模式的區別顯示
- [ ] 路徑過長時正確截斷並顯示
- [ ] 表格對齊和框線顯示正確
- [ ] 增加適當的顏色和圖示增強可讀性
- [ ] 所有測試通過
- [ ] 程式碼品質檢查無警告
- [ ] 在不同終端環境下顯示正常
