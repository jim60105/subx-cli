## MODIFIED Requirements

### Requirement: Progress Bar Styling

The helper `ui::create_progress_bar(total)` SHALL return an `indicatif::ProgressBar` with a fixed template of the form `{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}` so that all batch UIs share a single visual style: green spinner, elapsed-time prefix, 40-cell cyan-on-blue bar, current/total counts, ETA, and a trailing free-form message segment.

The trailing `{msg}` segment SHALL be present so that a caller which sets a message on the bar — such as the parallel batch's `Active: … | Queued: … | Completed: …` line, or a terminating `finish_with_message` — has that message rendered. A bar whose caller never sets a message SHALL render the segment as empty.

This helper SHALL be the **only** `indicatif::ProgressBar` constructor in `subx-cli`. No module SHALL define a private constructor with a competing template.

#### Scenario: Progress bar template applied
- **GIVEN** `create_progress_bar(100)` is called
- **WHEN** the bar is rendered
- **THEN** the style SHALL be built from the template `{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}`

#### Scenario: Caller-set message is rendered
- **GIVEN** a bar from `create_progress_bar(10)` on which `set_message("Active: 2 | Queued: 5 | Completed: 3/10")` has been called
- **WHEN** the bar renders a frame
- **THEN** that message text SHALL appear in the rendered frame

#### Scenario: No competing constructor exists
- **GIVEN** the working tree after this change
- **WHEN** `grep -rn "ProgressBar::new" subx-cli/src/` is run
- **THEN** the only non-test hit SHALL be inside `ui::create_progress_bar` in `subx-cli/src/cli/ui.rs`

### Requirement: Progress Bar Visibility Follows Configuration

The CLI SHALL construct exactly one progress bar per batch operation, in its `Reporter` implementation (`subx-cli/src/cli/reporter.rs`), on receipt of `ProgressEvent::Started`, and SHALL hide it when `general.enable_progress_bar = false` by calling `ProgressBar::set_draw_target(ProgressDrawTarget::hidden())` on the constructed bar. When the flag is `true` (the default) the progress bar SHALL remain visible. Construction SHALL go through `ui::create_progress_bar`, so the JSON-mode force-hide rule applies to it without a second decision.

The reporter SHALL advance the bar's position on `ProgressEvent::Advanced`, SHALL set its message from `Advanced`'s `item` when one is supplied, and SHALL finish it on `ProgressEvent::Finished`. A `Started` received while a bar is already open SHALL replace that bar rather than nesting a second one.

No command module SHALL construct a progress bar of its own. `src/commands/match_command.rs` in particular SHALL NOT define a private `create_progress_bar` and SHALL NOT call `set_draw_target` itself; the parallel match path SHALL report through the reporter, which owns the bar and the flag.

#### Scenario: Progress bar hidden when disabled
- **GIVEN** a configuration with `general.enable_progress_bar = false` and a parallel match run with one or more video files
- **WHEN** the CLI reporter receives `ProgressEvent::Started` and constructs the batch progress bar
- **THEN** the bar SHALL have its draw target set to `ProgressDrawTarget::hidden()` and no progress output SHALL appear on the terminal

#### Scenario: Progress bar visible by default
- **GIVEN** a configuration with `general.enable_progress_bar = true`
- **WHEN** the CLI reporter constructs the batch progress bar
- **THEN** the bar SHALL retain its default (visible) draw target

#### Scenario: Commands do not construct bars
- **GIVEN** the working tree after this change
- **WHEN** `grep -rn "create_progress_bar\|ProgressDrawTarget" subx-cli/src/commands/` is run
- **THEN** it SHALL return zero non-test hits
