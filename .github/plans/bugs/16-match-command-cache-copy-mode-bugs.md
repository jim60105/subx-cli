# Bug #16: Match Command Cache 重用與 Copy 模式錯誤修復

## 問題描述

在 `match` 指令的 cache 重用機制和 copy 模式中發現兩個關鍵錯誤：

### 問題 1: Cache 重用時忽略 Copy/Move 參數

**重現步驟：**
1. 執行 `subx-cli match --recursive -c --dry-run .` 建立 cache
2. 執行 `subx-cli match --recursive -c .` (非 dry-run 模式)
3. **問題：** 字幕檔案僅被重新命名，沒有執行 copy 操作

**預期行為：**
- 即使使用 cache，也應該根據命令行參數執行相應的 copy 或 move 操作

### 問題 2: Copy 模式下原始檔案被意外重新命名

**重現步驟：**
1. 清除 cache：`subx-cli cache clear`
2. 執行 `subx-cli match --recursive -c .` (非 dry-run 模式)
3. **問題：** 字幕檔案被複製到正確位置，但原始檔案被重新命名

**預期行為：**
- Copy 模式下，原始檔案應該保持不變，只在目標位置建立副本

## 根本原因分析

### 問題 1 的根本原因
在 `src/core/matcher/engine.rs` 的 `check_cache` 方法中：

```rust
ops.push(MatchOperation {
    // ...existing fields...
    relocation_mode: self.config.relocation_mode.clone(),
    relocation_target_path: None,    // 🚨 Cache 不儲存重定位路徑
    requires_relocation: false,      // 🚨 強制設為 false
});
```

Cache 重建時會將 `relocation_target_path` 設為 `None`，`requires_relocation` 設為 `false`，導致忽略了使用者指定的 copy/move 參數。

### 問題 2 的根本原因
在 `execute_relocation_operation` 方法中：

```rust
let source_path = if op.new_subtitle_name == op.subtitle_file.name {
    op.subtitle_file.path.clone()    // ✅ 使用原始路徑
} else {
    op.subtitle_file.path.with_file_name(&op.new_subtitle_name)  // 🚨 使用重新命名後的路徑
};
```

當檔案已被重新命名時，`source_path` 會指向新位置而非原始位置，導致 copy 操作實際上移動了已重新命名的檔案。

## 修復方案

### 修復 1: 改進 Cache 重用邏輯

#### 1.1 增強 Cache 資料結構

**檔案：** `src/core/matcher/cache.rs`

在 `CacheData` 結構中新增配置資訊：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheData {
    // ...existing fields...
    
    /// 記錄產生 cache 時的重定位模式
    pub original_relocation_mode: String,
    
    /// 記錄是否啟用了 backup
    pub original_backup_enabled: bool,
}
```

#### 1.2 修改 Cache 檢查邏輯

**檔案：** `src/core/matcher/engine.rs`

修改 `check_cache` 方法，正確重新計算重定位資訊：

```rust
// Rebuild match operation list with current configuration
let files = self.discovery.scan_directory(directory, recursive)?;
let mut ops = Vec::new();

