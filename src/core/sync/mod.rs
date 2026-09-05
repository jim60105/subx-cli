//! Refactored sync module focused on VAD (Voice Activity Detection).
//!
//! Provides unified subtitle synchronization functionality using local
//! VAD (Voice Activity Detection) for voice detection and sync offset calculation.
//!
//! # Core Components
//!
//! - [`SyncEngine`] - VAD-based sync engine
//! - [`SyncMethod`] - Sync method enumeration (VAD and manual)
//! - [`SyncResult`] - Sync result structure containing offset and confidence
//!
//! # Usage
//!
//! ```no_run
//! use subx_cli::core::sync::{SyncEngine, SyncMethod};
//! use subx_cli::config::SyncConfig;
//! use std::path::Path;
//! use subx_cli::core::formats::{Subtitle, SubtitleFormatType, SubtitleMetadata};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let engine = SyncEngine::new(SyncConfig::default())?;
//! let video_path = Path::new("video.mp4");
//! let metadata = SubtitleMetadata::new(SubtitleFormatType::Srt);
//! let subtitle = Subtitle::new(SubtitleFormatType::Srt, metadata);
//! let result = engine.detect_sync_offset(video_path, &subtitle, Some(SyncMethod::LocalVad)).await?;
//! # Ok(())
//! # }
//! ```

pub mod engine;

// Re-export main types
pub use engine::{MethodSelectionStrategy, SyncEngine, SyncMethod, SyncResult};

use crate::core::input::InputPathHandler;
use crate::error::SubXError;
use std::path::{Path, PathBuf};

/// Video container extensions recognised when auto-pairing a sync input.
///
/// This is the single definition used by both the pairing resolver
/// ([`resolve_sync_pairing`]) and the CLI's input-handler construction
/// (`SyncArgs::get_input_handler`), so the two cannot drift apart.
pub const SYNC_VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov"];

/// Subtitle extensions recognised when auto-pairing a sync input.
///
/// This is the single definition used by both the pairing resolver
/// ([`resolve_sync_pairing`]) and the CLI's input-handler construction
/// (`SyncArgs::get_input_handler`), so the two cannot drift apart.
pub const SYNC_SUBTITLE_EXTENSIONS: &[&str] = &["srt", "ass", "vtt", "sub"];

/// How the caller requested batch processing.
///
/// Parser-agnostic replacement for clap's `Option<Option<PathBuf>>` encoding
/// of `--batch [DIR]` (`num_args = 0..=1`): `Off` is "flag absent", `Auto` is
/// "flag present without a value", `Directory` is "flag present with a value".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BatchRequest {
    /// Batch not requested.
    #[default]
    Off,
    /// Batch requested without an explicit directory.
    Auto,
    /// Batch requested for a specific directory.
    Directory(PathBuf),
}

/// Parser-agnostic description of one `sync` invocation's inputs.
///
/// [`resolve_sync_pairing`] consumes this instead of a clap argument struct,
/// so mode resolution and auto-pairing are callable without any
/// argument-parsing type. Derives [`Default`] so callers fill only the fields
/// their case needs.
#[derive(Debug, Clone, Default)]
pub struct SyncPairingRequest {
    /// Positional file or directory paths in invocation order.
    pub positional_paths: Vec<PathBuf>,
    /// Paths supplied via repeated `-i` arguments.
    pub input_paths: Vec<PathBuf>,
    /// Explicit `--video` path.
    pub video: Option<PathBuf>,
    /// Explicit `--subtitle` path.
    pub subtitle: Option<PathBuf>,
    /// How batch processing was requested.
    pub batch: BatchRequest,
    /// Whether directory scanning is recursive.
    pub recursive: bool,
    /// Whether archive extraction is disabled.
    pub no_extract: bool,
    /// Manual-offset mode: a video is not required.
    pub manual: bool,
}

/// Sync mode: single file or batch.
#[derive(Debug)]
pub enum SyncMode {
    /// Single file sync mode, specify video and subtitle files
    Single {
        /// Video file path
        video: PathBuf,
        /// Subtitle file path
        subtitle: PathBuf,
    },
    /// Batch sync mode, using InputPathHandler to process multiple paths
    Batch(InputPathHandler),
}

