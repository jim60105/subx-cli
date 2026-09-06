//! Characterisation tests for the `Warning: Skipping relocation due to
//! existing file:` message emitted by `MatchEngine::resolve_filename_conflict`
//! during `subx-cli cache apply` (tasks A1 §1.2, as redirected by the plan
//! critique).
//!
//! `subx-cli match` hardcodes `ConflictResolution::AutoRename`, so the Skip
//! branch is only reachable from the CLI through `cache apply`, which builds
//! a `MatchConfig` with `ConflictResolution::Skip`
//! (`src/commands/cache_command.rs`). The test seeds a `match_cache.json`
//! whose relocation target already exists on disk and applies it with
//! `--force`, driving the branch end-to-end.
//!
//! Written against **unmodified** source; must pass unchanged after the
//! `Reporter` rewiring. Pinning `cache apply` here matters because the
//! helper `apply_cached_operations` builds its own `MatchEngine` — if that
//! engine does not receive the terminal reporter, text mode silently loses
//! this warning.
//!
//! Wired into the test crate via `tests/cache_apply_conflict_warning_tests.rs`
//! (Cargo does not discover `tests/cli/`).

use assert_cmd::Command;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:02,000\nHello world\n\n";

/// Seed a workdir whose cache relocations would overwrite an existing
/// target: video in `videos/`, subtitle at the root, and the destination
/// `videos/Movie.srt` already present.
fn build_apply_workdir() -> TempDir {
    let dir = TempDir::new().unwrap();
    let videos = dir.path().join("videos");
    fs::create_dir_all(&videos).unwrap();
    fs::write(videos.join("Movie.mp4"), b"fake video bytes").unwrap();
    fs::write(dir.path().join("random.srt"), SAMPLE_SRT).unwrap();
    // Pre-existing relocation target — forces the conflict Skip branch.
    fs::write(videos.join("Movie.srt"), b"existing content\n").unwrap();

    let xdg = dir.path().join(".xdg");
    let subx = xdg.join("subx");
    fs::create_dir_all(&subx).unwrap();
    let video_abs = videos.join("Movie.mp4").to_string_lossy().into_owned();
    let subtitle_abs = dir.path().join("random.srt").to_string_lossy().into_owned();
    let cache = json!({
        "cache_version": "2.0",
        "directory": dir.path().to_string_lossy(),
        "file_snapshot": [],
        "match_operations": [{
            "video_file": video_abs,
            "subtitle_file": subtitle_abs,
            "new_subtitle_name": "Movie.srt",
            "confidence": 0.95,
            "reasoning": ["seeded"],
        }],
        "created_at": 1_700_000_000u64,
        "ai_model_used": "seed-model",
        // --force bypasses hash/snapshot/target checks; value is irrelevant.
        "config_hash": "deadbeef",
        "original_relocation_mode": "Copy",
        "original_backup_enabled": false,
    });
    fs::write(subx.join("match_cache.json"), cache.to_string()).unwrap();
    dir
}

fn run_cache_apply(workdir: &std::path::Path, global_args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_subx-cli"));
    cmd.env("SUBX_AI_PROVIDER", "local")
        .env("LOCAL_LLM_API_KEY", "sk-mock-test-key")
        .env("LOCAL_LLM_BASE_URL", "http://127.0.0.1:9")
        .env("XDG_CONFIG_HOME", workdir.join(".xdg"))
        .env("HOME", workdir)
        .env("SUBX_GENERAL_ENABLE_PROGRESS_BAR", "false")
        .args(global_args)
        .args(["cache", "apply", "--force", "--yes"])
        .current_dir(workdir)
        .timeout(std::time::Duration::from_secs(60));
    cmd.assert()
}

const WARNING: &str = "Warning: Skipping relocation due to existing file:";

/// A1 §1.2 — text mode: the conflict warning reaches stderr byte-prefixed
/// as today, and never stdout.
#[test]
fn text_mode_cache_apply_prints_skip_warning_on_stderr() {
    let dir = build_apply_workdir();
    let assert = run_cache_apply(dir.path(), &[]).success();
    let out = assert.get_output();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();

    assert!(
        stderr.contains(WARNING),
        "text-mode stderr missing the skip warning:\n{stderr}"
    );
    assert!(
        stderr.contains(
            &dir.path()
                .join("videos")
                .join("Movie.srt")
                .display()
                .to_string()
        ),
        "warning missing the target path:\n{stderr}"
    );
    assert!(
        !stdout.contains(WARNING),
        "skip warning leaked to stdout:\n{stdout}"
    );
}

/// A1 §1.2 — `--quiet` does NOT silence the engine warning in text mode.
#[test]
fn quiet_text_mode_cache_apply_still_prints_skip_warning() {
    let dir = build_apply_workdir();
    let assert = run_cache_apply(dir.path(), &["--quiet"]).success();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(
        stderr.contains(WARNING),
        "--quiet must not silence engine warnings:\n{stderr}"
    );
}

/// A1 §1.2 — `--output json` suppresses the warning on both streams.
#[test]
fn json_mode_cache_apply_prints_no_skip_warning() {
    let dir = build_apply_workdir();
    let assert = run_cache_apply(dir.path(), &["--output", "json"]).success();
    let out = assert.get_output();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        !stderr.contains(WARNING) && !stdout.contains(WARNING),
        "--output json must suppress the skip warning:\nstdout:{stdout}\nstderr:{stderr}"
    );
}
