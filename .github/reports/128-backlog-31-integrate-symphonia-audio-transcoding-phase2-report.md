---
title: "Job Report: Backlog #31 - 整合 Symphonia 音訊轉碼功能（階段2）"
date: "2025-06-13T23:11:36Z"
---

# Backlog #31 - 整合 Symphonia 音訊轉碼功能（階段2） 工作報告

**日期**：2025-06-13T23:11:36Z  
**任務**：實作 Symphonia 核心轉碼功能，包括解碼管道、WAV 編碼及立體/單聲道支援。  
**類型**：Backlog  
**狀態**：已完成

## 一、任務概述

本階段 (階段2) 聚焦於實現音訊檔案的核心轉碼功能，涵蓋：
- 利用 Symphonia 偵測格式及初始化解碼器
- 解碼音訊封包並緩衝樣本
- 使用 hound 寫入 WAV 檔
- 支援單聲道與立體聲聲道設定

## 二、實作內容

### 2.1 引入必要依賴與型別
- 新增對 `tempfile::TempDir`、`hound` 及 `symphonia::core::audio::Layout` 的引入，簡化暫存目錄管理並支援 WAV 寫入。
- 【F:src/services/audio/transcoder.rs†L4-L15】【F:Cargo.toml†L90-L91】

```rust
use hound::{SampleFormat, WavSpec, WavWriter};
use std::fs::File;
use tempfile::TempDir;
use symphonia::core::{
    audio::{Layout, SampleBuffer},
    codecs::CODEC_TYPE_NULL,
    errors::Error as SymphoniaError,
    io::MediaSourceStream,
};
```

### 2.2 實作 Symphonia 初始化與格式偵測
- 依據新的 API 呼叫 `probe.format`，增加 metadata 參數以符合 Signataure。
- 【F:src/services/audio/transcoder.rs†L64-L72】

```rust
let probed = self
    .probe
    .format(
        &Default::default(),
        mss,
        &Default::default(),
        &Default::default(),
    )
    .map_err(|e| SubXError::audio_processing(format!("Format probe error: {}", e)))?;
```

### 2.3 實作 WAV 編碼器與解碼管道
- 建立 `WavWriter`，將逐包解碼後的樣本透過 `SampleBuffer::<i16>` 轉為 PCM 寫入。
- 【F:src/services/audio/transcoder.rs†L85-L117】

```rust
let mut writer = WavWriter::create(&wav_path, spec)?;
loop {
    match format.next_packet() {
        Ok(packet) => {
            let audio_buf = decoder.decode(&packet)?;
            let mut sample_buf = SampleBuffer::<i16>::new(
                audio_buf.capacity() as u64,
                *audio_buf.spec(),
            );
            sample_buf.copy_interleaved_ref(audio_buf);
            for sample in sample_buf.samples() {
                writer.write_sample(*sample)?;
            }
        }
        Err(SymphoniaError::IoError(err))
            if err.kind() == std::io::ErrorKind::UnexpectedEof =>
        {
            break;
        }
        Err(e) => return Err(SubXError::audio_processing(format!("Packet read error: {}", e))),
    }
}
writer.finalize()?;
```

### 2.4 支援單聲道與立體聲
- 使用 `track.codec_params.channel_layout` 決定聲道數量，若未知則回退為 `Layout::Stereo`。
- 【F:src/services/audio/transcoder.rs†L86-L90】

```rust
let layout = track.codec_params.channel_layout.unwrap_or(Layout::Stereo);
let channels = layout.into_channels().count() as u16;
```

## 三、技術細節

### 3.1 API 變更
- `AudioTranscoder::new` 改用 `TempDir`；`cleanup` 由 `fs::remove_dir_all` 轉為 `TempDir::close()`。
【F:src/services/audio/transcoder.rs†L25-L37】【F:src/services/audio/transcoder.rs†L138-L144】

### 3.2 相依性變更
- `Cargo.toml` 新增 `tempfile`、`hound` 依賴。
【F:Cargo.toml†L90-L91】

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check
```

## 五、影響評估

### 5.1 向後相容性
- 現有 `needs_transcoding`、`cleanup` API 簡單調整，並未移除方法，向後相容。

## 六、後續事項

### 6.1 待完成項目
- 實作格式轉碼後的 `load_audio_file_with_transcoding` 整合（階段3）。

### 6.2 建議的下一步
- 實作 MP4/MKV/OGG 等多種容器支援與更多編碼器。
