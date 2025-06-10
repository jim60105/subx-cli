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

    /// Create encoding detector with custom configuration
    pub fn with_config(config: &crate::config::Config) -> Self {
        Self {
            confidence_threshold: config.formats.encoding_detection_confidence,
            max_sample_size: 8192,
            supported_charsets: Self::default_charsets(),
        }
    }

    /// Create encoding detector with default settings (for testing)
    pub fn with_defaults() -> Self {
        Self {
            confidence_threshold: 0.7,
            max_sample_size: 8192,
            supported_charsets: Self::default_charsets(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_detector() -> EncodingDetector {
        EncodingDetector {
            confidence_threshold: 0.7,
            max_sample_size: 8192,
            supported_charsets: EncodingDetector::default_charsets(),
        }
    }

    /// Test UTF-8 encoding detection
    #[test]
    fn test_utf8_detection_accuracy() {
        let detector = create_test_detector();
        let utf8_text = "Hello, 世界! Bonjour, monde! 🌍";

        let result = detector.detect_encoding(utf8_text.as_bytes()).unwrap();

        assert_eq!(result.charset, Charset::Utf8);
        assert!(result.confidence > 0.8);
        assert!(!result.bom_detected);
        assert!(result.sample_text.contains("Hello"));
    }

    /// Test UTF-8 BOM detection
    #[test]
    fn test_utf8_bom_detection() {
        let detector = create_test_detector();
        let mut bom_data = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        bom_data.extend_from_slice("Hello, World!".as_bytes());

        let result = detector.detect_encoding(&bom_data).unwrap();

        assert_eq!(result.charset, Charset::Utf8);
        assert_eq!(result.confidence, 1.0);
        assert!(result.bom_detected);
        assert_eq!(result.sample_text, "UTF-8 with BOM");
    }

    /// Test UTF-16 BOM detection
    #[test]
    fn test_utf16_bom_detection() {
        let detector = create_test_detector();

        // UTF-16 LE BOM
        let utf16le_data = vec![0xFF, 0xFE, 0x48, 0x00, 0x65, 0x00]; // "He" in UTF-16 LE
        let result = detector.detect_encoding(&utf16le_data).unwrap();
        assert_eq!(result.charset, Charset::Utf16Le);
        assert!(result.bom_detected);

        // UTF-16 BE BOM
        let utf16be_data = vec![0xFE, 0xFF, 0x00, 0x48, 0x00, 0x65]; // "He" in UTF-16 BE
        let result = detector.detect_encoding(&utf16be_data).unwrap();
        assert_eq!(result.charset, Charset::Utf16Be);
        assert!(result.bom_detected);
    }

    /// Test file encoding detection
    #[test]
    fn test_file_encoding_detection() {
        let detector = create_test_detector();
        let temp_dir = TempDir::new().unwrap();

        // Create UTF-8 file
        let utf8_path = temp_dir.path().join("utf8.txt");
        fs::write(&utf8_path, "測試檔案編碼檢測功能。").unwrap();

        let result = detector
            .detect_file_encoding(utf8_path.to_str().unwrap())
            .unwrap();

        assert_eq!(result.charset, Charset::Utf8);
        assert!(result.confidence > 0.7);
    }

    /// Test error handling for non-existent files
    #[test]
    fn test_nonexistent_file_error() {
        let detector = create_test_detector();
        let result = detector.detect_file_encoding("nonexistent.txt");

        assert!(result.is_err());
    }

    /// Test GBK encoding pattern detection
    #[test]
    fn test_gbk_pattern_detection() {
        let detector = create_test_detector();

        // Simulate GBK encoding pattern (high byte range)
        let gbk_pattern = vec![
            0xC4, 0xE3, 0xBA, 0xC3, // 你好 (GBK)
            0xCA, 0xC0, 0xBD, 0xE7, // 世界 (GBK)
        ];

        let result = detector.detect_encoding(&gbk_pattern).unwrap();

        // Should detect as GBK or at least not UTF-8
        assert!(result.confidence > 0.3);
        if result.charset == Charset::Gbk {
            assert!(result.confidence > 0.5);
        }
    }

    /// Test Shift-JIS encoding detection
    #[test]
    fn test_shift_jis_detection() {
        let detector = create_test_detector();

        // Simulate Shift-JIS encoding pattern
        let shift_jis_pattern = vec![
            0x82, 0xB1, 0x82, 0xF1, // こん (Shift-JIS)
            0x82, 0xB1, 0x82, 0xF1, // こん (Shift-JIS)
            0x82, 0xC9, 0x82, 0xBF, // にち (Shift-JIS)
        ];

        let result = detector.detect_encoding(&shift_jis_pattern).unwrap();

        // Should detect as Shift-JIS or related encoding
        assert!(result.confidence > 0.2);
    }

    /// Test encoding confidence ranking
    #[test]
    fn test_encoding_confidence_ranking() {
        let detector = create_test_detector();

        // Clear UTF-8 text should have highest confidence
        let clear_utf8 = "Clear English text with numbers 123.";
        let utf8_result = detector.detect_encoding(clear_utf8.as_bytes()).unwrap();

        // Ambiguous data should have lower confidence
        let ambiguous_data: Vec<u8> = (0x80..=0xFF).cycle().take(50).collect();
        let ambiguous_result = detector.detect_encoding(&ambiguous_data).unwrap();

        assert!(utf8_result.confidence > ambiguous_result.confidence);
    }

    /// Test maximum sample size limit
    #[test]
    fn test_max_sample_size_limit() {
        let detector = create_test_detector();

        // Create data exceeding sample size limit
        let large_data = vec![b'A'; 10000]; // Assuming limit is 8192
        let result = detector.detect_encoding(&large_data).unwrap();

        // Should successfully detect without failing due to data size
        assert_eq!(result.charset, Charset::Utf8);
        assert!(result.confidence > 0.9);
    }

    /// Test encoding candidate selection logic
    #[test]
    fn test_encoding_candidate_selection() {
        let detector = create_test_detector();

        // Create data with mixed encoding features
        let mut mixed_data = b"English text ".to_vec();
        mixed_data.extend_from_slice(&[0xC3, 0xA9]); // é in UTF-8
        mixed_data.extend_from_slice(b" and more text");

        let result = detector.detect_encoding(&mixed_data).unwrap();

        // Should correctly choose UTF-8
        assert_eq!(result.charset, Charset::Utf8);
        assert!(result.confidence > 0.7);
    }

    /// Test fallback mechanism for unknown encodings
    #[test]
    fn test_unknown_encoding_fallback() {
        let detector = create_test_detector();

        // Create completely random data
        let random_data: Vec<u8> = (0..100).map(|i| (i * 7 + 13) as u8).collect();
        let result = detector.detect_encoding(&random_data).unwrap();

        // Should have a fallback encoding choice
        assert!(result.confidence >= 0.0);
        assert!(result.confidence <= 1.0);
    }

    /// Test encoding detection performance
    #[test]
    fn test_detection_performance() {
        let detector = create_test_detector();

        // Create medium-sized text file
        let large_text = "Hello, World! ".repeat(500);

        let start = std::time::Instant::now();
        let _result = detector.detect_encoding(large_text.as_bytes()).unwrap();
        let duration = start.elapsed();

        // Detection should complete within reasonable time (< 100ms)
        assert!(duration.as_millis() < 100);
    }
}
