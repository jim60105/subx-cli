---
title: "Job Report: Backlog #31 - 整合 Symphonia 音訊轉碼功能（階段1）"
date: "2025-06-13T23:02:55Z"
---

# Backlog #31 - 整合 Symphonia 音訊轉碼功能（階段1） 工作報告

**日期**：2025-06-13T23:02:55Z  
**任務**：建立 `AudioTranscoder` 結構體與基本介面，實作暫存檔案管理與檔案格式檢測，並建立錯誤處理框架。  
**類型**：Backlog  
**狀態**：已完成

> [!TIP]
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、任務概述

本階段聚焦於為 SubX 的音訊處理流程建立初步的轉碼服務骨架（Stage 1），包含：
- 定義 `AudioTranscoder` 結構體與初始化機制
- 設計並建立暫存目錄管理
- 根據檔案副檔名實作格式檢測
- 建立錯誤處理方法，以統一 `SubXError::audio_processing`

## 二、實作內容

### 2.1 建立 `AudioTranscoder` 結構體與初始化
- 定義包含 `temp_dir`、`probe`、`codecs` 欄位的 `AudioTranscoder`。
- `new()` 建立暫存目錄並初始化 Symphonia probe/codecs。
- 【F:src/services/audio/transcoder.rs†L9-L29】

```rust
pub struct AudioTranscoder {
    temp_dir: PathBuf,
    probe: &'static Probe,
    codecs: &'static CodecRegistry,
}

impl AudioTranscoder {
    pub fn new() -> Result<Self> {
        // 建立暫存目錄
        // 初始化 Symphonia probe & codecs
        #...
    }
}
```

### 2.2 實作格式檢測邏輯 (`needs_transcoding`)
- 依據副檔名判斷是否為 WAV；非 WAV 回傳 `true`，WAV 回傳 `false`；缺少副檔名則回傳錯誤。
- 【F:src/services/audio/transcoder.rs†L32-L41】

```rust
pub fn needs_transcoding<P: AsRef<Path>>(&self, audio_path: P) -> Result<bool> {
    if let Some(ext) = audio_path.as_ref().extension().and_then(|s| s.to_str()) {
        let ext_lc = ext.to_lowercase();
        Ok(ext_lc != "wav")
    } else {
        Err(SubXError::audio_processing("Missing file extension".to_string()))
    }
}
```

### 2.3 定義 `transcode_to_wav` 與 `cleanup` Stub
- 加入 `transcode_to_wav` 非同步方法 Stub，回傳尚未實作錯誤。
- 加入 `cleanup` 移除暫存目錄實作。
- 【F:src/services/audio/transcoder.rs†L44-L59】

```rust
pub async fn transcode_to_wav<P: AsRef<Path>>(&self, _input_path: P) -> Result<PathBuf> {
    Err(SubXError::audio_processing("transcode_to_wav not implemented".to_string()))
}

pub fn cleanup(&self) -> Result<()> {
    // 清理暫存目錄
    #...
}
```

### 2.4 模組引入與依賴更新
- 在 `src/services/audio/mod.rs` 新增 `transcoder` 模組與 re-export `AudioTranscoder`。
- 更新 `Cargo.toml` 啟用 Symphonia 的 all-codecs、all-formats features。
- 【F:src/services/audio/mod.rs†L144-L145】【F:Cargo.toml†L87-L88】

```diff
 pub mod dialogue_detector;
 pub use dialogue_detector::AusDialogueDetector;

 pub mod transcoder;
 pub use transcoder::AudioTranscoder;

 [dependencies]
- symphonia = { version = "0.5", features = ["all"] }
+ symphonia = { version = "0.5", features = ["all-codecs", "all-formats"] }
```

## 三、技術細節

本階段僅完成服務骨架及格式檢測，後續 Stage 2 將完成核心解碼與 WAV 編碼實作。

## 四、測試與驗證

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
timeout 30 scripts/quality_check.sh -v
```

## 五、後續事項

### 待完成項目
- Stage 2：實作 Symphonia 解碼與 WAV 編碼

### 相關任務
- Backlog #31 全階段規劃

### 建議的下一步
- 優先實作 MP4 → WAV 轉碼流程

## 八、檔案異動清單

| 檔案路徑                           | 異動類型 | 描述                         |
|------------------------------------|----------|------------------------------|
| `src/services/audio/transcoder.rs`| 新增     | 音訊轉碼服務骨架與格式檢測介面 |
| `src/services/audio/mod.rs`       | 修改     | 新增 transcoder 模組與 re-export |
| `Cargo.toml`                       | 修改     | 更新 symphonia features      |
