//! Compile-level guarantee that the legacy `subx_cli::cli` compatibility
//! re-exports introduced by `relocate-misplaced-core-modules` keep resolving
//! to the core types.
//!
//! The out-of-tree Tauri GUI imports exactly these paths
//! (`subx_cli::cli::{CollectedFiles, InputPathHandler}` and
//! `subx_cli::cli::sync_args::create_default_output_path`); they stay
//! available — without deprecation warnings — until the GUI migrates to the
//! `subx_core::` paths after the crate split. The aliases are retired only
//! at that point, not here.

use subx_cli::cli::sync_args::create_default_output_path;
use subx_cli::cli::{CollectedFiles, InputPathHandler, SyncMode};

#[test]
fn legacy_cli_paths_resolve_to_core_types() {
    use std::path::PathBuf;

    // The aliases name the very same types as the core paths.
    fn assert_input_core(_: &CollectedFiles, _: &InputPathHandler) {}
    let _: fn(&subx_cli::core::input::CollectedFiles, &subx_cli::core::input::InputPathHandler) =
        assert_input_core;

    let mode: SyncMode =
        subx_cli::core::sync::resolve_sync_pairing(&subx_cli::core::sync::SyncPairingRequest {
            video: Some(PathBuf::from("v.mp4")),
            subtitle: Some(PathBuf::from("s.srt")),
            ..Default::default()
        })
        .unwrap();
    assert!(matches!(mode, SyncMode::Single { .. }));

    assert_eq!(
        create_default_output_path(std::path::Path::new("subs/movie.srt")),
        PathBuf::from("subs/movie_synced.srt")
    );
}
