# Bug #17: Sync 指令 --offset 參數使用時完全忽略視頻檔案需求

## 問題描述

當 `sync` 指令使用 `--offset` 參數時，應該完全忽略視頻檔案，僅對字幕檔案應用指定的時間偏移量。目前的實作仍然要求使用者提供視頻檔案參數，即使在手動偏移模式下並不需要進行音訊分析。

### 當前行為

```bash
# 目前必須提供視頻檔案，即使不會使用
subx-cli sync video.mp4 subtitle.srt --offset 2.5
```

### 期望行為

```bash
# 應該能夠只提供字幕檔案，完全跳過視頻檔案
subx-cli sync --offset 2.5 subtitle.srt
```

## 根本原因分析

### 1. CLI 參數結構設計問題

**檔案位置：** `src/cli/sync_args.rs`

```rust
#[derive(Args, Debug)]
pub struct SyncArgs {
    /// Video file path for audio analysis. (目前為必填)
    pub video: PathBuf,
    
    /// Subtitle file path to be synchronized. (目前為必填)
    pub subtitle: PathBuf,
    
    /// Manual time offset in seconds (overrides automatic detection).
    #[arg(long)]
    pub offset: Option<f64>,
    // ... 其他欄位
}
```

**問題：** `video` 欄位被定義為必填參數，無法根據 `--offset` 的存在與否動態調整為可選。

### 2. 命令執行邏輯未優化

**檔案位置：** `src/commands/sync_command.rs:335-390`

```rust
async fn execute_sync_logic(
    args: &SyncArgs,
    app_config: crate::config::Config,
    sync_engine: SyncEngine,
) -> Result<()> {
    // 問題：即使在手動模式下，仍執行對話檢測
    if app_config.sync.enable_dialogue_detection {
        let detector = DialogueDetector::new(&app_config.sync);
        let segs = detector.detect_dialogue(&args.video).await?; // 仍然需要視頻檔案
        // ...
    }

    if let Some(manual_offset) = args.offset {
        // 手動同步模式：應該完全跳過視頻處理
        let mut subtitle = load_subtitle(&args.subtitle).await?;
        sync_engine.apply_sync_offset(&mut subtitle, manual_offset as f32)?;
        save_subtitle(&subtitle, &args.subtitle).await?;
        println!("✓ Applied manual offset: {}s", manual_offset);
    }
    // ...
}
```

**問題：** 即使在手動偏移模式下，程式碼仍然嘗試存取視頻檔案進行對話檢測。

### 3. 參數驗證邏輯不完整

目前沒有在命令開始執行前驗證參數組合的合理性，導致不必要的檔案存取和處理。

## 解決方案設計

### 階段 1: CLI 參數結構重新設計

#### 1.1 修改 SyncArgs 結構

**目標：** 讓視頻檔案在手動偏移模式下變為可選。

**實作方式：條件性必填參數**

```rust
#[derive(Args, Debug)]
pub struct SyncArgs {
    /// Video file path for audio analysis (required for automatic sync).
    /// Optional when using --offset for manual synchronization.
    #[arg(required_unless_present = "offset")]
    pub video: Option<PathBuf>,
    
    /// Subtitle file path to be synchronized.
    pub subtitle: PathBuf,
    
    /// Manual time offset in seconds (overrides automatic detection).
    /// When specified, video file is not required.
    #[arg(long)]
    pub offset: Option<f64>,
    
    // ... 其他欄位維持不變
}
```

**技術要點：**
- 使用 `required_unless_present = "offset"` 讓視頻參數在有 --offset 時變為可選
- 保持原有的位置參數結構，維持向後相容性
- 視頻和字幕檔案參數保持位置參數形式

#### 1.2 更新參數驗證邏輯

```rust
impl SyncArgs {
    /// 驗證參數組合的有效性
    pub fn validate(&self) -> Result<()> {
        match (self.offset.is_some(), self.video.is_some()) {
            (true, _) => {
                // 手動模式：視頻檔案可選
                Ok(())
            }
            (false, true) => {
                // 自動模式：必須有視頻檔案
                Ok(())
            }
            (false, false) => {
                // 自動模式但缺少視頻檔案
                Err(SubXError::InvalidArguments(
                    "Video file is required for automatic synchronization. \
                     Use --offset for manual synchronization without video file.".to_string()
                ))
            }
        }
    }
    
    /// 判斷是否需要視頻檔案
    pub fn requires_video(&self) -> bool {
        self.offset.is_none()
    }
    
    // 現有的 sync_method 保持不變
    pub fn sync_method(&self) -> SyncMethod {
        if self.offset.is_some() {
            SyncMethod::Manual
        } else {
            SyncMethod::Auto
        }
    }
}
```

