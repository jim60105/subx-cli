# Machine-Readable Output

## Purpose

Expose a stable, machine-readable JSON output mode for every SubX-CLI subcommand so that scripts, automation pipelines, and downstream tooling can parse SubX results without screen-scraping human-oriented text. The mode is opt-in via a global `--output json` flag (or `SUBX_OUTPUT=json`) and preserves byte-identical behavior in the default `text` mode. Implemented across `src/cli/mod.rs` (root `Cli` flags), `src/main.rs` (process-boundary rendering and clap error handling), `src/cli/ui.rs` (helper suppression), and per-command renderers under `src/commands/`.

## Requirements


### Requirement: Global Output Mode Flag

The CLI SHALL expose a top-level flag `--output <text|json>` on the root `Cli` struct (`src/cli/mod.rs`) that selects the output rendering mode for every subcommand. The default value SHALL be `text`, which preserves the existing human-oriented UI exactly. When the value is `json`, every supported subcommand SHALL emit a single JSON envelope on stdout in place of free-form prose, status symbols, progress bars, and result tables. The flag SHALL also be settable via the environment variable `SUBX_OUTPUT` with the same accepted values; an explicit `--output` argument SHALL take precedence over the environment variable.

The flag SHALL be defined on the root `Cli` **without** `clap::Arg::global(true)` so that it is only accepted **before** the subcommand token (e.g., `subx-cli --output json convert ...`). This positional constraint avoids colliding with the existing subcommand-local `--output <PATH>` arguments on `convert`, `sync`, and `translate` (each of which already designates an output **file**). An `--output` token appearing **after** the subcommand token SHALL be parsed by the subcommand and SHALL NOT change the output mode; the mode in that case is governed solely by `SUBX_OUTPUT` and any earlier root-level `--output`.

#### Scenario: Default mode is text
- **WHEN** the user runs any subcommand without `--output` and without `SUBX_OUTPUT` set
- **THEN** the CLI SHALL render exactly the existing text UI (status symbols, progress bars, tables) and SHALL NOT emit any JSON document on stdout

#### Scenario: Explicit JSON mode (flag before subcommand)
- **WHEN** the user runs `subx-cli --output json <subcommand>`
- **THEN** stdout SHALL contain exactly one JSON document (defined by the JSON Envelope requirement) terminated by a single trailing newline, and SHALL NOT contain any progress-bar frames, ANSI styling, or status-symbol prefixes

#### Scenario: Subcommand-local --output is unaffected
- **GIVEN** the user runs `subx-cli convert --input a.srt --output a.ass --format ass` without preceding `--output`
- **WHEN** the binary runs
- **THEN** the value `a.ass` SHALL be parsed by `ConvertArgs` as the output **file path**, the output mode SHALL remain `text` (or whatever `SUBX_OUTPUT` selects), and the CLI SHALL NOT misinterpret `a.ass` as an output mode

#### Scenario: Mode flag and subcommand --output coexist
- **WHEN** the user runs `subx-cli --output json convert --input a.srt --output a.ass --format ass`
- **THEN** the CLI SHALL emit a JSON envelope on stdout, AND `ConvertArgs.output` SHALL be `a.ass`

#### Scenario: Environment-variable selection
- **GIVEN** the environment variable `SUBX_OUTPUT=json` is set and `--output` is not passed
- **WHEN** the CLI runs any supported subcommand
- **THEN** the CLI SHALL behave as if `--output json` were passed

#### Scenario: Argument overrides environment
- **GIVEN** `SUBX_OUTPUT=json` is set
- **WHEN** the user runs `subx-cli --output text <subcommand>`
- **THEN** the CLI SHALL render in text mode

### Requirement: Quiet Flag

The CLI SHALL expose a top-level boolean flag `--quiet` on the root `Cli` struct. The two output modes interact with `--quiet` as follows:

- In `text` mode, `--quiet` SHALL suppress ancillary status output: success messages (`print_success`), warning messages (`print_warning`), progress bars, and the AI match results table.
- In `json` mode, stdout is *implicitly* quiet by construction (only the JSON envelope is ever emitted, see "Stdout/Stderr Discipline in JSON Mode"). `--quiet` is therefore orthogonal: it does not change stdout, but it SHALL additionally suppress the free-form stderr status chatter that JSON mode would otherwise allow (e.g., `tracing` info logs, AI provider diagnostics, indicatif debug output).

`--quiet` SHALL never suppress the JSON envelope on stdout in JSON mode, and SHALL never suppress the process exit code.

#### Scenario: Quiet in text mode silences status chatter
- **WHEN** the user runs `subx-cli --quiet match <path>` in text mode
- **THEN** stdout SHALL NOT contain `print_success` or `print_warning` lines and SHALL NOT contain progress-bar output

#### Scenario: JSON mode is implicitly quiet on stdout
- **WHEN** the user runs `subx-cli --output json match <path>` without `--quiet`
- **THEN** stdout SHALL contain exactly one JSON envelope and SHALL NOT contain any `print_success`/`print_warning`/progress-bar/match-table bytes, regardless of `general.enable_progress_bar`

#### Scenario: --quiet in JSON mode additionally silences stderr chatter
- **WHEN** the user runs `subx-cli --quiet --output json match <path>`
- **THEN** stdout SHALL still contain exactly one JSON envelope, AND stderr SHALL NOT contain any free-form status chatter (tracing info logs, provider diagnostics) that JSON mode would otherwise have allowed

#### Scenario: --quiet must precede the subcommand token
- **GIVEN** no subcommand currently defines its own `--quiet` flag
- **WHEN** the user runs `subx-cli --quiet match <path>` (flag before subcommand)
- **THEN** `Cli.quiet` SHALL parse as `true` and the quiet semantics defined above SHALL apply
- **AND WHEN** the user runs `subx-cli match --quiet <path>` (flag after subcommand)
- **THEN** clap SHALL reject the invocation as an unknown argument for the subcommand (mirroring the placement constraint of `--output`), and `Cli.quiet` SHALL NOT be silently set

### Requirement: JSON Envelope Schema

When the active output mode is `json`, every supported subcommand SHALL emit on stdout a single UTF-8 JSON object, terminated by a single newline character, with the following top-level keys:

- `schema_version` (string, semver-like, starts at `"1.0"`).
- `command` (string identifying the subcommand: one of `"match"`, `"sync"`, `"convert"`, `"detect-encoding"`, `"translate"`, `"cache"`, `"config"`).
- `status` (string: `"ok"` or `"error"`).
- `data` (object; present and command-specific when `status == "ok"`; the `data` key SHALL be omitted entirely when `status == "error"`, never serialized as `null`).
- `warnings` (optional array of objects, each with `code` (string) and `message` (string)).
- `error` (object; present only when `status == "error"`; defined by the Error Envelope requirement).

Additional top-level keys MAY be added in future minor schema versions and SHALL be additive. Removing or renaming any documented key requires a major schema-version bump.

#### Scenario: Successful invocation envelope shape
- **WHEN** a supported command finishes successfully in JSON mode
- **THEN** the emitted JSON object SHALL contain `schema_version`, `command`, `status == "ok"`, and a `data` object, and SHALL NOT contain an `error` key

#### Scenario: Failed invocation envelope shape
- **WHEN** a supported command fails with any `SubXError` in JSON mode
- **THEN** the emitted JSON object SHALL contain `schema_version`, `command`, `status == "error"`, and an `error` object, and the process SHALL exit with the corresponding `SubXError::exit_code`

#### Scenario: Trailing newline only
- **WHEN** any JSON envelope is written to stdout
- **THEN** stdout SHALL end with exactly one `\n` after the closing `}` and contain no additional bytes after it

### Requirement: Error Envelope

When `status == "error"`, the JSON envelope SHALL contain an `error` object with the following keys:

- `code` (stable string identifier such as `E_AI_SERVICE`, `E_SUBTITLE_FORMAT`, `E_FILE_NOT_FOUND`, `E_ARGUMENT_PARSING`, `E_OUTPUT_MODE_UNSUPPORTED`).
- `category` (stable snake_case string mirroring the `SubXError` variant family, such as `io`, `config`, `subtitle_format`, `ai_service`, `api`, `audio_processing`, `file_matching`, `file_already_exists`, `file_not_found`, `invalid_file_name`, `file_operation_failed`, `command_execution`, `no_input_specified`, `invalid_path`, `path_not_found`, `directory_read_error`, `invalid_sync_configuration`, `unsupported_file_type`, `argument_parsing`, `other`).
- `exit_code` (integer matching `SubXError::exit_code` for the same error, or — for synthetic envelopes such as clap parse failures — the exit code that would have been used by the underlying handler).
- `message` (string equal to the value `SubXError::user_friendly_message` would have produced, or — for synthetic envelopes — the human-readable error text with ANSI styling stripped).
- `details` (optional object carrying extra context such as `path`, `format`, or `partial_results`).

The numeric process exit code SHALL equal `error.exit_code`.

The category set is closed for envelopes derived from `SubXError`; the additional categories `argument_parsing` (used for clap parse failures rendered by `main.rs`) MAY appear without a corresponding `SubXError` variant. Any future additions to the category set are additive and follow the schema-version policy in the Schema Version Stability requirement.

#### Scenario: AI service failure envelope
- **GIVEN** the AI provider returns an error during `match`
- **WHEN** the command runs with `--output json`
- **THEN** the envelope SHALL satisfy `status == "error"`, `error.category == "ai_service"`, `error.exit_code == 3`, and the process SHALL exit with status `3`

#### Scenario: File-not-found envelope on convert
- **GIVEN** the user passes a non-existent input path to `convert`
- **WHEN** the command runs with `--output json`
- **THEN** the envelope's `error.category` SHALL be one of `path_not_found` or `file_not_found`, `error.exit_code` SHALL match `SubXError::exit_code`, and the process exit status SHALL equal `error.exit_code`

#### Scenario: Partial results on mid-run failure
- **GIVEN** `match` has applied one rename successfully and the next operation fails
- **WHEN** the command runs with `--output json`
- **THEN** `error.details.partial_results` SHALL include the already-applied operations so a script can reconcile state without rescanning the filesystem

### Requirement: CLI Parsing Flow Honors Output Mode

The process boundary in `src/main.rs` SHALL guarantee that **every invocation in JSON mode emits exactly one JSON document on stdout**, including invocations that fail clap argument parsing before any subcommand runs. To achieve this:

- `main.rs` SHALL perform an early, permissive sniff of `env::args_os()` and the `SUBX_OUTPUT` environment variable to compute a tentative `OutputMode` *before* invoking clap. The sniff SHALL recognise `--output <value>`, `--output=<value>`, and `SUBX_OUTPUT=<value>` (case-insensitive `text`/`json`) and SHALL default to `text` when ambiguous.
- `main.rs` SHALL invoke clap via `Cli::try_parse()` (not `parse()`).
- On `Err(clap::Error)` in tentative `text` mode, `main.rs` SHALL render the clap error exactly as it does today (preserving help and usage formatting on stderr) and SHALL exit with `clap::Error::exit_code()`.
- On `Err(clap::Error)` in tentative `json` mode, `main.rs` SHALL emit a synthetic JSON error envelope on stdout with `command` set to the matched subcommand string from the sniff (or omitted/`null` when no subcommand was identified), `status == "error"`, `error.category == "argument_parsing"`, `error.code == "E_ARGUMENT_PARSING"`, `error.exit_code` equal to `clap::Error::exit_code()`, and `error.message` equal to the rendered clap message with ANSI styling removed. The process SHALL then exit with the clap exit code.
- On successful parse, `cli::run_with_config` (or its successor) SHALL return a structured outcome that exposes the resolved `OutputMode`, the resolved subcommand identifier (e.g., as a `&'static str` such as `"match"`, `"sync"`, `"convert"`, `"detect-encoding"`, `"translate"`, `"cache"`, `"config"`, `"generate-completion"`, or equivalent), and the command's `Result<(), SubXError>` to `main.rs` so that the final envelope can be rendered without re-parsing argv.
- Clap's `--help` and `--version` output (which clap surfaces via `Err(clap::Error)` with `ErrorKind::DisplayHelp`, `ErrorKind::DisplayVersion`, or `ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand` and an exit code of `0`) is **exempt** from JSON envelope wrapping. Even when tentative `--output json` (or `SUBX_OUTPUT=json`) is active, those three error kinds SHALL be rendered as clap's existing text on stdout/stderr exactly as today, and the process SHALL exit with status `0`. The synthetic JSON error envelope path therefore covers only actual argument-parsing failures (every other `ErrorKind`).

