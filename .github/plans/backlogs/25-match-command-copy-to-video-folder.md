# Product Backlog #25: Match 命令字幕檔案複製/移動至影片資料夾功能

## 背景與問題描述

目前的 `match` 命令具備遞歸搜尋功能，可以掃描整個目錄樹來尋找影片與字幕檔案並進行匹配。然而，在遞歸搜尋模式下，字幕檔案和影片檔案可能分散在不同的資料夾中，匹配成功後字幕檔案仍保留在原始位置，使用者必須手動將字幕檔案移動或複製到與影片檔案相同的資料夾才能正常播放。

### 現有問題場景

```
media/
├── movies/
│   ├── Action/
│   │   └── Movie1.mkv
│   └── Drama/
│       └── Movie2.mp4
└── subtitles/
    ├── english/
    │   ├── Movie1.srt
    │   └── Movie2.srt
    └── chinese/
        ├── Movie1.zh.srt
        └── Movie2.zh.srt
```

在此結構下，即使 AI 成功匹配了檔案對應關係，字幕檔案仍分散在 `subtitles` 目錄中，播放器無法自動載入字幕。

### 用戶需求

使用者希望在匹配成功後，系統能夠自動將字幕檔案複製或移動到與對應影片檔案相同的資料夾中，實現以下目標：

1. **自動檔案組織**：匹配成功的字幕檔案自動複製或移動到影片檔案所在資料夾
2. **播放器相容性**：符合主流播放器的檔案載入慣例
3. **檔案操作彈性**：提供複製和移動兩種操作模式供使用者選擇
4. **彈性控制**：提供參數讓使用者選擇操作模式

## 目標與價值

### 主要目標

1. **簡化媒體管理流程**：減少使用者手動整理檔案的工作量
2. **提升播放體驗**：讓播放器能夠自動載入對應的字幕檔案
3. **提供操作模式選擇**：支援複製（保留原檔）和移動（清理原位置）兩種操作
4. **維持系統一致性**：與現有的 dry-run、backup 等功能保持一致的設計模式

### 業務價值

- **提升使用者體驗**：簡化媒體檔案的管理和播放流程
- **減少操作錯誤**：避免使用者手動移動檔案時的錯誤
- **增強工具實用性**：讓 SubX 成為更完整的媒體管理解決方案

## 詳細功能規格

### 1. 新增命令列參數

#### 參數定義

**複製模式參數**:
- **參數名稱**: `--copy` 或簡寫 `-c`
- **類型**: 布林型標誌參數
- **預設值**: `false`
- **描述**: 將匹配成功的字幕檔案複製到對應影片檔案所在的資料夾，保留原始檔案

**移動模式參數**:
- **參數名稱**: `--move` 或簡寫 `-m`
- **類型**: 布林型標誌參數
- **預設值**: `false`
- **描述**: 將匹配成功的字幕檔案移動到對應影片檔案所在的資料夾，刪除原始檔案

#### 參數互斥性
兩個參數不能同時使用，同時指定時應產生錯誤：
```
Error: --copy and --move cannot be used together. Please choose one operation mode.
```

#### 參數行為
```bash
# 複製模式：保留原始檔案
subx match /media --recursive --copy

# 移動模式：刪除原始檔案
subx match /media --recursive --move

# 與其他參數組合使用
subx match /media --recursive --copy --dry-run --backup

# 錯誤：同時使用兩個參數
subx match /media --recursive --copy --move
# Error: --copy and --move cannot be used together.
```

### 2. 檔案操作邏輯

#### 操作條件判斷
1. **匹配成功**: AI 匹配信心度超過設定閾值
2. **路徑不同**: 字幕檔案與影片檔案不在同一資料夾
3. **參數啟用**: 使用者明確指定 `--copy` 或 `--move` 參數
4. **目標可寫入**: 影片檔案所在資料夾具有寫入權限

