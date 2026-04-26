## Why

SubX-CLI is increasingly used inside scripts, batch pipelines, CI jobs, and
higher-level tooling that need to parse command output reliably. Today every
command prints free-form, color-decorated, human-oriented text via
`src/cli/ui.rs` (`✓`/`✗`/`⚠` symbols, ANSI colors, progress bars, formatted
match tables), and structured information (match candidates, confidence,
sync offsets, conversion plans, encoding detections, cache entries) is
intermixed with status chatter on stdout/stderr. There is no stable contract
for programmatic consumers, so callers must scrape ad-hoc text that has
already changed several times across releases. Exit codes are well-defined
in `SubXError::exit_code` (1–6), but error details are only visible as
prose, which makes it impossible for scripts to react to specific error
categories without parsing English strings.

A consistent, versioned, machine-readable output mode unlocks safe
automation, third-party integrations, and end-to-end testing while
preserving today's polished interactive UX as the default.

## What Changes

- Add a top-level `--output <text|json>` flag on the root `Cli` (in
  `src/cli/mod.rs`), defaulting to `text` so the existing human UX is
  unchanged. The flag SHALL be defined on `Cli` itself (not as
  `clap::Arg::global(true)`) so it must appear **before** the
  subcommand token: `subx-cli --output json convert ...`. This
  positional constraint avoids colliding with the existing
  subcommand-local `--output <PATH>` arguments on `convert`, `sync`,
  and `translate` (each of which uses `--output PATH` to designate an
  output **file**). Tooling/docs SHALL make the precedence rule
  explicit. (The alternative of renaming the new flag to
  `--output-format` was considered and rejected — see `design.md` —
  to keep the flag name short and aligned with the `SUBX_OUTPUT`
  environment variable.)
- Add a global `--quiet` flag (top-level on `Cli`, with the same
  "must precede the subcommand" caveat as `--output`) whose semantics
  are clarified explicitly:
  in `--output json` mode, stdout is *implicitly* quiet (only the JSON
  envelope is ever written, see "Stdout/stderr discipline" below);
  `--quiet` additionally suppresses the free-form stderr status chatter
  that JSON mode otherwise allows (`tracing` info logs, AI provider
  diagnostics, etc.). In `text` mode, `--quiet` retains its current
  behavior of suppressing `print_success`/`print_warning`/progress bars.
- Define a stable JSON envelope emitted on stdout for every supported
  command, with shape:
  - `schema_version` (semver-style string, starts at `"1.0"`).
  - `command` (e.g. `"match"`, `"sync"`, `"convert"`,
    `"detect-encoding"`, `"translate"`, `"cache"`).
  - `status` (`"ok"` or `"error"`).
  - `data` (command-specific payload object; the `data` key SHALL be
    omitted entirely when `status == "error"`, never serialized as
    `null`).
  - `error` (object with `code`, `category`, `exit_code`, `message`,
    optional `details`; only present when `status = "error"`).
  - Optional `warnings` array of `{code, message}`.
- Specify stdout/stderr discipline in JSON mode:
  - Stdout SHALL contain exactly one JSON document terminated by a
    newline (NDJSON-friendly when callers stream multiple invocations).
  - Stderr MAY contain free-form diagnostic logs but SHALL NOT contain
    any JSON envelope, status symbols, or progress bars.
  - Progress bars SHALL be force-hidden (regardless of
    `general.enable_progress_bar`); `print_success`/`print_warning`/
    `print_error` SHALL be suppressed on stdout when `--output json` is
    active.
- Define a uniform error envelope mapping every `SubXError` variant to a
  stable `category` string (`io`, `config`, `subtitle_format`,
  `ai_service`, `api`, `audio_processing`, `file_matching`,
  `file_already_exists`, `file_not_found`, `invalid_file_name`,
  `file_operation_failed`, `command_execution`, `no_input_specified`,
  `invalid_path`, `path_not_found`, `directory_read_error`,
  `invalid_sync_configuration`, `unsupported_file_type`, `other`) plus a
  stable machine `code` (e.g. `E_AI_SERVICE`, `E_SUBTITLE_FORMAT`). The
  numeric `exit_code` SHALL match `SubXError::exit_code` exactly so
  existing scripts that only rely on exit status keep working. A
  synthetic envelope (not backed by `SubXError`) SHALL also cover clap
  argument parsing failures so JSON consumers always see a valid
  envelope; the additional category `argument_parsing` is therefore
  synthetic — it is emitted only on clap parse errors, has no
  corresponding `SubXError` variant, and is not part of the closed
  `SubXError`-derived category set above (see "CLI parsing flow" in the
  design document).
