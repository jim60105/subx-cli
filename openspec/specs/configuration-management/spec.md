# Configuration Management

## Purpose

Hold the CLI half of the configuration agreement: the `config` subcommand operations (`get`/`set`/`list`), the repair path for strict-invalid configuration invoked from the CLI load flow, the workspace directory override, and the display-site obligation that sensitive values are masked wherever configuration is shown. Implemented in `src/commands/config_command.rs` and `src/cli/config_args.rs`. The layered schema, load-and-validate pipeline, `ConfigService` abstraction, validation rules, environment overrides, provider canonicalization, and permission enforcement are specified by the same-named `configuration-management` capability in `subx-core`.

## Requirements

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

### Requirement: Repair Path For Strict-Invalid Configuration

The system SHALL allow `subx config set <key> <value>`, `subx config get <key>`, and `subx config list` to operate on a `config.toml` whose contents fail cross-section (strict) validation, so long as the file parses as TOML and each individual field passes field-level validation.

The tolerant load these three subcommands use, its exclusion of environment-variable overlays, the post-mutation strict validation performed before the file is written, the cache invariant, and the unchanged strict load path used by every other entry point are specified by the `configuration-management` capability's *Tolerant Configuration Load Path* requirement in `subx-core`. This requirement governs only what the three subcommands do with the result.

For `subx config set`, when the post-mutation configuration fails cross-section validation the command SHALL fail, SHALL NOT print a success confirmation, and the on-disk file SHALL remain unchanged. When it is strict-valid the command SHALL report success.

For `subx config get` and `subx config list`, the command SHALL produce its normal output and SHALL additionally surface a non-fatal warning when the on-disk configuration is strict-invalid. In text mode the warning SHALL be a single line on stderr beginning with `warning: configuration is currently invalid:` followed by the validator's message. When the global `--output json` flag is in effect, the warning SHALL be appended to the existing `Envelope::warnings` array (a `Vec<String>` per the machine-readable-output capability); when the configuration is strict-valid the `warnings` field SHALL remain absent from the JSON document, matching today's shape. The exit code SHALL remain `0` for successful reads even when the warning is emitted.

The advisory SHALL track on-disk state, not the environment-merged effective view, which follows from the tolerant load's exclusion of environment overlays.

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

#### Scenario: Advisory reflects file state, not env-merged state

- **GIVEN** `~/.config/subx/config.toml` is strict-valid (e.g. `ai.provider = "local"` with `ai.base_url = "http://localhost:1234/v1"`) but environment variables would create a strict-invalid effective view (e.g. `SUBX_AI_PROVIDER=openai` is exported)
- **WHEN** the user runs `subx config get ai.base_url`
- **THEN** the command SHALL exit with status `0`, stdout SHALL contain the file's `ai.base_url`, and stderr SHALL NOT contain an "configuration is currently invalid" advisory (because the file itself is valid; advisories track on-disk state)
### Requirement: Workspace Directory Override

The system SHALL, at CLI startup, change the process's current working directory to a workspace directory when one is configured, preferring the `SUBX_WORKSPACE` environment variable over `general.workspace` in configuration. A failure to change directory SHALL surface as a `SubXError::CommandExecution` error; an empty `general.workspace` SHALL leave the working directory unchanged. Implemented in `src/cli/mod.rs` around the CLI-run entry point.

#### Scenario: Environment variable takes precedence
- **GIVEN** `SUBX_WORKSPACE` is set to `/path/A` and `general.workspace` is set to `/path/B`
- **WHEN** the CLI initializes before dispatching a subcommand
- **THEN** the process working directory SHALL be changed to `/path/A`

#### Scenario: Configured workspace is applied when env var is unset
- **GIVEN** `SUBX_WORKSPACE` is not set and `general.workspace` is `/path/B`
- **WHEN** the CLI initializes
- **THEN** the process working directory SHALL be changed to `/path/B`

#### Scenario: Empty configured workspace leaves cwd unchanged
- **GIVEN** `SUBX_WORKSPACE` is not set and `general.workspace` is an empty path
- **WHEN** the CLI initializes
- **THEN** the process working directory SHALL NOT be changed by the workspace logic

### Requirement: Sensitive value masking in config display

The `config list`, `config get`, and `config set` confirmation output SHALL mask values for keys matching `api_key`, `token`, or `secret` (case-insensitive substring). The masked format SHALL be `****<last 4 chars>`, or `****` if the value is 4 characters or fewer.

#### Scenario: config get masks api_key
- **WHEN** the user runs `config get ai.api_key`
- **THEN** the output SHALL show `****<last4>` instead of the full value

#### Scenario: non-sensitive key shown in full
- **WHEN** the user runs `config get ai.provider`
- **THEN** the full value SHALL be displayed normally
