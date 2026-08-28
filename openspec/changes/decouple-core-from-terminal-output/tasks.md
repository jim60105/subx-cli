## 1. Characterisation Tests (lock current behaviour first)

> These tasks land against **unmodified** source. They must pass before phase 2 begins and must still pass, unchanged, at the end of phase 6. Do not weaken an assertion to make the rewiring pass — a failure here means behaviour moved.

- [ ] 1.1 Extend `tests/cli/match_command_json_silence.rs` (already has `human_mode_dry_run_still_prints_ai_analysis_results`) with a `text`-mode characterisation test that asserts the full `🔍 AI Analysis Results:` block reaches **stderr** — the header, `   - Total matches:`, `   - Confidence threshold:` and one `   - file_… -> file_… (confidence: …)` line — and that **stdout** contains none of it
- [ ] 1.2 Add to the same file a `text`-mode test with `--quiet` asserting that `Warning: Skipping relocation due to existing file:` (`src/core/matcher/engine.rs:1914-1919`) is **still** printed to stderr, pinning the rule that `--quiet` does not silence engine warnings
- [ ] 1.3 Add to the same file a `text`-mode test driving an AI response whose `video_file_id`/`subtitle_file_id` do not match any scanned file, asserting stderr contains `⚠️  Cannot find AI-suggested file pair:` (`src/core/matcher/engine.rs:1429-1434`) and that stdout does not
- [ ] 1.4 Create `tests/cli/ai_usage_output_characterization.rs` plus its harness shim `tests/ai_usage_output_characterization_tests.rs` (`#[path = "cli/ai_usage_output_characterization.rs"] mod ai_usage_output_characterization;` — Cargo does not auto-discover `tests/cli/`); with a wiremock provider returning a `usage` object, assert the `🤖 AI API Call Details:` block and its four `   Model:` / `   Prompt tokens:` / `   Completion tokens:` / `   Total tokens:` lines land on **stdout** in `text` mode and nowhere in `--output json` mode
- [ ] 1.5 Create `tests/cli/translation_progress_characterization.rs` plus its harness shim `tests/translation_progress_characterization_tests.rs`; assert the `📊 Translation Progress:` block (`src/core/translation/engine.rs:404-406`) reaches **stderr** in `text` mode, is absent under `--quiet`, and is absent under `--output json`. Note in the module docs that `FileManager::rollback` (`src/core/file_manager.rs:270-277`) and `WorkerPool::shutdown` (`src/core/parallel/worker.rs:124-138`) get **no** CLI-level characterisation test because neither is reachable from a command today — `convert_command.rs:385` calls `remove_file` only, and no production code constructs a `WorkerPool`
- [ ] 1.6 Run `cargo nextest run --filter-expr 'test(json_silence) + test(ai_usage_output) + test(translation_progress)' || true` and confirm every new test passes against unmodified source

## 2. The `core::report` Seam

- [ ] 2.1 Create `src/core/report/mod.rs` with a module-level rustdoc header explaining that this is the transport-agnostic sink core reports through, that the CLI owns the only terminal implementation, and that it travels with `src/core/` into `subx-core`
- [ ] 2.2 Define `pub trait Reporter: Send + Sync` with the four methods `diagnostic(&self, message: &str)`, `warn(&self, message: &str)`, `ai_usage(&self, usage: &AiUsage)` and `progress(&self, event: &ProgressEvent<'_>)`, **each with a default no-op body** (`let _ = message;` to keep `unused_variables` quiet); rustdoc each channel with `# Arguments` and a note on which transport suppresses it. In the same file define `pub struct NoopReporter;`, `impl Reporter for NoopReporter {}` and `pub fn noop() -> std::sync::Arc<dyn Reporter>` returning `Arc::new(NoopReporter)`, each with a compiling `# Examples` block
- [ ] 2.3 Define `#[non_exhaustive] pub enum ProgressEvent<'a> { Message(&'a str) }` deriving `Debug, Clone, PartialEq, Eq`, with rustdoc stating it covers free-form status chatter including retry notices and that `expose-core-orchestration-apis` (D2) will add structured variants
- [ ] 2.4 Define `pub struct AiUsage { pub model: String, pub prompt_tokens: u32, pub completion_tokens: u32, pub total_tokens: u32 }` deriving `Debug, Clone, PartialEq, Eq`, with full rustdoc (`# Examples` included)
- [ ] 2.5 Declare `pub mod report;` in `src/core/mod.rs` (module list at `:19-30`) and add `report` to the subsystem bullet list in that file's header comment (`:8-16`)
- [ ] 2.6 Add unit tests in `src/core/report/mod.rs`: a `RecordingReporter` double backed by `Mutex<Vec<…>>` (per-test, never a global) asserting each channel records what it was given; `NoopReporter` swallows all four channels; a `const _: fn() = || { fn assert_send_sync<T: Send + Sync>() {} assert_send_sync::<std::sync::Arc<dyn Reporter>>(); };` static assertion; and a `match` over `ProgressEvent` with a `_` arm to prove `#[non_exhaustive]` usability

