# Bug 18: 修復音訊解碼器錯誤處理邏輯

## 問題描述

### 背景
在 `src/services/audio/transcoder.rs` 的 `transcode_to_wav` 方法中（第 153-155 行），當前的錯誤處理實現不符合 Symphonia 解碼器的 API 規範，導致可恢復的解碼錯誤被錯誤地視為致命錯誤，中斷整個轉碼過程。

### 問題程式碼
```rust
let audio_buf = decoder
    .decode(&packet)
    .map_err(|e| SubXError::audio_processing(format!("Decode error: {}", e)))?;
```

### 根本原因
根據 Symphonia API 文檔，`decoder.decode()` 方法的錯誤處理應該遵循以下規則：

1. **可恢復錯誤**：
   - `DecodeError`：解碼錯誤，應丟棄當前封包並繼續處理下一個封包
   - `IoError`：I/O 錯誤，應丟棄當前封包並繼續處理下一個封包

2. **需要重置的錯誤**：
   - `ResetRequired`：需要重置，消費者應期待解碼音頻緩衝區的 duration 和 SignalSpec 會發生變化，但仍可繼續解碼

3. **不可恢復錯誤**：
   - 其他所有錯誤都是不可恢復的，應該中斷轉碼過程

### 當前實現的問題
- 所有解碼錯誤都使用 `?` 運算符直接返回，中斷整個轉碼過程
- 沒有區分可恢復和不可恢復的錯誤類型
- 沒有處理 `ResetRequired` 情況
- 對於可恢復的錯誤，應該跳過當前封包繼續處理，而不是終止

## 影響範圍

### 使用者體驗影響
- 部分損壞或有問題的音訊檔案無法成功轉碼，即使大部分內容是可用的
- 轉碼過程在遇到單個損壞封包時就完全失敗
- 錯誤訊息不夠精確，無法指導使用者如何處理問題

### 系統穩定性影響
- 音訊轉碼功能的可靠性降低
- 可能導致 sync 命令在處理某些音訊檔案時失敗
- 影響整體的音訊處理管線穩定性

### 業務邏輯影響
- 違反了 Symphonia API 的最佳實踐
- 降低了對不完美音訊檔案的容錯能力
- 可能影響自動化處理大量音訊檔案的場景

## 解決方案

### 設計原則
1. **容錯性優先**：盡可能恢復並繼續處理，而不是立即失敗
2. **精確錯誤分類**：根據 Symphonia API 規範正確處理不同類型的錯誤
3. **詳細錯誤記錄**：提供足夠的錯誤資訊供診斷
4. **向後相容性**：確保修改不會破壞現有功能

### 實作策略

#### 1. 錯誤類型處理
根據 Symphonia 錯誤類型實現不同的處理邏輯：

```rust
use symphonia::core::errors::Error as SymphoniaError;
use log::warn;

// 在解碼循環中
match decoder.decode(&packet) {
    Ok(audio_buf) => {
        // 正常處理音訊緩衝區
        let mut sample_buf = SampleBuffer::<i16>::new(
            audio_buf.capacity() as u64, 
            *audio_buf.spec()
        );
        sample_buf.copy_interleaved_ref(audio_buf);
        
        for sample in sample_buf.samples() {
            writer.write_sample(*sample)
                .map_err(|e| SubXError::audio_processing(
                    format!("Write sample error: {}", e)
                ))?;
        }
    },
    Err(SymphoniaError::DecodeError(decode_err)) => {
        // 可恢復的解碼錯誤 - 記錄並跳過當前封包
        warn!("Decode error (recoverable), skipping packet: {}", decode_err);
        continue;
    },
    Err(SymphoniaError::IoError(io_err)) => {
        // 可恢復的 I/O 錯誤 - 記錄並跳過當前封包
        warn!("I/O error (recoverable), skipping packet: {}", io_err);
        continue;
    },
    Err(SymphoniaError::ResetRequired) => {
        // 需要重置 - 記錄警告但繼續處理
        warn!("Decoder reset required, audio specs may change");
        continue;
    },
    Err(other_error) => {
        // 不可恢復的錯誤 - 中斷轉碼
        return Err(SubXError::audio_processing(
            format!("Unrecoverable decode error: {}", other_error)
        ));
    }
}
```

