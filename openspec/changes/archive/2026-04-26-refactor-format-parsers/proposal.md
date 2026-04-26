## Why

`src/core/formats/mod.rs` has grown to ~1,800 lines and mixes three concerns
that should be independently navigable: (a) shared subtitle data types
(`Subtitle`, `SubtitleEntry`, `SubtitleMetadata`, `StylingInfo`,
`SubtitleFormatType`), (b) the `SubtitleFormat` trait contract, and (c) a
single monolithic `#[cfg(test)] mod tests` block that exercises types and
formats together. The four per-format files (`srt.rs`, `ass.rs`, `vtt.rs`,
`sub.rs`) each implement parsing and serialization but carry inconsistent —
or in some cases nearly absent — inline tests, while the existing
`subtitle-parser-hardening` spec only enumerates a handful of malformed-input
scenarios. As we add more hardening rules and consider corpus/fuzz-style
testing, the current layout makes it hard to locate per-format logic, prove
behavior preservation, and grow targeted coverage without further bloating
`mod.rs`.

This change refactors the format module into focused per-format submodules
(parser + serializer + tests co-located) backed by a small shared
`types`/`trait` core, and expands the parser-hardening capability with a
documented malformed-input matrix, golden-file regression strategy, and an
optional, CI-stable fuzz/corpus harness.

## What Changes

- Split `src/core/formats/mod.rs` so it only re-exports public items and
  declares submodules; move the shared data types **and** the
  `SubtitleFormat` trait into a single new `formats::types` submodule
  (chosen as the canonical location across this change — no separate
  `traits.rs`, no inline-facade alternative). **Not BREAKING** — every
  public path remains re-exported from `crate::core::formats`.
- Reorganize each per-format file (`srt.rs`, `ass.rs`, `vtt.rs`, `sub.rs`)
  into a directory module with co-located parser, serializer, time helpers,
  and `tests.rs` (e.g. `formats/srt/{mod,parser,serializer,time,tests}.rs`)
  while keeping `FormatManager::new()` registration and the `SubtitleFormat`
  impl public surface unchanged.
- Extract the existing monolithic `tests` block in `mod.rs` into a new
  `formats::types::tests` module covering only shared types, and move
  format-specific assertions into the corresponding per-format `tests.rs`.
- Add a regression harness: golden-file fixtures under
  `tests/fixtures/formats/<format>/` plus a snapshot-style integration test
  (`tests/format_roundtrip_tests.rs`) that parses canonical inputs, asserts
  structural equality, and re-serializes to a normalized form for
  byte-stable comparison.
- Expand `subtitle-parser-hardening` requirements to define a malformed-input
  matrix (truncated header, BOM-mixed content, negative/overflow timestamps,
  out-of-order cues, gigantic single cue guarded by a parser-local fixed
  byte cap, mismatched ASS `Format:` columns, VTT cues without blank-line
  separators, SUB frame numbers exceeding 24 h) with explicit "skip and
  continue" vs. "return typed error" semantics per scenario. The matrix
  is **additive**: existing per-format requirements in
  `subtitle-parser-hardening` (ASS missing `Format:` fields, ASS timestamp
  overflow, SRT skip-bad-block, SUB > 24 h frame guard) are NOT removed
  or rephrased — the matrix consolidates and extends them, and any
  scenario already covered by an existing requirement remains
  authoritative under that existing requirement. New scenarios introduced
  here (empty input, BOM consumption at the parser layer, per-cue byte
  cap, ASS column-count mismatch, VTT EOF without trailing blank line,
  out-of-order cue preservation) are net-new behavior.
- Note on encoding-layer interaction: BOM stripping for files read via
  `FormatManager::parse_file` already happens in
  `src/core/formats/encoding/converter.rs::skip_bom` before
  `SubtitleFormat::parse(&str)` is called. Adding parser-level BOM
  consumption is a deliberate *new defensive layer* that protects
  callers who feed `&str` content directly through `parse_auto` /
  `SubtitleFormat::parse` (e.g. tests, in-memory pipelines). This is
  not a behavior-preservation requirement but an explicit hardening
  addition, scoped under `subtitle-parser-hardening`.