## 3. Reporter Attachment Points (signatures unchanged)

- [ ] 3.1 Replace the `AiUsageStats` struct definition at `src/services/ai/mod.rs:322-333` with `pub use crate::core::report::AiUsage as AiUsageStats;`, documented in prose as a legacy alias (no `#[deprecated]` — forbidden by AGENTS.md); confirm `AiResponse::usage` (`:341`) and `ui::display_ai_usage`'s parameter type (`src/cli/ui.rs:303`) still compile untouched
- [ ] 3.2 Add `reporter: Arc<dyn Reporter>` to `MatchEngine` (`src/core/matcher/engine.rs:1264-1268`), default it to `report::noop()` in `new` (`:1272-1278`, **signature unchanged**), and add `pub fn with_reporter(mut self, reporter: Arc<dyn Reporter>) -> Self`
- [ ] 3.3 Do the same for `TranslationEngine` (`src/core/translation/engine.rs:34-38` struct, `new` at `:54-65`, signature unchanged); the reporter field must not appear in the hand-written `Debug` impl at `:44-51`
- [ ] 3.4 Do the same for `SyncEngine` (`src/core/sync/engine.rs:21-40`, `new` signature unchanged). It has no reporting site today; store the field and document it as the attachment point D2 will use for progress and cancellation
- [ ] 3.5 Do the same for `ComponentFactory` (`src/core/factory.rs:42-58`, `new` signature unchanged)
- [ ] 3.6 Do the same for `FileManager` (`src/core/file_manager.rs:99-104` struct, `new` at `:157-160`) — keep `new()` and `impl Default` (`:300-304`) byte-compatible so all eight rustdoc examples keep compiling
- [ ] 3.7 Do the same for `WorkerPool` (`src/core/parallel/worker.rs:9-12` struct, `new` at `:42-47`); the field is `Arc<dyn Reporter>` so the hand-written `Clone` impl at `:155-165` clones it with a refcount bump
- [ ] 3.8 Add the field and `with_reporter` to the four AI clients — `OpenAIClient` (`src/services/ai/openai.rs:19`, constructors `:322`, `:342`, `:366`, `:393`), `OpenRouterClient` (`openrouter.rs:19`, `:58`, `:80`, `:108`), `AzureOpenAIClient` (`azure_openai.rs:17`, `:52`, `:82`), `LocalLLMClient` (`local.rs:34`, `:76`, `:123`) — every constructor signature unchanged, every one defaulting to `report::noop()`
- [ ] 3.9 Add `pub fn create_ai_provider_with_reporter(ai_config: &AIConfig, reporter: Arc<dyn Reporter>) -> Result<Box<dyn AIProvider>>` in `src/core/factory.rs`, attaching the reporter to the **concrete** client before boxing; rewrite the existing free `create_ai_provider` (`:213-240`) to delegate with `report::noop()`, keeping its signature
- [ ] 3.10 Propagate the factory's reporter in `ComponentFactory::create_match_engine` (`:69-82`), `create_file_manager` (`:86-92`), `create_ai_provider` (`:103-105`, now calling the reporter-aware variant) and `create_translation_engine` (`:146-160`)

## 4. Rewire `src/core/**` (9 sites)