for item in cache_data.match_operations {
    if let (Some(video), Some(subtitle)) = (/* find video and subtitle */) {
        // 重新計算重定位需求（基於當前配置，不是 cache 中的配置）
        let requires_relocation = self.config.relocation_mode != FileRelocationMode::None
            && subtitle.path.parent() != video.path.parent();

        let relocation_target_path = if requires_relocation {
            let video_dir = video.path.parent().unwrap();
            Some(video_dir.join(&item.new_subtitle_name))
        } else {
            None
        };

        ops.push(MatchOperation {
            // ...existing fields...
            relocation_mode: self.config.relocation_mode.clone(), // 使用當前配置
            relocation_target_path,
            requires_relocation,
        });
    }
}
```

### 修復 2: 修正 Copy 模式邏輯

#### 2.1 分離重新命名與重定位操作

**檔案：** `src/core/matcher/engine.rs`

修改 `execute_operations` 方法，確保操作順序正確：

```rust
pub async fn execute_operations(&self, operations: &[MatchOperation], dry_run: bool) -> Result<()> {
    for op in operations {
        if dry_run {
            // ...preview logic...
        } else {
            // 根據重定位模式決定操作順序
            match op.relocation_mode {
                FileRelocationMode::Copy => {
                    // Copy 模式：先複製到目標位置，再重新命名複製的檔案
                    if op.requires_relocation {
                        self.execute_copy_then_rename(op).await?;
                    } else {
                        // 只需要重新命名
                        self.rename_file(op).await?;
                    }
                }
                FileRelocationMode::Move => {
                    // Move 模式：先重新命名，再移動
                    self.rename_file(op).await?;
                    if op.requires_relocation {
                        self.execute_relocation_operation(op).await?;
                    }
                }
                FileRelocationMode::None => {
                    // 只重新命名
                    self.rename_file(op).await?;
                }
            }
        }
    }
    Ok(())
}
```

#### 2.2 新增專用的 Copy-then-Rename 方法

```rust
/// Execute copy operation followed by rename of the copied file
async fn execute_copy_then_rename(&self, op: &MatchOperation) -> Result<()> {
    if let Some(target_path) = &op.relocation_target_path {
        // 1. 複製原始檔案到目標位置
        let final_target = self.resolve_filename_conflict(target_path.clone())?;
        
        // Create target directory if needed
        if let Some(parent) = final_target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Backup target if exists and enabled
        if self.config.backup_enabled && final_target.exists() {
            let backup_path = final_target.with_extension(format!(
                "{}.backup",
                final_target.extension().and_then(|s| s.to_str()).unwrap_or("")
            ));
            std::fs::copy(&final_target, backup_path)?;
        }
        
        // Copy original file to target location
        std::fs::copy(&op.subtitle_file.path, &final_target)?;
        
        // 2. 如果需要重新命名原始檔案，則重新命名
        if op.new_subtitle_name != op.subtitle_file.name {
            let renamed_original = op.subtitle_file.path.with_file_name(&op.new_subtitle_name);
            std::fs::rename(&op.subtitle_file.path, &renamed_original)?;
            
            // Display rename operation
            if renamed_original.exists() {
                println!("  ✓ Renamed: {} -> {}", 
                    op.subtitle_file.name, op.new_subtitle_name);
            }
        }
        
        // Display copy operation
        if final_target.exists() {
            println!("  ✓ Copied: {} -> {}", 
                op.subtitle_file.path.file_name().unwrap_or_default().to_string_lossy(),
                final_target.file_name().unwrap_or_default().to_string_lossy());
        }
    }
    Ok(())
}
```

## 測試計畫

### 單元測試

#### 測試 1: Cache 重用正確性測試

**檔案：** `tests/match_cache_reuse_tests.rs`

```rust
#[tokio::test]
async fn test_cache_reuse_preserves_copy_mode() {
    // 1. 建立測試環境
    // 2. 執行 dry-run 建立 cache
    // 3. 執行 copy 模式，驗證：
    //    - 使用了 cache 中的匹配結果
    //    - 正確執行了 copy 操作
    //    - 原始檔案未被移動
}

#[tokio::test]
async fn test_cache_reuse_preserves_move_mode() {
    // 類似上述測試，但驗證 move 模式
}
```

#### 測試 2: Copy 模式行為測試

**檔案：** `tests/match_copy_behavior_tests.rs`

```rust
#[tokio::test]
async fn test_copy_mode_preserves_original_file() {
    // 1. 建立測試檔案
    // 2. 執行 copy 模式匹配
    // 3. 驗證：
    //    - 原始檔案仍存在於原位置
    //    - 目標位置有正確的副本
    //    - 副本內容與原始檔案相同
}

#[tokio::test]
async fn test_copy_mode_with_rename() {
    // 測試需要重新命名的 copy 操作
}
```

### 整合測試

#### 測試場景 1: 完整的 Cache 重用流程

```bash
# 建立測試資料
mkdir -p test_dir/videos test_dir/subtitles
echo "video content" > test_dir/videos/movie.mp4
echo "subtitle content" > test_dir/subtitles/subtitle.srt

