// src/cli/convert_args.rs
use clap::{Args, ValueEnum};
use std::path::PathBuf;

/// 格式轉換參數
#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// 輸入檔案或資料夾路徑
    pub input: PathBuf,

    /// 目標格式 (預設值由配置檔案指定)
    #[arg(long, value_enum)]
    pub format: Option<OutputSubtitleFormat>,

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
#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum OutputSubtitleFormat {
    Srt,
    Ass,
    Vtt,
    Sub,
}

impl OutputSubtitleFormat {
    /// 取得格式字串
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputSubtitleFormat::Srt => "srt",
            OutputSubtitleFormat::Ass => "ass",
            OutputSubtitleFormat::Vtt => "vtt",
            OutputSubtitleFormat::Sub => "sub",
        }
    }
}

impl std::fmt::Display for OutputSubtitleFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// 測試參數解析行為
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_convert_args_default_values() {
        let cli = Cli::try_parse_from(&["subx-cli", "convert", "in_path"]).unwrap();
        let args = match cli.command {
            Commands::Convert(c) => c,
            _ => panic!("Expected Convert command"),
        };
        assert_eq!(args.input, PathBuf::from("in_path"));
        assert_eq!(args.format, None);
        assert_eq!(args.output, None);
        assert!(!args.keep_original);
        assert_eq!(args.encoding, "utf-8");
    }

    #[test]
    fn test_convert_args_parsing() {
        let cli = Cli::try_parse_from(&[
            "subx-cli",
            "convert",
            "in",
            "--format",
            "vtt",
            "--output",
            "out",
            "--keep-original",
            "--encoding",
            "gbk",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Convert(c) => c,
            _ => panic!("Expected Convert command"),
        };
        assert_eq!(args.input, PathBuf::from("in"));
        assert_eq!(args.format.unwrap(), OutputSubtitleFormat::Vtt);
        assert_eq!(args.output, Some(PathBuf::from("out")));
        assert!(args.keep_original);
        assert_eq!(args.encoding, "gbk");
    }
}
