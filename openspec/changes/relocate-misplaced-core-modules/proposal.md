## Why

Three pieces of pure domain logic are filed under `src/cli/` for no reason other than history, and one piece of terminal presentation is filed under `src/error.rs` for the same reason. All four are on the wrong side of the line that SDR §2.1–§2.3 draws between the future `subx-core` library and the `subx-cli` binary.

| Misplaced today | What it actually is | Evidence |
|---|---|---|
| `src/cli/input_handler.rs` (569 LOC) | Filesystem walking, symlink skipping, extension filtering, archive extraction, `TempDir` ownership | Its entire import list is `std`, `log`, `tempfile`, `crate::core::archive`, `crate::error`. **Zero** clap involvement. The Tauri GUI imports it as `subx_cli::cli::input_handler::{CollectedFiles, InputPathHandler}` and has never touched a clap type (SDR §8). |
| `SyncArgs::get_sync_mode`'s body (`src/cli/sync_args.rs:301-435`, ~135 LOC) plus the `SyncMode` enum (`:438-450`) | Filesystem probing that auto-pairs `video.mp4` ↔ `video.srt` by testing `cand.exists()` across `["mp4","mkv","avi","mov"]` / `["srt","ass","vtt","sub"]`, then decides Single vs Batch | The `self.` accesses are all plain `Option<PathBuf>` / `Vec<PathBuf>` fields. Nothing in the 135 lines needs `clap::Args`; the struct is a value carrier, not a parser, at that point. |
| `create_default_output_path` (`src/cli/sync_args.rs:455-466`) | A free function deriving `<stem>_synced.<ext>` | Already imported *out* of the CLI layer by `src/commands/sync_command.rs:10`, and by the GUI. |
| `SubXError::exit_code()` (`src/error.rs:1130`) and `SubXError::user_friendly_message()` (`:1248`) | Process exit codes and multi-line terminal prose with `Hint:` lines | Neither concept exists for a library consumer. The GUI calls `category()`, `hint()`, and `to_string()` — never these two. |

Leaving them where they are turns B2 (`move-core-sources-into-subx-core`) from a near-pure `git mv` into a simultaneous move-and-refactor across a new crate boundary, with `broken_intra_doc_links = "deny"` turning every stale doc link into a hard build failure at the worst possible moment. SDR §12 therefore schedules this work as **A2**, inside the existing single crate, where every symbol is still reachable by its old path and the compiler can verify each step in isolation.

There is also one genuine coupling defect that only shows up once the presentation split is drawn: `src/core/matcher/engine.rs:1255-1261` (`operation_error_from`) calls `err.user_friendly_message()` from *inside* core. That is a core → presentation edge that A1's `Reporter` seam does not cover, and it must be cut here.

## What Changes

**1. `src/cli/input_handler.rs` → `src/core/input/mod.rs`.**

- The file moves verbatim to `src/core/input/mod.rs`, exposed as `crate::core::input::{InputPathHandler, CollectedFiles}`. `pub mod input;` is added to `src/core/mod.rs` (`:19-30`).
- Every associated item moves with it unchanged: `InputPathHandler::{from_args, merge_paths_from_multiple_sources, with_extensions, with_no_extract, validate, get_directories, collect_files, extract_and_collect, matches_extension, scan_directory_flat, scan_directory_recursive}`, `CollectedFiles::{new, with_archives, archive_origin, into_paths}`, and the `Deref<Target = Vec<PathBuf>>` / `AsRef<[PathBuf]>` impls. The `symlink_tests` module (`:529-568`) moves with it.
- `src/cli/mod.rs:53` becomes a re-export shim: `pub use crate::core::input::{CollectedFiles, InputPathHandler};`, and `mod input_handler;` (`:37`) is deleted.
- The five rustdoc examples inside the file are rewritten from `subx_cli::cli::InputPathHandler` to `subx_cli::core::input::InputPathHandler`. The "Command Integration" example (`:107-129`) — which constructs a clap `MatchArgs` — is **deleted**, because a core-resident doctest may not reference a CLI type once B2 lands.
- The five in-crate `use crate::cli::InputPathHandler;` sites (`convert_args.rs:30`, `detect_encoding_args.rs:31`, `match_args.rs:4`, `translate_args.rs:23`, `sync_args.rs:36`) are repointed at `crate::core::input::InputPathHandler`, and `src/commands/translate_command.rs:325`'s parameter type `&crate::cli::CollectedFiles` becomes `&crate::core::input::CollectedFiles`.

**2. `create_default_output_path` → `crate::core::sync`.**

- The function moves verbatim from `src/cli/sync_args.rs:455-466` into `src/core/sync/mod.rs` and is re-exported there. Its four unit tests (`sync_args.rs:1068-1099`) move with it.
- `src/cli/sync_args.rs` keeps `pub use crate::core::sync::create_default_output_path;` so `SyncArgs::get_output_path` (`:227-235`) and `src/commands/sync_command.rs:10` keep resolving; `sync_command.rs` is repointed at the core path directly.

