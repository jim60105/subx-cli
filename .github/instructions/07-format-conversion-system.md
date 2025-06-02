# Product Backlog #07: 格式轉換系統

## 領域範圍
字幕格式互轉、批量轉換、樣式保留、編碼處理

## 完成項目

### 1. 轉換引擎架構
- [ ] 統一轉換介面設計
- [ ] 格式轉換映射表
- [ ] 轉換品質保證機制
- [ ] 錯誤處理和回滾

### 2. 格式特定轉換器
- [ ] SRT ↔ ASS 轉換器
- [ ] SRT ↔ VTT 轉換器
- [ ] ASS ↔ VTT 轉換器
- [ ] SUB 格式轉換支援

### 3. 樣式資訊處理
- [ ] ASS 樣式保留和轉換
- [ ] 字型資訊映射
- [ ] 顏色和格式化轉換
- [ ] 位置和動畫處理

### 4. 批量轉換功能
- [ ] 資料夾批量處理
- [ ] 並行轉換處理
- [ ] 進度追蹤和報告
- [ ] 轉換統計資訊

### 5. 轉換選項和配置
- [ ] 樣式保留選項
- [ ] 編碼選擇
- [ ] 輸出檔名策略
- [ ] 原檔案保留選項

### 6. 品質驗證
- [ ] 轉換前後比較
- [ ] 時間軸完整性檢查
- [ ] 內容一致性驗證
- [ ] 格式規範檢查

## 技術設計

### 轉換引擎核心
```rust
// src/core/formats/converter.rs
use crate::core::formats::{Subtitle, SubtitleFormat};

pub struct FormatConverter {
    format_manager: FormatManager,
    config: ConversionConfig,
}

#[derive(Debug, Clone)]
pub struct ConversionConfig {
    pub preserve_styling: bool,
    pub target_encoding: String,
    pub keep_original: bool,
    pub validate_output: bool,
}

#[derive(Debug)]
pub struct ConversionResult {
    pub success: bool,
    pub input_format: String,
    pub output_format: String,
    pub original_entries: usize,
    pub converted_entries: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl FormatConverter {
    pub fn new(config: ConversionConfig) -> Self {
        Self {
            format_manager: FormatManager::new(),
            config,
        }
    }
    
    pub async fn convert_file(
        &self,
        input_path: &Path,
        output_path: &Path,
        target_format: &str,
    ) -> crate::Result<ConversionResult> {
        // 1. 讀取和解析輸入檔案
        let input_content = self.read_file_with_encoding(input_path).await?;
        let input_subtitle = self.format_manager.parse_auto(&input_content)?;
        
        // 2. 執行格式轉換
        let converted_subtitle = self.transform_subtitle(input_subtitle, target_format)?;
        
        // 3. 序列化為目標格式
        let target_formatter = self.format_manager
            .get_format(target_format)
            .ok_or_else(|| crate::SubXError::SubtitleParse(
                format!("不支援的目標格式: {}", target_format)
            ))?;
        
        let output_content = target_formatter.serialize(&converted_subtitle)?;
        
        // 4. 寫入檔案
        self.write_file_with_encoding(output_path, &output_content).await?;
        
        // 5. 驗證轉換結果
        let result = if self.config.validate_output {
            self.validate_conversion(&input_subtitle, &converted_subtitle).await?
        } else {
            ConversionResult {
                success: true,
                input_format: input_subtitle.format.to_string(),
                output_format: target_format.to_string(),
                original_entries: input_subtitle.entries.len(),
                converted_entries: converted_subtitle.entries.len(),
                warnings: Vec::new(),
                errors: Vec::new(),
            }
        };
        
        Ok(result)
    }
    
    pub async fn convert_batch(
        &self,
        input_dir: &Path,
        target_format: &str,
        recursive: bool,
    ) -> crate::Result<Vec<ConversionResult>> {
        let subtitle_files = self.discover_subtitle_files(input_dir, recursive).await?;
        let semaphore = Arc::new(Semaphore::new(4)); // 限制並行數
        
        let tasks: Vec<_> = subtitle_files.into_iter().map(|file_path| {
            let sem = semaphore.clone();
            let converter = self.clone();
            let format = target_format.to_string();
            
            tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                
                let output_path = file_path.with_extension(&format);
                converter.convert_file(&file_path, &output_path, &format).await
            })
        }).collect();
        
        let results = futures::future::join_all(tasks).await;
        
        results.into_iter()
            .map(|result| result.unwrap())
            .collect::<Result<Vec<_>, _>>()
    }
}
```

