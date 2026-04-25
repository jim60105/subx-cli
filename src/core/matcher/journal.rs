//! Transactional journal for file relocation operations.
//!
//! The journal records every file operation (rename, copy, move) performed
//! during a match batch so that the action can be reliably resumed or
//! rolled back if the process is interrupted. Entries are persisted to
//! disk using an atomic write-then-rename strategy to guarantee that the
//! on-disk journal is never left in a partially written state.

use crate::Result;
use crate::error::SubXError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Status of an individual journal entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JournalEntryStatus {
    /// The operation has been recorded but not yet executed on disk.
    Pending,
    /// The operation has been successfully executed on disk.
    Completed,
}

/// The type of file system operation described by a journal entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JournalOperationType {
    /// A rename (in-place) of the source file to a new name.
    Renamed,
    /// A copy from source to destination preserving the source.
    Copied,
    /// A move of the source file to a new directory or name.
    Moved,
}

/// A single recorded file operation within a journal batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    /// The type of file system operation performed.
    pub operation_type: JournalOperationType,
    /// Absolute path of the original source file.
    pub source: PathBuf,
    /// Absolute path of the destination file.
    pub destination: PathBuf,
    /// Optional path to a backup of the source file, if a backup was created.
    pub backup_path: Option<PathBuf>,
    /// Current execution status of this entry.
    pub status: JournalEntryStatus,
    /// Size of the source file in bytes at the time of recording.
    pub file_size: u64,
    /// Modification time of the source file as Unix epoch seconds.
    pub file_mtime: u64,
}

/// Persistent journal data describing a single match batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalData {
    /// Unique identifier for this batch of operations.
    pub batch_id: String,
    /// Creation timestamp as Unix epoch seconds.
    pub created_at: u64,
    /// Recorded operations belonging to this batch.
    pub entries: Vec<JournalEntry>,
}

impl JournalData {
    /// Atomically persist this journal to `path`.
    ///
    /// The data is first serialized as pretty JSON, written to a
    /// sibling temporary file, flushed and fsynced, and finally renamed
    /// into place. Parent directories are created on demand. All
    /// blocking I/O is executed inside [`tokio::task::spawn_blocking`].
    pub async fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let path = path.to_path_buf();

        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            use std::io::Write;

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let tmp_path = match path.file_name() {
                Some(name) => {
                    let mut tmp_name = std::ffi::OsString::from(".");
                    tmp_name.push(name);
                    tmp_name.push(".tmp");
                    path.with_file_name(tmp_name)
                }
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "journal path has no file name",
                    ));
                }
            };

            {
                let mut file = std::fs::File::create(&tmp_path)?;
                file.write_all(json.as_bytes())?;
                file.flush()?;
                file.sync_all()?;
            }

            std::fs::rename(&tmp_path, &path)?;

            if let Some(parent) = path.parent() {
                if let Ok(dir) = std::fs::File::open(parent) {
                    let _ = dir.sync_all();
                }
            }

            Ok(())
        })
        .await
        .map_err(|e| SubXError::Io(std::io::Error::other(e.to_string())))??;

        Ok(())
    }

    /// Load and deserialize a journal from `path`.
    pub async fn load(path: &Path) -> Result<Self> {
        let path = path.to_path_buf();
        let content = tokio::task::spawn_blocking(move || std::fs::read_to_string(&path))
            .await
            .map_err(|e| SubXError::Io(std::io::Error::other(e.to_string())))??;
        let data: JournalData = serde_json::from_str(&content)?;
        Ok(data)
    }
}

/// Resolve the canonical path to the match journal file.
///
/// Honors `XDG_CONFIG_HOME` when set (used in tests), otherwise falls back
/// to the platform-specific user configuration directory.
pub fn journal_path() -> Result<PathBuf> {
    let dir = if let Some(xdg_config) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config)
    } else {
        dirs::config_dir()
            .ok_or_else(|| SubXError::config("Unable to determine config directory"))?
    };
    Ok(dir.join("subx").join("match_journal.json"))
}

/// Resolve the canonical path to the match batch lock file.
///
/// Used to serialize concurrent invocations that modify the journal.
pub fn lock_path() -> Result<PathBuf> {
    let dir = if let Some(xdg_config) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config)
    } else {
        dirs::config_dir()
            .ok_or_else(|| SubXError::config("Unable to determine config directory"))?
    };
    Ok(dir.join("subx").join("subx.lock"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_data() -> JournalData {
        JournalData {
            batch_id: "batch-123".to_string(),
            created_at: 1_700_000_000,
            entries: vec![
                JournalEntry {
                    operation_type: JournalOperationType::Renamed,
                    source: PathBuf::from("/a/old.srt"),
                    destination: PathBuf::from("/a/new.srt"),
                    backup_path: None,
                    status: JournalEntryStatus::Pending,
                    file_size: 1024,
                    file_mtime: 1_699_999_000,
                },
                JournalEntry {
                    operation_type: JournalOperationType::Copied,
                    source: PathBuf::from("/a/src.srt"),
                    destination: PathBuf::from("/b/dst.srt"),
                    backup_path: Some(PathBuf::from("/a/src.srt.bak")),
                    status: JournalEntryStatus::Completed,
                    file_size: 2048,
                    file_mtime: 1_699_999_500,
                },
            ],
        }
    }

    #[tokio::test]
    async fn save_and_load_roundtrip() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("journal.json");
        let data = sample_data();

        data.save(&path).await.expect("save");
        assert!(path.exists());

        let loaded = JournalData::load(&path).await.expect("load");
        assert_eq!(loaded, data);
    }

    #[tokio::test]
    async fn save_creates_parent_directories() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nested").join("deep").join("journal.json");
        let data = sample_data();

        data.save(&path).await.expect("save");
        assert!(path.exists());
    }

    #[tokio::test]
    async fn load_missing_file_returns_error() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("does-not-exist.json");
        let err = JournalData::load(&path).await.unwrap_err();
        assert!(matches!(err, SubXError::Io(_)));
    }

    #[tokio::test]
    async fn atomic_save_leaves_no_temp_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("journal.json");
        let data = sample_data();

        data.save(&path).await.expect("save");

        let entries: Vec<_> = std::fs::read_dir(temp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], "journal.json");
    }

    #[test]
    fn status_serializes_lowercase() {
        let json = serde_json::to_string(&JournalEntryStatus::Pending).unwrap();
        assert_eq!(json, "\"pending\"");
        let json = serde_json::to_string(&JournalEntryStatus::Completed).unwrap();
        assert_eq!(json, "\"completed\"");
    }

    #[test]
    fn operation_type_serializes_lowercase() {
        let json = serde_json::to_string(&JournalOperationType::Renamed).unwrap();
        assert_eq!(json, "\"renamed\"");
        let json = serde_json::to_string(&JournalOperationType::Copied).unwrap();
        assert_eq!(json, "\"copied\"");
        let json = serde_json::to_string(&JournalOperationType::Moved).unwrap();
        assert_eq!(json, "\"moved\"");
    }

    #[test]
    fn journal_and_lock_paths_end_with_expected_names() {
        let p = journal_path().unwrap();
        assert_eq!(p.file_name().unwrap(), "match_journal.json");
        assert_eq!(p.parent().unwrap().file_name().unwrap(), "subx");

        let l = lock_path().unwrap();
        assert_eq!(l.file_name().unwrap(), "subx.lock");
        assert_eq!(l.parent().unwrap().file_name().unwrap(), "subx");
    }
}
