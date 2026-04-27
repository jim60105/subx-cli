## Context

SubX-CLI currently uses three distinct identifier schemes:

1. `src/core/matcher/discovery.rs::generate_file_id` — `DefaultHasher`
   over `(canonicalized path, file size)` formatted as
   `file_<16 hex chars>` (length 21).
2. `src/core/parallel/{worker,scheduler}.rs` — `Uuid::new_v4()` for
   worker and task identifiers (random UUIDv4).
3. `src/core/translation/uuidv7.rs::CueIdGenerator` — UUIDv7 with strict
   1ms spacing.

This fragmentation creates three problems:

- The hash-based ID is opaque and not time-sortable; engineers reading
  logs cannot tell which file was discovered first.
- UUIDv4 worker IDs are also not time-sortable, so post-mortem log
  ordering of worker spans relies on system timestamps rather than the
  ID itself.
- The `Cargo.toml` pulls in both the `v4` and `v7` features of the
  `uuid` crate, doubling the surface area of the dependency.

In parallel, the JSON output mode introduced by the
`machine-readable-output` capability requires that JSON-mode stdout
contain exactly one envelope and that stderr be free of status emojis,
ANSI codes, and progress-bar frames. The `match` command's
`match_file_list_with_audit` violates the spirit of this contract by
emitting a free-form `🔍 AI Analysis Results:` debug block via
`eprintln!` regardless of the active output mode. The user's bug report
shows the block appearing in their terminal session alongside the JSON
envelope and asks that JSON mode emit nothing other than the envelope.

## Goals / Non-Goals

**Goals:**

- Unify all in-process identifier generation on UUIDv7 with strict 1ms
  spacing through a single shared generator.
- Reduce the `uuid` dependency surface area by dropping the `v4`
  feature.
- Guarantee that `--output json` (and `SUBX_OUTPUT=json`) produce no
  free-form `eprintln!`/`println!` chatter from any subcommand,
  closing the gap demonstrated by the `match` command's AI-debug
  block.
- Tighten the `machine-readable-output` capability spec so future code
  cannot regress this property.

**Non-Goals:**

- Replacing structured `tracing`/`log` output. Log records gated by
  `RUST_LOG` are out of scope; users opt into them explicitly and they
  do not violate the JSON-mode contract.
- Introducing a new schema-version bump for the JSON envelope. The
  envelope shape is unchanged; only stderr discipline tightens.
- Persisting file IDs across invocations. UUIDv7 is intentionally
  non-deterministic — the cache layer correlates files by canonical
  path, not by ID, and that contract is preserved.
- Replacing the `subtitle-translation` cue-ID generator's behavior; the
  generator is relocated to a crate-wide module but its semantics are
  unchanged.

## Decisions

### Decision: Relocate the UUIDv7 generator to `src/core/uuidv7.rs`

The existing implementation in `src/core/translation/uuidv7.rs` already
delivers exactly the behavior the matcher needs (UUIDv7, strict 1ms
spacing via timestamp-aware sleep, deterministic monotonic ordering).
Rather than duplicate the implementation, move the file to
`src/core/uuidv7.rs`, expose it as `pub mod uuidv7` from `src/core/mod.rs`,
and re-export `CueIdGenerator`/`generate_cue_ids`/`unix_time_ms` from
`src/core/translation/mod.rs` so existing translation call sites compile
unchanged.

**Alternatives considered:**

- *Inline a second copy in the matcher.* Rejected — duplicates the 1ms
  spin-loop logic and the unit tests, raising the risk that the two
  implementations drift.
- *Publish the generator as a separate internal crate.* Rejected —
  overkill for two call sites in the same workspace; we would still
  need a shared module location.

### Decision: Rename the generator type to `Uuidv7Generator`

`CueIdGenerator` is a translation-domain name. After relocation, the
struct is shared by media discovery and translation, so the type name
`Uuidv7Generator` better describes its responsibility. `CueIdGenerator`
is retained as a deprecated type alias inside the translation module
for one release cycle, but the AGENTS.md rule forbids new
`#[deprecated]` attributes — therefore the alias is implemented as a
plain `pub use crate::core::uuidv7::Uuidv7Generator as CueIdGenerator;`
re-export with no attribute, which is functionally equivalent for our
internal call sites.

