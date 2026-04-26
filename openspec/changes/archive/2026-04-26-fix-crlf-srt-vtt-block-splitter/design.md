## Context

The current SRT and VTT cue-block splitters are LF-only string splits:

- `src/core/formats/srt/parser.rs:55` — `let blocks: Vec<&str> = content.split("\n\n").collect();`
- `src/core/formats/vtt/parser.rs:82` — `for block in content.split("\n\n") { ... }`

When a CRLF-encoded SRT or VTT file is fed to these parsers, the actual cue-block separator is `\r\n\r\n`, which contains the substring `\n\n` only at offset 1, so the split produces a single chunk that contains the entire file. SRT then accepts that single chunk as one absurdly large cue (entry-count = 1, payload absorbs the rest of the file); VTT rejects every cue and returns 0 entries.

The bug is silent in SRT because the round-trip through the LF-only serializer happens to produce byte-stable output for the degenerate single-cue case, so end-to-end tests with byte fixtures never tripped on it. It surfaces immediately for VTT because the entry count is 0.

ASS and SUB parsers are unaffected — both are line-based (`for line in input.lines()` semantics) and `str::lines()` already handles `\r\n` correctly.

The `refactor-format-parsers` change (archived 2026-04-26) deliberately preserved this LF-only behavior to keep the refactor strictly behavior-preserving; it documented the gap in `CHANGELOG.md` Known Limitations and `design.md`. This change closes that gap.

## Goals / Non-Goals

**Goals:**

- SRT and VTT parsers MUST split cue blocks correctly when the input uses `\r\n` line endings, `\n` line endings, or any mix of the two.
- All existing parser disposition behaviors are preserved verbatim: empty-input rejection, parser-level BOM strip, 1 MiB per-cue cap, out-of-order cue index preservation, negative-timestamp skip-and-continue, malformed-block skip-and-continue (SRT), `WEBVTT` header validation (VTT), and `NOTE`/`STYLE` block skipping (VTT).
- LF-only inputs produce byte-identical parse output to today (regression-locked via existing fixtures).
- CRLF SRT and CRLF VTT inputs produce the *same parsed `Subtitle` structure* as their LF counterparts (entry counts and per-entry contents identical; serialized output is LF-terminated as today).
- Round-trip golden-file regression coverage is added with byte-pinned CRLF fixtures.

**Non-Goals:**

