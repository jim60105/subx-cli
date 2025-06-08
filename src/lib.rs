//! SubX library root.
#![allow(
    clippy::new_without_default,
    clippy::manual_clamp,
    clippy::useless_vec,
    clippy::items_after_test_module,
    clippy::needless_borrow
)]

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