- [ ] 4.1 `src/core/matcher/engine.rs`: delete `let json_mode = crate::cli::output::active_mode().is_json();` at `:1306` and emit the `🔍 AI Analysis Results:` block (`:1363-1376`) as a **single** `self.reporter.diagnostic(&msg)` call whose `msg` joins the header, `   - Total matches:`, `   - Confidence threshold:` and the per-match lines with `\n` (no trailing `\n` — the reporter adds it)
- [ ] 4.2 `src/core/matcher/engine.rs:1429-1434`: replace the `⚠️  Cannot find AI-suggested file pair:` `eprintln!` with `self.reporter.warn(&format!(…))`, preserving the embedded `\n     Video ID: '{}'\n     Subtitle ID: '{}'` layout exactly
- [ ] 4.3 `src/core/matcher/engine.rs:1568-1592`: delete the `json_mode` local and emit each dry-run `Preview: …` line through `self.reporter.diagnostic(…)`; add a code comment recording Decision 4b — this branch is unreachable from the CLI (`src/commands/match_command.rs:576-584` calls `execute_operations` only when `!args.dry_run`), so the stdout→stderr move is not CLI-observable
- [ ] 4.4 `src/core/matcher/engine.rs:1911-1920` and `:1961-1966`: delete the `json_mode` local in `resolve_filename_conflict` and route `Warning: Skipping relocation due to existing file: {}` and `Warning: Conflict resolution prompt not implemented, using auto-rename` through `self.reporter.warn(…)`
- [ ] 4.5 `src/core/matcher/engine.rs:2284-2295`: rewrite `log_available_files` to build one `\n`-joined string (`   Available {} files:` plus one `     - ID: … | Name: … | Path: …` line per file) and emit it with `self.reporter.diagnostic(…)`; delete the JSON early-return
- [ ] 4.6 `src/core/matcher/engine.rs:2299-2331`: rewrite `log_no_matches_found` the same way, **keeping the leading `\n`** of `\n❌ No matching files found that meet the criteria` inside the string so the blank line is preserved; delete the JSON early-return
- [ ] 4.7 `src/core/parallel/worker.rs:124-138`: replace the `!is_quiet() && !active_mode().is_json()` guard in `shutdown` with `self.reporter.progress(&ProgressEvent::Message(&format!("Waiting for worker {} to complete task {}", id, info.task_id)))`
- [ ] 4.8 `src/core/translation/engine.rs:285-296`: replace the `!is_quiet() && !active_mode().is_json()` guard in `translate_batch_with_unknown_retry` with `self.reporter.progress(&ProgressEvent::Message("⚠ Translation response contained an unknown cue ID; discarding the batch response and retrying once."))`
- [ ] 4.9 `src/core/translation/engine.rs:392-402`: delete the free function `log_translation_progress` entirely; at its two call sites (`:159` and `:184`) call `self.reporter.progress(&ProgressEvent::Message(&format_translation_progress(translations.len(), cue_ids.len())))`. Keep `format_translation_progress` (`:404-406`) and its unit test (`:690`) untouched
- [ ] 4.10 `src/core/file_manager.rs:270-277`: replace the JSON guard around `Warning: Cannot restore removed file (backup not implemented)` with `self.reporter.warn(…)`
- [ ] 4.11 Confirm `grep -rn "crate::cli" src/core` returns zero hits

## 5. Rewire `src/services/ai/**` (4 sites)

- [ ] 5.1 Delete the `use crate::cli::display_ai_usage;` import from all four clients — `src/services/ai/openai.rs:2`, `azure_openai.rs:1`, `openrouter.rs:2`, `local.rs:12`
- [ ] 5.2 Replace each `display_ai_usage(&stats)` call with `self.reporter.ai_usage(&stats)`, leaving the surrounding `if let Some(usage_obj) = …` parsing and the `AiUsageStats { … }` literal untouched — `openai.rs:520`, `azure_openai.rs:274`, `openrouter.rs:233`, `local.rs:245`
- [ ] 5.3 Confirm `grep -rn "crate::cli" src/services` returns zero hits

## 6. CLI `TerminalReporter` and Command Wiring

- [ ] 6.1 Create `src/cli/reporter.rs` with `pub struct TerminalReporter;` implementing `crate::core::report::Reporter` exactly per `design.md` Decision 3: `diagnostic` and `warn` → `eprintln!` unless `output::active_mode().is_json()`; `ai_usage` → `ui::display_ai_usage(usage)`; `progress` → `eprintln!` unless `active_mode().is_json() || output::is_quiet()`
- [ ] 6.2 Add `pub fn terminal_reporter() -> std::sync::Arc<dyn Reporter>` in the same file, and rustdoc that this type is the **only** consumer of `output::active_mode()` / `output::is_quiet()` on behalf of core; `src/cli/output.rs:78-79` is not modified by this change
- [ ] 6.3 Declare `pub mod reporter;` in `src/cli/mod.rs` (module list `:32-43`) and re-export `TerminalReporter` / `terminal_reporter` from the `pub use` block (`:45-60`)
- [ ] 6.4 Update `ui::display_ai_usage` (`src/cli/ui.rs:303-313`): keep its body and its JSON early-return exactly as they are, retype the parameter to `&crate::core::report::AiUsage`, and update its rustdoc to state that core no longer calls it and that `TerminalReporter::ai_usage` is its only caller
- [ ] 6.5 Attach the reporter to both of the match command's factories — `ComponentFactory::new(config_service)?.with_reporter(crate::cli::terminal_reporter())` at `src/commands/match_command.rs:283` (`execute`) and `ComponentFactory::new(config_service.as_ref())?.with_reporter(…)` at `:319` (`execute_with_config`)
- [ ] 6.6 `src/commands/match_command.rs:436`: `MatchEngine::new(ai_client, match_config).with_reporter(crate::cli::terminal_reporter())` — note the `ai_client` already carries the factory's reporter via task 3.10, and `execute_with_client` may also be reached with an externally supplied client, so the engine attaches its own
- [ ] 6.7 `src/commands/translate_command.rs:177`: `ComponentFactory::new(config_service)?.with_reporter(crate::cli::terminal_reporter())`
- [ ] 6.8 `src/commands/sync_command.rs:434`: `SyncEngine::new(config.sync.clone())?.with_reporter(crate::cli::terminal_reporter())`
- [ ] 6.9 Re-run the phase-1 characterisation tests **unchanged**: `cargo nextest run --filter-expr 'test(json_silence) + test(ai_usage_output) + test(translation_progress)' || true`. Any diff in expected output is a rewiring bug, not a test to update

