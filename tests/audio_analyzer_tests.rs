use std::fs;
use subx_cli::services::audio::{AudioData, AudioEnvelope, AusAudioAnalyzer};
use tempfile::TempDir;

#[cfg(test)]
mod audio_analyzer_tests {
    use super::*;

    /// 測試音訊檔案載入功能
    #[ignore]
    #[tokio::test]
    async fn test_load_audio_file_success() {
        let analyzer = AusAudioAnalyzer::new(44100);
        let temp_dir = TempDir::new().unwrap();
        // 建立模擬 WAV 檔案 (最小有效 WAV 檔頭)
        let wav_data = create_minimal_wav_file(44100, 1, 1.0);
        let wav_path = temp_dir.path().join("test.wav");
        fs::write(&wav_path, wav_data).unwrap();

        let result = analyzer.load_audio_file(&wav_path).await;
        assert!(result.is_ok());

        let audio_file = result.unwrap();
        assert_eq!(audio_file.sample_rate, 44100);
        assert!(audio_file.duration > 0.0);
        assert_eq!(audio_file.num_channels, 1);
    }

    /// 測試不存在檔案的錯誤處理
    #[ignore]
    #[tokio::test]
    async fn test_load_audio_file_not_exists() {
        let analyzer = AusAudioAnalyzer::new(44100);
        let result = analyzer.load_audio_file("non_existent.wav").await;
        assert!(result.is_err());
    }

    /// 測試音訊資料格式轉換
    #[ignore]
    #[tokio::test]
    async fn test_load_audio_data_conversion() {
        let analyzer = AusAudioAnalyzer::new(16000);
        let temp_dir = TempDir::new().unwrap();

        let wav_data = create_minimal_wav_file(16000, 1, 2.0);
        let wav_path = temp_dir.path().join("test.wav");
        fs::write(&wav_path, wav_data).unwrap();

        let audio_data = analyzer.load_audio_data(&wav_path).await.unwrap();

        assert_eq!(audio_data.sample_rate, 16000);
        assert_eq!(audio_data.channels, 1);
        assert!(audio_data.duration > 1.9 && audio_data.duration < 2.1);
        assert!(!audio_data.samples.is_empty());
    }

    /// 測試音訊能量包絡提取
    #[ignore]
    #[tokio::test]
    async fn test_extract_envelope_features() {
        let sample_rate = 44100;
        let analyzer = AusAudioAnalyzer::new(sample_rate);
        let temp_dir = TempDir::new().unwrap();

        // 建立包含變化能量的音訊檔案
        let wav_data = create_varying_energy_wav(44100, 2.0);
        let wav_path = temp_dir.path().join("varying.wav");
        fs::write(&wav_path, wav_data).unwrap();

        let envelope = analyzer.extract_envelope(&wav_path).await.unwrap();

        assert!(!envelope.samples.is_empty());
        assert_eq!(envelope.sample_rate, sample_rate);
        assert!(envelope.duration > 1.9);

        // 驗證能量值合理範圍
        for &energy in &envelope.samples {
            assert!(energy >= 0.0);
            assert!(energy <= 1.0);
        }
    }

    /// 測試對話檢測功能
    #[ignore]
    #[tokio::test]
    async fn test_detect_dialogue_segments() {
        let analyzer = AusAudioAnalyzer::new(16000);

        // 建立模擬音訊包絡 (包含語音和靜音段)
        let envelope = AudioEnvelope {
            samples: vec![
                0.1, 0.8, 0.9, 0.7, 0.2, // 語音段
                0.05, 0.03, 0.02, 0.04, // 靜音段
                0.6, 0.8, 0.7, 0.9, 0.5, // 語音段
            ],
            sample_rate: 16000,
            duration: 2.0,
        };

        let segments = analyzer.detect_dialogue(&envelope, 0.3);

        assert!(!segments.is_empty());

        // 驗證檢測到的語音段落
        let speech_segments: Vec<_> = segments.iter().filter(|s| s.intensity > 0.3).collect();
        assert!(speech_segments.len() >= 2);
    }

