## Context

`subx-cli config set` is the only programmatic way for users to edit their configuration; it loads `~/.config/subx/config.toml`, mutates one key, and writes the file back. Today the load step (`ProductionConfigService::get_config`) is unconditionally strict: it runs the full `validator::validate_config` cross-section check and returns an error if the on-disk file fails. That cross-section check enforces, among other rules, "hosted providers (`openai`, `openrouter`, `azure-openai`) MUST use an `https://` `ai.base_url`".

The combination "any user who, before `add-local-llm-provider` shipped, set `ai.base_url = http://localhost:1234/v1` while keeping `ai.provider = openai` (this used to be a common LM Studio / Ollama recipe) and then upgrades" produces an on-disk file that is strict-invalid. From that moment on, every `subx-cli config set <KEY> <VALUE>` they try — including the very fix we tell them to apply — fails before reaching the mutation logic. The error message even names the right knob to turn (`ai.provider`), but the CLI refuses to turn it.

The same lockout affects `config get <KEY>` and `config list`, which both currently call `get_config()` and therefore cannot be used to *inspect* the bad pairing either. Only `config reset` works, but it nukes every other user setting, which is a hostile recovery path.

The fix has to preserve the strict cross-section check for every other code path — running `subx-cli match`, `subx-cli sync`, etc. against an invalid file MUST still fail loudly with the existing error message — because those commands depend on the invariants the validator enforces (e.g. they will issue HTTPS requests, expect a non-empty API key, etc.). What we are changing is *only* the precondition under which the configuration-editing surface itself runs.

A correct design has to nail down three things that the rubber-duck pass flagged: (1) the new helper must be on the `ConfigService` *trait*, not just the production impl, because `commands::config_command` only sees `&dyn ConfigService`; (2) the env-vs-file boundary in the tolerant path must be explicit, otherwise repair commands could silently bake env-only values into the file; (3) the existing parse-fallback in `load_and_validate` (which falls back to `Config::default()` when full deserialization of the TOML fails) must not be reused as-is, because it would mask malformed individual fields and contradict the "field-level validation still runs" promise.

## Goals / Non-Goals

**Goals:**

- A user with a strict-invalid `config.toml` can run `subx-cli config set <KEY> <VALUE>` to repair it, provided the *post-mutation* file is strict-valid.
- A user with a strict-invalid `config.toml` can run `subx-cli config get <KEY>` and `subx-cli config list` to see what is in the file, with a clear advisory that the file is currently invalid and why.
- Loading a configuration *to use it* (any non-config command, plus the cache populated after a successful `config set`) still goes through strict cross-section validation; behavior for strict-valid files is byte-identical to today.
- Field-level validation of the new value (type, range, enum membership, URL parse-ability, etc.) is unchanged and still runs *before* the mutation is applied, so users cannot widen validation by abusing `config set`.
- The post-mutation strict re-validation is identical to today's `validate_and_set_value` step (today it already calls `validate_configuration` after mutation at `service.rs:444`); failures still surface the standard error message.
- `config set` operates on the *file view only* — the tolerant load deliberately omits env-variable overlays so that successful `config set` writes only file-derived values plus the new mutation, and never persists env-only secrets like `OPENAI_API_KEY`.

**Non-Goals:**

- Relaxing or removing any cross-section rule in `validator::validate_config`. The fix is purely about *when* the rule runs in the `config set`/`get`/`list` paths.
- Adding `--force`, `--allow-invalid`, or any new CLI flag. The repair behavior is the new default for these subcommands and needs no opt-in.
- Repairing semantically corrupted TOML (a file that fails parsing). Those errors are upstream of the validator and remain hard failures; users still need `config reset` for that.
- Multi-step repair planning ("apply these N sets to migrate from state A to state B"). The user is responsible for picking the single mutation that lands them in a strict-valid state.
- Changing `ai.provider` canonicalization (`ollama → local`, etc.) — that already happens in `field_validator::normalize_ai_provider` and is reused as-is.
- Adding a `config unset` subcommand. Today, clearing optional values is done via `config set <KEY> ""` (per `src/cli/config_args.rs` doc-comments). This change does not introduce a new `unset` verb.
- Changing the JSON warnings shape. The existing `Envelope::warnings: Option<Vec<String>>` (`src/cli/output.rs:104-120`) is reused. Each warning is a single human-readable string; we are *not* migrating to `{code, message}` objects in this change.

## Decisions

### Decision 1: tolerant load lives on the `ConfigService` *trait*

