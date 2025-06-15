# 26. 輸入路徑參數 (-i) 功能實作

## 概述

實作 `-i` 參數功能，讓使用者可以指定要處理的資料夊路徑或檔案。此參數需要支援多次使用，並且要整合到所有適用的命令中，包括 `match`、`sync`、`convert`、`detect-encoding` 等命令。同時為尚未支援的命令新增 `--recursive` 參數，以控制目錄掃描的深度：預設僅處理目錄下一層的檔案，使用 `--recursive` 參數時才進行遞迴掃描所有子目錄。

## 背景與需求分析

### 現有命令分析（2025年6月更新）

根據當前程式碼分析，SubX CLI 目前包含以下命令：

1. **`match`** - AI 驅動的字幕檔案匹配與重新命名
   - 現有參數：`path`（目標目錄路徑）、`--recursive`（遞迴處理子目錄）
   - 適用性：✅ 需要添加 `-i` 參數，已支援 `--recursive`
   - 現況：已支援 `--copy`, `--move` 參數

2. **`convert`** - 字幕格式轉換
   - 現有參數：`input`（輸入檔案或目錄路徑）
   - 適用性：✅ 需要添加 `-i` 參數和 `--recursive` 參數
   - 現況：支援 `--keep-original`, `--encoding` 參數

3. **`sync`** - 音訊字幕同步
   - 現有參數：`video`（可選）、`subtitle`（必要）、`--method`（同步方法）
   - 適用性：✅ 需要添加 `-i` 參數和 `--recursive` 參數來支援批次處理
   - 現況：已支援 VAD 和 Manual 方法，有 `--batch` 模式

4. **`detect-encoding`** - 字元編碼偵測
   - 現有參數：`file_paths`（檔案路徑列表）
   - 適用性：✅ 需要重構為 `-i` 參數和 `--recursive` 參數
   - 現況：已支援多檔案處理

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
- **`convert` 命令**：⚠️ 目前支援單一檔案或目錄，但無 `--recursive` 參數
- **`sync` 命令**：⚠️ 目前支援單一檔案對，有 `--batch` 參數但缺少 `--recursive`
- **`detect-encoding` 命令**：⚠️ 目前支援多檔案但無目錄處理和 `--recursive` 參數

### 功能需求

1. **多重輸入支援**：使用者可以多次使用 `-i` 參數來指定多個輸入來源
2. **檔案與目錄支援**：參數應同時支援單一檔案和目錄路徑
3. **路徑解析**：支援相對路徑和絕對路徑
4. **遞迴目錄處理**：預設只處理目錄下一層的檔案，使用 `--recursive` 參數才進行遞迴掃描
5. **錯誤處理**：對於無效路徑或不存在的檔案提供清晰的錯誤訊息
6. **向後相容性**：保持現有 CLI 介面的相容性
7. **優先級處理**：當同時指定現有參數和 `-i` 參數時的行為定義
8. **與現有功能整合**：與 `--copy`, `--move`, `--batch` 等現有功能良好整合

## 技術設計

### 1. 共用 Input 處理模組設計

建立一個共用的輸入處理模組，採用與當前架構一致的設計模式：

