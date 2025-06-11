# 26. 輸入路徑參數 (-i) 功能實作

## 概述

實作 `-i` 參數功能，讓使用者可以指定要處理的資料夊路徑或檔案。此參數需要支援多次使用，並且要整合到所有適用的命令中，包括 `match`、`sync`、`convert`、`detect-encoding` 等命令。同時為尚未支援的命令新增 `--recursive` 參數，以控制目錄掃描的深度：預設僅處理目錄下一層的檔案，使用 `--recursive` 參數時才進行遞迴掃描所有子目錄。

## 背景與需求分析

### 現有命令分析

根據程式碼分析，SubX CLI 目前包含以下命令：

1. **`match`** - AI 驅動的字幕檔案匹配與重新命名
   - 現有參數：`path`（目標目錄路徑）、`--recursive`（遞迴處理子目錄）
   - 適用性：✅ 需要添加 `-i` 參數，已支援 `--recursive`

2. **`convert`** - 字幕格式轉換
   - 現有參數：`input`（輸入檔案或目錄路徑）
   - 適用性：✅ 需要添加 `-i` 參數和 `--recursive` 參數

3. **`sync`** - 音訊字幕同步
   - 現有參數：`video`、`subtitle`（特定檔案路徑）
   - 適用性：✅ 需要添加 `-i` 參數和 `--recursive` 參數來支援批次處理

4. **`detect-encoding`** - 字元編碼偵測
   - 現有參數：`file_paths`（檔案路徑列表）
   - 適用性：✅ 需要添加 `-i` 參數和 `--recursive` 參數

5. **`config`** - 設定管理
   - 現有參數：action 子命令
   - 適用性：❌ 不適用於檔案處理

6. **`cache`** - 快取管理
   - 現有參數：action 子命令
   - 適用性：❌ 不適用於檔案處理

7. **`generate-completion`** - Shell 補全腳本生成
   - 現有參數：shell 類型
   - 適用性：❌ 不適用於檔案處理

### 目錄處理能力分析

- **`match` 命令**：✅ 已支援多層目錄結構處理（透過現有的 `--recursive` 參數）
- **`convert` 命令**：❌ 目前不支援多層目錄結構，需要新增 `--recursive` 參數
- **`sync` 命令**：❌ 目前主要針對單一檔案對，需要新增批次處理和 `--recursive` 參數
- **`detect-encoding` 命令**：❌ 目前不支援目錄處理，需要新增 `--recursive` 參數

### 功能需求

1. **多重輸入支援**：使用者可以多次使用 `-i` 參數來指定多個輸入來源
2. **檔案與目錄支援**：參數應同時支援單一檔案和目錄路徑
3. **路徑解析**：支援相對路徑和絕對路徑
4. **遞迴目錄處理**：預設只處理目錄下一層的檔案，使用 `--recursive` 參數才進行遞迴掃描
5. **錯誤處理**：對於無效路徑或不存在的檔案提供清晰的錯誤訊息
6. **向後相容性**：保持現有 CLI 介面的相容性
7. **優先級處理**：當同時指定現有參數和 `-i` 參數時的行為定義

## 技術設計

### 1. 共用 Input 結構體設計

建立一個共用的輸入處理結構體，供所有支援的命令使用：

```rust
/// 通用輸入路徑處理結構
#[derive(Debug, Clone)]
pub struct InputPaths {
    /// 輸入路徑列表
    pub paths: Vec<PathBuf>,
    /// 是否遞迴處理子目錄
    pub recursive: bool,
}

impl InputPaths {
    /// 從命令列參數建立 InputPaths
    pub fn from_args(input_args: &[PathBuf], recursive: bool) -> Result<Self, Error> {
        // 實作路徑驗證與正規化邏輯
    }
    
    /// 展開目錄並收集所有檔案
    pub fn expand_directories(&self, extensions: &[&str]) -> Result<Vec<PathBuf>, Error> {
        // 實作目錄掃描與檔案過濾邏輯，根據 recursive 參數決定是否遞迴
    }
    
    /// 驗證所有路徑是否存在
    pub fn validate(&self) -> Result<(), Error> {
        // 實作路徑存在性檢查
    }
}
```

