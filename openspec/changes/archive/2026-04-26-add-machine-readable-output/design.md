## Context

SubX-CLI today ships a polished interactive UI: colored status symbols
(`src/cli/ui.rs`), `indicatif` progress bars sized via
`general.enable_progress_bar`, an AI match results table
(`src/cli/table.rs`), and free-form per-command prose. Errors flow through
`SubXError` (`src/error.rs`) which already exposes per-category exit codes
(1–6) at the process boundary in `src/main.rs`. There is, however, no
machine-readable output contract: scripted callers must scrape English
text that is allowed to evolve, and there is no stable way to extract
structured data such as match candidates, sync offsets, conversion plans,
detected encodings, or cache entries.

This change introduces a versioned JSON output mode while keeping today's
text mode as the unchanged default. The renderer is hooked at the CLI
boundary so that every existing command can opt in incrementally without
rewriting its core logic.

## Goals / Non-Goals

**Goals:**
- Provide a stable, versioned JSON contract suitable for shell scripts,
  CI pipelines, and third-party tools.
- Cover `match`, `sync`, `convert`, `detect-encoding`, and `cache` with
  rich payload schemas in the first iteration.
- Provide a uniform error envelope for every `SubXError` variant on
  every command (including `translate`, `config`, and any future
  command), so error handling is consistent even where the success
  payload is not yet defined.
- Preserve all existing exit codes, default behavior, and human UX.
- Make stdout/stderr discipline strict in JSON mode so consumers can
  pipe directly into `jq`/`json` without sanitization.

**Non-Goals:**
- Defining a rich success payload for `translate` and `config` (a
  follow-up change). They get the uniform envelope but minimal `data`.
- Producing structured output for `generate-completion` (its stdout is
  by design a shell script).
- Streaming/NDJSON progress events. The contract is one JSON document
  per invocation; richer event streams may be a future capability.
- Localization of error messages (`message` remains the existing
  English text from `user_friendly_message`).

## Decisions

### Decision 1: Top-level flag `--output <text|json>` on `Cli`

Add the flag at the top-level `Cli` struct (`src/cli/mod.rs`) rather
than per-subcommand so users learn it once and tooling can rely on a
uniform invocation. Default is `text`. A complementary `--quiet`
top-level flag suppresses `print_success`/`print_warning` and progress
bars in either mode.

**Flag-positioning constraint (important).** `convert`, `sync`, and
`translate` already define a subcommand-local `--output <PATH>`
argument that designates the **output file**
(`src/cli/convert_args.rs`, `src/cli/sync_args.rs:107`,
`src/cli/translate_args.rs:82`). To avoid clashing with these, the
new `--output text|json` flag SHALL be defined on the root `Cli`
**without** `clap::Arg::global(true)`, so it is only accepted before
the subcommand token. Concretely:

- `subx-cli --output json convert --output a.ass --format ass` is
  unambiguous: the first `--output` is the output **mode** (parsed
  by the root `Cli`), and the second is the output **file** (parsed
  by `ConvertArgs`).
- `subx-cli convert --output json --format ass` would still be
  parsed as `convert`'s file-path argument receiving the value
  `json` and SHALL NOT silently switch to JSON mode. (The early argv
  sniff used for clap-error rendering — Decision 2a — also requires
  `--output <value>` to occur before the first subcommand token,
  matching this constraint.)
- The `--quiet` flag follows the same constraint: top-level only,
  must precede the subcommand token.

Documentation (`docs/command-reference.md`,
`docs/machine-readable-output.md`, both READMEs) SHALL spell out the
"flag must precede the subcommand" rule with examples.

**Alternatives considered:**
- Per-command `--json` flag: rejected because each subcommand would
  need redundant plumbing and the flag surface would drift.
- Renaming the new flag to `--output-format` (or `--format`) to
  sidestep the name collision: rejected. `--format` is already used
  by `convert` for the target subtitle format. `--output-format` is
  ergonomically heavier and the env-var pairing
  (`SUBX_OUTPUT` / `SUBX_OUTPUT_FORMAT`) becomes inconsistent. The
  positional constraint above is the chosen mitigation.
- Environment variable only (`SUBX_OUTPUT=json`): rejected as a
  primary mechanism but kept as an optional override (resolved before
  arg parsing in `cli::run_with_config`) for use inside scripts that
  cannot easily change argv.

### Decision 2: Renderer abstraction in `src/cli/output.rs`

