## Context

SubX is still one crate. `src/cli/` is supposed to hold the clap surface and the terminal UI; `src/core/` and `src/services/` hold the engines. A0 (`prune-unused-dependencies`) cleaned the manifest and deleted the orphan `src/cli/validation.rs`; A1 (`decouple-core-from-terminal-output`) cut the thirteen `core`/`services` → `crate::cli` edges by introducing the `core::report::Reporter` seam and its CLI-side `TerminalReporter` (`src/cli/reporter.rs`). After A1, the dependency arrow points one way for *behaviour*.

It still points the wrong way for *file placement*. Four bodies of code sit on the wrong side of the line SDR §2.1–§2.3 draws:

- **`src/cli/input_handler.rs` (569 LOC).** Its complete import list is `std::collections::HashMap`, `std::fs`, `std::path::{Path, PathBuf}`, `log::warn`, `tempfile::TempDir`, `crate::core::archive`, `crate::error::SubXError`. There is no clap type anywhere in the file, no `Args` derive, no `#[arg(...)]`. What it actually contains is directory traversal with symlink skipping, a case-insensitive extension filter, archive detection and extraction into a `TempDir`, RAII ownership of those temp dirs, and a temp-root → archive-path map. That is filesystem domain logic. It lives under `src/cli/` because that is where the `-i` flag it was written to serve lives.
- **`SyncArgs::get_sync_mode` (`src/cli/sync_args.rs:301-435`).** Roughly 135 lines that read a positional path's extension, then *probe the filesystem* — `dir.join(format!("{stem}.{sub_ext}")).exists()` — across `["srt","ass","vtt","sub"]` to find the subtitle beside a video, or across `["mp4","mkv","avi","mov"]` to find the video beside a subtitle. It also decides Single vs Batch and builds an `InputPathHandler` for the batch case. Every field it reads (`positional_paths`, `input_paths`, `video`, `subtitle`, `batch`, `recursive`, `no_extract`) is a plain `Option<PathBuf>` / `Vec<PathBuf>` / `bool`. It is domain behaviour wearing a clap costume.
- **`create_default_output_path` (`src/cli/sync_args.rs:455-466`).** A twelve-line free function deriving `<stem>_synced.<ext>`. It is already imported *out* of the CLI layer by `src/commands/sync_command.rs:10`.
- **`SubXError::exit_code()` (`src/error.rs:1130`) and `SubXError::user_friendly_message()` (`:1248`).** Process exit codes and multi-line terminal prose with embedded `Hint:` lines. Neither concept exists for a library consumer.

The Tauri GUI (`../subx`, SDR §8) is the proof that the first three are library concerns: it imports `subx_cli::cli::input_handler::{CollectedFiles, InputPathHandler}` and `subx_cli::cli::sync_args::create_default_output_path` — and **nothing else** from `cli`. No `*Args` type, no `ui`, no `output`, no `table`. It also proves the fourth is not: it calls `category()`, `hint()` and `to_string()`, never `exit_code()` or `user_friendly_message()`.

Two constraints shape everything below. First, `Cargo.toml` sets `broken_intra_doc_links = "deny"`, so any rustdoc link left pointing at a moved item is a hard build failure — and once B2 splits the crate, a link that crosses the new boundary cannot be repaired at all. Second, the change must fit roughly one workday, so every relocated body moves **verbatim**; the only new code is the adapter that feeds the extracted sync-pairing function and the extension trait that receives the two error methods.

This is series change **A2** (SDR §12). It runs entirely inside the existing single crate: no submodule, no workspace, no second `Cargo.toml`, and no dependency edits. Its success criterion is structural, not behavioural: after it lands, `src/cli/` contains only clap argument structs, `mod.rs`, `output.rs`, `reporter.rs`, `table.rs`, `ui.rs` and the new `error_ext.rs` — and B2 becomes a `git mv` of `src/core/`, `src/services/`, `src/config/` and `src/error.rs`.

## Goals / Non-Goals

**Goals:**

- Put every item destined for `subx-core` under `src/core/`, `src/services/` or `src/config/` **before** the physical split, so B2 moves files rather than refactoring them.
- Move each body verbatim. Behaviour, error variants, message text, ordering and edge cases are preserved exactly; the CLI's observable output is byte-identical.
- Draw the `error.rs` presentation seam exactly where SDR §2.3 puts it: machine contracts (`category`, `machine_code`, `hint`) stay in core; process/terminal contracts (`exit_code`, `user_friendly_message`) become a `subx-cli` extension trait.
- Cut the one remaining core → presentation edge that A1's `Reporter` seam does not cover: `core::matcher::engine::operation_error_from` calling `err.user_friendly_message()`.
- Rewrite every rustdoc intra-doc link and doctest that would break — now, in this change, not deferred to C3.
- Keep the GUI compiling unchanged for the whole A2→B2 window via legacy re-exports in `src/cli`.

