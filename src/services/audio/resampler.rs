//! 音訊重採樣核心模組
#![allow(dead_code)]

pub mod converter;
pub mod detector;
pub mod optimizer;
pub mod quality;
pub mod simplified;

pub use converter::{AudioResampler, ResampleConfig, ResampleQuality};
pub use detector::{AudioUseCase, AusSampleRateDetector as SampleRateDetector};
pub use optimizer::{AutoOptimizationResult, OptimizationResult, SampleRateOptimizer};
pub use quality::QualityAssessor;
pub use simplified::SimplifiedResampler;
