# Backlog 43: VAD 音頻處理最佳化

## 計劃摘要

本計劃旨在最佳化 VAD (Voice Activity Detection) 音頻處理模組，移除不必要的重採樣和聲道轉換設計，改為使用原始音頻採樣率進行語音檢測，並簡化配置結構。

## 背景分析

### 現狀問題

目前 VAD 音頻處理模組存在以下設計問題：

1. **不必要的重採樣**：`VadAudioProcessor::resample_audio()` 方法將所有音頻重採樣到配置的採樣率（默認 16kHz）
2. **多聲道混合**：`VadAudioProcessor::convert_to_mono()` 方法將多聲道音頻平均混合為單聲道
3. **配置複雜度**：`VadConfig` 包含 `sample_rate` 和 `chunk_size` 參數，增加了配置複雜度
4. **處理效率**：重採樣和混合操作增加了不必要的計算開銷

### 技術架構分析

根據代碼分析，現有的 VAD 處理流程如下：

```
音頻檔案 → DirectAudioLoader → VadAudioProcessor → LocalVadDetector
                                     ↓
                            1. resample_audio()
                            2. convert_to_mono()
                                     ↓
                            ProcessedAudioData → VAD 分析
```

## 最佳化目標

### 主要目標

1. **移除重採樣**：直接使用原始音頻採樣率進行 VAD 分析
2. **簡化聲道處理**：只使用第一個聲道（左聲道）進行 VAD 分析
3. **動態 chunk_size**：根據真實採樣率動態計算 chunk_size
4. **配置簡化**：從 `VadConfig` 中移除 `sample_rate` 和 `chunk_size` 參數

### 技術優勢

- **性能提升**：減少 CPU 計算和記憶體使用
- **音質保持**：避免重採樣可能導致的音質損失
- **配置簡化**：減少用戶需要調整的參數
- **維護性**：簡化代碼結構，提高可維護性

## 詳細實施方案

### 階段 1: 移除重採樣和聲道混合設計

#### 1.1 修改 `VadAudioProcessor`

**檔案**: `src/services/vad/audio_processor.rs`

**變更內容**:

1. **移除 `target_sample_rate` 及 `target_channels` 參數**：
   ```rust
   pub struct VadAudioProcessor {
       // 兩者皆移除，結構體將不再有任何欄位
   }
   ```

2. **簡化 `new()` 方法**：
   ```rust
   pub fn new() -> Result<Self> {
       Ok(Self {})
   }
   ```

3. **修改 `load_and_prepare_audio_direct()` 方法**：
   ```rust
   pub async fn load_and_prepare_audio_direct(
       &self,
       audio_path: &Path,
   ) -> Result<ProcessedAudioData> {
       // 1. 使用 DirectAudioLoader 載入音頻
       let loader = DirectAudioLoader::new()?;
       let (samples, info) = loader.load_audio_samples(audio_path)?;
       
       // 2. 如果是多聲道，只保留第一個聲道
       let mono_samples = if info.channels == 1 {
           samples
       } else {
           self.extract_first_channel(&samples, info.channels as usize)
       };
       
       // 3. 更新音頻資訊
       let mono_info = AudioInfo {
           sample_rate: info.sample_rate, // 保持原始採樣率
           channels: 1,
           duration_seconds: mono_samples.len() as f64 / info.sample_rate as f64,
           total_samples: mono_samples.len(),
       };
       
       Ok(ProcessedAudioData {
           samples: mono_samples,
           info: mono_info,
       })
   }
   ```

4. **新增 `extract_first_channel()` 方法**：
   ```rust
   fn extract_first_channel(&self, samples: &[i16], channels: usize) -> Vec<i16> {
       samples.iter()
           .step_by(channels)
           .copied()
           .collect()
   }
   ```

5. **移除不需要的方法**：
   - 移除 `resample_audio()` 方法
   - 移除 `convert_to_mono()` 方法
   - 移除 `target_channels` 欄位及其所有相關引用

#### 1.2 更新 `LocalVadDetector`

**檔案**: `src/services/vad/detector.rs`

**變更內容**:

1. **修改 `new()` 方法**：
   ```rust
   pub fn new(config: VadConfig) -> Result<Self> {
       Ok(Self {
           config,
           audio_processor: VadAudioProcessor::new()?, // 移除參數
       })
   }
   ```

