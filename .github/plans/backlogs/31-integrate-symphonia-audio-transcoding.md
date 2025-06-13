# Backlog 31: 整合 Symphonia 音訊轉碼功能

## 概覽

本 backlog 專注於為 SubX 專案的 `sync` 命令整合 Symphonia 音訊轉碼功能，實現對多種音訊格式的支援。目前 `aus` crate 僅支援 WAV 檔案，此功能將允許自動將其他格式的音訊檔案轉換為 WAV 格式，然後再傳遞給 `aus` crate 進行處理。

## 背景

### 當前限制
- `aus` crate 僅支援 WAV 格式音訊檔案
- 用戶若要同步非 WAV 格式的音訊檔案需要手動轉換
- 常見的影片格式（MP4、MKV、WebM、OGG）無法直接處理

### 需求分析
根據 Symphonia 文件研究，該函式庫支援以下格式和編碼器：

#### 支援的容器格式
- **MP4** (ISO/MP4) - 需要透過 `isomp4` feature 啟用
- **MKV/WebM** - 透過 `mkv` feature 啟用
- **OGG** - 透過 `ogg` feature 啟用
- **WAV** - 透過 `wav` feature 啟用（已支援）

#### 支援的編碼器
- **AAC-LC** - 透過 `aac` feature 啟用
- **ALAC** (Apple Lossless) - 透過 `alac` feature 啟用
- **FLAC** - 透過 `flac` feature 啟用
- **MP3** - 透過 `mp3` feature 啟用
- **Opus** - 透過 `opus` feature 啟用
- **PCM** - 透過 `pcm` feature 啟用
- **Vorbis** - 透過 `vorbis` feature 啟用
- **WavPack** - 透過 `wavpack` feature 啟用

## 目標

### 主要目標
1. **多格式音訊支援**：支援 MP4、MKV/WebM、OGG、WAV 等容器格式
2. **多編碼器支援**：支援 AAC、ALAC、FLAC、MP3、Opus、PCM、Vorbis、WavPack 編碼器
3. **自動轉碼**：將非 WAV 格式自動轉換為 WAV 後傳遞給 aus crate
4. **無縫整合**：不影響現有的 sync 命令工作流程

### 次要目標
1. **性能優化**：快速轉碼並管理暫存檔案
2. **錯誤處理**：提供清晰的格式不支援和轉碼錯誤訊息
3. **資源管理**：適當清理暫存檔案

## 技術規格

### Symphonia 使用流程

基於 Symphonia 文件，音訊轉碼流程如下：

1. **初始化 Probe 和 CodecRegistry**
   ```rust
   use symphonia::default::{get_probe, get_codecs};
   
   let probe = get_probe();
   let codecs = get_codecs();
   ```

2. **開啟音訊檔案**
   ```rust
   use symphonia::core::io::MediaSourceStream;
   use std::fs::File;
   
   let file = File::open(audio_path)?;
   let media_source = MediaSourceStream::new(Box::new(file), Default::default());
   ```

3. **探測格式並建立 FormatReader**
   ```rust
   let format_result = probe.format(&Default::default(), media_source, &Default::default())?;
   let mut format_reader = format_result.format;
   ```

4. **選擇音訊軌道並建立 Decoder**
   ```rust
   let track = format_reader.tracks()
       .iter()
       .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
       .ok_or_else(|| SubXError::audio_processing("No audio track found"))?;
       
   let mut decoder = codecs.make(&track.codec_params, &Default::default())?;
   ```

5. **解碼音訊並寫入 WAV**
   ```rust
   use symphonia::core::audio::SampleBuffer;
   
   let mut sample_buffer = None;
   
   loop {
       let packet = match format_reader.next_packet() {
           Ok(packet) => packet,
           Err(symphonia::core::errors::Error::IoError(ref err)) 
               if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
           Err(err) => return Err(err.into()),
       };
       
       let audio_buf = decoder.decode(&packet)?;
       
       if sample_buffer.is_none() {
           let spec = *audio_buf.spec();
           sample_buffer = Some(SampleBuffer::<f32>::new(audio_buf.capacity() as u64, spec));
       }
       
       if let Some(ref mut buf) = sample_buffer {
           buf.copy_interleaved_ref(audio_buf);
           // 寫入 WAV 檔案...
       }
   }
   ```

