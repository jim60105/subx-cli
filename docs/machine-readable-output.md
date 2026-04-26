# Machine-Readable Output

SubX-CLI ships a stable, versioned JSON output mode designed for shell
scripts, CI pipelines, and third-party tools. When you opt in, every
supported subcommand emits a single JSON envelope on stdout in place of
the human-oriented status symbols, progress bars, and result tables.
This document is the authoritative reference for that contract.

The default text mode is unchanged. JSON mode is strictly opt-in and
the existing exit-code contract (1–6, mapped through
`SubXError::exit_code`) is preserved across both modes.

## Table of Contents

- [Overview](#overview)
- [Activation](#activation)
- [Envelope Schema](#envelope-schema)
- [Schema-Version Policy](#schema-version-policy)
- [Error Envelope](#error-envelope)
- [Per-Command Payload Schemas](#per-command-payload-schemas)
- [CLI Parsing Flow](#cli-parsing-flow)
- [`generate-completion` Rejection](#generate-completion-rejection)
- [Stdout/Stderr Discipline](#stdoutstderr-discipline)
- [`cache status --json` Legacy Alias](#cache-status---json-legacy-alias)
- [Scripting Recipes](#scripting-recipes)
- [Stability Guarantees](#stability-guarantees)
- [References](#references)

## Overview

JSON mode replaces SubX-CLI's interactive UX with a single JSON document
per invocation:

- **Default (text mode)** — colored status symbols (`✓`/`✗`/`⚠`),
  `indicatif` progress bars, the AI match results table, and free-form
  prose. Output strings are not part of any contract and may evolve.
- **Opt-in (JSON mode)** — exactly one UTF-8 JSON document terminated
  by a single `\n` is written to stdout. The shape of that document is
  fixed by `schema_version` and is the contract scripts rely on.

Use JSON mode when you need to:

- Parse match candidates, sync offsets, conversion plans, encoding
  detections, or cache state programmatically.
- React to specific error categories from a script without scraping
  English error messages.
- Pipe SubX-CLI output through `jq` or another JSON tool without a
  sanitization step.

Stay in text mode for interactive use; the default UX is unaffected.

## Activation

JSON mode is selected by either:

1. The top-level `--output json` flag, **before** the subcommand token:

   ```bash
   subx-cli --output json match ./media
   subx-cli --output json convert --format srt ./subs/
   ```

2. The `SUBX_OUTPUT` environment variable:

   ```bash
   export SUBX_OUTPUT=json
   subx-cli match ./media
   ```

`--output` and `SUBX_OUTPUT` accept `text` or `json` (case-insensitive).

### Precedence

An explicit top-level `--output` argument always wins over
`SUBX_OUTPUT`. So `SUBX_OUTPUT=json subx-cli --output text match …`
runs in text mode.

### Placement constraint

The top-level `--output` flag is **only accepted before the subcommand
token**. This is intentional, because `convert`, `sync`, and
`translate` already define their own subcommand-local
`--output <PATH>` argument that designates an **output file**. The
positional rule keeps both meanings unambiguous:

```bash
# UNAMBIGUOUS:
#   first --output  -> output MODE   (parsed by the root Cli)
#   second --output -> output FILE   (parsed by ConvertArgs)
subx-cli --output json convert --output a.ass --format ass ./a.srt

# DOES NOT switch to JSON mode:
#   --output here is ConvertArgs.output and receives the literal "json"
subx-cli convert --output json --format ass ./a.srt
```

The same constraint applies to `--quiet`. Both flags must precede the
subcommand; placing them after the subcommand causes clap to reject the
invocation as an unknown argument (no subcommand currently defines its
own `--quiet`).

### `--quiet`

`--quiet` is orthogonal to the output mode:

- In text mode, `--quiet` suppresses `print_success`/`print_warning`
  helpers, progress bars, and the match result table.
- In JSON mode, stdout is **implicitly quiet** by construction (only the
  envelope is ever written). `--quiet` additionally silences the
  free-form stderr chatter (`tracing` info logs, AI provider
  diagnostics) that JSON mode would otherwise allow on stderr.

`--quiet` never suppresses the JSON envelope on stdout and never
changes the process exit code.

## Envelope Schema

Every JSON-mode invocation writes exactly one UTF-8 JSON object on
stdout, terminated by a single `\n`, with the following top-level keys:

| Key              | Type        | When present | Description |
|------------------|-------------|--------------|-------------|
| `schema_version` | string      | Always       | Semver-style version, currently `"1.0"`. |
| `command`        | string      | Always       | Subcommand name: `"match"`, `"sync"`, `"convert"`, `"detect-encoding"`, `"translate"`, `"cache"`, `"config"`, or `"generate-completion"`. May be empty for synthetic argument-parsing envelopes when the subcommand could not be identified. |
| `status`         | string      | Always       | `"ok"` or `"error"`. |
| `data`           | object      | `status == "ok"` | Command-specific payload. **Omitted entirely** (never `null`) when `status == "error"`. |
| `error`          | object      | `status == "error"` | Defined by the [Error Envelope](#error-envelope) section. **Omitted entirely** on success. |
| `warnings`       | array\|null | Optional     | Reserved for non-fatal warnings. Currently always omitted; consumers SHALL tolerate its presence as an array of `{code, message}` objects. |

### Success example

```json
{
  "schema_version": "1.0",
  "command": "convert",
  "status": "ok",
  "data": {
    "conversions": [
      {
        "input": "/media/movie.srt",
        "output": "/media/movie.ass",
        "source_format": "srt",
        "target_format": "ass",
        "encoding": "UTF-8",
        "applied": true,
        "entry_count": 412,
        "status": "ok"
      }
    ]
  }
}
```

### Error example

```json
{
  "schema_version": "1.0",
  "command": "convert",
  "status": "error",
  "error": {
    "category": "subtitle_format",
    "code": "E_SUBTITLE_FORMAT",
    "exit_code": 4,
    "message": "Subtitle format error (SRT): unexpected end of file"
  }
}
```

## Schema-Version Policy

`schema_version` follows semantic versioning.

- **Patch / minor bumps** (`1.0` → `1.1`, `1.1` → `1.2`, …) are
  backward-compatible. New optional fields MAY be added to `data`,
  `error.details`, or as new top-level optional keys (e.g. a future
  top-level `errors[]` array mirroring per-item failures, which is
  reserved by `design.md` but not emitted in `1.0`). Every key
  documented for an earlier minor version SHALL keep the same type and
  semantics.
- **Major bumps** (`1.x` → `2.0`) are reserved for renaming or removing
  documented keys, changing types, or restructuring the envelope. Any
  major bump goes through a dedicated OpenSpec change proposal.

Scripts SHOULD test the major version (`startswith("1.")`) and accept
any minor / patch number.

## Error Envelope

When `status == "error"`, the envelope contains an `error` object with
these keys:

| Key         | Type    | When present | Description |
|-------------|---------|--------------|-------------|
| `category`  | string  | Always       | Stable snake_case identifier (see table below). |
| `code`      | string  | Always       | Stable upper-snake-case machine code, prefixed `E_` (e.g. `E_AI_SERVICE`). |
| `exit_code` | integer | Always       | The numeric process exit code, equal to `SubXError::exit_code` for that variant (or to the underlying handler's exit code for synthetic envelopes). The process exit status equals this value. |
| `message`   | string  | Always       | Human-readable English text from `SubXError::user_friendly_message` (or the rendered clap error with ANSI removed for synthetic envelopes). |
| `hint`      | string  | Optional     | Short remediation hint surfaced from `SubXError::hint`. |
| `details`   | object  | Optional     | Free-form structured context (e.g. `path`, `format`, `partial_results`). |

The numeric process exit code SHALL equal `error.exit_code`.

### Category and machine-code table

The category set is closed for envelopes derived from `SubXError`. The
synthetic `argument_parsing` category is added by the CLI parsing flow
(see [CLI Parsing Flow](#cli-parsing-flow)) and has no `SubXError`
backing variant.

| `SubXError` variant            | `category`                   | `code`                          | `exit_code` |
|--------------------------------|------------------------------|---------------------------------|-------------|
| `Io`                           | `io`                         | `E_IO`                          | 1 |
| `Config`                       | `config`                     | `E_CONFIG`                      | 2 |
| `SubtitleFormat`               | `subtitle_format`            | `E_SUBTITLE_FORMAT`             | 4 |
| `AiService`                    | `ai_service`                 | `E_AI_SERVICE`                  | 3 |
| `Api`                          | `api`                        | `E_API`                         | 3 |
| `AudioProcessing`              | `audio_processing`           | `E_AUDIO_PROCESSING`            | 5 |
| `FileMatching`                 | `file_matching`              | `E_FILE_MATCHING`               | 6 |
| `FileAlreadyExists`            | `file_already_exists`        | `E_FILE_ALREADY_EXISTS`         | 1 |
| `FileNotFound`                 | `file_not_found`             | `E_FILE_NOT_FOUND`              | 1 |
| `InvalidFileName`              | `invalid_file_name`          | `E_INVALID_FILE_NAME`           | 1 |
| `FileOperationFailed`          | `file_operation_failed`      | `E_FILE_OPERATION_FAILED`       | 1 |
| `CommandExecution`             | `command_execution`          | `E_COMMAND_EXECUTION`           | 1 |
| `NoInputSpecified`             | `no_input_specified`         | `E_NO_INPUT_SPECIFIED`          | 1 |
| `InvalidPath`                  | `invalid_path`               | `E_INVALID_PATH`                | 1 |
| `PathNotFound`                 | `path_not_found`             | `E_PATH_NOT_FOUND`              | 1 |
| `DirectoryReadError`           | `directory_read_error`       | `E_DIRECTORY_READ_ERROR`        | 1 |
| `InvalidSyncConfiguration`     | `invalid_sync_configuration` | `E_INVALID_SYNC_CONFIGURATION`  | 1 |
| `UnsupportedFileType`          | `unsupported_file_type`      | `E_UNSUPPORTED_FILE_TYPE`       | 1 |
| `OutputModeUnsupported`        | `command_execution`          | `E_OUTPUT_MODE_UNSUPPORTED`     | 1 |
| `Other`                        | `other`                      | `E_OTHER`                       | 1 |
| *(synthetic — clap parse failure)* | `argument_parsing`       | `E_ARGUMENT_PARSING`            | clap exit code (typically `2`) |

`OutputModeUnsupported` deliberately reuses the `command_execution`
category because that is its `SubXError::exit_code` group; the more
specific signal is in `code` (`E_OUTPUT_MODE_UNSUPPORTED`). See
[`generate-completion` Rejection](#generate-completion-rejection).

### Partial results

For commands that may fail mid-run after applying some operations, the
top-level `error.details.partial_results` object MAY carry the set of
already-applied changes so a script can reconcile state without
re-scanning the filesystem. This is an optional enrichment; scripts
MUST tolerate its absence.

## Per-Command Payload Schemas

The following sections document each covered command's `data` shape.
Field names are committed contract; types are JSON types.

### Batch-vs-fatal failure rule

Several commands process multiple files in a single invocation
(`match`, `sync`, `convert`, `detect-encoding`, `cache apply`,
`translate`). They follow a uniform per-item / top-level rule:

- **`status` is `"ok"` at the top level whenever the command made
  forward progress on at least one item.** Per-item failures appear in
  the success payload with their own `status: "error"` plus an
  `error { code, category, message }` object (the per-item error has
  no `exit_code`).
- **`status` is `"error"` at the top level only when the entire command
  failed before any per-item progress** — config invalid, no inputs
  specified, fatal I/O, or a single-input invocation whose only input
  failed. In that case `data` is omitted and the top-level `error`
  carries the failure.

The process exit code follows the top-level decision: `0` when
top-level `status == "ok"` (even if some per-item entries failed), and
`error.exit_code` otherwise.

### `match`

```json
{
  "schema_version": "1.0",
  "command": "match",
  "status": "ok",
  "data": {
    "dry_run": false,
    "confidence_threshold": 80,
    "candidates": [
      {
        "video": "/media/Movie.mkv",
        "subtitle": "/media/sub.srt",
        "confidence": 92,
        "accepted": true
      },
      {
        "video": "/media/Other.mkv",
        "subtitle": "/media/maybe.srt",
        "confidence": 41,
        "accepted": false,
        "reason": "below_threshold"
      }
    ],
    "operations": [
      {
        "kind": "rename",
        "source": "/media/sub.srt",
        "target": "/media/Movie.srt",
        "applied": true,
        "status": "ok"
      }
    ],
    "summary": {
      "total_candidates": 2,
      "accepted": 1,
      "applied": 1,
      "skipped": 1,
      "failed": 0
    }
  }
}
```

`data.candidates[].reason` is one of `"below_threshold"` or
`"id_not_found"` and is only present when `accepted == false`.
`data.operations[].kind` is one of `"rename"`, `"copy"`, or `"move"`.
Per-operation `status` is `"ok"` or `"error"`; the latter carries an
`error { code, category, message }`. `summary.failed` counts operations
whose per-item `status == "error"`.

### `sync`

`sync` always emits a uniform payload shape exposing a top-level
`method`, an `inputs` array describing the per-subtitle analysis stage,
and an `operations` array describing the per-subtitle write stage.
Single-pair invocations produce 1-element arrays; batch invocations
produce N parallel entries (one input ↔ one operation per processed
subtitle).

**Single-pair invocation** — VAD-driven sync against a single
subtitle/video pair:

```json
{
  "schema_version": "1.0",
  "command": "sync",
  "status": "ok",
  "data": {
    "method": "vad",
    "inputs": [
      {
        "subtitle_path": "/media/movie.srt",
        "audio_path": "/media/movie.mkv",
        "detected_offset_ms": -1280,
        "confidence": 0.91,
        "vad": {
          "sensitivity": 0.5,
          "padding_ms": 300,
          "segments": [{"start": 1.2, "end": 4.8, "duration": 3.6}]
        },
        "status": "ok"
      }
    ],
    "operations": [
      {
        "subtitle_path": "/media/movie.srt",
        "output_path": "/media/movie.srt",
        "applied": true,
        "dry_run": false,
        "status": "ok"
      }
    ]
  }
}
```

`method` is `"vad"`, `"manual"`, or `"auto"` and reflects the active
dispatch decision. Inside an input entry, `confidence` and `vad` are
absent for manual offsets; `audio_path` is absent when sync ran without
a video. For manual sync, `detected_offset_ms` equals the user-supplied
offset converted to milliseconds. Inside an operation entry,
`output_path` is absent on dry runs that produced no concrete target.

**Batch invocation** — same shape, one entry per processed subtitle.
Per-file failures surface as entries with `status == "error"` plus an
`error` object (with `code`, `category`, `message`); the top-level
envelope stays `status == "ok"` as long as **at least one** input
succeeded or **at least one** operation was applied:

```json
{
  "data": {
    "method": "manual",
    "inputs": [
      {
        "subtitle_path": "/media/a.srt",
        "audio_path": "/media/a.mp4",
        "detected_offset_ms": 500,
        "status": "ok"
      },
      {
        "subtitle_path": "/media/b.srt",
        "audio_path": "/media/b.mp4",
        "detected_offset_ms": 0,
        "status": "error",
        "error": {
          "code": "E_SUBTITLE_FORMAT",
          "category": "subtitle_format",
          "message": "..."
        }
      }
    ],
    "operations": [
      { "subtitle_path": "/media/a.srt", "output_path": "/media/a.srt",
        "applied": true,  "dry_run": false, "status": "ok" },
      { "subtitle_path": "/media/b.srt",
        "applied": false, "dry_run": false, "status": "error",
        "error": {
          "code": "E_SUBTITLE_FORMAT",
          "category": "subtitle_format",
          "message": "..."
        }
      }
    ]
  }
}
```

A batch with **zero** successful items (no `inputs[i].status == "ok"`
and no `operations[i].applied == true`) does NOT emit a success
envelope wrapping all-error items. Instead, the command exits with a
top-level error envelope (typically `error.category == "file_matching"`
when no subtitle/video pairing succeeded). Whole-command failures such
as `InvalidSyncConfiguration` likewise produce a top-level error
envelope.

### `convert`

```json
{
  "schema_version": "1.0",
  "command": "convert",
  "status": "ok",
  "data": {
    "conversions": [
      {
        "input": "/media/a.srt",
        "output": "/media/a.ass",
        "source_format": "srt",
        "target_format": "ass",
        "encoding": "UTF-8",
        "applied": true,
        "entry_count": 412,
        "status": "ok"
      },
      {
        "input": "/media/corrupt.srt",
        "output": "/media/corrupt.ass",
        "target_format": "ass",
        "encoding": "UTF-8",
        "applied": false,
        "status": "error",
        "error": {
          "code": "E_SUBTITLE_FORMAT",
          "category": "subtitle_format",
          "message": "Subtitle format error (SRT): unexpected end of file"
        }
      }
    ]
  }
}
```

`source_format` and `entry_count` are omitted on entries that failed
before parsing succeeded. A batch with at least one successful
conversion keeps top-level `status == "ok"` and exit code `0`; a
single-input invocation that fails produces the top-level error
envelope with `error.category == "subtitle_format"` and
`error.exit_code == 4`.

### `detect-encoding`

```json
{
  "schema_version": "1.0",
  "command": "detect-encoding",
  "status": "ok",
  "data": {
    "files": [
      {
        "path": "/media/a.srt",
        "status": "ok",
        "encoding": "UTF-8",
        "confidence": 1.0,
        "has_bom": true,
        "bytes_sampled": 8192
      },
      {
        "path": "/media/missing.srt",
        "status": "error",
        "error": {
          "code": "E_FILE_NOT_FOUND",
          "category": "file_not_found",
          "message": "File not found: /media/missing.srt"
        }
      }
    ]
  }
}
```

`encoding`, `confidence`, `has_bom`, and `bytes_sampled` are omitted on
failed entries. A single-input invocation against a missing file
produces the top-level error envelope (`E_FILE_NOT_FOUND` /
`E_PATH_NOT_FOUND`).

### `cache`

`cache`'s `data` shape varies by subcommand. The top-level `command`
field is always `"cache"`.

#### `cache status`

```json
{
  "command": "cache",
  "status": "ok",
  "data": {
    "path": "/home/user/.local/share/subx/cache.json",
    "exists": true,
    "journal_present": true,
    "total": 12,
    "pending": 3,
    "applied": 9,
    "size_bytes": 4096,
    "created_at": 1700000000,
    "age_seconds": 3600,
    "cache_version": "1",
    "ai_model": "gpt-4.1-mini",
    "operation_count": 12,
    "config_hash": "abcdef…",
    "current_config_hash": "abcdef…",
    "config_hash_match": true,
    "snapshot_status": "valid"
  }
}
```

The required fields per the `cache-management` spec are `total`,
`pending`, and `applied` (non-negative integers). Every other field is
an additive enrichment and MAY be absent.

#### `cache clear`

```json
{ "command": "cache", "status": "ok",
  "data": { "removed": 2, "kind": "all",
            "cache_path": "…", "cache_removed": true,
            "journal_path": "…", "journal_removed": true } }
```

`removed` is `0` when the cache was already empty.

#### `cache rollback`

```json
{ "command": "cache", "status": "ok",
  "data": { "rolled_back": 5 } }
```

#### `cache apply`

```json
{
  "command": "cache",
  "status": "ok",
  "data": {
    "applied": 4,
    "failed": 1,
    "items": [
      { "id": "/path/to/sub-a.srt", "status": "ok" },
      { "id": "/path/to/sub-b.srt", "status": "error",
        "error": {
          "code": "E_FILE_NOT_FOUND",
          "category": "file_not_found",
          "message": "File not found: /path/to/sub-b.srt"
        } }
    ]
  }
}
```

`applied + failed == items.len()`. Per-item failures keep the top-level
`status == "ok"`; only fatal whole-command errors (e.g., missing
journal) produce a top-level error envelope.

#### `cache list`

`cache list` is reserved by the spec for a future iteration; the
current implementation does not expose a `list` subcommand. When
added, it SHALL emit
`{ "entries": [{ "id", "kind", "created_at", "summary" }] }`.

### `translate`

```json
{
  "schema_version": "1.0",
  "command": "translate",
  "status": "ok",
  "data": {
    "translated_files": [
      { "input": "/media/a.srt", "output": "/media/a.zh-TW.srt",
        "applied": true },
      { "input": "/media/b.srt", "output": "/media/b.zh-TW.srt",
        "applied": false }
    ]
  }
}
```

The `translate` payload is intentionally minimal in `1.0`. Per-item
errors are surfaced through `applied: false` plus a top-level
`CommandExecution` error envelope when one or more files failed; a
fully successful batch always returns `status == "ok"`.

### `config`

```json
{ "command": "config", "status": "ok",
  "data": { "config": { /* resolved configuration object */ } } }
```

- `config get` and `config list` emit `data.config` (with sensitive
  values like `ai.api_key` masked).
- `config set` emits `data: { "key": "<key>", "value": "<masked-value>" }`.
- `config reset` emits the same `data.config` shape as `list` after
  resetting to defaults.

## CLI Parsing Flow

The process boundary in `src/main.rs` guarantees that every JSON-mode
invocation produces exactly one JSON document on stdout, including the
case where clap rejects argv before any subcommand runs:

1. **Early argv/env sniff.** Before invoking clap, `main.rs` scans
   `env::args_os()` for `--output <value>` / `--output=<value>` and
   reads `SUBX_OUTPUT` to compute a tentative `OutputMode`. The sniff
   is permissive and defaults to `text` when ambiguous.
2. **`Cli::try_parse()`**, never `parse()`. On `Err(clap::Error)`:
   - In tentative `text` mode, the clap error is rendered as today
     (preserving help/usage formatting on stderr) and the process
     exits with `clap::Error::exit_code()`.
   - In tentative `json` mode, a synthetic JSON error envelope is
     written to stdout with `error.category == "argument_parsing"`,
     `error.code == "E_ARGUMENT_PARSING"`, `error.exit_code` equal to
     the clap exit code, and `error.message` equal to the rendered
     clap error with ANSI styling stripped. The process exits with the
     clap exit code.
3. **`--help` and `--version` are exempt.** clap surfaces them through
   `Err(clap::Error)` with `ErrorKind::DisplayHelp`,
   `ErrorKind::DisplayVersion`, or
   `ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand` and exit code
   `0`. Even in JSON mode, those three kinds short-circuit to clap's
   own text rendering, exit `0`, and do **not** emit a JSON envelope.
4. **Successful parse.** `cli::run_with_config` returns a structured
   `RunOutcome { output_mode, command, result }`; `main.rs` uses it to
   render the final envelope without re-parsing argv.

### Synthetic argument-parsing envelope example

```bash
$ SUBX_OUTPUT=json subx-cli --bogus-flag match ./media
```

```json
{
  "schema_version": "1.0",
  "command": "",
  "status": "error",
  "error": {
    "category": "argument_parsing",
    "code": "E_ARGUMENT_PARSING",
    "exit_code": 2,
    "message": "error: unexpected argument '--bogus-flag' found\n\nUsage: subx-cli [OPTIONS] <COMMAND>\n\nFor more information, try '--help'."
  }
}
```

## `generate-completion` Rejection

`generate-completion`'s stdout is, by design, a shell-completion
script and is incompatible with the JSON envelope contract. When
invoked under JSON mode (via `--output json` or `SUBX_OUTPUT=json`),
the command refuses to run and emits a top-level error envelope:

```bash
$ subx-cli --output json generate-completion bash
```

```json
{
  "schema_version": "1.0",
  "command": "generate-completion",
  "status": "error",
  "error": {
    "category": "command_execution",
    "code": "E_OUTPUT_MODE_UNSUPPORTED",
    "exit_code": 1,
    "message": "JSON output mode is not supported for 'generate-completion'. Use --output text or omit --output to write the shell-completion script."
  }
}
```

No shell-completion bytes are written to stdout in this case. The exit
code is `SubXError::CommandExecution(_).exit_code()` (currently `1`);
the contract pins the helper, not the literal number. In text mode the
existing behavior is unchanged: the completion script is written to
stdout and the process exits `0`.

## Stdout/Stderr Discipline

When the active output mode is `json`:

- **Stdout** contains exactly one JSON document followed by a single
  trailing `\n` and nothing else. No ANSI escape sequences, no `✓` /
  `✗` / `⚠` status symbols, no `indicatif` progress-bar frames, no
  match table, no free-form prose.
- **Stderr** MAY contain free-form diagnostic logs (`tracing` info,
  AI provider diagnostics) but SHALL NOT contain a JSON envelope,
  status symbols, ANSI styling emitted by `print_success` /
  `print_warning` / `print_error`, or `indicatif` progress-bar frames.
  ANSI styling on stderr is stripped in JSON mode.
- **Progress bars** are force-hidden via
  `ProgressBar::set_draw_target(ProgressDrawTarget::hidden())`,
  regardless of `general.enable_progress_bar`.
- **`--quiet`** additionally suppresses stderr chatter (tracing,
  diagnostics) while leaving the stdout envelope intact.

In text mode (default), all of the above behave exactly as in prior
releases.

## `cache status --json` Legacy Alias

The legacy `cache status --json` flag (defined on `StatusArgs` in
`src/cli/cache_args.rs`) predates the global output mode. It is
preserved as a thin, backward-compatible alias that forwards to the
global JSON renderer:

```bash
# These two invocations emit byte-identical JSON envelopes on stdout:
subx-cli --output json cache status
subx-cli cache status --json
```

The alias is limited to `cache status`; no other cache subcommand
exposes a `--json` flag, and none will be added. New scripts should
prefer the global `--output json` form for uniformity across
subcommands.

## Scripting Recipes

The recipes below assume `jq` is on `$PATH`.

### Extract a match command's first candidate confidence

```bash
subx-cli --output json match --dry-run ./media \
  | jq -r '.data.candidates[0].confidence'
```

`jq -r` (raw output) prints the integer with no surrounding quotes.

### Iterate `convert` items and report failures

```bash
subx-cli --output json convert --format vtt ./subs/ \
  | jq -r '.data.conversions[]
           | select(.status == "error")
           | "FAILED: \(.input) — \(.error.code) \(.error.message)"'
```

The top-level envelope stays `status == "ok"` on partial success, so
the script keeps running; per-item failures are surfaced via
`select(.status == "error")`.

### Detect a single error envelope and exit nonzero in CI

```bash
#!/usr/bin/env bash
set -euo pipefail

out=$(subx-cli --output json sync ./video.mp4 ./sub.srt) || rc=$?
rc=${rc:-0}

status=$(printf '%s' "$out" | jq -r '.status')

if [[ "$status" == "error" ]]; then
    code=$(printf '%s' "$out" | jq -r '.error.code')
    message=$(printf '%s' "$out" | jq -r '.error.message')
    printf 'subx-cli failed: %s — %s\n' "$code" "$message" >&2
    exit "$rc"
fi
```

Note that `subx-cli` always exits with the documented exit code
(1–6 from `SubXError::exit_code`), so `set -e` plus inspecting `$?`
remains a valid simpler approach when you only care about
success/failure; parsing `error.code` is for scripts that want to
react to specific categories.

### Bonus: count cached operations awaiting apply

```bash
subx-cli --output json cache status \
  | jq '.data.pending'
```

## Stability Guarantees

Within a major schema version, SubX-CLI guarantees:

- Every documented top-level key is present with the same JSON type
  and the same semantics.
- Every documented field inside a covered command's `data` payload is
  present (or marked optional in this document) with the same JSON
  type and semantics.
- Every error `category` and `code` listed in
  [Category and machine-code table](#category-and-machine-code-table)
  is stable. New categories may be added in minor bumps.
- `error.exit_code` continues to equal `SubXError::exit_code` for the
  underlying variant. The numeric exit-code contract (1–6) is
  preserved across patch releases.
- `OutputModeUnsupported` continues to map to category
  `command_execution` and code `E_OUTPUT_MODE_UNSUPPORTED`.

What MAY change in a minor bump (`1.x`) without breaking consumers:

- New optional fields in `data` payloads.
- New optional fields in `error.details`.
- New optional top-level keys (e.g., a future top-level `errors[]`
  array mirroring per-item failures).
- New error categories or codes for previously-unmapped error paths.
- Richer payload schemas for `translate` and `config` (currently
  minimal in `1.0`).

What requires a major bump:

- Renaming or removing any documented field.
- Changing the JSON type of any documented field.
- Restructuring the envelope (e.g., dropping `data` or `error`).

## References

- OpenSpec capability: `openspec/changes/add-machine-readable-output/specs/machine-readable-output/spec.md`
- Error-handling additions: `openspec/changes/add-machine-readable-output/specs/error-handling/spec.md`
- Per-command additions: the `cache-management`, `encoding-detection`,
  `format-conversion`, `progress-reporting`, `subtitle-matching`, and
  `timeline-sync` spec files under the same change directory.
- Source of truth for the envelope and error mapping:
  - `src/cli/output.rs` (envelope, renderer, schema-version constant)
  - `src/error.rs` (`SubXError::category`, `machine_code`, `exit_code`,
    `user_friendly_message`, `hint`)
- Command reference: [`command-reference.md`](command-reference.md)
- Configuration reference: [`configuration-guide.md`](configuration-guide.md)
