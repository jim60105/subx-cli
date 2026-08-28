## Why

`src/core/**` and `src/services/**` reach **upward** into `src/cli/**` at thirteen non-test sites. This is the single edge that makes the crate split impossible: the moment `src/core/` becomes `subx-core/src/core/`, every one of these lines is a compile error, because `crate::cli` will no longer exist in that crate — and `clap`, `colored`, `indicatif` and the CLI's process-global output mode are permanently `subx-cli`-only (SDR D8).

Nine sites read the CLI's process-global output mode purely to decide whether their own `println!`/`eprintln!` should fire:

| Site | Reads | Prints |
|---|---|---|
| `src/core/matcher/engine.rs:1306` | `active_mode().is_json()` | the `🔍 AI Analysis Results:` block (`:1363`) and `⚠️ Cannot find AI-suggested file pair` (`:1429`) |
| `src/core/matcher/engine.rs:1568` | `active_mode().is_json()` | the dry-run `Preview:` lines (`:1570`, `:1583`) |
| `src/core/matcher/engine.rs:1911` | `active_mode().is_json()` | `Warning: Skipping relocation…` (`:1914`) and `Warning: Conflict resolution prompt not implemented…` (`:1962`) |
| `src/core/matcher/engine.rs:2285` | `active_mode().is_json()` | `log_available_files`' scanned-file dump |
| `src/core/matcher/engine.rs:2304` | `active_mode().is_json()` | `log_no_matches_found`' diagnostic dump |
| `src/core/parallel/worker.rs:131` | `is_quiet()` **and** `active_mode().is_json()` | `Waiting for worker … to complete task …` |
| `src/core/translation/engine.rs:291` | `is_quiet()` **and** `active_mode().is_json()` | the unknown-cue-ID retry notice |
| `src/core/translation/engine.rs:395` | `is_quiet()` **and** `active_mode().is_json()` | the `📊 Translation Progress:` block |
| `src/core/file_manager.rs:273` | `active_mode().is_json()` | `Warning: Cannot restore removed file…` |

The globals being read are `static ACTIVE_MODE: OnceLock<OutputMode>` and `static QUIET: OnceLock<bool>` (`src/cli/output.rs:78-79`).

Four more sites import and call `crate::cli::display_ai_usage` (`src/cli/ui.rs:303-313`), a `colored`-flavoured `println!` helper that is itself gated on `output::active_mode().is_json()`:

- `src/services/ai/openai.rs:2` (called `:520`), `src/services/ai/azure_openai.rs:1` (`:274`), `src/services/ai/openrouter.rs:2` (`:233`), `src/services/ai/local.rs:12` (`:245`).

The shape of the problem is uniform: **core knows that a terminal exists, that a JSON mode exists, and that a `--quiet` flag exists.** None of those are core's concerns. The Tauri GUI (`../subx`) already drives `MatchEngine`, `TranslationEngine`, `ComponentFactory` and `FileManager` directly (SDR §8) and gets this chatter dumped into its process stdout/stderr with no way to intercept it.

This is series change **A1**, and it is the critical-path item: SDR §3 states the thirteen edges "must all be gone before any file moves". Doing it here, inside the existing single crate, is deliberate — it makes B2 (`move-core-sources-into-subx-core`) a near-pure `git mv` instead of a move-plus-redesign. The same seam is later extended by D2 (`expose-core-orchestration-apis`) for progress and cancellation, so it is designed for that from the start.

## What Changes

- Add a new module `src/core/report/mod.rs` exposing the transport-agnostic sink prescribed by SDR §3:
  - `trait Reporter: Send + Sync` with four methods, **all with default no-op bodies** so implementors opt in: `diagnostic(&self, message: &str)`, `warn(&self, message: &str)`, `ai_usage(&self, usage: &AiUsage)`, and the forward-looking `progress(&self, event: &ProgressEvent<'_>)`.
  - `struct NoopReporter` with a blanket `impl Reporter for NoopReporter {}`, plus `fn noop() -> Arc<dyn Reporter>`.
  - `#[non_exhaustive] enum ProgressEvent<'a> { Message(&'a str) }` — the minimal shape D2 will extend with structured started/advanced/finished variants without a breaking change.
  - `struct AiUsage { model, prompt_tokens, completion_tokens, total_tokens }` — the plain, core-owned payload that `display_ai_usage` currently prints.
