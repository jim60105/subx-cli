//! SubX library root.

/// 套件版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod cli;
pub mod commands;
pub mod config;
pub mod core;
pub mod error;
pub type Result<T> = error::SubXResult<T>;

pub mod services;