/// Decide whether a sync invocation is a single pair or a batch, auto-pairing
/// a lone video or subtitle with its sibling on disk.
///
/// Algorithm (locked by the `timeline-sync` capability spec):
/// 1. Any batch trigger (`batch != Off`, non-empty `input_paths`, or an
///    extension-less positional path) selects [`SyncMode::Batch`], with the
///    handler's paths ordered batch directory → `-i` paths → positionals and
///    defaulting to `["."]` when empty.
/// 2. A lone positional path is classified by its lower-cased extension and
///    probed for a `<stem>.<ext>` sibling over the declared extension lists.
/// 3. Two positional paths are classified by extension without probing.
/// 4. Otherwise the explicit `video`/`subtitle` fields are used.
/// 5. In manual mode a resolved subtitle with no video yields
///    [`SyncMode::Single`] with an empty [`PathBuf`] video ("no video
///    required" sentinel).
/// 6. Anything else returns [`SubXError::InvalidSyncConfiguration`].
pub fn resolve_sync_pairing(request: &SyncPairingRequest) -> Result<SyncMode, SubXError> {
    // Batch mode: process directories or multiple inputs when -b, -i, or directory positional used
    if request.batch != BatchRequest::Off
        || !request.input_paths.is_empty()
        || request
            .positional_paths
            .iter()
            .any(|p| p.extension().is_none())
    {
        let mut paths = Vec::new();

        // Include batch directory argument if provided
        if let BatchRequest::Directory(batch_dir) = &request.batch {
            paths.push(batch_dir.clone());
        }

        // Include input paths (-i) and any positional paths
        paths.extend(request.input_paths.clone());
        paths.extend(request.positional_paths.clone());

        // If still no paths, use current directory
        if paths.is_empty() {
            paths.push(PathBuf::from("."));
        }

        let handler = InputPathHandler::from_args(&paths, request.recursive)?
            .with_extensions(&[SYNC_VIDEO_EXTENSIONS, SYNC_SUBTITLE_EXTENSIONS].concat())
            .with_no_extract(request.no_extract);

        return Ok(SyncMode::Batch(handler));
    }

    // Single file positional mode: auto-infer video/subtitle pairing
    if !request.positional_paths.is_empty() {
        if request.positional_paths.len() == 1 {
            let path = &request.positional_paths[0];
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            let mut video = None;
            let mut subtitle = None;
            if SYNC_VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                video = Some(path.clone());
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let dir = path.parent().unwrap_or_else(|| Path::new("."));
                    for sub_ext in SYNC_SUBTITLE_EXTENSIONS {
                        let cand = dir.join(format!("{stem}.{sub_ext}"));
                        if cand.exists() {
                            subtitle = Some(cand);
                            break;
                        }
                    }
                }
            } else if SYNC_SUBTITLE_EXTENSIONS.contains(&ext.as_str()) {
                subtitle = Some(path.clone());
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let dir = path.parent().unwrap_or_else(|| Path::new("."));
                    for vid_ext in SYNC_VIDEO_EXTENSIONS {
                        let cand = dir.join(format!("{stem}.{vid_ext}"));
                        if cand.exists() {
                            video = Some(cand);
                            break;
                        }
                    }
                }
            }
            // For manual mode, we don't need video file if we have subtitle
            if request.manual {
                if let Some(subtitle_path) = subtitle {
                    return Ok(SyncMode::Single {
                        video: PathBuf::new(), // Empty video path for manual mode
                        subtitle: subtitle_path,
                    });
                }
            }
            if let (Some(v), Some(s)) = (video, subtitle) {
                return Ok(SyncMode::Single {
                    video: v,
                    subtitle: s,
                });
            }
            return Err(SubXError::InvalidSyncConfiguration);
        } else if request.positional_paths.len() == 2 {
            let mut video = None;
            let mut subtitle = None;
            for p in &request.positional_paths {
                if let Some(ext) = p
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase())
                {
                    if SYNC_VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                        video = Some(p.clone());
                    }
                    if SYNC_SUBTITLE_EXTENSIONS.contains(&ext.as_str()) {
                        subtitle = Some(p.clone());
                    }
                }
            }
            if let (Some(v), Some(s)) = (video, subtitle) {
                return Ok(SyncMode::Single {
                    video: v,
                    subtitle: s,
                });
            }
            return Err(SubXError::InvalidSyncConfiguration);
        }
    }

    // Explicit mode: video and subtitle options
    if let (Some(video), Some(subtitle)) = (request.video.as_ref(), request.subtitle.as_ref()) {
        Ok(SyncMode::Single {
            video: video.clone(),
            subtitle: subtitle.clone(),
        })
    } else if request.manual {
        if let Some(subtitle) = request.subtitle.as_ref() {
            // Manual mode only requires subtitle file
            Ok(SyncMode::Single {
                video: PathBuf::new(), // Empty video path for manual mode
                subtitle: subtitle.clone(),
            })
        } else {
            Err(SubXError::InvalidSyncConfiguration)
        }
    } else {
        Err(SubXError::InvalidSyncConfiguration)
    }
}