Introduce an `OutputRenderer` enum/trait with two implementations:
`TextRenderer` (delegates to today's `ui` helpers and table) and
`JsonRenderer` (buffers a typed payload, emits one JSON document on
finalize). Each command receives an `&OutputRenderer` from the
dispatcher, replacing direct calls to `print_success` / `print_warning`
/ `display_match_results` for content destined for stdout. The renderer
owns stdout exclusively in JSON mode.

All direct `println!` / `print!` / `eprintln!` / `writeln!(io::stdout())`
call sites currently scattered across `src/commands/*.rs`,
`src/cli/ui.rs`, `src/cli/table.rs`, the `match` command's parallel
progress bar (`src/commands/match_command.rs`), and the existing
ad-hoc `cache status --json` path (`src/cli/cache_args.rs:64`,
honoured by `src/commands/cache_command.rs`) SHALL be audited and
re-routed through the renderer (see tasks §1.7 and §7.4). The
pre-existing `cache status --json` flag remains accepted but becomes
a thin alias for `--output json cache status`; no other cache
subcommand currently exposes a `--json` flag, and none SHALL be
added.

**Alternatives considered:**
- Inline `if json { … }` branches per command: rejected as it would
  scatter formatting concerns and make snapshot testing harder.
- A general "event bus" abstraction: rejected as over-engineered for
  the one-document-per-invocation contract.

### Decision 2a: CLI parsing flow at the process boundary

`Cli::parse()` aborts the process with a clap-formatted text message on
parse errors, so a naive implementation cannot honor `--output json`
when the user mistypes a flag. The process boundary SHALL therefore use
the following flow:

1. `main.rs` performs an **early argv/env sniff** to determine the
   tentative `OutputMode` *before* clap runs. The sniff scans `env::args_os()`
   for `--output <value>`, `--output=<value>`, and reads `SUBX_OUTPUT`.
   It is intentionally permissive: if it cannot resolve a mode, the
   tentative mode defaults to `text`.
2. `main.rs` then calls `Cli::try_parse()` (not `parse()`). On
   `Err(clap::Error)`:
   - In tentative `text` mode, the clap error is rendered exactly as
     today (preserving help/usage formatting) and the process exits
     with `clap::Error::exit_code()`.
   - In tentative `json` mode, `main.rs` emits a synthetic JSON error
     envelope on stdout with `command == null` (or, if the sniff
     observed a subcommand token, the matched subcommand string),
     `status == "error"`, `error.category == "argument_parsing"`,
     `error.code == "E_ARGUMENT_PARSING"`, `error.exit_code` equal to
     the clap exit code, and `error.message` equal to the rendered
     clap message with ANSI styling stripped. The process then exits
     with the clap exit code.
3. On successful parse, `cli::run_with_config` SHALL return a
   `RunOutcome { output_mode: OutputMode, command: &'static str,
   result: Result<(), SubXError> }` (or equivalent struct). `main.rs`
   uses the `output_mode` and `command` to render the final envelope
   without re-parsing argv. `cli::run` keeps a `Result<()>` shim for
   backward compatibility with existing callers but is no longer the
   public entry point used by `main.rs`. Note that clap's `--help` and
   `--version` paths are EXEMPT from JSON envelope wrapping: clap
   emits those via `Err(clap::Error)` with `ErrorKind::DisplayHelp`,
   `ErrorKind::DisplayVersion`, or
   `ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand` and an exit
   code of `0`. In tentative `json` mode, those three kinds SHALL be
   short-circuited to clap's own text rendering (preserving today's
   help/version output verbatim on stdout/stderr) and exit `0`; the
   synthetic JSON envelope path therefore wraps only *actual* parse
   failures (any other `ErrorKind`).
4. `SUBX_OUTPUT` is consulted both in step 1 (for clap-error
   rendering) and inside the parsed `Cli` resolution (so an explicit
   `--output text` still wins over `SUBX_OUTPUT=json`).

This flow guarantees that **every invocation of the binary in JSON
mode emits exactly one JSON document on stdout**, including the cases
where clap rejects the arguments before any subcommand runs.

### Decision 3: Stable JSON envelope

```jsonc
{
  "schema_version": "1.0",
  "command": "match",
  "status": "ok",                  // or "error"
  "data": { /* command-specific */ },
  "warnings": [ { "code": "...", "message": "..." } ],
  "error": {                       // only when status == "error"
    "code": "E_AI_SERVICE",        // stable machine code
    "category": "ai_service",      // stable kebab/snake category
    "exit_code": 3,                // matches SubXError::exit_code
    "message": "...",              // user_friendly_message text
    "details": { /* optional */ }
  }
}
```

