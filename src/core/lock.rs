//! Exclusive file-lock infrastructure for SubX operations.
//!
//! This module provides [`acquire_subx_lock`], an async helper that acquires
//! an exclusive advisory file lock on `$CONFIG_DIR/subx/subx.lock` with a
//! 2-second timeout. The returned [`SubxLockGuard`] releases the lock on drop.
//!
//! All commands that mutate `match_cache.json` or `match_journal.json` must
//! acquire this lock before proceeding (Design Decision D9).

use crate::Result;
use crate::core::matcher::journal::lock_path;
use crate::error::SubXError;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// RAII guard that holds an exclusive file lock on the SubX lock file.
///
/// Dropping this guard releases the lock via [`std::fs::File::unlock`].
#[derive(Debug)]
pub struct SubxLockGuard {
    file: std::fs::File,
}

impl Drop for SubxLockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Acquire the SubX exclusive file lock with a 2-second timeout.
///
/// Creates/opens `$CONFIG_DIR/subx/subx.lock` and attempts a non-blocking
/// exclusive lock ([`std::fs::File::try_lock`]), retrying every 100ms for up
/// to 2 seconds. Returns a [`SubxLockGuard`] on success. If the lock cannot
/// be acquired within the timeout, returns an error with a diagnostic message.
pub async fn acquire_subx_lock() -> Result<SubxLockGuard> {
    let path = lock_path()?;
    tokio::task::spawn_blocking(move || acquire_lock_blocking(&path))
        .await
        .map_err(|e| SubXError::Io(std::io::Error::other(e.to_string())))?
}

fn acquire_lock_blocking(path: &PathBuf) -> Result<SubxLockGuard> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)?;

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(SubxLockGuard { file }),
            Err(std::fs::TryLockError::WouldBlock) => {}
            Err(e) => return Err(SubXError::Io(e.into())),
        }
        if Instant::now() >= deadline {
            return Err(SubXError::config(format!(
                "Another SubX operation is in progress. \
                 Please wait for it to finish or terminate the other process. \
                 Lock file: {}",
                path.display()
            )));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn acquire_lock_succeeds_when_uncontended() {
        let tmp = TempDir::new().unwrap();
        unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path()) };
        let guard = acquire_subx_lock().await;
        assert!(guard.is_ok());
        drop(guard);
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
    }

    #[tokio::test]
    async fn lock_is_released_on_drop() {
        let tmp = TempDir::new().unwrap();
        unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path()) };
        {
            let _g1 = acquire_subx_lock().await.unwrap();
        }
        // Second acquire should succeed immediately after drop
        let g2 = acquire_subx_lock().await;
        assert!(g2.is_ok());
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
    }

    #[tokio::test]
    async fn contention_produces_timeout_error() {
        let tmp = TempDir::new().unwrap();
        let lock_file = tmp.path().join("subx").join("subx.lock");
        std::fs::create_dir_all(lock_file.parent().unwrap()).unwrap();

        // Hold lock via std file locking
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_file)
            .unwrap();
        file.lock().unwrap();

        unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path()) };
        let start = Instant::now();
        let result = acquire_subx_lock().await;
        let elapsed = start.elapsed();

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Another SubX operation is in progress"));
        assert!(elapsed >= Duration::from_secs(2));

        file.unlock().unwrap();
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
    }
}