```rust
// src/cli/input_handler.rs
/// 通用輸入路徑處理結構
#[derive(Debug, Clone)]
pub struct InputPathHandler {
    /// 輸入路徑列表
    pub paths: Vec<PathBuf>,
    /// 是否遞迴處理子目錄
    pub recursive: bool,
    /// 檔案類型過濾器
    pub file_extensions: Vec<String>,
}

impl InputPathHandler {
    /// 從命令列參數建立 InputPathHandler
    pub fn from_args(input_args: &[PathBuf], recursive: bool) -> Result<Self, SubXError> {
        // 實作路徑驗證與正規化邏輯
        let mut handler = Self {
            paths: input_args.to_vec(),
            recursive,
            file_extensions: vec![],
        };
        handler.validate()?;
        Ok(handler)
    }
    
    /// 設定支援的檔案副檔名
    pub fn with_extensions(mut self, extensions: &[&str]) -> Self {
        self.file_extensions = extensions.iter().map(|s| s.to_string()).collect();
        self
    }
    
    /// 展開目錄並收集所有檔案
    pub fn collect_files(&self) -> Result<Vec<PathBuf>, SubXError> {
        let mut files = Vec::new();
        
        for path in &self.paths {
            if path.is_file() {
                files.push(path.clone());
            } else if path.is_dir() {
                let dir_files = if self.recursive {
                    self.scan_directory_recursive(path)?
                } else {
                    self.scan_directory_flat(path)?
                };
                files.extend(dir_files);
            } else {
                return Err(SubXError::InvalidPath(path.clone()));
            }
        }
        
        Ok(files)
    }
    
    /// 驗證所有路徑是否存在
    pub fn validate(&self) -> Result<(), SubXError> {
        for path in &self.paths {
            if !path.exists() {
                return Err(SubXError::PathNotFound(path.clone()));
            }
        }
        Ok(())
    }
    
    /// 非遞迴掃描目錄
    fn scan_directory_flat(&self, dir: &Path) -> Result<Vec<PathBuf>, SubXError> {
        // 實作邏輯
    }
    
    /// 遞迴掃描目錄
    fn scan_directory_recursive(&self, dir: &Path) -> Result<Vec<PathBuf>, SubXError> {
        // 實作邏輯
    }
}
```

### 2. CLI 參數整合策略

#### Match 命令更新（基於現有架構）

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

    // 保持現有的其他參數
    #[arg(long)]
    pub dry_run: bool,
    
    #[arg(long, default_value = "80", value_parser = clap::value_parser!(u8).range(0..=100))]
    pub confidence: u8,
    
    #[arg(long)]
    pub backup: bool,
    
    #[arg(long, short = 'c')]
    pub copy: bool,
    
    #[arg(long = "move", short = 'm')]
    pub move_files: bool,
}

impl MatchArgs {
    /// 取得所有輸入路徑，優先使用 -i 參數
    pub fn get_input_handler(&self) -> Result<InputPathHandler, SubXError> {
        if !self.input_paths.is_empty() {
            InputPathHandler::from_args(&self.input_paths, self.recursive)?
                .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"])
        } else if let Some(path) = &self.path {
            InputPathHandler::from_args(&[path.clone()], self.recursive)?
                .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"])
        } else {
            Err(SubXError::NoInputSpecified)
        }
    }
    
    /// 現有的驗證方法保持不變
    pub fn validate(&self) -> Result<(), String> {
        if self.copy && self.move_files {
            return Err(
                "Cannot use --copy and --move together. Please choose one operation mode."
                    .to_string(),
            );
        }
        Ok(())
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

    /// 目標輸出格式
    #[arg(long, value_enum)]
    pub format: Option<OutputSubtitleFormat>,

    /// 輸出檔案路徑
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// 保留原始檔案
    #[arg(long)]
    pub keep_original: bool,

    /// 字元編碼
    #[arg(long, default_value = "utf-8")]
    pub encoding: String,
}

impl ConvertArgs {
    /// 取得所有輸入路徑
    pub fn get_input_handler(&self) -> Result<InputPathHandler, SubXError> {
        if !self.input_paths.is_empty() {
            InputPathHandler::from_args(&self.input_paths, self.recursive)?
                .with_extensions(&["srt", "ass", "vtt", "sub", "ssa"])
        } else if let Some(input) = &self.input {
            InputPathHandler::from_args(&[input.clone()], self.recursive)?
                .with_extensions(&["srt", "ass", "vtt", "sub", "ssa"])
        } else {
            Err(SubXError::NoInputSpecified)
        }
    }
}
```

#### Sync 命令更新（與現有批次功能整合）

```rust
#[derive(Args, Debug)]
pub struct SyncArgs {
    /// 影片檔案路徑（現有參數，批次模式時可選）
    #[arg(
        short = 'v',
        long = "video",
        value_name = "VIDEO",
        help = "Video file path (required for single file, optional for batch mode)",
        required_unless_present_any = ["offset", "input_paths", "batch"]
    )]
    pub video: Option<PathBuf>,
    
    /// 字幕檔案路徑（現有參數，批次模式時可選）
    #[arg(
        short = 's',
        long = "subtitle",
        value_name = "SUBTITLE",
        help = "Subtitle file path (required for single file)",
        required_unless_present_any = ["input_paths", "batch"]
    )]
    pub subtitle: Option<PathBuf>,
    
    /// 指定要處理的目錄路徑，用於批次處理（新增參數）
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input_paths: Vec<PathBuf>,

    /// 遞迴處理子目錄（新增參數）
    #[arg(short, long)]
    pub recursive: bool,

    // 保持現有的其他參數
    #[arg(long, conflicts_with_all = ["method", "window", "vad_sensitivity"])]
    pub offset: Option<f32>,

    #[arg(short, long, value_enum)]
    pub method: Option<SyncMethodArg>,

    #[arg(short = 'w', long, default_value = "30")]
    pub window: u32,

    #[arg(long)]
    pub vad_sensitivity: Option<f32>,

    #[arg(long, value_parser = validate_chunk_size)]
    pub vad_chunk_size: Option<usize>,

    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    #[arg(long)]
    pub verbose: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub force: bool,

    #[arg(short, long)]
    pub batch: bool,
}

