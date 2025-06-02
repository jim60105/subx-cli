# Product Backlog #04: 字幕格式解析引擎

## 領域範圍
字幕格式解析、統一資料結構、格式檢測和驗證

## 完成項目

### 1. 統一字幕資料結構
- [ ] 定義 `Subtitle` 和 `SubtitleEntry` 結構
- [ ] 實作時間戳記處理 (毫秒精度)
- [ ] 支援樣式和格式化資訊
- [ ] 實作 metadata 處理

### 2. SRT 格式支援
- [ ] SRT 格式解析器實作
- [ ] 時間格式解析 (`HH:MM:SS,mmm --> HH:MM:SS,mmm`)
- [ ] 多行文字內容處理
- [ ] 序列號驗證和修復

### 3. ASS/SSA 格式支援
- [ ] ASS 格式解析器實作
- [ ] 樣式和事件解析
- [ ] 進階格式化標籤處理
- [ ] 字型和顏色資訊保留

### 4. VTT 格式支援
- [ ] WebVTT 格式解析器實作
- [ ] Cue 設定和樣式處理
- [ ] NOTE 和 STYLE 區塊處理
- [ ] 時間戳記格式支援

### 5. SUB 格式支援
- [ ] MicroDVD SUB 格式
- [ ] SubViewer SUB 格式
- [ ] 幀率相關時間轉換
- [ ] 格式自動檢測

### 6. 格式檢測和驗證
- [ ] 檔案格式自動檢測
- [ ] 編碼檢測和轉換
- [ ] 格式錯誤診斷
- [ ] 修復建議提供

## 技術設計

### 核心資料結構
```rust
// src/core/formats/mod.rs
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Subtitle {
    pub entries: Vec<SubtitleEntry>,
    pub metadata: SubtitleMetadata,
    pub format: SubtitleFormatType,
}

#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    pub index: usize,
    pub start_time: Duration,
    pub end_time: Duration,
    pub text: String,
    pub styling: Option<StylingInfo>,
}

#[derive(Debug, Clone)]
pub struct SubtitleMetadata {
    pub title: Option<String>,
    pub language: Option<String>,
    pub encoding: String,
    pub frame_rate: Option<f32>,
    pub original_format: SubtitleFormatType,
}

#[derive(Debug, Clone)]
pub struct StylingInfo {
    pub font_name: Option<String>,
    pub font_size: Option<u32>,
    pub color: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}
```

### 格式 Trait 定義
```rust
// src/core/formats/mod.rs
pub trait SubtitleFormat {
    /// 解析字幕內容
    fn parse(&self, content: &str) -> crate::Result<Subtitle>;
    
    /// 序列化為字幕格式
    fn serialize(&self, subtitle: &Subtitle) -> crate::Result<String>;
    
    /// 檢測是否為此格式
    fn detect(content: &str) -> bool;
    
    /// 格式名稱
    fn format_name(&self) -> &'static str;
    
    /// 支援的副檔名
    fn file_extensions(&self) -> &'static [&'static str];
}
```

### SRT 格式實作
```rust
// src/core/formats/srt.rs
use regex::Regex;
use std::time::Duration;

pub struct SrtFormat;

impl SubtitleFormat for SrtFormat {
    fn parse(&self, content: &str) -> crate::Result<Subtitle> {
        let time_regex = Regex::new(
            r"(\d{2}):(\d{2}):(\d{2}),(\d{3}) --> (\d{2}):(\d{2}):(\d{2}),(\d{3})"
        )?;
        
        let mut entries = Vec::new();
        let blocks: Vec<&str> = content.split("\n\n").collect();
        
        for block in blocks {
            if block.trim().is_empty() {
                continue;
            }
            
            let lines: Vec<&str> = block.lines().collect();
            if lines.len() < 3 {
                continue;
            }
            
            // 解析序列號
            let index: usize = lines[0].trim().parse()
                .map_err(|_| crate::SubXError::SubtitleParse("無效的序列號".to_string()))?;
            
            // 解析時間戳記
            if let Some(captures) = time_regex.captures(lines[1]) {
                let start_time = self.parse_time(&captures, 1)?;
                let end_time = self.parse_time(&captures, 5)?;
                
                // 組合文字內容
                let text = lines[2..].join("\n");
                
                entries.push(SubtitleEntry {
                    index,
                    start_time,
                    end_time,
                    text,
                    styling: None,
                });
            }
        }
        
        Ok(Subtitle {
            entries,
            metadata: SubtitleMetadata {
                title: None,
                language: None,
                encoding: "utf-8".to_string(),
                frame_rate: None,
                original_format: SubtitleFormatType::Srt,
            },
            format: SubtitleFormatType::Srt,
        })
    }
    
    fn serialize(&self, subtitle: &Subtitle) -> crate::Result<String> {
        let mut output = String::new();
        
        for (i, entry) in subtitle.entries.iter().enumerate() {
            output.push_str(&format!("{}\n", i + 1));
            output.push_str(&self.format_time_range(entry.start_time, entry.end_time));
            output.push_str(&format!("{}\n\n", entry.text));
        }
        
        Ok(output)
    }
    
    fn detect(content: &str) -> bool {
        let time_pattern = Regex::new(r"\d{2}:\d{2}:\d{2},\d{3} --> \d{2}:\d{2}:\d{2},\d{3}").unwrap();
        time_pattern.is_match(content)
    }
    
    fn format_name(&self) -> &'static str {
        "SRT"
    }
    
    fn file_extensions(&self) -> &'static [&'static str] {
        &["srt"]
    }
}

impl SrtFormat {
    fn parse_time(&self, captures: &regex::Captures, start_group: usize) -> crate::Result<Duration> {
        let hours: u64 = captures[start_group].parse()?;
        let minutes: u64 = captures[start_group + 1].parse()?;
        let seconds: u64 = captures[start_group + 2].parse()?;
        let milliseconds: u64 = captures[start_group + 3].parse()?;
        
        Ok(Duration::from_millis(
            hours * 3600000 + minutes * 60000 + seconds * 1000 + milliseconds
        ))
    }
    
    fn format_time_range(&self, start: Duration, end: Duration) -> String {
        format!("{} --> {}\n", 
            self.format_duration(start),
            self.format_duration(end)
        )
    }
    
    fn format_duration(&self, duration: Duration) -> String {
        let total_ms = duration.as_millis();
        let hours = total_ms / 3600000;
        let minutes = (total_ms % 3600000) / 60000;
        let seconds = (total_ms % 60000) / 1000;
        let milliseconds = total_ms % 1000;
        
        format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, milliseconds)
    }
}
```

