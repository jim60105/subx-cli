# Bug Fix #06: Sync 命令參數簡化

## 問題描述

目前 `subx-cli sync` 命令包含 `--method` 參數來區分手動偏移和自動偏移，但實際上可以根據是否提供 `offset` 參數來自動判斷同步方法，不需要額外的 `--method` 參數。

具體問題：
- `--method` 參數是多餘的，增加了使用者的認知負擔
- 可以透過檢查 `--offset` 參數的存在與否來自動決定同步方法
- 簡化後的介面更直觀和易用

## 問題分析

### 現狀分析
- 目前需要同時指定 `--method` 和 `--offset` 參數
- 兩個參數之間存在邏輯依賴關係
- 使用者需要理解兩種不同的同步方法

### 根本原因
1. **參數設計冗餘**：`--method` 參數可以由 `--offset` 的存在與否推導出來
2. **介面複雜性**：過多的參數選項影響使用體驗
3. **邏輯判斷分散**：同步方法的選擇邏輯沒有集中處理

### 預期改善
```bash
# 原本的用法（複雜）
subx-cli sync video.mp4 subtitle.srt --method manual --offset 2.5
subx-cli sync video.mp4 subtitle.srt --method auto

# 簡化後的用法
subx-cli sync video.mp4 subtitle.srt --offset 2.5  # 手動偏移
subx-cli sync video.mp4 subtitle.srt               # 自動偏移
```

## 技術方案

### 架構設計
1. **簡化參數結構**
   - 移除 `--method` 參數
   - 保留 `--offset` 參數作為可選項

2. **智慧方法選擇**
   - 有 `--offset` 參數時使用手動偏移
   - 沒有 `--offset` 參數時使用自動偏移

3. **向後相容性處理**
   - 提供平滑的移轉路徑
   - 顯示適當的棄用警告

### 邏輯流程
```
檢查命令參數
├── 有 --offset 參數
│   └── 使用手動偏移模式
└── 無 --offset 參數  
    └── 使用自動偏移模式
```

## 實作步驟

### 第一階段：更新參數定義
1. **修改 sync 參數結構**
   - 檔案：`src/cli/sync_args.rs`
   - 移除 `method` 欄位
   - 保持 `offset` 為可選參數

2. **更新說明文件**
   - 更新參數的說明和範例
   - 確保使用者了解新的使用方式

### 第二階段：修改同步邏輯
1. **更新同步命令**
   - 檔案：`src/commands/sync_command.rs`
   - 實作自動方法選擇邏輯
   - 移除對 `method` 欄位的依賴

2. **簡化方法選擇**
   - 根據 `offset` 參數的存在自動選擇方法
   - 提供清晰的執行回饋

### 第三階段：向後相容性處理
1. **棄用警告機制**
   - 如果偵測到舊的使用方式，顯示警告
   - 提供移轉指導

2. **文件更新**
   - 更新所有相關文件和範例
   - 確保一致性

## 詳細實作指南

### 步驟 1：更新參數結構
```rust
// src/cli/sync_args.rs
use clap::Args;

#[derive(Debug, Args)]
pub struct SyncArgs {
    /// 影片檔案路徑
    #[arg(value_name = "VIDEO")]
    pub video: String,
    
    /// 字幕檔案路徑
    #[arg(value_name = "SUBTITLE")]
    pub subtitle: String,
    
    /// 手動時間偏移（秒）
    /// 如果提供此參數，將使用手動偏移模式
    /// 如果不提供，將使用自動同步模式
    #[arg(long, short = 'o', value_name = "SECONDS", 
          help = "手動時間偏移（秒）。提供此參數時使用手動偏移，否則使用自動同步")]
    pub offset: Option<f64>,
    
    /// 輸出檔案路徑（可選）
    #[arg(long, short = 'O', value_name = "OUTPUT")]
    pub output: Option<String>,
    
    /// 預覽模式，不實際修改檔案
    #[arg(long)]
    pub dry_run: bool,
    
    /// 強制覆寫已存在的檔案
    #[arg(long, short = 'f')]
    pub force: bool,
}

impl SyncArgs {
    /// 根據參數自動判斷同步方法
    pub fn sync_method(&self) -> SyncMethod {
        if self.offset.is_some() {
            SyncMethod::Manual
        } else {
            SyncMethod::Auto
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncMethod {
    /// 自動同步：使用音訊分析
    Auto,
    /// 手動偏移：使用指定的時間偏移
    Manual,
}
```

