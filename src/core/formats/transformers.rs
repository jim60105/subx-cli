use crate::core::formats::converter::FormatConverter;
use crate::core::formats::{Subtitle, SubtitleFormatType};

impl FormatConverter {
    /// 轉換字幕物件至目標格式
    pub(crate) fn transform_subtitle(
        &self,
        subtitle: Subtitle,
        target_format: &str,
    ) -> crate::Result<Subtitle> {
        match (subtitle.format.as_str(), target_format) {
            ("srt", "ass") => self.srt_to_ass(subtitle),
            ("ass", "srt") => self.ass_to_srt(subtitle),
            ("srt", "vtt") => self.srt_to_vtt(subtitle),
            ("vtt", "srt") => self.vtt_to_srt(subtitle),
            ("ass", "vtt") => self.ass_to_vtt(subtitle),
            ("vtt", "ass") => self.vtt_to_ass(subtitle),
            (source, target) if source == target => Ok(subtitle),
            _ => Err(crate::error::SubXError::subtitle_format(
                subtitle.format.to_string(),
                format!("不支援的轉換: {} -> {}", subtitle.format, target_format),
            )),
        }
    }

    /// SRT 轉 ASS
    pub(crate) fn srt_to_ass(&self, mut subtitle: Subtitle) -> crate::Result<Subtitle> {
        let _default_style = crate::core::formats::ass::AssStyle {
            name: "Default".to_string(),
            font_name: "Arial".to_string(),
            font_size: 16,
            primary_color: crate::core::formats::ass::Color::white(),
            secondary_color: crate::core::formats::ass::Color::red(),
            outline_color: crate::core::formats::ass::Color::black(),
            shadow_color: crate::core::formats::ass::Color::black(),
            bold: false,
            italic: false,
            underline: false,
            alignment: 2,
        };
        for entry in &mut subtitle.entries {
            if self.config.preserve_styling {
                entry.styling = Some(self.extract_srt_styling(&entry.text)?);
            }
            entry.text = self.convert_srt_tags_to_ass(&entry.text);
        }
        subtitle.format = SubtitleFormatType::Ass;
        subtitle.metadata.original_format = SubtitleFormatType::Srt;
        Ok(subtitle)
    }

    /// ASS 轉 SRT
    pub(crate) fn ass_to_srt(&self, mut subtitle: Subtitle) -> crate::Result<Subtitle> {
        for entry in &mut subtitle.entries {
            entry.text = self.strip_ass_tags(&entry.text);
            if self.config.preserve_styling {
                entry.text = self.convert_ass_tags_to_srt(&entry.text);
            }
            entry.styling = None;
        }
        subtitle.format = SubtitleFormatType::Srt;
        Ok(subtitle)
    }

    /// SRT 轉 VTT
    pub(crate) fn srt_to_vtt(&self, mut subtitle: Subtitle) -> crate::Result<Subtitle> {
        subtitle.metadata.title = Some("WEBVTT".to_string());
        for entry in &mut subtitle.entries {
            entry.text = self.convert_srt_tags_to_vtt(&entry.text);
        }
        subtitle.format = SubtitleFormatType::Vtt;
        Ok(subtitle)
    }

    /// ASS 轉 VTT
    pub(crate) fn ass_to_vtt(&self, subtitle: Subtitle) -> crate::Result<Subtitle> {
        // 預留實作
        Err(crate::error::SubXError::subtitle_format(
            subtitle.format.to_string(),
            "ASS->VTT 尚未實作",
        ))
    }

    /// VTT 轉 SRT
    pub(crate) fn vtt_to_srt(&self, subtitle: Subtitle) -> crate::Result<Subtitle> {
        // 預留實作
        Err(crate::error::SubXError::subtitle_format(
            subtitle.format.to_string(),
            "VTT->SRT 尚未實作",
        ))
    }

    /// VTT 轉 ASS
    pub(crate) fn vtt_to_ass(&self, subtitle: Subtitle) -> crate::Result<Subtitle> {
        // 預留實作
        Err(crate::error::SubXError::subtitle_format(
            subtitle.format.to_string(),
            "VTT->ASS 尚未實作",
        ))
    }
}
