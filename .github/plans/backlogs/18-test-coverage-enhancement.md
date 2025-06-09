# Product Backlog #18: 測試覆蓋率提升計畫

## 領域範圍
針對性測試覆蓋率提升、低覆蓋率模組補強、測試策略優化、覆蓋率監控系統

## 完成項目

### 1. CLI 層級測試補強 (當前覆蓋率: 0%)
- [ ] **main.rs 測試 (目標: 40%)**
  - [ ] 命令列參數解析測試
  - [ ] 基本執行流程測試
  - [ ] 錯誤退出碼驗證
  - [ ] 版本資訊顯示測試
  
- [ ] **CLI 參數模組測試 (目標: 60%)**
  - [ ] cache_args.rs 參數驗證測試
  - [ ] config_args.rs 參數驗證測試
  - [ ] convert_args.rs 參數驗證測試
  - [ ] detect_encoding_args.rs 參數驗證測試
  - [ ] match_args.rs 參數驗證測試
  - [ ] sync_args.rs 參數驗證測試
  
- [ ] **CLI UI 模組測試 (目標: 50%)**
  - [ ] 表格輸出格式測試
  - [ ] 進度條顯示測試
  - [ ] 錯誤訊息格式測試
  - [ ] 使用者介面整合測試

### 2. 指令層級測試補強 (當前覆蓋率: 5-30%)
- [ ] **cache_command.rs 測試提升 (目標: 70%)**
  - [ ] 快取清理功能測試
  - [ ] 快取統計顯示測試
  - [ ] 快取檔案管理測試
  
- [ ] **config_command.rs 測試提升 (目標: 70%)**
  - [ ] 配置顯示功能測試
  - [ ] 配置設定功能測試
  - [ ] 配置驗證功能測試
  
- [ ] **convert_command.rs 測試提升 (目標: 70%)**
  - [ ] 格式轉換流程測試
  - [ ] 批量轉換測試
  - [ ] 轉換錯誤處理測試
  
- [ ] **detect_encoding_command.rs 測試提升 (目標: 70%)**
  - [ ] 編碼檢測功能測試
  - [ ] 多檔案編碼檢測測試
  - [ ] 編碼轉換建議測試
  
- [ ] **sync_command.rs 測試提升 (目標: 70%)**
  - [ ] 音訊同步流程測試
  - [ ] 同步參數驗證測試
  - [ ] 同步結果輸出測試

### 3. 服務層測試補強 (當前覆蓋率: 0-40%)
- [ ] **AI 快取服務測試 (目標: 80%)**
  - [ ] 快取鍵生成測試
  - [ ] 快取命中/未命中測試
  - [ ] 快取過期機制測試
  - [ ] 快取大小限制測試
  
- [ ] **音訊服務測試 (目標: 60%)**
  - [ ] 音訊檔案載入測試
  - [ ] 格式支援驗證測試
  - [ ] 錯誤處理測試
  - [ ] 資源釋放測試

### 4. 核心模組覆蓋率提升
- [ ] **worker.rs 提升 (32.28% → 70%)**
  - [ ] 工作分配邏輯測試
  - [ ] 錯誤恢復機制測試
  - [ ] 並行處理測試
  
- [ ] **validator.rs 提升 (18.25% → 70%)**
  - [ ] 檔案驗證邏輯測試
  - [ ] 格式驗證測試
  - [ ] 驗證錯誤報告測試
  
- [ ] **analyzer.rs 提升 (6.93% → 70%)**
  - [ ] 內容分析算法測試
  - [ ] 統計資料生成測試
  - [ ] 分析結果輸出測試

### 5. 整合測試擴充
- [ ] **端到端工作流程測試**
  - [ ] 完整字幕匹配流程測試
  - [ ] 配置載入到執行測試
  - [ ] 錯誤處理鏈測試
  
- [ ] **跨模組互動測試**
  - [ ] 配置與服務整合測試
  - [ ] CLI 與核心邏輯整合測試
  - [ ] 格式處理鏈整合測試

### 6. 測試基礎設施優化
- [ ] **測試輔助工具擴充**
  - [ ] CLI 測試輔助函式
  - [ ] 模擬檔案系統工具
  - [ ] 測試資料產生器增強
  
- [ ] **模擬服務優化**
  - [ ] AI 服務模擬增強
  - [ ] 檔案系統操作模擬
  - [ ] 網路請求模擬

## 技術設計

### 測試策略分析

**當前覆蓋率狀況：**
```
整體覆蓋率: 62.25% (目標: ≥50% ✓)
核心模組平均: 45.2% (目標: ≥70% ✗)

需要重點提升的模組:
- CLI 層級: 0-10% → 50-60%
- 指令層級: 5-30% → 70%
- 服務層級: 0-40% → 60-80%
- 核心模組: 6-32% → 70%
```

**優先級策略：**
1. **高影響低成本：** CLI 參數驗證測試
2. **核心功能：** 指令執行邏輯測試  
3. **服務穩定性：** AI 和音訊服務測試
4. **整合驗證：** 端到端流程測試