#### Scenario: Unknown flag in text mode preserves clap output
- **GIVEN** the user invokes `subx-cli --bogus-flag match` without any JSON-mode signal
- **WHEN** the binary runs
- **THEN** stderr SHALL contain the standard clap "unexpected argument" message and the process SHALL exit with the clap exit code, identical to pre-change behavior

#### Scenario: Unknown flag in JSON mode emits synthetic envelope
- **GIVEN** the user invokes `subx-cli --output json --bogus-flag match`
- **WHEN** the binary runs
- **THEN** stdout SHALL contain exactly one JSON document with `status == "error"`, `error.category == "argument_parsing"`, `error.code == "E_ARGUMENT_PARSING"`, and `error.exit_code` equal to the clap exit code, AND the process SHALL exit with that exit code

#### Scenario: SUBX_OUTPUT=json triggers envelope on clap failure
- **GIVEN** `SUBX_OUTPUT=json` is set and the user omits a required argument
- **WHEN** the binary runs
- **THEN** stdout SHALL contain a synthetic JSON error envelope with `error.category == "argument_parsing"`, regardless of whether `--output` appeared in argv

#### Scenario: --help is exempt from JSON envelope wrapping
- **GIVEN** the user invokes `subx-cli --output json --help` (or `subx-cli --output json <subcommand> --help`, or sets `SUBX_OUTPUT=json` and runs `--help`)
- **WHEN** clap returns `Err(clap::Error)` with `ErrorKind::DisplayHelp` (or `ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand`) and exit code `0`
- **THEN** the CLI SHALL print clap's standard help text on stdout/stderr exactly as today, SHALL NOT emit a JSON envelope, AND the process SHALL exit with status `0`

#### Scenario: --version is exempt from JSON envelope wrapping
- **GIVEN** the user invokes `subx-cli --output json --version` (or sets `SUBX_OUTPUT=json` and runs `--version`)
- **WHEN** clap returns `Err(clap::Error)` with `ErrorKind::DisplayVersion` and exit code `0`
- **THEN** the CLI SHALL print clap's standard version text on stdout exactly as today, SHALL NOT emit a JSON envelope, AND the process SHALL exit with status `0`

### Requirement: generate-completion Rejects JSON Output Mode

The `generate-completion` subcommand's stdout is, by design, a shell-completion script and is incompatible with the JSON envelope contract. When `generate-completion` is invoked with the active output mode set to `json` (via `--output json` or `SUBX_OUTPUT=json`), the CLI SHALL refuse to generate the script and SHALL emit a top-level JSON error envelope on stdout with:

- `command == "generate-completion"`.
- `status == "error"`.
- `error.category == "command_execution"`.
- `error.code == "E_OUTPUT_MODE_UNSUPPORTED"`.
- `error.exit_code` equal to `SubXError::CommandExecution(_).exit_code()` (which currently resolves to `1` because `CommandExecution` falls through the wildcard arm of `SubXError::exit_code` in `src/error.rs`; the contract pins the helper, not the literal number).
- `error.message` SHALL be a human-readable string explaining that JSON output is not supported for this subcommand and recommending `--output text` (or omitting `--output`).

