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

### Requirement: Malformed-input disposition matrix

Every subtitle format parser SHALL classify each malformed-input scenario
listed below as either **return-typed-error** (the parse call returns a
`SubXError::SubtitleFormat` and produces no `Subtitle`) or
**skip-and-continue** (the offending block/cue is skipped, a `debug!` log
records the skip, and parsing of the remaining file proceeds). No parser
SHALL panic, abort, or produce a `Subtitle` containing silently corrupted
entries for any scenario in the matrix.

This matrix is **additive** to the existing per-format requirements in
this capability (`ASS parser error recovery on missing format fields`,
`ASS timestamp overflow protection`, `SRT parser continues on malformed
blocks`, `SUB parser safe timestamp conversion`). Where a scenario in
the matrix overlaps an existing requirement (ASS `Format:` missing
fields, ASS timestamp overflow, SRT skip-bad-block, SUB > 24 h), the
existing requirement remains authoritative for that scenario; the
matrix entry restates the disposition for completeness but does not
relax it. Net-new scenarios introduced by this matrix are: empty input,
parser-level BOM consumption, out-of-order cue preservation, ASS
`Format:`-vs-`Dialogue:` column-count mismatch, per-cue byte cap, and
VTT EOF without trailing blank line.

The disposition for each scenario is:

| Scenario | Disposition |
| --- | --- |
| Empty input file | return-typed-error |
| Truncated header (e.g. ASS without `[Events]`, VTT without `WEBVTT`) | return-typed-error |
| BOM-prefixed UTF-8 valid content | parse successfully (BOM is consumed) |
| BOM-prefixed UTF-8 invalid content for parsers that validate a header (VTT `WEBVTT`, ASS `[Events]`) | return-typed-error |
| BOM-prefixed UTF-8 invalid content for parsers without a header guard (SRT, SUB) | parse-empty-or-skip-and-continue (the BOM is stripped, then existing per-block error recovery applies — this preserves current behavior) |
| Single malformed cue/block among valid ones | skip-and-continue |
| Out-of-order cues by timestamp | parse successfully (order preserved) |
| Negative timestamp | skip-and-continue |
| Timestamp arithmetic overflow | return-typed-error |
| Cue whose body or surrounding block exceeds a parser-local fixed per-cue byte cap | return-typed-error |
| ASS `Format:` line missing `Start`, `End`, or `Text` (validated at the first `Dialogue:` row encountered) | return-typed-error |
| ASS `Dialogue:` row with fewer comma-separated fields than the `Format:` line declares | skip-and-continue |
| VTT cue missing trailing blank-line separator at EOF | parse successfully |
| SUB frame number that decodes to > 24 h | skip-and-continue |
| SUB non-numeric frame range | skip-and-continue |

#### Scenario: empty input is rejected with typed error

- **WHEN** any parser is given an empty byte slice
- **THEN** it SHALL return `SubXError::SubtitleFormat` and SHALL NOT
  panic

#### Scenario: BOM-prefixed valid content parses successfully

- **WHEN** an SRT, VTT, or ASS file begins with a UTF-8 BOM (`EF BB BF`)
  and the remaining bytes are valid for the format
- **THEN** the parser SHALL consume the BOM and return a populated
  `Subtitle`
- **AND** this parser-level BOM consumption SHALL be a *new defensive
  layer* additive to the encoding-layer BOM strip in
  `src/core/formats/encoding/converter.rs::skip_bom`; both layers
  coexist by design so callers using `FormatManager::parse_auto` or
  direct `<Fmt>Format::parse(&str)` on in-memory strings (which bypass
  the encoding layer) also receive BOM tolerance

#### Scenario: out-of-order cues are preserved

- **WHEN** an SRT or VTT file contains cues whose start times are not
  monotonically increasing
- **THEN** the parser SHALL return a `Subtitle` whose `entries` preserve
  the file's original order

#### Scenario: ASS Dialogue row with too few columns is skipped

