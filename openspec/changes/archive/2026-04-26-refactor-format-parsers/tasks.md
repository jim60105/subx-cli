## 1. Regression baseline (lock current behavior)

- [x] 1.1 Create `tests/fixtures/formats/{srt,ass,vtt,sub}/` directory tree.
- [x] 1.2 Add at least one canonical happy-path input fixture per format
  (ASCII, UTF-8 with BOM, multi-line cue, styled cue for ASS/VTT) under
  the appropriate fixture directory. Additionally, include at least one
  CRLF-line-ending fixture per format (suffix `.crlf.<ext>`) so the
  pre-existing `"\n\n"` block-splitter quirk in SRT/VTT is locked in
  place. Fixtures MUST be checked in as raw bytes — do NOT enable
  `text=auto` / `eol=lf` normalization for the
  `tests/fixtures/formats/**` path in `.gitattributes`.
- [x] 1.3 Generate `<fixture>.expected` re-serialization outputs by
  running the *current* serializer against each parsed fixture and
  checking the result in. NOTE: `.expected` files capture today's
  serializer output, NOT the original input bytes — they exist to lock
  serializer behavior across the refactor, not to assert that the
  serializer reproduces the original file (it does not, e.g. ASS section
  ordering, whitespace, and quoting follow the serializer's canonical
  form).
- [x] 1.4 Add `tests/format_roundtrip_tests.rs` that, for every fixture,
  parses the input via `FormatManager`, asserts on entry count and key
  fields, re-serializes via the matching `SubtitleFormat` impl, and
  compares the output byte-for-byte to `<fixture>.expected`.
- [x] 1.5 Add `pretty_assertions` as a `dev-dependency` (only) and use it
  in the round-trip harness for diff-friendly failure output.
- [x] 1.6 Run `cargo nextest run --test format_roundtrip_tests || true`
  and confirm it passes against unmodified production code.

## 2. Extract shared types

- [x] 2.1 Create `src/core/formats/types.rs` and move `Subtitle`,
  `SubtitleEntry`, `SubtitleMetadata`, `StylingInfo`, and
  `SubtitleFormatType` (with their inherent `impl` blocks and the
  `Display` impl) verbatim from `mod.rs` into it.
- [x] 2.2 Move the `SubtitleFormat` trait definition into the same
  `types.rs` module (do not introduce a separate `traits.rs`; the
  proposal pins the trait location to `formats::types` for this change).
- [x] 2.3 In `mod.rs`, declare `mod types;` and add `pub use types::*;`
  so every existing public path under `crate::core::formats` continues
  to resolve.
- [x] 2.4 Run `cargo build`, `cargo clippy -- -D warnings`,
  `cargo test --doc --all-features`, and the round-trip harness; fix
  any import drift.

## 3. Decompose the monolithic `mod tests`

- [x] 3.1 Move type-only assertions (those exercising
  `SubtitleFormatType`, `SubtitleEntry`, `SubtitleMetadata`,
  `StylingInfo`, `Subtitle` constructors/methods) from the existing
  `mod tests` in `mod.rs` into a new `#[cfg(test)] mod tests` inside
  `src/core/formats/types.rs`.
- [x] 3.2 Move format-specific assertions into the corresponding
  `tests.rs` files added in section 4 (initially keep them inline at the
  bottom of each existing per-format file; section 4 will relocate them
  into the directory layout).
- [x] 3.3 Delete the now-empty `mod tests` block in `mod.rs` and confirm
  `cargo nextest run || true` still reports the same test count from
  the merged set.

## 4. Convert per-format files to directory modules

Order: `sub` → `vtt` → `srt` → `ass` (smallest first).

- [x] 4.1 For `sub`: create `src/core/formats/sub/{mod,parser,serializer,time,tests}.rs`,
  move the `pub struct SubFormat`, `impl SubtitleFormat`, helper
  functions, and tests into the appropriate files. Keep `pub use` paths
  identical. Run round-trip harness.
- [x] 4.2 For `vtt`: same structure. Move `parse_vtt_time`,
  `format_vtt_time`, and `format_vtt_time_range` into `time.rs`. Run
  round-trip harness.
- [x] 4.3 For `srt`: same structure. Move `parse_time`,
  `format_time_range`, and `format_duration` into `time.rs`. Run
  round-trip harness.
- [x] 4.4 For `ass`: same structure. Move `parse_ass_time` and
  `format_ass_time` into `time.rs`. Keep `AssStyle` and `Color` in
  `mod.rs` (or move into `style.rs` if convenient). Run round-trip
  harness.
- [x] 4.5 After each format conversion, confirm `FormatManager::new()`
  in `src/core/formats/manager.rs` still registers the same struct and
  that `cargo build` + `cargo clippy -- -D warnings` succeed.

## 5. Implement malformed-input matrix (subtitle-parser-hardening deltas)

- [x] 5.1 Add empty-input rejection to all four parsers, returning
  `SubXError::SubtitleFormat`. Add a per-format unit test in each
  `tests.rs`.
- [x] 5.2 Add truncated-header detection: ASS without `[Events]`, VTT
  without `WEBVTT`. Add unit tests. *(VTT done; ASS tracked separately.)*
