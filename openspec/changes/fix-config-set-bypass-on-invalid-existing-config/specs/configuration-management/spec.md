## ADDED Requirements

### Requirement: Repair Path For Strict-Invalid Configuration

The system SHALL allow `subx config set <key> <value>`, `subx config get <key>`, and `subx config list` to operate on a `config.toml` whose contents fail cross-section (strict) validation, so long as the file parses as TOML and each individual field passes field-level validation.

For these three subcommands the system SHALL load the configuration *from the file only*, without applying any environment-variable overlays, so that repair operations always reflect and modify the on-disk state rather than a transient env-merged view.

For `subx config set`, the system SHALL still strict-validate the *post-mutation* configuration before writing the file: if the post-mutation configuration also fails cross-section validation, the command SHALL fail and the on-disk file SHALL remain unchanged. If the post-mutation configuration is strict-valid, the command SHALL write the file and update the in-memory cache.

For `subx config get` and `subx config list`, the system SHALL produce its normal output and SHALL additionally surface a non-fatal warning when the on-disk configuration is strict-invalid. In text mode the warning SHALL be a single line on stderr beginning with `warning: configuration is currently invalid:` followed by the validator's message. When the global `--output json` flag is in effect, the warning SHALL be appended to the existing `Envelope::warnings` array (a `Vec<String>` per the machine-readable-output capability); when the configuration is strict-valid the `warnings` field SHALL remain absent from the JSON document, matching today's shape. The exit code SHALL remain `0` for successful reads even when the warning is emitted.

The strict load path used by `get_config`, `reload`, and every non-`config` command SHALL remain unchanged: those entry points SHALL continue to fail with the existing error when the on-disk configuration is strict-invalid, so that strict invariants assumed by command-execution sites are not weakened.

The in-memory configuration cache SHALL only ever hold strict-valid configurations: tolerant loads SHALL NOT populate the cache, and a failed `config set` SHALL NOT alter the cache.

#### Scenario: Repair via provider switch from a strict-invalid pairing

- **GIVEN** `~/.config/subx/config.toml` contains `ai.provider = "openai"` and `ai.base_url = "http://localhost:1234/v1"`, which fails cross-section validation because hosted providers require `https://`
- **WHEN** the user runs `subx config set ai.provider local`
- **THEN** the command SHALL exit with status `0`, the on-disk file SHALL contain `ai.provider = "local"`, the existing `ai.base_url` and `ai.api_key` values SHALL be preserved verbatim, and the resulting file SHALL pass strict cross-section validation

#### Scenario: Repair via base_url switch from a strict-invalid pairing

- **GIVEN** `~/.config/subx/config.toml` contains `ai.provider = "openai"` and `ai.base_url = "http://localhost:1234/v1"`
- **WHEN** the user runs `subx config set ai.base_url https://api.openai.com/v1`
- **THEN** the command SHALL exit with status `0`, the on-disk file SHALL contain the new `https://` URL, the existing `ai.provider` value SHALL be preserved, and the resulting file SHALL pass strict cross-section validation

#### Scenario: Non-repair edit on a strict-invalid file is rejected

- **GIVEN** `~/.config/subx/config.toml` contains `ai.provider = "openai"` and `ai.base_url = "http://localhost:1234/v1"`
- **WHEN** the user runs `subx config set general.backup_enabled true` (an unrelated key whose mutation does not heal the cross-section error)
- **THEN** the command SHALL fail with the standard cross-section error naming the offending `ai.base_url` scheme, the on-disk file SHALL remain byte-identical to its prior contents, and the in-memory cache SHALL NOT be populated

#### Scenario: Field-level invalid new value is still rejected on a strict-invalid file

- **GIVEN** `~/.config/subx/config.toml` is strict-invalid for any reason
- **WHEN** the user runs `subx config set sync.max_offset_seconds -5` (a value the field validator rejects)
- **THEN** the command SHALL fail with the field-level error explaining the acceptable range, and the on-disk file SHALL remain unchanged

#### Scenario: `config get` on a strict-invalid file emits the value plus an advisory

- **GIVEN** `~/.config/subx/config.toml` is strict-invalid because of the `provider=openai + http://` pairing
- **WHEN** the user runs `subx config get ai.base_url`
- **THEN** the command SHALL exit with status `0`, stdout SHALL contain `http://localhost:1234/v1`, and stderr SHALL contain a single-line advisory beginning with `warning: configuration is currently invalid:` followed by the validator's message

#### Scenario: `config list` JSON output on a strict-invalid file populates `warnings`

- **GIVEN** `~/.config/subx/config.toml` is strict-invalid
- **WHEN** the user runs `subx-cli --output json config list`
- **THEN** the command SHALL exit with status `0`, stdout SHALL be valid JSON, and the JSON SHALL include a top-level `warnings` array containing at least one non-empty string reproducing the validator's error

#### Scenario: `config list` JSON output on a strict-valid file omits `warnings`