### 架構設計

#### 新增 AudioTranscoder 服務

```rust
// src/services/audio/transcoder.rs
pub struct AudioTranscoder {
    temp_dir: PathBuf,
    probe: Probe,
    codecs: CodecRegistry,
}

impl AudioTranscoder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            temp_dir: std::env::temp_dir().join("subx_audio_transcode"),
            probe: get_probe(),
            codecs: get_codecs(),
        })
    }
    
    /// 檢查檔案是否需要轉碼
    pub fn needs_transcoding<P: AsRef<Path>>(&self, audio_path: P) -> Result<bool> {
        // 檢查檔案副檔名和格式
    }
    
    /// 將音訊檔案轉碼為 WAV 格式
    pub async fn transcode_to_wav<P: AsRef<Path>>(&self, input_path: P) -> Result<PathBuf> {
        // 實作轉碼邏輯
    }
    
    /// 清理暫存檔案
    pub fn cleanup(&self) -> Result<()> {
        // 清理暫存目錄
    }
}
```

#### 修改 AusAudioAnalyzer

```rust
// src/services/audio/analyzer.rs
impl AusAudioAnalyzer {
    /// 載入音訊檔案（支援自動轉碼）
    pub async fn load_audio_file_with_transcoding<P: AsRef<Path>>(&self, audio_path: P) -> Result<AudioFile> {
        let transcoder = AudioTranscoder::new()?;
        
        let wav_path = if transcoder.needs_transcoding(&audio_path)? {
            transcoder.transcode_to_wav(&audio_path).await?
        } else {
            audio_path.as_ref().to_path_buf()
        };
        
        // 使用現有的 load_audio_file 方法載入 WAV 檔案
        let result = self.load_audio_file(&wav_path).await;
        
        // 如果是轉碼的暫存檔案，清理它
        if wav_path != audio_path.as_ref() {
            let _ = std::fs::remove_file(&wav_path);
        }
        
        result
    }
}
```

### Cargo.toml 修改

需要為 Symphonia 啟用所需的 features：

```toml
# 更新 symphonia 依賴
symphonia = { 
    version = "0.5", 
    features = [
        "all-codecs",  # 啟用所有編碼器
        "all-formats", # 啟用所有格式
        # 或者明確指定需要的 features：
        # "aac", "alac", "flac", "mp3", "opus", "pcm", "vorbis", "wavpack",
        # "isomp4", "mkv", "ogg", "wav"
    ] 
}
```

## 實作階段

### 階段 1：基礎架構設計（2-3 天）
- [ ] 建立 `AudioTranscoder` 結構體和基本介面
- [ ] 設計暫存檔案管理機制
- [ ] 實作檔案格式檢測邏輯
- [ ] 建立錯誤處理框架

### 階段 2：核心轉碼功能（3-4 天）
- [ ] 實作 Symphonia 初始化和設定
- [ ] 實作音訊解碼管道
- [ ] 實作 WAV 格式編碼器
- [ ] 支援立體聲和單聲道轉換

### 階段 3：格式支援實作（3-4 天）
- [ ] 實作 MP4 容器格式支援
- [ ] 實作 MKV/WebM 容器格式支援
- [ ] 實作 OGG 容器格式支援
- [ ] 支援多種編碼器：AAC、ALAC、FLAC、MP3、Opus、PCM、Vorbis、WavPack

### 階段 4：整合和優化（2-3 天）
- [ ] 修改 `AusAudioAnalyzer` 整合轉碼功能
- [ ] 更新 `sync_command` 以支援新格式
- [ ] 實作性能優化和暫存檔案管理
- [ ] 添加進度指示器（可選）

### 階段 5：測試和驗證（3-4 天）
- [ ] 建立各種格式的測試音訊檔案
- [ ] 實作單元測試覆蓋所有支援格式
- [ ] 實作整合測試驗證端到端工作流程
- [ ] 性能測試和記憶體使用優化

