# Bug Fix #05: Convert 命令檔案移除功能

## 問題描述

當執行 `subx-cli convert` 命令且沒有使用 `--keep-original` 參數時，系統應該移除原始檔案，但目前的實作沒有實現這個功能。

具體問題：
- 轉換完成後，原始檔案仍然存在
- 使用者預期在沒有 `--keep-original` 的情況下，舊檔案會被自動移除
- 這會導致檔案重複和儲存空間浪費

## 問題分析

### 現狀分析
- `convert` 命令目前只執行格式轉換，建立新檔案
- 沒有檢查 `--keep-original` 參數的邏輯
- 缺少檔案清理機制

### 根本原因
1. **檔案清理邏輯缺失**：轉換後沒有移除原始檔案的程式碼
2. **參數處理不完整**：沒有正確處理 `--keep-original` 參數
3. **錯誤處理不足**：沒有考慮檔案移除可能失敗的情況

### 預期行為
```bash
# 保留原始檔案
subx-cli convert --keep-original input.srt --to ass
# 結果：input.srt 和 input.ass 都存在

# 移除原始檔案（預設行為）
subx-cli convert input.srt --to ass  
# 結果：只有 input.ass 存在，input.srt 被移除
```

## 技術方案

### 架構設計
1. **檔案生命週期管理**
   - 轉換過程中的檔案狀態追蹤
   - 確保轉換成功後才移除原始檔案

2. **安全檔案操作**
   - 實作安全的檔案移除機制
   - 提供回滾機制以防轉換失敗

3. **參數驅動的行為控制**
   - 根據 `--keep-original` 參數決定是否保留檔案
   - 提供清晰的使用者回饋

### 設計原則
- **安全第一**：確保轉換成功後才移除原始檔案
- **可恢復性**：轉換失敗時保持原始檔案不變
- **清晰回饋**：明確告知使用者檔案操作結果

## 實作步驟

### 第一階段：增強 convert 命令參數處理
1. **檢查參數處理邏輯**
   - 檔案：`src/cli/convert_args.rs`
   - 確保 `--keep-original` 參數正確定義

2. **更新命令結構**
   - 檔案：`src/commands/convert_command.rs`
   - 增加檔案清理相關邏輯

### 第二階段：實作檔案生命週期管理
1. **建立檔案操作管理器**
   - 檔案：`src/core/file_manager.rs`
   - 實作安全的檔案操作功能

2. **增強錯誤處理**
   - 處理檔案移除失敗的情況
   - 提供詳細的錯誤資訊

### 第三階段：整合轉換和清理邏輯
1. **修改轉換流程**
   - 在轉換成功後執行清理操作
   - 確保原子性操作

2. **增加使用者回饋**
   - 顯示檔案操作的詳細資訊
   - 提供操作確認和結果回報

## 詳細實作指南

### 步驟 1：檢查和完善參數定義
```rust
// src/cli/convert_args.rs
use clap::Args;

#[derive(Debug, Args)]
pub struct ConvertArgs {
    /// 輸入檔案路徑
    #[arg(value_name = "INPUT")]
    pub input: String,
    
    /// 目標格式
    #[arg(long, short = 't', value_name = "FORMAT")]
    pub to: String,
    
    /// 輸出檔案路徑（可選）
    #[arg(long, short = 'o', value_name = "OUTPUT")]
    pub output: Option<String>,
    
    /// 保留原始檔案
    #[arg(long, help = "保留原始檔案，不進行移除")]
    pub keep_original: bool,
    
    /// 強制覆寫已存在的檔案
    #[arg(long, short = 'f')]
    pub force: bool,
}
```

