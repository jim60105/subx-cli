# Secrets Protection

## Purpose

Keep API keys, tokens, and other sensitive configuration values out of
terminal output, `Debug` formatting, log files, and disk paths that
third parties could read. This includes masking secrets in CLI output,
redacting them in `Debug` impls, restricting on-disk permissions, and
warning when secrets would be transmitted over plaintext HTTP.

## Requirements

### Requirement: Mask sensitive config values in CLI output

The system SHALL mask values for config keys matching `api_key`, `token`, or `secret` (case-insensitive) when displaying them via `config set`, `config list`, or `config get`. Masked format: `****<last 4 chars>` (or `****` if value is ≤4 chars).

#### Scenario: config set echoes masked value

- **WHEN** user runs `config set ai.api_key "sk-abc123def456"`
- **THEN** stdout shows `****f456` not the full key

#### Scenario: config list masks api_key

- **WHEN** user runs `config list`
- **THEN** the `api_key` field displays `****<last4>` instead of the plaintext value

#### Scenario: short secret is fully masked

- **WHEN** the api_key value is 3 characters
- **THEN** the display shows `****`

### Requirement: Redact API keys in Debug output

All structs holding API keys (OpenAIClient, OpenRouterClient, AzureOpenAIClient, AIConfig) SHALL NOT expose the `api_key` field via `Debug` formatting. The Debug output MUST show `[REDACTED]` for the api_key field.

#### Scenario: Debug format redacts key

- **WHEN** `format!("{:?}", client)` is called on an AI client struct
- **THEN** the output contains `[REDACTED]` instead of the actual API key

### Requirement: Restrict config file permissions

On Unix systems, the system SHALL set file permissions to `0o600` on the config file after every write operation. The config directory SHALL be set to `0o700`.

#### Scenario: new config file has restricted permissions

- **WHEN** `config set` writes a new config file
- **THEN** the file permission mode is `0o600` on Unix

#### Scenario: existing config file permissions corrected

- **WHEN** `save_config` writes to an existing config file
- **THEN** the file permission mode is set to `0o600`

### Requirement: Warn on insecure HTTP endpoint

When an AI client is constructed with a `http://` (non-HTTPS) base URL and a non-empty API key, the system SHALL emit a warning log message indicating that the API key will be transmitted in plaintext.

#### Scenario: HTTP endpoint with key triggers warning

- **WHEN** the base URL is `http://example.com` and api_key is set
- **THEN** a warning is logged about plaintext transmission

#### Scenario: HTTPS endpoint produces no warning

- **WHEN** the base URL is `https://api.openai.com` and api_key is set
- **THEN** no insecure-transport warning is logged

#### Scenario: HTTP localhost produces no warning

- **WHEN** the base URL is `http://127.0.0.1:8080` and api_key is set
- **THEN** no insecure-transport warning is logged (localhost is acceptable for development)
