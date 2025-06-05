use crate::core::formats::{Subtitle, SubtitleFormat};
use crate::error::SubXError;
use crate::Result;
use regex::Regex;

/// MicroDVD/SubViewer SUB 格式解析（暫未實作）
pub struct SubFormat;

impl SubtitleFormat for SubFormat {
    fn parse(&self, _content: &str) -> Result<Subtitle> {
        Err(SubXError::subtitle_format(
            self.format_name(),
            "SUB 格式尚未實作",
        ))
    }

    fn serialize(&self, _subtitle: &Subtitle) -> Result<String> {
        Err(SubXError::subtitle_format(
            self.format_name(),
            "SUB 格式序列化尚未實作",
        ))
    }

    fn detect(&self, content: &str) -> bool {
        if let Ok(re) = Regex::new(r"^\{\d+\}\{\d+\}") {
            return re.is_match(content.trim_start());
        }
        false
    }

    fn format_name(&self) -> &'static str {
        "SUB"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["sub"]
    }
}