#### 2. 統計和報告機制
實現錯誤統計機制，讓使用者了解轉碼過程中的問題：

```rust
struct TranscodeStats {
    total_packets: u64,
    decoded_packets: u64,
    skipped_decode_errors: u64,
    skipped_io_errors: u64,
    reset_required_count: u64,
}

impl TranscodeStats {
    fn new() -> Self {
        Self {
            total_packets: 0,
            decoded_packets: 0,
            skipped_decode_errors: 0,
            skipped_io_errors: 0,
            reset_required_count: 0,
        }
    }
    
    fn success_rate(&self) -> f64 {
        if self.total_packets == 0 {
            0.0
        } else {
            self.decoded_packets as f64 / self.total_packets as f64
        }
    }
}
```

#### 3. 配置選項
添加配置選項控制錯誤處理行為：

```rust
// 移除 TranscodeConfig 結構體，直接使用最小成功率參數
// min_success_rate: 最小成功解碼率，低於此比率將中斷轉碼
// 預設值為 0.5，表示至少 50% 的封包需要成功解碼
```

#### 4. 完整修改的方法簽名
```rust
impl AudioTranscoder {
    pub async fn transcode_to_wav_with_config<P: AsRef<Path>>(
        &self, 
        input_path: P, 
        min_success_rate: Option<f64>
    ) -> Result<(PathBuf, TranscodeStats)> {
        use symphonia::core::errors::Error as SymphoniaError;
        use log::warn;
        
        let input = input_path.as_ref();
        let min_success_rate = min_success_rate.unwrap_or(0.5); // 預設最小成功率 50%
        let mut stats = TranscodeStats::new();
        
        // 開啟原始音訊檔案
        let file = File::open(input).map_err(|e| {
            SubXError::audio_processing(format!(
                "Failed to open input file {}: {}",
                input.display(),
                e
            ))
        })?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        
        // 偵測格式並建立 FormatReader
        let probed = self
            .probe
            .format(
                &Default::default(),
                mss,
                &Default::default(),
                &Default::default(),
            )
            .map_err(|e| SubXError::audio_processing(format!("Format probe error: {}", e)))?;
        let mut format = probed.format;
        
        // 選擇第一個有效音軌
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| SubXError::audio_processing("No audio track found".to_string()))?;
        
        // 建立解碼器
        let mut decoder = self
            .codecs
            .make(&track.codec_params, &Default::default())
            .map_err(|e| SubXError::audio_processing(format!("Decoder error: {}", e)))?;
        
        // 設定 WAV 寫入規格
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let layout = track.codec_params.channel_layout.unwrap_or(Layout::Stereo);
        let channels = layout.into_channels().count() as u16;
        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        
        let wav_path = self
            .temp_dir
            .path()
            .join(input.file_stem().unwrap_or_default())
            .with_extension("wav");
        let mut writer = WavWriter::create(&wav_path, spec)
            .map_err(|e| SubXError::audio_processing(format!("WAV writer error: {}", e)))?;
        
        // 解碼並寫入 WAV
        loop {
            stats.total_packets += 1;
            
            match format.next_packet() {
                Ok(packet) => {
                    // 嘗試解碼封包
                    match decoder.decode(&packet) {
                        Ok(audio_buf) => {
                            // 成功解碼
                            stats.decoded_packets += 1;
                            
                            // 處理音訊緩衝區
                            let mut sample_buf = SampleBuffer::<i16>::new(
                                audio_buf.capacity() as u64, 
                                *audio_buf.spec()
                            );
                            sample_buf.copy_interleaved_ref(audio_buf);
                            
                            for sample in sample_buf.samples() {
                                writer.write_sample(*sample).map_err(|e| {
                                    SubXError::audio_processing(format!("Write sample error: {}", e))
                                })?;
                            }
                        },
                        Err(SymphoniaError::DecodeError(decode_err)) => {
                            // 可恢復的解碼錯誤 - 記錄並跳過當前封包
                            warn!("Decode error (recoverable), skipping packet: {}", decode_err);
                            stats.skipped_decode_errors += 1;
                            continue;
                        },
                        Err(SymphoniaError::IoError(io_err)) => {
                            // 可恢復的 I/O 錯誤 - 記錄並跳過當前封包
                            warn!("I/O error (recoverable), skipping packet: {}", io_err);
                            stats.skipped_io_errors += 1;
                            continue;
                        },
                        Err(SymphoniaError::ResetRequired) => {
                            // 需要重置 - 記錄警告但繼續處理
                            warn!("Decoder reset required, audio specs may change");
                            stats.reset_required_count += 1;
                            continue;
                        },
                        Err(other_error) => {
                            // 不可恢復的錯誤 - 中斷轉碼
                            return Err(SubXError::audio_processing(
                                format!("Unrecoverable decode error: {}", other_error)
                            ));
                        }
                    }
                }
                Err(SymphoniaError::IoError(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    // 正常結束
                    break;
                }
                Err(e) => {
                    return Err(SubXError::audio_processing(format!(
                        "Packet read error: {}",
                        e
                    )));
                }
            }
        }
        
        writer
            .finalize()
            .map_err(|e| SubXError::audio_processing(format!("Finalize WAV error: {}", e)))?;
        
        // 最終檢查成功率
        if stats.success_rate() < min_success_rate {
            warn!(
                "Final decode success rate ({:.1}%) is below minimum threshold ({:.1}%)", 
                stats.success_rate() * 100.0,
                min_success_rate * 100.0
            );
        }
        
        // 檢查成功率是否低於閾值（至少需要處理 10 個封包後才檢查）
        if stats.total_packets > 10 && stats.success_rate() < min_success_rate {
            return Err(SubXError::audio_processing(format!(
                "Decode success rate ({:.1}%) below minimum threshold ({:.1}%), output quality unacceptable",
                stats.success_rate() * 100.0,
                min_success_rate * 100.0
            )));
        }
        
        Ok((wav_path, stats))
    }
    
    // 保持向後相容性的方法
    pub async fn transcode_to_wav<P: AsRef<Path>>(&self, input_path: P) -> Result<PathBuf> {
        let (path, stats) = self.transcode_to_wav_with_config(input_path, None).await?;
        
        // 如果成功率太低，返回警告
        if stats.success_rate() < 0.8 {
            warn!("Low decode success rate ({:.1}%), output quality may be affected", 
                  stats.success_rate() * 100.0);
        }
        
        Ok(path)
    }
}
```

