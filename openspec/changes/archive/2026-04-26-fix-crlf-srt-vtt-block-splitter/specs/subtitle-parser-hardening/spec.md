## ADDED Requirements

### Requirement: SRT and VTT parsers handle CRLF line endings

The SRT parser (`src/core/formats/srt/parser.rs`) and the VTT parser (`src/core/formats/vtt/parser.rs`) SHALL split cue blocks correctly regardless of whether the input uses LF (`\n`), CRLF (`\r\n`), bare-CR (`\r`, old-Mac convention), or any mix as line terminators. A blank line between cue blocks — which terminates one block and starts the next — MUST be recognized whether it is encoded as `\n\n`, `\r\n\r\n`, `\n\r\n`, `\r\n\n`, `\r\r`, or any other pair of CR/LF terminators surrounding an otherwise-empty line.

This requirement applies only to the parser input path. Serializer output remains LF-terminated; CRLF→CRLF round-trip identity is NOT required.

The 1 MiB per-cue cap (`MAX_CUE_BYTES`) defined by the existing "SRT cue body size limit" / "VTT cue body size limit" requirements continues to be enforced on the *raw, pre-normalization* per-block byte length, so an attacker cannot submit a multi-MiB cue padded with `\r` characters that happens to normalize below the cap.

All other documented parser behavior remains unchanged: empty-input rejection, parser-level UTF-8 BOM strip, out-of-order cue index preservation, negative-timestamp skip-and-continue, malformed-block skip-and-continue (SRT), `WEBVTT` header validation (VTT), and `NOTE` / `STYLE` block skipping (VTT).

#### Scenario: CRLF SRT file parses every cue

- **WHEN** an SRT file uses `\r\n` line endings throughout, including `\r\n\r\n` between cue blocks, and contains N well-formed cues
- **THEN** the parser returns a `Subtitle` with exactly N entries whose indices, start/end timestamps, and text payloads match the LF-encoded equivalent of the same file

#### Scenario: CRLF VTT file parses every cue

- **WHEN** a VTT file begins with `WEBVTT\r\n\r\n` and uses `\r\n` line endings throughout, with `\r\n\r\n` separating cue blocks, and contains N well-formed cues
- **THEN** the parser returns a `Subtitle` with exactly N entries (matching the LF-encoded equivalent) and does NOT return zero entries

#### Scenario: Mixed LF and CRLF line endings parse correctly

- **WHEN** an SRT or VTT file mixes `\n` and `\r\n` line terminators (for example, header in CRLF, body in LF, or one cue block separator written as `\r\n\n`)
- **THEN** the parser still produces the same entry count and per-entry contents as a fully-normalized LF version of the same file

#### Scenario: Bare-CR (old-Mac) line endings parse correctly

- **WHEN** an SRT file uses bare `\r` as a line terminator (no `\n`), with `\r\r` separating cue blocks
- **THEN** the parser produces the same entry count and per-entry contents as the LF-encoded equivalent

#### Scenario: Multi-line cue text with CRLF line endings preserves text content

- **WHEN** an SRT or VTT cue's text payload spans multiple lines separated by `\r\n` (for example, two-line dialogue inside a single cue)
- **THEN** the parsed `SubtitleEntry::text` is byte-identical to the parsed `text` of the LF-encoded equivalent (multi-line cue text continues to be joined with `\n` as today)

#### Scenario: LF-only inputs continue to parse identically

- **WHEN** an SRT or VTT file uses LF-only line endings (the dominant case in the existing test fixtures)
- **THEN** the parser produces byte-identical results to the pre-fix implementation, with no observable change in entry count, indices, timestamps, or per-entry text

#### Scenario: Existing hardening behaviors are preserved on CRLF input

- **WHEN** a CRLF SRT or VTT input contains a malformed block (non-numeric SRT index, missing timing line, oversized cue, negative timestamp, or — for VTT — missing `WEBVTT` header)
- **THEN** the parser applies the exact same disposition (skip-and-continue, typed `SubXError`, etc.) it would apply to the LF-encoded equivalent

#### Scenario: 1 MiB per-cue cap is enforced on raw bytes, not normalized bytes

- **WHEN** a CRLF input contains a single cue block whose pre-normalization (raw) byte length exceeds the 1 MiB per-cue cap, even if the same block normalizes to ≤ 1 MiB after `\r\n` → `\n` collapsing
- **THEN** the parser returns a `SubXError::SubtitleFormat` mentioning the cap, exactly as it does for an oversized LF-encoded block, so an attacker cannot bypass the hardening guard by stuffing in `\r` bytes