### 階段 2: 命令執行邏輯優化

#### 2.1 重構 execute_sync_logic 函數

```rust
async fn execute_sync_logic(
    args: &SyncArgs,
    app_config: crate::config::Config,
    sync_engine: SyncEngine,
) -> Result<()> {
    // 參數驗證
    args.validate()?;
    
    match args.sync_method() {
        SyncMethod::Manual => {
            execute_manual_sync(args, &sync_engine).await
        }
        SyncMethod::Auto => {
            execute_automatic_sync(args, app_config, &sync_engine).await
        }
    }
}

/// 執行手動同步：完全跳過視頻檔案處理
async fn execute_manual_sync(
    args: &SyncArgs,
    sync_engine: &SyncEngine,
) -> Result<()> {
    let manual_offset = args.offset.expect("Manual sync requires offset");
    
    println!("🔧 執行手動時間軸調整...");
    println!("📝 字幕檔案: {}", args.subtitle.display());
    println!("⏱️  偏移量: {}s", manual_offset);
    
    let mut subtitle = load_subtitle(&args.subtitle).await?;
    sync_engine.apply_sync_offset(&mut subtitle, manual_offset as f32)?;
    save_subtitle(&subtitle, &args.subtitle).await?;
    
    println!("✅ 手動偏移套用完成: {}s", manual_offset);
    Ok(())
}

/// 執行自動同步：需要視頻檔案進行音訊分析
async fn execute_automatic_sync(
    args: &SyncArgs,
    app_config: crate::config::Config,
    sync_engine: &SyncEngine,
) -> Result<()> {
    let video_path = args.video.as_ref()
        .expect("Auto sync requires video file");
    
    println!("🎵 執行自動音訊分析同步...");
    println!("🎬 視頻檔案: {}", video_path.display());
    println!("📝 字幕檔案: {}", args.subtitle.display());
    
    // 對話檢測（僅在自動模式執行）
    if app_config.sync.enable_dialogue_detection {
        let detector = DialogueDetector::new(&app_config.sync);
        let segs = detector.detect_dialogue(video_path).await?;
        println!("🎤 檢測到 {} 個對話片段", segs.len());
        println!("🗣️  語音比例: {:.1}%", detector.get_speech_ratio(&segs) * 100.0);
    }
    
    if args.batch {
        execute_batch_sync(args, sync_engine).await
    } else {
        execute_single_sync(args, video_path, sync_engine).await
    }
}
```

#### 2.2 批量處理邏輯調整

```rust
async fn execute_batch_sync(
    args: &SyncArgs,
    sync_engine: &SyncEngine,
) -> Result<()> {
    let video_path = args.video.as_ref()
        .expect("Batch sync requires video directory");
        
    let media_pairs = discover_media_pairs(video_path).await?;
    
    println!("📁 批量處理模式: 找到 {} 個媒體檔案對", media_pairs.len());
    
    for (video_file, subtitle_file) in media_pairs {
        match sync_single_pair(sync_engine, &video_file, &subtitle_file).await {
            Ok(result) => {
                println!(
                    "✅ {} - 偏移: {:.2}s (信心度: {:.2})",
                    subtitle_file.display(),
                    result.offset_seconds,
                    result.confidence
                );
            }
            Err(e) => {
                println!("❌ {} - 錯誤: {}", subtitle_file.display(), e);
            }
        }
    }
    Ok(())
}
```

### 階段 3: 使用者體驗改善

#### 3.1 更新命令說明和範例

**更新 README 和文檔：**

```markdown
### Sync 命令使用方式

#### 自動同步（需要視頻檔案）
```bash
# 基本自動同步
subx-cli sync movie.mp4 movie.srt

# 自定義參數的自動同步
subx-cli sync movie.mp4 movie.srt --range 10.0 --threshold 0.8

