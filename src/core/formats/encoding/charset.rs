/// Character encoding types supported by the subtitle processing system.
///
/// This enum covers the most common text encodings encountered in subtitle
/// files across different languages and regions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Charset {
    /// UTF-8 encoding (Unicode)
    Utf8,
    /// UTF-16 Little Endian encoding
    Utf16Le,
    /// UTF-16 Big Endian encoding
    Utf16Be,
    /// UTF-32 Little Endian encoding
    Utf32Le,
    /// UTF-32 Big Endian encoding
    Utf32Be,
    /// GBK encoding (Chinese Simplified)
    Gbk,
    /// Shift JIS encoding (Japanese)
    ShiftJis,
    /// ISO 8859-1 encoding (Latin-1)
    Iso88591,
    /// Windows-1252 encoding (Western European)
    Windows1252,
    /// Big5 encoding (Chinese Traditional)
    Big5,
    /// EUC-KR encoding (Korean)
    Euckr,
    /// Unknown or undetectable encoding
    Unknown,
}

/// 編碼檢測結果資訊
#[derive(Debug, Clone)]
pub struct EncodingInfo {
    /// 偵測到的字符集
    pub charset: Charset,
    /// 檢測信心度 (0.0-1.0)
    pub confidence: f32,
    /// 是否檢測到 BOM
    pub bom_detected: bool,
    /// 解碼後的樣本文字
    pub sample_text: String,
}