- **WHEN** an ASS file's `Format:` line declares N columns and a
  `Dialogue:` row supplies fewer comma-separated fields than N
- **THEN** the parser SHALL skip that single `Dialogue:` row, log at
  `debug!` level, and continue parsing remaining rows
- **AND** rows with extra commas inside the trailing `Text` column are
  preserved verbatim because ASS `Text` is the last column and may
  legitimately contain commas; the parser uses
  `splitn(N, ',')` so additional commas after the `Text` column
  boundary remain part of the cue text rather than being treated as a
  column-count mismatch

#### Scenario: per-cue size cap is enforced

- **WHEN** a single cue's text would exceed a parser-local fixed per-cue
  byte cap defined as the constant
  `const MAX_CUE_BYTES: usize = 1 * 1024 * 1024;` (1 MiB) inside the
  parser module
- **THEN** the parser SHALL return `SubXError::SubtitleFormat` rather
  than allocating an unbounded string
- **AND** the cap SHALL NOT depend on `general.max_subtitle_bytes` or
  any other configuration value, because `SubtitleFormat::parse(&self,
  content: &str)` does not receive a `ConfigService` and threading
  configuration into parser construction is out of scope for this change.
  File-level enforcement of `general.max_subtitle_bytes` continues to
  live at the command/file-read layer, unchanged.

#### Scenario: VTT trailing blank line is optional

- **WHEN** a VTT file ends without a trailing blank line after its final
  cue
- **THEN** the parser SHALL still recognize that final cue and return a
  populated `Subtitle`

### Requirement: Property-style parser fuzz coverage gated by feature

The system SHALL provide property-style tests that feed each parser
randomized and structurally-mutated inputs and assert that (a) no panic
occurs and (b) any error returned is a typed `SubXError` variant. These
tests MUST be gated behind the existing `slow-tests` cargo feature so the
default `cargo nextest run` invocation remains unaffected. The mutator
SHALL be hand-rolled using only `std` and crates already present in the
default dev-dependency closure (no new property-testing dependency such
as `proptest` or `quickcheck` is added, even feature-gated, so that
`Cargo.lock` resolution is unchanged regardless of feature flags).

#### Scenario: property tests do not run by default

- **WHEN** `cargo nextest run` is invoked without `--features slow-tests`
- **THEN** the property/fuzz test cases SHALL NOT execute and overall
  test runtime SHALL match pre-change baselines within normal variance

#### Scenario: property tests catch panics under slow-tests

- **WHEN** `cargo nextest run --features slow-tests` is invoked
- **THEN** the property tests SHALL execute against each of the four
  format parsers, every iteration SHALL either return `Ok(_)` or a
  `SubXError`, and any panic SHALL fail the test run

### Requirement: Optional cargo-fuzz harness lives outside default workspace

If a `cargo-fuzz` harness is added for any format parser, it SHALL live
in a sibling `fuzz/` directory that is excluded from the main
`Cargo.toml` workspace members. The harness MUST NOT be required for any
default build, default test run, `scripts/quality_check.sh` invocation,
or release build, so that CI runtime and the supply-chain footprint of
the default build are unchanged.

#### Scenario: default workspace build excludes fuzz targets

- **WHEN** `cargo build` is invoked at the workspace root
- **THEN** no `cargo-fuzz` target SHALL be built and no `cargo-fuzz`-only
  dependency SHALL be resolved into the default workspace's
  `Cargo.lock` (this exclusion is specifically about `cargo-fuzz` and
  any crates that exist only to support fuzz harnesses; ordinary
  dev-dependencies introduced for unit/integration tests remain allowed
  as long as they are not added by this change)

#### Scenario: quality check does not depend on fuzz harness

- **WHEN** `scripts/quality_check.sh` (or `-v -p ci --full`) is invoked
- **THEN** it SHALL pass without requiring `cargo-fuzz` to be installed
  and without invoking any fuzz target

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