Add a new trait method, working title `load_for_repair(&self) -> Result<Config>`, to the `ConfigService` trait at `src/config/service.rs` (the trait declaration around line 81). Implement it on `ProductionConfigService` and `TestConfigService`. Default-implement nothing — both impls supply a real body so callers don't fall through to a stub.

For `ProductionConfigService`, the body parses the TOML file directly into a `Config` (no fallback to `Config::default()` on deserialize failure), normalizes the AI provider via `field_validator::normalize_ai_provider`, runs field-level validation, and returns the `Config` *without* invoking `validator::validate_config` and *without* applying any env-variable overlay.

For `TestConfigService`, the body returns the in-memory test config directly (test configs are already field-valid by construction; tolerant and strict are equivalent for tests).

**Rationale:** the rubber-duck pass correctly noted that putting the helper only on `ProductionConfigService` would make `commands::config_command` unable to call it. The trait method makes the policy explicit at every call site, and tests that depend on `&dyn ConfigService` automatically get a working tolerant path.

### Decision 2: `set_config_value` rewires to tolerant load → mutate → strict-validate → save

The new sequence inside `ProductionConfigService::set_config_value`:

1. `let mut config = self.load_for_repair()?;` — fails only on parse errors or field-level invariants. Reads file view *without* env overlay.
2. `self.validate_and_set_value(&mut config, key, value)?;` — kept as-is. This helper today already (a) field-validates the new value via `field_validator`, (b) mutates `config`, and then (c) calls `validate_configuration` on the post-mutation full config. We do **not** duplicate step (c) at the outer level (the rubber-duck pass flagged that today's outer `validate_config` call at line 781 is actually a duplicate of the validation already done inside `validate_and_set_value`); the redundant call at line 781 is removed.
3. `self.save_config_to_file_with_config(&path, &config)?;` — write the file (only reached if step 2's internal strict validation passed).
4. Replace cache with the now-strict-valid `config`.

If step 2 fails for cross-section reasons, we explicitly do *not* write the file and we do *not* clear the cache. The next `config set` invocation will re-run from step 1, so the user can iterate.

**Rationale:** the failure modes the user can encounter are exactly: (a) on-disk file fails parsing or has a malformed individual field → step 1 fails; (b) the new key/value is itself bogus → step 2 fails with a precise field error; (c) the *combination* is still invalid after the mutation → step 2 fails with the standard cross-section error from `validate_configuration`. Each case has a distinct, actionable message, and (c) is the case where the user picked the wrong repair.

### Decision 3: `config get <KEY>` and `config list` switch to tolerant load + advisory

Both subcommand handlers in `src/commands/config_command.rs` are rewired to call `config_service.load_for_repair()` instead of `config_service.get_config()`. After loading, they run `validator::validate_config` purely for its diagnostic output: if it returns `Err`, the command emits its normal output *plus*:

- text mode: a single line on stderr formatted as `warning: configuration is currently invalid: <message>` (no color, no panic).
- `--output json` mode: the warning string is appended to `Envelope::warnings`, using the existing `Option<Vec<String>>` shape from `src/cli/output.rs:104-120` and the convention established in `add-machine-readable-output`. For strict-valid configs `warnings` remains `None` (absent from the JSON document via `skip_serializing_if`), so existing JSON snapshots are unchanged.
- Exit code stays `0` (the read succeeded).

**Rationale:** users need to *see* the bad value before they can pick the correct `config set` mutation; today they cannot, because `get`/`list` themselves abort. The advisory keeps the existing strict-valid output format byte-identical so machine-readable consumers don't break, while making the invalid case observable through the channel the project already established.

### Decision 4: tolerant load is file-view only; env vars do not participate

The new `load_for_repair` body deliberately skips the env-overlay step that `load_and_validate` runs at `service.rs:297-406`. Concretely:

- It reads `config.toml` from the resolved path.
- It does not check `OPENAI_API_KEY`, `AZURE_OPENAI_API_KEY`, `OPENROUTER_API_KEY`, `SUBX_*`, `AZURE_OPENAI_ENDPOINT`, or any other env var.
- It does not invoke any "compatibility env var" canonicalization that would mutate `ai.provider` or `ai.base_url`.

The downstream consequences are:

1. `config get` / `config list` show the *file* values, which is what the user is being asked to fix. If the file says `ai.provider = openai + http://`, the advisory fires regardless of whatever env vars happen to be exported, because the file *is* invalid.
2. `config set` writes only "file values plus the one mutation" back to disk. It cannot bake env-only secrets into the persisted file (for example, a user with `OPENAI_API_KEY` exported in their shell who runs `subx-cli config set ai.provider local` will end up with `ai.provider = "local"` in the file and the `ai.api_key` field unchanged from whatever was in the file — *not* polluted with the env value).
3. Strict validation of the post-mutation config in step 2 of `set_config_value` is also run on the file view only. This is acceptable because (a) hosted providers can validly omit `api_key` from the file when they expect to read it from the env at command time — and the existing strict validator already accommodates that case (an empty `ai.api_key` is acceptable for `openai` if the env var supplies it); (b) the user explicitly invoked `config set` to edit the file, not the effective merged config, so failures must be evaluated against the file.

**Rationale:** repair commands operate on the file. Env overlays are a runtime concern of *using* the config; they have no business shaping what `config set` writes back to disk. Mixing them in is the source of an existing latent bug (today's `set_config_value` reads `get_config()` which already env-merged, then writes the merged config back via `save_config_to_file_with_config`, which means env-only values can leak into the file) — this change incidentally fixes that, but the test coverage is what makes it intentional rather than incidental.

### Decision 5: parsing path in `load_for_repair` does not reuse the deserialize-fallback

`ProductionConfigService::load_and_validate` at `service.rs:255-295` falls back to `Config::default()` when full TOML deserialization fails, then partially recovers AI env fields. Reusing that body would silently swallow `sync.max_offset_seconds = "not a number"` (the key would be dropped during fallback and a default value would surface).

`load_for_repair` therefore implements its own narrower file path:

1. Read the file as a `String`.
2. `toml::from_str::<Config>(&content)` directly. On error, return `SubXError::config(format!("failed to parse {}: {}", path.display(), err))`. **No fallback.**
3. Run `field_validator::normalize_ai_provider` to canonicalize the provider field.
4. Run `field_validator::validate_all_fields(&config)` (or the equivalent helper that field-validates each section). On error, return the field error.
5. Return `Ok(config)`.

Field-level errors and parse errors are both surfaced to the user; only cross-section errors are deliberately skipped at this stage. This keeps the spec's promise that "individual malformed fields still error" honest.

**Rationale:** the rubber-duck pass identified this masking risk. The fix is to write a small, purpose-built loader for repair instead of trying to share code with the strict path.

### Decision 6: cache semantics preserved

Tolerant load does *not* populate `cached_config`. Only strict-valid configs may enter the cache. `set_config_value` writes the new strict-valid config into the cache on success (step 4 above). After a tolerant load that the strict validator would reject (i.e., during a `config get` / `config list` advisory path), the cache stays empty — so the very next `get_config()` call (e.g. a follow-up command) will re-load and re-validate, producing the strict error as expected. This preserves the invariant "anything in the cache has passed strict validation".

**Rationale:** the cache exists to short-circuit strict validation; allowing strict-invalid configs into it would silently bypass validation for downstream commands, which is exactly the regression we must avoid.

### Decision 7: integration regression tests mirror the bug report

Tests use `CLITestHelper` (which provides a `TempDir` and writes a real `config.toml` on disk) plus invocations of the actual binary (or `cli::run_with_config` driven by a `ProductionConfigService` rooted at the tempdir, so the production strict/tolerant code path is what's actually exercised). `TestConfigService` is *not* used for these tests, because the bug only exists in the production code path.

The fixture seeds:

```toml
[ai]
provider = "openai"
base_url = "http://localhost:1234/v1"
api_key  = "sk-test"
```

Test variants:

1. **Repair via provider:** `subx-cli config set ai.provider local` — exit 0, file's `[ai].provider` is `local`, file's other keys preserved verbatim, file passes strict validation.
2. **Repair via URL:** `subx-cli config set ai.base_url https://api.openai.com/v1` — exit 0, file's `[ai].base_url` is the new value, file's other keys preserved, file passes strict validation.
3. **Non-repair edit:** `subx-cli config set general.backup_enabled true` — non-zero exit, file on disk byte-identical to fixture, in-memory cache empty.
4. **`config get` advisory:** `subx-cli config get ai.base_url` — exit 0, stdout = `http://localhost:1234/v1`, stderr contains the advisory.
5. **`config list --output json` advisory:** `subx-cli --output json config list` — exit 0, JSON parses, `data` field contains the file's values, `warnings` is a non-empty `[string]` array containing the validator's message.
6. **`config list --output json` strict-valid:** same command on a fixture file that *is* strict-valid — `warnings` is absent or `null` (per `skip_serializing_if`), output otherwise byte-equivalent to the pre-change shape.
7. **Env-edge case A:** fixture is strict-invalid as above; `OPENAI_API_KEY=sk-fromenv` is set in the test env; `subx-cli config set ai.provider local` succeeds and the resulting file's `ai.api_key` is still `sk-test` (not `sk-fromenv`).
8. **Env-edge case B:** fixture is strict-valid; an env var would *create* a strict-invalid effective view. `subx-cli config get ai.base_url` does *not* emit an advisory (because the file is the source of truth for the advisory), and the output shows the file's `ai.base_url`.
9. **Non-config command:** with the strict-invalid fixture, `subx-cli match --dry-run <some-tempdir>` fails at config load with the existing strict-validation error; the error message and exit code match today's behavior.

**Rationale:** these nine cases pin the contract from all sides: forward-fix via two different keys; rejection of non-repair edits and field-level-bogus values; observability via stderr and JSON; preservation of the strict load path for non-config commands; and env-vs-file separation.

### Decision 8: existing JSON snapshot tests get explicit updates

`tests/cli/output_format_config.rs` and `tests/cli/output_format_cross_command.rs` currently snapshot the JSON shape for `config list --output json` against strict-valid fixtures. Per Decision 3, those outputs do not change (warnings stays `None` / absent); the existing snapshots therefore must continue to pass byte-for-byte. Any test that would change is a strict-invalid fixture, and there are none today, so the new `warnings` test cases are pure additions, not modifications.

The integration test file added by this change goes under `tests/cli/` (matching the existing organization for output-format tests) or `tests/commands/` if a closer fit exists. Whichever directory is chosen, the file is wired through an existing top-level `tests/*.rs` harness (per AGENTS.md's note that nested test directories are not auto-discovered).

**Rationale:** documents which existing tests must keep passing unchanged so reviewers don't waste time hunting for shape regressions, and codifies the test-file placement conventions.

### Decision 9: documentation surface

Add a short "Repairing a strict-invalid configuration" subsection to `docs/configuration-guide.md` that walks through the canonical scenario (`provider=openai + http://` → fix provider, or fix URL) and notes that `config set`/`get`/`list` are now repair-aware. Cross-link from `docs/command-reference.md` under the `config` subcommand. Add a `### Fixed` entry to `CHANGELOG.md` `[Unreleased]` with the user-visible summary "subx-cli config set/get/list no longer abort on existing strict-invalid configuration files; the post-mutation file is still strict-validated."

If `docs/config-usage-analysis.md` references the call hierarchy for `set_config_value` or `get_config`, update those notes to reflect the tolerant-load branch.

## Risks / Trade-offs

- **Risk: a future cross-section rule is added that should *also* gate `config set`.** Mitigation: cross-section rules run inside `validate_and_set_value` against the post-mutation config, so any new rule automatically applies. The only thing tolerant load skips is the *pre-mutation* check, which is exactly the precondition we're loosening.
- **Risk: tolerant load silently masks corrupted state for `get`/`list`.** Mitigation: the stderr / JSON-`warnings` advisory makes the invalid state visible at every read; users (and CI tooling) cannot miss it.
- **Risk: someone re-introduces strict load in `set_config_value` during a future refactor.** Mitigation: the integration regression tests from Decision 7 guard every variant of this path; a regression turns red immediately.
- **Risk: the env-vs-file split changes observable behavior for users who relied on `config set` baking env values into the file.** Mitigation: this is documented behavior in the changelog. The previous behavior was a latent bug — env values are runtime-only and should not be persisted by a generic `set` command.
- **Trade-off: two load paths (`load_and_validate` and `load_for_repair`) add a small amount of code surface.** Acceptable: the two policies are genuinely different and naming them separately is clearer than a parameter.
- **Trade-off: the advisory in `config list` text mode goes to stderr, which scripting users piping `config list` into `grep` won't see by default.** Acceptable: stdout is reserved for the actual config dump so machine-readable consumers can keep their pipelines; users who want the warning visible can `2>&1` or use `--output json` and consult `warnings`.

## Open Questions

- Should `config get <KEY>` echo the advisory even when the *requested* key is itself valid in isolation? Current plan: yes, because the cross-section rule that fails may involve the requested key (e.g. `get ai.base_url` on the offending pairing). The advisory is cheap and keeps the behavior uniform.
- Should the advisory include a one-liner suggesting the canonical `config set` command to fix the issue? Current plan: no, because the validator's existing message already names the offending field and the docs link out from there. Auto-suggesting a fix risks suggesting the wrong one (set provider vs set URL) and is best left to the docs.
