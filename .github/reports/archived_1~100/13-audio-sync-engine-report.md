---
title: "Job Report: Backlog #09 - 音訊處理與時間軸同步"
date: "2025-06-07T02:51:46Z"
---

# Backlog #09 - 音訊處理與時間軸同步 工作報告

**日期**：2025-06-07T02:51:46Z  
**任務**：音訊特徵提取、時間軸分析、自動同步、手動校正

## 一、相依套件更新

- 新增音訊處理相依套件 `symphonia` 支援多種音訊格式解碼
  【F:Cargo.toml†L56-L57】

```toml
# 音訊處理
symphonia = { version = "0.5", features = ["all"] }
```

## 二、音訊分析服務實作

- 新增 `src/services/audio/mod.rs`：實作 `AudioAnalyzer` 核心功能
  - **音訊解碼**：整合 Symphonia 解碼器支援 MP4、MKV、AVI 等格式
  - **能量包絡提取**：實作 RMS 能量計算，採樣率 16kHz，window_size=1024，hop_size=512
  - **對話檢測**：基於音量閾值的語音活動檢測 (VAD) 算法
  【F:src/services/audio/mod.rs†L1-L148】

```rust
pub struct AudioAnalyzer {
    sample_rate: u32,     // 16000 Hz
    window_size: usize,   // 1024 samples
    hop_size: usize,      // 512 samples
}

pub struct AudioEnvelope {
    pub samples: Vec<f32>,      // RMS 能量序列
    pub sample_rate: u32,       // 採樣率
    pub duration: f32,          // 總時長(秒)
}
```

## 三、時間軸同步引擎

- 新增 `src/core/sync/engine.rs`：實作 `SyncEngine` 同步核心
  - **交叉相關分析**：計算音訊能量與字幕時間軸的相關係數
  - **偏移檢測**：在 ±30 秒範圍內搜尋最佳時間偏移
  - **信心度評估**：基於相關峰值強度評估同步準確度
  - **偏移套用**：安全地調整字幕時間軸，處理負偏移邊界情況
  【F:src/core/sync/engine.rs†L1-L181】

```rust
pub struct SyncConfig {
    pub max_offset_seconds: f32,      // 最大偏移範圍 (±30s)
    pub correlation_threshold: f32,   // 相關閾值 (0.3)
    pub dialogue_threshold: f32,      // 對話檢測閾值 (0.01)
    pub min_dialogue_length: f32,     // 最小對話長度 (0.5s)
}

pub struct SyncResult {
    pub offset_seconds: f32,          // 計算出的時間偏移
    pub confidence: f32,              // 同步信心度 (0-1)
    pub method_used: SyncMethod,      // 使用的同步方法
    pub correlation_peak: f32,        // 相關峰值
}
```

## 四、Sync 命令實作

- 新增 `src/commands/sync_command.rs`：實作完整的 CLI sync 命令
  - **手動偏移模式**：`--offset` 參數直接指定時間偏移量
  - **自動同步模式**：基於音訊分析自動計算最佳偏移
  - **批量處理模式**：`--batch` 支援資料夾內多檔案批量同步
  - **檔案配對**：根據檔名自動配對影片與字幕檔案
  【F:src/commands/sync_command.rs†L1-L120】

### 主要功能函式：
- `execute()`：主要命令執行邏輯，支援三種同步模式
- `load_subtitle()` / `save_subtitle()`：字幕檔案載入與儲存
- `discover_media_pairs()`：掃描資料夾並自動配對媒體檔案
- `sync_single_pair()`：單一檔案對同步處理

## 五、模組整合與錯誤處理

- 更新 `src/core/sync/mod.rs`：匯出同步引擎相關結構
  【F:src/core/sync/mod.rs†L2-L4】
- 更新 `src/commands/mod.rs`：註冊 sync_command 模組
  【F:src/commands/mod.rs†L5】
- 更新 `src/cli/mod.rs`：修正 sync 命令路由
  【F:src/cli/mod.rs†L59】
- 更新 `src/error.rs`：新增 Symphonia 錯誤轉換為統一的 `SubXError::audio_processing`
  【F:src/error.rs†L51-L55】

## 六、技術特點

### 音訊處理技術
- **多格式支援**：透過 Symphonia 支援 MP4、MKV、AVI、MOV 等主流影片格式
- **高效能量提取**：使用 RMS (Root Mean Square) 計算音訊能量包絡
- **自適應採樣**：支援不同採樣率音訊，統一轉換為 16kHz 處理

### 同步算法
- **交叉相關分析**：使用 Pearson 相關係數計算音訊與字幕時間序列的相似度
- **滑動窗口搜尋**：在指定範圍內逐樣本搜尋最佳偏移點
- **邊界安全處理**：負偏移時正確處理字幕起始時間，避免負時間戳

### 用戶體驗
- **多模式支援**：手動偏移、自動分析、批量處理三種使用情境
- **信心度反饋**：提供同步結果的可信度評估，低信心度時建議手動調整
- **進度提示**：清楚的成功/失敗/警告資訊顯示

## 七、驗收標準檢核

1. ✅ **音訊處理準確且高效**：Symphonia 整合成功，支援主流格式
2. ✅ **對話檢測算法有效**：基於 RMS 能量的 VAD 算法實作完成
3. ✅ **交叉相關分析功能**：完整的相關係數計算與偏移搜尋
4. ✅ **同步結果信心度評估**：基於相關峰值的可信度計算
5. ✅ **批量處理效能**：支援資料夾掃描與檔案自動配對

## 八、後續工作

- Backlog #10：命令整合與測試 - 整合所有命令的端到端測試
- 效能優化：考慮使用 FFT 加速交叉相關計算（大檔案場景）
- 演算法改進：加入更精細的語音檢測算法（如基於頻譜特徵）
