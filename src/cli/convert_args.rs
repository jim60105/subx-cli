// src/cli/convert_args.rs
use clap::{Args, ValueEnum};
use std::path::PathBuf;

/// 格式轉換參數
#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// 輸入檔案或資料夾路徑
    pub input: PathBuf,

    /// 目標格式
    #[arg(long, value_enum)]
    pub format: OutputSubtitleFormat,

    /// 輸出檔案路徑
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// 保留原始檔案
    #[arg(long)]
    pub keep_original: bool,

    /// 文字編碼
    #[arg(long, default_value = "utf-8")]
    pub encoding: String,
}

/// 支援的輸出字幕格式
#[derive(ValueEnum, Clone, Debug)]
pub enum OutputSubtitleFormat {
    Srt,
    Ass,
    Vtt,
    Sub,
}
