//! Refactored sync command CLI argument definitions.
//!
//! Supports multiple synchronization methods: OpenAI Whisper API, local VAD,
//! automatic selection, and manual offset. Provides fine-grained parameter
//! control and intelligent defaults.
//!
//! # Synchronization Methods
//!
//! ## OpenAI Whisper API
//! Cloud transcription service providing high-precision speech detection.
//!
//! ## Local VAD
//! Voice Activity Detection using local processing for privacy and speed.
//!
//! ## Manual Offset
//! Direct time offset specification for precise manual synchronization.

/// Synchronization method selection for CLI arguments.
///
/// Defines the available synchronization methods that can be specified
/// via command-line arguments.
#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum SyncMethodArg {
    /// Use local voice activity detection only
    Vad,
    /// Apply manual offset (requires --offset parameter)
    Manual,
}

impl From<SyncMethodArg> for crate::core::sync::SyncMethod {
    fn from(arg: SyncMethodArg) -> Self {
        match arg {
            SyncMethodArg::Vad => Self::LocalVad,
            SyncMethodArg::Manual => Self::Manual,
        }
    }
}

use crate::error::{SubXError, SubXResult};
use clap::{Args, ValueEnum};
use std::path::{Path, PathBuf};

/// 重構後的同步指令參數，支援多種同步方法
///
/// 提供完整的字幕-音訊同步功能，包括 OpenAI Whisper API、本地 VAD 檢測
/// 和手動偏移等多種方法。支援方法選擇、參數自訂和智能回退機制。
#[derive(Args, Debug)]
pub struct SyncArgs {
    /// 視訊檔案路徑，用於音訊分析
    #[arg(
        short = 'v',
        long = "video",
        value_name = "VIDEO",
        help = "視訊檔案路徑 (自動同步時必需，手動偏移時可選)",
        required_unless_present = "offset"
    )]
    pub video: Option<PathBuf>,

    /// 要同步的字幕檔案路徑
    #[arg(
        short = 's',
        long = "subtitle",
        value_name = "SUBTITLE",
        help = "字幕檔案路徑"
    )]
    pub subtitle: PathBuf,

    /// 手動時間偏移（秒），正值延遲字幕，負值提前字幕
    #[arg(
        long,
        value_name = "SECONDS",
        help = "手動偏移秒數 (正值延遲字幕，負值提前字幕)",
        conflicts_with_all = ["method", "window", "vad_sensitivity"]
    )]
    pub offset: Option<f32>,

    /// 同步方法選擇
    #[arg(short, long, value_enum, help = "同步方法")]
    pub method: Option<SyncMethodArg>,

    /// 分析時間窗口（秒）
    #[arg(
        short = 'w',
        long,
        value_name = "SECONDS",
        default_value = "30",
        help = "分析第一句字幕周圍的時間窗口（秒）"
    )]
    pub window: u32,

    // === VAD 選項 ===
    /// VAD 敏感度
    #[arg(long, value_name = "SENSITIVITY", help = "VAD 敏感度閾值 (0.0-1.0)")]
    pub vad_sensitivity: Option<f32>,

    /// VAD 塊大小
    #[arg(
        long,
        value_name = "SIZE",
        help = "VAD 音訊塊大小（樣本數）",
        value_parser = validate_chunk_size
    )]
    pub vad_chunk_size: Option<usize>,

    // === 輸出選項 ===
    /// 輸出檔案路徑
    #[arg(
        short = 'o',
        long,
        value_name = "PATH",
        help = "輸出檔案路徑 (預設: input_synced.ext)"
    )]
    pub output: Option<PathBuf>,

    /// 詳細輸出
    #[arg(long, help = "啟用包含詳細進度資訊的詳細輸出")]
    pub verbose: bool,

    /// 乾跑模式
    #[arg(long, help = "分析並顯示結果，但不儲存輸出檔案")]
    pub dry_run: bool,

    /// 強制覆蓋現有輸出檔案
    #[arg(long, help = "不經確認覆蓋現有輸出檔案")]
    pub force: bool,

    /// 啟用批次處理模式
    #[arg(short, long, help = "啟用批次處理模式")]
    pub batch: bool,

    // === 舊版/隱藏選項（已棄用） ===
    /// 最大偏移搜尋範圍（已棄用，請使用配置檔案）
    #[arg(long, hide = true)]
    #[deprecated(note = "Use configuration file instead")]
    pub range: Option<f32>,

    /// 最小相關閾值（已棄用，請使用配置檔案）
    #[arg(long, hide = true)]
    #[deprecated(note = "Use configuration file instead")]
    pub threshold: Option<f32>,
}

/// 同步方法枚舉（向後兼容）
#[derive(Debug, Clone, PartialEq)]
pub enum SyncMethod {
    /// 使用音訊分析的自動同步
    Auto,
    /// 使用指定時間偏移的手動同步
    Manual,
}

