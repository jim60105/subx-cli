## Why

Two distinct quality gaps were observed in production:

1. The discovery-stage media file identifier (`generate_file_id` in
   `src/core/matcher/discovery.rs`) is a 64-bit hash of `(canonicalized path,
   file size)`. The hash provides intra-invocation uniqueness but is
   collision-prone, opaque, and not time-ordered, so it cannot be sorted by
   creation, cannot be inspected for relative ordering in logs, and bears
   no relationship to the UUIDv7 identifier scheme already used by the
   subtitle-translation pipeline (`src/core/translation/uuidv7.rs`).
   Worker IDs in `src/core/parallel/{worker,scheduler}.rs` still use
   `Uuid::new_v4()`, leaving the codebase split between three different ID
   strategies.
2. The `match` command emits a free-form debug block (`🔍 AI Analysis
   Results: ...`) via `eprintln!` from
   `src/core/matcher/engine.rs::match_file_list_with_audit` even when the
   user has explicitly opted into machine-readable output via `--output
   json` / `SUBX_OUTPUT=json`. This violates the user expectation that
   JSON mode produces no human-oriented chatter on either stream and
   forces downstream consumers to filter unstructured noise out of stderr.

Unifying the identifier scheme on UUIDv7 with strict 1ms spacing and
silencing all free-form human-oriented diagnostic chatter in JSON mode
fixes both regressions and removes the now-unused `uuid/v4` Cargo feature.

## What Changes

- **BREAKING (developer-facing log format)**: Replace the
  `file_<16-hex-chars>` (length 21) media file identifier with
  `file_<uuid-v7>` (length 41) generated through a shared, stateful
  generator that enforces strict 1ms spacing, mirroring
  `src/core/translation/uuidv7.rs::CueIdGenerator`.
  - Move the existing UUIDv7 generator to a crate-wide location
    (`src/core/uuidv7.rs`) so both the matcher and the translator depend
    on a single implementation; re-export it from
    `src/core/translation/mod.rs` for backward compatibility within the
    crate.
  - Update `generate_file_id` to consume the shared generator. The
    function signature changes from
    `generate_file_id(path: &Path, file_size: u64) -> String` to
    `generate_file_id(generator: &mut Uuidv7Generator) -> String`; the
    `path` and `file_size` arguments are dropped because UUIDv7 IDs are
    inherently unique without hashing inputs. All call sites in
    `src/core/matcher/{discovery,engine}.rs` are updated to thread a
    shared generator through the discovery scan so a single batch produces
    monotonically increasing IDs across both video and subtitle files.
- Switch `Uuid::new_v4()` to UUIDv7 in `src/core/parallel/worker.rs`
  (`WorkerPool::execute`, `Worker::new`) and in
  `src/core/parallel/scheduler.rs::CounterTask::task_id` so worker and
  task identifiers are also time-sortable.
- **BREAKING (Cargo manifest only)**: Remove the `"v4"` feature from the
  `uuid` dependency in `Cargo.toml` after eliminating the last
  `Uuid::new_v4()` call site. The `"v7"` feature is the only remaining
  feature flag.
- Suppress every free-form human-oriented diagnostic line that the
  `match` command writes outside of the JSON envelope when JSON output
  mode is active. Concretely:
  - Gate the `🔍 AI Analysis Results:` block in
    `src/core/matcher/engine.rs::match_file_list_with_audit` on
    `crate::cli::output::active_mode().is_json() == false`, matching the
    existing pattern at `engine.rs::execute_operations` line 917.
  - Gate the `Warning: Skipping relocation ...` and
    `Warning: Conflict resolution prompt not implemented ...` warnings
    in `engine.rs::resolve_filename_conflict` (lines ~1262 and ~1308)
    under the same `is_json()` guard so the live-execution path is also
    silent in JSON mode.
  - Audit and gate all other ad-hoc `eprintln!`/`println!` debug strings
    inside the matcher engine (`engine.rs` lines around 747–758,
    813–815, the AI debug echo path, and the warnings inside
    `resolve_filename_conflict`) under the same `is_json()` guard, so
    JSON mode never emits a status emoji or free-form prose on stderr.
  - Audit `src/core/parallel/worker.rs` (e.g., the worker error
    `eprintln!` around lines 129–134) and the rest of `src/commands/`
    for any unconditional `eprintln!`/`println!` calls reachable in
    JSON-mode execution paths and apply the same guard.
- Strengthen the `machine-readable-output` capability spec to require
  that every command's free-form `eprintln!`/`println!` debug chatter
  (status emojis such as `🔍`, AI provider response echoes,
  per-operation preview lines, conflict-resolution warnings) be
  suppressed in JSON mode, irrespective of the `--quiet` flag. Stderr
  in JSON mode is reserved for `tracing`/`log` output gated by the
  user's `RUST_LOG`/log-level configuration; ad-hoc `*println!` to
  stderr is forbidden in JSON mode.
- Update the `Quiet Flag` requirement of `machine-readable-output` so
  the documented stderr semantics agree with the new Stdout/Stderr
  Discipline rule: in JSON mode, free-form `eprintln!`/`println!`
  chatter is suppressed unconditionally; `--quiet` additionally
  silences `tracing`/`log` records.

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `media-discovery`: The "Deterministic File Identifier" requirement is
  replaced with a UUIDv7-based identifier requirement (length 41,
  `file_<uuid-v7>`, strictly increasing per invocation) and the
  associated scenarios are updated. The deterministic-id property and
  the "Different size produces a different id" / "Absolute path used for
  hashing" scenarios are removed.