### 格式管理器
```rust
// src/core/formats/manager.rs
pub struct FormatManager {
    formats: Vec<Box<dyn SubtitleFormat>>,
}

impl FormatManager {
    pub fn new() -> Self {
        Self {
            formats: vec![
                Box::new(SrtFormat),
                Box::new(AssFormat),
                Box::new(VttFormat),
                Box::new(SubFormat),
            ],
        }
    }
    
    /// 自動檢測格式並解析
    pub fn parse_auto(&self, content: &str) -> crate::Result<Subtitle> {
        for format in &self.formats {
            if format.detect(content) {
                return format.parse(content);
            }
        }
        
        Err(crate::SubXError::SubtitleParse("未知的字幕格式".to_string()))
    }
    
    /// 根據格式名稱取得解析器
    pub fn get_format(&self, name: &str) -> Option<&dyn SubtitleFormat> {
        self.formats.iter()
            .find(|f| f.format_name().to_lowercase() == name.to_lowercase())
            .map(|f| f.as_ref())
    }
    
    /// 根據副檔名取得解析器
    pub fn get_format_by_extension(&self, ext: &str) -> Option<&dyn SubtitleFormat> {
        let ext = ext.to_lowercase();
        self.formats.iter()
            .find(|f| f.file_extensions().contains(&ext.as_str()))
            .map(|f| f.as_ref())
    }
}
```

### 編碼檢測
```rust
// src/core/formats/encoding.rs
use encoding_rs::{Encoding, UTF_8};

pub fn detect_encoding(bytes: &[u8]) -> &'static Encoding {
    // 嘗試 UTF-8
    if UTF_8.decode_without_bom_handling(bytes).2 {
        return UTF_8;
    }
    
    // 檢測其他常見編碼
    let encodings = [
        encoding_rs::GBK,
        encoding_rs::BIG5,
        encoding_rs::SHIFT_JIS,
        encoding_rs::EUC_KR,
    ];
    
    for encoding in &encodings {
        let (decoded, _used, had_errors) = encoding.decode(bytes);
        if !had_errors {
            return encoding;
        }
    }
    
    // 預設返回 UTF-8
    UTF_8
}

pub fn convert_to_utf8(bytes: &[u8]) -> crate::Result<String> {
    let encoding = detect_encoding(bytes);
    let (decoded, _encoding, had_errors) = encoding.decode(bytes);
    
    if had_errors {
        return Err(crate::SubXError::SubtitleParse(
            "編碼轉換失敗".to_string()
        ));
    }
    
    Ok(decoded.into_owned())
}
```

## 驗收標準
1. 所有支援格式能正確解析和序列化
2. 格式自動檢測準確率 > 95%
3. 編碼檢測和轉換正常運作
4. 錯誤格式有清楚的錯誤提示
5. 效能測試：解析 1000 條字幕 < 100ms

## 估計工時
5-6 天

## 相依性
- 依賴 Backlog #01 (專案基礎建設)

## 風險評估
- 中風險：字幕格式複雜度較高
- 注意事項：邊界情況處理、編碼相容性