## 實作步驟

### 第一階段：核心錯誤處理修復（1 天）
1. **修改 `transcode_to_wav` 方法**
   - 實現基於錯誤類型的分支處理邏輯
   - 添加錯誤統計和連續錯誤計數
   - 實現最小成功率檢查

2. **添加必要的 imports**
   - 確保導入正確的 Symphonia 錯誤類型
   - 添加統計結構體定義
   - 導入 `log::warn` 宏用於錯誤記錄

3. **基礎測試**
   - 運行現有測試確保無回歸
   - 添加基本的錯誤處理測試

### 第二階段：增強功能和配置（1 天）
1. **實現 `min_success_rate` 參數處理**
   - 直接使用 f64 參數而非配置結構體
   - 實現預設值處理邏輯

2. **添加 `transcode_to_wav_with_config` 方法**
   - 實現帶配置的版本
   - 保持原有 API 的向後相容性

3. **實現統計報告機制**
   - 添加 `TranscodeStats` 結構體
   - 實現統計收集和報告邏輯
   - 簡化成功率檢查邏輯

### 第三階段：測試和驗證（1 天）
1. **單元測試**
   - 測試各種錯誤類型的處理
   - 測試統計機制的正確性
   - 測試最小成功率參數的效果

2. **整合測試**
   - 使用有損壞封包的測試檔案
   - 驗證轉碼結果的品質
   - 測試大檔案的處理性能

