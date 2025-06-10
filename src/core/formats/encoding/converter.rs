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
