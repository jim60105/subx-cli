use crate::Result;
use crate::core::formats::encoding::charset::Charset;
use std::collections::HashMap;

/// 單字節與雙字節統計分析器
pub struct ByteAnalyzer {
    byte_frequency: HashMap<u8, usize>,
    bigram_frequency: HashMap<(u8, u8), usize>,
    total_bytes: usize,
}

impl ByteAnalyzer {
    pub fn new() -> Self {
        Self {
            byte_frequency: HashMap::new(),
            bigram_frequency: HashMap::new(),
            total_bytes: 0,
        }
    }

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

/// 統計分析結果
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub ascii_ratio: f32,
    pub entropy: f32,
    pub control_char_ratio: f32,
    pub byte_distribution: HashMap<u8, usize>,
    pub likely_encodings: Vec<Charset>,
}

/// 基於語言模型的統計分析器
pub struct StatisticalAnalyzer {
    language_models: HashMap<Charset, LanguageModel>,
}

impl StatisticalAnalyzer {
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

/// 語言模型結構
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