No shell-completion script SHALL be written to stdout in this case. The process SHALL exit with the same exit code as `error.exit_code`.

In `text` mode (default) the existing behavior is unchanged: the shell-completion script is written to stdout and the process exits with status `0`.

#### Scenario: --output json on generate-completion emits error envelope
- **WHEN** the user runs `subx-cli --output json generate-completion bash`
- **THEN** stdout SHALL contain exactly one JSON envelope with `command == "generate-completion"`, `error.category == "command_execution"`, `error.code == "E_OUTPUT_MODE_UNSUPPORTED"`, no shell-completion bytes SHALL be written, and the process exit code SHALL equal `SubXError::CommandExecution(_).exit_code()`

#### Scenario: text mode generate-completion is unchanged
- **WHEN** the user runs `subx-cli generate-completion bash` without selecting JSON mode
- **THEN** stdout SHALL contain the bash completion script exactly as in pre-change behavior and the process SHALL exit with status `0`

### Requirement: Per-Item Status in Batch Payloads

Batch-capable commands (`convert`, `detect-encoding`, `sync`, and the parallel branches of `match` and `cache apply`) SHALL preserve the existing per-file error isolation contract (already required by the `format-conversion` capability) when running in JSON mode. To achieve this:

- Each per-item entry in a success payload's array (e.g., `data.conversions[i]`, `data.files[i]`, `data.operations[i]`, `data.items[i]`) SHALL include a `status` field with value `"ok"` or `"error"`.
- When a per-item `status == "error"`, the entry SHALL also include an `error` object with keys `code`, `category`, and `message` (same shape as the top-level `error` object minus `exit_code`). The `category` and `code` values SHALL come from `SubXError::category()` / `SubXError::machine_code()` for the underlying variant.
- The top-level envelope SHALL remain `status == "ok"` whenever the command completed its batch loop and made forward progress on at least one item, mirroring today's text-mode behavior where corrupt files are reported on stderr but the command exits successfully.
- Whole-command failures (configuration errors, missing or invalid inputs that prevent the command from starting, fatal I/O before any per-item processing, or a single-input command receiving a fatal error) SHALL emit a top-level error envelope as defined by the Error Envelope requirement; the per-item `status`/`error` fields are not used for these cases.
- The envelope MAY also expose an optional top-level `errors` array — a flat mirror of the per-item `error` objects (each augmented with the originating path/identifier) — for consumers that prefer iterating failures without scanning every per-item array. Adding this array later is a minor schema-version bump.

#### Scenario: Batch convert with one corrupt file keeps top-level ok
- **GIVEN** a batch of three input files where one is corrupt
- **WHEN** the user runs `subx-cli --output json convert -i <dir> --format vtt`
- **THEN** the top-level envelope SHALL satisfy `status == "ok"`, `data.conversions` SHALL contain three entries, two with `status == "ok"` and `applied == true`, one with `status == "error"`, an `error.category == "subtitle_format"`, and the process SHALL exit with status `0`

#### Scenario: Single-input fatal failure uses top-level error envelope
- **GIVEN** a single corrupt input file passed to `convert`
- **WHEN** the user runs `subx-cli --output json convert --input bad.srt --format ass`
- **THEN** the envelope SHALL satisfy `status == "error"`, `error.category == "subtitle_format"`, `error.exit_code == 4`, and the process SHALL exit with status `4`

#### Scenario: Per-item error carries category and code
- **GIVEN** a batch where one file fails with `SubXError::SubtitleFormat`
- **WHEN** the command runs with `--output json`
- **THEN** the failing entry SHALL contain `status == "error"` and `error == { code: "E_SUBTITLE_FORMAT", category: "subtitle_format", message: <user_friendly_message> }`

### Requirement: Stdout/Stderr Discipline in JSON Mode

When the active output mode is `json`, the CLI SHALL guarantee:

