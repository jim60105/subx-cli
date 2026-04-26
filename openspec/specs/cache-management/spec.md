# Cache Management

## Purpose

Store and invalidate the subtitle-matching cache that allows `subx match` to skip repeated AI calls and to apply previously computed dry-run results. Implemented in `src/commands/cache_command.rs`, `src/cli/cache_args.rs`, and `src/core/matcher/cache.rs`.

## Requirements

### Requirement: Match Cache Location

The system SHALL persist the match cache as a JSON file at `$CONFIG_DIR/subx/match_cache.json`, where `$CONFIG_DIR` is resolved via the standard platform configuration directory (`dirs::config_dir()`).

#### Scenario: Cache persisted after a match run
- **GIVEN** `subx match` executes successfully and produces at least one match operation
- **WHEN** the run finishes
- **THEN** a file at `$CONFIG_DIR/subx/match_cache.json` SHALL contain the serialized `CacheData` including file snapshot, match operations, and a configuration hash

### Requirement: Cache Clear Subcommand

The system SHALL expose `subx cache clear` as the only cache subcommand (`CacheAction::Clear`); executing it SHALL delete the match cache file if it exists and SHALL report a user-visible confirmation message.

#### Scenario: Clear existing cache
- **GIVEN** `$CONFIG_DIR/subx/match_cache.json` exists
- **WHEN** the user runs `subx cache clear`
- **THEN** the file SHALL be removed and the command SHALL print `Cache file cleared: <path>`

#### Scenario: Clear when no cache exists
- **GIVEN** no match cache file exists
- **WHEN** the user runs `subx cache clear`
- **THEN** the command SHALL print `No cache file found` and exit successfully without creating any file

### Requirement: Configuration-Aware Invalidation

The cache SHALL record the configuration hash and relocation settings under which its entries were generated so that changes in matching-relevant configuration invalidate prior results.

#### Scenario: Config hash stored with cache
- **GIVEN** a cache is written by `save_file_list_cache`
- **WHEN** the JSON file is inspected
- **THEN** it SHALL contain a `config_hash` field, an `original_relocation_mode` field, and an `original_backup_enabled` field reflecting the settings in effect during the producing run

### Requirement: Cache Reuse Preserves Relocation Mode

When a cached match result is reused on a subsequent `subx match` run with the same relocation mode, the system SHALL apply operations using the relocation mode and target directories that were in effect when the cache was produced, without issuing a new AI request. Implemented in `src/core/matcher/engine.rs` and exercised by `tests/match_cache_reuse_tests.rs` and `tests/match_cache_target_directory_tests.rs`.

#### Scenario: Copy mode cache reused without a second AI call
- **GIVEN** `subx match --copy <dir>` has populated the cache for a given set of files
- **WHEN** the user runs `subx match --copy <dir>` again on the same files and configuration
- **THEN** the cached match operations SHALL be reused, no additional AI provider call SHALL be made, and the subtitle SHALL be copied to the video's directory preserving the original subtitle at its source

#### Scenario: Target directory consistency between dry-run and actual execution
- **GIVEN** `subx match --copy --dry-run <dir>` has populated the cache for a given file set
- **WHEN** the user runs `subx match --copy <dir>` (actual execution) on the same files and configuration
- **THEN** the files written to disk SHALL land in the same target directories that the dry-run preview reported

### Requirement: Dry-Run Cache Reuse Without AI Calls

When a `subx match` invocation (including `--dry-run`) has populated the match cache for a given set of files, subsequent invocations with the same input files, relocation mode, backup setting, and configuration hash SHALL reuse the cached match operations and SHALL NOT call the AI provider again. Implemented in `src/core/matcher/engine.rs` (via `check_file_list_cache` / `save_file_list_cache`) and exercised by `tests/match_cache_reuse_tests.rs`.

#### Scenario: Repeated dry-run does not call the AI provider
- **GIVEN** the user has run `subx match --copy --dry-run <dir>` once against a mock AI provider and the cache has been populated
- **WHEN** the user runs the same `subx match --copy --dry-run <dir>` command a second time with no file or configuration changes
- **THEN** the mock AI provider SHALL have received exactly one chat-completion request in total across both invocations

