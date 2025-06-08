use crate::core::formats::{Subtitle, SubtitleFormat};

/// 格式管理器：自動檢測與選擇適當的解析器
pub struct FormatManager {
    formats: Vec<Box<dyn SubtitleFormat>>,
}

impl Default for FormatManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatManager {
    /// 建立管理器並註冊所有格式
    pub fn new() -> Self {
        Self {
            formats: vec![
                Box::new(crate::core::formats::ass::AssFormat),
                Box::new(crate::core::formats::vtt::VttFormat),
                Box::new(crate::core::formats::srt::SrtFormat),
                Box::new(crate::core::formats::sub::SubFormat),
            ],
        }
    }

    /// 自動檢測格式並解析
    pub fn parse_auto(&self, content: &str) -> crate::Result<Subtitle> {
        for fmt in &self.formats {
            if fmt.detect(content) {
                return fmt.parse(content);
            }
        }
        Err(crate::error::SubXError::subtitle_format(
            "Unknown",
            "未知的字幕格式",
        ))
    }

    /// 根據格式名稱取得解析器
    pub fn get_format(&self, name: &str) -> Option<&dyn SubtitleFormat> {
        let lname = name.to_lowercase();
        self.formats
            .iter()
            .find(|f| f.format_name().to_lowercase() == lname)
            .map(|f| f.as_ref())
    }

    /// 根據副檔名取得解析器
    pub fn get_format_by_extension(&self, ext: &str) -> Option<&dyn SubtitleFormat> {
        let ext_lc = ext.to_lowercase();
        self.formats
            .iter()
            .find(|f| f.file_extensions().contains(&ext_lc.as_str()))
            .map(|f| f.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::formats::SubtitleFormatType;
    use std::time::Duration;

    const SAMPLE_SRT: &str = "1\n00:00:00,000 --> 00:00:01,000\nOne\n";
    const SAMPLE_VTT: &str = "WEBVTT\n\n1\n00:00:00.000 --> 00:00:01.000\nOne\n";
    const SAMPLE_WEBVTT_THREE_LINES: &str = "WEBVTT\n\n1\n00:00:01.000 --> 00:00:03.000\n第一句字幕內容\n\n2\n00:00:04.000 --> 00:00:06.000\n第二句字幕內容\n\n3\n00:00:07.000 --> 00:00:09.000\n第三句字幕內容\n";

    #[test]
    fn test_get_format_by_name_and_extension() {
        let mgr = FormatManager::new();
        let srt = mgr.get_format("srt").expect("get_format srt");
        assert_eq!(srt.format_name(), "SRT");
        let vtt = mgr
            .get_format_by_extension("vtt")
            .expect("get_format_by_extension vtt");
        assert_eq!(vtt.format_name(), "VTT");
    }

    #[test]
    fn test_parse_auto_supported_and_error() {
        let mgr = FormatManager::new();
        let sub = mgr.parse_auto(SAMPLE_SRT).expect("parse_auto srt");
        assert_eq!(sub.format, SubtitleFormatType::Srt);
        let subv = mgr.parse_auto(SAMPLE_VTT).expect("parse_auto vtt");
        assert_eq!(subv.format, SubtitleFormatType::Vtt);
        let err = mgr.parse_auto("no format");
        assert!(err.is_err());
    }

    #[test]
    fn test_webvtt_parse_auto_first_subtitle_content() {
        let mgr = FormatManager::new();

        let subtitle = mgr
            .parse_auto(SAMPLE_WEBVTT_THREE_LINES)
            .expect("Failed to parse WEBVTT format using parse_auto");

        // 驗證自動檢測為 WEBVTT 格式
        assert_eq!(
            subtitle.format,
            SubtitleFormatType::Vtt,
            "Auto detection should identify as WEBVTT format"
        );

        // 驗證共解析到 3 條字幕
        assert_eq!(
            subtitle.entries.len(),
            3,
            "Should parse exactly 3 subtitle entries"
        );

        // 驗證第一條字幕的內容、索引與時間軸
        let first = &subtitle.entries[0];
        assert_eq!(
            first.text, "第一句字幕內容",
            "First subtitle content should be correctly parsed"
        );
        assert_eq!(first.index, 1, "First subtitle should have index 1");
        assert_eq!(
            first.start_time,
            Duration::from_millis(1000),
            "First subtitle start time should be 1 second"
        );
        assert_eq!(
            first.end_time,
            Duration::from_millis(3000),
            "First subtitle end time should be 3 seconds"
        );

        // 驗證其他字幕內容
        assert_eq!(subtitle.entries[1].text, "第二句字幕內容");
        assert_eq!(subtitle.entries[2].text, "第三句字幕內容");
    }
}
