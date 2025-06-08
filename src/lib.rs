//! SubX library root.

/// 套件版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod cli;
pub mod commands;
pub mod config;
pub use config::{init_config_manager, load_config};
pub mod core;
pub mod error;
pub type Result<T> = error::SubXResult<T>;

pub mod services;