2. **修改 `detect_speech()` 方法**：
   ```rust
   pub async fn detect_speech(&self, audio_path: &Path) -> Result<VadResult> {
       let start_time = Instant::now();

       // 1. 載入和預處理音頻
       let audio_data = self
           .audio_processor
           .load_and_prepare_audio_direct(audio_path)
           .await?;

       // 2. 使用真實採樣率計算 chunk_size
       let chunk_size = self.calculate_chunk_size(audio_data.info.sample_rate);

       // 3. 創建 VAD 實例
       let vad = VoiceActivityDetector::builder()
           .sample_rate(audio_data.info.sample_rate) // 使用真實採樣率
           .chunk_size(chunk_size)
           .build()
           .map_err(|e| SubXError::audio_processing(format!("Failed to create VAD: {}", e)))?;

       // 4. 執行語音檢測
       let speech_segments = self.detect_speech_segments(vad, &audio_data.samples, audio_data.info.sample_rate)?;

       let processing_duration = start_time.elapsed();

       Ok(VadResult {
           speech_segments,
           processing_duration,
           audio_info: audio_data.info,
       })
   }
   ```

3. **新增 `calculate_chunk_size()` 方法**：
   ```rust
   fn calculate_chunk_size(&self, sample_rate: u32) -> usize {
       let calculated_size = sample_rate / 16;
       calculated_size.max(1024) as usize // 最小值為 1024
   }
   ```

4. **修改 `detect_speech_segments()` 方法**：
   ```rust
   fn detect_speech_segments(
       &self,
       vad: VoiceActivityDetector,
       samples: &[i16],
       sample_rate: u32, // 新增參數
   ) -> Result<Vec<SpeechSegment>> {
       let mut segments = Vec::new();
       let chunk_size = self.calculate_chunk_size(sample_rate);
       let chunk_duration_seconds = chunk_size as f64 / sample_rate as f64;

       // 其餘邏輯保持不變...
   }
   ```

### 階段 2: 移除配置參數

#### 2.1 修改 `VadConfig`

**檔案**: `src/config/mod.rs`

**變更內容**:

1. **移除配置欄位**：
   ```rust
   #[derive(Debug, Serialize, Deserialize, Clone)]
   pub struct VadConfig {
       /// Whether to enable local VAD method
       pub enabled: bool,
       /// Speech detection sensitivity (0.0-1.0)
       pub sensitivity: f32,
       // 移除 pub chunk_size: usize,
       // 移除 pub sample_rate: u32,
       /// Padding chunks before and after speech detection
       pub padding_chunks: u32,
       /// Minimum speech duration in milliseconds
       pub min_speech_duration_ms: u32,
       /// Speech segment merge gap in milliseconds
       pub speech_merge_gap_ms: u32,
   }
   ```

2. **更新 `Default` 實現**：
   ```rust
   impl Default for VadConfig {
       fn default() -> Self {
           Self {
               enabled: true,
               sensitivity: 0.75,
               // 移除 chunk_size: 512,
               // 移除 sample_rate: 16000,
               padding_chunks: 3,
               min_speech_duration_ms: 100,
               speech_merge_gap_ms: 200,
           }
       }
   }
   ```

#### 2.2 更新配置驗證

**檔案**: `src/config/validator.rs`

**變更內容**:

1. **修改 `VadConfig::validate()` 方法**：
   ```rust
   pub fn validate(&self) -> Result<()> {
       // 檢查敏感度範圍
       if !(0.0..=1.0).contains(&self.sensitivity) {
           return Err(SubXError::config(
               "VAD sensitivity must be between 0.0 and 1.0"
           ));
       }

       // 移除 chunk_size 和 sample_rate 的驗證邏輯

       // 檢查其他參數...
       if self.padding_chunks > 10 {
           return Err(SubXError::config(
               "VAD padding_chunks must not exceed 10"
           ));
       }

       if self.min_speech_duration_ms > 5000 {
           return Err(SubXError::config(
               "VAD min_speech_duration_ms must not exceed 5000ms"
           ));
       }

       if self.speech_merge_gap_ms > 2000 {
           return Err(SubXError::config(
               "VAD speech_merge_gap_ms must not exceed 2000ms"
           ));
       }

       Ok(())
   }
   ```

### 階段 3: 更新相關程式碼

#### 3.1 更新 CLI 參數處理

**檔案**: `src/commands/sync_command.rs`

