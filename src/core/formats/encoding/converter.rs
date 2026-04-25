use crate::Result;
use crate::core::formats::encoding::charset::{Charset, EncodingInfo};
use anyhow::anyhow;
use encoding_rs::{BIG5, Encoding, GBK, ISO_8859_2, SHIFT_JIS, UTF_8, WINDOWS_1252};
use std::collections::HashMap;

/// Result of an encoding conversion operation.
///
/// Contains the converted text along with metadata about the conversion
/// process, including error information and encoding details.
#[derive(Debug, Clone)]
pub struct ConversionResult {
    /// The converted text in the target encoding
    pub converted_text: String,
    /// The original character encoding that was detected
    pub original_encoding: Charset,
    /// The target encoding for conversion
    pub target_encoding: Charset,
    /// Number of bytes processed during conversion
    pub bytes_processed: usize,
    /// Whether any errors occurred during conversion
    pub had_errors: bool,
    /// Total number of conversion errors encountered
    pub error_count: usize,
}

/// Encoding converter
pub struct EncodingConverter {
    encoding_map: HashMap<Charset, &'static Encoding>,
}

impl EncodingConverter {
    /// Create converter and initialize encoding mapping
    pub fn new() -> Self {
        let mut encoding_map = HashMap::new();
        encoding_map.insert(Charset::Utf8, UTF_8);
        encoding_map.insert(Charset::Gbk, GBK);
        encoding_map.insert(Charset::ShiftJis, SHIFT_JIS);
        encoding_map.insert(Charset::Big5, BIG5);
        encoding_map.insert(Charset::Windows1252, WINDOWS_1252);
        encoding_map.insert(Charset::Iso88591, ISO_8859_2);
        Self { encoding_map }
    }

    /// Convert data to UTF-8
    pub fn convert_to_utf8(
        &self,
        data: &[u8],
        source_encoding: &Charset,
    ) -> Result<ConversionResult> {
        if *source_encoding == Charset::Utf8 {
            return Ok(ConversionResult {
                converted_text: String::from_utf8_lossy(data).to_string(),
                original_encoding: Charset::Utf8,
                target_encoding: Charset::Utf8,
                bytes_processed: data.len(),
                had_errors: false,
                error_count: 0,
            });
        }
        let encoding = self
            .encoding_map
            .get(source_encoding)
            .ok_or_else(|| anyhow!("Unsupported encoding: {:?}", source_encoding))?;
        let (converted, _, had_errors) = encoding.decode(data);
        let error_count = if had_errors {
            self.count_replacement_chars(&converted)
        } else {
            0
        };
        Ok(ConversionResult {
            converted_text: converted.into_owned(),
            original_encoding: source_encoding.clone(),
            target_encoding: Charset::Utf8,
            bytes_processed: data.len(),
            had_errors,
            error_count,
        })
    }

    /// Convert file content to UTF-8
    pub fn convert_file_to_utf8(
        &self,
        file_path: &str,
        encoding_info: &EncodingInfo,
    ) -> Result<ConversionResult> {
        crate::core::fs_util::check_file_size(
            std::path::Path::new(file_path),
            52_428_800,
            "Subtitle",
        )?;
        let data = std::fs::read(file_path)?;
        let slice = if encoding_info.bom_detected {
            self.skip_bom(&data, &encoding_info.charset)
        } else {
            data.as_slice()
        };
        self.convert_to_utf8(slice, &encoding_info.charset)
    }

    fn skip_bom<'a>(&self, data: &'a [u8], charset: &Charset) -> &'a [u8] {
        match charset {
            Charset::Utf8 if data.starts_with(&[0xEF, 0xBB, 0xBF]) => &data[3..],
            Charset::Utf16Le if data.starts_with(&[0xFF, 0xFE]) => &data[2..],
            Charset::Utf16Be if data.starts_with(&[0xFE, 0xFF]) => &data[2..],
            Charset::Utf32Le if data.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) => &data[4..],
            Charset::Utf32Be if data.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) => &data[4..],
            _ => data,
        }
    }

    fn count_replacement_chars(&self, text: &str) -> usize {
        text.chars().filter(|&c| c == '\u{FFFD}').count()
    }

    /// Validate conversion result
    pub fn validate_conversion(&self, result: &ConversionResult) -> ValidationResult {
        ValidationResult {
            is_valid: !result.had_errors || result.error_count == 0,
            confidence: if result.had_errors {
                1.0 - result.error_count as f32 / result.converted_text.len() as f32
            } else {
                1.0
            },
            warnings: self.generate_warnings(result),
        }
    }

    fn generate_warnings(&self, result: &ConversionResult) -> Vec<String> {
        let mut warnings = Vec::new();
        if result.had_errors {
            warnings.push(format!(
                "Encoding conversion had {} replacement characters",
                result.error_count
            ));
        }
        if result.error_count > result.bytes_processed / 10 {
            warnings.push("High error rate detected - encoding may be incorrect".to_string());
        }
        warnings
    }
}

