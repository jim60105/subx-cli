use regex::Regex;

/// 解析後的檔案名稱資訊
#[derive(Debug, Clone)]
pub struct ParsedFilename {
    pub title: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub year: Option<u32>,
    pub quality: Option<String>,
    pub language: Option<String>,
    pub group: Option<String>,
}

/// 檔名分析器
pub struct FilenameAnalyzer {
    season_episode_patterns: Vec<Regex>,
    year_pattern: Regex,
    quality_pattern: Regex,
}

impl FilenameAnalyzer {
    /// 建立新的檔名分析器，初始化常用正規表達式
    pub fn new() -> Self {
        Self {
            season_episode_patterns: vec![
                Regex::new(r"[Ss](\d{1,2})[Ee](\d{1,3})").unwrap(),
                Regex::new(r"(\d{1,2})x(\d{1,3})").unwrap(),
                Regex::new(r"Season\s*(\d{1,2}).*Episode\s*(\d{1,3})").unwrap(),
                Regex::new(r"第(\d{1,2})季.*第(\d{1,3})集").unwrap(),
            ],
            year_pattern: Regex::new(r"\b(19|20)\d{2}\b").unwrap(),
            quality_pattern: Regex::new(r"\b(720p|1080p|4K|2160p|BluRay|WEB-DL|HDRip)\b").unwrap(),
        }
    }

    /// 解析檔名，並擷取標題、季集、年份、畫質等資訊
    pub fn parse(&self, filename: &str) -> ParsedFilename {
        let mut parsed = ParsedFilename {
            title: String::new(),
            season: None,
            episode: None,
            year: None,
            quality: None,
            language: None,
            group: None,
        };

        // 提取季集資訊
        for pattern in &self.season_episode_patterns {
            if let Some(caps) = pattern.captures(filename) {
                parsed.season = caps.get(1).and_then(|m| m.as_str().parse().ok());
                parsed.episode = caps.get(2).and_then(|m| m.as_str().parse().ok());
                break;
            }
        }

        // 提取年份
        if let Some(m) = self.year_pattern.find(filename) {
            parsed.year = m.as_str().parse().ok();
        }

        // 提取品質資訊
        if let Some(m) = self.quality_pattern.find(filename) {
            parsed.quality = Some(m.as_str().to_string());
        }

        // 提取標題
        parsed.title = self.extract_title(filename, &parsed);

        parsed
    }

    fn extract_title(&self, filename: &str, _parsed: &ParsedFilename) -> String {
        let mut title = filename.to_string();

        // 移除季集、年份、品質等模式
        for pattern in &self.season_episode_patterns {
            title = pattern.replace_all(&title, "").to_string();
        }
        title = self.year_pattern.replace_all(&title, "").to_string();
        title = self.quality_pattern.replace_all(&title, "").to_string();

        // 清理字元並標準化空格
        title = title.replace(['.', '_', '-'], " ");
        title = title.trim().to_string();
        while title.contains("  ") {
            title = title.replace("  ", " ");
        }

        title
    }
}

impl Default for FilenameAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
