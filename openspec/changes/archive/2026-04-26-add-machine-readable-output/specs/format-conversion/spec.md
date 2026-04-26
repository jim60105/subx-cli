## ADDED Requirements

### Requirement: Convert Command Emits Structured JSON Payload

When the `convert` command runs with the global output mode set to `json`, it SHALL emit a single JSON envelope on stdout (per the `machine-readable-output` capability) and SHALL NOT print free-form progress chatter or status symbols on stdout. The envelope's `data` object SHALL contain:

- `conversions` (array of objects with `input` (string path), `output` (string path), `source_format` (string, lowercase, e.g., `"srt"`, `"ass"`, `"vtt"`, `"sub"`), `target_format` (string, lowercase), `encoding` (string identifying the output encoding, e.g., `"UTF-8"`), `applied` (bool), `status` (`"ok"` or `"error"`), and an optional `error` object with `code`, `category`, `message` when `status == "error"`).

The convert command's existing per-file error isolation contract (already required by this capability — see "Per-File Error Isolation") SHALL be preserved in JSON mode by representing per-file failures as entries with `status == "error"` rather than as top-level error envelopes. The top-level envelope SHALL therefore satisfy `status == "ok"` whenever the batch loop completed and processed at least one file (regardless of how many entries individually failed). The process exit code SHALL remain `0` in this case, matching today's text-mode behavior.

A top-level error envelope (per the `machine-readable-output` capability's Error Envelope requirement) SHALL only be emitted for whole-command failures: configuration errors, missing or invalid inputs that prevent the batch loop from starting, fatal I/O before any file is processed, or a single-input invocation receiving a fatal error.

In `text` mode (the default) the convert command's existing UX is unchanged.

#### Scenario: SRT to ASS single-file conversion
- **WHEN** the user runs `subx-cli --output json convert --input a.srt --output a.ass --format ass`
- **THEN** `data.conversions` SHALL contain exactly one entry with `source_format == "srt"`, `target_format == "ass"`, `applied == true`, and `status == "ok"` on success

#### Scenario: Batch conversion reports each file
- **GIVEN** a directory containing multiple `.srt` files passed via `-i`
- **WHEN** the user runs `subx-cli --output json convert -i <dir> --format vtt`
- **THEN** `data.conversions` SHALL contain one entry per processed file with `applied == true` and `status == "ok"` for each successful conversion

#### Scenario: Per-file isolation of corrupt input in batch
- **GIVEN** a directory containing three `.srt` files where one is corrupt
- **WHEN** the user runs `subx-cli --output json convert -i <dir> --format vtt`
- **THEN** the top-level envelope SHALL satisfy `status == "ok"`, `data.conversions` SHALL contain three entries, two with `status == "ok"` and `applied == true`, one with `status == "error"`, `applied == false`, and an `error.category == "subtitle_format"`, AND the process SHALL exit with status `0`

#### Scenario: Single-input fatal error produces top-level error envelope
- **GIVEN** a single corrupt input file passed via `--input bad.srt`
- **WHEN** the user runs `subx-cli --output json convert --input bad.srt --format ass`
- **THEN** the envelope SHALL satisfy `status == "error"`, `error.category == "subtitle_format"`, `error.exit_code == 4`, and the process SHALL exit with status `4`
