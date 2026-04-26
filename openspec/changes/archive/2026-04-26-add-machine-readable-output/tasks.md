## 1. Output Renderer Scaffolding

- [x] 1.1 Add `OutputMode { Text, Json }` enum and `--output`/`--quiet` top-level flags to `Cli` in `src/cli/mod.rs`. The flags SHALL be defined directly on `Cli` (not as `clap::Arg::global(true)`) so they only apply when they appear before the subcommand token, avoiding clashes with the existing subcommand-local `--output <PATH>` arguments on `convert`/`sync`/`translate`. Add a unit test asserting that `subx-cli convert --output a.ass --format ass` parses with `ConvertArgs.output == "a.ass"` and `Cli.output == OutputMode::Text`.
- [x] 1.1b Add a unit test asserting that `--quiet` is accepted only when it precedes the subcommand token: `subx-cli --quiet match <path>` SHALL parse with `Cli.quiet == true`, while `subx-cli match --quiet <path>` SHALL be rejected by clap as an unknown argument (since no subcommand currently defines a local `--quiet` flag). This mirrors the placement constraint already required for `--output` and is a regression-prevention test.
- [x] 1.2 Wire `SUBX_OUTPUT` environment-variable fallback (resolved before/within `cli::run_with_config`) with `--output` taking precedence
- [x] 1.3 Create new module `src/cli/output.rs` defining `OutputRenderer` (trait or enum) with `TextRenderer` and `JsonRenderer` variants and a stable `SCHEMA_VERSION = "1.0"` constant
- [x] 1.4 Implement `JsonRenderer` envelope writer that buffers a typed `Envelope { schema_version, command, status, data, warnings, error }` and emits exactly one JSON document + trailing `\n` to stdout on finalize
- [x] 1.5 Make `print_success`/`print_warning`/`print_error`/`create_progress_bar`/`display_match_results` aware of the active mode (force-hide progress bars in JSON mode; suppress success/warning helpers; strip ANSI/symbol from `print_error` in JSON mode)
- [x] 1.6 Thread the chosen `OutputMode` through `commands::dispatcher::dispatch_command(_with_ref)` to every command's `execute_with_config`
- [x] 1.7 Audit every direct `println!`/`print!`/`eprintln!`/`writeln!(std::io::stdout(), …)` call site under `src/` (commands, sync engine, AI client diagnostics, cache command, format converter) and route them through the renderer or stderr-only diagnostic helpers; document each remaining stderr-only call site (e.g., `tracing` macros, fatal panics) with a code comment
- [ ] 1.8 (Deferred) Audit `indicatif::ProgressBar` construction sites — currently `src/cli/ui.rs` (`create_progress_bar`, ~line 206) and `src/commands/match_command.rs` (~line 586) — and route every site through a single helper (e.g., `ui::progress_draw_target_for(mode: OutputMode)`) that consults the active `OutputMode`; ensure any future `ProgressBar::new` call site (e.g., parallel scheduler, sync VAD progress) is added through the same helper. Deferred: only one rogue site found, all current JSON-mode paths already gate progress output explicitly, no contract break observed.

## 2. CLI Parsing Flow at the Process Boundary

- [x] 2.1 Implement an early argv/env sniff in `src/main.rs` that resolves the tentative `OutputMode` from `--output`/`--output=`/`SUBX_OUTPUT` *before* invoking clap, defaulting to `Text` when ambiguous
- [x] 2.2 Replace `Cli::parse()` with `Cli::try_parse()` at the process boundary; on `Err(clap::Error)` in tentative `Json` mode, emit a synthetic JSON error envelope (category `argument_parsing`, code `E_ARGUMENT_PARSING`, exit code from `clap::Error::exit_code()`, message from the rendered clap error with ANSI stripped) and exit with the clap exit code
- [x] 2.3 In tentative `Text` mode, render the clap error exactly as today (preserving help/usage formatting) and exit with the clap exit code
- [x] 2.4 Change `cli::run_with_config` to return a structured `RunOutcome { output_mode, command, result }` so `main.rs` can render the final envelope without re-parsing argv; keep `cli::run` as a backward-compatible `Result<()>` shim if any external caller depends on it
- [x] 2.5 Add integration tests under `tests/cli/output_format_clap_errors.rs` covering: unknown flag in text mode, unknown flag in JSON mode, missing required argument in JSON mode, `--help` in JSON mode (still text — clap emits via `ErrorKind::DisplayHelp` with exit code `0`; assert no JSON envelope is written and exit status is `0`), and `--version` in JSON mode (still text — clap emits via `ErrorKind::DisplayVersion` with exit code `0`; assert no JSON envelope is written and exit status is `0`)

