/// 字符集與編碼資訊定義
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Charset {
    Utf8,
    Utf16Le,
    Utf16Be,
    Utf32Le,
    Utf32Be,
    Gbk,
    ShiftJis,
    Iso88591,
    Windows1252,
    Big5,
    Euckr,
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
