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

#### Scenario: Token breakdown rendered
- **GIVEN** an `AiUsageStats { model: "gpt-4", prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 }`
- **WHEN** `display_ai_usage` is invoked
- **THEN** stdout SHALL contain the header `🤖 AI API Call Details:` followed by lines showing `Model: gpt-4`, `Prompt tokens: 10`, `Completion tokens: 5`, and `Total tokens: 15`