**變更內容**:

1. **移除相關 CLI 參數**：
   - 移除 `--vad-sample-rate` 參數
   - 移除 `--vad-chunk-size` 參數

2. **更新參數處理邏輯**：
   ```rust
   // 移除對 sample_rate 和 chunk_size 的覆蓋邏輯
   if let Some(sensitivity) = args.vad_sensitivity {
       config.sync.vad.sensitivity = sensitivity;
   }
   ```

#### 3.2 更新配置系統

**檔案**: `src/config/service.rs`

**變更內容**:

1. **移除配置鍵**：
   - 移除 `"sync.vad.sample_rate"` 鍵的處理
   - 移除 `"sync.vad.chunk_size"` 鍵的處理

2. **更新 `get_config_value()` 和 `set_config_value()` 方法**：
   ```rust
   // 移除對應的 match 分支
   ```

### 階段 4: 測試更新

#### 4.1 更新單元測試

**檔案**: `tests/vad_*.rs`

**變更內容**:

1. **更新 `VadAudioProcessor` 測試**：
   ```rust
   #[tokio::test]
   async fn test_load_and_prepare_real_audio_file() -> Result<()> {
       let audio_path = Path::new(env!("CARGO_MANIFEST_DIR"))
           .join("assets")
           .join("SubX - The Subtitle Revolution.mp4");

       let processor = VadAudioProcessor::new()?; // 移除參數

       let processed_audio = processor.load_and_prepare_audio_direct(&audio_path).await?;

       // 驗證保持原始採樣率
       assert_eq!(processed_audio.info.sample_rate, 48000); // 原始採樣率
       assert_eq!(processed_audio.info.channels, 1);
       assert!(!processed_audio.samples.is_empty());

       Ok(())
   }
   ```

2. **更新 `LocalVadDetector` 測試**：
   ```rust
   #[tokio::test]
   async fn test_vad_detector_with_real_audio() {
       let audio_path = get_test_audio_path();
       let vad_config = VadConfig::default();
       let detector = LocalVadDetector::new(vad_config).unwrap();
       let result = detector.detect_speech(&audio_path).await.unwrap();

       // 驗證使用真實採樣率
       assert_eq!(result.audio_info.sample_rate, 48000); // 真實採樣率
       assert!(!result.speech_segments.is_empty());
   }
   ```

3. **新增 chunk_size 計算測試**：
   ```rust
   #[test]
   fn test_chunk_size_calculation() {
       let vad_config = VadConfig::default();
       let detector = LocalVadDetector::new(vad_config).unwrap();
       
       // 測試不同採樣率的 chunk_size 計算
       assert_eq!(detector.calculate_chunk_size(16000), 1024); // 16000/16 = 1000, 使用最小值 1024
       assert_eq!(detector.calculate_chunk_size(48000), 3000); // 48000/16 = 3000
       assert_eq!(detector.calculate_chunk_size(8000), 1024);  // 8000/16 = 500, 使用最小值 1024
   }
   ```

#### 4.2 更新整合測試

**檔案**: `tests/vad_integration_tests.rs`

**變更內容**:

1. **更新配置相關測試**：
   ```markdown
   #[tokio::test]
   async fn test_vad_audio_format_compatibility() {
       let temp_dir = TempDir::new().unwrap();

       // 測試不同音頻格式和參數
       let test_cases = vec![
           (8000, 1),   // 8kHz mono
           (16000, 1),  // 16kHz mono
           (44100, 1),  // 44.1kHz mono
           (44100, 2),  // 44.1kHz stereo
           (48000, 2),  // 48kHz stereo
       ];

       let config = VadConfig::default();
       let detector = LocalVadDetector::new(config).unwrap();

       for (sample_rate, channels) in test_cases {
           let audio_path = temp_dir
               .path()
               .join(&format!("test_{}_{}.wav", sample_rate, channels));
           create_test_audio_with_format(&audio_path, sample_rate, channels);

           let result = detector.detect_speech(&audio_path).await;
           assert!(result.is_ok(), "Failed for format: {}Hz, {} channels", sample_rate, channels);
           
           let vad_result = result.unwrap();
           // 驗證保持原始採樣率
           assert_eq!(vad_result.audio_info.sample_rate, sample_rate);
           assert_eq!(vad_result.audio_info.channels, 1); // 應該轉為單聲道
       }
   }
   ```

