# Product Backlog #09: 音訊處理與時間軸同步

## 領域範圍
音訊特徵提取、時間軸分析、自動同步、手動校正

## 完成項目

### 1. 音訊處理基礎
- [ ] 音訊檔案讀取 (MP4, MKV, AVI 等)
- [ ] 音訊格式解碼支援
- [ ] 採樣率轉換和標準化
- [ ] 音訊能量包絡提取

### 2. 對話檢測算法
- [ ] 語音活動檢測 (VAD)
- [ ] 音量閾值分析
- [ ] 對話時間段識別
- [ ] 靜音區段過濾

### 3. 字幕時間軸分析
- [ ] 字幕時間點提取
- [ ] 對話持續時間計算
- [ ] 字幕密度分析
- [ ] 時間軸信號生成

### 4. 交叉相關分析
- [ ] 時域交叉相關計算
- [ ] 最佳偏移檢測
- [ ] 信心度評估
- [ ] 多重峰值處理

### 5. 同步引擎實作
- [ ] 自動偏移檢測
- [ ] 手動偏移調整
- [ ] 批量同步處理
- [ ] 同步結果驗證

### 6. 音訊工具整合
- [ ] FFmpeg 整合
- [ ] 音訊預處理管線
- [ ] 快取和效能優化
- [ ] 錯誤恢復機制

## 技術設計

### 音訊處理核心
```rust
// src/services/audio/mod.rs
use symphonia::core::audio::{AudioBuffer, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use std::path::Path;

pub struct AudioAnalyzer {
    sample_rate: u32,
    window_size: usize,
    hop_size: usize,
}

#[derive(Debug, Clone)]
pub struct AudioEnvelope {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration: f32,
}

#[derive(Debug, Clone)]
pub struct DialogueSegment {
    pub start_time: f32,
    pub end_time: f32,
    pub intensity: f32,
}

impl AudioAnalyzer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            window_size: 1024,
            hop_size: 512,
        }
    }
    
    pub async fn extract_envelope(&self, audio_path: &Path) -> crate::Result<AudioEnvelope> {
        // 使用 symphonia 讀取音訊
        let file = std::fs::File::open(audio_path)?;
        let media_source_stream = symphonia::core::io::MediaSourceStream::new(
            Box::new(file), 
            Default::default()
        );
        
        let format_opts = FormatOptions::default();
        let metadata_opts = Default::default();
        
        let probed = symphonia::default::get_probe()
            .format(&format_opts, media_source_stream, &metadata_opts)?;
        
        let mut format = probed.format;
        let decoder_opts = DecoderOptions::default();
        
        // 找到音訊軌道
        let track = format.tracks()
            .iter()
            .find(|t| t.codec_params.codec.is_audio())
            .ok_or_else(|| crate::SubXError::AudioProcessing("找不到音訊軌道".to_string()))?;
        
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)?;
        
        let mut samples = Vec::new();
        let mut total_duration = 0.0;
        
        // 解碼音訊資料
        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(_) => break,
            };
            
            if packet.track_id() == track.id {
                let audio_buf = decoder.decode(&packet)?;
                
                // 轉換為單聲道並提取能量
                let envelope_chunk = self.extract_energy_from_buffer(&audio_buf);
                samples.extend(envelope_chunk);
                
                total_duration += packet.dur as f32 / track.codec_params.sample_rate.unwrap_or(self.sample_rate) as f32;
            }
        }
        
        Ok(AudioEnvelope {
            samples,
            sample_rate: self.sample_rate,
            duration: total_duration,
        })
    }
    
    fn extract_energy_from_buffer(&self, audio_buf: &AudioBuffer<f32>) -> Vec<f32> {
        let mut energy_samples = Vec::new();
        let channels = audio_buf.frames();
        
        // 轉換為單聲道並計算 RMS 能量
        for chunk in channels.chunks(self.hop_size) {
            let mut sum_squares = 0.0;
            let mut sample_count = 0;
            
            for frame in chunk {
                // 混合所有聲道為單聲道
                let mono_sample: f32 = frame.iter().sum::<f32>() / frame.len() as f32;
                sum_squares += mono_sample * mono_sample;
                sample_count += 1;
            }
            
            let rms = if sample_count > 0 {
                (sum_squares / sample_count as f32).sqrt()
            } else {
                0.0
            };
            
            energy_samples.push(rms);
        }
        
        energy_samples
    }
    
    pub fn detect_dialogue(&self, envelope: &AudioEnvelope, threshold: f32) -> Vec<DialogueSegment> {
        let mut segments = Vec::new();
        let mut in_dialogue = false;
        let mut segment_start = 0.0;
        
        let time_per_sample = envelope.duration / envelope.samples.len() as f32;
        
        for (i, &energy) in envelope.samples.iter().enumerate() {
            let current_time = i as f32 * time_per_sample;
            
            if energy > threshold && !in_dialogue {
                // 對話開始
                in_dialogue = true;
                segment_start = current_time;
            } else if energy <= threshold && in_dialogue {
                // 對話結束
                in_dialogue = false;
                
                // 只保留足夠長的對話段
                if current_time - segment_start > 0.5 {
                    segments.push(DialogueSegment {
                        start_time: segment_start,
                        end_time: current_time,
                        intensity: energy,
                    });
                }
            }
        }
        
        segments
    }
}
```

