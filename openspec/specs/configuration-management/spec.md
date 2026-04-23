# Configuration Management

## Purpose

Load, validate, expose, and mutate application settings (AI, formats, sync, general, parallel) through a dependency-injected `ConfigService` abstraction, with both a production file-backed implementation and a test implementation. Implemented in `src/config/` (notably `mod.rs`, `service.rs`, `validator.rs`, `field_validator.rs`, `environment.rs`).

## Requirements

### Requirement: Unified Configuration Schema

The system SHALL expose a single `Config` structure aggregating `AIConfig`, `FormatsConfig`, `SyncConfig`, `GeneralConfig`, and `ParallelConfig`, serializable to and deserializable from TOML.

#### Scenario: Default configuration is valid
- **GIVEN** a fresh `Config::default()` value
- **WHEN** the defaults are inspected
- **THEN** `config.ai.provider` SHALL equal `"openai"` and `config.formats.default_output` SHALL equal `"srt"`

### Requirement: Configuration Service Abstraction

The system SHALL access all configuration through the `ConfigService` trait rather than global state; production code SHALL use `ProductionConfigService` and tests SHALL use `TestConfigService` (built via `TestConfigBuilder`).

#### Scenario: Command receives an injected service
- **GIVEN** a command dispatcher invocation
- **WHEN** any subcommand executes
- **THEN** the command handler SHALL obtain configuration by calling `config_service.get_config()` on the injected service rather than reading a global or static

### Requirement: `config` Subcommand Operations

The system SHALL provide `subx config` subcommands `set <key> <value>`, `get <key>`, `list`, and `reset`, where keys are expressed in dot-notation (for example `ai.provider`, `sync.max_offset_seconds`).

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

### Requirement: Value Validation

The system SHALL validate configuration values at the moment they are set, rejecting out-of-range numerics, empty required strings, and values of the wrong type.

#### Scenario: Invalid value is rejected
- **GIVEN** the user runs `subx config set sync.max_offset_seconds -5`
- **WHEN** the field validator runs
- **THEN** the command SHALL fail with an error explaining the acceptable range and the persisted configuration SHALL remain unchanged

#### Scenario: Enum field rejects out-of-set values
- **GIVEN** the user runs `subx config set sync.default_method whisper`
- **WHEN** the field validator runs
- **THEN** the command SHALL fail because `sync.default_method` only accepts the values `auto`, `vad`, or `manual`, and the persisted configuration SHALL remain unchanged

### Requirement: Boolean Value Flexibility

The system SHALL accept common boolean aliases (`true`/`false`, `1`/`0`, `yes`/`no`, `on`/`off`, `enabled`/`disabled`) when setting boolean-typed keys, treating them as equivalent to the canonical `true`/`false` values.

#### Scenario: Alternative boolean syntax
- **GIVEN** the user runs `subx config set general.backup_enabled yes`
- **WHEN** the command completes
- **THEN** `general.backup_enabled` SHALL be persisted as `true`

### Requirement: AI Environment Variable Overrides

The system SHALL recognize a fixed set of `SUBX_`-prefixed environment variables that map to AI configuration fields, and SHALL apply them over file-backed configuration when present. Implemented in `src/config/service.rs:222-251` (the partial-load path over the `config` crate's `Environment` source). The supported variables are exactly:

- `SUBX_AI_APIKEY` → `ai.api_key`
- `SUBX_AI_PROVIDER` → `ai.provider`
- `SUBX_AI_MODEL` → `ai.model`
- `SUBX_AI_BASE_URL` → `ai.base_url`

#### Scenario: AI provider overridden by environment
- **GIVEN** the configuration file sets `ai.provider = "openai"` and the process environment has `SUBX_AI_PROVIDER=openrouter`
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** `config.ai.provider` SHALL equal `"openrouter"`

### Requirement: Custom Configuration File Path

The system SHALL honor the `SUBX_CONFIG_PATH` environment variable as an override for the configuration file location used by `ProductionConfigService`, instead of the default platform config directory.

#### Scenario: Custom path loaded
- **GIVEN** `SUBX_CONFIG_PATH` points to an existing TOML file with valid settings
- **WHEN** `ProductionConfigService` is constructed
- **THEN** it SHALL read configuration from the custom path rather than from the default `dirs::config_dir()` location

### Requirement: Legacy Sync Configuration Rejected

The system SHALL reject legacy `[sync]` TOML that lacks the new required fields (such as `default_method`), failing to deserialize into `Config` rather than silently applying partial defaults. Implemented by the `SyncConfig` schema and exercised by `tests/config_migration_tests.rs`.

#### Scenario: Old sync schema fails parsing
- **GIVEN** a TOML document with `[sync] max_offset_seconds = 10.0` and `correlation_threshold = 0.8` but no `default_method`
- **WHEN** the document is deserialized into `Config`
- **THEN** deserialization SHALL fail with a parse error

### Requirement: Config Service Reload

`ProductionConfigService` SHALL expose a `reload()` operation that re-reads configuration (file and environment) and SHALL produce a subsequent `get_config()` result consistent with the latest on-disk and environment state.

#### Scenario: Reload returns fresh configuration
- **GIVEN** a `ProductionConfigService` has been constructed and has returned a configuration once
- **WHEN** `service.reload()` is called and then `service.get_config()` is called again
- **THEN** the second call SHALL succeed and SHALL reflect any applicable on-disk or environment changes without restarting the process
