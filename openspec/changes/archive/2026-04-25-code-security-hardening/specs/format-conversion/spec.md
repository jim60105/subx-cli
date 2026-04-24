# Format Conversion (Delta)

## ADDED Requirements

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