**3. `SyncMode` + the pairing body → `crate::core::sync::{SyncMode, SyncPairingRequest, BatchRequest, resolve_sync_pairing}`.**

- `SyncMode` (`sync_args.rs:438-450`) moves to `src/core/sync/mod.rs` unchanged, including `SyncMode::Batch(InputPathHandler)` — which now names a core type.
- The ~135-line body of `SyncArgs::get_sync_mode` (`sync_args.rs:301-435`) becomes `pub fn resolve_sync_pairing(request: &SyncPairingRequest) -> Result<SyncMode, SubXError>`, driven by a clap-free `SyncPairingRequest` struct (`positional_paths`, `input_paths`, `video`, `subtitle`, `batch: BatchRequest`, `recursive`, `no_extract`, `manual`). `BatchRequest` replaces clap's `Option<Option<PathBuf>>` with the explicit `Off` / `Auto` / `Directory(PathBuf)` triple.
- The two hard-coded extension lists become `pub const SYNC_VIDEO_EXTENSIONS: &[&str]` and `pub const SYNC_SUBTITLE_EXTENSIONS: &[&str]` in `core::sync`, used by both `resolve_sync_pairing` and `SyncArgs::get_input_handler` (`sync_args.rs:296`).
- `SyncArgs::get_sync_mode` shrinks to a ~15-line adapter that fills `SyncPairingRequest` from its own fields and calls `resolve_sync_pairing`. Behaviour is bit-for-bit preserved, including the `PathBuf::new()` sentinel for manual-mode video and the `SubXError::InvalidSyncConfiguration` failure paths.
- `src/cli/mod.rs:56` re-exports `SyncMode` from `crate::core::sync` instead of `sync_args`.

**4. `src/error.rs` presentation split.**

- A new `src/cli/error_ext.rs` defines `pub trait SubXErrorExt` with `fn exit_code(&self) -> i32` and `fn user_friendly_message(&self) -> String`, plus `impl SubXErrorExt for SubXError` carrying the two bodies moved verbatim from `src/error.rs:1130-1141` and `:1248-1289`.
- `category()` (`:1148`), `machine_code()` (`:1180`) and **`hint()` (`:1210`) stay on `SubXError`** in core. `hint()` staying is deliberate: the GUI calls it at `../subx/src-tauri/src/error.rs:57` and uses only its `Option` presence.
- The `OutputModeUnsupported` variant stays in the core enum so `category()`/`machine_code()`/`hint()` remain exhaustive without a wildcard arm.
- The four CLI-side callers gain `use crate::cli::error_ext::SubXErrorExt;`: `src/main.rs:58,60,141` and `src/cli/output.rs:157-158` (`ErrorEnvelope::from_error`).
- `src/core/matcher/engine.rs:1255-1261` (`operation_error_from`) stops calling `user_friendly_message()` and uses `err.to_string()` instead. This is provably byte-identical: both construction sites (`:1792`, `:1849`) build `SubXError::FileOperationFailed(_)`, whose `Display` (`error.rs:111`) and `user_friendly_message` (`:1278`) both render `File operation failed: {0}`. A regression test locks the equality.

**5. Intra-doc link and doctest sweep** (required by `broken_intra_doc_links = "deny"`): `src/error.rs:1208`, `src/core/matcher/engine.rs:1249`, `src/commands/cache_command.rs:551`, `src/cli/translate_args.rs:7,173`, and the doctests inside the relocated files.

**Compatibility shims.** `src/cli` keeps `InputPathHandler`, `CollectedFiles`, `SyncMode`, and `create_default_output_path` reachable at their current paths, so the GUI's `subx_cli::cli::{CollectedFiles, InputPathHandler}` and `subx_cli::cli::sync_args::create_default_output_path` imports keep compiling through this change and are only retired when the GUI switches to `subx_core::`. AGENTS.md forbids new `#[deprecated]` attributes, so each shim is marked legacy in rustdoc prose only.

