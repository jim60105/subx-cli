# Format Conversion

## Purpose

Convert subtitle files between the four supported output formats (SRT, ASS, VTT, SUB) while preserving timing, text, and — where possible — styling, supporting both single files and batch directories. Implemented in `src/commands/convert_command.rs`, `src/cli/convert_args.rs`, and `src/core/formats/`.

## Requirements

### Requirement: Supported Output Formats

The system SHALL accept `--format` values `srt`, `ass`, `vtt`, and `sub`, defined by the `OutputSubtitleFormat` enum, and SHALL produce output files with the matching file extension.

#### Scenario: Convert SRT to VTT
- **GIVEN** an input SRT file and `--format vtt`
- **WHEN** the convert command runs
- **THEN** the output file SHALL contain a `WEBVTT` header and SRT-style comma timecodes SHALL be converted to dot timecodes (for example `00:00:01.000 --> 00:00:02.000`)

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

### Requirement: File size check before parsing

Before reading a subtitle file for format conversion, the system SHALL check the file size against the configured `general.max_subtitle_bytes` limit. If the file exceeds the limit, the system SHALL return an error without reading the file, preventing unbounded memory allocation from malicious or malformed oversized inputs.

#### Scenario: oversized file rejected before conversion
- **WHEN** a 100 MiB subtitle file is submitted for conversion and the limit is 50 MiB
- **THEN** the system SHALL return an error without reading the file

#### Scenario: normal file converted
- **WHEN** a 500 KiB subtitle file is submitted
- **THEN** conversion SHALL proceed normally

### Requirement: Parser robustness on malformed input

All subtitle format parsers SHALL return `SubXError` values instead of panicking when encountering malformed input. No parser SHALL use `.unwrap()` on data derived from file content.

#### Scenario: malformed ASS file returns error
- **WHEN** an ASS file has an invalid Format line
- **THEN** the parser SHALL return `SubXError::SubtitleFormat` instead of panicking

#### Scenario: malformed SRT block is skipped
- **WHEN** an SRT file has one malformed block among many
- **THEN** the parser SHALL skip the bad block and parse the rest

### Requirement: Parser/serializer round-trip stability

The system SHALL guarantee that, for every canonical input fixture stored
under `tests/fixtures/formats/<format>/`, parsing the input via the
registered `SubtitleFormat` impl and re-serializing the resulting
`Subtitle` produces output that is byte-identical to the corresponding
checked-in `<fixture>.expected` file. The `<fixture>.expected` files are
generated from the *current* serializer's canonical output (not from the
original input) and serve to lock serializer behavior across this
refactor; they do NOT assert that the serializer reproduces the original
input bytes. Refactoring the format module internals MUST NOT change the
bytes a serializer emits for any fixture covering SRT, ASS, VTT, or SUB.

#### Scenario: SRT round-trip is byte-stable

- **GIVEN** a canonical SRT fixture in `tests/fixtures/formats/srt/`
- **WHEN** the round-trip integration test parses the fixture and
  re-serializes the resulting `Subtitle`
- **THEN** the re-serialized output SHALL be byte-identical to the
  matching `.expected` file (which captures the current serializer's
  canonical output, not the original input)

#### Scenario: ASS round-trip is byte-stable

- **GIVEN** a canonical ASS fixture in `tests/fixtures/formats/ass/`
  including at least one styled cue
- **WHEN** the round-trip integration test parses and re-serializes the
  fixture
- **THEN** the output SHALL be byte-identical to the corresponding
  `.expected` file. The `.expected` file reflects the serializer's
  canonical emission of `[Script Info]`, `[V4+ Styles]`, and `[Events]`
  sections (whose order, whitespace, and field formatting are decided by
  the serializer, not copied from the original input). The refactor
  MUST NOT change those emitted bytes.

#### Scenario: VTT round-trip is byte-stable

- **GIVEN** a canonical VTT fixture in `tests/fixtures/formats/vtt/`
- **WHEN** the round-trip integration test parses and re-serializes the
  fixture