### 2. CLI 參數整合策略

#### Match 命令更新

```rust
#[derive(Args, Debug)]
pub struct MatchArgs {
    /// 目標目錄路徑（現有參數，保持向後相容性）
    pub path: Option<PathBuf>,

    /// 指定要處理的檔案或目錄路徑（新增參數）
    /// 可以多次使用來指定多個輸入來源
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input_paths: Vec<PathBuf>,

    /// 遞迴處理子目錄（現有參數）
    #[arg(short, long)]
    pub recursive: bool,

    // ... 其他現有參數保持不變
}

impl MatchArgs {
    /// 取得所有輸入路徑，優先使用 -i 參數
    pub fn get_input_paths(&self) -> Result<InputPaths, Error> {
        if !self.input_paths.is_empty() {
            InputPaths::from_args(&self.input_paths, self.recursive)
        } else if let Some(path) = &self.path {
            InputPaths::from_args(&[path.clone()], self.recursive)
        } else {
            Err(Error::NoInputSpecified)
        }
    }
}
```

#### Convert 命令更新

```rust
#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// 輸入檔案或目錄路徑（現有參數）
    pub input: Option<PathBuf>,
    
    /// 指定要處理的檔案或目錄路徑（新增參數）
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input_paths: Vec<PathBuf>,

    /// 遞迴處理子目錄（新增參數）
    #[arg(short, long)]
    pub recursive: bool,

    // ... 其他現有參數保持不變
}

impl ConvertArgs {
    /// 取得所有輸入路徑
    pub fn get_input_paths(&self) -> Result<InputPaths, Error> {
        if !self.input_paths.is_empty() {
            InputPaths::from_args(&self.input_paths, self.recursive)
        } else if let Some(input) = &self.input {
            InputPaths::from_args(&[input.clone()], self.recursive)
        } else {
            Err(Error::NoInputSpecified)
        }
    }
}
```

#### Sync 命令更新

```rust
#[derive(Args, Debug)]
pub struct SyncArgs {
    /// 影片檔案路徑（現有參數）
    pub video: Option<PathBuf>,
    
    /// 字幕檔案路徑（現有參數）
    pub subtitle: Option<PathBuf>,
    
    /// 指定要處理的目錄路徑，用於批次處理（新增參數）
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input_paths: Vec<PathBuf>,

    /// 遞迴處理子目錄（新增參數）
    #[arg(short, long)]
    pub recursive: bool,

    // ... 其他現有參數保持不變
}

impl SyncArgs {
    /// 取得同步模式：單檔或批次
    pub fn get_sync_mode(&self) -> Result<SyncMode, Error> {
        if !self.input_paths.is_empty() {
            Ok(SyncMode::Batch(InputPaths::from_args(&self.input_paths, self.recursive)?))
        } else if self.video.is_some() && self.subtitle.is_some() {
            Ok(SyncMode::Single {
                video: self.video.as_ref().unwrap().clone(),
                subtitle: self.subtitle.as_ref().unwrap().clone(),
            })
        } else {
            Err(Error::InvalidSyncConfiguration)
        }
    }
}

#[derive(Debug)]
pub enum SyncMode {
    Single { video: PathBuf, subtitle: PathBuf },
    Batch(InputPaths),
}
```

#### DetectEncoding 命令更新

```rust
#[derive(Args, Debug)]
pub struct DetectEncodingArgs {
    /// 要分析的檔案路徑（現有參數）
    pub file_paths: Vec<String>,
    
    /// 指定要處理的檔案或目錄路徑（新增參數）
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input_paths: Vec<PathBuf>,

    /// 遞迴處理子目錄（新增參數）
    #[arg(short, long)]
    pub recursive: bool,

    /// 顯示詳細資訊
    #[arg(short, long)]
    pub verbose: bool,
}

impl DetectEncodingArgs {
    /// 取得所有要處理的檔案路徑
    pub fn get_all_file_paths(&self) -> Result<Vec<PathBuf>, Error> {
        let mut all_paths = Vec::new();
        
        // 添加現有的 file_paths
        for path_str in &self.file_paths {
            all_paths.push(PathBuf::from(path_str));
        }
        
        // 添加通過 -i 參數指定的路徑
        if !self.input_paths.is_empty() {
            let input_paths = InputPaths::from_args(&self.input_paths, self.recursive)?;
            all_paths.extend(input_paths.expand_directories(&["srt", "ass", "vtt", "ssa", "sub"])?);
        }
        
        if all_paths.is_empty() {
            return Err(Error::NoInputSpecified);
        }
        
        Ok(all_paths)
    }
}
```