### 格式轉換映射
```rust
// src/core/formats/transformers.rs
impl FormatConverter {
    fn transform_subtitle(
        &self,
        mut subtitle: Subtitle,
        target_format: &str,
    ) -> crate::Result<Subtitle> {
        match (subtitle.format.as_str(), target_format) {
            ("srt", "ass") => self.srt_to_ass(subtitle),
            ("ass", "srt") => self.ass_to_srt(subtitle),
            ("srt", "vtt") => self.srt_to_vtt(subtitle),
            ("vtt", "srt") => self.vtt_to_srt(subtitle),
            ("ass", "vtt") => self.ass_to_vtt(subtitle),
            ("vtt", "ass") => self.vtt_to_ass(subtitle),
            (source, target) if source == target => Ok(subtitle),
            _ => Err(crate::SubXError::SubtitleParse(
                format!("不支援的轉換: {} -> {}", subtitle.format, target_format)
            )),
        }
    }
    
    fn srt_to_ass(&self, mut subtitle: Subtitle) -> crate::Result<Subtitle> {
        // 建立基本 ASS 樣式
        let default_style = AssStyle {
            name: "Default".to_string(),
            font_name: "Arial".to_string(),
            font_size: 16,
            primary_color: Color::white(),
            secondary_color: Color::red(),
            outline_color: Color::black(),
            shadow_color: Color::black(),
            bold: false,
            italic: false,
            underline: false,
            alignment: 2, // 底部居中
        };
        
        // 轉換每個字幕項目
        for entry in &mut subtitle.entries {
            if self.config.preserve_styling {
                // 嘗試從 SRT 標籤提取樣式資訊
                entry.styling = Some(self.extract_srt_styling(&entry.text)?);
            }
            
            // 清理 SRT 標籤並轉換為 ASS 格式
            entry.text = self.convert_srt_tags_to_ass(&entry.text);
        }
        
        subtitle.format = SubtitleFormatType::Ass;
        subtitle.metadata.original_format = SubtitleFormatType::Srt;
        
        Ok(subtitle)
    }
    
    fn ass_to_srt(&self, mut subtitle: Subtitle) -> crate::Result<Subtitle> {
        for entry in &mut subtitle.entries {
            // 移除 ASS 特定的標籤和樣式
            entry.text = self.strip_ass_tags(&entry.text);
            
            // 轉換基本格式標籤為 SRT
            if self.config.preserve_styling {
                entry.text = self.convert_ass_tags_to_srt(&entry.text);
            }
            
            entry.styling = None; // SRT 不支援詳細樣式
        }
        
        subtitle.format = SubtitleFormatType::Srt;
        Ok(subtitle)
    }
    
    fn srt_to_vtt(&self, mut subtitle: Subtitle) -> crate::Result<Subtitle> {
        // VTT 需要 WEBVTT 標頭
        subtitle.metadata.title = Some("WEBVTT".to_string());
        
        for entry in &mut subtitle.entries {
            // 轉換時間格式 (VTT 使用 . 而不是 ,)
            // 轉換標籤格式
            entry.text = self.convert_srt_tags_to_vtt(&entry.text);
        }
        
        subtitle.format = SubtitleFormatType::Vtt;
        Ok(subtitle)
    }
}
```