#### 操作策略
```rust
// File operation logic (English comments)
match (args.copy, args.move) {
    (true, true) => {
        return Err(Error::new("Cannot use --copy and --move together"));
    }
    (true, false) => {
        // Copy operation: preserve original file
        if subtitle_path.parent() != video_path.parent() {
            let target_dir = video_path.parent().unwrap();
            let target_path = target_dir.join(subtitle_file.file_name().unwrap());
            
            // Handle filename conflicts
            let final_target = resolve_filename_conflict(target_path);
            
            // Execute copy operation using parallel processing
            file_operations.push(FileOperation::Copy {
                source: subtitle_path.clone(),
                target: final_target,
            });
        }
    }
    (false, true) => {
        // Move operation: remove original file
        if subtitle_path.parent() != video_path.parent() {
            let target_dir = video_path.parent().unwrap();
            let target_path = target_dir.join(subtitle_file.file_name().unwrap());
            
            // Handle filename conflicts
            let final_target = resolve_filename_conflict(target_path);
            
            // Execute move operation using parallel processing
            file_operations.push(FileOperation::Move {
                source: subtitle_path.clone(),
                target: final_target,
            });
        }
    }
    (false, false) => {
        // No file relocation operation
    }
}
```

#### 檔名衝突處理
當目標資料夾已存在同名字幕檔案時，採用以下策略：

1. **檔案內容比較**: 比較檔案大小和修改時間
2. **自動重命名**: 在檔名末尾添加數字後綴（如 `movie.srt` -> `movie.1.srt`）
3. **使用者選擇**: 在互動模式下詢問使用者處理方式
4. **跳過處理**: 在非互動模式下跳過衝突檔案並記錄警告

#### 並行處理整合
利用現有的並行檔案處理系統來提升性能：

```rust
// Integration with existing parallel processing system
use crate::core::parallel::{
    FileProcessingTask, ProcessingOperation, Task, TaskResult, TaskScheduler,
};

impl FileProcessingTask for FileRelocationTask {
    fn execute(&self) -> TaskResult {
        match &self.operation {
            ProcessingOperation::CopyToVideoFolder { source, target } => {
                // Execute copy operation
                match std::fs::copy(source, target) {
                    Ok(_) => TaskResult::Success(format!("Copied: {} -> {}", 
                        source.display(), target.display())),
                    Err(e) => TaskResult::Failure(format!("Copy failed: {}", e)),
                }
            }
            ProcessingOperation::MoveToVideoFolder { source, target } => {
                // Execute move operation
                match std::fs::rename(source, target) {
                    Ok(_) => TaskResult::Success(format!("Moved: {} -> {}", 
                        source.display(), target.display())),
                    Err(e) => TaskResult::Failure(format!("Move failed: {}", e)),
                }
            }
        }
    }
}
```

### 3. Dry-run 模式整合

#### 預覽顯示
在 dry-run 模式下，系統應顯示：
```
Preview Operations:
┌─────────┬─────────────────────────────────────────────────┐
│ Status  │ File Operations                                 │
├─────────┼─────────────────────────────────────────────────┤
│ 🔍      │ Video:     /media/movies/Action/Movie1.mkv      │
│         │ Subtitle:  /media/subtitles/Movie1.srt          │
│         │ → Rename:  Movie1.srt                           │
│         │ → Copy to: /media/movies/Action/Movie1.srt      │
├─────────┼─────────────────────────────────────────────────┤
│ 🔍      │ Video:     /media/movies/Drama/Movie2.mp4       │
│         │ Subtitle:  /media/subtitles/Movie2.zh.srt       │
│         │ → Rename:  Movie2.zh.srt                        │
│         │ → Move to: /media/movies/Drama/Movie2.zh.srt    │
└─────────┴─────────────────────────────────────────────────┘
```

#### 預覽功能要求
1. **清楚標示操作類型**: 顯示 "Copy to" 或 "Move to" 資訊行
2. **檔案路徑完整性**: 顯示完整的來源和目標路徑
3. **衝突警告**: 預先檢測檔名衝突並顯示警告
4. **操作計數**: 統計總共將執行的複製/移動操作數量

### 4. 備份功能整合

#### 備份策略
當同時啟用 `--backup` 和檔案重新定位功能時：

1. **原始檔案備份**: 
   - 複製模式：在字幕檔案原始位置建立 `.backup` 檔案
   - 移動模式：在字幕檔案原始位置建立 `.backup` 檔案（移動前備份）
2. **目標檔案備份**: 如果目標位置已存在同名檔案，先備份既有檔案
3. **備份命名規則**: 保持與現有備份功能一致的命名慣例

