# Subtitle Styling

## Purpose

Preserve or gracefully degrade subtitle styling (bold, italic, underline, font color, ASS style sections, and frame-rate metadata) when parsing and converting between the supported subtitle formats SRT, ASS/SSA, WebVTT, and MicroDVD/SubViewer SUB. Implemented in `src/core/formats/styling.rs`, `src/core/formats/manager.rs`, `src/core/formats/ass.rs`, `src/core/formats/vtt.rs`, and `src/core/formats/sub.rs`, and dispatched through `FormatManager` / `FormatConverter`.

## Requirements

### Requirement: SRT and ASS Inline Tag Mapping

The system SHALL translate SRT inline HTML-like styling tags to and from ASS override tags for bold, italic, and underline when converting between SRT and ASS, using the following mapping: `<b>...</b>` to/from `{\b1}...{\b0}`, `<i>...</i>` to/from `{\i1}...{\i0}`, and `<u>...</u>` to/from `{\u1}...{\u0}`. Implemented in `src/core/formats/styling.rs` (`convert_srt_tags_to_ass`, `convert_ass_tags_to_srt`).

#### Scenario: Bold SRT converted to ASS
- **GIVEN** an SRT cue text `Hello <b>world</b>`
- **WHEN** `FormatConverter::convert_srt_tags_to_ass` is invoked on the cue text
- **THEN** the output SHALL be `Hello {\b1}world{\b0}`

#### Scenario: Italic ASS converted to SRT
- **GIVEN** an ASS dialogue text `{\i1}emphasis{\i0} done`
- **WHEN** `FormatConverter::convert_ass_tags_to_srt` is invoked
- **THEN** the output SHALL be `<i>emphasis</i> done`

#### Scenario: Underline round-trip
- **GIVEN** an SRT cue text `<u>title</u>`
- **WHEN** the text is converted to ASS via `convert_srt_tags_to_ass` and then back via `convert_ass_tags_to_srt`
- **THEN** the final text SHALL equal `<u>title</u>`

### Requirement: SRT Font Color Translation to ASS

The system SHALL translate SRT `<font color="...">...</font>` tags to ASS color override tags of the form `{\c&H<color>&}...{\c}`, deriving the ASS color payload by stripping a leading `#` from the SRT color string. Implemented in `src/core/formats/styling.rs` (`convert_srt_tags_to_ass`, `convert_color_to_ass`).

#### Scenario: Hex color converted
- **GIVEN** an SRT cue text `<font color="#FF0000">red</font>`
- **WHEN** `convert_srt_tags_to_ass` is invoked
- **THEN** the output SHALL contain `{\c&HFF0000&}red{\c}`

### Requirement: Styling Flags Extracted From SRT

The system SHALL expose `FormatConverter::extract_srt_styling`, which returns a `StylingInfo` whose `bold`, `italic`, and `underline` flags are set to `true` when the corresponding HTML-like tags (case-insensitive opening forms `<b>`/`<B>`, `<i>`/`<I>`, `<u>`/`<U>`) are present in the SRT cue text.

#### Scenario: Detect bold and italic
- **GIVEN** an SRT cue text `<B>hi</B> and <i>there</i>`
- **WHEN** `extract_srt_styling` is called
- **THEN** the returned `StylingInfo` SHALL have `bold = true`, `italic = true`, and `underline = false`

### Requirement: ASS Style Sections Round-Tripped on ASS-to-ASS

When a subtitle is parsed from ASS and then serialized back to ASS, the system SHALL emit the `[Script Info]`, `[V4+ Styles]`, and `[Events]` sections with their `Format:` header lines, preserving the dialogue timing, text, and style fields that were read from the source. Implemented in `src/core/formats/ass.rs` (`AssFormat::parse`, `AssFormat::serialize`).

#### Scenario: ASS dialogue re-emitted
- **GIVEN** an ASS document containing a `[V4+ Styles]` section and a `Dialogue: 0,0:00:01.00,0:00:02.50,Default,,0000,0000,0000,,Hello\NASS` line
- **WHEN** the document is parsed by `AssFormat::parse` and then serialized by `AssFormat::serialize`
- **THEN** the output SHALL contain the section headers `[Script Info]`, `[V4+ Styles]`, and `[Events]`, the `Format:` lines for styles and events, and a dialogue line reproducing the same start time, end time, and text content

### Requirement: ASS-to-SRT Tag Stripping Fallback

When converting ASS content to a format that does not support ASS override tags, the system SHALL provide `FormatConverter::strip_ass_tags` to remove any `{...}` override blocks, so that unsupported styling is dropped gracefully rather than appearing as literal tags in the target format.

#### Scenario: Unknown ASS override dropped
- **GIVEN** an ASS dialogue text `{\pos(100,200)}{\fs30}Hello`
- **WHEN** `FormatConverter::strip_ass_tags` is invoked
- **THEN** the result SHALL be `Hello`

### Requirement: VTT Preamble and Cue Structure

The VTT parser SHALL accept input whose first block begins with `WEBVTT`, SHALL skip `NOTE` and `STYLE` blocks, and SHALL parse timestamp lines of the form `HH:MM:SS.mmm --> HH:MM:SS.mmm`. Any text following the timestamp line (including lines placed after optional cue identifiers and any trailing cue-setting text on the timestamp line) SHALL be preserved verbatim as the cue body. Implemented in `src/core/formats/vtt.rs` (`VttFormat::parse`, `VttFormat::serialize`).