- Stdout contains exactly one JSON envelope plus one trailing `\n` and nothing else.
- Stderr MAY contain free-form diagnostic logs (for example, `tracing` output) but SHALL NOT contain any JSON envelope, `✓`/`✗`/`⚠` status symbols, ANSI color escape sequences emitted by `print_success`/`print_warning`/`print_error`, or `indicatif` progress-bar frames.
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

### Requirement: Match Command JSON Payload

When the `match` command runs with `--output json`, the envelope's `data` object SHALL contain at least the following fields:

- `dry_run` (bool).
- `confidence_threshold` (integer, 0–100).
- `candidates` (array of objects, each containing `video` (string path), `subtitle` (string path), `confidence` (integer 0–100), `accepted` (bool), and an optional `reason` (string) when not accepted).
- `operations` (array of objects, each containing `kind` (`"rename"`, `"copy"`, or `"move"`), `source` (string path), `target` (string path), `applied` (bool), `status` (`"ok"` or `"error"`), and an optional `error` (object with `code`, `category`, `message`) when `status == "error"`).
- `summary` (object with integer fields `total_candidates`, `accepted`, `applied`, `skipped`, `failed`).

#### Scenario: Successful match emits structured payload
- **GIVEN** an input directory with one accepted video/subtitle pair and an AI provider configured
- **WHEN** the user runs `subx-cli --output json match <path>`
- **THEN** `data.candidates` SHALL contain one entry with `accepted == true` and the corresponding `data.operations` entry SHALL appear with `applied == true` and `status == "ok"` (live mode) or `applied == false` and `status == "ok"` (dry-run)

#### Scenario: Dry-run flag is reflected
- **WHEN** the user runs `subx-cli --output json match --dry-run <path>`
- **THEN** `data.dry_run == true` and every entry in `data.operations` SHALL have `applied == false`

### Requirement: Sync Command JSON Payload

When the `sync` command runs with `--output json`, the envelope's `data` object SHALL contain at least:

- `method` (string: e.g., `"vad"`, `"manual"`).
- `inputs` (array of objects with `subtitle` (string path), optional `video` (string path), `detected_offset_ms` (integer, MAY be null when not detected), `applied_offset_ms` (integer), `status` (`"ok"`/`"error"`), optional `error`).
- `operations` (array of objects with `subtitle` (string path), `before_ms` (integer), `after_ms` (integer), `applied` (bool), `status` (`"ok"`/`"error"`), optional `error`).

#### Scenario: VAD sync emits offsets
- **GIVEN** a subtitle/video pair and VAD-based sync
- **WHEN** the user runs `subx-cli --output json sync <args>`
- **THEN** `data.method == "vad"`, `data.inputs[0].detected_offset_ms` SHALL be an integer, `data.inputs[0].status == "ok"`, and `data.operations[0].applied` SHALL reflect whether the file was modified

### Requirement: Convert Command JSON Payload

When the `convert` command runs with `--output json`, the envelope's `data` object SHALL contain at least:

- `conversions` (array of objects with `input` (string path), `output` (string path), `source_format` (string), `target_format` (string), `encoding` (string), `applied` (bool), `status` (`"ok"`/`"error"`), optional `error`).

#### Scenario: SRT to ASS conversion is reported
- **WHEN** the user runs `subx-cli --output json convert --input a.srt --output a.ass --format ass`
- **THEN** `data.conversions[0]` SHALL satisfy `source_format == "srt"`, `target_format == "ass"`, `applied == true`, and `status == "ok"` on success

### Requirement: Detect-Encoding Command JSON Payload

When the `detect-encoding` command runs with `--output json`, the envelope's `data` object SHALL contain at least:

- `files` (array of objects with `path` (string path), `encoding` (string), `confidence` (number in `[0.0, 1.0]`), `has_bom` (bool), `status` (`"ok"`/`"error"`), optional `error`).

#### Scenario: UTF-8 BOM file
- **GIVEN** a subtitle file encoded in UTF-8 with BOM
- **WHEN** the user runs `subx-cli --output json detect-encoding <path>`
- **THEN** `data.files[0]` SHALL satisfy `encoding == "UTF-8"` (case-insensitive match permitted), `has_bom == true`, and `status == "ok"`

