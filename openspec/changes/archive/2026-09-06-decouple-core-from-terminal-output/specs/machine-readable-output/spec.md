## MODIFIED Requirements

### Requirement: Quiet Flag

The CLI SHALL expose a top-level boolean flag `--quiet` on the root `Cli` struct. The two output modes interact with `--quiet` as follows:

- In `text` mode, `--quiet` SHALL suppress ancillary status output: success messages (`print_success`), warning messages (`print_warning`), progress bars, and the AI match results table.
- In `json` mode, stdout is *implicitly* quiet by construction (only the JSON envelope is ever emitted, see "Stdout/Stderr Discipline in JSON Mode"). Stderr in `json` mode is *also* implicitly quiet for free-form `eprintln!`/`println!` chatter (e.g., the matcher's `🔍 AI Analysis Results:` block, conflict-resolution warnings, AI provider response echoes), independently of `--quiet`. `--quiet` is therefore additive: it SHALL further suppress structured `tracing` / `log` records on stderr that JSON mode would otherwise have allowed (e.g., `tracing` info logs, AI provider diagnostics, indicatif debug output).
- Free-form chatter from core engines and service clients no longer decides for itself whether `--quiet` applies. It reaches the terminal only through the `Reporter` seam (see the `core-reporting` capability), and the CLI's `TerminalReporter` maps `--quiet` onto that seam as follows: `--quiet` SHALL silence the reporter's `progress` channel, and SHALL NOT silence its `diagnostic` or `warn` channels. This reproduces today's per-call-site behaviour exactly — the worker-pool shutdown notice, the translation unknown-cue retry notice and the `📊 Translation Progress:` block are silenced by `--quiet` in `text` mode, while the matcher's `🔍 AI Analysis Results:` block, its `⚠️ Cannot find AI-suggested file pair` notice and its `Warning:` conflict lines are not.

`--quiet` SHALL never suppress the JSON envelope on stdout in JSON mode, and SHALL never suppress the process exit code.

#### Scenario: Quiet in text mode silences status chatter

- **WHEN** the user runs `subx-cli --quiet match <path>` in text mode
- **THEN** stdout SHALL NOT contain `print_success` or `print_warning` lines and SHALL NOT contain progress-bar output

#### Scenario: JSON mode is implicitly quiet on stdout

- **WHEN** the user runs `subx-cli --output json match <path>` without `--quiet`
- **THEN** stdout SHALL contain exactly one JSON envelope and SHALL NOT contain any `print_success`/`print_warning`/progress-bar/match-table bytes, regardless of `general.enable_progress_bar`

#### Scenario: JSON mode is implicitly quiet on stderr for free-form chatter

- **WHEN** the user runs `subx-cli --output json match <path>` without `--quiet`
- **THEN** stderr SHALL NOT contain any free-form `eprintln!`/`println!` chatter, including but not limited to the `🔍 AI Analysis Results:` block, the `Preview:` lines, or the `Warning: Skipping relocation`/`Warning: Conflict resolution prompt not implemented` warnings emitted by the matcher engine

#### Scenario: --quiet in JSON mode additionally silences tracing chatter

- **WHEN** the user runs `subx-cli --quiet --output json match <path>`
- **THEN** stdout SHALL still contain exactly one JSON envelope, AND stderr SHALL NOT contain any `tracing`/`log` records that JSON mode would otherwise have allowed

#### Scenario: --quiet must precede the subcommand token

- **GIVEN** no subcommand currently defines its own `--quiet` flag
- **WHEN** the user runs `subx-cli --quiet match <path>` (flag before subcommand)
- **THEN** `Cli.quiet` SHALL parse as `true` and the quiet semantics defined above SHALL apply
- **AND WHEN** the user runs `subx-cli match --quiet <path>` (flag after subcommand)
- **THEN** clap SHALL reject the invocation as an unknown argument for the subcommand (mirroring the placement constraint of `--output`), and `Cli.quiet` SHALL NOT be silently set

#### Scenario: --quiet silences the progress channel but not warnings

- **GIVEN** a `text`-mode run that emits both a matcher conflict warning through `Reporter::warn` and translation/worker chatter through `Reporter::progress`
- **WHEN** the user runs the command with `--quiet`
- **THEN** stderr SHALL still contain the matcher's `Warning:` line, AND SHALL NOT contain any output produced through the reporter's `progress` channel

### Requirement: Stdout/Stderr Discipline in JSON Mode

When the active output mode is `json`, the CLI SHALL guarantee:

- Stdout contains exactly one JSON envelope plus one trailing `\n` and nothing else.
- Stderr SHALL NOT contain any JSON envelope, `✓`/`✗`/`⚠` status symbols, ANSI color escape sequences emitted by `print_success`/`print_warning`/`print_error`, or `indicatif` progress-bar frames.
- Stderr SHALL NOT contain any free-form `eprintln!` or `println!` chatter emitted directly from command implementations or core engines. This includes (non-exhaustively): the `🔍 AI Analysis Results:` debug block previously emitted by `src/core/matcher/engine.rs::match_file_list_with_audit`; the `Preview:` lines emitted by `execute_operations`; AI-provider response echoes; and the `Warning: Skipping relocation` / `Warning: Conflict resolution prompt not implemented` warnings emitted by `src/core/matcher/engine.rs::resolve_filename_conflict`. Enforcement no longer lives at the call site: no module under `src/core/` or `src/services/` may read the CLI's output mode, and none may call `println!`/`eprintln!` for human-oriented output at all. Every such message SHALL be emitted through the `Reporter` seam owned by the `core-reporting` capability, and the CLI's `TerminalReporter` SHALL suppress **all** of its channels — `diagnostic`, `warn`, `ai_usage` and `progress` — whenever the active output mode is `json`. A component constructed without a reporter attached (the default `NoopReporter`) emits nothing on any stream, so the guarantee also holds for library consumers that never install a CLI reporter.
- Stderr MAY still contain structured `tracing`/`log` records produced through the `tracing` or `log` crates and gated by the user's `RUST_LOG` (or equivalent) configuration. These records are not "free-form chatter" within the meaning of this requirement because they are opt-in through standard log filters. When `--quiet` is also active, those records SHALL additionally be silenced (see the Quiet Flag requirement).
- All `indicatif` progress bars constructed during the command SHALL be force-hidden (e.g., via `ProgressBar::set_draw_target(ProgressDrawTarget::hidden())`), regardless of the value of `general.enable_progress_bar`.
- The match results table from `src/cli/table.rs` SHALL NOT be rendered.

#### Scenario: No ANSI codes on stdout

- **WHEN** any supported command runs with `--output json`
- **THEN** stdout SHALL NOT contain any byte sequence matching the ANSI CSI prefix `\x1b[`

#### Scenario: Progress bars hidden

- **GIVEN** `general.enable_progress_bar = true`
- **WHEN** the user runs `subx-cli --output json match <path>`
- **THEN** no progress-bar frame SHALL be written to stdout or stderr by `indicatif`

#### Scenario: Match table suppressed

- **WHEN** the match command runs with `--output json`
- **THEN** the formatted match table from `src/cli/table.rs` SHALL NOT be rendered on stdout

#### Scenario: AI Analysis debug block is suppressed in JSON mode

- **GIVEN** the `match` command runs with `--output json` against a mocked AI provider that returns at least one candidate match
- **WHEN** the command completes
- **THEN** stderr SHALL NOT contain the byte sequence `🔍 AI Analysis Results:`, SHALL NOT contain the substring `Total matches:`, SHALL NOT contain any line beginning with `   - file_`, AND SHALL NOT contain the substring `Preview:`

#### Scenario: Conflict-resolution warnings are suppressed in JSON mode

- **GIVEN** the `match` command runs with `--output json` against a mocked AI provider where at least one match would trigger `ConflictResolution::Skip` because the target filename already exists
- **WHEN** the command completes
- **THEN** stderr SHALL NOT contain the substring `Warning: Skipping relocation` and SHALL NOT contain the substring `Warning: Conflict resolution prompt not implemented`

#### Scenario: Free-form eprintln is forbidden in JSON mode

- **GIVEN** any subcommand runs with `--output json`
- **WHEN** the implementation reaches a code path that would otherwise call `eprintln!` or `println!` for human-oriented progress, status, or debug output
- **THEN** that call site SHALL instead route the message through `Reporter` (`diagnostic`, `warn`, `ai_usage` or `progress`), the CLI's `TerminalReporter` SHALL suppress it in JSON mode, and the module SHALL NOT reference `crate::cli::output::active_mode()` at all
