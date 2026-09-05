# Progress Reporting

## Purpose

Render the SubX-CLI terminal user interface: status messages with consistent symbols, configurable progress bars for batch operations, and a formatted result table for AI matching output. Implemented in `src/cli/ui.rs`, `src/cli/table.rs`, and exercised by `src/commands/match_command.rs`. Configuration coupling with `general.enable_progress_bar` is covered here from the UI-behaviour perspective; the parallel-processing spec owns the scheduler side of the same flag.

## Requirements

### Requirement: Consistent Status Symbols

The CLI SHALL render user-facing status messages through dedicated helpers in `src/cli/ui.rs`: `print_success` prefixed with a bold green `✓`, `print_error` prefixed with a bold red `✗` emitted to stderr, and `print_warning` prefixed with a bold yellow `⚠` emitted to stdout. All three helpers SHALL format output as `<symbol> <message>` on a single line.

#### Scenario: Success message on stdout
- **GIVEN** the call `print_success("done")`
- **WHEN** the helper runs
- **THEN** a single line SHALL be written to stdout whose visible content begins with `✓ ` followed by `done`

#### Scenario: Error message on stderr
- **GIVEN** the call `print_error("failed")`
- **WHEN** the helper runs
- **THEN** a single line SHALL be written to stderr whose visible content begins with `✗ ` followed by `failed`, leaving stdout untouched

#### Scenario: Warning message on stdout
- **GIVEN** the call `print_warning("careful")`
- **WHEN** the helper runs
- **THEN** a single line SHALL be written to stdout whose visible content begins with `⚠ ` followed by `careful`

### Requirement: Progress Bar Styling

The helper `ui::create_progress_bar(total)` SHALL return an `indicatif::ProgressBar` with a fixed template of the form `{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})` so that all batch UIs share a single visual style: green spinner, elapsed-time prefix, 40-cell cyan-on-blue bar, current/total counts, and ETA.

#### Scenario: Progress bar template applied
- **GIVEN** `create_progress_bar(100)` is called
- **WHEN** the bar is rendered
- **THEN** the style SHALL be built from the template `{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})`

### Requirement: Progress Bar Visibility Follows Configuration

The match command SHALL construct a progress bar for parallel batch execution and SHALL hide it when `general.enable_progress_bar = false`, by calling `ProgressBar::set_draw_target(ProgressDrawTarget::hidden())` on the constructed bar. When the flag is `true` (the default) the progress bar SHALL remain visible. Implemented in `src/commands/match_command.rs` (parallel match path).

#### Scenario: Progress bar hidden when disabled
- **GIVEN** a configuration with `general.enable_progress_bar = false` and a parallel match run with one or more video files
- **WHEN** the match command constructs the batch progress bar
- **THEN** the bar SHALL have its draw target set to `ProgressDrawTarget::hidden()` and no progress output SHALL appear on the terminal

#### Scenario: Progress bar visible by default
- **GIVEN** a configuration with `general.enable_progress_bar = true`
- **WHEN** the match command constructs the batch progress bar
- **THEN** the bar SHALL retain its default (visible) draw target

### Requirement: Batch Progress Updates

While a parallel batch executes, the match command SHALL update the progress bar position after each task completes and SHALL periodically update the progress bar message every 500 ms with the number of active tasks, queued tasks, and completed-vs-total counts, in the format `Active: {active} | Queued: {queued} | Completed: {completed}/{total}`. When every task has finished the bar SHALL be finalized with `finish_with_message("All tasks completed")`.

#### Scenario: Position advances per completion
- **GIVEN** a batch of N tasks executed via `monitor_batch_execution`
- **WHEN** each task finishes
- **THEN** the progress bar SHALL have its position incremented to reflect the number of completed tasks, reaching N when all tasks have finished

#### Scenario: Periodic message refresh
- **GIVEN** a running batch where at least one task is in-flight
- **WHEN** the 500 ms timer fires before the current task completes
- **THEN** the progress bar message SHALL be set to `Active: {active} | Queued: {queued} | Completed: {completed}/{total}` using the scheduler's current counts

### Requirement: Match Result Table Layout

When match operations exist, `ui::display_match_results(results, is_dry_run)` SHALL print a header `📋 File Matching Results`, then a dry-run banner `🔍 Preview mode (files will not be modified)` iff `is_dry_run`, then a table rendered by `table::create_match_table` whose rows are built by expanding each `MatchOperation` into:

1. a `Video {idx}` row prefixed with the status symbol — `🔍` when `is_dry_run`, `✓` otherwise — holding the video file path;
2. a `Subtitle {idx}` row prefixed with `├` holding the original subtitle path;
3. a `New name {idx}` row prefixed with `├` holding the renamed subtitle filename; and
4. when the operation requires relocation, a trailing row prefixed with `└` whose label includes the relocation icon and verb — `📄 Copy to` for `FileRelocationMode::Copy` and `📁 Move to` for `FileRelocationMode::Move` — holding the relocation target path.

When no relocation row is appended, the last existing row's leading `├` SHALL be rewritten to `└` so the group always terminates with a `└`. After the table the helper SHALL print `Total processed {N} file mappings` where `N` is `results.len()`. When `results` is empty the helper SHALL print `No matching file pairs found` and SHALL NOT render the table.

#### Scenario: Live match with no relocation
- **GIVEN** one `MatchOperation` where `requires_relocation = false` and `is_dry_run = false`
- **WHEN** `display_match_results` is invoked
- **THEN** the rendered table SHALL contain exactly three rows for the operation in order `✓ Video 1`, `├ Subtitle 1`, `└ New name 1`, with the `├` of the last row rewritten to `└`

#### Scenario: Dry-run marker
- **GIVEN** one `MatchOperation` and `is_dry_run = true`
- **WHEN** `display_match_results` is invoked
- **THEN** the output SHALL include the banner `🔍 Preview mode (files will not be modified)` and the video row label SHALL begin with `🔍 Video 1`

#### Scenario: Move relocation row appended
- **GIVEN** one `MatchOperation` with `requires_relocation = true`, `relocation_mode = FileRelocationMode::Move`, and a `relocation_target_path` set
- **WHEN** `display_match_results` is invoked
- **THEN** the rendered group SHALL contain four rows in order `Video`, `├ Subtitle`, `├ New name`, `└ 📁 Move to`, and the `└ 📁 Move to` row SHALL hold the relocation target path

#### Scenario: Empty results short-circuit
- **GIVEN** an empty `results` slice
- **WHEN** `display_match_results` is invoked
- **THEN** the function SHALL print `No matching file pairs found` and SHALL NOT invoke `create_match_table`

### Requirement: Two-Column Match Table Style

`table::create_match_table(rows)` SHALL build the match result table from `MatchDisplayRow` records using the `tabled` crate with the `Style::rounded()` border style and with `Alignment::left()` applied to all data rows. The table SHALL expose exactly two columns named `Type` (the row label) and `Path` (the file path), sized automatically to their content without truncation.

#### Scenario: Rounded two-column table
- **GIVEN** a non-empty vector of `MatchDisplayRow` values
- **WHEN** `create_match_table` is invoked
- **THEN** the returned string SHALL render a rounded-border table with two columns whose headers are `Type` and `Path` and whose data rows are left-aligned and contain the untruncated `file_type` and `file_path` values

### Requirement: AI Usage Summary Display

`ui::display_ai_usage(usage)` SHALL emit a four-line block summarising an AI API call: a header `🤖 AI API Call Details:`, followed by indented lines `   Model: {model}`, `   Prompt tokens: {prompt_tokens}`, `   Completion tokens: {completion_tokens}`, and `   Total tokens: {total_tokens}`, terminated by a blank line.

The helper SHALL accept the core-owned `crate::core::report::AiUsage` value type (reachable under its legacy alias `crate::services::ai::AiUsageStats`), and SHALL continue to write to stdout and to return without output when the active output mode is `json`.

`ui::display_ai_usage` SHALL NOT be called from any module under `src/core/` or `src/services/`. The four AI provider clients (`src/services/ai/openai.rs`, `azure_openai.rs`, `openrouter.rs`, `local.rs`) SHALL report their token counts by calling `Reporter::ai_usage` on the reporter attached to the client, and SHALL NOT import `crate::cli::display_ai_usage`. The CLI's `Reporter` implementation is the only caller of `ui::display_ai_usage`, which it invokes from its `ai_usage` method.

Consequently, when a client has no reporter attached — the default `NoopReporter` — no usage block is emitted at all, and when the CLI's reporter is attached the emitted block is byte-identical to today's.