### CLI 測試實作範例

**main.rs 測試基礎設施：**
```rust
// tests/cli_integration_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_version_display() {
    let mut cmd = Command::cargo_bin("subx").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("subx"));
}

#[test]
fn test_help_display() {
    let mut cmd = Command::cargo_bin("subx").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("智慧字幕工具"));
}

#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("subx").unwrap();
    cmd.arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("錯誤"));
}

#[test]
fn test_config_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("subx").unwrap();
    cmd.arg("config")
        .arg("--config-dir")
        .arg(temp_dir.path())
        .arg("show")
        .assert()
        .success();
}
```

**CLI 參數測試範例：**
```rust
// src/cli/match_args.rs (測試模組)
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_match_args_default_values() {
        let args = MatchArgs {
            video_path: "test.mp4".into(),
            subtitle_path: None,
            output_path: None,
            dry_run: false,
            force: false,
            threshold: None,
        };
        
        assert_eq!(args.video_path, "test.mp4");
        assert!(!args.dry_run);
        assert!(!args.force);
        assert!(args.threshold.is_none());
    }

    #[test]
    fn test_match_args_parsing() {
        let args = MatchArgs::try_parse_from(&[
            "subx",
            "match",
            "video.mp4",
            "--subtitle-path", "sub.srt",
            "--dry-run",
            "--threshold", "0.8"
        ]).unwrap();
        
        assert_eq!(args.video_path, "video.mp4");
        assert_eq!(args.subtitle_path, Some("sub.srt".into()));
        assert!(args.dry_run);
        assert_eq!(args.threshold, Some(0.8));
    }

    #[test]
    fn test_match_args_invalid_threshold() {
        let result = MatchArgs::try_parse_from(&[
            "subx", "match", "video.mp4", 
            "--threshold", "1.5"
        ]);
        assert!(result.is_err());
    }
}
```

### 指令測試實作範例

**convert_command.rs 測試增強：**
```rust
// src/commands/convert_command.rs (測試模組擴充)
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_convert_srt_to_vtt() {
        let temp_dir = TempDir::new().unwrap();
        let input_file = temp_dir.path().join("test.srt");
        let output_file = temp_dir.path().join("test.vtt");
        
        fs::write(&input_file, "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n\n").unwrap();
        
        let args = ConvertArgs {
            input_path: input_file.clone(),
            output_path: Some(output_file.clone()),
            target_format: Some("vtt".to_string()),
            preserve_style: false,
        };
        
        let result = execute_convert_command(args);
        assert!(result.is_ok());
        assert!(output_file.exists());
        
        let content = fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("WEBVTT"));
        assert!(content.contains("00:00:01.000 --> 00:00:02.000"));
    }

    #[test]
    fn test_convert_batch_processing() {
        let temp_dir = TempDir::new().unwrap();
        
        // 建立多個測試檔案
        for i in 1..=3 {
            let file = temp_dir.path().join(format!("test{}.srt", i));
            fs::write(&file, format!("1\n00:00:0{},000 --> 00:00:0{},000\nTest {}\n\n", i, i+1, i)).unwrap();
        }
        
        let args = ConvertArgs {
            input_path: temp_dir.path().to_path_buf(),
            output_path: Some(temp_dir.path().join("output")),
            target_format: Some("vtt".to_string()),
            preserve_style: false,
        };
        
        let result = execute_convert_command(args);
        assert!(result.is_ok());
        
        // 驗證輸出檔案
        for i in 1..=3 {
            let output_file = temp_dir.path().join("output").join(format!("test{}.vtt", i));
            assert!(output_file.exists());
        }
    }

    #[test]
    fn test_convert_unsupported_format() {
        let temp_dir = TempDir::new().unwrap();
        let input_file = temp_dir.path().join("test.txt");
        fs::write(&input_file, "not a subtitle").unwrap();
        
        let args = ConvertArgs {
            input_path: input_file,
            output_path: None,
            target_format: Some("srt".to_string()),
            preserve_style: false,
        };
        
        let result = execute_convert_command(args);
        assert!(result.is_err());
    }
}
```

### 服務層測試實作範例