### 步驟 2：更新同步命令
```rust
// src/commands/sync_command.rs
use crate::cli::sync_args::{SyncArgs, SyncMethod};

impl SyncCommand {
    pub fn new(args: SyncArgs) -> Self {
        Self { args }
    }
    
    pub async fn execute(&self) -> Result<()> {
        let sync_method = self.args.sync_method();
        
        println!("🎯 開始字幕同步");
        println!("   影片: {}", self.args.video);
        println!("   字幕: {}", self.args.subtitle);
        
        match sync_method {
            SyncMethod::Manual => {
                let offset = self.args.offset.unwrap(); // 安全，因為已經檢查過
                println!("   模式: 手動偏移 ({} 秒)", offset);
                self.execute_manual_sync(offset).await?;
            },
            SyncMethod::Auto => {
                println!("   模式: 自動同步");
                self.execute_auto_sync().await?;
            }
        }
        
        println!("✅ 字幕同步完成");
        Ok(())
    }
    
    async fn execute_manual_sync(&self, offset: f64) -> Result<()> {
        // 載入字幕
        let mut subtitle = self.load_subtitle(&self.args.subtitle).await?;
        
        // 應用手動偏移
        subtitle.apply_offset(offset)?;
        
        // 儲存結果
        let output_path = self.determine_output_path()?;
        self.save_subtitle(subtitle, &output_path).await?;
        
        println!("📝 已應用 {} 秒偏移", offset);
        Ok(())
    }
    
    async fn execute_auto_sync(&self) -> Result<()> {
        // 分析音訊和字幕
        let sync_result = self.analyze_sync().await?;
        
        // 載入字幕
        let mut subtitle = self.load_subtitle(&self.args.subtitle).await?;
        
        // 應用自動計算的偏移
        subtitle.apply_offset(sync_result.offset)?;
        
        // 儲存結果
        let output_path = self.determine_output_path()?;
        self.save_subtitle(subtitle, &output_path).await?;
        
        println!("🔍 自動偵測偏移: {} 秒", sync_result.offset);
        println!("📊 同步信心度: {:.2}%", sync_result.confidence * 100.0);
        Ok(())
    }
}
```

### 步驟 3：實作向後相容性支援
```rust
// src/cli/sync_args.rs
impl SyncArgs {
    /// 檢查是否使用了棄用的參數組合
    pub fn check_deprecated_usage(&self) {
        // 這個方法可以在未來用於檢測舊的使用模式
        // 目前暫時保留為擴展點
    }
    
    /// 顯示使用提示
    pub fn show_usage_hint(&self) {
        match self.sync_method() {
            SyncMethod::Manual => {
                println!("💡 提示: 使用手動偏移模式 (--offset {:.1})", 
                         self.offset.unwrap());
            },
            SyncMethod::Auto => {
                println!("💡 提示: 使用自動同步模式");
                println!("   如需手動調整，請使用 --offset 參數");
            }
        }
    }
}
```

### 步驟 4：更新 CLI 說明
```rust
// src/cli/mod.rs
impl SyncArgs {
    pub fn about() -> &'static str {
        "同步字幕和影片的時間軸

使用方式:
  自動同步: subx-cli sync video.mp4 subtitle.srt
  手動偏移: subx-cli sync video.mp4 subtitle.srt --offset 2.5

當提供 --offset 參數時，將使用手動偏移模式。
當不提供 --offset 參數時，將使用自動同步模式。"
    }
}
```

## 測試計劃

### 單元測試
1. **參數解析測試**
   - 測試方法選擇邏輯
   - 測試參數驗證

2. **同步方法測試**
   - 測試手動偏移模式
   - 測試自動同步模式