# 批量自動同步
subx-cli sync /path/to/videos /path/to/subtitles --batch
```

#### 手動同步（僅需要字幕檔案）
```bash
# 基本手動同步
subx-cli sync --offset 2.5 movie.srt

# 負偏移（提前字幕）
subx-cli sync --offset -1.2 movie.srt

# 精確調整
subx-cli sync --offset 0.75 episode.srt
```

#### 向後相容性
```bash
# 舊的命令格式仍然支援
subx-cli sync movie.mp4 movie.srt --offset 2.5
```
```

#### 3.2 增強錯誤訊息

```rust
impl SyncArgs {
    pub fn validate(&self) -> Result<()> {
        match (self.offset.is_some(), self.video.is_some()) {
            (true, _) => Ok(()),
            (false, true) => Ok(()),
            (false, false) => {
                Err(SubXError::InvalidArguments(format!(
                    "視頻檔案在自動同步模式下是必填的。\n\n\
                    使用方式:\n\
                    • 自動同步: subx-cli sync video.mp4 subtitle.srt\n\
                    • 手動同步: subx-cli sync --offset 2.5 subtitle.srt\n\n\
                    需要幫助嗎？執行: subx-cli sync --help"
                )))
            }
        }
    }
}
```

### 階段 4: 測試實作

#### 4.1 單元測試

**檔案：** `tests/commands/sync_command_manual_offset_tests.rs`

```rust
use tempfile::TempDir;
use std::fs;
use subx_cli::cli::SyncArgs;
use subx_cli::commands::sync_command;
use subx_cli::config::test_service::TestConfigService;
use std::sync::Arc;
use std::path::PathBuf;

#[tokio::test]
async fn test_manual_sync_without_video_file() {
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");
    
    // 建立測試字幕檔案
    let srt_content = r#"1
00:00:01,000 --> 00:00:03,000
測試字幕 1

2
00:00:04,000 --> 00:00:06,000
測試字幕 2
"#;
    fs::write(&subtitle_path, srt_content).unwrap();
    
    let args = SyncArgs {
        video: None, // 手動模式不需要視頻檔案
        subtitle: subtitle_path.clone(),
        offset: Some(2.5),
        batch: false,
        range: None,
        threshold: None,
    };
    
    let config_service = Arc::new(TestConfigService::new());
    let result = sync_command::execute_with_config(args, config_service).await;
    
    assert!(result.is_ok(), "手動同步應該成功執行");
    
    // 驗證字幕時間軸已調整
    let updated_content = fs::read_to_string(&subtitle_path).unwrap();
    assert!(updated_content.contains("00:00:03,500")); // 1s + 2.5s offset
    assert!(updated_content.contains("00:00:06,500")); // 4s + 2.5s offset
}

#[tokio::test]
async fn test_auto_sync_requires_video_file() {
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");
    
    let args = SyncArgs {
        video: None, // 自動模式缺少視頻檔案
        subtitle: subtitle_path,
        offset: None,
        batch: false,
        range: None,
        threshold: None,
    };
    
    let result = args.validate();
    assert!(result.is_err(), "自動模式缺少視頻檔案應該產生錯誤");
}

#[tokio::test]
async fn test_backward_compatibility() {
    let temp = TempDir::new().unwrap();
    let video_path = temp.path().join("video.mp4");
    let subtitle_path = temp.path().join("test.srt");
    
    // 建立空的測試檔案
    fs::write(&video_path, b"").unwrap();
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest").unwrap();
    
    // 測試舊的參數格式
    let args = SyncArgs {
        video: Some(video_path),
        subtitle: subtitle_path,
        offset: Some(1.5),
        batch: false,
        range: None,
        threshold: None,
    };
    
    let result = args.validate();
    assert!(result.is_ok(), "向後相容性應該保持");
}
```

#### 4.2 整合測試

**檔案：** `tests/cli/sync_manual_offset_integration_tests.rs`

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_manual_sync_cli_interface() {
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");
    
    let srt_content = r#"1
00:00:01,000 --> 00:00:03,000
測試內容
"#;
    fs::write(&subtitle_path, srt_content).unwrap();
    
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("sync")
        .arg("--offset")
        .arg("2.0")
        .arg(&subtitle_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("手動偏移套用完成"));
}

