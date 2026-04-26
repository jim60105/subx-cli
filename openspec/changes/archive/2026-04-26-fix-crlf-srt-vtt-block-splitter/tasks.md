## 1. Implementation

- [x] 1.1 In `src/core/formats/srt/parser.rs`, add CRLF normalization at the top of `parse()` (after the BOM strip and the empty-input check). Use `Cow::Borrowed` when `content.contains('\r')` is false to keep the LF fast path zero-allocation; use `Cow::Owned(content.replace("\r\n", "\n").replace('\r', "\n"))` otherwise. Bind the result back to `content: &str` so the existing `content.split("\n\n")` call works unchanged.
- [x] 1.2 In `src/core/formats/vtt/parser.rs`, apply the same CRLF normalization shape *after* the existing `content.trim_start().starts_with("WEBVTT")` header check (so the diagnostic source position and zero-cost LF fast path are preserved), and before the `content.split("\n\n")` loop.
- [x] 1.3 Enforce the 1 MiB `MAX_CUE_BYTES` cap on the *raw, pre-normalization* per-block byte length per design Decision 7. When `content.contains('\r')`, walk the original buffer with a CRLF-aware splitter in parallel with the normalized split and assert each pre-normalization block segment is ≤ `MAX_CUE_BYTES`. The existing post-normalization block check stays (it is implied by the pre-check since normalization never grows a buffer).
- [x] 1.4 Verify (manual code review) that no other site in `src/core/formats/{srt,vtt}/**` relies on the pre-normalization `\r` bytes — in particular that `block.lines()` and the timing regexes operate on the normalized buffer.

## 2. Unit tests (per-parser)

- [x] 2.1 Add `crlf_only_input_parses_all_cues` to `src/core/formats/srt/tests.rs`: feed a 3-cue SRT body with `\r\n` everywhere (including `\r\n\r\n` between blocks) and assert the parsed entry count, indices, timestamps, and text payloads match the LF baseline.
- [x] 2.2 Add `crlf_only_input_parses_all_cues` to `src/core/formats/vtt/tests.rs` (mirror, including the `WEBVTT\r\n\r\n` header).
- [x] 2.3 Add `mixed_lf_and_crlf_parses_correctly` to both test modules: build a buffer that mixes `\r\n` and `\n` separators (e.g., header in CRLF, body in LF, plus one `\r\n\n` separator inside the body) and assert the same entry count and contents as the pure-LF baseline.
- [x] 2.4 Add `bare_cr_blank_line_separates_blocks` to both test modules: feed an SRT/VTT body that uses bare `\r` as the line terminator (with `\r\r` between cue blocks) and assert it parses to the same entries as the LF baseline.
- [x] 2.5 Add `multi_line_cue_text_with_crlf_preserves_text` to both test modules: a single cue whose text payload is two lines separated by `\r\n`, asserted to produce a `text` field byte-identical to the LF-encoded equivalent (joined with `\n`).
- [x] 2.6 Add a regression test in `src/core/formats/srt/tests.rs` that confirms a CRLF SRT input no longer collapses to a single cue (entry count > 1 and the text payload of the first entry does NOT contain "-->").
- [x] 2.7 Add a regression test in `src/core/formats/vtt/tests.rs` that confirms a CRLF VTT input no longer parses to zero entries (entry count == expected count).
- [x] 2.8 Add `crlf_oversized_cue_caps_on_raw_bytes` to `src/core/formats/srt/tests.rs`: a CRLF block whose pre-normalization byte length exceeds 1 MiB but whose post-normalization length is ≤ 1 MiB MUST still return the typed cap error (verifies design Decision 7).
- [x] 2.9 Add a CRLF + malformed-block test for SRT: a CRLF input with a non-numeric index in the middle still skip-and-continues (the surrounding cues still parse).

## 3. Round-trip golden fixtures (update existing, do not add parallel ones)

- [x] 3.1 Update `tests/format_roundtrip_tests.rs` `expected_entry_count()` to set `"basic.crlf.srt" => 2` (was `=> 1`) and `"basic.crlf.vtt" => 2` (was `=> 0`), matching their LF counterparts. This is the regression lock that catches re-introduction of the LF-only splitter.
- [x] 3.2 Inspect `tests/fixtures/formats/srt/basic.crlf.srt` to confirm it actually encodes 2 cues in CRLF; if it does not, replace it with a byte-pinned 2-cue CRLF fixture and update its `.expected` companion to the LF-terminated correct serialization (matching the LF `basic.srt.expected` byte-for-byte). Do the same for `basic.crlf.vtt` and `basic.crlf.vtt.expected`.
- [x] 3.3 Verify `.gitattributes` continues to disable EOL/text normalization for `tests/fixtures/formats/**` so the CRLF bytes survive `git checkout` on Windows; extend if needed.
- [x] 3.4 Confirm no other test or doc snippet references the buggy entry counts (`basic.crlf.srt => 1` or `basic.crlf.vtt => 0`); update or delete any stale references.

## 4. Documentation and release notes

- [x] 4.1 Update `CHANGELOG.md`: under the active `## [Unreleased]` (or the next version section if Unreleased is empty), add a `### Fixed` entry describing the CRLF SRT/VTT parser fix and crediting the v1.7.0 Known Limitation as the source.
- [x] 4.2 In `CHANGELOG.md`, remove the SRT/VTT CRLF paragraph from the v1.7.0 `### Known limitations` block (delete just that paragraph; preserve the heading if other limitations remain, or remove the heading if it becomes empty).
- [x] 4.3 Update the rustdoc disposition matrix on `SrtFormat` and `VttFormat` (in `src/core/formats/srt/mod.rs` and `src/core/formats/vtt/mod.rs`) to state that LF, CRLF, bare-CR, and mixed line endings are accepted on input, and that the per-cue size cap is enforced on raw pre-normalization bytes.

## 5. Verification

- [x] 5.1 Run `scripts/quality_check.sh` (single invocation by the main agent) and confirm: `cargo fmt`, `cargo clippy -- -D warnings`, doc tests, and the full `cargo nextest run` all pass.
- [x] 5.2 Run the parser-scoped tests directly with `cargo nextest run --filter-expr 'package(subx-cli) and (test(srt) or test(vtt))' || true` and inspect the output for unexpected failures.
- [x] 5.3 Run the round-trip harness with the fixture path filter to confirm the CRLF entry-count locks now pass at the corrected counts.
- [x] 5.4 Run `cargo test --doc --all-features` and confirm doc tests still pass after rustdoc updates.
- [x] 5.5 Confirm `openspec validate fix-crlf-srt-vtt-block-splitter --strict` exits 0.

## 6. OpenSpec sync (after implementation)

- [x] 6.1 Sync the delta spec into `openspec/specs/subtitle-parser-hardening/spec.md` (one new requirement: "SRT and VTT parsers handle CRLF line endings").
- [ ] 6.2 Archive the change to `openspec/changes/archive/<date>-fix-crlf-srt-vtt-block-splitter/`.
- [ ] 6.3 Commit using the conventional-commit + Copilot co-author trailer convention.
