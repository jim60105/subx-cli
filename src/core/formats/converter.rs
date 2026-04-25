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
            .map(|f| f.path) // Use path field, behavior unchanged
            .collect();
        Ok(paths)
    }

    /// Read file and convert to UTF-8 string
    async fn read_file_with_encoding(&self, path: &Path) -> crate::Result<String> {
        crate::core::fs_util::check_file_size(path, 52_428_800, "Subtitle")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn default_config() -> ConversionConfig {
        ConversionConfig {
            preserve_styling: false,
            target_encoding: "UTF-8".to_string(),
            keep_original: false,
            validate_output: false,
        }
    }

    fn validating_config() -> ConversionConfig {
        ConversionConfig {
            validate_output: true,
            ..default_config()
        }
    }

    const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:02,500\nHello world\n\n2\n00:00:03,000 --> 00:00:04,000\nSecond line\n\n";
    const SAMPLE_VTT: &str = "WEBVTT\n\n1\n00:00:01.000 --> 00:00:02.500\nHello world\n\n2\n00:00:03.000 --> 00:00:04.000\nSecond line\n\n";
    const SAMPLE_ASS: &str = "[Script Info]\nScriptType: v4.00+\n\n[V4+ Styles]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n\n[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:01.00,0:00:02.50,Default,,0000,0000,0000,,Hello world\nDialogue: 0,0:00:03.00,0:00:04.00,Default,,0000,0000,0000,,Second line\n";

    fn write_temp_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, content).expect("write temp file");
        path
    }

    // ── ConversionConfig & ConversionResult ──────────────────────────────────

    #[test]
    fn conversion_config_clone_and_debug() {
        let cfg = default_config();
        let cloned = cfg.clone();
        assert_eq!(cloned.target_encoding, "UTF-8");
        assert!(!cloned.preserve_styling);
        assert!(!cloned.keep_original);
        assert!(!cloned.validate_output);
        let dbg = format!("{:?}", cfg);
        assert!(dbg.contains("ConversionConfig"));
    }

    #[test]
    fn conversion_result_debug() {
        let r = ConversionResult {
            success: true,
            input_format: "srt".to_string(),
            output_format: "ass".to_string(),
            original_entries: 2,
            converted_entries: 2,
            warnings: vec![],
            errors: vec![],
        };
        let dbg = format!("{:?}", r);
        assert!(dbg.contains("ConversionResult"));
    }

    #[test]
    fn format_converter_clone() {
        let conv = FormatConverter::new(default_config());
        let cloned = conv.clone();
        assert!(!cloned.config.preserve_styling);
    }

    // ── convert_file: SRT → ASS ──────────────────────────────────────────────

    #[tokio::test]
    async fn convert_file_srt_to_ass() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.srt", SAMPLE_SRT);
        let output = dir.path().join("test.ass");

        let conv = FormatConverter::new(default_config());
        let result = conv.convert_file(&input, &output, "ass").await.unwrap();

        assert!(result.success);
        assert_eq!(result.input_format, "srt");
        assert_eq!(result.output_format, "ass");
        assert_eq!(result.original_entries, 2);
        assert_eq!(result.converted_entries, 2);
        assert!(result.warnings.is_empty());
        assert!(result.errors.is_empty());
        assert!(output.exists());
    }

    // ── convert_file: ASS → SRT ──────────────────────────────────────────────

    #[tokio::test]
    async fn convert_file_ass_to_srt() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.ass", SAMPLE_ASS);
        let output = dir.path().join("test.srt");

        let conv = FormatConverter::new(default_config());
        let result = conv.convert_file(&input, &output, "srt").await.unwrap();

        assert!(result.success);
        assert_eq!(result.input_format, "ass");
        assert_eq!(result.output_format, "srt");
        assert_eq!(result.original_entries, 2);
        assert_eq!(result.converted_entries, 2);
        assert!(output.exists());
    }

    // ── convert_file: SRT → VTT ──────────────────────────────────────────────

    #[tokio::test]
    async fn convert_file_srt_to_vtt() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.srt", SAMPLE_SRT);
        let output = dir.path().join("test.vtt");

        let conv = FormatConverter::new(default_config());
        let result = conv.convert_file(&input, &output, "vtt").await.unwrap();

        assert!(result.success);
        assert_eq!(result.input_format, "srt");
        assert_eq!(result.output_format, "vtt");
        assert_eq!(result.converted_entries, 2);
        assert!(output.exists());
        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("WEBVTT"));
    }

    // ── convert_file: VTT → SRT ──────────────────────────────────────────────

    #[tokio::test]
    async fn convert_file_vtt_to_srt() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.vtt", SAMPLE_VTT);
        let output = dir.path().join("test.srt");

        let conv = FormatConverter::new(default_config());
        let result = conv.convert_file(&input, &output, "srt").await.unwrap();

        assert!(result.success);
        assert_eq!(result.input_format, "vtt");
        assert_eq!(result.output_format, "srt");
        assert_eq!(result.converted_entries, 2);
        assert!(output.exists());
        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("-->"));
    }

    // ── convert_file: ASS → VTT ──────────────────────────────────────────────

    #[tokio::test]
    async fn convert_file_ass_to_vtt() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.ass", SAMPLE_ASS);
        let output = dir.path().join("test.vtt");

        let conv = FormatConverter::new(default_config());
        let result = conv.convert_file(&input, &output, "vtt").await.unwrap();

        assert!(result.success);
        assert_eq!(result.input_format, "ass");
        assert_eq!(result.output_format, "vtt");
        assert_eq!(result.converted_entries, 2);
        assert!(output.exists());
    }

    // ── convert_file: VTT → ASS ──────────────────────────────────────────────

    #[tokio::test]
    async fn convert_file_vtt_to_ass() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.vtt", SAMPLE_VTT);
        let output = dir.path().join("test.ass");

        let conv = FormatConverter::new(default_config());
        let result = conv.convert_file(&input, &output, "ass").await.unwrap();

        assert!(result.success);
        assert_eq!(result.input_format, "vtt");
        assert_eq!(result.output_format, "ass");
        assert_eq!(result.converted_entries, 2);
        assert!(output.exists());
    }

    // ── convert_file: validate_output=true (equal entry counts) ─────────────

    #[tokio::test]
    async fn convert_file_with_validate_output_success() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.srt", SAMPLE_SRT);
        let output = dir.path().join("test.ass");

        let conv = FormatConverter::new(validating_config());
        let result = conv.convert_file(&input, &output, "ass").await.unwrap();

        assert!(result.success);
        assert!(result.errors.is_empty());
        assert_eq!(result.original_entries, result.converted_entries);
    }

    // ── convert_file: validate_output=true, VTT → SRT ───────────────────────

    #[tokio::test]
    async fn convert_file_vtt_to_srt_with_validation() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.vtt", SAMPLE_VTT);
        let output = dir.path().join("test.srt");

        let conv = FormatConverter::new(validating_config());
        let result = conv.convert_file(&input, &output, "srt").await.unwrap();

        assert!(result.success);
        assert_eq!(result.original_entries, result.converted_entries);
    }

    // ── convert_file: non-existent input file → error ───────────────────────

    #[tokio::test]
    async fn convert_file_missing_input_returns_error() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("nonexistent.srt");
        let output = dir.path().join("out.ass");

        let conv = FormatConverter::new(default_config());
        let err = conv.convert_file(&input, &output, "ass").await;
        assert!(err.is_err());
    }

    // ── convert_file: unsupported target format → error ─────────────────────

    #[tokio::test]
    async fn convert_file_unsupported_target_format_returns_error() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.srt", SAMPLE_SRT);
        let output = dir.path().join("test.xyz");

        let conv = FormatConverter::new(default_config());
        let err = conv.convert_file(&input, &output, "xyz").await;
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(
            msg.to_lowercase().contains("unsupported")
                || msg.contains("xyz")
                || msg.contains("format")
        );
    }

    // ── convert_file: invalid/unrecognized content → error ───────────────────

    #[tokio::test]
    async fn convert_file_unrecognized_format_returns_error() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.srt", "this is not a subtitle file");
        let output = dir.path().join("out.ass");

        let conv = FormatConverter::new(default_config());
        let err = conv.convert_file(&input, &output, "ass").await;
        assert!(err.is_err());
    }

    // ── convert_file: output file is written correctly ───────────────────────

    #[tokio::test]
    async fn convert_file_output_is_valid_ass() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.srt", SAMPLE_SRT);
        let output = dir.path().join("test.ass");

        let conv = FormatConverter::new(default_config());
        conv.convert_file(&input, &output, "ass").await.unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("[Events]"));
        assert!(content.contains("Dialogue:"));
    }

    // ── convert_file: SRT → SRT (same format) ───────────────────────────────

    #[tokio::test]
    async fn convert_file_srt_to_srt_same_format() {
        let dir = TempDir::new().unwrap();
        let input = write_temp_file(&dir, "test.srt", SAMPLE_SRT);
        let output = dir.path().join("out.srt");

        let conv = FormatConverter::new(default_config());
        let result = conv.convert_file(&input, &output, "srt").await.unwrap();

        assert!(result.success);
        assert_eq!(result.input_format, "srt");
        assert_eq!(result.output_format, "srt");
        assert_eq!(result.original_entries, 2);
    }

    // ── convert_batch: empty directory ───────────────────────────────────────

    #[tokio::test]
    async fn convert_batch_empty_directory_returns_empty_vec() {
        let dir = TempDir::new().unwrap();

        let conv = FormatConverter::new(default_config());
        let results = conv.convert_batch(dir.path(), "ass", false).await.unwrap();
        assert!(results.is_empty());
    }

    // ── convert_batch: directory with SRT files ───────────────────────────────

    #[tokio::test]
    async fn convert_batch_converts_srt_files_to_ass() {
        let dir = TempDir::new().unwrap();
        write_temp_file(&dir, "a.srt", SAMPLE_SRT);
        write_temp_file(&dir, "b.srt", SAMPLE_SRT);

        let conv = FormatConverter::new(default_config());
        let results = conv.convert_batch(dir.path(), "ass", false).await.unwrap();

        assert_eq!(results.len(), 2);
        for r in &results {
            assert!(r.success);
            assert_eq!(r.output_format, "ass");
        }
    }

    // ── convert_batch: recursive flag ────────────────────────────────────────

    #[tokio::test]
    async fn convert_batch_recursive_discovers_nested_files() {
        let dir = TempDir::new().unwrap();
        let subdir = dir.path().join("sub");
        std::fs::create_dir(&subdir).unwrap();
        write_temp_file(&dir, "top.srt", SAMPLE_SRT);
        std::fs::write(subdir.join("nested.srt"), SAMPLE_SRT).unwrap();

        let conv = FormatConverter::new(default_config());

        // Non-recursive: only top-level
        let flat = conv.convert_batch(dir.path(), "vtt", false).await.unwrap();
        assert_eq!(flat.len(), 1);

        // Recursive: includes nested
        let dir2 = TempDir::new().unwrap();
        let subdir2 = dir2.path().join("sub");
        std::fs::create_dir(&subdir2).unwrap();
        write_temp_file(&dir2, "top.srt", SAMPLE_SRT);
        std::fs::write(subdir2.join("nested.srt"), SAMPLE_SRT).unwrap();
        let recursive = conv.convert_batch(dir2.path(), "vtt", true).await.unwrap();
        assert_eq!(recursive.len(), 2);
    }

    // ── convert_batch: VTT files ─────────────────────────────────────────────

    #[tokio::test]
    async fn convert_batch_converts_vtt_files_to_srt() {
        let dir = TempDir::new().unwrap();
        write_temp_file(&dir, "a.vtt", SAMPLE_VTT);

        let conv = FormatConverter::new(default_config());
        let results = conv.convert_batch(dir.path(), "srt", false).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(results[0].output_format, "srt");
    }
}
