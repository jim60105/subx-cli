//! Core processing engine for SubX.
//!
//! This module contains core subsystems for file operations, subtitle format
//! handling, language detection, matching algorithms, parallel processing,
//! synchronization, and dependency injection management.
//!
//! Each subsystem is organized into its own submodule:
//! - `file_manager` for safe file operations with rollback support
//! - `formats` for parsing and converting subtitle formats
//! - `language` for language detection and handling
//! - `matcher` for AI-powered subtitle matching algorithms
//! - `parallel` for task scheduling and parallel execution
//! - `sync` for audio-text synchronization engines
//! - `factory` for component creation with dependency injection
//! - `services` for service container and dependency management
//!
#![allow(dead_code)]

pub mod factory;
pub mod file_manager;
pub mod formats;
pub mod language;
pub mod matcher;
pub mod parallel;
pub mod services;
pub mod sync;

// Re-export commonly used types
pub use factory::ComponentFactory;
pub use services::ServiceContainer;
