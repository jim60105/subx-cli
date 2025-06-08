//! 音訊重採樣核心模組
#![allow(dead_code)]

pub mod converter;
pub mod detector;
pub mod optimizer;
pub mod quality;

pub use converter::{AudioResampler, ResampleConfig, ResampleQuality};
pub use detector::SampleRateDetector;
pub use optimizer::{AutoOptimizationResult, OptimizationResult, SampleRateOptimizer};
pub use quality::QualityAssessor;
