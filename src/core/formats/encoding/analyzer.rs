use crate::Result;
use crate::core::formats::encoding::charset::Charset;
use std::collections::HashMap;

/// Single-byte and double-byte statistical analyzer
pub struct ByteAnalyzer {
    byte_frequency: HashMap<u8, usize>,
    bigram_frequency: HashMap<(u8, u8), usize>,
    total_bytes: usize,
}

impl ByteAnalyzer {
    /// Creates a new ByteAnalyzer instance.
    ///
    /// Initializes empty frequency maps and resets counters.
    pub fn new() -> Self {
        Self {
            byte_frequency: HashMap::new(),
            bigram_frequency: HashMap::new(),
            total_bytes: 0,
        }
    }

    /// Analyzes the given byte data and returns encoding analysis results.
    ///
    /// # Arguments
    ///
    /// * `data` - The byte data to analyze for encoding detection
    ///
    /// # Returns
    ///
    /// Returns an `AnalysisResult` containing statistical information about
    /// the data that can be used for encoding detection.
    ///
    /// # Errors
    ///
    /// Returns an error if the analysis cannot be completed due to
    /// insufficient data or computational issues.
    pub fn analyze(&mut self, data: &[u8]) -> Result<AnalysisResult> {
        self.collect_statistics(data);
        self.calculate_metrics()
    }

    fn collect_statistics(&mut self, data: &[u8]) {
        self.total_bytes = data.len();
        for &b in data {
            *self.byte_frequency.entry(b).or_insert(0) += 1;
        }
        for window in data.windows(2) {
            if let [b1, b2] = window {
                *self.bigram_frequency.entry((*b1, *b2)).or_insert(0) += 1;
            }
        }
    }

    fn calculate_metrics(&self) -> Result<AnalysisResult> {
        let ascii_ratio = self.calculate_ascii_ratio();
        let entropy = self.calculate_entropy();
        let control_char_ratio = self.calculate_control_char_ratio();
        Ok(AnalysisResult {
            ascii_ratio,
            entropy,
            control_char_ratio,
            byte_distribution: self.byte_frequency.clone(),
            likely_encodings: self.suggest_encodings(ascii_ratio, entropy, control_char_ratio),
        })
    }

    fn calculate_ascii_ratio(&self) -> f32 {
        let ascii = self
            .byte_frequency
            .iter()
            .filter(|&(&b, _)| b < 0x80)
            .map(|(_, &c)| c)
            .sum::<usize>();
        if self.total_bytes > 0 {
            ascii as f32 / self.total_bytes as f32
        } else {
            0.0
        }
    }

    fn calculate_entropy(&self) -> f32 {
        let mut entropy = 0.0;
        for &count in self.byte_frequency.values() {
            if count > 0 {
                let p = count as f32 / self.total_bytes as f32;
                entropy -= p * p.log2();
            }
        }
        entropy
    }

    fn calculate_control_char_ratio(&self) -> f32 {
        let control = self
            .byte_frequency
            .iter()
            .filter(|&(&b, _)| b < 0x20 && b != 0x09 && b != 0x0A && b != 0x0D)
            .map(|(_, &c)| c)
            .sum::<usize>();
        if self.total_bytes > 0 {
            control as f32 / self.total_bytes as f32
        } else {
            0.0
        }
    }

    fn suggest_encodings(
        &self,
        ascii_ratio: f32,
        entropy: f32,
        control_ratio: f32,
    ) -> Vec<Charset> {
        let mut suggestions = Vec::new();
        if ascii_ratio > 0.9 {
            suggestions.push(Charset::Utf8);
        }
        if entropy > 6.0 && ascii_ratio < 0.8 {
            suggestions.extend_from_slice(&[Charset::Gbk, Charset::Big5, Charset::ShiftJis]);
        }
        if control_ratio > 0.01 {
            suggestions.push(Charset::Windows1252);
        }
        if suggestions.is_empty() {
            suggestions.push(Charset::Utf8);
        }
        suggestions
    }
}

/// Statistical analysis result for encoding detection.
///
/// Contains various metrics computed from byte data analysis that help
/// determine the most likely character encoding for text data.
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Ratio of ASCII characters (0-127) in the data
    pub ascii_ratio: f32,
    /// Shannon entropy of the byte distribution
    pub entropy: f32,
    /// Ratio of control characters in the data
    pub control_char_ratio: f32,
    /// Frequency distribution of all bytes
    pub byte_distribution: HashMap<u8, usize>,
    /// List of encodings ordered by likelihood
    pub likely_encodings: Vec<Charset>,
}

/// Statistical language model-based analyzer for encoding detection.
///
/// Uses statistical models and language patterns to improve encoding
/// detection accuracy beyond simple byte frequency analysis.
pub struct StatisticalAnalyzer {
    language_models: HashMap<Charset, LanguageModel>,
}