**Non-Goals:**

- Moving `src/cli/*_args.rs`, `src/commands/**`, `src/cli/{ui,table,output,reporter}.rs` anywhere. SDR D7 keeps `commands/` in `subx-cli`; SDR D8 keeps clap out of core permanently.
- Introducing a workspace, submodule or second crate. That is B1/B2.
- Moving the batch prefix-match pairing in `src/commands/sync_command.rs:540-600` (the `starts_with` heuristic and the "exactly one video + one subtitle" override). That runs *after* `SyncMode::Batch` is chosen, lives in a command module, and stays in `subx-cli`.
- Replacing `sync_command.rs`'s clone-and-mutate-`SyncArgs` working-state pattern with a core options struct. See Decision 5.
- Changing any error message, exit code, category string, machine code, or JSON envelope field. `err.category()`'s literal strings are load-bearing for the GUI's `core.{category}` i18n keys (SDR §8) and are untouched.
- Adding `#[deprecated]` to the compatibility shims. AGENTS.md forbids new `#[deprecated]` attributes; see Decision 3.
- Deleting the compatibility shims. They are retired by the GUI's own migration PR after B2 (implementation plan §7), not here.
- Touching `Cargo.toml`. A0 owns the manifest in this phase; A2 adds and removes zero dependencies.
- Fixing `tests/cli/input_handler_tests.rs`. It contains `use crate::cli::InputPathHandler;` and is not referenced by any `#[path = "cli/…"]` harness shim, so it is not compiled today. Wiring it up is a test-suite question that belongs to B3.

## Decisions

### Decision 1: `input_handler.rs` becomes `src/core/input/mod.rs`, a directory module