### 階段 6：文件和發布（1-2 天）
- [ ] 更新使用者文件說明新支援的格式
- [ ] 更新 CLI 幫助訊息
- [ ] 準備發布說明和變更日誌

## 測試策略

### 單元測試
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_mp4_to_wav_transcoding() {
        let transcoder = AudioTranscoder::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        
        // 建立測試 MP4 檔案
        let mp4_path = create_test_mp4_file(&temp_dir);
        
        let wav_path = transcoder.transcode_to_wav(&mp4_path).await.unwrap();
        assert!(wav_path.extension().unwrap() == "wav");
        
        // 驗證可以被 aus crate 載入
        let analyzer = AusAudioAnalyzer::new(44100);
        let audio_file = analyzer.load_audio_file(&wav_path).await.unwrap();
        assert!(!audio_file.samples.is_empty());
    }

    #[test]
    fn test_format_detection() {
        let transcoder = AudioTranscoder::new().unwrap();
        
        assert!(transcoder.needs_transcoding("test.mp4").unwrap());
        assert!(transcoder.needs_transcoding("test.mkv").unwrap());
        assert!(transcoder.needs_transcoding("test.ogg").unwrap());
        assert!(!transcoder.needs_transcoding("test.wav").unwrap());
    }
}
```

### 整合測試
```rust
#[tokio::test]
async fn test_sync_command_with_mp4_audio() {
    let temp_dir = TempDir::new().unwrap();
    
    // 建立測試 MP4 檔案和字幕檔案
    let mp4_path = create_test_mp4_with_audio(&temp_dir);
    let srt_path = create_test_subtitle_file(&temp_dir);
    
    let args = SyncArgs {
        video: mp4_path,
        subtitle: srt_path,
        offset: None,
        batch: false,
        range: None,
        threshold: None,
    };
    
    // 應該成功處理 MP4 檔案
    let result = sync_command::execute(args).await;
    assert!(result.is_ok());
}
```

## 風險和緩解策略

### 技術風險
1. **格式相容性問題**
   - 風險：某些特殊編碼的檔案可能無法正確解碼
   - 緩解：提供清晰的錯誤訊息和支援格式列表

2. **性能影響**
   - 風險：轉碼過程可能較慢，特別是大檔案
   - 緩解：實作並行處理和進度指示器

3. **暫存檔案管理**
   - 風險：暫存檔案可能佔用大量磁碟空間
   - 緩解：及時清理和磁碟空間檢查

### 依賴風險
1. **Symphonia 函式庫限制**
   - 風險：某些格式或編碼器可能有 bugs 或限制
   - 緩解：充分測試並準備降級方案

## 驗收標準

### 功能要求
- [ ] 支援所有指定的容器格式（MP4、MKV、WebM、OGG、WAV）
- [ ] 支援所有指定的編碼器（AAC、ALAC、FLAC、MP3、Opus、PCM、Vorbis、WavPack）
- [ ] 轉碼後的 WAV 檔案能被 aus crate 正確處理
- [ ] 暫存檔案自動清理

### 品質要求
- [ ] 所有新功能有完整的單元測試
- [ ] 整合測試覆蓋主要使用場景
- [ ] 錯誤處理提供有用的訊息
- [ ] 性能符合預期（轉碼時間不超過音訊長度的 5 倍）

### 相容性要求
- [ ] 不影響現有 WAV 檔案的處理流程
- [ ] 向後相容所有現有的 sync 命令參數
- [ ] 所有現有測試繼續通過

## 後續改進

### 短期改進
- 添加轉碼進度顯示
- 支援批量轉碼優化
- 實作更智慧的暫存檔案管理

### 長期改進
- 考慮支援更多音訊格式
- 實作音訊品質和採樣率配置選項
- 添加音訊預處理功能（降噪、正規化等）

## 參考資料

- [Symphonia 官方文件](https://docs.rs/symphonia/latest/symphonia/)
- [Symphonia GitHub 專案](https://github.com/pdeljanov/Symphonia)
- [aus crate 文件](https://docs.rs/aus/latest/aus/)
- 現有 SubX 音訊處理架構文件