#### 備份操作順序
```
1. Check if target location has existing file with same name
2. If exists, create backup of target file
3. Create backup of original subtitle file (for both copy and move)
4. Execute rename operation (if needed)
5. Execute copy/move operation to video folder
```

### 5. 錯誤處理與復原

#### 錯誤類型與處理
1. **權限錯誤**: 目標資料夾無寫入權限
   - 記錄錯誤並繼續處理其他檔案
   - 在總結報告中顯示失敗的操作

2. **磁碟空間不足**: 目標磁碟空間不足
   - 中止當前操作
   - 提供清晰的錯誤訊息和建議

3. **檔案系統錯誤**: I/O 錯誤或檔案系統問題
   - 記錄詳細錯誤資訊
   - 提供復原操作建議

#### 部分失敗處理
當批次操作中部分檔案複製失敗時：
- 繼續處理其他成功的檔案
- 在操作完成後顯示詳細的成功/失敗報告
- 提供重試失敗操作的建議命令

### 6. 進度顯示與回饋

#### 進度指示器
```
Matching files: [████████████████████] 100% (25/25)
Processing subtitles: [██████░░░░░░░░░] 40% (10/25)
Status: Copying Movie1.srt to /media/movies/Action/
```

#### 完成總結
```
Match Operation Summary:
├─ Files matched: 25
├─ Files renamed: 23
├─ Files copied to video folders: 15
├─ Files moved to video folders: 5
├─ Copy/move operations skipped: 3 (same folder)
├─ Copy/move operations failed: 2 (permission denied)
└─ Total processing time: 1.2s
```

## 技術實作規格

### 1. 程式碼結構變更

#### 1.1 命令列參數擴充

**檔案位置**: `src/cli/match_args.rs`

**修改內容**:
```rust
#[derive(Args, Debug)]
pub struct MatchArgs {
    // ...existing fields...
    
    /// Copy matched subtitle files to the same folder as their corresponding video files.
    ///
    /// When enabled along with recursive search, subtitle files that are matched
    /// with video files in different directories will be copied to the video file's
    /// directory. This ensures that media players can automatically load subtitles.
    /// The original subtitle files are preserved in their original locations.
    #[arg(long, short = 'c')]
    pub copy: bool,

    /// Move matched subtitle files to the same folder as their corresponding video files.
    ///
    /// When enabled along with recursive search, subtitle files that are matched
    /// with video files in different directories will be moved to the video file's
    /// directory. This ensures that media players can automatically load subtitles.
    /// The original subtitle files are removed from their original locations.
    #[arg(long, short = 'm')]
    pub move_files: bool,
}

impl MatchArgs {
    /// Validate that copy and move arguments are not used together
    pub fn validate(&self) -> Result<(), String> {
        if self.copy && self.move_files {
            return Err("Cannot use --copy and --move together. Please choose one operation mode.".to_string());
        }
        Ok(())
    }
}
```

#### 1.2 匹配操作資料結構擴充

**檔案位置**: `src/core/matcher/engine.rs`

**新增欄位**:
```rust
#[derive(Debug, Clone)]
pub struct MatchOperation {
    // ...existing fields...
    
    /// File relocation mode
    pub relocation_mode: FileRelocationMode,
    
    /// Target relocation path if operation is needed
    pub relocation_target_path: Option<PathBuf>,
    
    /// Whether relocation operation is needed (different folders)
    pub requires_relocation: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileRelocationMode {
    /// No file relocation
    None,
    /// Copy subtitle file to video folder
    Copy,
    /// Move subtitle file to video folder
    Move,
}
```

#### 1.3 並行檔案處理整合

**檔案位置**: `src/core/parallel/mod.rs`