impl SyncArgs {
    /// 取得同步模式：單檔或批次
    pub fn get_sync_mode(&self) -> Result<SyncMode, SubXError> {
        if !self.input_paths.is_empty() || self.batch {
            let paths = if !self.input_paths.is_empty() {
                self.input_paths.clone()
            } else if let Some(video) = &self.video {
                vec![video.clone()]
            } else {
                return Err(SubXError::NoInputSpecified);
            };
            
            let handler = InputPathHandler::from_args(&paths, self.recursive)?
                .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"]);
            
            Ok(SyncMode::Batch(handler))
        } else if let (Some(video), Some(subtitle)) = (&self.video, &self.subtitle) {
            Ok(SyncMode::Single {
                video: video.clone(),
                subtitle: subtitle.clone(),
            })
        } else {
            Err(SubXError::InvalidSyncConfiguration)
        }
    }
}

#[derive(Debug)]
pub enum SyncMode {
    Single { video: PathBuf, subtitle: PathBuf },
    Batch(InputPathHandler),
}
```

#### DetectEncoding 命令重構

```rust
#[derive(Args, Debug)]
pub struct DetectEncodingArgs {
    /// 要分析的檔案路徑（現有參數，與 -i 互斥）
    #[arg(conflicts_with = "input_paths")]
    pub file_paths: Vec<String>,
    
    /// 指定要處理的檔案或目錄路徑（新增參數）
    #[arg(short = 'i', long = "input", value_name = "PATH", conflicts_with = "file_paths")]
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
    pub fn get_file_paths(&self) -> Result<Vec<PathBuf>, SubXError> {
        if !self.input_paths.is_empty() {
            let handler = InputPathHandler::from_args(&self.input_paths, self.recursive)?
                .with_extensions(&["srt", "ass", "vtt", "ssa", "sub", "txt"]);
            handler.collect_files()
        } else if !self.file_paths.is_empty() {
            let paths: Vec<PathBuf> = self.file_paths.iter().map(PathBuf::from).collect();
            Ok(paths)
        } else {
            Err(SubXError::NoInputSpecified)
        }
    }
}
```

### 3. 錯誤處理（與現有錯誤體系整合）

```rust
// 擴充現有的 SubXError 枚舉
#[derive(Debug, thiserror::Error)]
pub enum SubXError {
    // 現有的錯誤類型...
    
    #[error("未指定輸入路徑")]
    NoInputSpecified,
    
    #[error("無效的路徑: {0}")]
    InvalidPath(PathBuf),
    
