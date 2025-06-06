//! Core subtitle formats 模組
#![allow(dead_code)]

pub mod ass;
pub mod converter;
pub mod encoding;
pub mod manager;
pub mod srt;
pub mod styling;
pub mod sub;
pub mod transformers;
pub mod vtt;

use std::time::Duration;

/// 支援的字幕格式類型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtitleFormatType {
    Srt,
    Ass,
    Vtt,
    Sub,
}

impl SubtitleFormatType {
    /// 取得格式對應字串
    pub fn as_str(&self) -> &'static str {
        match self {
            SubtitleFormatType::Srt => "srt",
            SubtitleFormatType::Ass => "ass",
            SubtitleFormatType::Vtt => "vtt",
            SubtitleFormatType::Sub => "sub",
        }
    }
}

impl std::fmt::Display for SubtitleFormatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 統一字幕資料結構
#[derive(Debug, Clone)]
pub struct Subtitle {
    pub entries: Vec<SubtitleEntry>,
    pub metadata: SubtitleMetadata,
    pub format: SubtitleFormatType,
}

/// 單條字幕項目
#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    pub index: usize,
    pub start_time: Duration,
    pub end_time: Duration,
    pub text: String,
    pub styling: Option<StylingInfo>,
}

/// 字幕元資料
#[derive(Debug, Clone)]
pub struct SubtitleMetadata {
    pub title: Option<String>,
    pub language: Option<String>,
    pub encoding: String,
    pub frame_rate: Option<f32>,
    pub original_format: SubtitleFormatType,
}

/// 樣式資訊
#[derive(Debug, Clone, Default)]
pub struct StylingInfo {
    pub font_name: Option<String>,
    pub font_size: Option<u32>,
    pub color: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

/// 字幕格式 Trait 定義
pub trait SubtitleFormat {
    /// 解析字幕內容
    fn parse(&self, content: &str) -> crate::Result<Subtitle>;

    /// 序列化為字幕格式
    fn serialize(&self, subtitle: &Subtitle) -> crate::Result<String>;

    /// 檢測是否為此格式
    fn detect(&self, content: &str) -> bool;

    /// 格式名稱
    fn format_name(&self) -> &'static str;

    /// 支援的副檔名
    fn file_extensions(&self) -> &'static [&'static str];
}