**新增處理操作類型**:
```rust
#[derive(Debug, Clone)]
pub enum ProcessingOperation {
    // ...existing operations...
    
    /// Copy subtitle file to video folder
    CopyToVideoFolder {
        source: PathBuf,
        target: PathBuf,
    },
    
    /// Move subtitle file to video folder
    MoveToVideoFolder {
        source: PathBuf,
        target: PathBuf,
    },
}

/// File relocation task for parallel processing
#[derive(Debug)]
pub struct FileRelocationTask {
    pub operation: ProcessingOperation,
    pub backup_enabled: bool,
}

impl Task for FileRelocationTask {
    type Output = TaskResult;

    fn execute(&self) -> Self::Output {
        match &self.operation {
            ProcessingOperation::CopyToVideoFolder { source, target } => {
                self.execute_copy_operation(source, target)
            }
            ProcessingOperation::MoveToVideoFolder { source, target } => {
                self.execute_move_operation(source, target)
            }
            _ => TaskResult::Failure("Invalid operation for FileRelocationTask".to_string()),
        }
    }
}

impl FileRelocationTask {
    fn execute_copy_operation(&self, source: &PathBuf, target: &PathBuf) -> TaskResult {
        // Create backup if enabled
        if self.backup_enabled && target.exists() {
            if let Err(e) = self.backup_existing_file(target) {
                return TaskResult::Failure(format!("Backup failed: {}", e));
            }
        }
        
        // Execute copy operation
        match std::fs::copy(source, target) {
            Ok(_) => TaskResult::Success(format!("Copied: {} -> {}", 
                source.display(), target.display())),
            Err(e) => TaskResult::Failure(format!("Copy failed: {}", e)),
        }
    }
    
    fn execute_move_operation(&self, source: &PathBuf, target: &PathBuf) -> TaskResult {
        // Create backup if enabled
        if self.backup_enabled {
            // Backup original file before moving
            if let Err(e) = self.backup_existing_file(source) {
                return TaskResult::Failure(format!("Backup failed: {}", e));
            }
            
            // Backup target file if exists
            if target.exists() {
                if let Err(e) = self.backup_existing_file(target) {
                    return TaskResult::Failure(format!("Target backup failed: {}", e));
                }
            }
        }
        
        // Execute move operation
        match std::fs::rename(source, target) {
            Ok(_) => TaskResult::Success(format!("Moved: {} -> {}", 
                source.display(), target.display())),
            Err(e) => TaskResult::Failure(format!("Move failed: {}", e)),
        }
    }
}
```

### 2. 設定整合

#### 2.1 匹配配置擴充

**檔案位置**: `src/core/matcher/mod.rs`

**配置擴充**:
```rust
#[derive(Debug, Clone)]
pub struct MatchConfig {
    // ...existing fields...
    
    /// File relocation mode
    pub relocation_mode: FileRelocationMode,
    
    /// Strategy for handling filename conflicts during relocation
    pub conflict_resolution: ConflictResolution,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileRelocationMode {
    /// No file relocation
    None,
    /// Copy subtitle files to video folders
    Copy,
    /// Move subtitle files to video folders
    Move,
}

#[derive(Debug, Clone)]
pub enum ConflictResolution {
    /// Skip relocation if conflict exists
    Skip,
    /// Automatically rename with numeric suffix
    AutoRename,
    /// Prompt user for decision (interactive mode only)
    Prompt,
}
```

### 3. 顯示與 UI 整合

#### 3.1 表格顯示擴充

**檔案位置**: `src/cli/table.rs`

**顯示邏輯修改**:
```rust
impl From<&MatchOperation> for Vec<MatchDisplayRow> {
    fn from(op: &MatchOperation) -> Self {
        let mut rows = Vec::new();
        
        // Basic operation rows (existing logic)
        // ...
        
        // Add relocation operation row if needed
        match op.relocation_mode {
            FileRelocationMode::Copy if op.requires_relocation => {
                if let Some(target) = &op.relocation_target_path {
                    rows.push(MatchDisplayRow {
                        status: "�".to_string(),
                        filename: format!("→ Copy to: {}", target.display()),
                    });
                }
            }
            FileRelocationMode::Move if op.requires_relocation => {
                if let Some(target) = &op.relocation_target_path {
                    rows.push(MatchDisplayRow {
                        status: "📁".to_string(),
                        filename: format!("→ Move to: {}", target.display()),
                    });
                }
            }
            _ => {}
        }
        
        rows
    }
}
```

#### 3.2 進度顯示增強

**新增進度追蹤**:
```rust
pub struct MatchProgress {
    // ...existing fields...
    
    pub relocation_operations_total: usize,
    pub copy_operations_completed: usize,
    pub move_operations_completed: usize,
    pub relocation_operations_failed: usize,
}
```

### 4. 測試策略

