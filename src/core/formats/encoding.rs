use crate::error::SubXError;
use crate::Result;
use encoding_rs::{Encoding, UTF_8};

/// 檢測位元組流編碼，優先嘗試 UTF-8，其次常見亞洲編碼
pub fn detect_encoding(bytes: &[u8]) -> &'static Encoding {
    if UTF_8.decode_without_bom_handling(bytes).1 {
        return UTF_8;
    }
    let encodings = [
        encoding_rs::GBK,
        encoding_rs::BIG5,
        encoding_rs::SHIFT_JIS,
        encoding_rs::EUC_KR,
    ];
    for &enc in &encodings {
        let (_decoded, _enc, had_errors) = enc.decode(bytes);
        if !had_errors {
            return enc;
        }
    }
    UTF_8
}

/// 將非 UTF-8 編碼內容轉換為 UTF-8 字串
pub fn convert_to_utf8(bytes: &[u8]) -> Result<String> {
    let encoding = detect_encoding(bytes);
    let (decoded, _enc, had_errors) = encoding.decode(bytes);
    if had_errors {
        return Err(SubXError::subtitle_format("Unknown", "編碼轉換失敗"));
    }
    Ok(decoded.into_owned())
}
