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
