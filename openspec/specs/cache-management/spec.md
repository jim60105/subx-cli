# Cache Management

## Purpose

Hold the CLI half of the subtitle-matching cache agreement: the `cache clear` subcommand and its independent canonical-path resolver (which must keep producing the path the core half defines, honouring `XDG_CONFIG_HOME`), and the structured JSON payloads of the cache subcommands. Implemented in `src/commands/cache_command.rs` and `src/cli/cache_args.rs`. The cache location, JSON format, configuration-aware invalidation, cache reuse, dry-run reuse without AI calls, and the scoping key are specified by the same-named `cache-management` capability in `subx-core`.

## Requirements

### Requirement: Cache Clear Subcommand

The system SHALL expose `subx cache clear` as the only cache subcommand (`CacheAction::Clear`); executing it SHALL delete the match cache file if it exists and SHALL report a user-visible confirmation message.

The path the subcommand deletes SHALL be the same path the cache producer writes. `subx-cli` resolves it independently in `src/commands/cache_command.rs` (`cache_path()`, over `get_config_dir()`), and the producer resolves it in `subx-core` under the `cache-management` capability's *Match Cache Location* requirement in that repository. Because the two resolvers are now in different repositories, with no compiler and no shared constant binding them, this requirement SHALL be treated as an agreement obligation on the CLI side: a change to either resolver SHALL be accompanied by a check that the other still produces the identical path, and the `XDG_CONFIG_HOME` override honoured by `get_config_dir()` SHALL be honoured by the producer too. A divergence is silent — `cache clear` reports `No cache file found` while a cache remains on disk — so it SHALL NOT be left to be discovered by a user.

#### Scenario: Clear existing cache
- **GIVEN** `$CONFIG_DIR/subx/match_cache.json` exists
- **WHEN** the user runs `subx cache clear`
- **THEN** the file SHALL be removed and the command SHALL print `Cache file cleared: <path>`

#### Scenario: Clear when no cache exists
- **GIVEN** no match cache file exists
- **WHEN** the user runs `subx cache clear`
- **THEN** the command SHALL print `No cache file found` and exit successfully without creating any file

#### Scenario: The two resolvers agree across the repository boundary
- **GIVEN** a run of `subx match` that wrote a match cache, and any value of `$XDG_CONFIG_HOME` in effect for both invocations
- **WHEN** the user then runs `subx cache clear`
- **THEN** the path `subx-cli` resolves SHALL be byte-identical to the path the producer in `subx-core` wrote, so that the file is found and removed rather than reported absent
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
