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
