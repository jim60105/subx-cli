use futures::future::join_all;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::core::formats::manager::FormatManager;
use crate::core::formats::Subtitle;
use crate::Result;

/// 統一格式轉換器
pub struct FormatConverter {
    format_manager: FormatManager,
    pub(crate) config: ConversionConfig,
}

impl Clone for FormatConverter {
    fn clone(&self) -> Self {
        FormatConverter::new(self.config.clone())
    }
}

/// 轉換配置
#[derive(Debug, Clone)]
pub struct ConversionConfig {
    pub preserve_styling: bool,
    pub target_encoding: String,
    pub keep_original: bool,
    pub validate_output: bool,
}

/// 轉換結果
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
    /// 建立新的轉換器
    pub fn new(config: ConversionConfig) -> Self {
        Self {
            format_manager: FormatManager::new(),
            config,
        }
    }

    /// 轉換單一檔案
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
        let converted_subtitle = self.transform_subtitle(input_subtitle.clone(), target_format)?;

        // 3. 序列化為目標格式
        let target_formatter = self
            .format_manager
            .get_format(target_format)
            .ok_or_else(|| {
                crate::error::SubXError::subtitle_format(
                    format!("不支援的目標格式: {}", target_format),
                    "",
                )
            })?;

        let output_content = target_formatter.serialize(&converted_subtitle)?;

        // 4. 寫入檔案
        self.write_file_with_encoding(output_path, &output_content)
            .await?;

        // 5. 驗證轉換結果
        let result = if self.config.validate_output {
            self.validate_conversion(&input_subtitle, &converted_subtitle)
                .await?
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

    /// 批量轉換檔案
    pub async fn convert_batch(
        &self,
        input_dir: &Path,
        target_format: &str,
        recursive: bool,
    ) -> crate::Result<Vec<ConversionResult>> {
        let subtitle_files = self.discover_subtitle_files(input_dir, recursive).await?;
        let semaphore = Arc::new(Semaphore::new(4));

        let tasks = subtitle_files.into_iter().map(|file_path| {
            let sem = semaphore.clone();
            let converter = self.clone();
            let format = target_format.to_string();
            async move {
                let _permit = sem.acquire().await.unwrap();
                let output_path = file_path.with_extension(&format);
                converter
                    .convert_file(&file_path, &output_path, &format)
                    .await
            }
        });

        let results = join_all(tasks).await;
        results.into_iter().collect::<Result<Vec<_>>>()
    }
    /// 探索目錄中的字幕檔案
    async fn discover_subtitle_files(
        &self,
        input_dir: &Path,
        recursive: bool,
    ) -> crate::Result<Vec<std::path::PathBuf>> {
        let discovery = crate::core::matcher::discovery::FileDiscovery::new();
        let media_files = discovery.scan_directory(input_dir, recursive)?;
        let paths = media_files
            .into_iter()
            .filter(|f| {
                matches!(
                    f.file_type,
                    crate::core::matcher::discovery::MediaFileType::Subtitle
                )
            })
            .map(|f| f.path)
            .collect();
        Ok(paths)
    }

    /// 讀取檔案並轉為 UTF-8 字串
    async fn read_file_with_encoding(&self, path: &Path) -> crate::Result<String> {
        let bytes = tokio::fs::read(path).await?;
        // 自動檢測編碼並轉換為 UTF-8
        let detector = crate::core::formats::encoding::EncodingDetector::new()?;
        let info = detector.detect_encoding(&bytes)?;
        let converter = crate::core::formats::encoding::EncodingConverter::new();
        let conversion = converter.convert_to_utf8(&bytes, &info.charset)?;
        Ok(conversion.converted_text)
    }

    /// 寫入檔案（暫以 UTF-8 編碼）
    async fn write_file_with_encoding(&self, path: &Path, content: &str) -> crate::Result<()> {
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// 簡易轉換品質驗證
    async fn validate_conversion(
        &self,
        original: &Subtitle,
        converted: &Subtitle,
    ) -> crate::Result<ConversionResult> {
        let success = original.entries.len() == converted.entries.len();
        let errors = if success {
            Vec::new()
        } else {
            vec![format!(
                "條目數量不符: {} -> {}",
                original.entries.len(),
                converted.entries.len()
            )]
        };
        Ok(ConversionResult {
            success,
            input_format: original.format.to_string(),
            output_format: converted.format.to_string(),
            original_entries: original.entries.len(),
            converted_entries: converted.entries.len(),
            warnings: Vec::new(),
            errors,
        })
    }
}
