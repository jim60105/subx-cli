## Context

`src/core/formats/mod.rs` is currently ~1,800 lines and combines the public
data model (`Subtitle`, `SubtitleEntry`, `SubtitleMetadata`, `StylingInfo`,
`SubtitleFormatType`), the `SubtitleFormat` trait contract, an extensive
rustdoc preamble, and a single ~650-line `#[cfg(test)] mod tests` block. The
per-format implementations already live in dedicated files (`srt.rs`,
`ass.rs`, `vtt.rs`, `sub.rs`) and are wired up through
`FormatManager::new()` in `src/core/formats/manager.rs`, but each of them
mixes parsing, serialization, time-helpers, and (sparse, inconsistent)
inline tests.

The existing `subtitle-parser-hardening` capability documents only four
narrowly-scoped malformed-input requirements (ASS missing fields, ASS
timestamp overflow, SRT skip-bad-block, SUB 24 h frame guard). Real-world
subtitle corpora include many more failure modes, and `format-conversion`
currently makes no explicit promise that parser/serializer round-trips are
byte-stable, even though the convert command depends on it.

Stakeholders: maintainers of subtitle conversion, downstream consumers
relying on `crate::core::formats` re-exports (translation, sync, matcher),
and CI which already runs under a tight budget via `scripts/quality_check.sh`
and the 75 % coverage gate enforced by `scripts/check_coverage.sh`.

## Goals / Non-Goals

**Goals:**

- Reduce the cognitive footprint of `src/core/formats/mod.rs` to a thin
  facade (re-exports + module declarations + crate-level rustdoc).
- Co-locate each format's parser, serializer, time helpers, and tests in a
  dedicated submodule directory so contributors only need to read one
  directory to understand a format.
- Establish a golden-file regression corpus that proves byte-equivalent
  parser and serializer behavior before and after the refactor.
- Extend `subtitle-parser-hardening` with an explicit malformed-input matrix
  so contributors know which scenarios MUST return typed errors vs. which
  MUST skip-and-continue.
- Provide an opt-in property/fuzz strategy that does not destabilize CI.

**Non-Goals:**

- No changes to public API names, the `SubtitleFormat` trait shape, the
  `FormatManager` constructor surface, configuration keys, CLI flags, exit
  codes, or on-disk output bytes for valid inputs.
- No new runtime dependency on the default build.
- No introduction of a new subtitle format.
- No changes to encoding-detection, styling, conversion, or sync logic
  beyond what is required to keep them compiling against the reorganized
  module tree.
- No mandatory fuzz/CI step; fuzz harness is optional and lives outside the
  default workspace.

## Decisions

### Decision 1: Move shared types **and the `SubtitleFormat` trait** to `formats::types`, keep `mod.rs` as a facade

`mod.rs` becomes a small module that:

1. Carries the existing crate-level rustdoc preamble.
2. Declares submodules (`pub mod ass; pub mod srt; pub mod sub; pub mod vtt;
   pub mod manager; pub mod converter; pub mod styling; pub mod transformers;
   pub mod encoding; mod types;`).
3. Re-exports the shared types **and the `SubtitleFormat` trait** so
   external call sites (`use crate::core::formats::Subtitle;`,
   `use crate::core::formats::SubtitleFormat;`) continue to work
   unchanged.

`SubtitleFormat` is intentionally kept in `formats::types` rather than a
separate `formats::traits` module to avoid the inconsistency between
proposal/tasks/design that earlier drafts had. `formats::types` is the
single canonical location for the trait throughout this change.

**Alternatives considered:**

- *Promote each type to its own file (`subtitle.rs`, `entry.rs`, ...).*
  Rejected: too granular, introduces noise without aiding navigation —
  these types are tightly coupled.
- *Leave types in `mod.rs` and only extract tests.* Rejected: does not
  reduce file size enough and keeps the awkward "trait + types + giant
  test block" mix that motivated this change.

### Decision 2: Per-format directory layout

Each of `srt`, `ass`, `vtt`, `sub` becomes a directory module:

```
src/core/formats/<fmt>/
  mod.rs          // declares the *Format unit struct + impl SubtitleFormat
  parser.rs       // pure parsing helpers (no I/O)
  serializer.rs   // pure serialization helpers (no I/O)
  time.rs         // timestamp parsing/formatting helpers
  tests.rs        // #[cfg(test)] inline tests for that format
```

`mod.rs` for each format remains the only public entry point and keeps the
`pub struct <Fmt>Format;` and `impl SubtitleFormat for <Fmt>Format` blocks
so that `FormatManager::new()` registration is byte-identical.