### Decision: Change `generate_file_id` to consume a shared generator

The current signature
`fn generate_file_id(path: &Path, file_size: u64) -> String` cannot
return monotonically increasing IDs because each call constructs its
own hash. The new signature is

```rust
pub fn generate_file_id(generator: &mut Uuidv7Generator) -> String;
```

`FileDiscovery::scan_directory` and `scan_file_list` instantiate one
`Uuidv7Generator` at the start of the scan and thread a `&mut`
reference through every classification call so that all video and
subtitle IDs from a single scan share a strictly increasing
`unix_time_ts`.

**Alternatives considered:**

- *Make the generator a hidden `thread_local!`*. Rejected because tests
  in `tests/` exercise multiple discoveries within a single test
  process and need fresh, deterministic ordering within each scan.
  An explicit `&mut` reference is also easier to reason about and to
  test.
- *Keep the function signature and use `Uuid::now_v7()` without 1ms
  spacing.* Rejected because the user explicitly requested 1ms spacing
  to make `unix_time_ts` strictly monotonic across the whole batch.

### Decision: Format `file_<uuid-v7>` (length 41) with `Uuid::Hyphenated`

UUIDv7 is rendered in canonical hyphenated form (8-4-4-4-12 = 36
characters). Prefixing with `file_` produces a 41-character identifier
that is visually distinct from raw UUIDs in logs, follows the existing
convention, and is trivially recognizable to humans skimming AI
prompts.

**Alternatives considered:**

- *Bare UUIDv7 (length 36).* Rejected — drops the `file_` prefix that
  AI prompts already rely on for disambiguation from `worker_<uuid>`
  and `task_<uuid>` fields.
- *Bare hex (length 32, no hyphens).* Rejected — forfeits the standard
  UUIDv7 textual form that downstream tooling expects.

### Decision: Gate every free-form `*println!` in `match` engine on `is_json()`

The matcher engine already gates one `println!` block on
`crate::cli::output::active_mode().is_json() == false` at
`src/core/matcher/engine.rs:917`. Apply the same guard to:

- The `🔍 AI Analysis Results:` block (lines ~747–758).
- The post-loop `eprintln!` at lines ~813–815 that echoes rejected
  matches.
- Any other `eprintln!`/`println!` that escaped into the matcher
  during prior development. A clippy lint
  (`#[deny(clippy::print_stderr, clippy::print_stdout)]`) is NOT
  introduced because doing so would conflict with intentional CLI
  output in `cli::ui`. Instead, a unit test asserts that JSON-mode
  stderr is free of the `🔍` byte sequence.

**Alternatives considered:**

- *Convert the debug lines to `tracing::debug!`.* Considered but
  rejected because the existing block is intentionally informational
  for end users in text mode (the threshold/match summary is useful
  diagnostic context). Gating on `is_json()` preserves the text-mode
  UX with zero cost.
- *Delete the block entirely.* Rejected — useful in text mode for
  diagnosing low-confidence matches.

### Decision: Tighten `machine-readable-output` stderr discipline

The current spec says:

> Stderr MAY contain free-form diagnostic logs (for example, `tracing`
> output) but SHALL NOT contain any JSON envelope, `✓`/`✗`/`⚠` status
> symbols, ANSI color escape sequences emitted by
> `print_success`/`print_warning`/`print_error`, or `indicatif`
> progress-bar frames.