## 7. Boundary Guard and Regression

- [ ] 7.1 Add `tests/core_cli_boundary.rs`: walk every `.rs` file under `src/core/` and `src/services/` resolved from `env!("CARGO_MANIFEST_DIR")` (never CWD-relative), collect every line containing the token `crate::cli`, and assert the collection is empty, failing with the `file:line` list
- [ ] 7.2 Add a unit test in `src/core/report/mod.rs` (or extend 2.6) that a `MatchEngine` built through `MatchEngine::new(...)` with **no** reporter attached produces no stdout/stderr bytes on a path that previously printed, using a `NoopReporter` and a stub `AIProvider`
- [ ] 7.3 Add a `TerminalReporter` unit test in `src/cli/reporter.rs` covering the channel/stream/suppression matrix; drive it through the `Reporter` trait only, and never mutate global state (`install_active_mode` is a process-wide `OnceLock` — assert against `output::active_mode()`'s default `Text` and cover the JSON branch through the existing `assert_cmd`-based tests in `tests/cli/match_command_json_silence.rs` instead)
- [ ] 7.4 Run `cargo nextest run --filter-expr 'test(core_cli_boundary) + test(report) + test(reporter) + test(match_engine) + test(translation) + test(worker) + test(file_manager) + test(factory)' || true` and confirm the targeted modules pass

## 8. Documentation

- [ ] 8.1 Update `docs/ai-provider-integration-guide.md`: replace `use crate::cli::display_ai_usage;` (`:42`), the `// emit usage stats via display_ai_usage` comment (`:83`), the "Every provider must call `display_ai_usage` (from `src/cli/`)" rule (`:108`) and the "Call `display_ai_usage` after every successful API response" guidance (`:563`) with the `Reporter::ai_usage` / `with_reporter` contract, including the `AiUsage` payload shape
- [ ] 8.2 Update `docs/tech-architecture.md:298` ("Each provider calls `display_ai_usage` (from `src/cli/`)") and add a short subsection describing the `core::report` seam: the four channels, `NoopReporter` as the default, `with_reporter` attachment, and the layering rule that `src/core/` and `src/services/` may not reference `crate::cli`
- [ ] 8.3 Add a `### Changed` entry under `[Unreleased]` in `CHANGELOG.md` recording that core and service modules now report through a transport-agnostic `Reporter` seam instead of reading the CLI's output mode, that terminal behaviour is unchanged, and that `services::ai::AiUsageStats` is now an alias of `core::report::AiUsage`; add an `### Added` entry for `core::report::{Reporter, NoopReporter, AiUsage, ProgressEvent}` and the `with_reporter` builders
- [ ] 8.4 Verify no rustdoc intra-doc link crosses into `crate::cli` from `src/core/` or `src/services/` — `broken_intra_doc_links = "deny"` makes any such link a hard build failure, and B2 would break every one of them

## 9. Quality Gate

- [ ] 9.1 Run `cargo fmt` and `cargo clippy -- -D warnings` and fix all warnings
- [ ] 9.2 Run `cargo nextest run --filter-expr 'test(report) + test(reporter) + test(core_cli_boundary) + test(json_silence) + test(ai_usage) + test(translation) + test(match_engine) + test(worker) + test(file_manager) + test(factory)' || true` and confirm the targeted modules pass
- [ ] 9.3 Run `scripts/quality_check.sh` once at the end (main agent only — do not invoke from sub-agents) and ensure it is green
- [ ] 9.4 Run `cargo test --doc --all-features` to confirm rustdoc examples still compile
