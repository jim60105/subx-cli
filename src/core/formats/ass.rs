use crate::core::formats::{Subtitle, SubtitleFormat};
use crate::error::SubXError;
use crate::Result;

/// ASS/SSA 高級字幕格式解析（暫未實作）
pub struct AssFormat;

impl SubtitleFormat for AssFormat {
    fn parse(&self, _content: &str) -> Result<Subtitle> {
        Err(SubXError::subtitle_format(
            self.format_name(),
            "ASS 格式尚未實作",
        ))
    }

    fn serialize(&self, _subtitle: &Subtitle) -> Result<String> {
        Err(SubXError::subtitle_format(
            self.format_name(),
            "ASS 格式序列化尚未實作",
        ))
    }

    fn detect(&self, content: &str) -> bool {
        content.contains("[Script Info]") || content.contains("Dialogue:")
    }

    fn format_name(&self) -> &'static str {
        "ASS"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["ass", "ssa"]
    }
}