This permits ad-hoc `eprintln!` calls — which the user reports as a
bug. The spec is updated to explicitly forbid ad-hoc `eprintln!` /
`println!` chatter on either stream, leaving only `tracing`/`log`
output (subject to the user's `RUST_LOG` filter) as legitimate stderr
content.

## Risks / Trade-offs

- **[Risk]** Removing the `(path, size)` deterministic ID property may
  surprise external scripts that grep for a specific
  `file_<hex>` pattern in logs. → **Mitigation**: The current ID is
  scoped to AI request/response correlation within a single invocation;
  it is not documented as a stable cross-invocation identifier and is
  not surfaced in the JSON envelope. Release notes call out the new
  shape (`file_<uuid-v7>`, length 41).
- **[Risk]** The 1ms spin-loop adds latency to scans of very large
  directories. → **Mitigation**: For 10 000 files the worst-case
  cumulative wait is 10 s, but in practice each `Uuid::now_v7()` call
  takes ~microseconds and the same-millisecond branch is rare. The
  matcher already takes seconds to scan typical directories, so the
  overhead is dominated by I/O. If profiling later shows the loop is a
  hotspot, the generator can be upgraded to use a per-millisecond
  counter (UUIDv7 supports up to 4096 IDs per millisecond) without a
  spec change.
- **[Risk]** Gating the AI Analysis Results block on `is_json()` could
  hide useful debug information from users running scripts in JSON mode
  who want to understand why a candidate was rejected. → **Mitigation**:
  The JSON envelope's `data.candidates[i].accepted` and
  `data.candidates[i].reason` fields already carry the same information
  in machine-readable form; the human-oriented eprintln duplicates that
  payload.
- **[Risk]** Shared `Uuidv7Generator` instances across nested async
  contexts could accidentally serialize too much work. → **Mitigation**:
  Discovery is single-threaded by design; the generator is held by
  value inside `FileDiscovery::scan_directory` and never crosses an
  `await` point that yields to other tasks. The matcher's worker pool
  uses its own per-call generator instance.

## Migration Plan

1. **Phase 1 — Move and rename the generator.** Relocate
   `src/core/translation/uuidv7.rs` to `src/core/uuidv7.rs`, rename
   `CueIdGenerator` → `Uuidv7Generator`, add a re-export alias from
   `src/core/translation/mod.rs`, and run the existing translation
   tests unchanged.
2. **Phase 2 — Adopt UUIDv7 in matcher discovery.** Change
   `generate_file_id` and thread a generator through `FileDiscovery`.
   Update unit tests in `discovery.rs::tests` and the matcher
   integration tests.
3. **Phase 3 — Adopt UUIDv7 in parallel processing.** Switch
   `Uuid::new_v4()` call sites to `Uuid::now_v7()`. The worker pool's
   per-execute call rate is low; a per-call `Uuid::now_v7()` (without a
   shared spin-loop generator) is sufficient because worker IDs do not
   need to be monotonically ordered relative to each other within the
   same millisecond.
4. **Phase 4 — Drop the `v4` Cargo feature.** Edit `Cargo.toml`,
   confirm `cargo build` succeeds, and run the full quality check.
5. **Phase 5 — Gate the matcher debug eprintlns.** Wrap the
   `🔍 AI Analysis Results:` block and any sibling `*println!` calls
   under `is_json()` and add a regression test that runs the match
   command via `MockOpenAITestHelper` with `--output json` and asserts
   stderr is empty of those byte sequences.
6. **Phase 6 — Update the spec.** Land the modified
   `machine-readable-output` requirement and the new `media-discovery`
   / `subtitle-matching` / `parallel-processing` requirements.

**Rollback strategy:** Each phase is a single commit on a feature
branch. Phases 2 and 3 are independently revertible. Phase 4 (the
manifest change) reverts trivially by re-adding `"v4"` to the features
list. The spec changes are versioned in `openspec/` and revertible by
restoring the previous archived change.

## Open Questions

- Should the `--output json match` integration test live under
  `tests/cli/output_json_*.rs` (alongside other JSON-mode harnesses) or
  under `tests/match_engine_id_integration_tests.rs`? → Default
  decision: place new file `tests/match_command_json_silence_test.rs`
  to keep the JSON discipline tests easy to locate.
- Are there other unaudited `eprintln!`/`println!` call sites in
  commands beyond `match`? → A repository-wide grep during Phase 5
  catches them; if found, gate them with the same `is_json()` guard
  rather than expanding the scope of this change.