- **GIVEN** `~/.config/subx/config.toml` is strict-valid
- **WHEN** the user runs `subx-cli --output json config list`
- **THEN** the JSON output SHALL NOT include a `warnings` field (or SHALL include `warnings: null`, matching today's `Option::is_none` serialization), and the document SHALL otherwise be byte-equivalent to the pre-change output for the same file

#### Scenario: Non-config command still rejects strict-invalid configuration

- **GIVEN** `~/.config/subx/config.toml` is strict-invalid
- **WHEN** the user runs any command other than `config set`, `config get`, `config list`, or `config reset` (for example `subx match` or `subx sync`)
- **THEN** the command SHALL fail at configuration load with the existing strict-validation error, the same error message users see today, and SHALL NOT proceed to the command body

#### Scenario: Cache invariant preserved after failed repair attempt

- **GIVEN** `~/.config/subx/config.toml` is strict-invalid and no `Config` is currently cached
- **WHEN** the user runs `subx config set` with a key/value that does not heal the cross-section error
- **THEN** the command SHALL fail (per the "non-repair edit" scenario above) and the in-memory configuration cache SHALL remain empty, so that the next invocation of any other command SHALL still reload and re-validate from disk and SHALL still produce the strict error

#### Scenario: TOML parse failure is still a hard error

- **GIVEN** `~/.config/subx/config.toml` is not valid TOML (for example, an unterminated string)
- **WHEN** the user runs `subx config set ai.provider local`
- **THEN** the command SHALL fail with a parse error, the on-disk file SHALL remain unchanged, and the user SHALL still need to run `subx config reset` or fix the file manually to recover (this scenario lies outside the repair path because the file cannot be loaded into a `Config` struct at all)

#### Scenario: Repair does not bake env-only values into the file

- **GIVEN** `~/.config/subx/config.toml` is strict-invalid because of the `provider=openai + http://` pairing, the file's `ai.api_key` is `"sk-test"`, and the environment has `OPENAI_API_KEY="sk-fromenv"` exported
- **WHEN** the user runs `subx config set ai.provider local`
- **THEN** the command SHALL exit with status `0`, the on-disk file SHALL contain `ai.provider = "local"`, and the on-disk file's `ai.api_key` SHALL remain `"sk-test"` (NOT `"sk-fromenv"`)

#### Scenario: Advisory reflects file state, not env-merged state

- **GIVEN** `~/.config/subx/config.toml` is strict-valid (e.g. `ai.provider = "local"` with `ai.base_url = "http://localhost:1234/v1"`) but environment variables would create a strict-invalid effective view (e.g. `SUBX_AI_PROVIDER=openai` is exported)
- **WHEN** the user runs `subx config get ai.base_url`
- **THEN** the command SHALL exit with status `0`, stdout SHALL contain the file's `ai.base_url`, and stderr SHALL NOT contain an "configuration is currently invalid" advisory (because the file itself is valid; advisories track on-disk state)

#### Scenario: Field-level malformed value in file fails the tolerant load

- **GIVEN** `~/.config/subx/config.toml` is parseable TOML but contains a syntactically broken individual field (for example `ai.base_url = "not a url"`)
- **WHEN** the user runs `subx config set ai.provider local`
- **THEN** the command SHALL fail with a field-level error identifying the malformed field, and the on-disk file SHALL remain unchanged (the tolerant load SHALL NOT silently substitute defaults for malformed individual fields)

## MODIFIED Requirements

### Requirement: `config` Subcommand Operations

The system SHALL provide `subx config` subcommands `set <key> <value>`, `get <key>`, `list`, and `reset`, where keys are expressed in dot-notation (for example `ai.provider`, `sync.max_offset_seconds`).

The `set`, `get`, and `list` subcommands SHALL function on configurations that fail cross-section (strict) validation as described in the requirement "Repair Path For Strict-Invalid Configuration", so that users can inspect and repair such configurations without resorting to `reset` or manual file editing. The `reset` subcommand SHALL continue to overwrite the configuration file with `Config::default()` regardless of the prior file's validity.

#### Scenario: Get a configuration value

- **GIVEN** `ai.provider` is set to `openai`
- **WHEN** the user runs `subx config get ai.provider`
- **THEN** the command SHALL print `openai`

#### Scenario: Set a typed configuration value

- **GIVEN** the user runs `subx config set sync.max_offset_seconds 15.0`
- **WHEN** the command completes
- **THEN** the persisted configuration SHALL contain `sync.max_offset_seconds = 15.0` as a floating-point value

#### Scenario: Reset restores defaults

- **GIVEN** any modified configuration
- **WHEN** the user runs `subx config reset`
- **THEN** the configuration SHALL be restored to the values produced by `Config::default()`

#### Scenario: Reset works even on strict-invalid configuration

- **GIVEN** the on-disk configuration is strict-invalid (for example because of a `provider=openai + http://base_url` pairing)
- **WHEN** the user runs `subx config reset`
- **THEN** the command SHALL succeed, the on-disk file SHALL be replaced with `Config::default()`, and the resulting file SHALL pass strict validation
