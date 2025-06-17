//! External services integration for SubX.
//!
//! This module provides comprehensive integration with external services that power
//! SubX's intelligent subtitle processing capabilities. It serves as the abstraction
//! layer between SubX's core processing engines and external AI providers, audio
//! analysis libraries, and other third-party services.
//!
//! # Architecture Overview
//!
//! The services layer follows a provider pattern with standardized interfaces:
//! - **Service Abstraction**: Common traits for different service types
//! - **Multiple Providers**: Support for different AI and audio processing backends
//! - **Async Integration**: Non-blocking service calls with proper error handling
//! - **Resource Management**: Connection pooling, rate limiting, and caching
//! - **Failover Support**: Automatic fallback between different service providers
//!
//! # Service Categories
//!
//! ## AI Services (`ai` module)
//! Intelligent content analysis and matching services:
//! - **Content Analysis**: Video and subtitle content understanding
//! - **Semantic Matching**: AI-powered file pairing with confidence scoring
//! - **Language Detection**: Automatic language identification and verification
//! - **Quality Assessment**: Content quality evaluation and recommendations
//! - **Multi-Provider Support**: OpenAI, Anthropic, and other AI service backends
//!
//! ## Audio Processing (`audio` module)
//! Advanced audio analysis and synchronization services:
//! - **Dialogue Detection**: Speech segment identification and timing
//! - **Audio Feature Extraction**: Waveform analysis and acoustic features
//! - **Synchronization Analysis**: Audio-subtitle timing correlation
//! - **Voice Activity Detection**: Speech vs. silence classification
//! - **Multi-Language Support**: Language-specific audio processing models
//!
//! # Usage Patterns
//!
//! ## AI-Powered Matching
//! ```rust,ignore
//! use subx_cli::services::ai::{AIClientFactory, AnalysisRequest};
//!
//! async fn intelligent_matching() -> subx_cli::Result<()> {
//!     let ai_client = AIClientFactory::create_client("openai").await?;
//!     
//!     let request = AnalysisRequest {
//!         video_files: vec!["movie.mp4".to_string()],
//!         subtitle_files: vec!["subs1.srt".to_string(), "subs2.srt".to_string()],
//!         content_samples: vec![/* content samples */],
//!     };
//!     
//!     let result = ai_client.analyze_content(request).await?;
//!     println!("Best match confidence: {}", result.confidence);
//!     Ok(())
//! }
//! ```
//!
//! ## Audio Synchronization
//! ```rust,ignore
//! use subx_cli::services::vad::LocalVadDetector;
//! use subx_cli::config::VadConfig;
//!
//! async fn synchronize_audio() -> subx_cli::Result<()> {
//!     let vad_config = VadConfig::default();
//!     let detector = LocalVadDetector::new(vad_config)?;
//!
//!     // 直接處理各種音訊格式，無需轉碼
//!     let result = detector.detect_speech("video.mp4").await?;
//!
//!     println!("Detected {} speech segments", result.speech_segments.len());
//!     Ok(())
//! }
//! ```
//!
//! # Performance Considerations
//!
//! - **Caching Strategy**: Aggressive caching of AI analysis results and audio features
//! - **Async Processing**: Non-blocking service calls with concurrent processing
//! - **Resource Pooling**: Connection and compute resource management
//! - **Rate Limiting**: Built-in rate limiting for external API compliance
//! - **Memory Efficiency**: Streaming processing for large audio files
//!
//! # Error Handling
//!
//! All services use standardized error handling with automatic retry logic:
//! - **Network Failures**: Automatic retry with exponential backoff
//! - **Service Unavailability**: Graceful fallback to alternative providers
//! - **Rate Limiting**: Intelligent backoff and queue management
//! - **Data Validation**: Input validation and sanitization
//!
//! # Configuration
//!
//! Services are configured through SubX's unified configuration system:
//! - API credentials and endpoints
//! - Performance tuning parameters
//! - Provider priority and fallback rules
//! - Caching and resource limits
//!
//! # Modules
//!
//! - [`ai`] - AI service providers for content analysis and intelligent matching
//! - [`audio`] - Audio processing and synchronization analysis utilities
#![allow(dead_code)]

pub mod ai;
pub mod audio;

// VAD service modules
pub mod vad;