    #[error("路徑不存在: {0}")]
    PathNotFound(PathBuf),
    
    #[error("無法讀取目錄: {path}")]
    DirectoryReadError { 
        path: PathBuf,
        #[source]
        source: std::io::Error 
    },
    
    #[error("同步設定無效：請指定影片和字幕檔案，或使用 -i 參數進行批次處理")]
    InvalidSyncConfiguration,
    
    #[error("不支援的檔案類型: {0}")]
    UnsupportedFileType(String),
}
```

## 實作步驟

### 階段 1：基礎設施建立

1. **建立輸入處理模組**
   - 在 `src/cli/` 目錄下建立 `input_handler.rs` 模組
   - 實作 `InputPathHandler` 結構體和相關方法
   - 與現有的 `SubXError` 錯誤體系整合

2. **更新模組匯出**
   - 在 `src/cli/mod.rs` 中新增 `input_handler` 模組匯出
   - 確保錯誤類型正確整合到主要錯誤體系

### 階段 2：命令參數更新（維持向後相容性）

1. **Match 命令**
   - 更新 `src/cli/match_args.rs`
   - 新增 `-i` 參數和相關方法
   - 保持與現有 `--copy`, `--move`, `--backup` 功能的相容性
   - 更新文件和範例

2. **Convert 命令**
   - 更新 `src/cli/convert_args.rs`
   - 新增 `-i` 參數和相關方法
   - **新增 `--recursive` 參數**（目前不存在）
   - 處理與現有 `input` 參數的優先級
   - 與 `--keep-original`, `--encoding` 功能整合

3. **Sync 命令**
   - 更新 `src/cli/sync_args.rs`
   - 新增 `-i` 參數和批次處理模式
   - **新增 `--recursive` 參數**（目前不存在）
   - 與現有 `--batch` 模式整合
   - 實作 `SyncMode` 列舉和相關邏輯

4. **DetectEncoding 命令**
   - 更新 `src/cli/detect_encoding_args.rs`
   - 重構現有的 `file_paths` 參數與新 `-i` 參數
   - **新增 `--recursive` 參數**（目前不存在）
   - 確保參數互斥性（`file_paths` vs `-i`）

### 階段 3：命令實作更新

1. **Match 命令實作**
   - 更新 `src/commands/match_command.rs`
   - 整合新的輸入路徑處理邏輯
   - 確保多路徑處理與現有功能正確運作

2. **Convert 命令實作**
   - 更新 `src/commands/convert_command.rs`
   - 實作批次轉換邏輯
   - 處理輸出檔案命名和衝突解決

3. **Sync 命令實作**
   - 更新 `src/commands/sync_command.rs`
   - 實作批次同步處理
   - 自動配對影片和字幕檔案
   - 與現有的 VAD 和 Manual 方法整合

4. **DetectEncoding 命令實作**
   - 更新 `src/commands/detect_encoding_command.rs`
   - 整合新的檔案路徑收集邏輯
   - 保持現有的詳細輸出格式

### 階段 4：測試實作

1. **單元測試**
   - 建立 `tests/cli/input_handler_tests.rs`
   - 測試路徑解析和驗證邏輯
   - 測試目錄掃描功能
   - 遵循現有的測試模式

2. **整合測試**
   - 更新現有的 CLI 整合測試
   - 新增 `-i` 參數的測試案例
   - 測試多路徑處理和錯誤情況
   - 驗證向後相容性

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
    fn test_input_handler_from_files() {
        // 測試單一檔案輸入
    }
    
    #[test]
    fn test_input_handler_from_directories() {
        // 測試目錄輸入
    }
    
    #[test]
    fn test_input_handler_mixed() {
        // 測試混合檔案和目錄輸入
    }
    
    #[test]
    fn test_input_handler_validation() {
        // 測試路徑驗證邏輯
    }
    
    #[test]
    fn test_directory_collection() {
        // 測試目錄展開和檔案過濾
    }
    
    #[test]
    fn test_recursive_vs_flat_scanning() {
        // 測試遞迴與非遞迴掃描差異
    }
    
    #[test]
    fn test_file_extension_filtering() {
        // 測試檔案副檔名過濾
    }
}
```