- Introduce an opt-in, CI-stable corpus/fuzz strategy: a deterministic,
  **hand-rolled** structural mutator (no new crate dependency) gated behind
  a `slow-tests` cargo feature, plus a documented `cargo-fuzz` target
  layout that lives outside the default workspace so it does not affect
  normal builds or CI runtime. Avoiding `proptest` keeps `Cargo.lock`
  resolution and the default dev-dependency closure unchanged.
- Document the incremental migration plan and behavior-preservation
  guarantees in `design.md` so reviewers can verify each step is
  byte-equivalent on the golden corpus.

No public API, CLI surface, configuration key, or on-disk file format
changes. No new runtime dependency is added to the default build.

## Capabilities

### New Capabilities
- None. This change reorganizes existing code and extends an existing
  hardening spec.

### Modified Capabilities
- `format-conversion`: clarify that parser/serializer reorganization MUST
  preserve byte-equivalent output for the golden corpus and that
  `FormatManager` registration semantics are unchanged. Adds a regression
  requirement for round-trip stability across the four supported formats.
- `subtitle-parser-hardening`: extend the requirements with the
  malformed-input matrix described above and add a requirement for
  property-style coverage gated behind the `slow-tests` feature so default
  CI runtime is not affected.

## Impact

- **Affected code**: `src/core/formats/mod.rs`, `src/core/formats/srt.rs`,
  `src/core/formats/ass.rs`, `src/core/formats/vtt.rs`,
  `src/core/formats/sub.rs`, `src/core/formats/manager.rs`,
  `src/core/formats/converter.rs` (only registration imports), and
  `src/core/formats/styling.rs` (re-export only). All public paths remain
  re-exported from `crate::core::formats`.
- **Tests**: new `tests/format_roundtrip_tests.rs`, new fixture directory
  `tests/fixtures/formats/`, and per-format `tests.rs` files inside each
  format submodule. The existing inline `tests` block in `mod.rs` is
  decomposed without losing any assertion.
- **Specs**: deltas to `format-conversion` and `subtitle-parser-hardening`.
- **Dependencies**: no new runtime dependency, **and no new
  dev-dependency** for the property/mutator harness — the slow-tests
  mutator is hand-rolled. The only optional new dev-dependency in this
  change is `pretty_assertions` for diff-friendly round-trip failures.
  `cargo-fuzz` targets, if added, live in a sibling `fuzz/` workspace and
  are not part of the default build, default test run, or
  quality-check script.
- **CI / quality gates**: `scripts/quality_check.sh` behavior is unchanged;
  the new round-trip tests run under default `cargo nextest run`. Heavy
  property/fuzz work is opt-in.
- **Risks**: behavior drift in serializers (e.g. trailing newlines,
  timestamp formatting) — mitigated by the golden corpus. Parser performance
  regressions — mitigated by keeping `regex` usage and control flow
  unchanged within each per-format module. Note that the existing
  `benches/` suite (`retry_performance`, `file_id_generation_bench`)
  does NOT exercise format parsers, so a "no benchmark regression"
  guarantee is not available from current tooling; the design defers
  adding a parser benchmark to a follow-up change and instead relies on
  the round-trip harness plus targeted micro-benchmarks run ad-hoc by
  the implementer if a hot path is touched.
- **Pre-existing parser quirks the refactor MUST preserve**: SRT and
  VTT parsers split blocks with the literal pattern `"\n\n"` (not
  `"\r?\n\r?\n"`). Contrary to a tempting intuition, `"\n\n"` is NOT a
  substring of `"\r\n\r\n"`, so CRLF files behave asymmetrically: ASS
  and SUB parse identically to LF (line-based parsing), but SRT CRLF
  collapses into a single block whose text payload absorbs the
  remaining cues, and VTT CRLF parses to zero cue entries because the
  trailing `\r` on cue marker lines defeats the time regex. Both
  behaviors round-trip byte-stably through the serializer for the
  current corpus, and golden fixtures MUST include at least one CRLF
  input per format to lock these quirks in place. Properly normalizing
  CRLF in the SRT/VTT splitters is deferred to a follow-up change.