- **THEN** the output SHALL be byte-identical to the corresponding
  `.expected` file (which includes the `WEBVTT` header as emitted by the
  current serializer)

#### Scenario: SUB round-trip is byte-stable

- **GIVEN** a canonical SUB (MicroDVD) fixture in
  `tests/fixtures/formats/sub/`
- **WHEN** the round-trip integration test parses and re-serializes the
  fixture
- **THEN** the output SHALL be byte-identical to the corresponding
  `.expected` file

#### Scenario: CRLF inputs are tolerated and locked by fixtures

- **GIVEN** a CRLF-line-ending fixture (suffix `.crlf.<ext>`) for each
  format under `tests/fixtures/formats/<format>/`
- **WHEN** the round-trip integration test parses the CRLF fixture and
  re-serializes the resulting `Subtitle`
- **THEN** parsing SHALL NOT panic, the parse call SHALL return `Ok(_)`
  for every CRLF fixture, and the re-serialized output SHALL be
  byte-identical to the matching `.expected` file. The CRLF fixtures
  lock the *current* serializer output bytes for behavior preservation;
  they do NOT assert semantic equivalence to the LF-equivalent fixtures
  and they do NOT promise the same in-memory entry count
- **AND** the following pre-existing CRLF parser quirks are explicitly
  acknowledged and frozen by the fixtures rather than fixed in this
  refactor:
  - SRT: `content.split("\n\n")` does not split blocks separated by
    `"\r\n\r\n"`, so a CRLF SRT file is treated as a single block whose
    text payload embeds the remaining cues. The serializer happens to
    re-emit byte-stable output because the embedded payload, when
    re-parsed by the same algorithm on LF output, reconstructs cues.
    The in-memory `Subtitle` from a CRLF SRT input contains fewer
    entries than its LF counterpart
  - VTT: the same block splitter combined with the trailing `\r` on the
    cue marker line causes CRLF VTT files to parse to zero cue entries;
    the `WEBVTT` header is still recognized so the parse succeeds with
    an empty `entries` vector
  - ASS and SUB: parsing is line-based and is unaffected by CRLF; their
    CRLF and LF fixtures parse to identical `Subtitle` values
- **AND** addressing the SRT and VTT CRLF semantics is deferred to a
  follow-up change; this refactor must not alter those parsers' output

### Requirement: Public format API stability across module reorganization

The system SHALL preserve every existing public path under
`crate::core::formats` (including `Subtitle`, `SubtitleEntry`,
`SubtitleMetadata`, `StylingInfo`, `SubtitleFormatType`,
`SubtitleFormat`, `SrtFormat`, `AssFormat`, `VttFormat`, `SubFormat`,
`FormatManager`, and `FormatConverter`) while internal modules are
reorganized. The full method signatures of the `SubtitleFormat` trait
(`parse`, `serialize`, `detect`, `format_name`, `file_extensions`,
`supports_styling`, `uses_frame_timing`) SHALL remain unchanged in
arity, parameter types, return types, and default-method semantics.
Downstream crates and other modules in `subx-cli` MUST continue to
compile without import path changes.

#### Scenario: Existing import paths still resolve

- **GIVEN** any pre-existing `use crate::core::formats::<Item>;`
  statement in the codebase or in published rustdoc examples
- **WHEN** the format module is reorganized into per-format submodules
- **THEN** the import SHALL still resolve and `cargo build`,
  `cargo clippy -- -D warnings`, and `cargo test --doc --all-features`
  SHALL pass

#### Scenario: FormatManager registration remains complete

- **GIVEN** a default `FormatManager::new()` instance after the refactor
- **WHEN** `detect_format` is called on a path with extension `srt`,
  `ass`, `ssa`, `vtt`, or `sub`
- **THEN** the manager SHALL return the corresponding
  `SubtitleFormatType` exactly as before the refactor

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
