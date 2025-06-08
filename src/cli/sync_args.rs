// src/cli/sync_args.rs
use clap::Args;
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
}

/// 同步方法
#[derive(Debug, Clone, PartialEq)]
pub enum SyncMethod {
    /// 自動同步：使用音訊分析
    Auto,
    /// 手動偏移：使用指定的時間偏移
    Manual,
}

impl SyncArgs {
    /// 根據 offset 參數自動判斷同步方法
    pub fn sync_method(&self) -> SyncMethod {
        if self.offset.is_some() {
            SyncMethod::Manual
        } else {
            SyncMethod::Auto
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_method_selection_manual() {
        let args = SyncArgs {
            video: PathBuf::from("video.mp4"),
            subtitle: PathBuf::from("subtitle.srt"),
            offset: Some(2.5),
            batch: false,
            range: None,
        };
        assert_eq!(args.sync_method(), SyncMethod::Manual);
    }

    #[test]
    fn test_sync_method_selection_auto() {
        let args = SyncArgs {
            video: PathBuf::from("video.mp4"),
            subtitle: PathBuf::from("subtitle.srt"),
            offset: None,
            batch: false,
            range: None,
        };
        assert_eq!(args.sync_method(), SyncMethod::Auto);
    }
}