/// Creates a default output path by appending `_synced` to the file stem.
///
/// # Arguments
///
/// * `input` - The input subtitle path
///
/// # Returns
///
/// The input path with its file name replaced by `<stem>_synced.<extension>`,
/// or the input path unchanged when it has no file stem or no extension.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use subx_cli::core::sync::create_default_output_path;
///
/// assert_eq!(
///     create_default_output_path(&PathBuf::from("subs/movie.srt")),
///     PathBuf::from("subs/movie_synced.srt")
/// );
/// assert_eq!(
///     create_default_output_path(&PathBuf::from("noextension")),
///     PathBuf::from("noextension")
/// );
/// ```
pub fn create_default_output_path(input: &Path) -> PathBuf {
    let mut output = input.to_path_buf();

    if let Some(stem) = input.file_stem().and_then(|s| s.to_str()) {
        if let Some(extension) = input.extension().and_then(|s| s.to_str()) {
            let new_filename = format!("{stem}_synced.{extension}");
            output.set_file_name(new_filename);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── create_default_output_path ───────────────────────────────────────

    #[test]
    fn test_create_default_output_path_srt() {
        let input = PathBuf::from("test.srt");
        let output = create_default_output_path(&input);
        assert_eq!(output.file_name().unwrap(), "test_synced.srt");
    }

    #[test]
    fn test_create_default_output_path_with_prefix() {
        let input = PathBuf::from("/path/to/movie.ass");
        let output = create_default_output_path(&input);
        assert_eq!(output.file_name().unwrap(), "movie_synced.ass");
        assert_eq!(output.parent().unwrap(), std::path::Path::new("/path/to"));
    }

    #[test]
    fn test_create_default_output_path_vtt() {
        let input = PathBuf::from("episode.vtt");
        let output = create_default_output_path(&input);
        assert_eq!(output.file_name().unwrap(), "episode_synced.vtt");
    }

    #[test]
    fn test_create_default_output_path_no_extension() {
        // File without extension: stem exists but extension does not; path returned unchanged
        let input = PathBuf::from("noextension");
        let output = create_default_output_path(&input);
        assert_eq!(output, PathBuf::from("noextension"));
    }

    // ── resolve_sync_pairing (re-pointed from the SyncArgs characterisation
    //    set, task 5.2) ────────────────────────────────────────────────────

    #[test]
    fn test_resolve_pairing_single_positional_video_probes_subtitle() {
        let tmp = tempfile::TempDir::new().unwrap();
        let video = tmp.path().join("movie.mp4");
        let sub = tmp.path().join("movie.srt");
        std::fs::write(&video, b"fake video").unwrap();
        std::fs::write(&sub, b"1\n00:00:01,000 --> 00:00:02,000\nHi\n").unwrap();

        let request = SyncPairingRequest {
            positional_paths: vec![video.clone()],
            ..Default::default()
        };
        match resolve_sync_pairing(&request).unwrap() {
            SyncMode::Single {
                video: v,
                subtitle: s,
            } => {
                assert_eq!(v, video);
                assert_eq!(s, sub);
            }
            other => panic!("Expected Single mode, got {other:?}"),
        }
    }

    #[test]
    fn test_resolve_pairing_single_positional_subtitle_probes_video() {
        let tmp = tempfile::TempDir::new().unwrap();
        let video = tmp.path().join("movie.mp4");
        let sub = tmp.path().join("movie.srt");
        std::fs::write(&video, b"fake video").unwrap();
        std::fs::write(&sub, b"1\n00:00:01,000 --> 00:00:02,000\nHi\n").unwrap();

        let request = SyncPairingRequest {
            positional_paths: vec![sub.clone()],
            ..Default::default()
        };
        match resolve_sync_pairing(&request).unwrap() {
            SyncMode::Single {
                video: v,
                subtitle: s,
            } => {
                assert_eq!(v, video);
                assert_eq!(s, sub);
            }
            other => panic!("Expected Single mode, got {other:?}"),
        }
    }

    #[test]
    fn test_resolve_pairing_probe_order_prefers_srt_over_ass() {
        let tmp = tempfile::TempDir::new().unwrap();
        let video = tmp.path().join("movie.mp4");
        let srt = tmp.path().join("movie.srt");
        let ass = tmp.path().join("movie.ass");
        std::fs::write(&video, b"fake video").unwrap();
        std::fs::write(&srt, b"1\n00:00:01,000 --> 00:00:02,000\nHi\n").unwrap();
        std::fs::write(&ass, b"[Script Info]\n").unwrap();

        let request = SyncPairingRequest {
            positional_paths: vec![video],
            ..Default::default()
        };
        match resolve_sync_pairing(&request).unwrap() {
            SyncMode::Single { subtitle, .. } => assert_eq!(subtitle, srt),
            other => panic!("Expected Single mode, got {other:?}"),
        }
    }

    #[test]
    fn test_resolve_pairing_manual_mode_subtitle_only_positional_empty_video() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sub = tmp.path().join("movie.srt");
        std::fs::write(&sub, b"1\n00:00:01,000 --> 00:00:02,000\nHi\n").unwrap();

        let request = SyncPairingRequest {
            positional_paths: vec![sub.clone()],
            manual: true,
            ..Default::default()
        };
        match resolve_sync_pairing(&request).unwrap() {
            SyncMode::Single { video, subtitle } => {
                assert_eq!(video, PathBuf::new());
                assert_eq!(subtitle, sub);
            }
            other => panic!("Expected Single mode, got {other:?}"),
        }
    }

    #[test]
    fn test_resolve_pairing_two_positionals_classified_without_probing() {
        // Neither file needs to exist: two positionals are classified purely
        // by extension, with no filesystem probing.
        let request = SyncPairingRequest {
            positional_paths: vec![
                PathBuf::from("nowhere/movie.srt"),
                PathBuf::from("nowhere/movie.mp4"),
            ],
            ..Default::default()
        };
        match resolve_sync_pairing(&request).unwrap() {
            SyncMode::Single { video, subtitle } => {
                assert_eq!(video, PathBuf::from("nowhere/movie.mp4"));
                assert_eq!(subtitle, PathBuf::from("nowhere/movie.srt"));
            }
            other => panic!("Expected Single mode, got {other:?}"),
        }
    }

    #[test]
    fn test_resolve_pairing_unpairable_single_positional_errors() {
        let tmp = tempfile::TempDir::new().unwrap();
        let video = tmp.path().join("movie.mp4");
        std::fs::write(&video, b"fake video").unwrap();
        // No subtitle sibling on disk, non-manual mode.
        let request = SyncPairingRequest {
            positional_paths: vec![video],
            ..Default::default()
        };
        assert!(matches!(
            resolve_sync_pairing(&request),
            Err(SubXError::InvalidSyncConfiguration)
        ));
    }

    // ── batch selection (task 5.3) ───────────────────────────────────────

    #[test]
    fn test_resolve_pairing_batch_directory() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();
        // from_args validates existence of every path it receives.
        std::fs::write(dir.join("extra"), b"x").unwrap();
        std::fs::write(dir.join("pos.srt"), b"x").unwrap();
        let request = SyncPairingRequest {
            batch: BatchRequest::Directory(dir.clone()),
            input_paths: vec![dir.join("extra")],
            positional_paths: vec![dir.join("pos.srt")],
            recursive: true,
            no_extract: true,
            ..Default::default()
        };
        match resolve_sync_pairing(&request).unwrap() {
            SyncMode::Batch(handler) => {
                // Ordered: batch directory, then -i paths, then positionals.
                assert_eq!(
                    handler.paths,
                    vec![dir.clone(), dir.join("extra"), dir.join("pos.srt")]
                );
                assert!(handler.recursive);
                assert!(handler.no_extract);
                let mut expected_ext: Vec<String> = SYNC_VIDEO_EXTENSIONS
                    .iter()
                    .chain(SYNC_SUBTITLE_EXTENSIONS)
                    .map(|s| s.to_string())
                    .collect();
                expected_ext.sort();
                let mut actual_ext = handler.file_extensions.clone();
                actual_ext.sort();
                assert_eq!(actual_ext, expected_ext);
            }
            other => panic!("Expected Batch mode, got {other:?}"),
        }
    }

    #[test]
    fn test_resolve_pairing_batch_via_input_paths() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();
        let request = SyncPairingRequest {
            input_paths: vec![dir],
            ..Default::default()
        };
        assert!(matches!(
            resolve_sync_pairing(&request).unwrap(),
            SyncMode::Batch(_)
        ));
    }

    #[test]
    fn test_resolve_pairing_batch_via_extensionless_positional() {
        let tmp = tempfile::TempDir::new().unwrap();
        let request = SyncPairingRequest {
            positional_paths: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };
        assert!(matches!(
            resolve_sync_pairing(&request).unwrap(),
            SyncMode::Batch(_)
        ));
    }

    #[test]
    fn test_resolve_pairing_batch_auto_without_paths_defaults_to_cwd() {
        let request = SyncPairingRequest {
            batch: BatchRequest::Auto,
            ..Default::default()
        };
        match resolve_sync_pairing(&request).unwrap() {
            SyncMode::Batch(handler) => {
                assert_eq!(handler.paths, vec![PathBuf::from(".")]);
            }
            other => panic!("Expected Batch mode, got {other:?}"),
        }
    }
}