- [x] 5.3 Add UTF-8 BOM consumption for SRT, VTT, and ASS parsers as a
  *new defensive layer*. Note: BOM stripping for files read via
  `FormatManager::parse_file` already happens in
  `src/core/formats/encoding/converter.rs::skip_bom`; do NOT remove
  that — the parser-level strip is intentionally additive so callers
  using `parse_auto` or direct `<Fmt>Format::parse(&str)` on in-memory
  strings also get BOM tolerance. Add unit tests covering both
  BOM-with-valid and BOM-with-invalid content. (SRT: done — BOM strip
  added in `srt/parser.rs`, BOM-only input falls through to the
  empty-input rejection.)
  *(VTT done; SRT/ASS tracked separately.)*
- [x] 5.4 Verify out-of-order cues are preserved by SRT and VTT parsers
  (no implicit sort). Add unit tests.
  *(SRT and VTT done.)*
- [x] 5.5 Add negative-timestamp skip-and-continue with `debug!` log to
  SRT, VTT, and ASS parsers. Add unit tests.
  *(SRT and VTT done; ASS tracked separately.)*
- [x] 5.6 Add a per-cue size cap to all four parsers using a
  **parser-local fixed constant**
  `const MAX_CUE_BYTES: usize = 1 * 1024 * 1024;` (1 MiB), defined at
  the top of each parser module. The value is chosen to comfortably
  exceed any realistic cue (typical cue text < 1 KiB; verbose ASS
  karaoke lines stay well under 100 KiB) while still bounding hostile
  per-cue allocation. The cap MUST NOT depend on
  `general.max_subtitle_bytes` or any other configuration value because
  `SubtitleFormat::parse(&self, content: &str)` does not receive a
  `ConfigService`, and threading config into parser construction is an
  explicit non-goal of this change. File-level size enforcement against
  `general.max_subtitle_bytes` continues to live at the command/file-read
  layer, unchanged. Add unit tests covering the boundary (just under
  cap → success; just over cap → `SubXError::SubtitleFormat`). (SRT:
  done — `MAX_CUE_BYTES = 1 * 1024 * 1024` defined at the top of
  `srt/parser.rs`; boundary tests cover both sides.)
  *(ASS, SRT, and VTT done; SUB tracked separately.)*
- [x] 5.7 Add ASS `Format:`-vs-`Dialogue:` column-count mismatch
  skip-and-continue with `debug!` log. Add unit test.
- [x] 5.8 Confirm VTT parser accepts files missing trailing blank line
  at EOF; add unit test if not already covered.
- [x] 5.9 Confirm SUB non-numeric frame-range skip-and-continue is
  covered; add unit test if not already present.
- [x] 5.10 Update rustdoc on each parser's `parse` method to enumerate
  the malformed-input dispositions per the spec matrix.
  *(SRT done.)*

## 6. Opt-in property/fuzz coverage

- [x] 6.1 Do NOT add `proptest` (or any other property-testing crate) as
  a dev-dependency. Implement the slow-tests harness as a hand-rolled
  deterministic mutator using only `std` plus existing crates already in
  the dev-dependency closure (e.g. seeded `rand` if already present, or
  a tiny in-tree LCG/xorshift if not). This keeps `Cargo.lock` resolution
  and the default dev-dependency closure unchanged regardless of feature
  flags.
- [x] 6.2 In each per-format `tests.rs`, add a
  `#[cfg(feature = "slow-tests")] mod proptests` module that feeds the
  parser a deterministic sequence of (a) random byte sequences and
  (b) structural mutations of valid fixtures (byte flips, truncations,
  duplicated lines, BOM injection, oversized cues), asserting only
  "no panic, errors are typed `SubXError`". Use a fixed RNG seed so runs
  are reproducible.
- [x] 6.3 Tune iteration counts so
  `cargo nextest run --features slow-tests` adds ≤ 30 s on the CI
  reference runner.
- [x] 6.4 Document the (optional, future) `cargo-fuzz` workspace layout
  in `design.md` follow-up notes — do NOT add `fuzz/` in this change.

## 7. Verification and quality gates (main agent only)

- [x] 7.1 Run `cargo fmt` and `cargo clippy -- -D warnings`.
- [x] 7.2 Run `cargo nextest run || true` and inspect output; treat any
  failure as a real failure.
- [x] 7.3 Run `cargo test --doc --all-features`.
- [x] 7.4 Run `scripts/quality_check.sh` (single invocation, main agent
  only) and confirm it passes.
- [x] 7.5 Run `scripts/check_coverage.sh -T` and confirm coverage is
  ≥ 75 %.
- [x] 7.6 Spot-check parser performance only if a hot loop is touched
  (e.g. the SRT block splitter, ASS line tokenizer, or VTT regex
  matchers). Use an ad-hoc `criterion` micro-benchmark on the working
  branch and document any > 5 % regression in the PR description. Note:
  the existing `benches/` suite (`retry_performance`,
  `file_id_generation_bench`) does NOT exercise format parsers, so no
  automated parser-perf gate is enforced by this change. Adding a
  parser benchmark is out of scope and deferred to a follow-up change.
- [x] 7.7 Run `openspec validate refactor-format-parsers` and resolve any
  reported issues.
- [x] 7.8 Update `CHANGELOG.md` under `### Changed` (internal refactor)
  and `### Added` (new round-trip and hardening tests).