#[test]
fn test_auto_sync_missing_video_error() {
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");
    fs::write(&subtitle_path, "test").unwrap();
    
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("sync")
        .arg(&subtitle_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("視頻檔案在自動同步模式下是必填的"));
}
```

## 實作順序

### 第 1 週：CLI 參數結構重新設計

1. **Day 1-2**: 修改 `SyncArgs` 結構，實作條件性必填參數
2. **Day 3-4**: 實作參數驗證邏輯和錯誤處理
3. **Day 5**: 更新相關的型別和介面

### 第 2 週：命令執行邏輯重構

1. **Day 1-2**: 重構 `execute_sync_logic` 函數，分離手動和自動同步邏輯
2. **Day 3-4**: 實作 `execute_manual_sync` 和 `execute_automatic_sync` 函數
3. **Day 5**: 調整批量處理邏輯

### 第 3 週：測試和驗證

1. **Day 1-2**: 編寫單元測試
2. **Day 3-4**: 編寫整合測試和 CLI 測試
3. **Day 5**: 執行完整測試套件，修復發現的問題

### 第 4 週：文檔和使用者體驗

1. **Day 1-2**: 更新 README 和命令說明
2. **Day 3-4**: 改善錯誤訊息和使用者回饋
3. **Day 5**: 最終測試和程式碼審查

## 品質保證

### 程式碼品質檢查

```bash
# 執行程式碼格式化
cargo fmt

# 執行靜態分析
cargo clippy -- -D warnings

# 執行品質檢查腳本
timeout 30 scripts/quality_check.sh

# 檢查測試覆蓋率
scripts/check_coverage.sh -T
```

### 效能考量

1. **記憶體使用**: 手動模式下完全避免載入視頻檔案，減少記憶體消耗
2. **執行速度**: 跳過音訊分析步驟，大幅提升手動同步的執行速度
3. **檔案 I/O**: 僅讀取和寫入字幕檔案，減少不必要的檔案操作

### 相容性考量

1. **向後相容**: 現有的命令格式仍然支援
2. **配置相容**: 現有的配置檔案設定保持不變
3. **API 相容**: 現有的函數介面盡量保持穩定

## 預期效益

### 使用者體驗改善

1. **簡化操作**: 手動同步時不再需要提供無用的視頻檔案
2. **提升效能**: 跳過音訊分析，手動同步執行速度大幅提升
3. **減少錯誤**: 避免因視頻檔案問題導致的手動同步失敗

### 技術債務減少

1. **邏輯清晰**: 手動和自動同步邏輯完全分離
2. **程式碼簡潔**: 移除不必要的條件判斷和處理邏輯
3. **維護性提升**: 各功能模組職責明確

## 風險評估

### 高風險項目

1. **CLI 參數變更**: 可能影響現有使用者和腳本
   - **緩解措施**: 保持向後相容性，提供詳細的遷移指南

2. **測試覆蓋不足**: 複雜的參數組合可能產生未預期的行為
   - **緩解措施**: 實作全面的測試套件，包含邊界情況

### 中風險項目

1. **效能回歸**: 重構可能引入新的效能問題
   - **緩解措施**: 建立效能基準測試，監控關鍵指標

2. **錯誤處理**: 新的錯誤情況可能沒有適當處理
   - **緩解措施**: 增強錯誤處理機制，提供清晰的錯誤訊息

## 成功標準

### 功能標準

- [ ] 手動同步模式下可以不提供視頻檔案
- [ ] 自動同步模式下視頻檔案仍為必填
- [ ] 向後相容性完全保持
- [ ] 錯誤訊息清晰且有幫助

### 品質標準

- [ ] 測試覆蓋率達到 90% 以上
- [ ] 所有 clippy 警告已解決
- [ ] 程式碼通過品質檢查腳本
- [ ] 效能測試顯示預期的改善

### 使用者體驗標準

- [ ] 命令執行時間在手動模式下減少 80% 以上
- [ ] 使用者回饋積極，操作流程更順暢
- [ ] 文檔清晰，範例完整且可執行

---

**預估完成時間**: 4 週  
**優先級**: 中等  
**複雜度**: 中等  
**影響範圍**: sync 命令、CLI 介面、測試系統