### 步驟 2：建立檔案管理器
```rust
// src/core/file_manager.rs
use std::fs;
use std::path::{Path, PathBuf};
use crate::error::Result;

pub struct FileManager {
    operations: Vec<FileOperation>,
}

#[derive(Debug)]
enum FileOperation {
    Created(PathBuf),
    Removed(PathBuf),
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }
    
    /// 安全地移除檔案
    pub fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        
        if !path.exists() {
            return Err(crate::error::SubXError::FileNotFound(
                path.to_string_lossy().to_string()
            ));
        }
        
        // 執行移除操作
        fs::remove_file(&path)?;
        
        // 記錄操作
        self.operations.push(FileOperation::Removed(path.clone()));
        
        println!("🗑️  已移除原始檔案: {}", path.display());
        Ok(())
    }
    
    /// 記錄檔案建立操作
    pub fn record_creation<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref().to_path_buf();
        self.operations.push(FileOperation::Created(path));
    }
    
    /// 回滾操作（轉換失敗時使用）
    pub fn rollback(&mut self) -> Result<()> {
        for operation in self.operations.drain(..).rev() {
            match operation {
                FileOperation::Created(path) => {
                    if path.exists() {
                        fs::remove_file(&path)?;
                        println!("🔄 已回滾建立的檔案: {}", path.display());
                    }
                },
                FileOperation::Removed(_) => {
                    // 已移除的檔案無法恢復，記錄警告
                    eprintln!("⚠️  警告：無法恢復已移除的檔案");
                }
            }
        }
        Ok(())
    }
}
```

### 步驟 3：更新轉換命令
```rust
// src/commands/convert_command.rs
use crate::core::file_manager::FileManager;
use std::path::{Path, PathBuf};

impl ConvertCommand {
    pub async fn execute(&self) -> Result<()> {
        let input_path = Path::new(&self.args.input);
        let output_path = self.determine_output_path(input_path)?;
        
        // 檢查輸入檔案是否存在
        if !input_path.exists() {
            return Err(SubXError::FileNotFound(self.args.input.clone()));
        }
        
        // 檢查輸出檔案是否已存在
        if output_path.exists() && !self.args.force {
            return Err(SubXError::FileAlreadyExists(
                output_path.to_string_lossy().to_string()
            ));
        }
        
        let mut file_manager = FileManager::new();
        
        // 執行轉換
        match self.perform_conversion(input_path, &output_path).await {
            Ok(_) => {
                // 轉換成功，記錄建立的檔案
                file_manager.record_creation(&output_path);
                
                println!("✅ 轉換完成: {} -> {}", 
                         input_path.display(), 
                         output_path.display());
                
                // 如果不保留原始檔案，則移除它
                if !self.args.keep_original {
                    if let Err(e) = file_manager.remove_file(input_path) {
                        // 移除失敗不應該影響轉換結果
                        eprintln!("⚠️  警告：無法移除原始檔案 {}: {}", 
                                 input_path.display(), e);
                    }
                }
                
                Ok(())
            },
            Err(e) => {
                // 轉換失敗，回滾操作
                eprintln!("❌ 轉換失敗: {}", e);
                if let Err(rollback_err) = file_manager.rollback() {
                    eprintln!("❌ 回滾失敗: {}", rollback_err);
                }
                Err(e)
            }
        }
    }
    
    fn determine_output_path(&self, input_path: &Path) -> Result<PathBuf> {
        if let Some(output) = &self.args.output {
            Ok(PathBuf::from(output))
        } else {
            // 自動產生輸出檔名
            let stem = input_path.file_stem()
                .ok_or_else(|| SubXError::InvalidFileName(
                    input_path.to_string_lossy().to_string()
                ))?;
            
            let new_ext = format!(".{}", &self.args.to);
            let output_name = format!("{}{}", stem.to_string_lossy(), new_ext);
            
            Ok(input_path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(output_name))
        }
    }
    
    async fn perform_conversion(&self, input: &Path, output: &Path) -> Result<()> {
        // 載入原始字幕
        let subtitle = self.load_subtitle(input).await?;
        
        // 轉換格式
        let converted = self.convert_format(subtitle, &self.args.to)?;
        
        // 儲存轉換後的字幕
        self.save_subtitle(converted, output).await?;
        
        Ok(())
    }
}
```

### 步驟 4：增強錯誤處理
```rust
// src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum SubXError {
    // ...existing errors...
    
    #[error("檔案已存在: {0}")]
    FileAlreadyExists(String),
    
    #[error("檔案不存在: {0}")]
    FileNotFound(String),
    
    #[error("無效的檔案名稱: {0}")]
    InvalidFileName(String),
    
    #[error("檔案操作失敗: {0}")]
    FileOperationFailed(String),
}
```

## 測試計劃

### 單元測試
1. **檔案管理器測試**
   - 測試檔案移除功能
   - 測試回滾機制
   - 測試操作記錄

2. **轉換命令測試**
   - 測試保留原始檔案的情況
   - 測試移除原始檔案的情況
   - 測試轉換失敗時的行為

