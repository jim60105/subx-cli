use crate::Result;
use crate::config::load_config;
use crate::core::formats::encoding::charset::{Charset, EncodingInfo};
use std::fs::File;
use std::io::Read;

/// Encoding detection engine
pub struct EncodingDetector {
    confidence_threshold: f32,
    max_sample_size: usize,
    supported_charsets: Vec<Charset>,
}

impl EncodingDetector {
    /// Create encoding detector, read confidence threshold from configuration
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        Ok(Self {
            confidence_threshold: config.formats.encoding_detection_confidence,
            max_sample_size: 8192,
            supported_charsets: Self::default_charsets(),
        })
    }

    /// Detect file encoding
    pub fn detect_file_encoding(&self, file_path: &str) -> Result<EncodingInfo> {
        let mut file = File::open(file_path)?;
        let mut buffer = vec![0; self.max_sample_size];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);
        self.detect_encoding(&buffer)
    }

    /// Detect data encoding
    pub fn detect_encoding(&self, data: &[u8]) -> Result<EncodingInfo> {
        if let Some(encoding) = self.detect_bom(data) {
            return Ok(encoding);
        }
        let candidates = self.analyze_byte_patterns(data)?;
        self.select_best_encoding(candidates, data)
    }

    fn detect_bom(&self, data: &[u8]) -> Option<EncodingInfo> {
        if data.len() < 3 {
            return None;
        }
        match &data[0..3] {
            [0xEF, 0xBB, 0xBF] => Some(EncodingInfo {
                charset: Charset::Utf8,
                confidence: 1.0,
                bom_detected: true,
                sample_text: String::from("UTF-8 with BOM"),
            }),
            [0xFF, 0xFE, ..] => Some(EncodingInfo {
                charset: Charset::Utf16Le,
                confidence: 1.0,
                bom_detected: true,
                sample_text: String::from("UTF-16 LE with BOM"),
            }),
            [0xFE, 0xFF, ..] => Some(EncodingInfo {
                charset: Charset::Utf16Be,
                confidence: 1.0,
                bom_detected: true,
                sample_text: String::from("UTF-16 BE with BOM"),
            }),
            _ => {
                if data.len() >= 4 {
                    match &data[0..4] {
                        [0xFF, 0xFE, 0x00, 0x00] => Some(EncodingInfo {
                            charset: Charset::Utf32Le,
                            confidence: 1.0,
                            bom_detected: true,
                            sample_text: String::from("UTF-32 LE with BOM"),
                        }),
                        [0x00, 0x00, 0xFE, 0xFF] => Some(EncodingInfo {
                            charset: Charset::Utf32Be,
                            confidence: 1.0,
                            bom_detected: true,
                            sample_text: String::from("UTF-32 BE with BOM"),
                        }),
                        _ => None,
                    }
                } else {
                    None
                }
            }
        }
    }

    fn analyze_byte_patterns(&self, data: &[u8]) -> Result<Vec<EncodingCandidate>> {
        let mut candidates = Vec::new();
        for charset in &self.supported_charsets {
            let confidence = self.calculate_encoding_confidence(data, charset)?;
            if confidence > 0.1 {
                candidates.push(EncodingCandidate {
                    charset: charset.clone(),
                    confidence,
                });
            }
        }
        candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        Ok(candidates)
    }

    fn calculate_encoding_confidence(&self, data: &[u8], charset: &Charset) -> Result<f32> {
        match charset {
            Charset::Utf8 => self.check_utf8_validity(data),
            Charset::Gbk => self.check_gbk_patterns(data),
            Charset::ShiftJis => self.check_shift_jis_patterns(data),
            Charset::Big5 => self.check_big5_patterns(data),
            Charset::Iso88591 => self.check_iso88591_patterns(data),
            Charset::Windows1252 => self.check_windows1252_patterns(data),
            _ => Ok(0.0),
        }
    }

    fn check_utf8_validity(&self, data: &[u8]) -> Result<f32> {
        let mut valid_chars = 0;
        let mut total_chars = 0;
        let mut i = 0;

        while i < data.len() {
            total_chars += 1;
            if data[i] & 0x80 == 0 {
                valid_chars += 1;
                i += 1;
            } else if data[i] & 0xE0 == 0xC0 {
                if i + 1 < data.len() && data[i + 1] & 0xC0 == 0x80 {
                    valid_chars += 1;
                }
                i += 2;
            } else if data[i] & 0xF0 == 0xE0 {
                if i + 2 < data.len() && data[i + 1] & 0xC0 == 0x80 && data[i + 2] & 0xC0 == 0x80 {
                    valid_chars += 1;
                }
                i += 3;
            } else if data[i] & 0xF8 == 0xF0 {
                if i + 3 < data.len()
                    && data[i + 1] & 0xC0 == 0x80
                    && data[i + 2] & 0xC0 == 0x80
                    && data[i + 3] & 0xC0 == 0x80
                {
                    valid_chars += 1;
                }
                i += 4;
            } else {
                i += 1;
            }
        }

        Ok(if total_chars > 0 {
            valid_chars as f32 / total_chars as f32
        } else {
            0.0
        })
    }

    fn check_gbk_patterns(&self, data: &[u8]) -> Result<f32> {
        let mut valid_chars = 0;
        let mut total_chars = 0;
        let mut i = 0;

        while i < data.len() {
            if data[i] < 0x80 {
                valid_chars += 1;
                total_chars += 1;
                i += 1;
            } else if i + 1 < data.len() {
                let byte1 = data[i];
                let byte2 = data[i + 1];
                if (0x81..=0xFE).contains(&byte1)
                    && ((0x40..=0x7E).contains(&byte2) || (0x80..=0xFE).contains(&byte2))
                {
                    valid_chars += 1;
                }
                total_chars += 1;
                i += 2;
            } else {
                total_chars += 1;
                i += 1;
            }
        }

        Ok(if total_chars > 0 {
            valid_chars as f32 / total_chars as f32
        } else {
            0.0
        })
    }

    fn check_shift_jis_patterns(&self, data: &[u8]) -> Result<f32> {
        let mut valid_chars = 0;
        let mut total_chars = 0;
        let mut i = 0;

        while i < data.len() {
            if data[i] < 0x80 {
                valid_chars += 1;
                total_chars += 1;
                i += 1;
            } else if i + 1 < data.len() {
                let byte1 = data[i];
                let byte2 = data[i + 1];
                if ((0x81..=0x9F).contains(&byte1) || (0xE0..=0xEF).contains(&byte1))
                    && (0x40..=0xFC).contains(&byte2)
                    && byte2 != 0x7F
                {
                    valid_chars += 1;
                }
                total_chars += 1;
                i += 2;
            } else {
                total_chars += 1;
                i += 1;
            }
        }

        Ok(if total_chars > 0 {
            valid_chars as f32 / total_chars as f32
        } else {
            0.0
        })
    }

    fn check_big5_patterns(&self, data: &[u8]) -> Result<f32> {
        let mut valid_chars = 0;
        let mut total_chars = 0;
        let mut i = 0;

        while i < data.len() {
            if data[i] < 0x80 {
                valid_chars += 1;
                total_chars += 1;
                i += 1;
            } else if i + 1 < data.len() {
                let byte1 = data[i];
                let byte2 = data[i + 1];
                if (0xA1..=0xFE).contains(&byte1)
                    && ((0x40..=0x7E).contains(&byte2) || (0xA1..=0xFE).contains(&byte2))
                {
                    valid_chars += 1;
                }
                total_chars += 1;
                i += 2;
            } else {
                total_chars += 1;
                i += 1;
            }
        }

        Ok(if total_chars > 0 {
            valid_chars as f32 / total_chars as f32
        } else {
            0.0
        })
    }

    fn check_iso88591_patterns(&self, data: &[u8]) -> Result<f32> {
        let _ascii_count = data.iter().filter(|&&b| b < 0x80).count();
        let extended_count = data.iter().filter(|&&b| b >= 0x80).count();
        if extended_count > 0 {
            let utf8_conf = self.check_utf8_validity(data)?;
            Ok(if utf8_conf < 0.5 { 0.7 } else { 0.2 })
        } else {
            Ok(0.5)
        }
    }

    fn check_windows1252_patterns(&self, data: &[u8]) -> Result<f32> {
        let control_chars = data.iter().filter(|&&b| (0x80..=0x9F).contains(&b)).count();
        let extended_chars = data.iter().filter(|&&b| b >= 0xA0).count();
        if control_chars > 0 || extended_chars > 0 {
            let utf8_conf = self.check_utf8_validity(data)?;
            Ok(if utf8_conf < 0.5 { 0.6 } else { 0.1 })
        } else {
            Ok(0.3)
        }
    }

    fn select_best_encoding(
        &self,
        candidates: Vec<EncodingCandidate>,
        data: &[u8],
    ) -> Result<EncodingInfo> {
        if candidates.is_empty() {
            return Ok(EncodingInfo {
                charset: Charset::Unknown,
                confidence: 0.0,
                bom_detected: false,
                sample_text: String::from("Unable to detect encoding"),
            });
        }
        let best = &candidates[0];
        if best.confidence < self.confidence_threshold {
            let config = load_config()?;
            return Ok(EncodingInfo {
                charset: Charset::Utf8,
                confidence: 0.5,
                bom_detected: false,
                sample_text: format!(
                    "Using default encoding: {}",
                    config.formats.default_encoding
                ),
            });
        }
        let sample = self.decode_sample(data, &best.charset)?;
        Ok(EncodingInfo {
            charset: best.charset.clone(),
            confidence: best.confidence,
            bom_detected: false,
            sample_text: sample,
        })
    }

    fn decode_sample(&self, data: &[u8], charset: &Charset) -> Result<String> {
        let sample_size = data.len().min(200);
        let sample_data = &data[0..sample_size];
        match charset {
            Charset::Utf8 => String::from_utf8(sample_data.to_vec())
                .or_else(|_| Ok(String::from_utf8_lossy(sample_data).into_owned())),
            _ => Ok(String::from_utf8_lossy(sample_data).into_owned()),
        }
    }

    fn default_charsets() -> Vec<Charset> {
        vec![
            Charset::Utf8,
            Charset::Gbk,
            Charset::ShiftJis,
            Charset::Big5,
            Charset::Iso88591,
            Charset::Windows1252,
        ]
    }
}

#[derive(Debug, Clone)]
struct EncodingCandidate {
    charset: Charset,
    confidence: f32,
}

impl Default for EncodingDetector {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            confidence_threshold: 0.7,
            max_sample_size: 8192,
            supported_charsets: Self::default_charsets(),
        })
    }
}
