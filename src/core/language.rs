//! 語言編碼識別模組
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

/// 語言資訊來源
#[derive(Debug, Clone, PartialEq)]
pub enum LanguageSource {
    /// 來自目錄名稱
    Directory,
    /// 來自檔名
    Filename,
    /// 來自副檔名前模式
    Extension,
}
impl Default for LanguageDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// 語言識別結果
#[derive(Debug, Clone)]
pub struct LanguageInfo {
    /// 標準化語言編碼，如 tc、sc、en
    pub code: String,
    /// 資訊來源
    pub source: LanguageSource,
    /// 識別信心度
    pub confidence: f32,
}

/// 語言編碼偵測器
pub struct LanguageDetector {
    language_codes: HashMap<String, String>,
    directory_patterns: Vec<String>,
    filename_patterns: Vec<Regex>,
}

impl LanguageDetector {
    /// 建立新的偵測器，初始化語言字典和模式
    pub fn new() -> Self {
        let mut language_codes = HashMap::new();
        // 繁體
        language_codes.insert("tc".to_string(), "tc".to_string());
        language_codes.insert("繁中".to_string(), "tc".to_string());
        language_codes.insert("繁體".to_string(), "tc".to_string());
        language_codes.insert("cht".to_string(), "tc".to_string());
        // 簡體
        language_codes.insert("sc".to_string(), "sc".to_string());
        language_codes.insert("簡中".to_string(), "sc".to_string());
        language_codes.insert("簡體".to_string(), "sc".to_string());
        language_codes.insert("chs".to_string(), "sc".to_string());
        // 英文
        language_codes.insert("en".to_string(), "en".to_string());
        language_codes.insert("英文".to_string(), "en".to_string());
        language_codes.insert("english".to_string(), "en".to_string());
        // 日文、韓文等可按需擴充

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
    /// 偵測路徑中的單一語言資訊，目錄優先，再檔名
    pub fn detect_from_path(&self, path: &Path) -> Option<LanguageInfo> {
        if let Some(lang) = self.detect_from_directory(path) {
            return Some(lang);
        }
        if let Some(lang) = self.detect_from_filename(path) {
            return Some(lang);
        }
        None
    }

    /// 偵測路徑中主要的語言編碼
    pub fn get_primary_language(&self, path: &Path) -> Option<String> {
        self.detect_all_languages(path)
            .into_iter()
            .next()
            .map(|lang| lang.code)
    }

    /// 收集所有可能的語言資訊，並依信心度排序去重
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
