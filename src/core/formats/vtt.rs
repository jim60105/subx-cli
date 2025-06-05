use crate::core::formats::{Subtitle, SubtitleFormat};
use crate::error::SubXError;
use crate::Result;

/// WebVTT (.vtt) 格式解析（暫未實作）
pub struct VttFormat;

impl SubtitleFormat for VttFormat {
    fn parse(&self, _content: &str) -> Result<Subtitle> {
        Err(SubXError::subtitle_format(
            self.format_name(),
            "VTT 格式尚未實作",
        ))
    }

    fn serialize(&self, _subtitle: &Subtitle) -> Result<String> {
        Err(SubXError::subtitle_format(
            self.format_name(),
            "VTT 格式序列化尚未實作",
        ))
    }

    fn detect(&self, content: &str) -> bool {
        content.trim_start().starts_with("WEBVTT")
    }

    fn format_name(&self) -> &'static str {
        "VTT"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["vtt"]
    }
}