- Make `services::ai::AiUsageStats` (`src/services/ai/mod.rs:322-333`) a re-export alias of `core::report::AiUsage` so `AiResponse.usage`, `ui::display_ai_usage`, and every existing test keep compiling unchanged. There is exactly one struct; `AiUsage` is the canonical name.
- Attach the reporter through a **builder**, never through a constructor signature change. `MatchEngine::new`, `SyncEngine::new`, `TranslationEngine::new`, `ComponentFactory::new`, `FileManager::new`, `WorkerPool::new` and the four AI client constructors keep their current signatures and default their reporter field to `NoopReporter`; each gains `with_reporter(self, Arc<dyn Reporter>) -> Self`. This is mandatory: the GUI constructs `MatchEngine::new` (`../subx/src-tauri/src/state.rs:992`) and `TranslationEngine::new` (`../subx/src-tauri/src/commands/translate.rs:489`) directly.
- `ComponentFactory` propagates its reporter into everything it builds — `create_match_engine`, `create_translation_engine`, `create_file_manager`, `create_ai_provider` — so a single `factory.with_reporter(…)` at the command boundary wires a whole command.
- Rewire all thirteen sites to call `self.reporter.<channel>(…)` and delete every `crate::cli::output::…` read and every `use crate::cli::display_ai_usage;` from `src/core/` and `src/services/`. After this change `grep -rn "crate::cli" src/core src/services` returns **zero** non-test hits.
- Add `src/cli/reporter.rs` with `TerminalReporter`, the CLI's `Reporter` implementation. **All** mode gating moves here: it is the only thing that reads `output::active_mode()` and `output::is_quiet()` on behalf of core. The `OnceLock` globals at `src/cli/output.rs:78-79` stay exactly where they are and keep their current shape.
- Wire `TerminalReporter` in at the five command-level construction sites: `src/commands/match_command.rs:283`, `:319`, `:436`, `src/commands/translate_command.rs:177`, `src/commands/sync_command.rs:434`.
- Add a boundary guard test (`tests/core_cli_boundary.rs`) that walks `src/core/` and `src/services/` from `CARGO_MANIFEST_DIR` and fails if any file references `crate::cli`, so the decoupling cannot silently regress between A1 and B2.

**Behaviour is observably unchanged for the CLI.** Every message keeps its exact text, its exact stream, and its exact suppression rule under `--quiet` and `--output json`. The one deliberate exception is documented in `design.md` Decision 4b: the dry-run `Preview:` lines in `MatchEngine::execute_operations` are **unreachable from the CLI today** (`src/commands/match_command.rs:576-584` only calls `execute_operations` when `!args.dry_run`), and they move from stdout to stderr along with every other diagnostic. Phase 1 of `tasks.md` locks the current output in characterisation tests *before* any rewiring happens.

No public constructor signature changes, no CLI flag changes, no configuration key changes, no dependency changes.

## Capabilities

### New Capabilities

- `core-reporting`: the transport-agnostic diagnostic / warning / AI-usage / progress sink that `src/core/` and `src/services/` report through. Owns the `Reporter` trait and its `Send + Sync` bound, the default no-op method bodies, `NoopReporter`, `AiUsage`, `ProgressEvent`, the `with_reporter` attachment contract that preserves every existing constructor signature, and the architectural rule that no module under `src/core/` or `src/services/` may reference `crate::cli`.

### Modified Capabilities

