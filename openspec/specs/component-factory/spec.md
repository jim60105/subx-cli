# Component Factory

## Purpose

Hold the CLI half of the component-factory agreement: commands consuming `ComponentFactory` services through dependency injection — never global or static configuration — and the test-suite's `TestConfigService` usage. Implemented in `src/commands/match_command.rs`, `src/commands/sync_command.rs`, and `src/commands/convert_command.rs`; the factory itself lives in `subx-core:src/core/factory.rs` and is specified by the same-named `component-factory` capability in `subx-core`.

## Requirements

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