impl StatisticalAnalyzer {
    /// Creates a new StatisticalAnalyzer with pre-built language models.
    ///
    /// Initializes language models for various character encodings to
    /// enable statistical analysis of text patterns.
    pub fn new() -> Self {
        Self {
            language_models: Self::build_language_models(),
        }
    }

    fn build_language_models() -> HashMap<Charset, LanguageModel> {
        let mut models = HashMap::new();
        models.insert(
            Charset::Utf8,
            LanguageModel {
                charset: Charset::Utf8,
                common_patterns: vec![
                    (0xC2, 0.05),
                    (0xC3, 0.08),
                    (0xE2, 0.12),
                    (0xE3, 0.15),
                    (0xE4, 0.18),
                    (0xE5, 0.20),
                ],
                invalid_patterns: vec![(0x80, 0.0), (0xBF, 0.0)],
            },
        );
        models.insert(
            Charset::Gbk,
            LanguageModel {
                charset: Charset::Gbk,
                common_patterns: vec![
                    (0xB0, 0.15),
                    (0xC4, 0.12),
                    (0xD6, 0.10),
                    (0xB8, 0.08),
                    (0xBF, 0.06),
                    (0xCE, 0.05),
                ],
                invalid_patterns: vec![(0x7F, 0.0)],
            },
        );
        models
    }

    /// Analyzes byte data using language models to determine encoding likelihood.
    ///
    /// # Arguments
    ///
    /// * `data` - The byte data to analyze
    ///
    /// # Returns
    ///
    /// Returns a HashMap mapping each charset to its likelihood score.
    /// Higher scores indicate higher likelihood that the data is encoded
    /// in that character set.
    ///
    /// # Errors
    ///
    /// Returns an error if the model scoring calculation fails.
    pub fn analyze_with_models(&self, data: &[u8]) -> Result<HashMap<Charset, f32>> {
        let mut scores = HashMap::new();
        for (cs, model) in &self.language_models {
            let score = self.calculate_model_score(data, model)?;
            scores.insert(cs.clone(), score);
        }
        Ok(scores)
    }

    fn calculate_model_score(&self, data: &[u8], model: &LanguageModel) -> Result<f32> {
        let mut score = 0.0;
        for &b in data {
            for &(pb, w) in &model.common_patterns {
                if b == pb {
                    score += w;
                }
            }
            for &(ib, _) in &model.invalid_patterns {
                if b == ib {
                    score -= 0.1;
                }
            }
        }
        Ok(if !data.is_empty() {
            score / data.len() as f32
        } else {
            0.0
        })
    }
}

/// Language model structure
#[derive(Debug, Clone)]
struct LanguageModel {
    charset: Charset,
    common_patterns: Vec<(u8, f32)>,
    invalid_patterns: Vec<(u8, f32)>,
}

