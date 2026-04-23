# AI Provider Integration

## Purpose

Abstract subtitle-matching AI calls behind a provider trait so `openai`, `openrouter`, and `azure-openai` back ends are interchangeable, with shared prompt construction, response parsing, and retry logic. Implemented in `src/services/ai/` (`mod.rs`, `openai.rs`, `openrouter.rs`, `azure_openai.rs`, `prompts.rs`, `retry.rs`) and wired up by `ComponentFactory`.

## Requirements

### Requirement: Provider Trait Abstraction

The system SHALL define an `AIProvider` trait exposing `analyze_content(AnalysisRequest) -> MatchResult` and `verify_match(VerificationRequest) -> ConfidenceScore`, and all matching code SHALL depend on the trait rather than on any concrete client.

#### Scenario: Dependency injection in the match engine
- **GIVEN** a `MatchEngine` is constructed via `MatchEngine::new(ai_client, match_config)`
- **WHEN** the engine analyzes files
- **THEN** the engine SHALL invoke `ai_client.analyze_content` through the `AIProvider` trait without referencing a concrete provider type

### Requirement: Multi-Provider Selection

The system SHALL resolve the active AI provider from `config.ai.provider` using `ComponentFactory::create_ai_provider`, supporting at minimum the providers implemented under `src/services/ai/`: `openai`, `openrouter`, and `azure-openai`.

#### Scenario: OpenAI provider selected
- **GIVEN** `ai.provider = "openai"` and a valid `ai.api_key`
- **WHEN** `ComponentFactory::create_ai_provider` is called
- **THEN** it SHALL return a boxed `OpenAIClient` configured with the user's API key, model, temperature, and token limits

#### Scenario: Unknown provider rejected
- **GIVEN** `ai.provider` is set to an unrecognized value
- **WHEN** `ComponentFactory::create_ai_provider` is called
- **THEN** it SHALL return a configuration error rather than panic

### Requirement: Shared Prompt and Response Schema

The system SHALL build analysis and verification prompts in English via the shared `build_analysis_prompt_base` / `build_verification_prompt_base` functions, and SHALL require providers to respond with JSON that deserializes into `MatchResult` / `ConfidenceScore`.

#### Scenario: Prompt contains stable contract
- **GIVEN** an `AnalysisRequest` with video file IDs and subtitle file IDs
- **WHEN** `build_analysis_prompt_base` runs
- **THEN** the generated prompt SHALL instruct the model to respond with a JSON object containing `matches[].video_file_id`, `matches[].subtitle_file_id`, `matches[].confidence`, `confidence`, and `reasoning`

#### Scenario: Unparseable response yields a typed error
- **GIVEN** a provider returns text that cannot be parsed as the expected JSON schema
- **WHEN** `parse_match_result_base` runs
- **THEN** it SHALL return `SubXError::AiService` with a message indicating `AI response parsing failed`

### Requirement: Retry with Exponential Backoff

The system SHALL retry transient AI service failures using exponential backoff with a configurable maximum attempt count, base delay, backoff multiplier, and maximum delay cap.

#### Scenario: Retry succeeds on second attempt
- **GIVEN** a `RetryConfig { max_attempts: 3, ... }` and an operation that fails once and then succeeds
- **WHEN** `retry_with_backoff` is invoked
- **THEN** the operation SHALL be called exactly twice and the final result SHALL be the successful value

#### Scenario: Attempts capped by max_attempts
- **GIVEN** an operation that always fails and `max_attempts = 2`
- **WHEN** `retry_with_backoff` is invoked
- **THEN** the operation SHALL be attempted exactly twice and the last error SHALL be returned

