# D2 implementation notes (apply stage)

## Recorded deviations / corrections to tasks.md expectations

- **1.4 (gap-3 evidence):** the `MatchConfig {` grep returns more than two
  sites: the two live ones plus `cache_command.rs:734` (a *cache-replay*
  literal whose values come from the cache file, not the loaded config —
  not migratable to `match_config()` without changing replay semantics)
  and test fixtures. Decision 2's migration claim was scoped to the two
  live config-derived constructions; only `match_command.rs` migrated.
- **3.4 spec correction:** the delta spec's "archive with no parent falls
  back to `.`" scenario was re-derived against real `Path` semantics:
  `Path::new("subs.zip").parent()` is `Some("")`, so a parentless archive
  renders the bare filename (both methods), never `./movie.vtt`; the
  `Path::new(".")` fallback is reachable only for a literally-empty path.
  Spec + rustdoc + tests updated to the true semantics (see
  `output_path_relative_archive_yields_empty_path_not_fallthrough`).
- **4.6:** `archive_origin` grep in `match_command.rs` is zero hits (loop
  replaced by `apply_archive_origin_relocation`).
- **11.1 left unchecked:** `cargo fmt` applied and the lib-gate
  `cargo clippy -p subx-core --all-features -- -D warnings` is green, but
  `--all-targets` surfaces ~40 pre-existing test-code warnings in files
  this change does not touch (`error.rs`, `archive/sevenz.rs`,
  `ai_retry_implementation_tests.rs`, vad tests, …); intentionally not
  chased — main-agent quality gate owns that call.
- **11.2 done via `cargo test` scoped filters** (lib +
  `match_progress_tests` + `factory_match_engine_tests` green); nextest
  invocation itself is the main agent's gate.

## Phase-8 sizing decision (Decision 11) — NOT taken

The `../subx` scratch-worktree verification (8.1–8.4) was deferred to the
archive stage: it produces no committed artifact and is explicitly
cuttable. Phases 5–7 were NOT squeezed out — see below for their state.

## Remaining for the next stage (apply was interrupted by budget)

- Phase 2.7 CLI adoption is DONE (match_command literal → `match_config()`).
- Phase 3.5/3.6/3.7 (convert/translate adoption + sync Decision-3 comments),
  4.5 (match_command call-site swap — the loop still exists),
  Phase 7 (TerminalReporter bar ownership, `ui` template `{msg}`,
  retire `match_command::create_progress_bar`, JSON-silence +
  disabled-flag characterisation tests), 8.x, 9.x, 10.x remain.
- The second Rubber-Duck (changes critique) has NOT run yet; the plan
  critique findings that bind the remaining work:
  1. JSON/quiet mode must still construct the bar via
     `ui::create_progress_bar` on `Started` (constructor is the forced
     hide route) — hide, don't skip.
  2. Parallel ticker must NOT misuse `Advanced.item` for the
     Active/Queued status text beyond what task 7.5 sanctions; keep
     `Advanced{item: None}` at completions.
  3. `execute_with_client`'s public signature must stay 3-arg (no factory
     param added).
  4. `TerminalReporter` is a public unit struct re-exported from `cli` —
     adding state breaks `let reporter = TerminalReporter;` in its unit
     test; migrate deliberately (constructor fn + update tests/re-export).
