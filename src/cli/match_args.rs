// src/cli/match_args.rs
use clap::Args;
use std::path::PathBuf;

/// AI 匹配重命名字幕檔案參數
#[derive(Args, Debug)]
pub struct MatchArgs {
    /// 目標資料夾路徑
    pub path: PathBuf,

    /// 預覽模式，不實際執行操作
    #[arg(long)]
    pub dry_run: bool,

    /// 最低信心度閾值 (0-100)
    #[arg(long, default_value = "80")]
    pub confidence: u8,

    /// 遞歸處理子資料夾
    #[arg(short, long)]
    pub recursive: bool,

    /// 重命名前備份原檔案
    #[arg(long)]
    pub backup: bool,
}