impl SyncArgs {
    /// 驗證參數組合的有效性
    pub fn validate(&self) -> Result<(), String> {
        // 檢查手動模式的參數組合
        if let Some(SyncMethodArg::Manual) = &self.method {
            if self.offset.is_none() {
                return Err("手動方法需要 --offset 參數。".to_string());
            }
        }

        // 檢查自動模式需要視訊檔案
        if self.offset.is_none() && self.video.is_none() {
            return Err("自動同步模式需要視訊檔案。\n\n\
使用方式：\n\
• 自動同步: subx sync <video> <subtitle>\n\
• 手動同步: subx sync --offset <seconds> <subtitle>\n\n\
需要幫助？執行: subx sync --help"
                .to_string());
        }

        // 檢查 VAD 參數僅在 VAD 方法時使用
        if self.vad_sensitivity.is_some() || self.vad_chunk_size.is_some() {
            match &self.method {
                Some(SyncMethodArg::Vad) => {}
                _ => return Err("VAD 選項只能與 --method vad 一起使用。".to_string()),
            }
        }

        Ok(())
    }

    /// 獲取輸出檔案路徑
    pub fn get_output_path(&self) -> PathBuf {
        if let Some(ref output) = self.output {
            output.clone()
        } else {
            create_default_output_path(&self.subtitle)
        }
    }

    /// 檢查是否為手動模式
    pub fn is_manual_mode(&self) -> bool {
        self.offset.is_some() || matches!(self.method, Some(SyncMethodArg::Manual))
    }

    /// 確定同步方法（向後兼容）
    pub fn sync_method(&self) -> SyncMethod {
        if self.offset.is_some() {
            SyncMethod::Manual
        } else {
            SyncMethod::Auto
        }
    }

    /// 驗證參數（向後兼容方法）
    pub fn validate_compat(&self) -> SubXResult<()> {
        match (self.offset.is_some(), self.video.is_some()) {
            // 手動模式：提供偏移，視訊可選
            (true, _) => Ok(()),
            // 自動模式：沒有偏移，需要視訊
            (false, true) => Ok(()),
            // 自動模式沒有視訊：無效
            (false, false) => Err(SubXError::CommandExecution(
                "自動同步模式需要視訊檔案。\n\n\
使用方式:\n\
• 自動同步: subx sync <video> <subtitle>\n\
• 手動同步: subx sync --offset <seconds> <subtitle>\n\n\
需要幫助？執行: subx sync --help"
                    .to_string(),
            )),
        }
    }

    /// 返回是否需要視訊檔案（自動同步）
    #[allow(dead_code)]
    pub fn requires_video(&self) -> bool {
        self.offset.is_none()
    }
}

// 輔助函數
fn validate_chunk_size(s: &str) -> Result<usize, String> {
    let size: usize = s.parse().map_err(|_| "無效的塊大小")?;

    if !(256..=2048).contains(&size) {
        return Err("塊大小必須介於 256 和 2048 之間".to_string());
    }

    if !size.is_power_of_two() {
        return Err("塊大小必須是 2 的冪次".to_string());
    }

    Ok(size)
}

fn create_default_output_path(input: &Path) -> PathBuf {
    let mut output = input.to_path_buf();

    if let Some(stem) = input.file_stem().and_then(|s| s.to_str()) {
        if let Some(extension) = input.extension().and_then(|s| s.to_str()) {
            let new_filename = format!("{}_synced.{}", stem, extension);
            output.set_file_name(new_filename);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_method_selection_manual() {
        let args = SyncArgs {
            video: Some(PathBuf::from("video.mp4")),
            subtitle: PathBuf::from("subtitle.srt"),
            offset: Some(2.5),
            method: None,
            window: 30,
            vad_sensitivity: None,
            vad_chunk_size: None,
            output: None,
            verbose: false,
            dry_run: false,
            force: false,
            batch: false,
            #[allow(deprecated)]
            range: None,
            #[allow(deprecated)]
            threshold: None,
        };
        assert_eq!(args.sync_method(), SyncMethod::Manual);
    }

    #[test]
    fn test_sync_method_selection_auto() {
        let args = SyncArgs {
            video: Some(PathBuf::from("video.mp4")),
            subtitle: PathBuf::from("subtitle.srt"),
            offset: None,
            method: None,
            window: 30,
            vad_sensitivity: None,
            vad_chunk_size: None,
            output: None,
            verbose: false,
            dry_run: false,
            force: false,
            batch: false,
            #[allow(deprecated)]
            range: None,
            #[allow(deprecated)]
            threshold: None,
        };
        assert_eq!(args.sync_method(), SyncMethod::Auto);
    }

    #[test]
    fn test_method_arg_conversion() {
        assert_eq!(
            crate::core::sync::SyncMethod::from(SyncMethodArg::Vad),
            crate::core::sync::SyncMethod::LocalVad
        );
        assert_eq!(
            crate::core::sync::SyncMethod::from(SyncMethodArg::Manual),
            crate::core::sync::SyncMethod::Manual
        );
    }

    #[test]
    fn test_validate_chunk_size() {
        assert!(validate_chunk_size("512").is_ok());
        assert!(validate_chunk_size("1024").is_ok());
        assert!(validate_chunk_size("256").is_ok());

        // 太小
        assert!(validate_chunk_size("128").is_err());
        // 太大
        assert!(validate_chunk_size("4096").is_err());
        // 不是 2 的冪次
        assert!(validate_chunk_size("500").is_err());
        // 無效數字
        assert!(validate_chunk_size("abc").is_err());
    }

    #[test]
    fn test_create_default_output_path() {
        let input = PathBuf::from("test.srt");
        let output = create_default_output_path(&input);
        assert_eq!(output.file_name().unwrap(), "test_synced.srt");

        let input = PathBuf::from("/path/to/movie.ass");
        let output = create_default_output_path(&input);
        assert_eq!(output.file_name().unwrap(), "movie_synced.ass");
    }
}