### 階段 5: 文件更新

#### 5.1 更新配置文件

**檔案**: `docs/configuration-guide.md`

**變更內容**:

1. **更新 VAD 配置說明**：
   ```markdown
   ### VAD 配置 (`[sync.vad]`)

   | 參數 | 類型 | 預設值 | 說明 |
   |------|------|--------|------|
   | `enabled` | bool | true | 是否啟用本地 VAD 方法 |
   | `sensitivity` | f32 | 0.75 | 語音檢測敏感度 (0.0-1.0) |
   | `padding_chunks` | u32 | 3 | 語音檢測前後填充塊數 |
   | `min_speech_duration_ms` | u32 | 100 | 最小語音持續時間（毫秒） |
   | `speech_merge_gap_ms` | u32 | 200 | 語音段合併間隔（毫秒） |

   > **注意**: 從 v1.x 開始，VAD 系統自動使用音頻檔案的原始採樣率，不再需要配置 `sample_rate` 和 `chunk_size` 參數。
   ```

2. **新增自動化說明**：
   ```markdown
   ### 自動化處理

   - **採樣率自動檢測**: VAD 系統自動使用音頻檔案的原始採樣率，無需手動配置
   - **Chunk Size 自動計算**: 根據公式 `chunk_size = max(sample_rate / 16, 1024)` 自動計算
   - **聲道簡化**: 多聲道音頻自動使用第一個聲道進行分析
   ```

#### 5.2 更新技術架構文件

**檔案**: `docs/tech-architecture.md`

**變更內容**:

1. **更新 VAD 處理流程說明**：
   ```markdown
   #### VAD 處理流程 (最佳化版本)

   ```
   音頻檔案 → DirectAudioLoader → 第一聲道提取 → VAD 分析
                                      ↓
                              保持原始採樣率，動態計算 chunk_size
   ```

   **最佳化特點**:
   - 無重採樣處理，保持原始音質
   - 只使用第一聲道，減少計算開銷
   - 動態 chunk_size 計算，適應不同採樣率
   - 簡化配置，減少用戶設定複雜度
   ```

## 風險評估與解決方案

### 主要風險

1. **向後兼容性**：移除配置參數可能影響現有配置檔案
2. **性能差異**：不同採樣率可能導致 VAD 性能差異
3. **第一聲道品質**：某些音頻檔案的第一聲道可能品質較差

### 解決方案

1. **配置遷移**：
   - 自動忽略不存在的配置鍵
   - 從程式碼中完全移除 `sample_rate` 和 `chunk_size` 參數，不要留下 [deprecated] 標記

2. **性能監控**：
   - 在測試中驗證不同採樣率的 VAD 性能
   - 調整 chunk_size 計算公式以確保最佳性能
   - 提供性能基準測試

3. **品質保證**：
   - 在測試中使用 "assets/SubX - The Subtitle Revolution.mp3" 驗證 (它的 sample_rate 是 48000)
   - 提供日誌記錄以便調試

## 測試計劃

### 單元測試

1. **VadAudioProcessor 測試**：
   - 測試不同音頻格式的載入
   - 測試第一聲道提取功能
   - 測試音頻資訊的正確性

2. **LocalVadDetector 測試**：
   - 測試 chunk_size 計算邏輯
   - 測試不同採樣率的 VAD 性能
   - 測試語音檢測準確性

3. **配置測試**：
   - 測試移除參數後的配置載入
   - 測試配置驗證邏輯
   - 測試預設值設定

### 整合測試

1. **端到端測試**：
   - 測試完整的 sync 命令流程

2. **相容性測試**：
   - 測試現有配置檔案的處理
   - 測試 CLI 參數的正確性
   - 測試錯誤處理

## 部署計劃

### 階段性部署

1. **開發階段**：
   - 實施所有程式碼變更
   - 完成單元測試和整合測試
   - 性能基準測試

2. **測試階段**：
   - 內部測試驗證

3. **發布階段**：
   - 更新文件和配置指南
   - 發布版本說明
   - 提供遷移指南

## 後續最佳化機會

## 成功指標

### 維護指標

- 程式碼複雜度降低
- 測試覆蓋率維持 75% 以上
- 維護成本降低

這個最佳化計劃將顯著提升 VAD 系統的性能和用戶體驗，同時簡化系統架構和維護複雜度。