### 整合測試
1. **命令執行測試**
   - 測試不同參數組合的行為
   - 測試輸出格式和回饋

### 測試用例
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_method_selection_manual() {
        let args = SyncArgs {
            video: "video.mp4".to_string(),
            subtitle: "subtitle.srt".to_string(),
            offset: Some(2.5),
            output: None,
            dry_run: false,
            force: false,
        };
        
        assert_eq!(args.sync_method(), SyncMethod::Manual);
    }

    #[test]
    fn test_sync_method_selection_auto() {
        let args = SyncArgs {
            video: "video.mp4".to_string(),
            subtitle: "subtitle.srt".to_string(),
            offset: None,
            output: None,
            dry_run: false,
            force: false,
        };
        
        assert_eq!(args.sync_method(), SyncMethod::Auto);
    }

    #[tokio::test]
    async fn test_manual_sync_execution() {
        let args = SyncArgs {
            video: "test_video.mp4".to_string(),
            subtitle: "test_subtitle.srt".to_string(),
            offset: Some(1.5),
            output: None,
            dry_run: true,
            force: false,
        };
        
        let command = SyncCommand::new(args);
        // 在 dry_run 模式下測試不會失敗
        let result = command.execute().await;
        // 根據實際實作調整測試邏輯
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

# 執行同步相關測試
cargo test sync

# 檢查參數解析
cargo test sync_args
```

### 使用者體驗測試
- 測試新的參數組合是否直觀
- 確保錯誤訊息清晰易懂
- 驗證說明文件的準確性

## 預期成果

### 功能改善
- 移除冗餘的 `--method` 參數
- 簡化使用者介面
- 保持所有原有功能

### 使用案例展示
```bash
# 簡化後的使用方式

# 自動同步（預設）
$ subx-cli sync movie.mp4 subtitle.srt
🎯 開始字幕同步
   影片: movie.mp4
   字幕: subtitle.srt
   模式: 自動同步
💡 提示: 使用自動同步模式
   如需手動調整，請使用 --offset 參數
🔍 自動偵測偏移: 1.2 秒
📊 同步信心度: 85.3%
✅ 字幕同步完成

# 手動偏移
$ subx-cli sync movie.mp4 subtitle.srt --offset 2.5
🎯 開始字幕同步
   影片: movie.mp4
   字幕: subtitle.srt
   模式: 手動偏移 (2.5 秒)
💡 提示: 使用手動偏移模式 (--offset 2.5)
📝 已應用 2.5 秒偏移
✅ 字幕同步完成

# 預覽模式
$ subx-cli sync movie.mp4 subtitle.srt --offset -1.0 --dry-run
🎯 開始字幕同步 (預覽模式)
   影片: movie.mp4
   字幕: subtitle.srt
   模式: 手動偏移 (-1.0 秒)
🔍 預覽: 將應用 -1.0 秒偏移
✅ 預覽完成（未修改檔案）
```

## 額外改善

### 進階功能
1. **智慧參數提示**
   - 當自動同步失敗時，建議使用手動偏移
   - 提供常見偏移值的建議

2. **參數驗證增強**
   - 檢查偏移值的合理性
   - 提供範圍建議

### 使用者體驗優化
1. **互動式模式**
   - 允許使用者在自動同步後微調偏移
   - 提供即時預覽功能

2. **記憶功能**
   - 記住成功的同步設定
   - 提供歷史偏移值參考

## 注意事項

### 向後相容性
- 確保現有的腳本和工作流程不受影響
- 提供清晰的移轉指導

### 錯誤處理
- 改善參數驗證和錯誤訊息
- 提供有用的使用提示

### 文件同步
- 更新所有相關文件
- 確保範例的一致性

## 驗收標準

- [ ] 成功移除 `--method` 參數
- [ ] 根據 `--offset` 參數的存在自動選擇同步方法
- [ ] 手動偏移模式功能正常
- [ ] 自動同步模式功能正常
- [ ] 提供清晰的模式指示和回饋
- [ ] 所有相關測試通過
- [ ] 程式碼品質檢查無警告
- [ ] 使用者介面更簡潔直觀
- [ ] 說明文件和範例已更新