### 整合測試
1. **端到端轉換測試**
   - 測試完整的轉換和清理流程
   - 測試不同格式的轉換

### 測試用例
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_file_manager_remove() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();
        
        let mut manager = FileManager::new();
        manager.remove_file(&test_file).unwrap();
        
        assert!(!test_file.exists());
    }

    #[tokio::test]
    async fn test_convert_without_keep_original() {
        let temp_dir = TempDir::new().unwrap();
        let input_file = temp_dir.path().join("input.srt");
        let output_file = temp_dir.path().join("input.ass");
        
        // 建立測試輸入檔案
        fs::write(&input_file, "test subtitle content").unwrap();
        
        let args = ConvertArgs {
            input: input_file.to_string_lossy().to_string(),
            to: "ass".to_string(),
            output: None,
            keep_original: false,
            force: false,
        };
        
        let command = ConvertCommand::new(args);
        command.execute().await.unwrap();
        
        // 驗證原始檔案被移除，新檔案存在
        assert!(!input_file.exists());
        assert!(output_file.exists());
    }

    #[tokio::test]
    async fn test_convert_with_keep_original() {
        let temp_dir = TempDir::new().unwrap();
        let input_file = temp_dir.path().join("input.srt");
        let output_file = temp_dir.path().join("input.ass");
        
        fs::write(&input_file, "test subtitle content").unwrap();
        
        let args = ConvertArgs {
            input: input_file.to_string_lossy().to_string(),
            to: "ass".to_string(),
            output: None,
            keep_original: true,
            force: false,
        };
        
        let command = ConvertCommand::new(args);
        command.execute().await.unwrap();
        
        // 驗證原始檔案和新檔案都存在
        assert!(input_file.exists());
        assert!(output_file.exists());
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

# 執行轉換相關測試
cargo test convert

# 執行檔案管理測試
cargo test file_manager
```

### 安全性考量
- 確保檔案移除操作的原子性
- 防止意外移除重要檔案
- 提供充分的錯誤回報

## 預期成果

### 功能改善
- 正確實現 `--keep-original` 參數的功能
- 預設情況下自動移除原始檔案
- 提供安全的檔案操作機制

### 使用案例展示
```bash
# 案例 1：預設行為（移除原始檔案）
$ ls
movie.srt

$ subx-cli convert movie.srt --to ass
✅ 轉換完成: movie.srt -> movie.ass
🗑️  已移除原始檔案: movie.srt

$ ls  
movie.ass

# 案例 2：保留原始檔案
$ ls
movie.srt

$ subx-cli convert movie.srt --to ass --keep-original
✅ 轉換完成: movie.srt -> movie.ass

$ ls
movie.srt  movie.ass

# 案例 3：轉換失敗時的回滾
$ subx-cli convert invalid.srt --to ass
❌ 轉換失敗: 無法解析字幕格式
🔄 已回滾建立的檔案: invalid.ass
```

## 額外功能

### 進階檔案管理
1. **備份機制**
   - 在移除前建立臨時備份
   - 轉換完成後清理備份

2. **批次處理支援**
   - 支援多個檔案的批次轉換
   - 統一的檔案清理策略

### 使用者體驗改善
1. **確認提示**
   - 在移除重要檔案前詢問確認
   - 提供詳細的操作預覽

2. **進度回報**
   - 顯示轉換和清理的進度
   - 提供詳細的操作日誌

## 注意事項

### 資料安全
- 確保轉換完全成功後才移除原始檔案
- 提供足夠的錯誤處理和回滾機制
- 避免意外的資料遺失

### 效能考量
- 檔案操作應該高效且可靠
- 避免不必要的檔案系統操作
- 考慮大檔案的處理效率

### 相容性
- 確保在不同作業系統上的正確行為
- 處理檔案權限和存取限制
- 支援各種檔案系統

## 驗收標準

- [ ] `--keep-original` 參數正確控制檔案保留行為
- [ ] 預設情況下成功移除原始檔案
- [ ] 轉換失敗時不移除原始檔案
- [ ] 提供清晰的檔案操作回饋資訊
- [ ] 實作安全的檔案移除機制
- [ ] 所有相關測試通過
- [ ] 程式碼品質檢查無警告
- [ ] 不影響現有轉換功能的正常運作
