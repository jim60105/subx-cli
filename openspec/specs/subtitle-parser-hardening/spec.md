# Subtitle Parser Hardening

## Purpose

Ensure every subtitle format parser (ASS, SRT, SUB, VTT) fails safely on
malformed input — returning typed `SubXError` values rather than
panicking, wrapping silently, or aborting the process — so that one bad
block or hostile timestamp cannot crash SubX.

## Requirements

### Requirement: ASS parser error recovery on missing format fields

The ASS parser SHALL return a `SubXError` instead of panicking when the `Format:` line is missing `Start`, `End`, or `Text` fields. The error message SHALL identify which field is missing.

#### Scenario: missing Start field

- **WHEN** an ASS file has a Format line without "Start"
- **THEN** the parser returns `SubXError::SubtitleFormat` with a message mentioning "Start"

#### Scenario: missing Text field

- **WHEN** an ASS file has a Format line without "Text"
- **THEN** the parser returns `SubXError::SubtitleFormat` with a message mentioning "Text"

#### Scenario: valid ASS file parses successfully

- **WHEN** an ASS file has all required Format fields
- **THEN** parsing succeeds as before

### Requirement: ASS timestamp overflow protection

The `parse_ass_time` function SHALL use checked arithmetic (`checked_mul`, `checked_add`) for timestamp computation. If overflow occurs, the function SHALL return a `SubXError` instead of wrapping silently or aborting.

#### Scenario: normal timestamp parses correctly

- **WHEN** the timestamp is `1:23:45.67`
- **THEN** the result is `Duration::from_millis(5025670)`

#### Scenario: overflowing timestamp returns error

- **WHEN** the timestamp contains `9999999999:00:00.00`
- **THEN** the function returns a `SubXError` instead of panicking or producing a wrapped value

### Requirement: SRT parser continues on malformed blocks

The SRT parser SHALL skip malformed subtitle blocks and continue parsing subsequent blocks, instead of aborting the entire parse on the first bad block.

#### Scenario: one bad block among many

- **WHEN** an SRT file has 100 blocks and block #5 has a non-numeric index
- **THEN** blocks 1-4 and 6-100 are parsed successfully and block #5 is skipped

### Requirement: SUB parser safe timestamp conversion

The SUB parser SHALL validate that frame numbers produce reasonable Duration values after floating-point conversion. If the computed duration exceeds 24 hours, the parser SHALL skip that entry with a `debug!`-level log (consistent with SRT's skip-and-continue behavior for malformed blocks).

#### Scenario: normal frame number converts correctly

- **WHEN** frame number is 3000 at 25fps
- **THEN** duration is 120000ms (2 minutes)

#### Scenario: absurdly large frame number is skipped

- **WHEN** frame number would produce a duration exceeding 24 hours
- **THEN** the entry is skipped with a debug log and the parser continues processing remaining entries