### 2. 整合測試重點

1. **CLI 參數解析**
   - 測試 `-i` 參數的多重使用
   - 測試與現有參數的組合和衝突
   - 測試錯誤情況和錯誤訊息

2. **檔案處理**
   - 測試大量檔案的處理效能
   - 測試各種檔案系統結構
   - 測試權限和存取問題

3. **向後相容性**
   - 確保現有的 CLI 介面繼續運作
   - 測試升級場景和移轉路徑
   - 驗證預設行為未改變

4. **與現有功能整合**
   - 測試與 `--copy`, `--move` 的整合
   - 測試與 `--batch` 模式的整合
   - 測試與依賴注入配置系統的整合

### 3. 效能測試

1. **大量檔案處理**
   - 測試處理數千個檔案的效能
   - 記憶體使用量監控
   - 處理時間基準測試

2. **目錄掃描效能**
   - 深層目錄結構的掃描效能
   - 不同檔案系統的效能比較
   - 平行掃描的效能提升潛力

## 使用範例（更新版）

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

# 混合檔案和目錄，結合現有功能
subx match -i ./video1.mp4 -i ./subtitles_dir -i ./video2.mkv --recursive --copy --backup

# 乾跑模式與高信心度閾值
subx match -i ./media --recursive --dry-run --confidence 90
```

### Convert 命令

```bash
# 單一檔案轉換（保持相容性）
subx convert movie.srt --format ass

# 批次轉換多個目錄（僅處理一層）
subx convert -i ./srt_files -i ./more_subtitles --format vtt

# 批次轉換多個目錄（遞迴處理子目錄）
subx convert -i ./srt_files -i ./more_subtitles --format vtt --recursive

# 混合檔案和目錄，保留原檔案
subx convert -i movie1.srt -i ./batch_dir -i movie2.ass --format srt --recursive --keep-original

# 指定編碼轉換
subx convert -i ./chinese_subtitles --format vtt --encoding utf-8 --recursive
```

### Sync 命令

```bash
# 傳統單檔同步（保持相容性）
subx sync --video video.mp4 --subtitle subtitle.srt

# 手動偏移模式
subx sync --offset 2.5 --subtitle subtitle.srt

# 批次同步整個目錄（僅處理一層）
subx sync -i ./movies_directory --batch

# 批次同步整個目錄（遞迴處理子目錄）
subx sync -i ./movies_directory --batch --recursive

# 多個目錄批次同步，指定方法
subx sync -i ./movies1 -i ./movies2 -i ./tv_shows --recursive --batch --method vad

# 批次模式的詳細輸出和乾跑
subx sync -i ./media --batch --recursive --dry-run --verbose
```

### DetectEncoding 命令

```bash
# 現有方式（保持相容性）
subx detect-encoding *.srt

# 使用新參數處理多個目錄（僅處理一層）
subx detect-encoding -i ./subtitles1 -i ./subtitles2 --verbose

# 使用新參數處理多個目錄（遞迴處理子目錄）
subx detect-encoding -i ./subtitles1 -i ./subtitles2 --verbose --recursive

# 混合指定檔案和目錄掃描（互斥，需選擇一種方式）
# 方式一：直接指定檔案
subx detect-encoding file1.srt file2.ass --verbose