3. **回歸測試**
   - 確保所有現有功能正常工作
   - 驗證向後相容性

## 測試策略

### 單元測試用例

#### 1. 可恢復錯誤處理測試
```rust
#[tokio::test]
async fn test_recoverable_decode_error_handling() {
    // 建立包含解碼錯誤的測試檔案
    let transcoder = AudioTranscoder::new().unwrap();
    let (wav_path, stats) = transcoder
        .transcode_to_wav_with_config(
            "test_with_decode_errors.mp3", 
            Some(0.5) // 50% 最小成功率
        )
        .await
        .unwrap();
    
    // 驗證轉碼成功但有統計記錄
    assert!(wav_path.exists());
    assert!(stats.skipped_decode_errors > 0);
    assert!(stats.success_rate() > 0.5);
}
```

#### 2. 連續錯誤處理測試
```rust
#[tokio::test]
async fn test_min_success_rate_threshold() {
    let transcoder = AudioTranscoder::new().unwrap();
    
    // 使用嚴重損壞的檔案測試
    let result = transcoder
        .transcode_to_wav_with_config("severely_corrupted.mp3", Some(0.9)) // 要求 90% 成功率
        .await;
    
    // 應該因為成功率不足而失敗
    assert!(result.is_err());
}
```

#### 3. 成功率閾值測試
```rust
#[tokio::test]
async fn test_default_success_rate() {
    let transcoder = AudioTranscoder::new().unwrap();
    
    let result = transcoder
        .transcode_to_wav_with_config("low_quality.mp3", None) // 使用預設 50% 成功率
        .await;
    
    // 驗證使用預設值的行為
    match result {
        Ok((_, stats)) => assert!(stats.success_rate() >= 0.5),
        Err(_) => {} // 低品質檔案可能失敗，這是預期行為
    }
}
```

### 整合測試用例

#### 1. 端到端容錯測試
```rust
#[tokio::test]
async fn test_sync_command_with_partially_corrupted_audio() {
    // 使用部分損壞的音訊檔案測試完整的 sync 流程
    let temp_dir = TempDir::new().unwrap();
    let corrupted_audio = create_partially_corrupted_audio_file(&temp_dir);
    let subtitle = create_test_subtitle_file(&temp_dir);
    
    let args = SyncArgs {
        video: corrupted_audio,
        subtitle: subtitle,
        offset: None,
        batch: false,
        range: None,
        threshold: None,
    };
    
    // 應該成功完成，儘管有部分錯誤
    let result = sync_command::execute(args).await;
    assert!(result.is_ok());
}
```

### 效能測試用例

#### 1. 大檔案錯誤處理效能測試
```rust
#[tokio::test]
async fn test_large_file_error_handling_performance() {
    let transcoder = AudioTranscoder::new().unwrap();
    let start_time = std::time::Instant::now();
    
    let (wav_path, stats) = transcoder
        .transcode_to_wav_with_config("large_file_with_errors.flac", None)
        .await
        .unwrap();
    
    let duration = start_time.elapsed();
    
    // 驗證效能在合理範圍內（例如不超過檔案長度的 5 倍）
    assert!(duration.as_secs() < 300); // 5 分鐘限制
    assert!(stats.success_rate() > 0.7); // 至少 70% 成功率
}
```

## 風險和緩解策略

### 技術風險

