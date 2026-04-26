## ADDED Requirements

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
