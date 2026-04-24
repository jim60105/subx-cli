# Component Factory

## Purpose

Provide a centralized dependency-injection factory, `ComponentFactory`, that constructs AI providers, match engines, file managers, VAD detectors, VAD sync detectors, and audio processors from a `ConfigService`. The factory is the single authoritative wiring point for runtime services so that commands and tests never reach for global or static configuration. Implemented in `src/core/factory.rs` and consumed by `src/commands/match_command.rs`, with sibling commands (`src/commands/sync_command.rs`, `src/commands/convert_command.rs`) receiving a `&dyn ConfigService` through the same dependency-injection discipline.

## Requirements

### Requirement: ConfigService-Driven Construction

`ComponentFactory::new` SHALL accept a `&dyn ConfigService` reference, load configuration via `ConfigService::get_config`, and fail fast with a configuration error if loading fails. The factory SHALL NOT read environment variables, files, or any global state directly; all configuration SHALL originate from the injected `ConfigService`.

#### Scenario: Factory constructed from a config service

- **GIVEN** a `ConfigService` implementation (e.g., `ProductionConfigService` or `TestConfigService`)
- **WHEN** `ComponentFactory::new(&config_service)` is called
- **THEN** the factory SHALL cache the loaded `Config` and expose it via `ComponentFactory::config`
- **AND** all subsequent `create_*` methods SHALL use that cached configuration

#### Scenario: Configuration loading failure propagates

- **GIVEN** a `ConfigService` whose `get_config` returns an error
- **WHEN** `ComponentFactory::new` is invoked
- **THEN** the call SHALL return that error without constructing any component

### Requirement: AI Provider Creation

`ComponentFactory::create_ai_provider` SHALL return a `Box<dyn AIProvider>` selected from `config.ai.provider`, dispatching to the providers implemented under `src/services/ai/`: `openai`, `openrouter`, and `azure-openai`. Unrecognized provider strings SHALL yield a configuration error (never a panic) and the error message SHALL include the substring `Unsupported AI provider`.

#### Scenario: Known provider resolved

- **GIVEN** `config.ai.provider = "openrouter"` with a non-empty API key, model, valid temperature, and non-zero `max_tokens`
- **WHEN** `create_ai_provider` is called
- **THEN** it SHALL return an `OpenRouterClient` boxed as `dyn AIProvider`

#### Scenario: Unknown provider rejected

- **GIVEN** `config.ai.provider = "unsupported-provider"`
- **WHEN** `create_ai_provider` is called
- **THEN** it SHALL return `Err(SubXError::Config(..))` whose message contains `Unsupported AI provider`

### Requirement: Pre-Construction Configuration Validation

Before constructing any AI provider that requires external resources, the factory SHALL validate the relevant `AIConfig` fields via `validate_ai_config` in `src/core/factory.rs`. Validation SHALL reject an empty or whitespace `ai.api_key`, an empty or whitespace `ai.model`, an `ai.temperature` outside the inclusive range `[0.0, 2.0]`, or `ai.max_tokens == 0`, each producing a `SubXError::Config` with a descriptive message.

#### Scenario: Missing API key rejected

- **GIVEN** `config.ai.provider = "openai"` and `config.ai.api_key = Some("")` (or `None`)
- **WHEN** `create_ai_provider` is called
- **THEN** it SHALL return a configuration error whose message contains `API key is required`
- **AND** no HTTP client SHALL be constructed

#### Scenario: Empty model rejected

- **GIVEN** `config.ai.provider = "openai"`, a non-empty `api_key`, and `config.ai.model = ""`
- **WHEN** `create_ai_provider` is called
- **THEN** it SHALL return a configuration error whose message contains `AI model is required`

#### Scenario: Temperature out of range rejected

- **GIVEN** a valid provider and `config.ai.temperature = 2.5`
- **WHEN** `create_ai_provider` is called
- **THEN** it SHALL return a configuration error indicating the temperature must be in `[0.0, 2.0]`

### Requirement: Match Engine Creation