#### Scenario: Token breakdown rendered
- **GIVEN** an `AiUsageStats { model: "gpt-4", prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 }`
- **WHEN** `display_ai_usage` is invoked
- **THEN** stdout SHALL contain the header `🤖 AI API Call Details:` followed by lines showing `Model: gpt-4`, `Prompt tokens: 10`, `Completion tokens: 5`, and `Total tokens: 15`

#### Scenario: Usage reaches stdout through the reporter in text mode
- **GIVEN** the CLI runs a command in `text` mode against a mocked AI provider whose response carries a `usage` object with `prompt_tokens: 10`, `completion_tokens: 5`, `total_tokens: 15`
- **WHEN** the provider client finishes the call and invokes `Reporter::ai_usage`
- **THEN** stdout SHALL contain the same `🤖 AI API Call Details:` block described above, and stderr SHALL NOT contain it

#### Scenario: Providers do not import the CLI display helper
- **GIVEN** the source of `src/services/ai/openai.rs`, `azure_openai.rs`, `openrouter.rs` and `local.rs`
- **WHEN** those files are scanned for the token `crate::cli`
- **THEN** no occurrence SHALL be found, and each file's usage-reporting block SHALL call `Reporter::ai_usage` instead

#### Scenario: Usage block suppressed in JSON mode
- **GIVEN** the active output mode is `json`
- **WHEN** an AI provider client completes a call whose response carries a `usage` object
- **THEN** neither stdout nor stderr SHALL receive the `🤖 AI API Call Details:` block

### Requirement: UI Helpers Suppressed in JSON Output Mode

When the active CLI output mode is `json`, the UI helpers in `src/cli/ui.rs` SHALL behave as follows:

- `print_success` SHALL produce no output on any stream.
- `print_warning` SHALL produce no output on any stream (warnings, when relevant, SHALL be surfaced through the JSON envelope's optional `warnings` array instead).
- `print_error` SHALL still write to stderr but SHALL NOT include ANSI color escape sequences nor the `✗ ` symbol prefix; the line SHALL contain the bare message followed by `\n` so log scrapers stay greppable.
- `display_match_results` SHALL NOT render the formatted match table.

In `text` mode (default) all of the above helpers behave exactly as before.

#### Scenario: Success and warning helpers are silent in JSON mode
- **GIVEN** the active output mode is `json`
- **WHEN** a command calls `print_success("done")` or `print_warning("careful")`
- **THEN** neither stdout nor stderr SHALL receive any bytes from those helpers

#### Scenario: Error helper drops styling in JSON mode
- **GIVEN** the active output mode is `json`
- **WHEN** a command calls `print_error("failed")`
- **THEN** stderr SHALL receive a single line whose bytes consist of `failed\n` with no `\x1b[` ANSI sequence and no `✗ ` prefix

#### Scenario: Match table suppressed in JSON mode
- **GIVEN** the active output mode is `json`
- **WHEN** the match command would normally call `display_match_results`
- **THEN** no table SHALL be rendered on stdout

### Requirement: Progress Bars Force-Hidden in JSON Output Mode

When the active CLI output mode is `json`, every `indicatif::ProgressBar` constructed by SubX SHALL have its draw target set to `ProgressDrawTarget::hidden()` regardless of the value of `general.enable_progress_bar`. As of this change the known `ProgressBar`-construction sites are:

- The public `ui::create_progress_bar` helper in `src/cli/ui.rs` (line 206 at the time of writing).
- The match command's parallel-execution progress bar in `src/commands/match_command.rs` (around line 586; note it already calls `pb.set_draw_target(ProgressDrawTarget::hidden())` conditionally — that path SHALL be extended to also trigger in JSON mode).

This requirement is forward-looking: any future progress-bar construction site added to SubX (for example a sync-engine spinner, a parallel `TaskScheduler` progress bar, or AI-provider retry indicators that are not yet implemented as `indicatif` bars) SHALL also be hidden in JSON mode.

To enforce this consistently, every progress-bar construction site SHALL obtain its `ProgressDrawTarget` from a single helper (e.g., `ui::progress_draw_target_for(mode: OutputMode)`) that consults the active output mode; ad-hoc `ProgressBar::new(...)` calls that bypass this helper SHALL be refactored to go through it.

In `text` mode the existing behavior governed by `general.enable_progress_bar` is unchanged.

#### Scenario: Public progress bar hidden even when configured visible
- **GIVEN** `general.enable_progress_bar = true` and the active output mode is `json`
- **WHEN** the match command constructs a parallel-execution progress bar via `ui::create_progress_bar`
- **THEN** no progress-bar frame SHALL be rendered on stdout or stderr

#### Scenario: Other progress-bar construction sites are hidden in JSON mode
- **GIVEN** `general.enable_progress_bar = true`, the active output mode is `json`, and a future SubX feature introduces a new `indicatif::ProgressBar` (for example a parallel scheduler bar or a sync-engine spinner)
- **WHEN** that progress bar is constructed via the shared `ui::progress_draw_target_for` helper required by this requirement
- **THEN** the bar SHALL have its draw target set to `ProgressDrawTarget::hidden()` and no frame SHALL be rendered on stdout or stderr

#### Scenario: Text mode honors configuration
- **GIVEN** the active output mode is `text` and `general.enable_progress_bar = true`
- **WHEN** the match command constructs a progress bar
- **THEN** the progress bar SHALL render on stderr exactly as it does today

### Requirement: Terminal Reporter Channels and Suppression

The CLI SHALL provide `TerminalReporter` (`src/cli/reporter.rs`), the implementation of `crate::core::report::Reporter` that renders core and service messages to the terminal. It SHALL be the only place where the CLI's process-global output mode and quiet flag are consulted on behalf of core, and it SHALL apply exactly the following stream-and-suppression matrix:

| Channel | Stream | Suppressed when |
|---|---|---|
| `diagnostic` | stderr | the active output mode is `json` |
| `warn` | stderr | the active output mode is `json` |
| `ai_usage` | stdout, via `ui::display_ai_usage` | the active output mode is `json` |
| `progress` | stderr | the active output mode is `json` **or** `--quiet` is active |

`diagnostic`, `warn` and `progress` SHALL each write the message followed by exactly one `\n`, with no added prefix, symbol, or ANSI colour sequence, so that a message a core engine emits reaches the terminal byte-identically to the `eprintln!` it replaces. A message containing embedded `\n` characters SHALL be written as a single atomic block.

`--quiet` SHALL silence the `progress` channel only. It SHALL NOT silence `diagnostic` or `warn`, so warnings a core engine raises about decisions it made — conflict skips, unresolvable AI-suggested pairs — remain visible under `subx-cli --quiet` in `text` mode.

The CLI SHALL attach a `TerminalReporter` at every command-level construction site that builds a reporting component: the match command's `ComponentFactory` and `MatchEngine`, the translate command's `ComponentFactory`, and the sync command's `SyncEngine`.

#### Scenario: Diagnostic reaches stderr in text mode
- **GIVEN** the active output mode is `text` and `--quiet` is not set
- **WHEN** a core engine calls `Reporter::diagnostic("detail")` on an attached `TerminalReporter`
- **THEN** stderr SHALL receive exactly `detail\n` and stdout SHALL receive nothing

#### Scenario: Warning survives --quiet in text mode
- **GIVEN** the active output mode is `text` and `--quiet` is set
- **WHEN** a core engine calls `Reporter::warn("Warning: Skipping relocation due to existing file: x")`
- **THEN** stderr SHALL receive that line

#### Scenario: Progress is silenced by --quiet
- **GIVEN** the active output mode is `text` and `--quiet` is set
- **WHEN** a core engine calls `Reporter::progress(&ProgressEvent::Message("📊 Translation Progress:\n   Processed cues: 5/10"))`
- **THEN** neither stdout nor stderr SHALL receive any bytes from that call

#### Scenario: Progress is emitted without --quiet
- **GIVEN** the active output mode is `text` and `--quiet` is not set
- **WHEN** the same `Reporter::progress` call is made
- **THEN** stderr SHALL receive the two-line block followed by exactly one trailing `\n`

#### Scenario: Every channel is silent in JSON mode
- **GIVEN** the active output mode is `json`
- **WHEN** `diagnostic`, `warn`, `ai_usage` and `progress` are each invoked on a `TerminalReporter`
- **THEN** neither stdout nor stderr SHALL receive any bytes from those calls

#### Scenario: Multi-line diagnostic is written atomically
- **GIVEN** a core engine that previously emitted a block as several consecutive `eprintln!` calls
- **WHEN** it emits the same block as one `Reporter::diagnostic` call with embedded `\n` separators
- **THEN** stderr SHALL receive byte-identical output to the previous sequence of calls