**Alternatives considered:**

- *Keep single-file modules and only add per-file tests modules.* Rejected
  for ASS specifically (310 lines plus future hardening additions push it
  past comfortable single-file size); applying the directory layout
  uniformly avoids special cases.
- *Introduce a `formats::common` helper crate-internal module for shared
  time helpers.* Deferred: the time formats are sufficiently divergent
  (`HH:MM:SS,mmm` vs `H:MM:SS.cc` vs `HH:MM:SS.mmm` vs frame numbers) that a
  shared module would mostly be a thin dispatch.

### Decision 3: Golden-file round-trip harness

Add a corpus under `tests/fixtures/formats/<format>/` containing canonical
"happy path" inputs (ASCII, UTF-8 with BOM, multi-line cues, styled cues
where applicable). A new top-level integration test
`tests/format_roundtrip_tests.rs` parses each fixture, asserts on the
resulting `Subtitle` structure, re-serializes via the same `SubtitleFormat`
impl, and compares the output to a `<fixture>.expected` file.

**Important clarification on what "round-trip stable" means here:** the
`.expected` files are produced by running the *current* serializer over
the parsed fixture and committing whatever bytes that serializer emits.
They do **not** assert that serializing reproduces the original input —
in particular, the ASS serializer canonicalizes `[Script Info]`,
`[V4+ Styles]`, and `[Events]` ordering, normalizes whitespace, and may
reformat field order; the SRT/VTT/SUB serializers similarly emit a
canonical form rather than a faithful copy of the input. The harness
therefore locks **serializer output stability across the refactor**, not
input-byte preservation. Any intentional serializer change after this
refactor must be accompanied by a deliberate `.expected` update, which is
the explicit signal we want.

Rationale: byte-stable round-trips of the *serializer output* are the
only practical proof that the refactor preserves behavior. The fixtures
double as documentation of canonical inputs each format accepts.

**Alternative considered:** Snapshot crates such as `insta`. Rejected to
avoid a new dependency; hand-rolled `pretty_assertions::assert_eq` against
checked-in `.expected` files is sufficient and keeps diff review trivial.

### Decision 4: Malformed-input matrix lives in `subtitle-parser-hardening`

Rather than scattering hardening rules across format-conversion and
parser-hardening, this change extends only `subtitle-parser-hardening`.
Each new requirement specifies the disposition (skip-and-continue with
`debug!` log vs. typed `SubXError::SubtitleFormat` return) so test authors
have an unambiguous contract.

### Decision 5: Property/fuzz coverage is opt-in **and dependency-free**

A property test module gated behind the existing `slow-tests` cargo feature
will exercise random byte sequences and structured mutations against each
parser, asserting only that no panic occurs and that any returned error is
a `SubXError` variant. The default `cargo nextest run` and
`scripts/quality_check.sh` invocations remain unaffected.

The mutator is **hand-rolled** using a small deterministic PRNG (seeded
xorshift/LCG) — we deliberately do **not** add `proptest`, `quickcheck`,
or any other property-testing crate as a dev-dependency. Even
feature-gated dev-dependencies can perturb `Cargo.lock` resolution and
the offline dev-dependency closure, which we want to avoid for a
refactor-only change. Inputs are generated as (a) random byte sequences
of bounded length and (b) structural mutations (byte flip, truncation,
duplicated lines, BOM injection, oversized cue body) of the existing
golden fixtures.

The malformed-input matrix in `subtitle-parser-hardening` includes a
per-cue size cap. That cap is a **parser-local fixed constant** in each
parser module, defined as `const MAX_CUE_BYTES: usize = 1 * 1024 * 1024;`
(1 MiB). The value is chosen to be larger than any realistic single cue
encountered in the wild (typical cue text is < 1 KiB; even verbose
karaoke `{\k...}` ASS lines stay well under 100 KiB) while still
bounding the per-cue allocation under hostile input. It deliberately
does NOT consult `general.max_subtitle_bytes` or any other configuration
value because `SubtitleFormat::parse(&self, content: &str)` does not
receive a `ConfigService`, and threading config into parser construction
is an explicit non-goal of this change. File-level enforcement of
`general.max_subtitle_bytes` continues to live at the command/file-read
layer and is unchanged by this refactor. If a future, real-world
subtitle file exceeds 1 MiB in a single cue, we will revisit the
constant in a separate change rather than make it configurable here.