No CLI flag, no configuration key, no JSON envelope field, and no observable command behaviour changes. Every relocated body moves verbatim.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `input-path-handling`: The collection algorithm becomes core-owned (`crate::core::input`) and is specified independently of argument parsing — `InputPathHandler` and `CollectedFiles` SHALL NOT depend on `clap` or on anything under `crate::cli`. The `-i` / `--recursive` / `--no-extract` flags remain the CLI's surface *over* that algorithm, and `crate::cli` keeps legacy re-exports. The "Unified Path Merging", "Directory Deduplication" and "No-Extract CLI Flag" requirements are restated against the new module boundary.
- `timeline-sync`: Sync-mode resolution and default output-path derivation become documented core APIs (`core::sync::{SyncMode, SyncPairingRequest, BatchRequest, resolve_sync_pairing, create_default_output_path, SYNC_VIDEO_EXTENSIONS, SYNC_SUBTITLE_EXTENSIONS}`) instead of implicit behaviour of a clap struct. The "Single-File and Batch Modes" requirement is restated so that the auto-pairing heuristic is a specified core contract, not a side effect of `SyncArgs`.
- `error-handling`: The taxonomy plus `category()` / `machine_code()` / `hint()` are the library contract; `exit_code()` and `user_friendly_message()` become the binary's contract via `subx_cli::cli::error_ext::SubXErrorExt`. The "User-Facing Error Formatting", "Process Exit Code Mapping", "Top-Level Error Rendering", "Stable Machine-Readable Category and Code" and "Process-Boundary Rendering Honors Output Mode" requirements are restated against the split, and a new requirement fixes which half owns what — including a prohibition on core code calling presentation methods.

## Impact

- **Code:** `src/cli/input_handler.rs` **deleted** → `src/core/input/mod.rs` **added** (569 LOC moved verbatim minus one doctest); `src/core/mod.rs:19-30` (new `pub mod input;`); `src/cli/mod.rs:37,53,56` (module list + shims); `src/core/sync/mod.rs:30-33` (new `SyncMode`, `SyncPairingRequest`, `BatchRequest`, `resolve_sync_pairing`, `create_default_output_path`, two extension consts); `src/cli/sync_args.rs:36,296,301-435,438-450,452-466` (body extracted, enum and free fn removed, shim re-exports added); `src/cli/error_ext.rs` **added**; `src/error.rs:1122-1141,1205-1237,1239-1289` (two methods removed, one rustdoc link rewritten); `src/main.rs:17,58,60,141`; `src/cli/output.rs:152-162`; `src/core/matcher/engine.rs:1249,1255-1261`; `src/commands/sync_command.rs:8,10,514,587,687-698`; `src/commands/translate_command.rs:325`; `src/commands/cache_command.rs:551`; `src/cli/{convert_args,detect_encoding_args,match_args,translate_args}.rs` import lines.
- **Tests:** Import rewrites only — `tests/unified_path_handling_tests.rs:10`, `tests/archive_input_extraction_tests.rs:16`, `tests/sync_argument_flexibility_tests.rs:5`. New tests: `resolve_sync_pairing` unit coverage for the single-positional video probe, single-positional subtitle probe, manual-mode subtitle-only path, two-positional pairing, batch trigger via each of `-b` / `-i` / extension-less positional, and the `InvalidSyncConfiguration` failure paths; a `Display` == `user_friendly_message` equality test for `SubXError::FileOperationFailed`; a compile-level test that the legacy `subx_cli::cli::{CollectedFiles, InputPathHandler, SyncMode}` and `subx_cli::cli::sync_args::create_default_output_path` paths still resolve. Note that `tests/cli/input_handler_tests.rs` is **not** wired into any harness shim (no `#[path = "cli/input_handler_tests.rs"]` exists) and therefore does not compile today; it is out of scope and left untouched.
- **APIs:** *Added:* `subx_cli::core::input` module; `subx_cli::core::sync::{SyncMode, SyncPairingRequest, BatchRequest, resolve_sync_pairing, create_default_output_path, SYNC_VIDEO_EXTENSIONS, SYNC_SUBTITLE_EXTENSIONS}`; `subx_cli::cli::error_ext::SubXErrorExt`. *Moved but still reachable:* `subx_cli::cli::{InputPathHandler, CollectedFiles, SyncMode}` and `subx_cli::cli::sync_args::create_default_output_path` (legacy re-exports, rustdoc-marked). *Breaking:* `SubXError::exit_code` and `SubXError::user_friendly_message` become trait methods — callers must import `SubXErrorExt`. Inherent-method callers outside the crate break; the GUI is unaffected because it calls neither (SDR §8).
- **Dependencies:** None added or removed. `tempfile` and `log`, previously reached from `src/cli/`, are now reached from `src/core/` — both are already `[dependencies]` and both are already in SDR §4's `subx-core` list.
- **Documentation:** `AGENTS.md:137` (the "Add/change CLI arguments" row still points at `src/cli/input_handler.rs` and the already-deleted `src/cli/validation.rs`) and `AGENTS.md:210-211` (exit-code / user-message sentence); `docs/tech-architecture.md:39-60` (CLI-layer module map gains `core/input` and loses `cli/input_handler`); `docs/machine-readable-output.md:864-865` (the source-of-truth file list splits `error.rs` from `cli/error_ext.rs`); `CHANGELOG.md` `[Unreleased]` gains `### Changed` and `### Added` entries. The `## Purpose` paragraphs of the three touched main specs cite `src/cli/input_handler.rs` and `src/cli/sync_args.rs` and are re-synced when this change is archived.
