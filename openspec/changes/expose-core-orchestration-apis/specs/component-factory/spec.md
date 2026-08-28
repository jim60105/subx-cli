## MODIFIED Requirements

### Requirement: Match Engine Creation

`ComponentFactory` SHALL expose the `MatchConfig` its loaded `Config` implies, and SHALL be able to build a `MatchEngine` from a caller-supplied `MatchConfig`, so that no caller has to hand-build the struct in order to choose a relocation mode or a confidence threshold.

`ComponentFactory::match_config` (`subx-core/src/core/factory.rs`) SHALL return a `MatchConfig` derived from the loaded `Config` as follows:

- `max_sample_length` from `config.ai.max_sample_length`
- `ai_model` from `config.ai.model`
- `backup_enabled` from `config.general.backup_enabled`
- `max_subtitle_bytes` from `config.general.max_subtitle_bytes`
- `enable_content_analysis` set to `true`
- `confidence_threshold` set to the default `0.8`
- `relocation_mode` set to `FileRelocationMode::None`
- `conflict_resolution` set to `ConflictResolution::AutoRename`

The last four are the caller-controlled defaults: a caller SHALL be able to overwrite any of them on the returned value before using it, because `MatchConfig`'s fields are public.

`ComponentFactory::create_match_engine_with(config: MatchConfig)` SHALL build a `MatchEngine` by (1) constructing an AI provider via `create_ai_provider`, (2) using the supplied `config` unmodified, and (3) injecting both into `MatchEngine::new`. It SHALL propagate the factory's reporter into the produced engine on the same terms as every other `create_*` method.

`ComponentFactory::create_match_engine` SHALL keep its existing signature and SHALL be defined as `self.create_match_engine_with(self.match_config())`, so its behaviour is unchanged in every field.

`ComponentFactory::new`'s signature SHALL NOT change.

`MatchConfig` SHALL NOT be made `#[non_exhaustive]` by this requirement: it would break every existing struct-literal construction. `match_config` is the migration path that makes such a change cheap later, and callers SHALL prefer it over a literal.

#### Scenario: Match engine wired with AI provider and config

- **GIVEN** a factory built from a valid `TestConfigService`
- **WHEN** `create_match_engine` is called
- **THEN** it SHALL return `Ok(MatchEngine)` whose `MatchConfig.max_sample_length` equals `config.ai.max_sample_length` and whose `MatchConfig.ai_model` equals `config.ai.model`

#### Scenario: AI provider failure bubbles up

- **GIVEN** a factory whose `ai.api_key` is empty
- **WHEN** `create_match_engine` is called
- **THEN** it SHALL return the same configuration error that `create_ai_provider` would return, and SHALL NOT construct a partial engine

#### Scenario: Exposed config matches the hardcoded defaults

- **GIVEN** a factory built from a valid `TestConfigService`
- **WHEN** `match_config()` is called
- **THEN** the returned value SHALL carry `confidence_threshold == 0.8`, `enable_content_analysis == true`, `relocation_mode == FileRelocationMode::None` and `conflict_resolution == ConflictResolution::AutoRename`, and its four config-derived fields SHALL equal the corresponding `Config` values

#### Scenario: Caller-chosen relocation mode reaches the engine

- **GIVEN** a factory built from a valid `TestConfigService`, and `let mut config = factory.match_config(); config.relocation_mode = FileRelocationMode::Copy;`
- **WHEN** `create_match_engine_with(config)` is called
- **THEN** it SHALL return `Ok(MatchEngine)` whose `MatchConfig.relocation_mode` is `FileRelocationMode::Copy`, and the engine SHALL plan copy relocations rather than in-place renames

#### Scenario: Factory reporter reaches an engine built from a supplied config

- **GIVEN** a `ComponentFactory::new(config_service)?.with_reporter(reporter)`
- **WHEN** `create_match_engine_with(factory.match_config())` is called and the produced engine emits a diagnostic
- **THEN** the supplied reporter SHALL receive it, on the same terms as an engine produced by `create_match_engine`