If we later add `cargo-fuzz`, it lives in a sibling `fuzz/` workspace that
is excluded from the main `Cargo.toml` workspace members so it never builds
during normal development or CI. **Note:** the "no new dependency" claim
in this change applies specifically to the default workspace's runtime
*and* dev-dependency closure; `cargo-fuzz` and any future fuzz-only crates
would live exclusively under that out-of-tree `fuzz/` workspace.

**Rationale:** the project's coverage and runtime budget are tight, and CI
already orchestrates three platforms. Property/fuzz coverage is most
valuable locally and in dedicated security audits.

### Decision 6: Parser-level BOM consumption is a new defensive layer, not behavior preservation

BOM stripping for files read through `FormatManager::parse_file` already
happens in `src/core/formats/encoding/converter.rs::skip_bom` *before*
the `&str` content is handed to any `SubtitleFormat::parse` impl. Today,
callers that bypass `parse_file` — most notably `FormatManager::parse_auto`
called on in-memory strings, plus direct `<Fmt>Format::parse(&str)`
callers in tests and translation/sync code paths — receive content with
a UTF-8 BOM still embedded if one was present. The hardening matrix in
this change adds parser-level BOM consumption (SRT, VTT, ASS) as a
deliberate *new defensive layer*. This is not a "preserve current
behavior" requirement; it is a net-new behavior introduced by the
hardening matrix. The double-strip when going through `parse_file`
(encoding layer strips, then parser tolerates absence) is harmless and
intentional. Reviewers MUST NOT remove the encoding-layer
`skip_bom` as part of this refactor — both layers coexist by design.

### Decision 7: Preserve pre-existing parser quirks; lock them in fixtures

The current SRT and VTT parsers split blocks with the literal pattern
`"\n\n"`. They do not use a CRLF-aware regex such as `\r?\n\r?\n`. The
empirical CRLF behavior of these two parsers is asymmetric and, for
SRT/VTT, partially broken — it must NOT be "fixed" in this refactor:

- **SRT (CRLF)**: `"\n\n"` is **not** a substring of `"\r\n\r\n"`
  (the bytes are `0d 0a 0d 0a`, with no two consecutive `\n`). A CRLF
  SRT file therefore yields a single block whose `lines()` iteration
  recognizes only the first index/timing line; the remaining cues are
  absorbed into the first entry's text payload. The serializer's
  multi-line text emission then happens to produce byte-stable round-trip
  output because the embedded payload re-creates cue structure when
  re-parsed on LF. The in-memory `Subtitle.entries.len()` from a CRLF
  SRT input is *smaller* than its LF counterpart.
- **VTT (CRLF)**: the same splitter behavior plus the trailing `\r` on
  cue marker lines causes the cue time regex to fail; the `WEBVTT`
  header check still passes, so a CRLF VTT file parses to a `Subtitle`
  with **zero** entries.
- **ASS (CRLF)**: parsing is line-based via `str::lines()` (which
  absorbs trailing `\r`), so CRLF ASS files parse identically to LF.
- **SUB (CRLF)**: also line-based and CRLF-tolerant.

This is fragile but is the contract today. Because the refactor must be
byte-equivalent on the golden corpus, fixtures MUST include at least
one CRLF input per format (`*.crlf.<ext>`) so any future change to
either splitter is caught immediately by `.expected` byte equality.
The fixtures must NOT silently LF-normalize on commit — the round-trip
harness reads them as raw bytes. Improving CRLF handling for SRT/VTT
is deferred to a follow-up change so this refactor remains strictly
behavior-preserving.

### Decision 8: Bench coverage is acknowledged as a gap

The current `benches/` directory contains `retry_performance` and
`file_id_generation_bench`. Neither exercises subtitle parsers. We
therefore do NOT promise a "no benchmark regression" guarantee in
tasks.md and we do NOT add a parser benchmark in this change. Adding a
parser benchmark is deferred to a follow-up change so this refactor
stays narrowly scoped. If the implementer touches a hot loop (e.g. the
SRT block splitter or ASS line tokenizer), they SHOULD spot-check
performance with an ad-hoc `criterion` micro-benchmark on a branch and
note any > 5 % regression in the PR description, but no automated gate
enforces this here.

## Risks / Trade-offs

- **Risk: Serializer drift produces byte-different output.** → Mitigation:
  golden fixtures with `.expected` files compared verbatim; the refactor is
  staged so tests are added before code moves (see Migration Plan).
- **Risk: Reduced inline coverage during the move.** → Mitigation: each
  per-format `tests.rs` is created by *moving* assertions out of the
  current monolithic `mod tests`, not rewriting them; `scripts/check_coverage.sh`
  must still pass at ≥ 75 %.