## 3. Error Envelope Plumbing

- [x] 3.1 Add `pub fn category(&self) -> &'static str` to `SubXError` with an exhaustive (no wildcard) match
- [x] 3.2 Add `pub fn machine_code(&self) -> &'static str` to `SubXError` with the same exhaustiveness guarantee
- [x] 3.3 Update `src/main.rs` to render the JSON error envelope on stdout when the active mode is `json`, while preserving the existing `print_error` path in `text` mode; keep `process::exit(SubXError::exit_code)` in both modes
- [x] 3.4 Unit-test `category()`/`machine_code()`/`exit_code()` round-trip for every `SubXError` variant (including `Io`, `Config`, `SubtitleFormat`, `AiService`, `Api`, `AudioProcessing`, `FileMatching`, `FileAlreadyExists`, `FileNotFound`, `InvalidFileName`, `FileOperationFailed`, `CommandExecution`, `NoInputSpecified`, `InvalidPath`, `PathNotFound`, `DirectoryReadError`, `InvalidSyncConfiguration`, `UnsupportedFileType`, `Other`)
- [ ] 3.5 (Deferred) Add an `error.details.partial_results` carrier helper used by commands that may fail mid-run (e.g., match operation loop). Deferred: per-command payloads already carry per-item `status`/`error` arrays; no current contract gap requires a separate carrier.

## 4. Match Command JSON Payload

- [x] 4.1 Define serializable types for `MatchPayload { dry_run, confidence_threshold, candidates, operations, summary }` in `src/commands/match_command.rs`, where each `operations[i]` carries `status` (`"ok"`/`"error"`) and an optional `error { code, category, message }` for per-file isolation
- [x] 4.2 Replace direct `display_match_results` and status-symbol calls with renderer-aware emission so JSON mode skips the table
- [x] 4.3 Populate `summary` counters (`total_candidates`, `accepted`, `applied`, `skipped`, `failed`) from the engine's results and threshold filtering; `failed` SHALL count operations whose per-item `status == "error"`
- [x] 4.4 Snapshot tests under `tests/cli/output_format_match.rs` covering: success live-mode, success dry-run, sub-threshold candidates, partial per-file failure (top-level `status == "ok"` plus per-item error), AI service error envelope (using `MockOpenAITestHelper`)

## 5. Sync Command JSON Payload

- [x] 5.1 Define `SyncPayload { method, inputs, operations }` types in `src/commands/sync_command.rs` with per-item `status`/`error` on both `inputs` and `operations`
- [x] 5.2 Map VAD and manual sync results into the structured payload
- [x] 5.3 Force-hide the sync progress bar / spinner when JSON mode is active
- [x] 5.4 Snapshot tests under `tests/cli/output_format_sync.rs` covering: VAD success, manual offset success, partial batch failure (one file errors), `InvalidSyncConfiguration` whole-command error envelope

## 6. Convert Command JSON Payload

- [x] 6.1 Define `ConvertPayload { conversions: [...] }` in `src/commands/convert_command.rs`, each entry carrying `status` (`"ok"`/`"error"`) and optional `error { code, category, message }` to honor the existing per-file error isolation contract
- [x] 6.2 Replace inline `print_success`/`print_warning` with renderer-aware helpers
- [x] 6.3 Snapshot tests under `tests/cli/output_format_convert.rs` covering: SRT→ASS single file, batch directory conversion with one corrupt input (top-level `status == "ok"`, one entry with `status == "error"`, exit code 0), single-input fatal `SubtitleFormat` error (top-level error envelope, exit code 4)

## 7. Detect-Encoding Command JSON Payload

- [x] 7.1 Define `DetectEncodingPayload { files: [...] }` in `src/commands/detect_encoding_command.rs` with per-file `status` and optional `error`
- [x] 7.2 Suppress the human-friendly per-file table on stdout in JSON mode
- [x] 7.3 Snapshot tests under `tests/cli/output_format_detect_encoding.rs` covering: UTF-8 BOM, multi-file array, one unreadable path inside a multi-file batch (per-item error), single missing path (top-level error envelope)

