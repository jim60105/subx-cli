//! Language detection module.
//!
//! Provides utilities to detect language codes from file paths and names,
//! using directory names, filename patterns, and file extensions.
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::core::language::LanguageDetector;
//! use std::path::Path;
//!
//! let detector = LanguageDetector::new();
//! let code = detector.get_primary_language(Path::new("subtitle.sc.srt")).unwrap();
//! assert_eq!(code, "sc");
//! ```
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

/// Source of detected language information.
#[derive(Debug, Clone, PartialEq)]
pub enum LanguageSource {
    /// Derived from a parent directory name.
    Directory,
    /// Derived from the file name pattern.
    Filename,
    /// Derived from the file extension or naming convention.
    Extension,
}
impl Default for LanguageDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Detected language information, including code, source, and confidence.
#[derive(Debug, Clone)]
pub struct LanguageInfo {
    /// Standardized language code (e.g., "tc", "sc", "en").
    pub code: String,
    /// Origin of the language detection result.
    pub source: LanguageSource,
    /// Confidence score of the detection (0.0 to 1.0).
    pub confidence: f32,
}

/// Detector for identifying language codes from filesystem paths.
pub struct LanguageDetector {
    language_codes: HashMap<String, String>,
    directory_patterns: Vec<String>,
    filename_patterns: Vec<Regex>,
}

impl LanguageDetector {
    /// Create a new `LanguageDetector` with default language mappings and patterns.
    ///
    /// Initializes internal dictionaries and regex patterns for detection.
    pub fn new() -> Self {
        //! Do not translate these language codes to English rustdoc!!!
        let mut language_codes = HashMap::new();
        // Traditional Chinese
        language_codes.insert("tc".to_string(), "tc".to_string());
        language_codes.insert("繁中".to_string(), "tc".to_string()); // Traditional Chinese (zh-Hant)
        language_codes.insert("繁體".to_string(), "tc".to_string()); // Traditional Chinese (zh-Hant)
        language_codes.insert("cht".to_string(), "tc".to_string());
        // Simplified Chinese
        language_codes.insert("sc".to_string(), "sc".to_string());
        language_codes.insert("简中".to_string(), "sc".to_string()); // Simplified Chinese (zh-Hans)
        language_codes.insert("简体".to_string(), "sc".to_string()); // Simplified Chinese (zh-Hans)
        language_codes.insert("chs".to_string(), "sc".to_string());
        // Hyphenated/underscored regional aliases the AI may emit when
        // asked for a "language" hint (BCP 47 short tags and common
        // English labels). Lower-cased before lookup so these only need
        // to be the canonical lower form.
        language_codes.insert("traditional-chinese".to_string(), "tc".to_string());
        language_codes.insert("traditional_chinese".to_string(), "tc".to_string());
        language_codes.insert("zh-hant".to_string(), "tc".to_string());
        language_codes.insert("zh_hant".to_string(), "tc".to_string());
        language_codes.insert("simplified-chinese".to_string(), "sc".to_string());
        language_codes.insert("simplified_chinese".to_string(), "sc".to_string());
        language_codes.insert("zh-hans".to_string(), "sc".to_string());
        language_codes.insert("zh_hans".to_string(), "sc".to_string());
        // English
        language_codes.insert("en".to_string(), "en".to_string());
        language_codes.insert("英文".to_string(), "en".to_string()); // English
        language_codes.insert("english".to_string(), "en".to_string());
        language_codes.insert("eng".to_string(), "en".to_string());
        // Common ISO 639-2 / ISO 639-3 synonyms for non-Chinese languages.
        language_codes.insert("ja".to_string(), "ja".to_string());
        language_codes.insert("jpn".to_string(), "ja".to_string());
        language_codes.insert("japanese".to_string(), "ja".to_string());
        language_codes.insert("ko".to_string(), "ko".to_string());
        language_codes.insert("kor".to_string(), "ko".to_string());
        language_codes.insert("korean".to_string(), "ko".to_string());
        language_codes.insert("fr".to_string(), "fr".to_string());
        language_codes.insert("fra".to_string(), "fr".to_string());
        language_codes.insert("fre".to_string(), "fr".to_string());
        language_codes.insert("french".to_string(), "fr".to_string());
        language_codes.insert("de".to_string(), "de".to_string());
        language_codes.insert("deu".to_string(), "de".to_string());
        language_codes.insert("ger".to_string(), "de".to_string());
        language_codes.insert("german".to_string(), "de".to_string());
        language_codes.insert("es".to_string(), "es".to_string());
        language_codes.insert("spa".to_string(), "es".to_string());
        language_codes.insert("spanish".to_string(), "es".to_string());
        language_codes.insert("pt".to_string(), "pt".to_string());
        language_codes.insert("por".to_string(), "pt".to_string());
        language_codes.insert("portuguese".to_string(), "pt".to_string());
        language_codes.insert("ru".to_string(), "ru".to_string());
        language_codes.insert("rus".to_string(), "ru".to_string());
        language_codes.insert("russian".to_string(), "ru".to_string());

        let filename_patterns = vec![
            Regex::new(r"\.([a-z]{2,3})\.").unwrap(), // .tc., .sc., .en.
            Regex::new(r"_([a-z]{2,3})\.").unwrap(),  // _tc., _sc., _en.
            Regex::new(r"-([a-z]{2,3})\.").unwrap(),  // -tc., -sc., -en.
        ];

        Self {
            language_codes,
            directory_patterns: vec!["tc".to_string(), "sc".to_string(), "en".to_string()],
            filename_patterns,
        }
    }
    /// Detect a single language information from the given path.
    ///
    /// # Behavior
    ///
    /// Attempts detection by directory name first, then by filename pattern.
    pub fn detect_from_path(&self, path: &Path) -> Option<LanguageInfo> {
        if let Some(lang) = self.detect_from_directory(path) {
            return Some(lang);
        }
        if let Some(lang) = self.detect_from_filename(path) {
            return Some(lang);
        }
        None
    }

