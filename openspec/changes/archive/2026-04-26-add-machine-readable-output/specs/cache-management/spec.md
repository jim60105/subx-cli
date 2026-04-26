## ADDED Requirements

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
