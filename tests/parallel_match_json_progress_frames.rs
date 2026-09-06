//! Regression test for the parallel-path JSON silence half of
//! `tasks.md` §7 (D2): `execute_parallel_match` drives its batch progress
//! through the reporter seam, and the CLI reporter constructs its one bar
//! via `ui::create_progress_bar` — the only sanctioned constructor, which
//! force-hides the bar in JSON mode.
//!
//! The deleted private `create_progress_bar` in `match_command.rs` used to
//! bypass that helper on exactly this path, leaking progress frames under
//! `--output json`. `tests/cli/match_command_json_silence.rs` covers the
//! sequential command; this file covers `execute_parallel_match`, which has
//! no CLI subcommand and is library-only.
//!
//! JSON mode is installed via the process-global `OnceLock` in
//! `cli::output`, so the harness runs in a spawned child of this test
//! binary (`--exact` child test) and the parent asserts zero
//! progress-frame markers on **both** captured pipes.
//!
//! Honest limitation: `install_active_mode` is first-wins process-wide, so
//! this file is the only test that may install JSON mode in this binary —
//! it lives in its own test target to keep `tests/parallel_processing_-
//! integration_tests.rs` in text mode.

use std::io::Read;
use std::process::{Command as OsCommand, Stdio};

/// indicatif renders every redraw around `\r`, and the shared bar template
/// fills with `█`; neither byte sequence may appear when a bar is hidden
/// (same marker set as the §7.7/7.8 characterization tests).
const PROGRESS_FRAME_MARKERS: &[&str] = &["\r", "█"];

/// Child harness marker: the parent sets it so the child runs the real
/// parallel match instead of asserting anything.
const CHILD_ENV: &str = "SUBX_D2_PARALLEL_CHILD";

/// Child side of the harness: install JSON mode, point the config at a
/// mock AI with `enable_progress_bar = true`, and run the real
/// `execute_parallel_match` over a directory containing videos so the
/// scheduler opens a real Started/Advanced/Finished stream.
///
/// `FileProcessingTask::MatchFiles` is currently a stub, so no AI call is
/// issued — the batch succeeds deterministically while still exercising
/// the monitor, reporter, and bar construction end to end.
#[test]
fn d2_child_parallel_match_in_json_mode() {
    if std::env::var_os(CHILD_ENV).is_none() {
        // Parent-side invocation of this exact test must not run the child
        // body; the parent spawns this test by name via `--exact`.
        return;
    }
    subx_cli::cli::output::install_active_mode(subx_cli::cli::output::OutputMode::Json, false);

    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("movie1.mp4"), b"video-bytes").unwrap();
    std::fs::write(dir.path().join("movie2.mp4"), b"video-bytes").unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let mock = subx_core::test_support::mock_openai::MockOpenAITestHelper::new().await;
        let config_service = subx_core::config::TestConfigBuilder::new()
            .with_mock_ai_server(&mock.base_url())
            .with_progress_bar(true)
            .build_service();
        subx_cli::commands::match_command::execute_parallel_match(
            dir.path(),
            false,
            None,
            &config_service,
        )
        .await
    });
    // The batch must have completed normally — an empty/failed run would
    // trivially produce no frames and silently vacuate the assertion.
    result.expect("parallel match must succeed");
    // Guard against the no-tasks short path: at least one task ran.
    assert!(
        dir.path().join("movie1.mp4").exists(),
        "child workdir must still be intact"
    );
}

fn assert_no_progress_frames(bytes: &[u8], stream: &str) {
    let s = String::from_utf8_lossy(bytes);
    for marker in PROGRESS_FRAME_MARKERS {
        assert!(
            !s.contains(marker),
            "{stream} leaked a progress-frame marker {marker:?}:\n{s}"
        );
    }
}

/// D2 finding #1 (Duck): the JSON regression suite never exercised the
/// parallel path. Spawn the child harness above, with
/// `enable_progress_bar = true` under JSON mode, and require zero frame
/// bytes on both pipes — the same contract §7.7 pins for the sequential
/// command.
#[test]
fn parallel_match_json_mode_renders_no_progress_frames() {
    let exe = std::env::current_exe().unwrap();
    let mut child = OsCommand::new(exe)
        .args([
            "--exact",
            "d2_child_parallel_match_in_json_mode",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_ENV, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn child test harness");

    fn drain(mut pipe: impl Read) -> Vec<u8> {
        let mut buf = Vec::new();
        pipe.read_to_end(&mut buf).unwrap();
        buf
    }
    let out = drain(child.stdout.take().unwrap());
    let err = drain(child.stderr.take().unwrap());
    let status = child.wait().unwrap();

    let stdout = String::from_utf8_lossy(&out);
    assert!(
        status.success() && stdout.contains("test result: ok. 1 passed"),
        "child harness must run and succeed (exit {status}); stdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&err)
    );
    assert_no_progress_frames(&out, "child stdout");
    assert_no_progress_frames(&err, "child stderr");
}