- **Risk: External call sites that imported types from a now-removed
  in-`mod.rs` location break.** → Mitigation: every public item is
  `pub use`-re-exported from `crate::core::formats`, matching today's
  public paths exactly. `cargo build` + `cargo clippy -- -D warnings` will
  catch any drift.
- **Risk: Property/fuzz tests slow down CI if accidentally enabled.** →
  Mitigation: gated behind `slow-tests`; default `cargo nextest run` is the
  validated invocation. `scripts/quality_check.sh -v -p ci --full` already
  enables `slow-tests` intentionally — we will tune iteration counts so the
  added budget is bounded (target: ≤ 30 s of additional CI time).
- **Risk: Fuzz harness in a separate workspace bit-rots.** → Mitigation:
  document its layout in `design.md` here and add a maintainer note in the
  `fuzz/README.md` (created in a follow-up change, not this one).
- **Trade-off: Directory-per-format layout adds files.** Accepted because
  the navigation/discoverability gain outweighs file count.

## Migration Plan

Each step below is independently reviewable and leaves the tree green
(`cargo build`, `cargo clippy -- -D warnings`, `cargo nextest run || true`,
`cargo test --doc --all-features`).

1. **Add the regression harness first** — introduce
   `tests/fixtures/formats/<format>/` with ≥ 1 canonical input and
   `.expected` output per format, plus
   `tests/format_roundtrip_tests.rs`. This must pass against the *current*
   code, locking in today's behavior as the baseline.
2. **Extract shared types** — create `src/core/formats/types.rs` and move
   `Subtitle`, `SubtitleEntry`, `SubtitleMetadata`, `StylingInfo`,
   `SubtitleFormatType`, and the `SubtitleFormat` trait. Update `mod.rs` to
   `pub use` them. Run regression harness.
3. **Decompose tests in `mod.rs`** — move type-level tests into
   `formats::types::tests`. No assertion changes. Run regression harness.
4. **Convert each per-format file to a directory** — one format per commit
   in this order: `sub` (smallest), `vtt`, `srt`, `ass`. For each format:
   create `formats/<fmt>/{mod,parser,serializer,time,tests}.rs`, move code
   verbatim, ensure `FormatManager::new()` still registers the same struct.
   Run regression harness after each.
5. **Apply hardening spec deltas** — implement new malformed-input
   requirements behind their per-format `parser.rs` and add corresponding
   `tests.rs` cases. Update `format-conversion` round-trip stability test.
6. **Add opt-in property tests** — under `#[cfg(feature = "slow-tests")]`
   in each `tests.rs`, exercise the parsers with structured mutators.
7. **Run `scripts/quality_check.sh`** once at the end (main agent only,
   per AGENTS.md), confirm 75 % coverage gate via
   `scripts/check_coverage.sh -T`, and verify benches in `benches/` show no
   regression > 5 % on representative inputs.

**Rollback strategy:** every step is a self-contained commit on a feature
branch; a problematic step can be reverted without unwinding earlier ones,
because earlier steps preserve the public surface and golden-file
guarantees.

## Open Questions

- Should the golden fixtures live under `tests/fixtures/formats/` (new) or
  reuse `assets/` (currently for media samples)? Recommendation:
  `tests/fixtures/formats/` to keep test-only data near tests.
- Should we migrate `transformers.rs` (807 LOC) into per-format
  submodules at the same time? Recommendation: no — that file is about
  cross-format transformations and is out of scope for this change.
- Do we want `pretty_assertions` as a dev-dependency for clearer diff
  output in the round-trip harness? Recommendation: yes, dev-only; it is
  already widely used in the Rust ecosystem and keeps fixture failures
  diagnosable without bloating the runtime build.

## Fuzz workspace layout (deferred)

A `cargo-fuzz` harness is **not** added by this change. If introduced
later, it would live at `<repo>/fuzz/` as a sibling crate that is
explicitly excluded from the main `Cargo.toml` workspace members (so
`cargo build` at the workspace root never resolves any fuzz-only
dependency into the default `Cargo.lock`). The fuzz crate would import
the parsers via a path dependency on `subx-cli` and define one
`fuzz_target!` per format (`fuzz_targets/srt.rs`, `vtt.rs`, `ass.rs`,
`sub.rs`), each calling the corresponding `<Fmt>Format::parse(&str)`
and asserting only "no panic; errors are typed `SubXError`". The
harness is explicitly NOT part of CI, `scripts/quality_check.sh`, or
release builds — it is an opt-in local tool for security-style
exploration only.