## 8. Cache Subcommands JSON Payloads

- [x] 8.1 Define typed payloads for `cache status`, `cache clear`, `cache rollback`, `cache apply` in `src/commands/cache_command.rs`; `cache apply` SHALL include an `items` array with per-item `status`/`error` mirroring per-file isolation. (`cache list` is intentionally deferred — see `specs/cache-management/spec.md`.)
- [x] 8.2 Replace `print_success`/`print_warning` confirmation messages with renderer-aware emission
- [x] 8.3 Snapshot tests under `tests/cli/output_format_cache.rs` covering: empty `list`, populated `list`, `clear` removed counter, `apply` mixed success/failure counters with per-item errors
- [x] 8.4 Reconcile the pre-existing `cache status --json` flag (the only existing JSON flag on any cache subcommand, defined on `StatusArgs` in `src/cli/cache_args.rs:64`) with the global `--output json`: treat the legacy flag as a thin alias that forwards to the global mode and emits byte-identical output; document the equivalence; do NOT add `--json` to any other cache subcommand

## 9. `generate-completion` Reject JSON Mode

- [x] 9.1 In `src/commands/generate_completion_command.rs` (or wherever the subcommand is dispatched), detect the active `OutputMode` and, when it is `Json`, emit a top-level error envelope (`command == "generate-completion"`, `error.category == "command_execution"`, `error.code == "E_OUTPUT_MODE_UNSUPPORTED"`, `error.exit_code == SubXError::CommandExecution(_).exit_code()` — currently `1`, since `CommandExecution` falls through the wildcard arm of `SubXError::exit_code` in `src/error.rs`) and exit with that exit code *without* writing any shell-completion script to stdout
- [x] 9.2 Document the behavior in `docs/machine-readable-output.md` and the command reference
- [x] 9.3 Integration test under `tests/cli/output_format_generate_completion.rs` asserting that `subx-cli --output json generate-completion bash` exits with `SubXError::CommandExecution(_).exit_code()`, stdout is exactly one JSON envelope, and no shell-script bytes are emitted

## 10. Translate and Config Minimum Envelope

- [x] 10.1 Emit the minimum `translate` envelope (`data.translated_files`) in `src/commands/translate_command.rs`
- [x] 10.2 Emit the minimum `config` envelope (`data.config` for `get`/`list`, `{key, value}` for `set`) in `src/commands/config_command.rs`
- [x] 10.3 Smoke-test that both commands produce a valid envelope on success and the uniform error envelope on failure

## 11. Stdout/Stderr Discipline Tests

- [x] 11.1 Assertion helper in `tests/common/` that scans stdout for ANSI escapes and `indicatif` artifacts and fails the test if any are present in JSON mode
- [x] 11.2 Cross-command test that runs each covered subcommand with `--output json` and verifies stdout parses as exactly one JSON document terminated by `\n`
- [x] 11.3 `jq`-based scripting smoke test under `tests/cli/output_format_jq.rs` (skipped when `jq` is unavailable on the test host) verifying success and error envelopes can be queried via `jq -e .status`
- [x] 11.4 Test that asserts `--quiet` in JSON mode additionally suppresses stderr `tracing` chatter while leaving the stdout envelope intact

## 12. Documentation and Release Notes

- [x] 12.1 Create `docs/machine-readable-output.md` documenting the envelope, schema-version policy, error-category/machine-code table, per-command payload schemas (including per-item `status`/`error` semantics), CLI parsing flow (clap errors in JSON mode), `generate-completion` rejection, and scripting recipes
- [x] 12.2 Add a "JSON output" subsection to each covered command in `docs/command-reference.md`
- [x] 12.3 Add a "Scripting" callout linking to `docs/machine-readable-output.md` in `README.md` and `README.zh-TW.md`
- [x] 12.4 Add a `### Added` entry to `CHANGELOG.md` describing the `--output json` contract and the covered commands

## 13. Quality Gates

- [x] 13.1 Run `cargo fmt` and resolve any formatting drift
- [x] 13.2 Run `cargo clippy -- -D warnings` and fix all warnings
- [x] 13.3 Run `cargo test --doc --all-features` for doc-tests touching the new module
- [x] 13.4 Run `scripts/quality_check.sh` (from the main agent only) and confirm all snapshot tests, integration tests, and coverage thresholds pass before commit
