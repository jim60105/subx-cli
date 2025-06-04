// src/cli/sync_args.rs
use clap::{Args, ValueEnum};
use std::path::PathBuf;

/// 時間軸同步校正參數
#[derive(Args, Debug)]
pub struct SyncArgs {
    /// 影片檔案路徑
    pub video: PathBuf,

    /// 字幕檔案路徑
    pub subtitle: PathBuf,

    /// 手動指定偏移量 (秒)
    #[arg(long)]
    pub offset: Option<f64>,

    /// 批量處理模式
    #[arg(long)]
    pub batch: bool,

    /// 偏移檢測範圍 (秒)
    #[arg(long)]
    pub range: Option<f64>,

    /// 同步方法 (audio|manual)
    #[arg(long, value_enum)]
    pub method: SyncMethod,
}

/// 支援的同步方法
#[derive(ValueEnum, Clone, Debug)]
pub enum SyncMethod {
    Audio,
    Manual,
}