#### 1. 效能影響
- **風險**：錯誤處理邏輯可能影響轉碼效能
- **緩解**：
  - 只在發生錯誤時才執行額外邏輯
  - 使用輕量級的統計結構體
  - 使用 Rust 標準日誌系統進行錯誤記錄

#### 2. 輸出品質
- **風險**：跳過錯誤封包可能影響音訊品質
- **緩解**：
  - 實現成功率閾值檢查
  - 提供詳細的統計報告
  - 在成功率過低時發出警告

#### 3. 向後相容性
- **風險**：API 變更可能破壞現有代碼
- **緩解**：
  - 保持原有方法簽名不變
  - 新功能通過可選參數提供
  - 充分的回歸測試

### 實作風險

#### 1. Symphonia API 理解偏差
- **風險**：對 Symphonia 錯誤類型的理解可能不準確
- **緩解**：
  - 仔細研讀官方文檔
  - 進行充分的測試驗證
  - 與社區交流確認最佳實踐

#### 2. 測試覆蓋不足
- **風險**：無法覆蓋所有錯誤情況
- **緩解**：
  - 建立多樣化的測試檔案集
  - 實現自動化的模糊測試
  - 監控生產環境的錯誤日誌

## 驗收標準

### 功能要求
- [ ] 正確處理 `DecodeError`、`IoError` 和 `ResetRequired` 錯誤類型
- [ ] 可恢復錯誤不會中斷整個轉碼過程
- [ ] 不可恢復錯誤會正確終止轉碼並返回適當錯誤
- [ ] 提供詳細的錯誤統計資訊
- [ ] 支援最小成功率參數控制錯誤處理行為

### 品質要求
- [ ] 所有新功能有完整的單元測試
- [ ] 整合測試覆蓋主要錯誤情境
- [ ] 效能不劣於修改前的實現
- [ ] 向後相容性完全保持
- [ ] 錯誤訊息清晰且有助於診斷

### 穩定性要求
- [ ] 處理大檔案時記憶體使用穩定
- [ ] 成功率過低時能及時終止並提供有意義的錯誤訊息
- [ ] 所有資源能正確釋放

## 後續改進計劃

### 短期改進（1-2 週內）
1. **錯誤日誌增強**
   - 實現結構化錯誤日誌
   - 添加錯誤發生位置（時間戳、封包索引）的記錄

2. **使用者體驗優化**
   - 添加進度條顯示轉碼進度和錯誤率
   - 提供更友好的錯誤報告格式

### 中期改進（1 個月內）
1. **自動恢復機制**
   - 實現更智慧的錯誤恢復策略
   - 添加音訊品質檢測和補償

2. **監控和度量**
   - 實現錯誤率監控
   - 添加效能度量收集

### 長期改進（3 個月內）
1. **機器學習輔助**
   - 使用歷史錯誤資料訓練模型
   - 預測和預防常見錯誤模式

2. **分散式處理**
   - 實現分塊處理大檔案
   - 支援並行錯誤處理

## 參考資料

### 技術文檔
- [Symphonia Decoder API 文檔](https://docs.rs/symphonia-core/latest/symphonia_core/codecs/trait.Decoder.html#tymethod.decode)
- [Symphonia Error 類型定義](https://docs.rs/symphonia-core/latest/symphonia_core/errors/enum.Error.html)
- [音訊處理最佳實踐指南](../docs/tech-architecture.md)

### 相關 Issue 和 PR
- 相關的音訊處理錯誤報告
- Symphonia 社區的錯誤處理討論
- 類似專案的解決方案參考

### 測試資料
- 各種格式的測試音訊檔案
- 包含已知錯誤的測試用例
- 效能基準測試資料

---

**預估工作量**：3 個工作天  
**優先級**：高  
**相關元件**：`services/audio/transcoder.rs`、音訊處理管線  
**影響範圍**：音訊轉碼功能、sync 命令穩定性