#### 4.1 單元測試

**測試檔案**: `tests/match_copy_operation_tests.rs`

**測試案例**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_copy_to_video_folder_basic() {
        // Test basic copy operation
    }
    
    #[tokio::test]
    async fn test_move_to_video_folder_basic() {
        // Test basic move operation
    }
    
    #[tokio::test]
    async fn test_copy_and_move_mutual_exclusion() {
        // Test that copy and move cannot be used together
        let args = MatchArgs {
            copy: true,
            move_files: true,
            ..Default::default()
        };
        assert!(args.validate().is_err());
    }
    
    #[tokio::test]
    async fn test_filename_conflict_resolution() {
        // Test filename conflict handling for both copy and move
    }
    
    #[tokio::test]
    async fn test_backup_with_copy_operation() {
        // Test copy operation with backup enabled
    }
    
    #[tokio::test]
    async fn test_backup_with_move_operation() {
        // Test move operation with backup enabled
    }
    
    #[tokio::test]
    async fn test_parallel_processing_integration() {
        // Test integration with parallel processing system
    }
    
    #[tokio::test]
    async fn test_permission_denied_handling() {
        // Test error handling for permission issues
    }
    
    #[tokio::test]
    async fn test_dry_run_preview_copy_move() {
        // Test dry-run mode display for both operations
    }
}
```

#### 4.2 整合測試

**測試檔案**: `tests/match_integration_copy_tests.rs`

**測試場景**:
1. 遞歸匹配配合複製/移動功能的端到端測試
2. 大量檔案的複製/移動性能測試
3. 複雜目錄結構的檔案重新定位操作測試
4. 錯誤恢復和部分失敗場景測試
5. 並行處理系統的整合測試
6. 參數互斥性驗證測試

## 實作時程與里程碑

### 階段一：核心功能實作（第 1-2 周）

#### 里程碑 1.1：命令列參數與資料結構 (3 天)
- [ ] 擴充 `MatchArgs` 結構體加入 `copy` 和 `move_files` 參數
- [ ] 實作參數互斥性驗證邏輯 `validate()` 方法
- [ ] 更新 `MatchOperation` 結構體支援檔案重新定位操作
- [ ] 擴充 `MatchConfig` 結構體加入重新定位相關設定
- [ ] 實作參數解析和驗證的單元測試

#### 里程碑 1.2：並行檔案處理整合 (4 天)
- [ ] 擴充 `ProcessingOperation` 枚舉支援複製和移動操作
- [ ] 實作 `FileRelocationTask` 結構體整合並行處理系統
- [ ] 實作複製操作的並行執行邏輯 `execute_copy_operation`
- [ ] 實作移動操作的並行執行邏輯 `execute_move_operation`
- [ ] 整合現有的任務調度器支援檔案重新定位操作

#### 里程碑 1.3：檔案操作核心邏輯 (4 天)
- [ ] 實作檔名衝突解決邏輯 `resolve_filename_conflict`
- [ ] 整合現有備份功能支援複製和移動操作
- [ ] 實作基本錯誤處理與重試邏輯
- [ ] 更新匹配引擎支援檔案重新定位模式判斷

### 階段二：UI 與顯示功能 (第 3 周)

#### 里程碑 2.1：Dry-run 模式支援 (3 天)
- [ ] 擴充預覽顯示支援複製和移動操作資訊
- [ ] 實作檔案重新定位操作的衝突預檢功能
- [ ] 更新表格顯示格式加入複製/移動操作行
- [ ] 實作重新定位操作統計資訊顯示

#### 里程碑 2.2：進度顯示與回饋 (2 天)
- [ ] 加入複製/移動操作的進度追蹤
- [ ] 實作檔案重新定位操作的即時狀態顯示
- [ ] 更新完成總結報告格式包含兩種操作模式
- [ ] 實作錯誤與警告訊息的英文本地化

#### 里程碑 2.3：使用者體驗優化 (2 天)
- [ ] 實作檔案權限預檢與警告
- [ ] 加入磁碟空間檢查與提醒
- [ ] 優化大量檔案處理的用戶體驗（利用並行處理）
- [ ] 實作操作取消與中斷處理
- [ ] 確保參數互斥性的使用者友善錯誤訊息

### 階段三：測試與文件 (第 4 周)

#### 里程碑 3.1：完整測試覆蓋 (4 天)
- [ ] 實作所有單元測試案例（包含參數互斥性測試）
- [ ] 建立整合測試套件（包含並行處理整合測試）
- [ ] 實作性能基準測試（複製 vs 移動操作）
- [ ] 建立錯誤場景測試（權限、空間、衝突等）

#### 里程碑 3.2：文件與範例 (2 天)
- [ ] 更新 README 和使用說明（包含複製/移動兩種模式）
- [ ] 建立功能使用範例（展示參數使用方式）
- [ ] 撰寫 API 文件註解（英文）
- [ ] 建立故障排除指南（涵蓋參數衝突處理）

#### 里程碑 3.3：品質保證與部署準備 (1 天)
- [ ] 執行完整的迴歸測試
- [ ] 進行程式碼審查與優化
- [ ] 確認所有文件完整性
- [ ] 準備功能發布說明

## 風險評估與緩解策略

### 技術風險

#### 風險 1：檔案系統權限問題
**影響**: 使用者可能在沒有寫入權限的資料夾中無法執行複製操作
**緩解**:
- 實作預先權限檢查
- 提供清晰的錯誤訊息和解決建議
- 支援部分成功操作，不因單一失敗中止整個流程

#### 風險 2：大量檔案處理的性能問題
**影響**: 處理大量檔案時可能導致系統回應緩慢
**緩解**:
- 利用現有的並行處理系統提升性能
- 加入進度顯示避免使用者感知卡頓
- 提供檔案大小閾值設定，大檔案給予特別處理
- 實作任務優先級調度，重要操作優先執行

#### 風險 3：磁碟空間不足風險
**影響**: 複製操作失敗，移動操作可能造成檔案遺失
**緩解**:
- 實作操作前的磁碟空間檢查
- 提供空間不足的警告和建議
- 支援空間預估功能
- 移動模式優先於複製模式的空間使用建議

### 相容性風險

#### 風險 4：向下相容性問題
**影響**: 現有使用者的工作流程可能受到影響
**緩解**:
- 新功能預設為關閉狀態
- 保持現有參數的行為不變
- 提供清晰的遷移指南

#### 風險 5：參數使用錯誤風險
**影響**: 使用者可能同時使用互斥參數導致混淆
**緩解**:
- 實作清晰的參數驗證和錯誤訊息
- 提供使用範例和文件說明
- 在 help 訊息中明確說明參數互斥性
- CLI 提供友善的建議修正命令

#### 風險 6：不同檔案系統的相容性
**影響**: 在不同操作系統或檔案系統上行為可能不一致
**緩解**:
- 實作跨平台檔案操作抽象層
- 充分測試各種檔案系統環境
- 提供特定環境的設定選項
- 整合現有的檔案操作框架確保一致性

## 成功指標與驗收標準

### 功能性指標

1. **基本功能完整性**
   - [ ] 參數 `--copy` 和 `--move` 能正確解析和執行
   - [ ] 參數互斥性驗證正常工作，同時使用時產生清晰錯誤訊息
   - [ ] 複製操作只在字幕和影片位於不同資料夾時執行
   - [ ] 移動操作正確移除原始檔案並保持檔案完整性
   - [ ] 支援所有現有的字幕和影片格式

2. **整合功能相容性**
   - [ ] 與 `--dry-run` 模式完全相容，正確預覽複製/移動操作
   - [ ] 與 `--backup` 功能正確整合，支援雙重備份
   - [ ] 與 `--recursive` 模式無縫協作
   - [ ] 與現有的信心度篩選功能協作正常
   - [ ] 與並行處理系統完全整合，提供性能優勢

3. **錯誤處理能力**
   - [ ] 權限不足時提供清晰錯誤訊息並繼續處理其他檔案
   - [ ] 磁碟空間不足時安全中止操作並提供恢復建議
   - [ ] 檔名衝突時能自動重命名或提供使用者選擇
   - [ ] 部分失敗不影響整體操作完成度
   - [ ] 移動操作失敗時能夠恢復原始檔案狀態

### 性能指標

1. **處理效率**
   - [ ] 1000 個檔案的複製操作在 30 秒內完成（SSD 環境）
   - [ ] 1000 個檔案的移動操作在 15 秒內完成（SSD 環境）
   - [ ] 記憶體使用量不超過基準值的 150%
   - [ ] 支援中斷與恢復功能，避免重複處理
   - [ ] 並行處理系統有效提升多檔案操作性能

2. **使用者體驗**
   - [ ] 複製/移動操作有清晰的進度指示
   - [ ] Dry-run 預覽資訊完整且易於理解
   - [ ] 操作完成後提供詳細的成功/失敗報告
   - [ ] 參數錯誤訊息清晰且提供解決建議

### 品質指標

1. **程式碼品質**
   - [ ] 測試覆蓋率達到 95% 以上
   - [ ] 所有新增程式碼通過 clippy 檢查
   - [ ] 符合專案既有的程式碼風格和架構模式

2. **文件完整性**
   - [ ] 所有公開 API 具有完整的 rustdoc 註解
   - [ ] 使用說明包含充足的範例和故障排除資訊
   - [ ] 變更記錄清楚說明新功能和潛在的影響

## 後續擴充計劃

### 短期擴充（下個版本）

1. **智慧檔案操作策略**
   - 根據檔案大小和磁碟空間自動選擇複製或移動策略
   - 支援符號連結作為輕量級替代方案
   - 實作檔案完整性驗證（如 checksum 比較）
   - 提供混合模式：小檔案複製，大檔案移動

2. **批次操作優化**
   - 支援檔案重新定位操作的批次確認模式
   - 實作操作的撤銷功能（複製和移動）
   - 加入操作歷史記錄和查詢功能
   - 整合更智慧的並行調度算法

### 中期擴充（未來 2-3 個版本）

1. **進階檔案管理**
   - 支援不同檔案操作策略（硬連結、軟連結、實體複製、移動）
   - 實作檔案同步功能，支援增量更新
   - 加入檔案壓縮和解壓縮支援
   - 實作分散式檔案處理能力

2. **工作流程整合**
   - 支援自訂後處理腳本執行
   - 整合外部媒體管理工具
   - 實作工作流程模板和自動化
   - 提供 API 接口供其他工具整合

這個功能的實作將顯著提升 SubX 工具的實用性，使其從單純的檔案匹配工具進化為完整的媒體檔案管理解決方案。透過提供複製和移動兩種操作模式，使用者可以根據自己的需求選擇最適合的檔案管理策略，為使用者提供更流暢和彈性的媒體觀看體驗。

## 實作經驗與教訓 (2025-06-11 更新)

### 失敗實作分析 (Commits: 67a44b3, bfdbc59)

#### 關鍵錯誤與學習點

##### 1. **架構設計根本性錯誤** ❌
**問題描述**：第一次實作完全繞過了 AI 匹配邏輯，直接基於檔名進行檔案操作
```rust
// 錯誤的實作方式
if args.copy || args.move_files {
    // 直接檔名匹配，跳過 AI 分析
    if file_stem_matches { copy_or_move() }
}
```

**教訓**：
- 新功能必須建構在現有核心功能之上，而非繞過它們
- AI 匹配是專案的核心價值，任何檔案操作都應基於 AI 分析結果
- 架構設計階段必須深入理解專案整體架構

**正確設計**：
```rust
// 正確的流程應該是
// 1. 執行完整的 AI 匹配分析
// 2. 基於信心度閾值篩選匹配結果  
// 3. 執行重新命名操作
// 4. 根據參數執行檔案重新定位操作
```

##### 2. **參數驗證邏輯未整合** ❌
**問題描述**：雖然實作了 `MatchArgs::validate()` 方法，但未在 CLI 執行流程中調用
**位置**：`src/cli/mod.rs:245-274`

**教訓**：
- 參數驗證必須在執行邏輯的最開始就進行
- 測試時必須驗證錯誤條件，不只是成功條件
- 整合測試應該涵蓋完整的 CLI 執行流程

**修復方案**：
```rust
// 需要在 CLI 執行時加入
Commands::Match(args) => {
    args.validate().map_err(|e| crate::Error::InvalidArguments(e))?;
    crate::commands::match_command::execute(args, config_service).await?;
}
```

##### 3. **測試設計不足** ⚠️
**問題描述**：
- 測試只涵蓋基本檔案操作，未測試與 AI 匹配的整合
- 缺少參數互斥性的整合測試
- 缺少錯誤處理和邊界條件測試

**教訓**：
- 測試必須涵蓋完整的使用場景，不只是單一功能
- 錯誤處理測試與成功路徑測試同等重要
- 整合測試應該模擬真實使用者操作流程

##### 4. **規格理解偏差** ⚠️
**問題描述**：將複雜的 AI 整合需求簡化為基本檔案操作
**根本原因**：
- 未充分研讀和理解規格文件的完整需求
- 低估了與現有系統整合的複雜度
- 過早開始程式碼實作，缺乏充分的設計階段

**教訓**：
- 實作前必須完全理解規格文件的每一個細節
- 複雜功能需要分階段設計和實作
- 架構設計比程式碼實作更重要

#### 程式碼審查發現的問題

##### 技術債務問題
1. **檔案衝突處理邏輯未使用**：實作了 `resolve_filename_conflict` 但未整合
2. **並行處理整合不完整**：`FileRelocationTask` 未實作 `Task` trait
3. **配置系統未整合**：重新定位設定未與配置檔案系統整合

##### 品質評估結果
- 功能完整性：40/100 （重大功能缺失）
- 規格符合度：30/100 （與規格差異較大）
- 程式碼品質：60/100 （基本品質達標但架構問題）
- 測試品質：40/100 （基本測試但涵蓋不足）

### 重新實作指導原則

#### 設計階段指導原則
1. **深度規格分析**：
   - 逐條分析功能需求和技術需求
   - 繪製完整的資料流程圖
   - 識別所有整合點和相依性

2. **架構優先設計**：
   - 確保新功能與現有系統的一致性
   - 設計清晰的抽象層和介面
   - 考慮擴展性和維護性

3. **分階段實作**：
   ```
   階段 1: 參數處理和驗證（包含 CLI 整合）
   階段 2: AI 匹配整合和配置擴充
   階段 3: 檔案重新定位邏輯和並行處理
   階段 4: UI 顯示和使用者體驗
   階段 5: 全面測試和文件
   ```

#### 實作階段指導原則
1. **測試驅動開發**：
   - 先寫失敗的測試，再實作功能
   - 每個階段完成後進行回歸測試
   - 包含單元測試、整合測試和端到端測試

2. **持續驗證**：
   - 每次提交都執行完整測試套件
   - 定期與規格文件對照檢查
   - 進行階段性程式碼審查

3. **增量交付**：
   - 每個階段都應該產生可工作的軟體
   - 避免大型單次提交
   - 保持 Git 歷史的清晰性

#### 品質保證指導原則
1. **程式碼品質**：
   - 必須通過 `cargo clippy -- -D warnings`
   - 程式碼覆蓋率達到 95% 以上
   - 符合專案既有的程式碼風格

2. **整合品質**：
   - 所有新功能必須與現有系統無縫整合
   - 不破壞現有功能的向下相容性
   - 提供清晰的錯誤訊息和使用者回饋

3. **文件品質**：
   - 完整的 API 文件（英文註解）
   - 使用者指南和範例（中文）
   - 設計決策和架構說明

### 重要提醒事項

#### 絕對避免的錯誤
1. **繞過核心功能**：任何新功能都不能繞過 AI 匹配系統
2. **忽略參數驗證**：所有參數驗證都必須在 CLI 層級執行
3. **不完整的測試**：測試必須涵蓋成功和失敗的所有路徑
4. **架構不一致**：新程式碼必須遵循現有的架構模式

#### 成功的關鍵因素
1. **完整理解需求**：實作前花充分時間理解規格
2. **正確的架構設計**：在現有系統基礎上擴展，而非重新發明
3. **全面的測試策略**：測試驅動開發和持續驗證
4. **階段性交付**：避免大型單次變更，分階段增量實作

### 相關文件參考
- 程式碼審查報告：`.github/codex/100-backlog-25-code-review-report.md`
- 失敗實作報告：`.github/codex/99-backlog-25-copy-move-logic-tests-report.md`
- Git 提交歷史：`67a44b3`, `bfdbc59` (已 revert)

這些經驗教訓將確保下次實作能夠避免相同的錯誤，並建構出符合專案標準和使用者需求的高品質功能。