- Specify the CLI parsing flow at the process boundary so JSON mode is
  honored even on clap parse errors:
  - `main.rs` SHALL detect the active output mode by sniffing argv for
    `--output <value>` / `--output=<value>` and the `SUBX_OUTPUT`
    environment variable **before** invoking clap.
  - clap SHALL be invoked via `Cli::try_parse()` (not `parse()`); when
    it returns an `Err(clap::Error)`, `main.rs` SHALL render either the
    default clap message (text mode) or a synthetic JSON error envelope
    (JSON mode) using category `argument_parsing` and the clap-provided
    exit code.
  - `cli::run`/`cli::run_with_config` SHALL return a structured
    `RunOutcome` (or equivalent) carrying the resolved `OutputMode` plus
    the `Result<(), SubXError>` so `main.rs` can render the final JSON
    envelope without re-parsing argv.
- Specify per-item status semantics for batch commands so the existing
  per-file error isolation contract (already required by
  `format-conversion`) is preserved in JSON mode:
  - For batch-capable commands (`convert`, `detect-encoding`, `sync`,
    the parallel branches of `match`, and the parallel branches of
    `cache apply`), each per-file entry in the
    success payload SHALL include a `status` field (`"ok"` or
    `"error"`) and, when `status == "error"`, an `error` object with
    `code`, `category`, and `message` (same shape as the top-level
    error envelope's `error`, minus `exit_code`).
  - The top-level envelope SHALL remain `status == "ok"` whenever any
    file processed successfully (matching today's batch semantics).
    Whole-command failures (config invalid, no inputs, fatal I/O before
    any work, all files failing in a non-isolated path) SHALL continue
    to use the top-level error envelope.
  - The envelope MAY also carry a top-level `errors` array summarizing
    per-item failures for callers that prefer flat iteration; this is
    purely a convenience mirror of the per-item `error` fields.
    Implementing this top-level mirror is **explicitly DEFERRED** to a
    future minor schema-version bump: it is documented here as a
    forward-compatibility allowance only, and is intentionally absent
    from `tasks.md` for the v1.0 implementation. v1.0 consumers SHALL
    obtain failure information by iterating the per-item arrays.
- Specify behavior for `generate-completion`, whose stdout is a shell
  script and is incompatible with the JSON envelope contract:
  - When invoked with `--output json` (explicitly or via
    `SUBX_OUTPUT=json`), `generate-completion` SHALL refuse to run and
    emit a top-level JSON error envelope with `error.category ==
    "command_execution"`, `error.code == "E_OUTPUT_MODE_UNSUPPORTED"`,
    and a non-zero exit code equal to
    `SubXError::CommandExecution(_).exit_code()` (currently `1`,
    because `CommandExecution` falls through the wildcard arm of
    `SubXError::exit_code` in `src/error.rs`). Stdout SHALL contain
    only the envelope; no shell script SHALL be written.
  - In `text` mode (default) the existing behavior is unchanged.
- Cover the following commands in the first iteration:
  - `match` — enumerate proposed pairs with confidence, planned file
    operations, dry-run flag, and final applied-operation summary.
  - `sync` — report detected method, computed offsets, per-file before/
    after timing, and operations performed.
  - `convert` — report each input/output file pair, source/target
    format, encoding, and operation status.
  - `detect-encoding` — report each path with detected encoding,
    confidence, and BOM status.
  - `cache status`/`cache clear`/`cache rollback`/`cache apply` —
    report action results and counts in structured form. (A future
    `cache list` subcommand is intentionally deferred; the current CLI
    does not expose a `cache list` action.)
- Preserve the pre-existing `cache status --json` flag as a thin,
  backward-compatible alias that forwards to the global `--output json`
  renderer and emits byte-identical output. No other `cache` subcommand
  SHALL grow a new `--json` flag; the global `--output json` is the
  single mechanism for selecting JSON output across the rest of the
  `cache` subcommand surface.
- Defer `translate` and `config` to a follow-up iteration (still emit a
  minimal JSON envelope so scripts get a uniform error contract, but
  rich payload definitions are out of scope here). `generate-completion`
  rejects `--output json` with an error envelope (see above) so that
  stdout under JSON mode is always a valid envelope.
- Add a stable mapping table from each command's textual output to its
  JSON payload schema, documented in `docs/command-reference.md` and a
  new `docs/machine-readable-output.md` reference.
- Snapshot-test JSON outputs (using `insta` or hand-rolled fixtures
  consistent with the existing test conventions) for representative
  success and failure paths of each covered command. Include scripting
  smoke tests that pipe stdout through `jq` (under
  `tests/cli/output_format_*.rs`) to prove the contract.
- Preserve backward compatibility: default behavior, default exit codes,
  and the existing text UI remain untouched. The `text` mode SHALL
  remain the contract-free, human-oriented format and MAY continue to
  evolve.

## Capabilities

### New Capabilities
- `machine-readable-output`: Defines the global `--output`/`--quiet`
  flags, the versioned JSON envelope, the stdout/stderr discipline,
  the error envelope shape, the per-command payload schemas for the
  initially-covered commands, and the contract that text mode is
  unaffected. Implemented across `src/cli/mod.rs`, a new
  `src/cli/output.rs` (renderer abstraction), `src/commands/dispatcher.rs`,
  and per-command modules under `src/commands/`.

### Modified Capabilities
- `error-handling`: Add a requirement that every `SubXError` variant
  exposes a stable machine-readable category and code suitable for the
  JSON error envelope, while keeping today's `exit_code` and
  `user_friendly_message` contracts unchanged.
- `progress-reporting`: Add a requirement that progress bars and the
  `print_success`/`print_warning` helpers SHALL suppress all stdout
  output when JSON output mode is selected, and that `print_error`
  SHALL NOT emit ANSI styling on stderr in JSON mode (so logs remain
  greppable). The existing text-mode behavior is unchanged.
- `subtitle-matching`: Add a requirement that, when `--output json` is
  active, the match command emits the structured match payload
  (candidates, applied operations, dry-run flag) on stdout instead of
  the colored result table, and SHALL NOT print the human-friendly
  table.
- `timeline-sync`: Add a requirement that, when `--output json` is
  active, the sync command emits the structured sync result payload
  (method, offsets, before/after timings, operations) on stdout instead
  of the human progress chatter.
- `format-conversion`: Add a requirement that, when `--output json` is
  active, the convert command emits a structured per-file conversion
  result payload on stdout.
- `encoding-detection`: Add a requirement that, when `--output json` is
  active, the detect-encoding command emits a structured per-file
  encoding result payload on stdout instead of the human table.
- `cache-management`: Add a requirement that cache subcommands (`list`,
  `status`, `clear`, `rollback`, `apply`) emit structured payloads on
  stdout when `--output json` is active.

## Impact

- Affected code:
  - `src/cli/mod.rs` — new global `--output` and `--quiet` flags on
    `Cli`.
  - `src/cli/ui.rs` — make `print_success`/`print_warning`/
    `print_error`/`create_progress_bar` aware of the active output
    mode (via a process-scoped renderer handle injected by the
    dispatcher).
  - new `src/cli/output.rs` — output mode enum, JSON envelope writer,
    schema-version constant.
  - `src/commands/dispatcher.rs` — propagate the chosen output mode
    into each command's execution path.
  - `src/commands/match_command.rs`, `sync_command.rs`,
    `convert_command.rs`, `detect_encoding_command.rs`,
    `cache_command.rs`, `translate_command.rs`,
    `config_command.rs` — emit structured payloads through the
    renderer instead of printing directly when JSON is active.
  - `src/error.rs` — add `category()` and `machine_code()` helpers on
    `SubXError`; `main.rs` consumes them when the top-level error is
    rendered in JSON mode.
- Dependencies: no new runtime crates required; `serde`/`serde_json`
  are already in the dependency graph. Snapshot tests may use the
  existing test infrastructure under `tests/common/`.
- Documentation:
  - New `docs/machine-readable-output.md` reference page.
  - Updates to `docs/command-reference.md` and both READMEs to
    document `--output json` per command.
  - `CHANGELOG.md` entry under `### Added`.
- Backward compatibility: fully preserved. No existing flag, exit
  code, or default output stream is changed. Scripts that rely solely
  on exit codes continue to work unmodified.
- Risk: small risk of accidental regression in interactive UX if the
  renderer abstraction leaks into the text path; mitigated by
  retaining the existing helpers as the text-mode implementation and
  by snapshot-testing both modes.