### 同步引擎實作
```rust
// src/core/sync/engine.rs
use crate::services::audio::{AudioAnalyzer, AudioEnvelope, DialogueSegment};
use crate::core::formats::Subtitle;

pub struct SyncEngine {
    audio_analyzer: AudioAnalyzer,
    config: SyncConfig,
}

#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub max_offset_seconds: f32,
    pub correlation_threshold: f32,
    pub dialogue_threshold: f32,
    pub min_dialogue_length: f32,
}

#[derive(Debug)]
pub struct SyncResult {
    pub offset_seconds: f32,
    pub confidence: f32,
    pub method_used: SyncMethod,
    pub correlation_peak: f32,
}

#[derive(Debug)]
pub enum SyncMethod {
    AudioCorrelation,
    ManualOffset,
    PatternMatching,
}

impl SyncEngine {
    pub fn new(config: SyncConfig) -> Self {
        Self {
            audio_analyzer: AudioAnalyzer::new(16000), // 16kHz 採樣率
            config,
        }
    }
    
    pub async fn sync_subtitle(
        &self,
        video_path: &Path,
        subtitle: &Subtitle,
    ) -> crate::Result<SyncResult> {
        // 1. 提取音訊能量包絡
        let audio_envelope = self.audio_analyzer.extract_envelope(video_path).await?;
        
        // 2. 檢測對話段落
        let dialogue_segments = self.audio_analyzer.detect_dialogue(
            &audio_envelope, 
            self.config.dialogue_threshold
        );
        
        // 3. 生成字幕時間信號
        let subtitle_signal = self.generate_subtitle_signal(subtitle, audio_envelope.duration);
        
        // 4. 執行交叉相關分析
        let correlation_result = self.calculate_cross_correlation(
            &audio_envelope,
            &subtitle_signal,
        )?;
        
        Ok(correlation_result)
    }
    
    fn generate_subtitle_signal(&self, subtitle: &Subtitle, total_duration: f32) -> Vec<f32> {
        let sample_rate = 16000.0; // 與音訊分析一致
        let signal_length = (total_duration * sample_rate) as usize;
        let mut signal = vec![0.0; signal_length];
        
        for entry in &subtitle.entries {
            let start_sample = (entry.start_time.as_secs_f32() * sample_rate) as usize;
            let end_sample = (entry.end_time.as_secs_f32() * sample_rate) as usize;
            
            // 在字幕時間範圍內設置信號強度
            for i in start_sample..end_sample.min(signal_length) {
                signal[i] = 1.0;
            }
        }
        
        signal
    }
    
    fn calculate_cross_correlation(
        &self,
        audio_envelope: &AudioEnvelope,
        subtitle_signal: &[f32],
    ) -> crate::Result<SyncResult> {
        let max_offset_samples = (self.config.max_offset_seconds * audio_envelope.sample_rate as f32) as i32;
        let mut best_offset = 0i32;
        let mut best_correlation = 0.0f32;
        
        // 遍歷可能的偏移範圍
        for offset in -max_offset_samples..=max_offset_samples {
            let correlation = self.calculate_correlation_at_offset(
                &audio_envelope.samples,
                subtitle_signal,
                offset,
            );
            
            if correlation > best_correlation {
                best_correlation = correlation;
                best_offset = offset;
            }
        }
        
        let offset_seconds = best_offset as f32 / audio_envelope.sample_rate as f32;
        let confidence = if best_correlation > self.config.correlation_threshold {
            best_correlation
        } else {
            0.0
        };
        
        Ok(SyncResult {
            offset_seconds,
            confidence,
            method_used: SyncMethod::AudioCorrelation,
            correlation_peak: best_correlation,
        })
    }
    
    fn calculate_correlation_at_offset(
        &self,
        audio_signal: &[f32],
        subtitle_signal: &[f32],
        offset: i32,
    ) -> f32 {
        let audio_len = audio_signal.len() as i32;
        let subtitle_len = subtitle_signal.len() as i32;
        
        let mut sum_product = 0.0;
        let mut sum_audio_sq = 0.0;
        let mut sum_subtitle_sq = 0.0;
        let mut valid_samples = 0;
        
        for i in 0..audio_len {
            let subtitle_idx = i + offset;
            
            if subtitle_idx >= 0 && subtitle_idx < subtitle_len {
                let audio_val = audio_signal[i as usize];
                let subtitle_val = subtitle_signal[subtitle_idx as usize];
                
                sum_product += audio_val * subtitle_val;
                sum_audio_sq += audio_val * audio_val;
                sum_subtitle_sq += subtitle_val * subtitle_val;
                valid_samples += 1;
            }
        }
        
        if valid_samples == 0 || sum_audio_sq == 0.0 || sum_subtitle_sq == 0.0 {
            return 0.0;
        }
        
        // 正規化相關係數
        sum_product / (sum_audio_sq.sqrt() * sum_subtitle_sq.sqrt())
    }
    
    pub fn apply_sync_offset(&self, subtitle: &mut Subtitle, offset_seconds: f32) -> crate::Result<()> {
        let offset_duration = std::time::Duration::from_secs_f32(offset_seconds.abs());
        
        for entry in &mut subtitle.entries {
            if offset_seconds >= 0.0 {
                // 延遲字幕
                entry.start_time += offset_duration;
                entry.end_time += offset_duration;
            } else {
                // 提前字幕
                if entry.start_time > offset_duration {
                    entry.start_time -= offset_duration;
                    entry.end_time -= offset_duration;
                } else {
                    // 如果提前時間超過開始時間，設為 0
                    let remaining = offset_duration - entry.start_time;
                    entry.start_time = std::time::Duration::ZERO;
                    
                    if entry.end_time > remaining {
                        entry.end_time -= remaining;
                    } else {
                        entry.end_time = std::time::Duration::ZERO;
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

### Sync 命令實作
```rust
// src/commands/sync_command.rs
use crate::cli::SyncArgs;
use crate::core::sync::{SyncEngine, SyncConfig};