### 樣式轉換處理
```rust
// src/core/formats/styling.rs
impl FormatConverter {
    fn extract_srt_styling(&self, text: &str) -> crate::Result<StylingInfo> {
        let mut styling = StylingInfo::default();
        
        // 檢測粗體
        if text.contains("<b>") || text.contains("<B>") {
            styling.bold = true;
        }
        
        // 檢測斜體
        if text.contains("<i>") || text.contains("<I>") {
            styling.italic = true;
        }
        
        // 檢測底線
        if text.contains("<u>") || text.contains("<U>") {
            styling.underline = true;
        }
        
        // 檢測顏色
        if let Some(color) = self.extract_color_from_tags(text) {
            styling.color = Some(color);
        }
        
        Ok(styling)
    }
    
    fn convert_srt_tags_to_ass(&self, text: &str) -> String {
        let mut result = text.to_string();
        
        // 基本標籤轉換
        result = result.replace("<b>", "{\\b1}").replace("</b>", "{\\b0}");
        result = result.replace("<i>", "{\\i1}").replace("</i>", "{\\i0}");
        result = result.replace("<u>", "{\\u1}").replace("</u>", "{\\u0}");
        
        // 顏色標籤轉換
        let color_regex = Regex::new(r#"<font color="([^"]+)">"#).unwrap();
        result = color_regex.replace_all(&result, |caps: &regex::Captures| {
            let color = &caps[1];
            format!("{{\\c&H{}&}}", self.convert_color_to_ass(color))
        }).to_string();
        
        result = result.replace("</font>", "{\\c}");
        
        result
    }
    
    fn strip_ass_tags(&self, text: &str) -> String {
        // 移除所有 ASS 標籤 {\...}
        let tag_regex = Regex::new(r"\{[^}]*\}").unwrap();
        tag_regex.replace_all(text, "").to_string()
    }
    
    fn convert_ass_tags_to_srt(&self, text: &str) -> String {
        let mut result = text.to_string();
        
        // 粗體轉換
        let bold_regex = Regex::new(r"\{\\b1\}([^{]*)\{\\b0\}").unwrap();
        result = bold_regex.replace_all(&result, "<b>$1</b>").to_string();
        
        // 斜體轉換
        let italic_regex = Regex::new(r"\{\\i1\}([^{]*)\{\\i0\}").unwrap();
        result = italic_regex.replace_all(&result, "<i>$1</i>").to_string();
        
        // 底線轉換
        let underline_regex = Regex::new(r"\{\\u1\}([^{]*)\{\\u0\}").unwrap();
        result = underline_regex.replace_all(&result, "<u>$1</u>").to_string();
        
        result
    }
}
```

### 批量轉換命令實作
```rust
// src/commands/convert_command.rs
use crate::cli::ConvertArgs;
use crate::core::formats::converter::{FormatConverter, ConversionConfig};

pub async fn execute(args: ConvertArgs) -> crate::Result<()> {
    let config = ConversionConfig {
        preserve_styling: true, // 從配置讀取
        target_encoding: args.encoding.clone(),
        keep_original: args.keep_original,
        validate_output: true,
    };
    
    let converter = FormatConverter::new(config);
    
    if args.input.is_file() {
        // 單檔案轉換
        let output_path = args.output.unwrap_or_else(|| {
            args.input.with_extension(&args.format.to_string())
        });
        
        let result = converter.convert_file(&args.input, &output_path, &args.format.to_string()).await?;
        
        if result.success {
            println!("✓ 轉換完成: {} -> {}", 
                args.input.display(), 
                output_path.display()
            );
        } else {
            println!("✗ 轉換失敗");
            for error in result.errors {
                println!("  錯誤: {}", error);
            }
        }
    } else {
        // 批量轉換
        let results = converter.convert_batch(&args.input, &args.format.to_string(), true).await?;
        
        let success_count = results.iter().filter(|r| r.success).count();
        let total_count = results.len();
        
        println!("批量轉換完成: {}/{} 成功", success_count, total_count);
        
        for result in results.iter().filter(|r| !r.success) {
            println!("失敗: {}", result.errors.join(", "));
        }
    }
    
    Ok(())
}
```

## 驗收標準
1. 支援所有主要格式間的轉換
2. 樣式資訊正確保留和轉換
3. 批量轉換效能良好
4. 轉換品質驗證有效
5. 錯誤處理和恢復機制完善

## 估計工時
4-5 天

## 相依性
- 依賴 Backlog #04 (字幕格式解析引擎)

## 風險評估
- 中風險：格式間差異較大
- 注意事項：樣式轉換的完整性、格式相容性