The file moves to `src/core/input/mod.rs` — a directory module, not `src/core/input.rs` — and `pub mod input;` joins the alphabetical list in `src/core/mod.rs` (`:19-30`, which by then also carries A1's `report`). The public path is `crate::core::input::{InputPathHandler, CollectedFiles}`, matching SDR §2.1 verbatim.

**Why a directory module:** SDR §2.1 names the destination `src/core/input/mod.rs`. Every other core subsystem that grew beyond one concern (`archive/`, `formats/`, `matcher/`, `parallel/`, `sync/`, `translation/`) is a directory. Starting as a directory means the first follow-up that splits collection from extraction adds a sibling file instead of converting `input.rs` → `input/mod.rs` and re-touching `core/mod.rs`.

**What moves:** the whole file, including the private helpers `matches_extension`, `scan_directory_flat`, `scan_directory_recursive`, `extract_and_collect`, the `Deref<Target = Vec<PathBuf>>` and `AsRef<[PathBuf]>` impls, and the `#[cfg(test)] mod symlink_tests` block (`:529-568`). No item is renamed and no signature changes.

**Why it is safe to move as-is:** `crate::core::archive` (its only crate-internal dependency besides `crate::error`) is already core, so the move creates no new edge in either direction. After the move, `src/core/input/` depends on `std`, `log`, `tempfile`, `crate::core::archive` and `crate::error` — all four of which SDR §4 assigns to `subx-core`.

**Alternatives considered:**

- *Leave it in `src/cli/` and let B2 move it.* Rejected — B2 would then have to move a file *and* rewrite its five doctests *and* repoint five `use` sites *and* delete a clap-referencing doctest, all while the crate boundary is being created and half the tree does not compile. That is precisely the risk SDR §12 created A0–A2 to eliminate.
- *Move it to a new top-level `src/input/`.* Rejected — SDR §2.1 fixes the destination as `core::input`, and a top-level module would need its own separate `git mv` and re-path in B2 rather than riding along with `src/core/**`.

### Decision 1a: The `MatchArgs` doctest is deleted, not rewritten

`src/cli/input_handler.rs:107-129` carries a third rustdoc example, "Command Integration", which constructs a full clap `MatchArgs` literal and calls `args.get_input_handler()`. It moves nowhere: it is **deleted**.

**Why delete rather than rewrite:** the example demonstrates the CLI's use of the handler, not the handler. Once the file lives in core, a doctest referencing `subx_cli::cli::MatchArgs` compiles today but becomes an unfixable cross-crate reference the moment B2 runs — core cannot depend on `subx-cli`. Rewriting it to avoid `MatchArgs` would leave it demonstrating nothing the first two examples do not already show. The equivalent CLI-side coverage already exists in `tests/unified_path_handling_tests.rs` and `tests/match_combined_paths_tests.rs`.

The two surviving examples (`:52-74` basic usage, `:78-105` directory processing) and the two method-level examples (`:160-178` on `merge_paths_from_multiple_sources`, `:257-276` on `get_directories`) keep their bodies and only change their `use` line from `subx_cli::cli::InputPathHandler` to `subx_cli::core::input::InputPathHandler`. Their `Ok::<(), subx_cli::error::SubXError>(())` tails are unaffected — `error` is core in SDR §2.1.

### Decision 2: `create_default_output_path`, `SyncMode` and the pairing body land in `core::sync`

`src/core/sync/mod.rs` currently declares one submodule (`pub mod engine;`) and re-exports four items (`:30-33`). It gains:

```rust
// src/core/sync/mod.rs

/// Video container extensions recognised when auto-pairing a sync input.
pub const SYNC_VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov"];
/// Subtitle extensions recognised when auto-pairing a sync input.
pub const SYNC_SUBTITLE_EXTENSIONS: &[&str] = &["srt", "ass", "vtt", "sub"];

/// How the caller requested batch processing.
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
#[derive(Debug, Clone, Default)]
pub struct SyncPairingRequest {
    pub positional_paths: Vec<PathBuf>,
    pub input_paths: Vec<PathBuf>,
    pub video: Option<PathBuf>,
    pub subtitle: Option<PathBuf>,
    pub batch: BatchRequest,
    pub recursive: bool,
    pub no_extract: bool,
    /// Manual-offset mode: a video is not required.
    pub manual: bool,
}

/// Sync mode: a resolved single pair, or a batch handler.
#[derive(Debug)]
pub enum SyncMode {
    Single { video: PathBuf, subtitle: PathBuf },
    Batch(InputPathHandler),
}

pub fn resolve_sync_pairing(request: &SyncPairingRequest) -> Result<SyncMode, SubXError>;
pub fn create_default_output_path(input: &Path) -> PathBuf;
```

`resolve_sync_pairing`'s body is `get_sync_mode`'s body with `self.batch` replaced by `request.batch`, `self.is_manual_mode()` replaced by `request.manual`, and the two inline extension arrays replaced by the new consts. Nothing else changes — including the `PathBuf::new()` sentinel used as the "no video" marker in manual mode, and every `SubXError::InvalidSyncConfiguration` return.

`SyncArgs::get_sync_mode` becomes a ~15-line adapter:

```rust
pub fn get_sync_mode(&self) -> Result<SyncMode, SubXError> {
    resolve_sync_pairing(&SyncPairingRequest {
        positional_paths: self.positional_paths.clone(),
        input_paths: self.input_paths.clone(),
        video: self.video.clone(),
        subtitle: self.subtitle.clone(),
        batch: match &self.batch {
            None => BatchRequest::Off,
            Some(None) => BatchRequest::Auto,
            Some(Some(dir)) => BatchRequest::Directory(dir.clone()),
        },
        recursive: self.recursive,
        no_extract: self.no_extract,
        manual: self.is_manual_mode(),
    })
}
```

**Why `BatchRequest` instead of passing `Option<Option<PathBuf>>` through:** `Option<Option<PathBuf>>` is clap's encoding of `num_args = 0..=1` (`sync_args.rs:130-138`). It is unreadable outside that context — `Some(None)` meaning "flag present, value absent" is a convention, not a type. Core should not inherit an argument parser's encoding; a three-variant enum says the same thing and makes the `match` in the adapter the single place that knows about clap's shape. It also removes the `if let Some(Some(batch_dir)) = &self.batch` idiom from the core body.

**Why a request struct instead of eight positional parameters:** eight parameters, five of which are path-ish, is a call site nobody can read or safely reorder. The struct derives `Default`, so the six unit tests for pairing construct only the two or three fields each case needs.

**Why `SYNC_VIDEO_EXTENSIONS` / `SYNC_SUBTITLE_EXTENSIONS` as public consts:** the same two arrays are written out three times today — `sync_args.rs:296` (`get_input_handler`), `:327` (the batch handler inside `get_sync_mode`), and `:349`/`:362`/`:398`/`:401` (the probe loops). Naming them once means the CLI's `get_input_handler` and the core's pairing cannot drift apart, and it gives the GUI a documented list instead of a hard-coded literal.

**`create_default_output_path` moves verbatim**, together with its four unit tests (`sync_args.rs:1068-1099`), which are re-homed into `src/core/sync/mod.rs`'s test module. `src/cli/sync_args.rs` keeps `pub use crate::core::sync::create_default_output_path;` so that `SyncArgs::get_output_path` (`:227-235`) and any consumer of `subx_cli::cli::sync_args::create_default_output_path` (the GUI) keep resolving unchanged.

### Decision 2a: The clap-shaped sync types stay in the CLI

Three sync-related items in `sync_args.rs` deliberately do **not** move:

- `SyncMethodArg` (`:18-25`) — it derives `clap::ValueEnum`.
- `impl From<SyncMethodArg> for crate::core::sync::SyncMethod` (`:27-34`) — it converts a clap type, so it must live where the clap type lives. The direction is already correct: CLI → core.
- `SyncMethod` (`:146-153`, the two-variant `Auto`/`Manual` back-compat enum returned by `SyncArgs::sync_method`) — distinct from `core::sync::SyncMethod` (`Auto`/`LocalVad`/`Manual`, `engine.rs:283-290`), reachable only from `SyncArgs`, and not consumed by the GUI. Merging the two enums is a real cleanup but it is an API change with test fallout, not a relocation, and it is out of A2's budget.

`SyncArgs::validate` (`:157-224`) also stays. Its output is user-facing multi-line CLI usage text (`Usage:\n• Batch with directory: subx sync -b <directory>\n…`) returned as `Result<(), String>` — terminal prose by construction, and the mirror image of the `user_friendly_message` decision below.

### Decision 3: Compatibility shims are rustdoc-marked legacy, never `#[deprecated]`

`src/cli/mod.rs` keeps all four moved names reachable at their present paths:

```rust
// src/cli/mod.rs
/// Legacy re-export. `InputPathHandler` and `CollectedFiles` now live in
/// [`crate::core::input`]; prefer that path. This alias exists so that
/// consumers written against `subx_cli::cli::…` keep compiling across the
/// `subx-core` split and will be removed once they have migrated.
pub use crate::core::input::{CollectedFiles, InputPathHandler};
```

with the same treatment for `SyncMode` (`mod.rs:56`, now sourced from `crate::core::sync`) and for `create_default_output_path` (re-exported from `src/cli/sync_args.rs`, which is a `pub mod`, so `subx_cli::cli::sync_args::create_default_output_path` keeps resolving).

**Why shims at all:** the GUI's three `cli` imports (SDR §8) are the entire reason. Breaking them in A2 would force an out-of-tree repo to be fixed in lockstep with an internal refactor, for a path that is going to be replaced wholesale by `subx_core::` after B2 anyway. The shims cost three lines and buy a clean, independently-schedulable GUI migration.

**Why no `#[deprecated]`:** AGENTS.md states plainly that new `#[deprecated]` attributes must not be introduced — the project's rule is "delete the item and update all call sites". We cannot delete these yet (out-of-tree consumer), so the compromise is prose: the rustdoc says *legacy*, says where the item lives now, and says when the alias goes away. This matches SDR D11, which applies the identical rule to `subx-cli`'s post-split `lib.rs` re-exports.

**Why in-crate call sites are repointed anyway:** the shim exists for out-of-tree consumers. Inside the crate, all five `use crate::cli::InputPathHandler;` sites and `translate_command.rs:325`'s `&crate::cli::CollectedFiles` parameter are rewritten to the core path, so that a `grep -rn "cli::InputPathHandler\|cli::CollectedFiles" src/` after this change returns only the shim line itself. Otherwise B2 inherits five hidden call sites that resolve through an alias.

### Decision 4: `exit_code` and `user_friendly_message` become `SubXErrorExt` in `src/cli/error_ext.rs`

```rust
// src/cli/error_ext.rs
/// Presentation-layer extensions to [`SubXError`] owned by the binary.
pub trait SubXErrorExt {
    /// Stable process exit code for this error (1–6).
    fn exit_code(&self) -> i32;
    /// Multi-line terminal message, including a `Hint:` line where one applies.
    fn user_friendly_message(&self) -> String;
}

impl SubXErrorExt for SubXError { /* the two bodies, moved verbatim */ }
```

Both bodies move byte-for-byte from `src/error.rs:1130-1141` and `:1248-1289`, wildcard arms and all. Four CLI-side call sites add `use crate::cli::error_ext::SubXErrorExt;`: `src/main.rs` (`:58` `user_friendly_message`, `:60` and `:141` `exit_code`) and `src/cli/output.rs` (`ErrorEnvelope::from_error`, `:157-158`).

**Why an extension trait rather than free functions:** `err.exit_code()` and `err.user_friendly_message()` are the existing call syntax at every site. A trait keeps the diff to one `use` line per file instead of rewriting eleven call expressions, and it reads correctly — these *are* operations on a `SubXError`, just ones only a terminal cares about.

**Why in `src/cli/` and not a new top-level `src/error_ext.rs`:** SDR §2.3 names the eventual path `subx_cli::error_ext::SubXErrorExt`. That is a re-export decision for B2's slim `lib.rs`, exactly as with `subx_core::report` in A1's Decision 1. Keeping the file under `src/cli/` in A2 means the module sits with the other presentation code (`ui.rs`, `table.rs`, `output.rs`, `reporter.rs`) and B2 can add `pub use cli::error_ext;` if it wants the shorter path. Nothing here forecloses that.

**What does not move:** `category()` (`:1148`), `machine_code()` (`:1180`), `hint()` (`:1210`), every `From` impl, every helper constructor, `ApiErrorSource`, and all 22 variants. They stay inherent on `SubXError` in `src/error.rs`, which SDR §2.1 moves wholesale to `subx-core`.

### Decision 4a: `hint()` stays in core even though its prose is CLI-flavoured

`hint()` returns strings such as `"Run 'subx-cli config --help' for configuration details."` (`:1213`) and `"Run the command without --output json (and without SUBX_OUTPUT=json) to receive the shell-completion script."` (`:1232-1234`). Those name a binary and a CLI flag. By the same logic that moves `user_friendly_message`, they should move too.

They do not, and this is a deliberate exception, not an oversight. The GUI calls `err.hint()` at `../subx/src-tauri/src/error.rs:57` and uses **only its `Option` presence** — it branches on whether a hint exists, and renders its own localized text. Moving `hint()` to a `subx-cli` trait would delete a method the GUI genuinely consumes, purely to relocate prose the GUI never displays. SDR §2.3 states the resolution directly: *"`hint()` also stays in core: the GUI calls it and only uses its `Option` presence. Its prose is CLI-flavoured; document that, do not move it."*

The mitigation is documentation, not code: `hint()`'s rustdoc gains an explicit note that the returned text is written for the `subx-cli` terminal, that library consumers should treat it as an availability signal rather than display copy, and that changing the prose is not a breaking change while changing whether a variant returns `Some` is. That note is what stops a future contributor "fixing" the inconsistency by moving it.

`hint()` is also load-bearing on the CLI side — `ErrorEnvelope::from_error` (`src/cli/output.rs:159`) puts it in `error.hint` of the JSON envelope — so both consumers keep working unchanged.

### Decision 4b: `OutputModeUnsupported` stays in the core enum

`SubXError::OutputModeUnsupported { command }` (`error.rs:155-160`) exists solely because `generate-completion` writes a shell script to stdout and therefore cannot honour `--output json`. It is constructed only by the CLI. It nevertheless stays a variant of the core enum.

**Why:** `category()` and `machine_code()` use exhaustive `match`es with no wildcard arm — the `error-handling` spec's "Adding a new variant breaks the build until mapped" scenario depends on that. Removing the variant from core would either force the CLI to define a *second* error type that also needs a category and a machine code (splitting the closed set the `machine-readable-output` capability locks), or force a wildcard arm into `category()`/`machine_code()` and destroy the compile-time exhaustiveness guarantee. Keeping one unused-by-core variant is far cheaper than either.

Its rustdoc gains a note that only the binary constructs it, so the asymmetry is recorded where a reader will hit it. Note the deliberate quirk already encoded at `:1163-1166`: its `category()` is `"command_execution"` while its `machine_code()` is the more specific `"E_OUTPUT_MODE_UNSUPPORTED"`. That pairing is spec-locked and untouched.

### Decision 4c: Core's one `user_friendly_message` call site switches to `Display`, provably losslessly

`src/core/matcher/engine.rs:1255-1261`:

```rust
fn operation_error_from(err: &SubXError) -> OperationError {
    OperationError {
        category: err.category(),
        code: err.machine_code(),
        message: err.user_friendly_message(),   // ← core calling presentation
    }
}
```

This is a core → presentation edge that A1 did not cover, because it is not a `println!` and does not read the output mode. If `user_friendly_message` moves to a `subx-cli` trait and this line stays, `src/core/matcher/engine.rs` would need `use crate::cli::error_ext::SubXErrorExt;` — re-opening exactly the class of edge A1 spent a day closing. So the line must change here.

It becomes `message: err.to_string()`, and **the rendered bytes do not change**. The function has exactly two call sites, `engine.rs:1792` and `:1849`, and both construct the error immediately above the call:

```rust
let err = SubXError::FileOperationFailed(err);
outcomes.push(OperationOutcome { applied: false, error: Some(operation_error_from(&err)) });
```

`FileOperationFailed` is therefore the only variant that can ever reach `operation_error_from`. For that variant:

| | rendering |
|---|---|
| `Display` (`error.rs:111`, `#[error("File operation failed: {0}")]`) | `File operation failed: {msg}` |
| `user_friendly_message` (`:1278`) | `format!("File operation failed: {}", msg)` |

Identical, and `hint()` returns `None` for it, so there is no `Hint:` line to lose either. The `machine-readable-output` capability's requirement that a per-item `error.message` equal `user_friendly_message()` is preserved byte-for-byte — which matters, because that capability is not in A2's modified list and must not be disturbed.

A regression test locks the equality: for a `SubXError::FileOperationFailed(_)` value, `err.to_string() == err.user_friendly_message()`. It lives on the CLI side (it needs the trait) and will move with `src/cli/` in B2. `operation_error_from`'s rustdoc records the argument, so a future contributor who widens the set of variants reaching this function is told what to re-check.

**Alternatives considered:**

- *Keep `user_friendly_message` in core.* Rejected — contradicts SDR §2.3, and the method's whole content is terminal prose with `Hint:` lines.
- *Have `OperationError` carry the raw `SubXError`.* Rejected — `OperationError` is `Clone`, `SubXError` is not, and `OperationOutcome`/`OperationError` are consumed by the GUI (SDR §8), so reshaping them is a gratuitous break.
- *Inject a message renderer through the `Reporter` seam.* Rejected — the seam is an output channel, not an error formatter; threading a formatter into it to solve one call site is scope creep into D2's territory.

### Decision 5: `sync_command.rs` keeps cloning and mutating `SyncArgs` as working state

`src/commands/sync_command.rs` treats `SyncArgs` as a mutable working record in three places: `:503-509` (single video + single subtitle in a batch directory), `:577-582` (prefix-matched pair in a batch directory), and `:688-698` (applying `SyncMode::Single`'s resolved paths, including defaulting a subtitle-only invocation to `offset = Some(0.0)` and `method = Some(SyncMethodArg::Manual)`). Each site does `let mut single_args = args.clone();`, clears `input_paths`/`batch`/`recursive`, sets `video`/`subtitle`, optionally rewrites `output`, and hands the result to `run_single`.

**Decision: leave it exactly as it is.**

**Why:** the pattern is ugly but it is entirely contained within `subx-cli`. `SyncArgs` stays in `src/cli/` (SDR D8 — clap never enters core) and `sync_command.rs` stays in `src/commands/` (SDR D7 — `commands/` stays in `subx-cli`, evidenced by the GUI's zero references to `subx_cli::commands` and to any `*Args` type). Both sides of the pattern are on the same side of the future crate boundary, so it does not block B2 by one line.

**What replacing it would cost:** a `SyncRunOptions` core struct would need every field `run_single` reads — and `run_single` reads `video`, `subtitle`, `offset`, `method`, `window`, `vad_sensitivity`, `output`, `verbose`, `dry_run`, `force`. It would have to be threaded through `run_single`, `resolve_method_string`, the batch loop, the archive-origin output rewrite at `:583-592`, and the four `SyncArgs { … }` literals in that file's own test module. That is a half-day on its own, in a file A1 has just finished editing and D1 is scheduled to edit again, and it delivers **zero** progress toward the split. Against a one-workday budget for four relocations plus a doc-link sweep, it does not earn its place.

**When it should be revisited:** D2 (`expose-core-orchestration-apis`) is already going to define caller-facing option structs for the orchestration APIs the GUI needs. If a `SyncRunOptions` falls out of that work naturally, `sync_command.rs` can adopt it then, as a CLI-internal cleanup with no cross-repo consequences. Recording the decision here means D2 finds a note rather than a surprise.

### Decision 6: The intra-doc link sweep is part of this change, not deferred to C3

`Cargo.toml` sets `broken_intra_doc_links = "deny"`. Every link to a moved item must be rewritten in the same commit, and the implementation plan (§6, hazard 2) calls this out as one of the three most likely ways the series derails. The complete set for A2 is small and enumerable:

| Site | Today | After |
|---|---|---|
| `src/error.rs:1208` (in `hint()`'s rustdoc) | ``[`Self::user_friendly_message`]`` | plain-text reference — the item is no longer on `Self`, and core must not link into `crate::cli` |
| `src/core/matcher/engine.rs:1249` (`OperationError::message`) | ``[`SubXError::user_friendly_message`]`` | ``[`SubXError`]``'s `Display`, matching Decision 4c |
| `src/commands/cache_command.rs:551` | ``[`SubXError::category`]/[`SubXError::machine_code`]/[`SubXError::user_friendly_message`]`` | last segment becomes ``[`crate::cli::error_ext::SubXErrorExt::user_friendly_message`]`` |
| `src/cli/translate_args.rs:7`, `:173` | ``[`InputPathHandler`]`` (resolved via the module's `use crate::cli::InputPathHandler`) | resolves through the rewritten `use crate::core::input::InputPathHandler` — verify, do not assume |
| `src/cli/input_handler.rs` doctests `:53`, `:79`, `:110`, `:161`, `:258` | `subx_cli::cli::InputPathHandler` | `subx_cli::core::input::InputPathHandler` (and `:110`'s example is deleted per Decision 1a) |

The two intra-file links inside the moved file — ``[`with_no_extract`](Self::with_no_extract)`` (`:32`), ``[`collect_files`](Self::collect_files)`` (`:38`), ``[`CollectedFiles`]`` (`:38`), ``[`archive_origin`](CollectedFiles::archive_origin)`` (`:45`) — move with the file and keep resolving, because their targets move with them.

`cargo doc --no-deps --all-features` is the gate; `cargo test --doc --all-features` catches the doctest half.

### Decision 7: `src/cli/` is finished after this change

Post-A2, `src/cli/` contains exactly: `mod.rs`, the eight `*_args.rs` clap structs (`cache`, `config`, `convert`, `detect_encoding`, `generate_completion`, `match`, `sync`, `translate`), `output.rs`, `reporter.rs` (from A1), `table.rs`, `ui.rs`, and `error_ext.rs` (new here). `validation.rs` was deleted by A0; `input_handler.rs` is deleted here.

This is the acceptance condition the implementation plan states for batch 2: *"`src/cli/` 之下只剩 clap arg struct、`ui.rs`、`table.rs`、`output.rs`、`error_ext.rs`"*. It is worth stating as a decision because it is the check that tells the next change (B1) that A2 is genuinely done: any file left under `src/cli/` that is not in that list is a relocation someone missed.

## Risks / Trade-offs

- **Risk: a "verbatim" move is not verbatim, and behaviour drifts.** → Mitigation: the relocations are done as `git mv` plus import-line edits wherever possible, so `git diff -M` shows a rename with a handful of changed lines rather than a delete-plus-add. `resolve_sync_pairing` is the one body that is genuinely re-typed (its `self.` accesses become `request.`); its six new unit tests are written against the *current* behaviour before the extraction, so any drift fails immediately.
- **Risk: `resolve_sync_pairing`'s `BatchRequest` translation is wrong for one of clap's three states.** → Mitigation: the `Option<Option<PathBuf>>` → `BatchRequest` mapping is a single three-arm `match` in `get_sync_mode`, and each arm is covered by a test — `-b <dir>` (`Directory`), bare `-b` (`Auto`), and no flag (`Off`). `tests/sync_argument_flexibility_tests.rs` already exercises the CLI parse for these; the new unit tests exercise the core side.
- **Risk: `SyncMode::Batch(InputPathHandler)` creates a `core::sync` → `core::input` dependency that did not exist.** → Mitigation: it is an intra-`core` edge, which is exactly what the relocation is for; both modules land in the same crate in B2. `core::input` does not reference `core::sync`, so there is no cycle.
- **Risk: an out-of-tree consumer calls `SubXError::exit_code()` as an inherent method and breaks.** → Mitigation: this is the one genuinely breaking API change in A2, and it is unavoidable given SDR §2.3. The known consumer is the GUI, which calls neither method (SDR §8, verified across all 16 `.rs` files of `../subx/src-tauri/src/`). SDR D6 already schedules `subx-cli` for a `2.0.0` major bump, so the break lands in a version that permits it. The CHANGELOG entry names the trait import as the migration.
- **Risk: the shim re-exports hide a call site that B2 then has to find.** → Mitigation: the in-crate rewrite in Decision 3 is verified by `grep -rn "cli::InputPathHandler\|cli::CollectedFiles\|cli::SyncMode\|cli::sync_args::create_default_output_path" src/ tests/`, which must return only the shim declarations themselves plus the deliberate shim-resolution test.
- **Risk: `operation_error_from` later receives a variant other than `FileOperationFailed`, silently changing the JSON `error.message`.** → Mitigation: the rustdoc on `operation_error_from` states the invariant and the reason; the `Display == user_friendly_message` regression test names `FileOperationFailed` explicitly, so widening the input set without revisiting it leaves an obviously-under-specified test behind. This is a documented invariant, not an enforced one — accepted, because enforcing it would mean matching on the variant inside core, which is worse.
- **Risk: moving `create_default_output_path`'s four unit tests changes their coverage attribution.** → Mitigation: none needed — both files are inside `src/`, so `.llvm-cov.toml`'s `exclude-from-report` (which lists only `benches/*`, `tests/*`, `src/main.rs`) is unaffected and the 75% floor sees the same lines.
- **Risk: A1 and A2 both edit `src/cli/mod.rs` and the command modules, producing a merge conflict.** → Mitigation: the implementation plan (§3) marks A1↔A2 as 🔴 *never parallel* and sequences A2 strictly after A1. A1 adds `pub mod reporter;` to the `mod` block; A2 removes `mod input_handler;` from it and edits two `pub use` lines. Sequenced, the two are independent edits to the same region; run in parallel they are a guaranteed conflict.
- **Risk: `docs/machine-readable-output.md` claims `src/error.rs` owns `exit_code` and `user_friendly_message` (`:864-865`) and goes stale.** → Mitigation: the documentation phase updates that file-list, `AGENTS.md:137` (which still points the "Add/change CLI arguments" row at `src/cli/input_handler.rs` and at the already-deleted `src/cli/validation.rs`) and `AGENTS.md:210-211`. C3 later rewrites all of it for the two-crate world; A2's job is only to keep it true in the meantime.
- **Trade-off: `hint()` keeps CLI-flavoured prose in a library crate.** → Accepted, per SDR §2.3 and Decision 4a. The alternative — deleting a method the GUI calls, or duplicating the enum's shape across two crates to relocate strings the GUI never renders — is strictly worse. The cost is one rustdoc paragraph.
- **Trade-off: four legacy re-exports survive into `subx-cli` 2.0.** → Accepted and time-boxed. They exist for one out-of-tree consumer across a known window (A2 → B2 → the GUI's migration PR). They carry no `#[deprecated]`, per AGENTS.md, so nothing warns on them; the rustdoc is the only signal, which is why Decision 3 requires it to say where the item moved *and* when the alias goes away.

## Migration Plan

1. Write the six `resolve_sync_pairing` characterisation tests against today's `SyncArgs::get_sync_mode`, so the extraction has a fixed target (phase 2).
2. `git mv src/cli/input_handler.rs src/core/input/mod.rs`; declare `pub mod input;` in `src/core/mod.rs`; fix the doctests and delete the `MatchArgs` example; add the `src/cli/mod.rs` shim (phase 1).
3. Repoint the five in-crate `use crate::cli::InputPathHandler;` sites and `translate_command.rs:325` (phase 1).
4. Move `create_default_output_path` + its tests, then `SyncMode`, then the `get_sync_mode` body into `core::sync`; add `SyncPairingRequest`, `BatchRequest` and the two extension consts; reduce `get_sync_mode` to the adapter (phase 2).
5. Create `src/cli/error_ext.rs`, move the two method bodies into it, add the four `use` lines, and switch `operation_error_from` to `Display` (phase 3).
6. Sweep the intra-doc links and doctests; run `cargo doc --no-deps --all-features` and `cargo test --doc --all-features` (phase 4).
7. Rewrite the three test-file imports and add the new tests, including the shim-resolution test and the `Display == user_friendly_message` lock (phase 5).
8. Update `AGENTS.md`, `docs/tech-architecture.md`, `docs/machine-readable-output.md`, and the `[Unreleased]` CHANGELOG entries (phase 6).
9. Run the quality gate (phase 7).

Rollback is a single `git revert`. Nothing persists to disk, no data format changes, no configuration key is added, and the only API change is additive-plus-one-trait-move — reverting restores the inherent methods and the old module paths simultaneously.

## Open Questions

_None._ Every line number, import list, and call-site count in this document was verified against the working tree at `b9de1f7`, and every allocation decision is fixed by SDR §2.1–§2.3.