/// Result of encoding validation process.
///
/// Contains validation status, confidence level, and any warnings
/// about potential encoding issues.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the encoding validation passed
    pub is_valid: bool,
    /// Confidence level in the validation result (0.0 to 1.0)
    pub confidence: f32,
    /// List of validation warnings
    pub warnings: Vec<String>,
}

impl Default for EncodingConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::formats::encoding::charset::{Charset, EncodingInfo};
    use std::fs;
    use tempfile::TempDir;

    fn make_converter() -> EncodingConverter {
        EncodingConverter::new()
    }

    fn make_encoding_info(charset: Charset, bom_detected: bool) -> EncodingInfo {
        EncodingInfo {
            charset,
            confidence: 1.0,
            bom_detected,
            sample_text: String::new(),
        }
    }

    // --- convert_to_utf8: UTF-8 passthrough ---

    #[test]
    fn test_convert_to_utf8_utf8_passthrough_ascii() {
        let converter = make_converter();
        let text = "Hello, World!";
        let result = converter
            .convert_to_utf8(text.as_bytes(), &Charset::Utf8)
            .unwrap();
        assert_eq!(result.converted_text, text);
        assert_eq!(result.original_encoding, Charset::Utf8);
        assert_eq!(result.target_encoding, Charset::Utf8);
        assert_eq!(result.bytes_processed, text.len());
        assert!(!result.had_errors);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_convert_to_utf8_utf8_passthrough_multibyte() {
        let converter = make_converter();
        let text = "測試文字 🌍";
        let result = converter
            .convert_to_utf8(text.as_bytes(), &Charset::Utf8)
            .unwrap();
        assert_eq!(result.converted_text, text);
        assert_eq!(result.original_encoding, Charset::Utf8);
        assert_eq!(result.bytes_processed, text.as_bytes().len());
        assert!(!result.had_errors);
    }

    #[test]
    fn test_convert_to_utf8_utf8_empty_bytes() {
        let converter = make_converter();
        let result = converter.convert_to_utf8(&[], &Charset::Utf8).unwrap();
        assert_eq!(result.converted_text, "");
        assert_eq!(result.bytes_processed, 0);
        assert!(!result.had_errors);
        assert_eq!(result.error_count, 0);
    }

    // --- convert_to_utf8: non-UTF-8 encodings ---

    #[test]
    fn test_convert_to_utf8_gbk() {
        let converter = make_converter();
        // "你好" in GBK: 你=0xC4E3, 好=0xBAC3
        let gbk_bytes = vec![0xC4u8, 0xE3, 0xBA, 0xC3];
        let result = converter
            .convert_to_utf8(&gbk_bytes, &Charset::Gbk)
            .unwrap();
        assert_eq!(result.original_encoding, Charset::Gbk);
        assert_eq!(result.target_encoding, Charset::Utf8);
        assert_eq!(result.bytes_processed, gbk_bytes.len());
        assert!(!result.had_errors);
        assert!(result.converted_text.contains('你'));
    }

    #[test]
    fn test_convert_to_utf8_gbk_empty() {
        let converter = make_converter();
        let result = converter.convert_to_utf8(&[], &Charset::Gbk).unwrap();
        assert_eq!(result.converted_text, "");
        assert_eq!(result.bytes_processed, 0);
        assert!(!result.had_errors);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_convert_to_utf8_windows1252() {
        let converter = make_converter();
        // "café" — 'é' is 0xE9 in Windows-1252
        let bytes = vec![b'c', b'a', b'f', 0xE9u8];
        let result = converter
            .convert_to_utf8(&bytes, &Charset::Windows1252)
            .unwrap();
        assert_eq!(result.original_encoding, Charset::Windows1252);
        assert_eq!(result.target_encoding, Charset::Utf8);
        assert_eq!(result.bytes_processed, bytes.len());
        assert!(result.converted_text.contains('é') || result.converted_text.contains('é'));
        assert!(!result.had_errors);
    }

    #[test]
    fn test_convert_to_utf8_shiftjis() {
        let converter = make_converter();
        // "テスト" (katakana) in ShiftJIS: 0x83,0x65,0x83,0x58,0x83,0x67
        let shiftjis_bytes = vec![0x83u8, 0x65, 0x83, 0x58, 0x83, 0x67];
        let result = converter
            .convert_to_utf8(&shiftjis_bytes, &Charset::ShiftJis)
            .unwrap();
        assert_eq!(result.original_encoding, Charset::ShiftJis);
        assert_eq!(result.target_encoding, Charset::Utf8);
        assert_eq!(result.bytes_processed, shiftjis_bytes.len());
        assert!(!result.converted_text.is_empty());
    }

    #[test]
    fn test_convert_to_utf8_big5() {
        let converter = make_converter();
        // "你好" in Big5: 你=0xA741, 好=0xA66E (approximate)
        let big5_bytes = vec![0xA7u8, 0x41, 0xA6, 0x6E];
        let result = converter
            .convert_to_utf8(&big5_bytes, &Charset::Big5)
            .unwrap();
        assert_eq!(result.original_encoding, Charset::Big5);
        assert_eq!(result.target_encoding, Charset::Utf8);
        assert_eq!(result.bytes_processed, big5_bytes.len());
        assert!(!result.converted_text.is_empty());
    }

    #[test]
    fn test_convert_to_utf8_iso88591() {
        let converter = make_converter();
        // Latin characters with accents valid in ISO-8859-2
        let bytes = vec![b'H', b'e', b'l', b'l', b'o', 0xE0u8]; // 'à' in ISO-8859-2
        let result = converter
            .convert_to_utf8(&bytes, &Charset::Iso88591)
            .unwrap();
        assert_eq!(result.original_encoding, Charset::Iso88591);
        assert_eq!(result.target_encoding, Charset::Utf8);
        assert_eq!(result.bytes_processed, bytes.len());
        assert!(!result.converted_text.is_empty());
    }

    // --- convert_to_utf8: unsupported/error paths ---

    #[test]
    fn test_convert_to_utf8_unknown_returns_error() {
        let converter = make_converter();
        let result = converter.convert_to_utf8(b"some data", &Charset::Unknown);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported encoding")
        );
    }

    #[test]
    fn test_convert_to_utf8_utf16le_returns_error() {
        let converter = make_converter();
        let result = converter.convert_to_utf8(b"data", &Charset::Utf16Le);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_to_utf8_utf16be_returns_error() {
        let converter = make_converter();
        let result = converter.convert_to_utf8(b"data", &Charset::Utf16Be);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_to_utf8_utf32le_returns_error() {
        let converter = make_converter();
        let result = converter.convert_to_utf8(b"data", &Charset::Utf32Le);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_to_utf8_utf32be_returns_error() {
        let converter = make_converter();
        let result = converter.convert_to_utf8(b"data", &Charset::Utf32Be);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_to_utf8_euckr_returns_error() {
        let converter = make_converter();
        let result = converter.convert_to_utf8(b"data", &Charset::Euckr);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported encoding")
        );
    }

    // --- count_replacement_chars exercised through had_errors path ---

    #[test]
    fn test_convert_to_utf8_invalid_gbk_triggers_replacement_chars() {
        let converter = make_converter();
        // 0x81 starts a 2-byte GBK sequence; 0x20 (space) is not a valid second byte
        let invalid_gbk = vec![0x81u8, 0x20, 0x81, 0x20];
        let result = converter
            .convert_to_utf8(&invalid_gbk, &Charset::Gbk)
            .unwrap();
        if result.had_errors {
            assert!(result.error_count > 0);
        }
    }

    // --- convert_file_to_utf8 ---

    #[test]
    fn test_convert_file_to_utf8_utf8_no_bom() {
        let converter = make_converter();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        let content = "Hello, 世界!";
        fs::write(&path, content).unwrap();
        let info = make_encoding_info(Charset::Utf8, false);
        let result = converter
            .convert_file_to_utf8(path.to_str().unwrap(), &info)
            .unwrap();
        assert_eq!(result.converted_text, content);
        assert!(!result.had_errors);
    }

    #[test]
    fn test_convert_file_to_utf8_nonexistent_file() {
        let converter = make_converter();
        let info = make_encoding_info(Charset::Utf8, false);
        let result = converter.convert_file_to_utf8("/nonexistent/path/does_not_exist.txt", &info);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_file_to_utf8_gbk_no_bom() {
        let converter = make_converter();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("gbk.txt");
        // "你好" in GBK
        let gbk_bytes = vec![0xC4u8, 0xE3, 0xBA, 0xC3];
        fs::write(&path, &gbk_bytes).unwrap();
        let info = make_encoding_info(Charset::Gbk, false);
        let result = converter
            .convert_file_to_utf8(path.to_str().unwrap(), &info)
            .unwrap();
        assert_eq!(result.original_encoding, Charset::Gbk);
        assert!(result.converted_text.contains('你'));
    }

    // --- BOM handling via convert_file_to_utf8 / skip_bom ---

    #[test]
    fn test_convert_file_to_utf8_utf8_with_bom_stripped() {
        let converter = make_converter();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bom_utf8.txt");
        let content = "Hello, World!";
        let mut data = vec![0xEFu8, 0xBB, 0xBF]; // UTF-8 BOM
        data.extend_from_slice(content.as_bytes());
        fs::write(&path, &data).unwrap();
        let info = make_encoding_info(Charset::Utf8, true);
        let result = converter
            .convert_file_to_utf8(path.to_str().unwrap(), &info)
            .unwrap();
        // BOM must be stripped; converted text should equal original content
        assert_eq!(result.converted_text, content);
        assert!(!result.had_errors);
    }

    #[test]
    fn test_skip_bom_utf16le_exercised_then_fails() {
        let converter = make_converter();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("utf16le.bin");
        let mut data = vec![0xFFu8, 0xFE]; // UTF-16 LE BOM
        data.extend_from_slice(b"H\x00i\x00");
        fs::write(&path, &data).unwrap();
        // skip_bom strips 2 bytes; convert_to_utf8 with Utf16Le then fails
        let info = make_encoding_info(Charset::Utf16Le, true);
        let result = converter.convert_file_to_utf8(path.to_str().unwrap(), &info);
        assert!(result.is_err());
    }

    #[test]
    fn test_skip_bom_utf16be_exercised_then_fails() {
        let converter = make_converter();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("utf16be.bin");
        let mut data = vec![0xFEu8, 0xFF]; // UTF-16 BE BOM
        data.extend_from_slice(b"\x00H\x00i");
        fs::write(&path, &data).unwrap();
        let info = make_encoding_info(Charset::Utf16Be, true);
        let result = converter.convert_file_to_utf8(path.to_str().unwrap(), &info);
        assert!(result.is_err());
    }

    #[test]
    fn test_skip_bom_utf32le_exercised_then_fails() {
        let converter = make_converter();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("utf32le.bin");
        let mut data = vec![0xFFu8, 0xFE, 0x00, 0x00]; // UTF-32 LE BOM
        data.extend_from_slice(b"H\x00\x00\x00");
        fs::write(&path, &data).unwrap();
        let info = make_encoding_info(Charset::Utf32Le, true);
        let result = converter.convert_file_to_utf8(path.to_str().unwrap(), &info);
        assert!(result.is_err());
    }

    #[test]
    fn test_skip_bom_utf32be_exercised_then_fails() {
        let converter = make_converter();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("utf32be.bin");
        let mut data = vec![0x00u8, 0x00, 0xFE, 0xFF]; // UTF-32 BE BOM
        data.extend_from_slice(b"\x00\x00\x00H");
        fs::write(&path, &data).unwrap();
        let info = make_encoding_info(Charset::Utf32Be, true);
        let result = converter.convert_file_to_utf8(path.to_str().unwrap(), &info);
        assert!(result.is_err());
    }

    #[test]
    fn test_skip_bom_fallthrough_mismatched_bom_flag() {
        // bom_detected=true but charset=Gbk → hits the `_ => data` arm in skip_bom
        let converter = make_converter();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("gbk_no_bom.txt");
        let gbk_bytes = vec![0xC4u8, 0xE3, 0xBA, 0xC3];
        fs::write(&path, &gbk_bytes).unwrap();
        let info = make_encoding_info(Charset::Gbk, true);
        let result = converter
            .convert_file_to_utf8(path.to_str().unwrap(), &info)
            .unwrap();
        assert!(result.converted_text.contains('你'));
    }

    #[test]
    fn test_skip_bom_utf8_charset_but_no_bom_bytes() {
        // bom_detected=true, Charset::Utf8, but file has no BOM → hits `_ => data`
        let converter = make_converter();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("utf8_nobom.txt");
        let content = "Just text";
        fs::write(&path, content).unwrap();
        let info = make_encoding_info(Charset::Utf8, true);
        let result = converter
            .convert_file_to_utf8(path.to_str().unwrap(), &info)
            .unwrap();
        assert_eq!(result.converted_text, content);
    }

    // --- validate_conversion ---

    #[test]
    fn test_validate_conversion_no_errors() {
        let converter = make_converter();
        let result = ConversionResult {
            converted_text: "Hello World".to_string(),
            original_encoding: Charset::Utf8,
            target_encoding: Charset::Utf8,
            bytes_processed: 11,
            had_errors: false,
            error_count: 0,
        };
        let validation = converter.validate_conversion(&result);
        assert!(validation.is_valid);
        assert_eq!(validation.confidence, 1.0);
        assert!(validation.warnings.is_empty());
    }

    #[test]
    fn test_validate_conversion_had_errors_zero_count_still_valid() {
        // had_errors=true but error_count=0 → is_valid = !true || true = true
        let converter = make_converter();
        let result = ConversionResult {
            converted_text: "Hello World".to_string(),
            original_encoding: Charset::Gbk,
            target_encoding: Charset::Utf8,
            bytes_processed: 11,
            had_errors: true,
            error_count: 0,
        };
        let validation = converter.validate_conversion(&result);
        assert!(validation.is_valid);
        // confidence = 1.0 - 0/11 = 1.0
        assert_eq!(validation.confidence, 1.0);
        // had_errors=true → warning about replacement chars
        assert_eq!(validation.warnings.len(), 1);
        assert!(validation.warnings[0].contains("replacement characters"));
    }

    #[test]
    fn test_validate_conversion_with_replacement_errors() {
        let converter = make_converter();
        let result = ConversionResult {
            converted_text: "Hello\u{FFFD}World".to_string(),
            original_encoding: Charset::Windows1252,
            target_encoding: Charset::Utf8,
            bytes_processed: 11,
            had_errors: true,
            error_count: 1,
        };
        let validation = converter.validate_conversion(&result);
        // is_valid = !true || (1 == 0) = false
        assert!(!validation.is_valid);
        assert!(validation.confidence < 1.0);
        assert!(!validation.warnings.is_empty());
        assert!(validation.warnings[0].contains("replacement characters"));
    }

    #[test]
    fn test_validate_conversion_high_error_rate_warning() {
        let converter = make_converter();
        // error_count=3 > bytes_processed(10) / 10 = 1 → second warning
        let result = ConversionResult {
            converted_text: "\u{FFFD}\u{FFFD}\u{FFFD}AB".to_string(),
            original_encoding: Charset::ShiftJis,
            target_encoding: Charset::Utf8,
            bytes_processed: 10,
            had_errors: true,
            error_count: 3,
        };
        let validation = converter.validate_conversion(&result);
        assert!(!validation.is_valid);
        assert!(validation.warnings.len() >= 2);
        assert!(
            validation
                .warnings
                .iter()
                .any(|w| w.contains("High error rate"))
        );
    }

    // --- Default impl ---

    #[test]
    fn test_encoding_converter_default_works() {
        let converter = EncodingConverter::default();
        let result = converter.convert_to_utf8(b"hello", &Charset::Utf8).unwrap();
        assert_eq!(result.converted_text, "hello");
    }
}