### 3. 檔案處理邏輯

#### 目錄掃描與檔案過濾

```rust
impl InputPaths {
    /// 展開目錄並根據副檔名過濾檔案
    pub fn expand_directories(&self, extensions: &[&str]) -> Result<Vec<PathBuf>, Error> {
        let mut result_files = Vec::new();
        
        for path in &self.paths {
            if path.is_file() {
                // 直接添加檔案
                result_files.push(path.clone());
            } else if path.is_dir() {
                // 掃描目錄
                let found_files = if self.recursive {
                    self.scan_directory_recursive(path, extensions)?
                } else {
                    self.scan_directory_flat(path, extensions)?
                };
                result_files.extend(found_files);
            } else {
                return Err(Error::InvalidPath(path.clone()));
            }
        }
        
        Ok(result_files)
    }
    
    /// 非遞迴掃描目錄（僅處理一層）
    fn scan_directory_flat(&self, dir: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>, Error> {
        let mut files = Vec::new();
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if extensions.contains(&ext_str.to_lowercase().as_str()) {
                            files.push(path);
                        }
                    }
                }
            }
        }
        
        Ok(files)
    }
    
    /// 遞迴掃描目錄
    fn scan_directory_recursive(&self, dir: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>, Error> {
        let mut files = Vec::new();
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if extensions.contains(&ext_str.to_lowercase().as_str()) {
                            files.push(path);
                        }
                    }
                }
            } else if path.is_dir() {
                // 遞迴處理子目錄
                let sub_files = self.scan_directory_recursive(&path, extensions)?;
                files.extend(sub_files);
            }
        }
        
        Ok(files)
    }
}
```

### 4. 錯誤處理

```rust
/// 輸入路徑相關錯誤
#[derive(Debug, thiserror::Error)]
pub enum InputPathError {
    #[error("未指定輸入路徑")]
    NoInputSpecified,
    
    #[error("無效的路徑: {0}")]
    InvalidPath(PathBuf),
    
    #[error("路徑不存在: {0}")]
    PathNotFound(PathBuf),
    
    #[error("無法讀取目錄: {0}")]
    DirectoryReadError(#[from] std::io::Error),
    
    #[error("同步設定無效：請指定影片和字幕檔案，或使用 -i 參數進行批次處理")]
    InvalidSyncConfiguration,
}
```

## 實作步驟

### 階段 1：基礎設施建立

1. **建立共用模組**
   - 在 `src/cli/` 目錄下建立 `input_paths.rs` 模組
   - 實作 `InputPaths` 結構體和相關方法
   - 定義輸入路徑錯誤類型

2. **更新模組匯出**
   - 在 `src/cli/mod.rs` 中新增 `input_paths` 模組匯出
   - 確保錯誤類型正確整合到主要錯誤體系

### 階段 2：命令參數更新

1. **Match 命令**
   - 更新 `src/cli/match_args.rs`
   - 新增 `-i` 參數和相關方法
   - 整合現有的 `--recursive` 參數邏輯
   - 更新文件和範例

2. **Convert 命令**
   - 更新 `src/cli/convert_args.rs`
   - 新增 `-i` 參數和相關方法
   - **新增 `--recursive` 參數**（目前不存在）
   - 處理與現有 `input` 參數的優先級

3. **Sync 命令**
   - 更新 `src/cli/sync_args.rs`
   - 新增 `-i` 參數和批次處理模式
   - **新增 `--recursive` 參數**（目前不存在）
   - 實作 `SyncMode` 列舉和相關邏輯