pub async fn execute(args: SyncArgs) -> crate::Result<()> {
    let config = SyncConfig {
        max_offset_seconds: args.range.unwrap_or(30.0),
        correlation_threshold: 0.3,
        dialogue_threshold: 0.01,
        min_dialogue_length: 0.5,
    };
    
    let sync_engine = SyncEngine::new(config);
    
    if let Some(manual_offset) = args.offset {
        // 手動偏移模式
        let mut subtitle = load_subtitle(&args.subtitle_file).await?;
        sync_engine.apply_sync_offset(&mut subtitle, manual_offset)?;
        save_subtitle(&subtitle, &args.subtitle_file).await?;
        
        println!("✓ 已應用手動偏移: {}秒", manual_offset);
    } else if args.batch {
        // 批量同步模式
        let media_pairs = discover_media_pairs(&args.directory).await?;
        
        for (video_file, subtitle_file) in media_pairs {
            match sync_single_pair(&sync_engine, &video_file, &subtitle_file).await {
                Ok(result) => {
                    println!("✓ {} - 偏移: {:.2}秒 (信心度: {:.2})", 
                        subtitle_file.display(), 
                        result.offset_seconds, 
                        result.confidence
                    );
                }
                Err(e) => {
                    println!("✗ {} - 錯誤: {}", subtitle_file.display(), e);
                }
            }
        }
    } else {
        // 單檔案同步模式
        let subtitle = load_subtitle(&args.subtitle_file).await?;
        let result = sync_engine.sync_subtitle(&args.video_file, &subtitle).await?;
        
        if result.confidence > 0.5 {
            let mut updated_subtitle = subtitle;
            sync_engine.apply_sync_offset(&mut updated_subtitle, result.offset_seconds)?;
            save_subtitle(&updated_subtitle, &args.subtitle_file).await?;
            
            println!("✓ 同步完成 - 偏移: {:.2}秒 (信心度: {:.2})", 
                result.offset_seconds, 
                result.confidence
            );
        } else {
            println!("⚠ 同步信心度較低 ({:.2})，建議手動調整", result.confidence);
        }
    }
    
    Ok(())
}
```

## 驗收標準
1. 音訊處理準確且高效
2. 對話檢測算法有效
3. 交叉相關分析準確度 > 80%
4. 同步結果信心度評估合理
5. 批量處理效能良好

## 估計工時
6-7 天

## 相依性
- 依賴 Backlog #04 (字幕格式解析引擎)

## 風險評估
- 高風險：音訊處理複雜度高
- 注意事項：音訊格式相容性、演算法準確度、效能優化