# 執行 dry-run
subx-cli match --recursive -c --dry-run test_dir

# 驗證 cache 檔案建立
test -f ~/.config/subx/match_cache.json

# 執行實際 copy 操作
subx-cli match --recursive -c test_dir

# 驗證結果：
# 1. 原始字幕檔案保持不變
test -f test_dir/subtitles/subtitle.srt

# 2. 目標位置有副本
test -f test_dir/videos/movie.srt

# 3. 檔案內容相同
diff test_dir/subtitles/subtitle.srt test_dir/videos/movie.srt
```

#### 測試場景 2: 清除 Cache 後的正常運作

```bash
# 清除 cache
subx-cli cache clear

# 執行 copy 操作
subx-cli match --recursive -c test_dir

# 驗證結果與場景 1 相同
```

## 實作優先順序

### Phase 1: 核心修復 (高優先級)
1. **修復 Cache 重用邏輯** - 立即修復
   - 修改 `check_cache` 方法
   - 確保重定位參數被重新計算

2. **修復 Copy 模式邏輯** - 立即修復
   - 實作 `execute_copy_then_rename` 方法
   - 修改 `execute_operations` 操作順序

### Phase 2: 測試強化 (中優先級)
3. **新增單元測試**
   - Cache 重用測試
   - Copy 模式行為測試

4. **新增整合測試**
   - 端到端流程測試
   - 邊界條件測試

### Phase 3: 改進優化 (低優先級)
5. **增強 Cache 結構**
   - 儲存更多配置資訊
   - 版本相容性檢查

6. **效能優化**
   - 減少重複檔案操作
   - 批次處理優化

## 驗收標準

### 功能正確性
- ✅ Cache 重用時正確執行 copy/move 操作
- ✅ Copy 模式下原始檔案保持不變
- ✅ Move 模式正常運作不受影響
- ✅ 所有現有功能保持正常

### 測試覆蓋率
- ✅ 新功能達到 90%+ 測試覆蓋率
- ✅ 回歸測試通過率 100%
- ✅ 整合測試涵蓋主要使用場景

### 效能標準
- ✅ Cache 重用效能不下降
- ✅ Copy 操作時間合理（< 2x move 操作時間）
- ✅ 記憶體使用量不顯著增加

## 風險評估

### 高風險
- **資料遺失風險**: Copy/Move 邏輯錯誤可能導致檔案遺失
  - **緩解措施**: 充分測試、啟用 backup 功能

### 中風險
- **向後相容性**: Cache 格式變更可能影響現有 cache
  - **緩解措施**: 實作 cache 版本檢查與自動清除

### 低風險
- **效能影響**: 新邏輯可能影響執行效能
  - **緩解措施**: 效能基準測試與優化

## 交付時間表

| 階段 | 任務 | 預估時間 | 依賴項目 |
|------|------|----------|----------|
| Phase 1.1 | Cache 重用邏輯修復 | 2 天 | - |
| Phase 1.2 | Copy 模式邏輯修復 | 3 天 | Phase 1.1 |
| Phase 2.1 | 單元測試新增 | 2 天 | Phase 1.2 |
| Phase 2.2 | 整合測試新增 | 1 天 | Phase 2.1 |
| Phase 3.1 | Cache 結構改進 | 1 天 | Phase 2.2 |
| Phase 3.2 | 效能優化 | 1 天 | Phase 3.1 |

**總計預估時間:** 10 個工作天

## 參考資料

- [Config Usage Analysis](../../docs/config-usage-analysis.md)
- [Tech Architecture](../../docs/tech-architecture.md)
- [Testing Guidelines](../../docs/testing-guidelines.md)
- [Match Command Copy Feature Backlog](../backlogs/25-match-command-copy-to-video-folder.md)

## 更新記錄

| 日期 | 版本 | 變更描述 | 作者 |
|------|------|----------|------|
| 2025-06-12 | 1.0 | 初始版本建立 | GitHub Copilot |