4. **DetectEncoding 命令**
   - 更新 `src/cli/detect_encoding_args.rs`
   - 新增 `-i` 參數整合
   - **新增 `--recursive` 參數**（目前不存在）
   - 合併現有和新增的檔案路徑處理

### 階段 3：命令實作更新

1. **Match 命令實作**
   - 更新 `src/commands/match_command.rs`
   - 整合新的輸入路徑處理邏輯
   - 確保多路徑處理正確運作

2. **Convert 命令實作**
   - 更新 `src/commands/convert_command.rs`
   - 實作批次轉換邏輯
   - 處理輸出檔案命名衝突

3. **Sync 命令實作**
   - 更新 `src/commands/sync_command.rs`
   - 實作批次同步處理
   - 自動配對影片和字幕檔案

4. **DetectEncoding 命令實作**
   - 更新 `src/commands/detect_encoding_command.rs`
   - 整合新的檔案路徑收集邏輯

### 階段 4：測試實作

1. **單元測試**
   - 建立 `tests/input_paths_tests.rs`
   - 測試路徑解析和驗證邏輯
   - 測試目錄掃描功能

2. **整合測試**
   - 更新現有的 CLI 整合測試
   - 新增 `-i` 參數的測試案例
   - 測試多路徑處理和錯誤情況

3. **CLI 參數解析測試**
   - 更新各命令的參數解析測試
   - 確保向後相容性
   - 測試參數優先級和衝突處理

### 階段 5：文件更新

1. **命令說明**
   - 更新各命令的 help 文件
   - 新增 `-i` 參數的使用範例
   - 說明與現有參數的關係

2. **使用者文件**
   - 更新 README.md 中的使用範例
   - 新增批次處理的使用案例
   - 提供最佳實踐建議

3. **API 文件**
   - 更新 Rust 文件註釋
   - 確保所有新增的結構體和方法都有完整文件
   - 新增程式碼範例

## 測試策略

### 1. 單元測試重點

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_input_paths_from_files() {
        // 測試單一檔案輸入
    }
    
    #[test]
    fn test_input_paths_from_directories() {
        // 測試目錄輸入
    }
    
    #[test]
    fn test_input_paths_mixed() {
        // 測試混合檔案和目錄輸入
    }
    
    #[test]
    fn test_input_paths_validation() {
        // 測試路徑驗證邏輯
    }
    
    #[test]
    fn test_directory_expansion() {
        // 測試目錄展開和檔案過濾
    }
}
```

### 2. 整合測試重點

1. **CLI 參數解析**
   - 測試 `-i` 參數的多重使用
   - 測試與現有參數的組合
   - 測試錯誤情況和錯誤訊息

2. **檔案處理**
   - 測試大量檔案的處理效能
   - 測試各種檔案系統結構
   - 測試權限和存取問題

3. **向後相容性**
   - 確保現有的 CLI 介面繼續運作
   - 測試升級場景
   - 驗證預設行為未改變

### 3. 效能測試

1. **大量檔案處理**
   - 測試處理數千個檔案的效能
   - 記憶體使用量監控
   - 處理時間基準測試

2. **目錄掃描效能**
   - 深層目錄結構的掃描效能
   - 不同檔案系統的效能比較
   - 並行掃描的效能提升

## 使用範例

### Match 命令

```bash
# 使用現有方式（保持相容性）
subx match ./videos

# 使用現有方式進行遞迴處理
subx match ./videos --recursive

# 使用新的 -i 參數
subx match -i ./videos -i ./more_videos

# 使用新的 -i 參數進行遞迴處理
subx match -i ./videos -i ./more_videos --recursive

# 混合檔案和目錄
subx match -i ./video1.mp4 -i ./subtitles_dir -i ./video2.mkv --recursive
```

### Convert 命令

```bash
# 單一檔案轉換
subx convert -i movie.srt --format ass

# 批次轉換多個目錄（僅處理一層）
subx convert -i ./srt_files -i ./more_subtitles --format vtt

# 批次轉換多個目錄（遞迴處理子目錄）
subx convert -i ./srt_files -i ./more_subtitles --format vtt --recursive