**AI 快取服務測試：**
```rust
// src/services/ai/cache.rs (測試模組)
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_cache_hit_miss() {
        let temp_dir = TempDir::new().unwrap();
        let cache = AiCache::new(temp_dir.path()).await.unwrap();
        
        let key = "test_prompt";
        let value = "test_response";
        
        // 快取未命中
        assert!(cache.get(key).await.unwrap().is_none());
        
        // 設定快取
        cache.set(key, value).await.unwrap();
        
        // 快取命中
        let cached = cache.get(key).await.unwrap();
        assert_eq!(cached, Some(value.to_string()));
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = AiCache::new(temp_dir.path()).await.unwrap();
        cache.set_ttl(Duration::from_millis(100));
        
        let key = "expire_test";
        let value = "expire_value";
        
        cache.set(key, value).await.unwrap();
        assert!(cache.get(key).await.unwrap().is_some());
        
        sleep(Duration::from_millis(150)).await;
        assert!(cache.get(key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cache_size_limit() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = AiCache::new(temp_dir.path()).await.unwrap();
        cache.set_max_entries(2);
        
        cache.set("key1", "value1").await.unwrap();
        cache.set("key2", "value2").await.unwrap();
        cache.set("key3", "value3").await.unwrap();
        
        // 最舊的項目應該被移除
        assert!(cache.get("key1").await.unwrap().is_none());
        assert!(cache.get("key2").await.unwrap().is_some());
        assert!(cache.get("key3").await.unwrap().is_some());
    }
}
```

## 實作階段

### 第一階段：CLI 層級補強 (2 天)
**目標：** 將 CLI 覆蓋率從 0% 提升到 50%

**任務優先序：**
1. **第一天**
   - [ ] main.rs 基本測試 (參數解析、版本顯示)
   - [ ] 最重要的 CLI 參數模組測試 (match_args, convert_args)
   - [ ] CLI 整合測試基礎設施

2. **第二天**  
   - [ ] 剩餘 CLI 參數模組測試
   - [ ] UI 模組測試 (表格、進度條)
   - [ ] 錯誤處理測試

### 第二階段：指令層級提升 (2 天)
**目標：** 將指令覆蓋率從 5-30% 提升到 70%

**任務優先序：**
1. **第三天**
   - [ ] convert_command.rs 完整測試
   - [ ] match_command.rs 測試擴充
   - [ ] 指令錯誤處理測試

2. **第四天**
   - [ ] config_command.rs 測試
   - [ ] cache_command.rs 測試  
   - [ ] detect_encoding_command.rs 測試
   - [ ] sync_command.rs 測試

### 第三階段：服務與核心模組 (2 天)
**目標：** 核心模組達到 70% 覆蓋率

**任務優先序：**
1. **第五天**
   - [ ] AI 快取服務完整測試
   - [ ] worker.rs 並行處理測試
   - [ ] validator.rs 驗證邏輯測試

2. **第六天**
   - [ ] analyzer.rs 分析算法測試
   - [ ] 音訊服務基礎測試
   - [ ] 整合測試擴充
   - [ ] 文件更新(如果需要)

## 測試覆蓋率目標追蹤

### 當前狀況 vs 目標
```
模組類別                    當前     目標     差距
======================================================
整體覆蓋率                  62.25%   ≥50%     ✓ 已達標
核心模組平均                45.2%    ≥70%     24.8%

詳細模組:
CLI 層級 (main.rs)          0%       40%      40%
CLI 參數模組                0%       60%      60%  
指令模組平均                15%      70%      55%
AI 快取服務                 0%       80%      80%
音訊服務                    0%       60%      60%
worker.rs                   32.28%   70%      37.72%
validator.rs               18.25%   70%      51.75%
analyzer.rs                6.93%    70%      63.07%
```

### 覆蓋率提升預期效果
**實作完成後預期覆蓋率：**
- 整體覆蓋率：62.25% → **75%** ✓
- 核心模組平均：45.2% → **72%** ✓
- CLI 層級：0% → **50%** ✓
- 指令層級：15% → **70%** ✓
- 服務層級：10% → **70%** ✓

## 品質保證措施

### 1. 測試品質檢查
- [ ] **測試案例覆蓋正常和異常路徑**
- [ ] **模擬物件使用適當且不過度**
- [ ] **測試隔離性確保無相互依賴**
- [ ] **測試執行時間合理 (<30秒總執行時間)**

### 2. 程式碼審查要點
- [ ] **測試邏輯清晰易懂**
- [ ] **測試資料管理妥當**
- [ ] **錯誤訊息測試完整**
- [ ] **邊界條件測試充分**

### 3. 持續整合檢查
- [ ] **覆蓋率自動報告**
- [ ] **覆蓋率回歸檢測**
- [ ] **測試執行時間監控**
- [ ] **測試穩定性追蹤**

## 相依性
- 依賴 Backlog #12 (測試基礎設施)
- 依賴所有功能模組已完成實作

## 風險評估
- **中風險：** 某些模組可能需要重構以支援測試
- **注意事項：**
  - CLI 測試可能需要模擬複雜的使用者互動
  - 並行處理測試的時序問題
  - 外部服務依賴的模擬複雜度
  - 測試資料管理和清理

## 成功指標
1. **整體測試覆蓋率達到 75%** (超越 50% 目標)
2. **核心模組覆蓋率達到 72%** (超越 70% 目標)  
3. **所有新增測試穩定通過且執行快速**

## 後續維護
- 每次新功能開發必須包含對應測試
- 定期檢查覆蓋率回歸情況
- 持續優化測試執行效率
- 根據實際使用情況調整測試策略
