## Why

The `subx-cli config set` command is currently unable to repair its own configuration file. When an existing `~/.config/subx/config.toml` already contains a combination of values that fails the cross-section validator — for example `ai.provider = "openai"` paired with an `http://` `ai.base_url` (which became disallowed after `add-local-llm-provider` shipped the "hosted providers MUST use https" rule) — every invocation of `subx-cli config set <KEY> <VALUE>` aborts with:

```
Configuration error: Configuration validation failed: Configuration error:
ai.base_url uses unsupported scheme `http://` for hosted provider `openai`; ...
```

This is a **chicken-and-egg lockout**: `set_config_value` calls `self.get_config()?` as its first step, and `get_config()` itself runs the full cross-section `validator::validate_config` on the loaded file. So even a user trying to *fix* the bad pairing — `subx-cli config set ai.provider "local"` or `subx-cli config set ai.base_url "https://api.openai.com/v1"` — is rejected before the new value is ever applied. The same lockout affects `subx-cli config get` and `subx-cli config list` (both also call `get_config()`), so users cannot even *inspect* the bad pairing through the CLI. The only escape paths today are manual file editing (defeats the purpose of `config set`) or `subx-cli config reset` (destroys every other user setting). Neither is acceptable, and the bug bites exactly the users who installed an older version with permissive validation, then upgraded to a stricter one — i.e., the precise audience the local-LLM hint message was written for.

## What Changes

- **BREAKING (good kind)**: `subx-cli config set` MUST be able to *repair* an existing config file even when the on-disk file fails cross-section validation. The single mutation being applied still has to pass field-level validation, and the *resulting* configuration (after the mutation) still has to pass full cross-section validation, so the user can never write a worse file than they already have on disk; but a user loading a bad file MUST still be able to issue the one set command that fixes it.
- Introduce a "tolerant load" code path on the `ConfigService` trait (so the `commands::config_command` handlers, which only see `&dyn ConfigService`, can invoke it). The path reads only the file (no environment-variable overlay), runs field-level validation only, and *skips* cross-section validation. The pre-existing strict load — file + env overlay + field validation + cross-section validation — is kept as-is and continues to drive `get_config`, `reload`, and every command-execution site outside the `config` subcommand.
- `set_config_value`'s ordering becomes: **tolerant file load** → field-validate the new value → apply it → strict-validate the *post-mutation* config → if strict validation now passes, save the file and rebuild the cache; if it still fails, return the strict error so the user knows the single mutation was not enough to repair the file.
- `subx-cli config get <KEY>` and `subx-cli config list` MUST also tolerate a strict-invalid file: they return the on-disk values and emit a non-fatal warning that the configuration is currently strict-invalid. The warning surfaces through the existing machine-readable warnings convention (`Envelope::warnings: Option<Vec<String>>` defined by `add-machine-readable-output`) when the global `--output json` flag is in effect, and as a single line on stderr in text mode. Exit code stays `0` for successful reads.
- Repair commands operate on the **file view only**: the tolerant load deliberately ignores environment-variable overrides so that (a) `config set` does not silently bake env-only values like `OPENAI_API_KEY` into the persisted file, and (b) the warning emitted by `config get`/`list` reflects the on-disk state the user is being asked to fix, not a transient env-merged view that may mask the bad pairing.
- `subx-cli config reset` keeps its current behavior (writes defaults unconditionally; no validation of the prior file required).
- The cache-population invariant is preserved: only strict-valid configs may enter `cached_config`. Tolerant load never populates the cache.
- Add integration regression tests that build a `config.toml` on disk with the offending `provider=openai + http://` pairing, then assert (a) `subx-cli config set ai.provider "local"` succeeds and the resulting file is strict-valid; (b) `subx-cli config set ai.base_url "https://api.openai.com/v1"` succeeds; (c) an unrelated mutation that does *not* heal the cross-section error fails and leaves the file unchanged; (d) `subx-cli config get` and `subx-cli config list` succeed with an advisory; (e) any non-`config` command (e.g. `subx match`) still aborts at config load with the existing strict error.
- Document the repair semantics in `docs/configuration-guide.md` and `docs/command-reference.md`, and add a release-notes `### Fixed` entry to `CHANGELOG.md`.

This is *not* a relaxation of validation. Loading a config to *use* it (every command other than `config set`/`get`/`list`) still goes through strict cross-section validation; misconfigured users still get the clear error pointing at the bad pairing. What changes is that the `config` subcommand is now the documented escape hatch instead of a dead end.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `configuration-management`: relaxes the precondition for `config set`, `config get`, and `config list` so they no longer require the on-disk file to pass cross-section validation; the post-mutation file written by `config set` MUST still pass strict validation. Adds a requirement that `config set` is a usable repair path for any single-mutation transition whose endpoint is strict-valid.

## Impact

- **Code**: `src/config/service.rs` (a new `load_tolerant`-style method on the `ConfigService` trait, added to `ProductionConfigService` and `TestConfigService`; the new method must split file parsing from env overlay so individual malformed fields still surface as field-level errors instead of being masked by the existing deserialize-fallback path; `set_config_value` rewired to use it; pre-mutation `get_config()` removed). `src/commands/config_command.rs` (the `Get`/`List` handlers gain the strict-invalid advisory path; the `Set` handler no longer pre-loads via `get_config`). No public CLI flags change.
- **Behavior**: pure bug fix from the user's perspective. Strict-valid configs behave exactly as today.
- **Tests**: new integration regression tests exercising the exact scenario from `tmp/propose.md`. Existing `config` tests must keep passing; `tests/cli/output_format_config.rs` and `tests/cli/output_format_cross_command.rs` will need their snapshots updated to reflect the `warnings` field shape change for strict-invalid inputs and to confirm strict-valid output is byte-equivalent (warnings remains `null`/absent for strict-valid).
- **Docs**: `docs/configuration-guide.md` ("Repairing an invalid config" section), `docs/command-reference.md` (note on `config set`/`get`/`list` behavior under strict-invalid file), `CHANGELOG.md` `### Fixed`.
- **Backward compatibility**: fully compatible. The set of inputs that previously succeeded is a strict subset of the inputs that succeed now, and the JSON output shape is unchanged for any strict-valid input (`warnings` was already `Option<Vec<String>>`, omitted when `None`).
- **No new dependencies**.