`schema_version` follows semver. Additive payload fields are minor
bumps; removing/renaming is a major bump and requires a new change
proposal.

### Decision 4: Map `SubXError` to stable `category` and `code`

Add `pub fn category(&self) -> &'static str` and
`pub fn machine_code(&self) -> &'static str` to `SubXError`. The
mapping is exhaustive (every variant) and locked by tests. Both helpers
are pure; they do not change `Display`, `exit_code`, or
`user_friendly_message`. `main.rs` consults them to render the error
envelope when JSON mode is active; otherwise the existing text path is
unchanged.

### Decision 5: Strict stdout/stderr discipline in JSON mode

- Stdout: exactly one JSON document followed by `\n`. No ANSI codes,
  no progress bars, no `✓`/`✗`/`⚠` prefixes, no match table.
- Stderr: free-form diagnostic logs are still allowed (so `tracing`
  output remains useful), but `print_error` SHALL strip color/symbol
  styling so logs stay greppable.
- Progress bars are force-hidden via
  `ProgressBar::set_draw_target(ProgressDrawTarget::hidden())`
  regardless of `general.enable_progress_bar`.
- The match results table (`display_match_results`) is suppressed.
- Tracing/logging continues to write to stderr.

### Decision 6: Per-command success payloads (initial coverage)

- `match.data`:
  - `dry_run: bool`
  - `confidence_threshold: u8` (0–100)
  - `candidates: [{ video, subtitle, confidence, accepted, reason? }]`
  - `operations: [{ kind: "rename"|"copy"|"move", source, target, applied, status, error? }]`
  - `summary: { total_candidates, accepted, applied, skipped, failed }`
- `sync.data`:
  - `method: "vad"|"manual"|...`
  - `inputs: [{ subtitle, video?, detected_offset_ms, applied_offset_ms, status, error? }]`
  - `operations: [{ subtitle, before_ms, after_ms, applied, status, error? }]`
- `convert.data`:
  - `conversions: [{ input, output, source_format, target_format, encoding, applied, status, error? }]`
- `detect-encoding.data`:
  - `files: [{ path, encoding, confidence, has_bom, status, error? }]`
- `cache.data`:
  - `list`: `{ entries: [...] }`
  - `status`: `{ total, pending, applied, ... }`
  - `clear`: `{ removed }`
  - `rollback`: `{ rolled_back }`
  - `apply`: `{ applied, failed, items: [{ id, status, error? }] }`

Each per-item `status` is `"ok"` or `"error"`. When `"error"`, the
per-item `error` object SHALL contain `code`, `category`, and
`message` keys (same shapes as the top-level error envelope minus
`exit_code`). The top-level envelope SHALL remain `status == "ok"`
whenever the command made forward progress on at least one item;
whole-command failures (configuration errors, missing inputs, fatal
I/O before any work) continue to emit a top-level error envelope.
The envelope MAY also expose a top-level `errors` array (mirror of
per-item failures, for callers that prefer flat iteration) without
breaking forward-compatibility. **This top-level mirror is
explicitly DEFERRED** beyond the v1.0 implementation: it is a
forward-compatibility allowance for a future minor schema-version
bump, not a v1.0 task. The absence of a corresponding entry in
`tasks.md` is therefore intentional, and v1.0 consumers SHALL
iterate the per-item arrays to enumerate failures.

Exact field names are committed as part of `specs/machine-readable-output/spec.md`.

### Decision 6a: `generate-completion` rejects `--output json`

`generate-completion`'s stdout is a shell script and is incompatible
with the JSON envelope contract. When invoked with `--output json` (or
`SUBX_OUTPUT=json`), the command SHALL refuse to run and emit a
top-level JSON error envelope with `command == "generate-completion"`,
`error.category == "command_execution"`, `error.code ==
"E_OUTPUT_MODE_UNSUPPORTED"`, `error.exit_code` equal to
`SubXError::CommandExecution(_).exit_code()` (which currently resolves
to `1` because `CommandExecution` falls through the wildcard arm of
`SubXError::exit_code` in `src/error.rs`), and a human-readable
`message`. No shell-completion script SHALL be written to stdout in
this case. The numeric value `1` is **not** part of the contract for
this rejection — the contract is "the exit code returned by
`SubXError::CommandExecution::exit_code()`"; the actual number will
follow any future change to that mapping without requiring a schema
bump.

