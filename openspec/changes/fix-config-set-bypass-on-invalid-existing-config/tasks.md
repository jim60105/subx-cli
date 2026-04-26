# Tasks: fix-config-set-bypass-on-invalid-existing-config

## 1. Investigate and confirm the bug surface

- [x] 1.1 Reproduce the lockout locally: write `[ai] provider="openai" base_url="http://localhost:1234/v1" api_key="sk-test"` to a tempdir `config.toml`, point the config home env at it, and run `subx-cli config set ai.provider local`; capture the exact error message.
- [x] 1.2 Confirm `subx-cli config get ai.base_url` and `subx-cli config list` (and `subx-cli --output json config list`) fail with the same strict-validation error.
- [x] 1.3 Confirm `subx-cli config reset` already succeeds in the same scenario; document this as the only existing escape hatch.
- [x] 1.4 Inventory every direct caller of `ConfigService::get_config` under `src/commands/` and `src/core/`; note which call sites must keep strict semantics (everything outside `commands::config_command`).
- [x] 1.5 Confirm in `src/config/service.rs` that `validate_and_set_value` (≈line 422) already runs `validate_configuration` (≈line 444) on the post-mutation config, and that the outer `validate_config` call inside `set_config_value` (≈line 781) is therefore redundant. Note the redundant call for removal in section 3.

## 2. Tolerant load on the `ConfigService` trait

- [x] 2.1 Add a new method to the `ConfigService` trait in `src/config/service.rs` (working name `load_for_repair(&self) -> Result<Config>`). Document on the rustdoc that it (a) reads only the file, (b) does not apply env overlays, (c) runs field-level validation, (d) skips cross-section validation, (e) does not populate the cache, and (f) is for use by `config set/get/list` only.
- [x] 2.2 Implement `load_for_repair` on `ProductionConfigService`. The body MUST:
   - Read the resolved config file path as a `String`.
   - Call `toml::from_str::<Config>(&content)` directly. On error, return `SubXError::config(...)` with a parse error message naming the file path. **No fallback to `Config::default()`.**
   - Run `field_validator::normalize_ai_provider` on `config.ai.provider`.
   - Run field-level validation across all sections (e.g. via `field_validator::validate_all_fields(&config)` if it exists, or by composing the existing per-field validators).
   - Return the `Config` without invoking `validator::validate_config` and without applying env overlay.
- [x] 2.3 Implement `load_for_repair` on `TestConfigService` (it can simply clone the in-memory test config; tolerant ≡ strict for tests).
- [x] 2.4 If a `field_validator::validate_all_fields` helper does not yet exist, extract one from the existing per-key field-validation logic so that `load_for_repair` and `validate_and_set_value` agree on the rule set.
- [ ] 2.5 Add a unit test asserting `load_for_repair` returns `Ok` for a `config.toml` that fails strict validation specifically because of the `provider=openai + http://` pairing.
- [ ] 2.6 Add a unit test asserting `load_for_repair` returns `Err` when the file contains a syntactically broken individual field (for example `ai.base_url = "not a url"` or `sync.max_offset_seconds = "not a number"`), and that the error message identifies the bad field.
- [ ] 2.7 Add a unit test asserting `load_for_repair` returns `Err` on a TOML parse failure, and that the error names the file path.
- [ ] 2.8 Add a unit test asserting `load_for_repair` does NOT apply env overlay: with the file holding `ai.api_key = "sk-file"` and `OPENAI_API_KEY="sk-env"` exported via `TestEnvironmentProvider`, the returned `Config.ai.api_key` is `"sk-file"`.
- [ ] 2.9 Add a unit test asserting `load_for_repair` does NOT populate `cached_config`: invoke it once on a strict-invalid file, then `get_config()` and confirm the strict error still surfaces (no cache poisoning).

## 3. Rewire `set_config_value`

- [x] 3.1 Change step 1 of `set_config_value` (≈line 775 in `src/config/service.rs`) to call `self.load_for_repair()` instead of `self.get_config()`.
- [x] 3.2 Remove the redundant `crate::config::validator::validate_config(&config)?` call at ≈line 781 (it is duplicated by the call inside `validate_and_set_value`).
- [x] 3.3 Confirm `validate_and_set_value` still runs (a) field validation of the new key/value and (b) full strict validation of the post-mutation config; if either is missing today, restore it.
- [x] 3.4 Confirm step 3 (write file) and step 4 (cache update) only run when step 2 succeeds; assert via test that a failed step 2 leaves the file byte-identical and the cache empty.
- [x] 3.5 Add a unit/integration test against a `TempDir`-backed `ProductionConfigService` that seeds the strict-invalid `provider=openai + http://` pairing and asserts `set_config_value("ai.provider", "local")` succeeds and the resulting file is strict-valid.
- [x] 3.6 Add a unit/integration test using the same fixture and asserting `set_config_value("ai.base_url", "https://api.openai.com/v1")` succeeds.
- [x] 3.7 Add a unit/integration test using the same fixture and asserting `set_config_value("general.backup_enabled", "true")` *fails* with the cross-section error and that the file on disk is unchanged.
- [x] 3.8 Add a unit/integration test asserting that after a failed `set_config_value`, a subsequent `get_config()` (strict path) still returns `Err` with the same cross-section error — i.e. the cache was not poisoned.
- [x] 3.9 Add a unit/integration test asserting that with `OPENAI_API_KEY="sk-env"` exported, `set_config_value("ai.provider", "local")` against a fixture with `ai.api_key = "sk-file"` writes `ai.api_key = "sk-file"` to disk (NOT `sk-env`).

