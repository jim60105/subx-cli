## ADDED Requirements

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