- `subtitle-matching`: The "Stable File Identifiers for Matching"
  requirement updates the documented identifier shape to length 41 and
  the AI-correlation scenario is reworded so the contract pins the
  shared `Uuidv7Generator` rather than the hash function.
- `parallel-processing`: A new requirement is added pinning UUIDv7 (not
  v4) for worker and task identifiers.
- `machine-readable-output`: The "Stdout/Stderr Discipline in JSON Mode"
  requirement is tightened so ad-hoc `eprintln!`/`println!` human
  chatter on stderr is forbidden in JSON mode (only `tracing`/`log`
  records, gated by the user's log filter, may appear on stderr). A new
  scenario covers the `match` command specifically.


## Impact

- **Affected code**:
  - `src/core/matcher/discovery.rs` — `generate_file_id` signature
    change; tests under `mod tests` updated.
  - `src/core/matcher/engine.rs` — call sites for `generate_file_id`,
    addition of `is_json()` guards around the AI Analysis debug block
    and adjacent `eprintln!`s.
  - `src/core/translation/uuidv7.rs` — relocated to `src/core/uuidv7.rs`
    and re-exported from `src/core/translation/mod.rs`.
  - `src/core/parallel/worker.rs`, `src/core/parallel/scheduler.rs` —
    UUIDv7 adoption.
  - `src/core/translation/engine.rs` — import path update only.
  - `Cargo.toml` — remove `"v4"` from the `uuid` features list.
  - `src/cli/output.rs` — no API change (the existing
    `active_mode().is_json()` accessor stays the source of truth).
- **Tests**:
  - `src/core/matcher/discovery.rs::tests::test_deterministic_id_generation`
    is replaced with `test_uuidv7_id_generation` asserting UUIDv7 shape,
    strictly increasing `unix_time_ts`, and length 41 prefix.
  - `src/core/matcher/discovery.rs::tests::test_recursive_mode_with_unique_ids`
    keeps its uniqueness assertion but stops asserting deterministic
    output.
  - `tests/match_engine_id_integration_tests.rs` (existing harness
    referenced by the matcher spec) is updated to assert the new ID
    shape.
  - **Existing integration tests that precompute file IDs across
    multiple discovery invocations and then assert ID equality SHALL
    be migrated** to either (a) drive the matcher with a mocked AI
    provider whose response echoes the IDs from the live request
    (`MockOpenAITestHelper` already supports request capture), or
    (b) compare files by canonical path rather than by ID. Affected
    candidates include but are not limited to
    `tests/output_format_match_tests.rs`,
    `tests/output_format_cross_command_tests.rs`,
    `tests/match_engine_id_integration_tests.rs`, and any
    cache-related tests under `tests/` that assert on
    `file_<hex>` shapes. A grep audit (`grep -rEn "file_[0-9a-f]{16}"
    tests/`) MUST return zero matches at the end of this change.
  - A new integration test asserts that
    `subx-cli --output json match` (driven by a mocked AI provider via
    `MockOpenAITestHelper`) produces stdout containing exactly one JSON
    envelope and stderr containing no `🔍`, no `Preview:`, no
    `Warning: Skipping relocation`, no `Warning: Conflict resolution
    prompt`, and no free-form `eprintln!` chatter.
  - A new unit test in `src/core/uuidv7.rs::tests` asserts that the
    shared generator produces strictly increasing `unix_time_ts` values
    when called repeatedly in tight loops (regression cover for the
    1ms-spacing contract).
- **APIs**: `generate_file_id` is reachable via the public path
  `subx_cli::core::matcher::discovery::generate_file_id` because
  `core`, `matcher`, and `discovery` are all `pub` modules. Changing
  the function signature is therefore a library-level breaking
  change for any downstream consumer using `subx-cli` as a Rust
  library. The CLI binary contract is unchanged; the library API
  break is acceptable because (a) the discovery module is not part
  of the documented embedding surface and (b) we are pre-1.0 and
  semver permits breaking API changes within minor releases. The
  CHANGELOG entry SHALL call this out explicitly under `### Changed`.
- **Dependencies**: `uuid` features list shrinks to `["v7"]` only.
- **Performance**: The 1ms-spacing constraint adds at most `count - 1`
  millisecond sleeps per discovery scan. For typical scans of dozens of
  files this is negligible (≤ tens of milliseconds, dominated by disk
  I/O). For very large directory scans (>1000 files) the cumulative
  wait is bounded by `(count - 1) * 1ms`. Discovery is already an I/O
  bound operation and runs once per command invocation; this overhead
  is acceptable.
- **Documentation**: `AGENTS.md`, `docs/tech-architecture.md`, and
  `docs/machine-readable-output.md` are updated to reference the
  shared `core::uuidv7` module, the unified UUIDv7 identifier scheme,
  and the tightened JSON-mode stderr discipline. The doc-comment at
  the top of `src/cli/output.rs` (lines 12–20) and the doc on
  `Cli.output` in `src/cli/mod.rs` (lines 80–85) are updated so
  internal rustdoc agrees with the spec. `README.md` and
  `README.zh-TW.md` are reviewed and updated only if they document
  the relaxed stderr contract.
