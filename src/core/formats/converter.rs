//! Subtitle format conversion engine.
//!
//! This module provides the `FormatConverter`, which performs
//! format conversions between different subtitle formats,
//! supporting concurrent processing and task coordination.
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::core::formats::converter::FormatConverter;
//! // Initialize with default configuration and run conversion tasks
//! let converter = FormatConverter::new(Default::default());
//! ```

use futures::future::join_all;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::Result;
use crate::core::formats::Subtitle;
use crate::core::formats::manager::FormatManager;

/// Subtitle format converter for handling conversion tasks.
///
/// The `FormatConverter` coordinates conversion requests across
/// multiple subtitle formats, managing concurrency and task scheduling.
pub struct FormatConverter {
    format_manager: FormatManager,
    pub(crate) config: ConversionConfig,
}

impl Clone for FormatConverter {
    fn clone(&self) -> Self {
        FormatConverter::new(self.config.clone())
    }
}

/// Conversion configuration
#[derive(Debug, Clone)]
pub struct ConversionConfig {
    /// Whether to preserve styling information during conversion
    pub preserve_styling: bool,
    /// Target character encoding for the output file
    pub target_encoding: String,
    /// Whether to keep the original file after conversion
    pub keep_original: bool,
    /// Whether to validate the output after conversion
    pub validate_output: bool,
}

/// Result of a subtitle format conversion operation.
///
/// Contains detailed information about the conversion process including
/// success status, format information, entry counts, and any issues encountered.
#[derive(Debug)]
pub struct ConversionResult {
    /// Whether the conversion completed successfully
    pub success: bool,
    /// Input subtitle format (e.g., "srt", "ass")
    pub input_format: String,
    /// Output subtitle format (e.g., "srt", "ass")
    pub output_format: String,
    /// Number of subtitle entries in the original file
    pub original_entries: usize,
    /// Number of subtitle entries successfully converted
    pub converted_entries: usize,
    /// Non-fatal warnings encountered during conversion
    pub warnings: Vec<String>,
    /// Errors encountered during conversion
    pub errors: Vec<String>,
}

impl FormatConverter {
    /// Create new converter
    pub fn new(config: ConversionConfig) -> Self {
        Self {
            format_manager: FormatManager::new(),
            config,
        }
    }

    /// Convert single file
    pub async fn convert_file(
        &self,
        input_path: &Path,
        output_path: &Path,
        target_format: &str,
    ) -> crate::Result<ConversionResult> {
        // 1. Read and parse input file
        let input_content = self.read_file_with_encoding(input_path).await?;
        let input_subtitle = self.format_manager.parse_auto(&input_content)?;

        // 2. Execute format conversion
        let converted_subtitle = self.transform_subtitle(input_subtitle.clone(), target_format)?;

        // 3. Serialize to target format
        let target_formatter = self
            .format_manager
            .get_format(target_format)
            .ok_or_else(|| {
                crate::error::SubXError::subtitle_format(
                    format!("Unsupported target format: {}", target_format),
                    "",
                )
            })?;

        let output_content = target_formatter.serialize(&converted_subtitle)?;

        // 4. Write file
        self.write_file_with_encoding(output_path, &output_content)
            .await?;

        // 5. Validate conversion result
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

    /// Batch convert files
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
    /// Discover subtitle files in directory
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
            .map(|f| f.path) // 使用 path 欄位，行為不變
            .collect();
        Ok(paths)
    }

    /// Read file and convert to UTF-8 string
    async fn read_file_with_encoding(&self, path: &Path) -> crate::Result<String> {
        let bytes = tokio::fs::read(path).await?;
        // Auto-detect encoding and convert to UTF-8
        let detector = crate::core::formats::encoding::EncodingDetector::with_defaults();
        let info = detector.detect_encoding(&bytes)?;
        let converter = crate::core::formats::encoding::EncodingConverter::new();
        let conversion = converter.convert_to_utf8(&bytes, &info.charset)?;
        Ok(conversion.converted_text)
    }

    /// Write file (temporarily using UTF-8 encoding)
    async fn write_file_with_encoding(&self, path: &Path, content: &str) -> crate::Result<()> {
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Simple conversion quality validation
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
                "Entry count mismatch: {} -> {}",
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