# 混合檔案和目錄
subx convert -i movie1.srt -i ./batch_dir -i movie2.ass --format srt --recursive
```

### Sync 命令

```bash
# 傳統單檔同步（保持相容性）
subx sync video.mp4 subtitle.srt

# 批次同步整個目錄（僅處理一層）
subx sync -i ./movies_directory

# 批次同步整個目錄（遞迴處理子目錄）
subx sync -i ./movies_directory --recursive

# 多個目錄批次同步
subx sync -i ./movies1 -i ./movies2 -i ./tv_shows --recursive
```

### DetectEncoding 命令

```bash
# 現有方式（保持相容性）
subx detect-encoding *.srt

# 使用新參數處理多個目錄（僅處理一層）
subx detect-encoding -i ./subtitles1 -i ./subtitles2 --verbose

# 使用新參數處理多個目錄（遞迴處理子目錄）
subx detect-encoding -i ./subtitles1 -i ./subtitles2 --verbose --recursive

# 混合指定和目錄掃描
subx detect-encoding file1.srt file2.ass -i ./more_subtitles --recursive
```

## 品質保證

### 1. 程式碼品質

- **代碼風格**：遵循 Rust 標準格式和 Clippy 建議
- **錯誤處理**：提供清晰且有用的錯誤訊息
- **記憶體安全**：避免不必要的記憶體分配和複製
- **效能最佳化**：使用有效率的檔案系統操作

### 2. 使用者體驗

- **直觀介面**：參數命名和行為符合使用者期望
- **清晰文件**：提供充足的使用範例和說明
- **錯誤回饋**：錯誤訊息清楚指出問題和可能的解決方案
- **向後相容**：確保現有使用者工作流程不受影響

### 3. 可維護性

- **模組化設計**：將共用邏輯提取到可重用的模組
- **充分測試**：達到高測試覆蓋率，特別是邊界情況
- **文件完整**：程式碼註釋和使用者文件保持同步
- **版本管理**：適當的版本號和變更記錄

## 風險評估與緩解策略

### 1. 相容性風險

**風險**：新參數可能與現有邏輯產生衝突

**緩解策略**：
- 保持現有參數的預設行為不變
- 新參數僅在明確指定時才啟用
- 廣泛的向後相容性測試

### 2. 效能風險

**風險**：大量檔案處理可能影響效能

**緩解策略**：
- 實作檔案數量限制和警告
- 提供進度回饋和取消機制
- 使用有效率的檔案系統 API

### 3. 使用複雜性風險

**風險**：新參數可能增加使用複雜性

**緩解策略**：
- 提供清晰的文件和範例
- 實作智慧預設值
- 提供使用建議和最佳實踐

## 交付成果

### 1. 程式碼交付

- [ ] `src/cli/input_paths.rs` - 輸入路徑處理模組
- [ ] 更新的 CLI 參數結構體（4個命令）
- [ ] 更新的命令實作（4個命令）
- [ ] 新增和更新的測試檔案

### 2. 文件交付

- [ ] 更新的 help 文件和 CLI 說明
- [ ] 更新的 README.md 和使用者指南
- [ ] API 文件和程式碼註釋
- [ ] 測試文件和最佳實踐指南

### 3. 品質確保交付

- [ ] 測試報告和覆蓋率分析
- [ ] 效能基準測試結果
- [ ] 相容性測試報告
- [ ] 使用者體驗評估報告

## 後續規劃

### 短期優化

1. **智慧檔案配對**：在批次處理模式下自動配對相關檔案
2. **並行處理**：利用多核心加速大量檔案處理
3. **進度顯示**：提供詳細的處理進度和狀態

### 長期增強

1. **過濾器支援**：新增檔案過濾和排除規則
2. **配置整合**：允許在配置檔案中預設輸入路徑
3. **外掛系統**：支援自訂的檔案發現和處理邏輯

這個 backlog 提供了實作 `-i` 參數功能的完整規劃，涵蓋技術設計、實作步驟、測試策略和品質保證措施。實作過程中應該遵循這些指導原則，確保功能的正確性、效能和使用者體驗。