    /// Normalize a raw language label (typically supplied by an AI model)
    /// into the project's canonical short code.
    ///
    /// # Behavior
    ///
    /// - The input is lower-cased before lookup.
    /// - Known synonyms are collapsed via the internal `language_codes` map
    ///   (e.g. `"english"`/`"eng"`/`"EN"` → `"en"`,
    ///   `"cht"`/`"繁中"`/`"traditional-chinese"` is matched against the same
    ///   map; for the latter, only entries the detector knows are mapped).
    /// - Unknown but otherwise valid `[A-Za-z0-9_-]{1,16}` tokens are passed
    ///   through verbatim after lower-casing.
    /// - Returns `None` for empty input.
    ///
    /// # Arguments
    ///
    /// * `raw` - The raw language label to normalize.
    ///
    /// # Returns
    ///
    /// `Some(code)` with the canonical or pass-through code, or `None` when
    /// the input is empty.
    pub fn normalize(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let lower = trimmed.to_lowercase();
        if let Some(code) = self.language_codes.get(&lower) {
            return Some(code.clone());
        }
        // Also probe the original casing in case the caller passed a non-ASCII
        // synonym (e.g. `"繁中"`) whose lower-case form is identical.
        if let Some(code) = self.language_codes.get(trimmed) {
            return Some(code.clone());
        }
        Some(lower)
    }

    /// Return the primary detected language code for the provided path.
    ///
    /// # Returns
    ///
    /// `Some(code)` if detected, otherwise `None`.
    pub fn get_primary_language(&self, path: &Path) -> Option<String> {
        self.detect_all_languages(path)
            .into_iter()
            .next()
            .map(|lang| lang.code)
    }

    /// Collect all potential language detections from the path.
    ///
    /// Sorts results by confidence and removes duplicates by code.
    pub fn detect_all_languages(&self, path: &Path) -> Vec<LanguageInfo> {
        let mut langs = Vec::new();
        if let Some(dir_lang) = self.detect_from_directory(path) {
            langs.push(dir_lang);
        }
        if let Some(file_lang) = self.detect_from_filename(path) {
            langs.push(file_lang);
        }
        langs.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        langs.dedup_by(|a, b| a.code == b.code);
        langs
    }

    fn detect_from_directory(&self, path: &Path) -> Option<LanguageInfo> {
        for comp in path.components() {
            if let Some(s) = comp.as_os_str().to_str() {
                let key = s.to_lowercase();
                if let Some(code) = self.language_codes.get(&key) {
                    return Some(LanguageInfo {
                        code: code.clone(),
                        source: LanguageSource::Directory,
                        confidence: 0.9,
                    });
                }
            }
        }
        None
    }

    fn detect_from_filename(&self, path: &Path) -> Option<LanguageInfo> {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            for re in &self.filename_patterns {
                if let Some(cap) = re.captures(name) {
                    if let Some(m) = cap.get(1) {
                        if let Some(code) = self.language_codes.get(m.as_str()) {
                            return Some(LanguageInfo {
                                code: code.clone(),
                                source: LanguageSource::Filename,
                                confidence: 0.8,
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_directory_language_detection() {
        let det = LanguageDetector::new();
        let p = Path::new("tc/subtitle.srt");
        let lang = det.get_primary_language(p).unwrap();
        assert_eq!(lang, "tc");
    }

    #[test]
    fn test_filename_language_detection() {
        let det = LanguageDetector::new();
        let p = Path::new("subtitle.sc.ass");
        let lang = det.get_primary_language(p).unwrap();
        assert_eq!(lang, "sc");
    }

    #[test]
    fn test_no_language_detection() {
        let det = LanguageDetector::new();
        let p = Path::new("subtitle.ass");
        assert!(det.get_primary_language(p).is_none());
    }
}
