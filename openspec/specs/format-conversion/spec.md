# Format Conversion

## Purpose

Hold the CLI half of the format-conversion agreement: the supported `--format` value surface, input/output path resolution, the command-level original-file preservation and per-file error-isolation framing, and the structured JSON payload of `convert`. Implemented in `src/commands/convert_command.rs` and `src/cli/convert_args.rs`. The per-format conversion semantics, parser robustness, round-trip stability, and public format API are specified by the same-named `format-conversion` capability in `subx-core`.

## Requirements

### Requirement: Supported Output Formats

The system SHALL accept `--format` values `srt`, `ass`, `vtt`, and `sub`, defined by the `OutputSubtitleFormat` enum in `src/cli/convert_args.rs`, and SHALL write output files with the file extension matching the selected value. When `--format` is omitted the command SHALL resolve the target format from `formats.default_output` in configuration.

`OutputSubtitleFormat` is a clap-derived enum and stays in `subx-cli` permanently under SDR D8, so the accepted value set and the extension mapping are CLI-owned. What each target format's output must look like — that an SRT-to-VTT conversion produces a `WEBVTT` header and dot timecodes, and equivalently for the other targets — is specified by the `format-conversion` capability's *Target Format Conversion Semantics* requirement in `subx-core`.

#### Scenario: Default output format from configuration
- **GIVEN** the user omits `--format` and `formats.default_output` is `srt` in configuration
- **WHEN** the command runs
- **THEN** every input file SHALL be converted to SRT
### Requirement: Input and Output Path Resolution

The system SHALL accept input files or directories via a positional path and/or repeated `-i/--input` flags, filter inputs to subtitle extensions (`srt`, `ass`, `vtt`, `sub`, `ssa`), and compute the output path either from `--output` or by replacing the input file's extension with the target format extension.

#### Scenario: Automatic output naming
- **GIVEN** input `movie.srt` and `--format ass` with no `--output`
- **WHEN** the command runs
- **THEN** the output file SHALL be written to `movie.ass`

#### Scenario: Batch conversion into output directory
- **GIVEN** an input directory containing multiple subtitle files and `--output <dir>` pointing at a directory
- **WHEN** the command runs
- **THEN** each converted file SHALL be written inside the output directory using `<stem>.<format>` naming

### Requirement: Original File Preservation

The system SHALL by default remove the source file after a successful conversion and SHALL retain the source file when `--keep-original` is passed.

#### Scenario: Keep original
- **GIVEN** `--keep-original` is passed and conversion succeeds
- **WHEN** the command completes
- **THEN** both the original and the converted file SHALL exist

#### Scenario: Default removes original
- **GIVEN** `--keep-original` is not passed and conversion succeeds
- **WHEN** the command completes
- **THEN** the converted file SHALL exist and the original file SHALL be removed

### Requirement: Per-File Error Isolation

The system SHALL report conversion errors on a per-file basis and SHALL continue processing the remaining files rather than aborting the whole batch.

#### Scenario: One file fails in a batch
- **GIVEN** a batch of three input files where one is corrupt
- **WHEN** the command runs
- **THEN** the two valid files SHALL be converted successfully and the corrupt file SHALL produce an error message on stderr while the command exits without failing the whole batch

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