`ComponentFactory::create_match_engine` SHALL build a `MatchEngine` by (1) constructing an AI provider via `create_ai_provider`, (2) deriving a `MatchConfig` from the loaded `Config` (using `ai.max_sample_length`, `ai.model`, `general.backup_enabled`, a default confidence threshold of `0.8`, `FileRelocationMode::None`, and `ConflictResolution::AutoRename`), and (3) injecting both into `MatchEngine::new`.

#### Scenario: Match engine wired with AI provider and config

- **GIVEN** a factory built from a valid `TestConfigService`
- **WHEN** `create_match_engine` is called
- **THEN** it SHALL return `Ok(MatchEngine)` whose `MatchConfig.max_sample_length` equals `config.ai.max_sample_length` and whose `MatchConfig.ai_model` equals `config.ai.model`

#### Scenario: AI provider failure bubbles up

- **GIVEN** a factory whose `ai.api_key` is empty
- **WHEN** `create_match_engine` is called
- **THEN** it SHALL return the same configuration error that `create_ai_provider` would return, and SHALL NOT construct a partial engine

### Requirement: VAD and Audio Component Creation

The factory SHALL expose `create_vad_sync_detector`, `create_vad_detector`, and `create_audio_processor`. `create_vad_sync_detector` and `create_vad_detector` SHALL construct their respective services from a clone of `config.sync.vad`. `create_audio_processor` SHALL construct a `VadAudioProcessor` with no external configuration.

#### Scenario: VAD sync detector built from sync config

- **GIVEN** a factory built from a `TestConfigService` with default sync settings
- **WHEN** `create_vad_sync_detector` is called
- **THEN** it SHALL return `Ok(VadSyncDetector)` constructed from `config.sync.vad`

#### Scenario: Local VAD detector and audio processor available

- **GIVEN** the same factory
- **WHEN** `create_vad_detector` and `create_audio_processor` are each called
- **THEN** both SHALL return `Ok(_)` without reading any global state

### Requirement: Commands Consume Services via Dependency Injection

Command entry points under `src/commands/` SHALL receive a `&dyn ConfigService` (or an `Arc<dyn ConfigService>` wrapper) and SHALL obtain concrete services either directly from that service or through `ComponentFactory::new(config_service)`. Command code SHALL NOT invoke any global configuration accessor or static singleton.

#### Scenario: Match command uses the factory

- **GIVEN** `subx match` is invoked with a `&dyn ConfigService`
- **WHEN** `match_command::execute` runs
- **THEN** it SHALL build a `ComponentFactory` from that service and obtain its AI provider via `factory.create_ai_provider()` (see `src/commands/match_command.rs:176-213`)

#### Scenario: Sync and convert commands receive injected config

- **GIVEN** `sync_command::execute` or `convert_command::execute` is called
- **WHEN** the function loads configuration
- **THEN** it SHALL call `config_service.get_config()` on the injected service (see `src/commands/sync_command.rs:168-173`, `src/commands/convert_command.rs:203-205`) rather than any global accessor

### Requirement: Tests Use TestConfigService via TestConfigBuilder

Tests that exercise factory-created components SHALL construct configuration through `TestConfigService` / `TestConfigBuilder` rather than mutating process-wide state, as mandated by `docs/testing-guidelines.md`. Integration tests for dependency injection (e.g., `tests/dependency_injection_integration_tests.rs`, `tests/openrouter_integration_tests.rs`, `tests/azure_openai_api_integration_tests.rs`) SHALL construct the factory using a `TestConfigService` instance.

#### Scenario: Unit test builds factory from TestConfigService

- **GIVEN** a unit test in `src/core/factory.rs`
- **WHEN** the test needs a factory
- **THEN** it SHALL instantiate `TestConfigService::default()` (optionally adjusted via `set_ai_settings_and_key` or `config_mut`) and pass it to `ComponentFactory::new`
- **AND** the test SHALL NOT set environment variables, write config files, or mutate global statics

#### Scenario: Integration test wires provider via factory

- **GIVEN** an integration test validating OpenRouter or Azure OpenAI wiring
- **WHEN** the test constructs its subject
- **THEN** it SHALL do so through `ComponentFactory::new(&test_config_service)` followed by `factory.create_ai_provider()` (see `tests/openrouter_integration_tests.rs:13-24`, `tests/azure_openai_api_integration_tests.rs:186`)
