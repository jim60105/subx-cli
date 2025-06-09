//! SubX 音訊服務模組

pub mod aus_adapter;
pub use aus_adapter::AusAdapter;

pub mod analyzer;
pub use analyzer::{AudioFeatures, AusAudioAnalyzer, FrameFeatures};

pub mod dialogue_detector;
pub use dialogue_detector::AusDialogueDetector;

/// 音訊能量包絡
#[derive(Debug, Clone)]
pub struct AudioEnvelope {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration: f32,
}

/// 對話段落
#[derive(Debug, Clone)]
pub struct DialogueSegment {
    pub start_time: f32,
    pub end_time: f32,
    pub intensity: f32,
}

/// 音訊原始資料元資料
#[derive(Debug, Clone)]
pub struct AudioMetadata {
    pub sample_rate: u32,
    pub channels: usize,
    pub duration: f32,
}

/// 音訊原始樣本資料
#[derive(Debug, Clone)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
    pub duration: f32,
}

/// 主要音訊分析器 (基於 aus 實作)
pub type AudioAnalyzer = AusAudioAnalyzer;