In `text` mode (the default) the existing behavior is unchanged.

**Alternatives considered:**
- Silently ignore `--output json` and emit the shell script: rejected
  because callers piping stdout to `jq` would receive non-JSON bytes
  and fail loudly with a confusing error far from the cause.
- Emit a JSON envelope that wraps the script in `data.script`:
  rejected because shell completion files are generally consumed via
  `eval "$(subx-cli generate-completion bash)"` or written to a file,
  neither of which is well-served by JSON wrapping.

### Decision 7: Translate and config emit the envelope only

In this iteration `translate` and `config` honor `--output json` to the
extent of producing a valid envelope (`data` may be a minimal object
such as `{ "translated_files": [...] }` or `{ "config": { ... } }`)
and emitting the same error envelope. Rich payload definitions are
deferred to a future change.

### Decision 8: Snapshot testing under `tests/cli/output_format_*.rs`

Each covered command gets snapshot tests for: success path, dry-run
path (where applicable), and at least one error path per error
category that the command can produce. Tests use the existing
`TestConfigService` and `MockOpenAITestHelper` infrastructure and
assert the JSON envelope shape (schema_version, command, status,
required keys) plus stable subset of the payload (volatile fields like
absolute paths are normalized in fixtures).

### Decision 9: Documentation

- New `docs/machine-readable-output.md` describes the envelope, error
  codes, per-command schemas, and scripting recipes.
- `docs/command-reference.md` gains a "JSON output" section per
  covered command.
- `README.md` and `README.zh-TW.md` get a short "Scripting" callout.
- `CHANGELOG.md` entry under `### Added`.

## Risks / Trade-offs

- [Risk] Renderer plumbing leaks into the text path and accidentally
  changes the human UI. → Mitigation: `TextRenderer` is a thin shim
  over today's `ui` helpers; existing CLI snapshot/integration tests
  stay green; no string changes in text mode.
- [Risk] Schema churn breaks scripts. → Mitigation: explicit
  `schema_version`, additive-only minor bumps, exhaustive snapshot
  tests, and a documented stability policy in
  `docs/machine-readable-output.md`.
- [Risk] Stderr noise (tracing, AI provider diagnostics) corrupts
  pipelines that swallow stderr. → Mitigation: stdout is guaranteed
  pure JSON; users redirect stderr if needed; logs are not part of
  the contract.
- [Risk] Some commands write artifacts (renames, conversions) before
  fatal errors. → Mitigation: the error envelope's optional `details`
  carries `partial_results` (already-applied operations) so scripts
  can reconcile without re-scanning the filesystem.
- [Risk] Adding `category`/`machine_code` to `SubXError` is fragile if
  variants are added later without updating the mapping. → Mitigation:
  exhaustive `match` (no `_` arm) plus a unit test that round-trips
  every variant.
- [Trade-off] The first iteration leaves `translate`/`config` with
  thin payloads. Acceptable because the uniform error envelope already
  covers the most common scripting need (detecting failure category)
  for those commands.

## Migration Plan

1. Land the renderer scaffolding and global flag with `text` as the
   only behavior change observable to users (no-op).
2. Migrate covered commands one-by-one behind feature-gated snapshot
   tests; each PR keeps text mode byte-identical.
3. After all covered commands ship, publish `docs/machine-readable-output.md`
   and announce the contract in `CHANGELOG.md` under `### Added`.
4. Rollback strategy: revert the renderer wiring; the `--output` flag
   becomes a no-op silently or the change is reverted wholesale. No
   data migrations are required because the feature is purely
   additive.

## Open Questions

- Should the envelope include a top-level `duration_ms` for
  benchmarking? Currently deferred; can be added in a minor bump.
- Should a `cache list` subcommand be added (with pagination in JSON
  mode)? Deferred to a follow-up change; the current CLI does not
  expose `cache list` and adding it requires new persistence/indexing
  work in the matcher cache layer.

## Resolved Questions

- **Q:** Should `--output json` imply `--quiet`?
  **A:** Resolved. `--output json` is implicitly quiet for stdout
  (only the JSON envelope is ever written, by construction — see the
  Stdout/stderr discipline requirement). `--quiet` is orthogonal and
  *additionally* suppresses the free-form stderr status chatter that
  JSON mode otherwise allows (`tracing` info logs, AI provider
  diagnostics). In `text` mode, `--quiet` retains its current
  semantics of silencing `print_success`/`print_warning`/progress
  bars. The two flags are independent and may be combined.
