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
    #[arg(long, default_value = "80", value_parser = clap::value_parser!(u8).range(0..=100))]
    pub confidence: u8,

    /// 遞歸處理子資料夾
    #[arg(short, long)]
    pub recursive: bool,

    /// 重命名前備份原檔案
    #[arg(long)]
    pub backup: bool,
}

// 測試參數解析行為
#[cfg(test)]
mod tests {
    use crate::cli::{Cli, Commands};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_match_args_default_values() {
        let cli = Cli::try_parse_from(&["subx-cli", "match", "path"]).unwrap();
        let args = match cli.command {
            Commands::Match(m) => m,
            _ => panic!("Expected Match command"),
        };
        assert_eq!(args.path, PathBuf::from("path"));
        assert!(!args.dry_run);
        assert!(!args.recursive);
        assert!(!args.backup);
        assert_eq!(args.confidence, 80);
    }

    #[test]
    fn test_match_args_parsing() {
        let cli = Cli::try_parse_from(&[
            "subx-cli",
            "match",
            "path",
            "--dry-run",
            "--recursive",
            "--backup",
            "--confidence",
            "50",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Match(m) => m,
            _ => panic!("Expected Match command"),
        };
        assert!(args.dry_run);
        assert!(args.recursive);
        assert!(args.backup);
        assert_eq!(args.confidence, 50);
    }

    #[test]
    fn test_match_args_invalid_confidence() {
        let res = Cli::try_parse_from(&["subx-cli", "match", "path", "--confidence", "150"]);
        assert!(res.is_err());
    }
}