impl Default for ByteAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for StatisticalAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test byte analyzer basic functionality
    #[test]
    fn test_byte_analyzer_basic_analysis() {
        let mut analyzer = ByteAnalyzer::new();
        let test_data = b"Hello, World! 123";

        let result = analyzer.analyze(test_data).unwrap();

        // Verify ASCII ratio
        assert!(result.ascii_ratio > 0.9);
        assert!(result.ascii_ratio <= 1.0);

        // Verify entropy within reasonable range
        assert!(result.entropy > 0.0);
        assert!(result.entropy < 8.0);

        // Verify control character ratio
        assert!(result.control_char_ratio < 0.1);

        // Verify encoding suggestions
        assert!(result.likely_encodings.contains(&Charset::Utf8));
    }

    /// Test Chinese text encoding analysis
    #[test]
    fn test_chinese_text_analysis() {
        let mut analyzer = ByteAnalyzer::new();
        let chinese_text = "你好，世界！測試中文編碼檢測。".as_bytes();

        let result = analyzer.analyze(chinese_text).unwrap();

        // Chinese text should have lower ASCII ratio
        assert!(result.ascii_ratio < 0.5);

        // Entropy should be greater than zero
        assert!(result.entropy > 0.0);

        // Should suggest UTF-8 or other Chinese encodings
        let has_unicode_encoding = result
            .likely_encodings
            .iter()
            .any(|charset| matches!(charset, Charset::Utf8 | Charset::Gbk | Charset::Big5));
        assert!(has_unicode_encoding);
    }

    /// Test binary data analysis
    #[test]
    fn test_binary_data_analysis() {
        let mut analyzer = ByteAnalyzer::new();
        let binary_data: Vec<u8> = (0..=255).cycle().take(1000).collect();

        let result = analyzer.analyze(&binary_data).unwrap();

        // Binary data should have high entropy
        assert!(result.entropy > 7.0);

        // ASCII ratio should be approximately 50%
        assert!(result.ascii_ratio > 0.4);
        assert!(result.ascii_ratio < 0.6);
    }

    /// Test entropy calculation accuracy
    #[test]
    fn test_entropy_calculation_accuracy() {
        let mut analyzer = ByteAnalyzer::new();

        // Completely uniform distribution should have maximum entropy
        let uniform_data: Vec<u8> = (0..=255).collect();
        let uniform_result = analyzer.analyze(&uniform_data).unwrap();

        // Reset analyzer
        analyzer = ByteAnalyzer::new();

        // Single character should have minimal entropy value
        let single_char_data = vec![b'A'; 100];
        let single_result = analyzer.analyze(&single_char_data).unwrap();

        assert!(uniform_result.entropy > single_result.entropy);
        assert!(single_result.entropy < 1.0);
    }

    /// Test control character detection
    #[test]
    fn test_control_character_detection() {
        let mut analyzer = ByteAnalyzer::new();

        // Create data containing control characters
        let mut data_with_control = Vec::new();
        data_with_control.extend_from_slice(b"Normal text ");
        data_with_control.push(0x01); // SOH
        data_with_control.push(0x02); // STX
        data_with_control.push(0x1F); // US
        data_with_control.extend_from_slice(b" more text");

        let result = analyzer.analyze(&data_with_control).unwrap();

        // Should detect control characters
        assert!(result.control_char_ratio > 0.0);
        assert!(result.control_char_ratio < 0.5);

        // May suggest Windows-1252 encoding
        assert!(result.likely_encodings.contains(&Charset::Windows1252));
    }

    /// Test statistical analyzer language models
    #[test]
    fn test_statistical_analyzer_language_models() {
        let analyzer = StatisticalAnalyzer::new();

        // Test UTF-8 Chinese text
        let utf8_chinese = "这是一个测试文本。".as_bytes();
        let utf8_scores = analyzer.analyze_with_models(utf8_chinese).unwrap();

        // UTF-8 should be detected as candidate encoding
        assert!(utf8_scores.contains_key(&Charset::Utf8));

        // Test GBK pattern text
        let gbk_pattern = vec![0xB0, 0xA1, 0xC4, 0xE3, 0xBA, 0xC3]; // Simulate GBK encoding
        let gbk_scores = analyzer.analyze_with_models(&gbk_pattern).unwrap();

        // GBK should have reasonable score
        assert!(gbk_scores.get(&Charset::Gbk).unwrap_or(&0.0) > &0.0);
    }

    /// Test byte frequency distribution analysis
    #[test]
    fn test_byte_frequency_distribution() {
        let mut analyzer = ByteAnalyzer::new();
        let repeated_data = b"aaabbbccc";

        let result = analyzer.analyze(repeated_data).unwrap();

        // Verify byte distribution is correctly recorded
        assert!(!result.byte_distribution.is_empty());
        assert_eq!(*result.byte_distribution.get(&b'a').unwrap(), 3);
        assert_eq!(*result.byte_distribution.get(&b'b').unwrap(), 3);
        assert_eq!(*result.byte_distribution.get(&b'c').unwrap(), 3);
    }

    /// Test empty data handling
    #[test]
    fn test_empty_data_handling() {
        let mut analyzer = ByteAnalyzer::new();
        let empty_data = b"";

        let result = analyzer.analyze(empty_data).unwrap();

        // Empty data should return default values
        assert_eq!(result.ascii_ratio, 0.0);
        assert_eq!(result.entropy, 0.0);
        assert_eq!(result.control_char_ratio, 0.0);
        assert!(!result.likely_encodings.is_empty());
    }

    /// Test encoding suggestion logic
    #[test]
    fn test_encoding_suggestion_logic() {
        let mut analyzer = ByteAnalyzer::new();

        // High ASCII ratio should suggest UTF-8
        let ascii_heavy = b"Hello World! 123 ABC";
        let ascii_result = analyzer.analyze(ascii_heavy).unwrap();
        assert!(ascii_result.likely_encodings.contains(&Charset::Utf8));

        // Reset analyzer
        analyzer = ByteAnalyzer::new();

        // High entropy and low ASCII ratio should suggest multibyte encodings
        let multibyte_pattern: Vec<u8> = (0x80..=0xFF).cycle().take(100).collect();
        let multibyte_result = analyzer.analyze(&multibyte_pattern).unwrap();

        let has_multibyte_encoding = multibyte_result
            .likely_encodings
            .iter()
            .any(|charset| matches!(charset, Charset::Gbk | Charset::Big5 | Charset::ShiftJis));
        assert!(has_multibyte_encoding);
    }

    /// Test bigram pattern analysis
    #[test]
    fn test_bigram_pattern_analysis() {
        let mut analyzer = ByteAnalyzer::new();

        // Create data with obvious bigram patterns
        let pattern_data = b"abcabcabcabc";
        let _result = analyzer.analyze(pattern_data).unwrap();

        // Note: Current implementation collects bigram frequencies but doesn't use them in results
        // This can be extended to verify bigram analysis logic
    }
}
