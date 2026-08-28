## MODIFIED Requirements

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

## ADDED Requirements

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