#### Scenario: NOTE and STYLE blocks ignored
- **GIVEN** a VTT document containing a `WEBVTT` header, a `NOTE` block, a `STYLE` block, and three cues
- **WHEN** `VttFormat::parse` is invoked
- **THEN** the resulting `Subtitle` SHALL contain exactly three entries whose text matches the body of each cue

#### Scenario: Cue identifier skipped
- **GIVEN** a VTT cue whose first line is a non-timestamp identifier and whose second line is `00:00:01.000 --> 00:00:03.000`
- **WHEN** the cue is parsed
- **THEN** the entry text SHALL be the lines following the timestamp line, and the timing SHALL be 1.000 s to 3.000 s

### Requirement: VTT HTML-like Tag Stripping

The system SHALL expose `FormatConverter::strip_vtt_tags` which removes any HTML-like tag matching `</?[^>]+>` from VTT cue text, so conversions from VTT into formats that do not support VTT inline markup drop those tags gracefully.

#### Scenario: VTT tags stripped
- **GIVEN** a VTT cue text `<c.yellow>Hi</c> <b>there</b>`
- **WHEN** `strip_vtt_tags` is invoked
- **THEN** the result SHALL be `Hi there`

### Requirement: SRT and VTT Tag Conversion Defaults

The system SHALL provide `FormatConverter::convert_srt_tags_to_vtt` and `FormatConverter::convert_vtt_tags_to_srt`, which SHALL pass the cue text through unchanged so that the HTML-like tag subset shared by both formats (e.g., `<b>`, `<i>`, `<u>`) is preserved on SRT-to-VTT and VTT-to-SRT conversion.

#### Scenario: SRT bold preserved into VTT
- **GIVEN** an SRT cue text `<b>hello</b>`
- **WHEN** `convert_srt_tags_to_vtt` is invoked
- **THEN** the output SHALL equal the input `<b>hello</b>`

### Requirement: Frame-Based SUB Timing

The SUB format parser and serializer SHALL treat timing as frame counts in braces (`{start_frame}{end_frame}`) and SHALL convert between frames and `Duration` using the subtitle's `metadata.frame_rate` when available and a default of 25.0 fps otherwise. Multi-line text SHALL be encoded on disk with `|` as the line separator and converted to and from `\n` in the in-memory `SubtitleEntry.text`. Implemented in `src/core/formats/sub.rs` (`SubFormat::parse`, `SubFormat::serialize`, `DEFAULT_SUB_FPS`).

#### Scenario: Default 25 fps conversion
- **GIVEN** a SUB line `{0}{25}Hello`
- **WHEN** `SubFormat::parse` is invoked
- **THEN** the resulting entry SHALL have `start_time = 0 ms` and `end_time = 1000 ms`

#### Scenario: Multi-line pipe encoding
- **GIVEN** an in-memory `SubtitleEntry` with text `Line1\nLine2` and timing 0 s to 1 s at 25 fps
- **WHEN** `SubFormat::serialize` is invoked
- **THEN** the output SHALL contain `{0}{25}Line1|Line2`

### Requirement: Graceful Degradation Into SUB

Because the SUB format has no native inline styling, conversions into SUB SHALL drop source styling rather than emit literal unsupported tags; callers SHALL rely on the ASS and VTT tag-stripping helpers (`strip_ass_tags`, `strip_vtt_tags`) before handing cue text to `SubFormat::serialize`.

#### Scenario: ASS styled cue flattened into SUB text
- **GIVEN** an ASS dialogue text `{\b1}bold{\b0} plain`
- **WHEN** it is passed through `FormatConverter::strip_ass_tags` before serialization by `SubFormat`
- **THEN** the text written into the SUB output SHALL be `bold plain` with no `{...}` override tags remaining

### Requirement: End-to-End ASS-to-SRT Drops Inline Override Styling

The end-to-end `FormatConverter::ass_to_srt` pipeline SHALL strip ASS override tags from each dialogue entry via `strip_ass_tags` before optionally re-applying `convert_ass_tags_to_srt`. As a consequence, inline override styling that existed in the ASS source (for example `{\b1}...{\b0}`) SHALL be removed and SHALL NOT reappear as SRT HTML-like tags in the resulting SRT file, even when `preserve_styling` is enabled. Callers that need inline tag translation SHALL invoke `convert_ass_tags_to_srt` directly on the original text. Implemented in `src/core/formats/transformers.rs::ass_to_srt` and `src/core/formats/styling.rs`.

#### Scenario: Bold override is stripped during end-to-end conversion
- **GIVEN** an ASS subtitle whose dialogue text is `{\b1}Title{\b0}` and a `FormatConverter` with `preserve_styling = true`
- **WHEN** `ass_to_srt` runs
- **THEN** the resulting SRT entry's text SHALL equal `Title` and SHALL NOT contain `<b>` or `</b>`

#### Scenario: Helper call still performs direct mapping
- **GIVEN** the same text `{\b1}Title{\b0}`
- **WHEN** a caller invokes `FormatConverter::convert_ass_tags_to_srt` directly on that text (without first stripping)
- **THEN** the returned string SHALL equal `<b>Title</b>`