## 4. Rewire `config get` and `config list`

- [x] 4.1 Change `commands::config_command` `Get` handler to call `config_service.load_for_repair()` instead of `config_service.get_config()`, then run `validator::validate_config` purely for advisory purposes.
- [x] 4.2 In text mode, on advisory failure print `warning: configuration is currently invalid: <message>` to stderr; keep stdout output identical to today.
- [x] 4.3 In `--output json` mode, append the warning string to `Envelope::warnings` (the existing `Option<Vec<String>>` field at `src/cli/output.rs:104-120`); for strict-valid configs leave `warnings = None` so `skip_serializing_if` keeps the field absent and existing JSON snapshots remain byte-equivalent.
- [x] 4.4 Apply the same change to the `List` handler so `subx config list` and `subx-cli --output json config list` behave consistently.
- [x] 4.5 Add an integration test under `tests/cli/output_format_config.rs` (or a new sibling file wired through an existing `tests/*.rs` harness) that exercises the strict-invalid fixture, invokes `subx-cli --output json config list`, asserts exit `0`, parses the JSON, and confirms `warnings` is a non-empty array containing the validator's message.
- [x] 4.6 Add an integration test asserting strict-valid JSON shape is unchanged: same command, strict-valid fixture, output byte-equivalent to the pre-change snapshot (i.e. no `warnings` field).
- [x] 4.7 Add an integration test invoking `subx-cli config get ai.base_url` against the strict-invalid fixture and asserting exit `0`, stdout contains the URL, and stderr contains the advisory text.
- [ ] 4.8 Update existing snapshots in `tests/cli/output_format_config.rs` and `tests/cli/output_format_cross_command.rs` only if they fail (they should not, by Decision 8 of design.md); investigate any failure to confirm it represents an intentional change.

## 5. Verify non-`config` commands still strict-load

- [x] 5.1 Add an integration test that seeds the strict-invalid fixture and invokes a non-config command (e.g. `subx match --dry-run` against a tempdir) and asserts the existing cross-section error is surfaced and the command does not proceed.
- [ ] 5.2 Manually disable the new tolerant-load wiring (temporarily) and confirm the test from 5.1 still passes — i.e. that the test isolates non-config command behavior independently of the new code path. Restore the wiring before commit.

## 6. Verify `reset` and env-edge cases

- [x] 6.1 Add a regression test seeding the strict-invalid fixture and asserting `subx config reset` rewrites the file with `Config::default()` and that the new file is strict-valid.
- [ ] 6.2 Add a regression test for the env-edge "advisory reflects file state" case: strict-valid file + env vars that would create a strict-invalid effective view; assert `config get` does NOT emit an advisory.

## 7. Documentation

- [x] 7.1 Add a "Repairing a strict-invalid configuration" subsection to `docs/configuration-guide.md` covering the canonical `provider=openai + http://` scenario and the two repair commands; mention that env overrides do not participate in repair operations.
- [x] 7.2 Cross-link from `docs/command-reference.md` under the `config` subcommand description.
- [x] 7.3 Add a `### Fixed` entry under `[Unreleased]` in `CHANGELOG.md` summarizing the user-visible fix; if no `[Unreleased]` block exists currently, create it at the top of the file per the project's Keep-a-Changelog convention.
- [ ] 7.4 If `docs/config-usage-analysis.md` references `set_config_value` or `get_config` semantics, update the call-hierarchy notes to reflect the tolerant-load path.

## 8. Quality gate

- [x] 8.1 Run `cargo fmt` and `cargo clippy -- -D warnings`.
- [x] 8.2 Run `cargo nextest run || true` and triage every failing test.
- [x] 8.3 Run `scripts/quality_check.sh` and confirm all gates pass.
- [x] 8.4 Run `openspec validate fix-config-set-bypass-on-invalid-existing-config --strict` and confirm the change validates.