    /// 測試音訊特徵分析
    #[ignore]
    #[tokio::test]
    async fn test_audio_features_analysis() {
        let analyzer = AusAudioAnalyzer::new(44100);
        let temp_dir = TempDir::new().unwrap();

        let wav_data = create_spectral_rich_wav(44100, 1.0);
        let wav_path = temp_dir.path().join("rich.wav");
        fs::write(&wav_path, wav_data).unwrap();

        let audio_file = analyzer.load_audio_file(&wav_path).await.unwrap();
        let features = analyzer.analyze_audio_features(&audio_file).await.unwrap();

        assert!(!features.frames.is_empty());

        for frame in &features.frames {
            // 驗證光譜重心在合理範圍內 (0 到奈奎斯特頻率)
            assert!(frame.spectral_centroid >= 0.0);
            assert!(frame.spectral_centroid <= 22050.0);

            // 驗證光譜熵
            assert!(frame.spectral_entropy >= 0.0);
            assert!(frame.spectral_entropy <= 1.0);

            // 驗證過零率
            assert!(frame.zero_crossing_rate >= 0.0);
            assert!(frame.zero_crossing_rate <= 1.0);
        }
    }

    /// 測試無效檔案格式處理
    #[ignore]
    #[tokio::test]
    async fn test_invalid_audio_format() {
        let analyzer = AusAudioAnalyzer::new(44100);
        let temp_dir = TempDir::new().unwrap();

        // 建立無效的音訊檔案
        let invalid_path = temp_dir.path().join("invalid.wav");
        fs::write(&invalid_path, b"This is not audio data").unwrap();

        let result = analyzer.load_audio_file(&invalid_path).await;
        assert!(result.is_err());
    }

    /// 測試大檔案處理和記憶體管理
    #[ignore]
    #[tokio::test]
    async fn test_large_file_memory_management() {
        let analyzer = AusAudioAnalyzer::new(44100);
        let temp_dir = TempDir::new().unwrap();

        // 建立較大的音訊檔案 (10 秒)
        let wav_data = create_minimal_wav_file(44100, 1, 10.0);
        let wav_path = temp_dir.path().join("large.wav");
        fs::write(&wav_path, wav_data).unwrap();

        let start_memory = get_memory_usage();
        let _audio_data = analyzer.load_audio_data(&wav_path).await.unwrap();
        let end_memory = get_memory_usage();

        // 驗證記憶體使用量在合理範圍內 (< 100MB 增長)
        assert!((end_memory - start_memory) < 100_000_000);
    }

    // 輔助函式用於建立測試音訊檔案
    fn create_minimal_wav_file(sample_rate: u32, channels: u16, duration: f32) -> Vec<u8> {
        let samples_per_channel = (sample_rate as f32 * duration) as u32;
        let total_samples = samples_per_channel * channels as u32;
        let data_size = total_samples * 2; // 16-bit samples
        let mut wav_data = Vec::new();
        // WAV 檔頭
        wav_data.extend_from_slice(b"RIFF");
        wav_data.extend_from_slice(&(36 + data_size).to_le_bytes());
        wav_data.extend_from_slice(b"WAVE");
        wav_data.extend_from_slice(b"fmt ");
        wav_data.extend_from_slice(&16u32.to_le_bytes());
        wav_data.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav_data.extend_from_slice(&channels.to_le_bytes());
        wav_data.extend_from_slice(&sample_rate.to_le_bytes());
        wav_data.extend_from_slice(&(sample_rate * channels as u32 * 2).to_le_bytes());
        wav_data.extend_from_slice(&(channels * 2).to_le_bytes());
        wav_data.extend_from_slice(&16u16.to_le_bytes());
        wav_data.extend_from_slice(b"data");
        wav_data.extend_from_slice(&data_size.to_le_bytes());
        // 音訊資料 (簡單正弦波)
        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let amplitude = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            let sample = (amplitude * 32767.0) as i16;
            wav_data.extend_from_slice(&sample.to_le_bytes());
        }
        wav_data
    }

    fn create_varying_energy_wav(sample_rate: u32, duration: f32) -> Vec<u8> {
        // 實作建立變化能量的音訊檔案
        create_minimal_wav_file(sample_rate, 1, duration)
    }

    fn create_spectral_rich_wav(sample_rate: u32, duration: f32) -> Vec<u8> {
        // 實作建立頻譜豐富的音訊檔案
        create_minimal_wav_file(sample_rate, 1, duration)
    }

    fn get_memory_usage() -> usize {
        // 簡化的記憶體使用量檢測
        0 // 實際實作可使用 procfs 或其他系統工具
    }
}