### Requirement: Cache Command JSON Payload

When the `cache` command runs with `--output json`, the envelope's `data` object SHALL be shaped according to the active subcommand:

- `cache status` → `{ "total": integer, "pending": integer, "applied": integer }` plus any additional counts already reported by the text path.
- `cache clear` → `{ "removed": integer }`.
- `cache rollback` → `{ "rolled_back": integer }`.
- `cache apply` → `{ "applied": integer, "failed": integer, "items": [ { "id": string, "status": "ok" | "error", "error"?: { code, category, message } } ] }`.

A `cache list` subcommand is intentionally NOT covered by this change
(see the `cache-management` capability for the full deferral rationale).

The legacy `cache status --json` flag (defined on `StatusArgs` in
`src/cli/cache_args.rs`; the only existing JSON-style flag on any
cache subcommand) SHALL be preserved as a backward-compatible alias
that forwards to the global JSON output mode and emits byte-identical
output.

#### Scenario: cache clear reports removed count
- **WHEN** the user runs `subx-cli --output json cache clear`
- **THEN** `data.removed` SHALL be a non-negative integer equal to the number of cache entries removed

#### Scenario: cache apply reports per-item status
- **GIVEN** a cache containing N pending operations of which K fail to apply
- **WHEN** the user runs `subx-cli --output json cache apply`
- **THEN** `data.applied + data.failed == N`, `data.failed == K`, and `data.items` SHALL contain exactly N entries with each failed entry carrying `status == "error"` and an `error` object

### Requirement: Translate and Config Minimum Envelope

When `translate` or `config` runs with `--output json`, the CLI SHALL emit a valid envelope with `schema_version`, `command`, and `status`. On success the `data` object MAY be minimal (for `translate`: `{ "translated_files": [ { "input": string, "output": string, "applied": bool } ] }`; for `config`: `{ "config": <object> }` for `get`/`list` or `{ "key": string, "value": string }` for `set`). The error envelope SHALL be the same as for any other command.

#### Scenario: translate emits envelope on success
- **WHEN** the user runs `subx-cli --output json translate <args>` and translation succeeds
- **THEN** the envelope SHALL satisfy `status == "ok"` and `data.translated_files` SHALL be a (possibly empty) array

#### Scenario: config list emits envelope on success
- **WHEN** the user runs `subx-cli --output json config list`
- **THEN** `data.config` SHALL be a JSON object containing the resolved configuration values

### Requirement: Backward Compatibility

This capability SHALL NOT change behavior for users who do not pass `--output` and do not set `SUBX_OUTPUT`. All existing exit codes (mapped via `SubXError::exit_code`), default text output, default progress-bar visibility, status-symbol formatting, and the match result table SHALL remain byte-identical to their pre-change behavior.

#### Scenario: Text mode is unchanged
- **WHEN** any subcommand is invoked without selecting JSON mode
- **THEN** stdout, stderr, and the process exit code SHALL match the pre-change behavior exactly

#### Scenario: Exit codes are preserved across modes
- **GIVEN** any error producible by a subcommand
- **WHEN** that error occurs in either `text` or `json` mode
- **THEN** the process exit code SHALL be identical and SHALL equal `SubXError::exit_code` for the underlying error

### Requirement: Schema Version Stability

The CLI SHALL guarantee that any change emitting an envelope with the same `schema_version` major number is backward-compatible. Adding new optional fields to `data`, `error.details`, or top-level optional keys SHALL be a minor bump (e.g., `1.0` → `1.1`). Removing or renaming documented fields, changing types, or changing top-level required keys SHALL require a major bump (e.g., `1.x` → `2.0`) introduced via a separate OpenSpec change.

#### Scenario: Documented fields are stable within a major version
- **GIVEN** a script written against `schema_version "1.0"`
- **WHEN** the CLI later emits `schema_version "1.1"` or `"1.2"`
- **THEN** every key documented for `1.0` SHALL still be present with the same type and semantics