- `progress-reporting`: the "AI Usage Summary Display" requirement changes — `ui::display_ai_usage` is no longer called from `src/services/ai/*`; it is reached only through the CLI's `Reporter` implementation, and its payload type is now the core-owned `AiUsage`. A new requirement pins `TerminalReporter`'s stream-and-suppression matrix (which channel writes to which stream, and which channels `--quiet` and JSON mode silence).
- `machine-readable-output`: the "Stdout/Stderr Discipline in JSON Mode" requirement changes — free-form chatter from core engines is no longer gated by each call site reading `crate::cli::output::active_mode()`; it is routed through `Reporter` and suppressed by the CLI's implementation. The "Quiet Flag" requirement changes to state explicitly that `--quiet` silences the reporter's `progress` channel and does not silence `diagnostic` or `warn`, which is exactly today's per-site behaviour made uniform.

## Impact

- **Code:** New `src/core/report/mod.rs` and `src/cli/reporter.rs`. Modified: `src/core/mod.rs` (declare `report`), `src/core/matcher/engine.rs` (`MatchEngine` struct/`new`/`with_reporter` at `:1264-1278`; sites `:1306`, `:1363-1376`, `:1429-1434`, `:1568-1592`, `:1911-1920`, `:1961-1966`, `:2284-2295`, `:2299-2331`), `src/core/parallel/worker.rs` (`:9-47`, `:124-138`), `src/core/translation/engine.rs` (`:34-65`, `:159`, `:184`, `:285-296`, `:392-402`), `src/core/file_manager.rs` (`:99-160`, `:270-277`, `:300-304`), `src/core/sync/engine.rs` (`:21-40`), `src/core/factory.rs` (`:42-58`, `:69-82`, `:86-92`, `:103-105`, `:146-160`, `:208-240`), `src/services/ai/mod.rs` (`:322-333`), `src/services/ai/{openai,azure_openai,openrouter,local}.rs` (import line + the `AiUsageStats`/`display_ai_usage` block in each), `src/cli/mod.rs` (`:32-43`, `:45-60`), `src/cli/ui.rs` (`:303-313` rustdoc only), `src/commands/match_command.rs` (`:283`, `:319`, `:436`), `src/commands/translate_command.rs` (`:177`), `src/commands/sync_command.rs` (`:434`). `src/cli/output.rs` is **not** modified.
- **Tests:** Characterisation tests added first, then re-run unchanged after the rewiring: `tests/cli/match_command_json_silence.rs` (extended with text-mode assertions alongside the existing `human_mode_dry_run_still_prints_ai_analysis_results`), new `tests/cli/ai_usage_output_characterization.rs` and `tests/cli/translation_progress_characterization.rs` with their `#[path]` harness shims (Cargo does not auto-discover `tests/cli/`), new unit tests in `src/core/report/mod.rs` for `NoopReporter` and a recording double, and the new boundary guard `tests/core_cli_boundary.rs`. No test may set global state (AGENTS.md); the recording reporter is per-test and `Arc`-owned.
- **APIs:** Purely additive. Every existing constructor keeps its signature. New public items: `core::report::{Reporter, NoopReporter, AiUsage, ProgressEvent, noop}`, `with_reporter` on six types plus the four AI clients, `factory::create_ai_provider_with_reporter`, `cli::reporter::TerminalReporter`. `services::ai::AiUsageStats` becomes an alias for `core::report::AiUsage`. Per AGENTS.md no new `#[deprecated]` is introduced; the alias is documented as legacy in rustdoc prose only (mirrors SDR D11).
- **Dependencies:** None added, none removed. The seam introduces no new crate — `Arc` and `dyn` are `std`.
- **Documentation:** `docs/ai-provider-integration-guide.md` (`:42`, `:83`, `:108`, `:563` — providers report usage through `Reporter::ai_usage`, not `crate::cli::display_ai_usage`), `docs/tech-architecture.md` (`:298`, plus a new subsection describing the reporter seam and the layering rule), and a `### Changed` entry under `[Unreleased]` in `CHANGELOG.md`.
- **Coverage:** Unchanged denominator — nothing moves between crates in this change, so the 75% floor enforced by `scripts/check_coverage.sh` applies as-is. The new `src/core/report/mod.rs` is small and fully unit-tested; `src/cli/reporter.rs` is covered by the characterisation tests.