### Requirement: Cache Invalidation On Relocation-Affecting Config Change

The `CacheData.config_hash` SHALL be derived from the configuration values that affect file-operation planning — specifically `relocation_mode` and `backup_enabled` — such that a change to either of those values on a subsequent run SHALL cause the cache entry to be treated as stale and a fresh AI call SHALL be performed. Implemented in `src/core/matcher/engine.rs::calculate_config_hash`.

#### Scenario: Relocation mode change invalidates cache
- **GIVEN** the cache was produced by a run with `relocation_mode = None`
- **WHEN** the user re-runs `subx match --copy <dir>` on the same files (relocation mode `Copy`)
- **THEN** the cached operations SHALL NOT be reused verbatim for the new relocation mode and the run SHALL either recompute or re-derive operations consistent with the new mode

### Requirement: Cache Scoped To File-List Directory Key

The cache SHALL be keyed on the sorted set of canonicalized input file paths (with size and modification time) combined with the configuration hash, so that cache reuse applies only when the same target file set is supplied on a subsequent run. Implemented by the `filelist_<hash>` key computed in `src/core/matcher/engine.rs` and exercised by `tests/match_cache_target_directory_tests.rs`.

#### Scenario: Different input file set misses the cache
- **GIVEN** a cache produced for file set `{A.mp4, A.srt}` in directory `videos/`
- **WHEN** the user runs `subx match` on a different file set `{B.mp4, B.srt}`
- **THEN** the new run SHALL compute a different cache key and SHALL NOT reuse the cached operations from the first file set

### Requirement: Cache Subcommands Emit Structured JSON Payloads

When any `cache` subcommand runs with the global output mode set to `json`, it SHALL emit a single JSON envelope on stdout (per the `machine-readable-output` capability) and SHALL NOT print free-form confirmation messages or status symbols on stdout. The envelope's `data` object SHALL be shaped according to the active subcommand:

- `cache status` → `{ "total": integer, "pending": integer, "applied": integer }`, plus any additional non-negative integer counters already exposed by the text path.
- `cache clear` → `{ "removed": integer }` reporting the number of cache entries removed (`0` when the cache was already empty).
- `cache rollback` → `{ "rolled_back": integer }` reporting the number of operations that were reverted.
- `cache apply` → `{ "applied": integer, "failed": integer, "items": [ { "id": string, "status": "ok" | "error", "error"?: { code, category, message } } ] }` reporting how many cached operations were applied successfully, how many failed, and per-item status for each entry processed.

A `cache list` subcommand is intentionally NOT covered by this change.
The current CLI does not expose a `cache list` action (see `CacheAction`
in `src/cli/cache_args.rs`); adding one would require new persistence
and indexing work in the matcher cache layer. Introducing the
subcommand together with its JSON payload (`{ entries: [...] }`) is
deferred to a follow-up change.

The pre-existing `cache status --json` flag (defined on
`StatusArgs` in `src/cli/cache_args.rs`; it is the only existing
JSON-style flag on any cache subcommand) SHALL be preserved as a
backward-compatible alias. When the user supplies either
`subx-cli --output json cache status` or
`subx-cli cache status --json`, both invocations SHALL share the
same renderer and emit byte-identical output. No other cache
subcommand currently exposes a `--json` flag, so the alias surface
is limited to `cache status`.

In `text` mode (the default) every cache subcommand's existing UX — confirmation messages and listing format — is unchanged.

#### Scenario: cache clear reports the removed count
- **WHEN** the user runs `subx-cli --output json cache clear`
- **THEN** `data.removed` SHALL be a non-negative integer equal to the number of cache entries removed

#### Scenario: cache apply reports applied and failed counts
- **GIVEN** a cache containing N pending operations of which K fail to apply
- **WHEN** the user runs `subx-cli --output json cache apply`
- **THEN** `data.applied + data.failed == N`, `data.failed == K`, and `data.items` SHALL contain exactly N entries with each failed entry carrying `status == "error"` and an `error` object
