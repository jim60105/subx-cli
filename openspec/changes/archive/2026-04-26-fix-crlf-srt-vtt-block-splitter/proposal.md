## Why

The SRT and VTT parsers split cue blocks on the literal LF-only sequence `"\n\n"`, which means CRLF-encoded files (Windows-authored subtitles, anything saved on a Windows host, anything served by certain CDNs) are mis-parsed: an entire CRLF SRT file collapses to a single cue whose payload absorbs the rest of the file (the round-trip happens to be byte-stable through the serializer, so the bug is silent), and a CRLF VTT file parses to zero cue entries. ASS and SUB are line-based so they are unaffected. This regression has been documented as a Known Limitation in the v1.7.0 CHANGELOG since `refactor-format-parsers` was kept strictly behavior-preserving; this change fixes it.

## What Changes

- Make the SRT block splitter (`src/core/formats/srt/parser.rs`) accept `\r\n`, `\n`, bare `\r`, and any mix as line terminators, recognizing all forms of blank-line cue-block separators (`\n\n`, `\r\n\r\n`, `\n\r\n`, `\r\n\n`, `\r\r`, etc.).
- Make the VTT block splitter (`src/core/formats/vtt/parser.rs`) line-ending-tolerant on the same terms, applied after the existing `WEBVTT` header strip.
- Preserve every existing parser behavior: defensive limits (1 MiB per cue, empty-input rejection, BOM strip, out-of-order cue preservation, negative-timestamp skip-and-continue, malformed-block skip-and-continue), out-of-order numeric SRT indexes, blank trailing lines, embedded blank lines inside multi-line cue text after the timestamp line.
- The 1 MiB per-cue cap (`MAX_CUE_BYTES`) is enforced on the *raw, pre-normalization* per-block byte length so an attacker cannot bypass the hardening guard by stuffing in `\r` bytes that disappear on normalization.
- Do **not** modify the on-disk byte form: serializers MUST keep emitting LF-terminated output as today. CRLF / bare-CR is only honored on the *input* path.
- Update the existing CRLF fixtures (`tests/fixtures/formats/srt/basic.crlf.srt`, `tests/fixtures/formats/vtt/basic.crlf.vtt`) and their entry-count locks in `tests/format_roundtrip_tests.rs` to assert the *correct* parsed entry counts (matching their LF counterparts), rather than the buggy `=> 1` and `=> 0` they currently encode. No new parallel fixtures are introduced.
- Remove the corresponding Known Limitation note from `CHANGELOG.md`.
- **Not BREAKING**: LF-only inputs (the dominant case) keep parsing exactly as today; the change only adds correct handling for CRLF / bare-CR / mixed inputs that previously parsed to garbage.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `subtitle-parser-hardening`: add a new requirement that the SRT and VTT parsers MUST split cue blocks correctly regardless of LF vs CRLF line endings, with scenarios for CRLF SRT, CRLF VTT, mixed-ending input, and the existing LF behavior.

## Impact

- **Code**: `src/core/formats/srt/parser.rs`, `src/core/formats/vtt/parser.rs`. No public API changes; `SubtitleFormat::parse` signatures and module paths stay identical.
- **Tests**: new fixtures under `tests/fixtures/formats/srt/` and `tests/fixtures/formats/vtt/` (byte-pinned via `.gitattributes`); new test cases in the round-trip harness (`tests/format_roundtrip_tests.rs`); inline unit tests in the parser modules.
- **Docs**: remove the CRLF caveat from `CHANGELOG.md` Known Limitations.
- **Dependencies**: none — implementation uses standard-library string operations (no regex crate added).
- **Performance**: O(n) parser pass remains; the splitter walks the input once.
- **Backward compatibility**: pure bug fix. LF inputs are byte-identical in and out; CRLF inputs that previously produced wrong parses now produce correct parses. Files that round-tripped to a byte-stable single-cue SRT file via the bug will after the fix round-trip to N correct cues with LF endings (this is intended; the buggy output was never a contract).