# 方式二：使用 -i 參數
subx detect-encoding -i ./more_subtitles -i specific_file.srt --recursive --verbose
```

## 品質保證

### 1. 程式碼品質

- **代碼風格**：遵循 Rust 標準格式和 Clippy 建議
- **錯誤處理**：與現有 `SubXError` 體系整合，提供清晰且有用的錯誤訊息
- **記憶體安全**：避免不必要的記憶體分配和複製
- **效能最佳化**：使用有效率的檔案系統操作
- **依賴注入**：與現有的配置服務系統整合

### 2. 使用者體驗

- **直觀介面**：參數命名和行為符合使用者期望
- **清晰文件**：提供充足的使用範例和說明
- **錯誤回饋**：錯誤訊息清楚指出問題和可能的解決方案
- **向後相容**：確保現有使用者工作流程不受影響
- **進度回饋**：利用現有的進度條和彩色輸出系統

### 3. 可維護性

- **模組化設計**：將共用邏輯提取到可重用的模組
- **充分測試**：達到高測試覆蓋率，特別是邊界情況
- **文件完整**：程式碼註釋和使用者文件保持同步
- **版本管理**：適當的版本號和變更記錄
- **架構一致性**：遵循現有的專案架構模式

## 風險評估與緩解策略

### 1. 相容性風險

**風險**：新參數可能與現有邏輯產生衝突

**緩解策略**：
- 保持現有參數的預設行為不變
- 新參數僅在明確指定時才啟用
- 廣泛的向後相容性測試
- 實作參數互斥性檢查（如 `detect-encoding` 命令）

### 2. 效能風險

**風險**：大量檔案處理可能影響效能

**緩解策略**：
- 實作檔案數量限制和警告
- 提供進度回饋和取消機制
- 使用有效率的檔案系統 API
- 考慮實作平行處理（與現有平行處理系統整合）

### 3. 使用複雜性風險

**風險**：新參數可能增加使用複雜性

**緩解策略**：
- 提供清晰的文件和範例
- 實作智慧預設值
- 提供使用建議和最佳實踐
- 保持現有簡單使用方式的可用性

### 4. 與現有功能衝突風險

**風險**：與 `--batch`、`--copy`、`--move` 等現有功能的整合問題

**緩解策略**：
- 仔細設計參數組合邏輯
- 詳細測試各種參數組合
- 提供清晰的參數組合文件

## 交付成果

### 1. 程式碼交付

- [ ] `src/cli/input_handler.rs` - 輸入路徑處理模組
- [ ] 更新的 CLI 參數結構體（4個命令）
- [ ] 更新的命令實作（4個命令）
- [ ] 新增和更新的測試檔案
- [ ] 錯誤處理體系擴充

### 2. 文件交付

- [ ] 更新的 help 文件和 CLI 說明
- [ ] 更新的 README.md 和使用者指南
- [ ] API 文件和程式碼註釋
- [ ] 測試文件和最佳實踐指南
- [ ] 遷移指南（針對現有使用者）

### 3. 品質確保交付

- [ ] 測試報告和覆蓋率分析
- [ ] 效能基準測試結果
- [ ] 相容性測試報告
- [ ] 使用者體驗評估報告
- [ ] 與現有功能的整合驗證報告

## 後續規劃

### 短期優化

1. **智慧檔案配對**：在批次處理模式下自動配對相關檔案（影片與字幕）
2. **平行處理整合**：利用現有的平行處理系統加速大量檔案處理
3. **進度顯示增強**：提供更詳細的處理進度和狀態顯示

### 長期增強

1. **過濾器支援**：新增檔案過濾和排除規則（例如：排除特定模式）
2. **配置整合**：允許在配置檔案中預設輸入路徑和行為
3. **監控模式**：支援目錄監控和自動處理新檔案
4. **外掛系統**：支援自訂的檔案發現和處理邏輯

### 與現有系統的整合機會

1. **快取系統**：與現有的快取管理系統整合，避免重複處理
2. **配置服務**：與依賴注入的配置系統深度整合
3. **AI 服務**：優化 AI 處理的批次效率
4. **音訊處理**：與音訊處理管線的批次優化

---

**實作狀態**：❌ 未實作  
**預估工時**：12-16 小時  
**優先級**：中等  
**依賴項目**：無  
**後續項目**：可與平行處理優化和使用者體驗改善相結合  

**備註**：這個 backlog 已根據 2025年6月的程式碼現況進行更新，確保與現有架構和功能的良好整合。實作時應特別注意與現有 `--batch`、`--copy`、`--move` 等功能的相容性。