- Changing the serializer output format. Output remains LF-terminated regardless of input line endings.
- Preserving CRLF round-trip identity. A CRLF input does NOT have to round-trip to a CRLF output — the proposal keeps serializer behavior unchanged. Round-trip identity remains scoped to LF inputs (matching today's contract).
- Touching ASS or SUB parsers. They are already correct.
- Adding a new dependency (no `regex` crate is already present and is already used by both parsers; we will reuse it where helpful, but the splitter does not require regex).
- Adding a public API for line-ending control. Detection is internal and automatic.
- Changing per-cue text representation. Multi-line cue text inside a block keeps its original line-ending characters from the lines API consumer's perspective; we do NOT smuggle CRLF into the in-memory `text` field — `str::lines()` already strips line terminators per Rust stdlib semantics.

## Decisions

### Decision 1: Normalize CRLF → LF before splitting

**Choice:** At the top of each parser (after the BOM strip and after the empty-input check), allocate a normalized `String` if and only if the input contains a `\r` byte:

```rust
// Pseudo
let normalized: Cow<'_, str> = if content.contains('\r') {
    Cow::Owned(content.replace("\r\n", "\n").replace('\r', "\n"))
} else {
    Cow::Borrowed(content)
};
let content: &str = &normalized;
```

Then the existing `content.split("\n\n")` call works unchanged.

The two-pass `replace` collapses `\r\n` first and then any remaining lone `\r` to `\n`. This deliberately treats bare-`\r` as a line terminator (old-Mac convention). Mixed sequences behave as: `\r\r\n` → `\r\n` → `\n\n` (a blank line — old-Mac followed by CRLF), `\n\r` → `\n\n` (a blank line — LF followed by old-Mac). These collapses are intentional and consistent with treating `\r` and `\n` as universal line-terminator bytes.

**Rationale:**

1. **Minimal blast radius.** The split logic, the per-block validators, the multi-line text concatenation, and the regex matches against the timing line all stay byte-for-byte identical for the LF case. Only one new line is added per parser, gated on `content.contains('\r')`.
2. **Allocation-free fast path.** LF-only inputs (the dominant case — every fixture in `tests/fixtures/formats/`, every test asset, every Linux-authored file) keep the zero-allocation `&str` path via `Cow::Borrowed`.
3. **Handles bare-`\r` correctly.** Old-Mac-style line endings (rare but possible in archives) collapse to LF too, matching the spec's "any mix" goal.
4. **No regex.** The existing `regex::Regex::new(r"\r?\n\r?\n")`-based split would work but is harder to reason about in the presence of trailing whitespace and would force a second compilation per call; keeping `replace` + `split` is simpler and zero-cost when not needed.
5. **`str::lines()` invariance.** The downstream `block.lines()` call inside per-block parsing relies on Rust's stdlib `Lines` iterator, which already strips `\r\n` *and* bare `\r` line terminators. So even if a CRLF fragment somehow leaked past the normalization, the downstream block handling would still yield correct line content.

**Alternatives considered:**

- **Regex-based split (`Regex::new(r"\r?\n\r?\n")`).** Heavier; constructs a new regex per call site (already two regexes per call); worse cache locality. Rejected.
- **Manual byte-walk splitter that recognizes `\r\n\r\n`, `\n\r\n\r\n` (mixed), `\n\n`, `\r\r` blank-line variants.** Fastest in theory but materially more code, harder to audit, easy to introduce off-by-one. Rejected.
- **Refuse CRLF and require callers to normalize.** Hostile to Windows users; violates the principle that the parser should "fail safely on malformed input" — but CRLF is *not* malformed, it is a valid universal line ending. Rejected.
- **Detect the line-ending convention up front and switch the split delimiter (`"\r\n\r\n"` vs `"\n\n"`) instead of normalizing.** Fails on mixed inputs. Rejected.

### Decision 2: Apply the normalization in both SRT and VTT, in the same shape

**Choice:** Identical helper logic at the top of `srt/parser.rs::parse` and `vtt/parser.rs::parse`. Optionally extract a `crate::core::formats::types::normalize_line_endings(&str) -> Cow<'_, str>` if the duplication is more than ~5 lines.

**Rationale:** Both parsers share the same hardening matrix; making the fix shape-identical keeps the parsers easy to audit side-by-side.

### Decision 3: VTT header check stays *before* normalization

**Choice:** The existing `content.trim_start().starts_with("WEBVTT")` check in `vtt/parser.rs` runs unchanged on the original `content` (before CRLF normalization). The `WEBVTT` token has no `\r` in it, so the check is unaffected by line endings either way; placing it before normalization preserves the source-position of the diagnostic and avoids one allocation in the error path.

### Decision 4: Serializer is unchanged

**Choice:** Both `SrtFormat::serialize` and `VttFormat::serialize` keep emitting LF-terminated output. CRLF input → LF output is intentional and aligns with the way every existing fixture (and every other tool in the ecosystem that emits SRT/VTT) treats round-trip semantics.

### Decision 5: Test strategy

- Add byte-pinned CRLF input fixtures: `tests/fixtures/formats/srt/crlf_basic.srt`, `tests/fixtures/formats/vtt/crlf_basic.vtt`. Their `.expected` companions store the LF-terminated serialized output, identical to the corresponding LF fixtures' `.expected`.
- Wire them into `tests/format_roundtrip_tests.rs` with `entries == 2` (or whatever count the LF counterpart locks) so a regression that re-introduces the LF-only splitter would change the entry count and fail loudly.
- Add focused unit tests inside each parser module: `crlf_only_input_parses_all_cues`, `mixed_lf_and_crlf_parses_correctly`, `bare_cr_blank_line_separates_blocks` (best-effort, may be marked as a softer assertion since bare `\r` in real files is vanishingly rare).
- Update `.gitattributes` if necessary so the new CRLF fixtures keep their bytes intact through git.

### Decision 6: Remove the Known Limitations entry

When the change is shipped, the corresponding `### Known limitations` paragraph in `CHANGELOG.md` (currently sitting under `## [Unreleased]` post-archive sync) is removed and a new `### Fixed` entry is added describing the parser fix.

### Decision 7: 1 MiB per-cue cap is enforced on raw (pre-normalization) bytes

**Choice:** The 1 MiB per-cue cap (`MAX_CUE_BYTES`) is enforced on the *original* input byte length of each block, not on the post-normalization length.

**Implementation:** When `content.contains('\r')`, walk the original buffer with a CRLF-aware splitter (same shape as the normalized walker — the trivial form is `original.split_terminator("\r\n\r\n")`-with-fallback or the regex `\r?\n\r?\n` constructed once) in parallel with the normalized split, and assert `original_block.len() <= MAX_CUE_BYTES` for every block. The post-normalization per-block check stays for free (it is implied by the pre-check since normalization never grows a buffer).

A simpler equivalent: skip the parallel walk and instead enforce the cap on the entire pre-normalization input length when `content.contains('\r')`, scaled by an estimate of block count — but this is too coarse. The parallel pre/post block-walk is the precise form.

**Rationale:** The existing `subtitle-parser-hardening` spec frames the 1 MiB cap as a defensive limit on per-cue *input* bytes ("a single SRT/VTT cue block exceeds the 1 MiB cap"). Allowing CRLF→LF normalization to slip larger inputs past the cap weakens that contract for direct `parse(&str)` callers (tests, library users) where the file-level `general.max_subtitle_bytes` guard at the read layer doesn't apply. Keeping the cap on raw bytes preserves the documented hardening behavior verbatim and lets this change stay an `ADDED` Requirement rather than a `MODIFIED` one.

**Rejected alternative:** Relax the cap to "normalized bytes". That would be a semantic change to existing hardening and would have to be a `MODIFIED Requirement` rather than a pure `ADDED`.

### Decision 8: Update existing CRLF fixtures, do not add parallel ones

**Choice:** The existing fixtures `tests/fixtures/formats/srt/basic.crlf.srt` and `tests/fixtures/formats/vtt/basic.crlf.vtt` already lock the buggy entry counts (`basic.crlf.srt => 1`, `basic.crlf.vtt => 0`) inside `expected_entry_count()` in `tests/format_roundtrip_tests.rs`. This change UPDATES those locks to the correct entry counts (`=> 2` for both, matching their LF counterparts) and updates the `.expected` companions to the LF-terminated correct serialization. No new `crlf_basic.*` fixtures are added.

**Rationale:** Adding parallel fixtures would leave the buggy locks in place and split CRLF coverage across two fixture pairs — the existing pair would either fail (because the fix changes the entry count from 1/0 to 2) or stay locked at the bug (defeating the purpose). Reusing the existing pair means the regression coverage is right where reviewers expect it.

## Risks / Trade-offs

- **Risk:** A user with a buggy pipeline that *depended* on the CRLF SRT degenerate single-cue output (e.g., they were using SubX to "validate" CRLF files end-to-end against a known-bad output) would now see a different (correct) parse. **Mitigation:** This was never a contract; the bug is documented in the v1.7.0 CHANGELOG as a Known Limitation; the fix is a strict improvement; release notes call it out under `### Fixed`.
- **Risk:** Allocation cost of `replace("\r\n", "\n").replace('\r', "\n")` for very large CRLF subtitle files (tens of MiB). **Mitigation:** Gated on `content.contains('\r')` so LF-only files pay zero cost. CRLF files allocate one ~N-byte `String`; this is bounded by the file-size guard (`general.max_subtitle_bytes`, default 50 MiB) at the command/file-read layer, so the allocation is ≤ the file size already accepted.
- **Risk:** The two `replace` calls scan the buffer twice. **Mitigation:** Acceptable for the SRT/VTT case (per-file, one-shot, bounded by the size guard). If profiling later shows it matters, swap to a single-pass `String::with_capacity` + manual loop without changing the public spec.
- **Trade-off:** The serializer still emits LF on CRLF input. This is intentional (Decision 4) but means CRLF→CRLF byte-stability is not promised. Anyone needing CRLF output can post-process — but in practice no one needs this and we'd rather not branch the serializer.
- **Trade-off:** We don't add a public `normalize_line_endings` API even if the helper is extracted. It stays `pub(crate)` (or `pub(super)`) so we don't expose internal parser plumbing across the SubtitleFormat trait surface.

## Migration Plan

This is a pure bug fix; there is no migration:

1. Land the parser changes and the fixtures.
2. Run the full quality-check pipeline (linting, parser unit tests, integration tests, round-trip fixtures, doc tests, slow-tests fuzzing harness).
3. Update `CHANGELOG.md`: drop the `### Known limitations` paragraph for SRT/VTT CRLF; add a `### Fixed` entry under the active section.
4. No config, no schema, no CLI flag changes. No deprecation notices required.
5. Rollback: if the fix is found to regress an unforeseen scenario, the change can be reverted by a single revert commit; the `Cow::Borrowed` fast path means LF behavior is unchanged regardless.

## Open Questions

None. The decisions above are deliberately conservative.
