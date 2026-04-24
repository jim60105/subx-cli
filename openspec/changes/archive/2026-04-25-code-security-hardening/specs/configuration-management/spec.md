# Configuration Management (Delta)

## ADDED Requirements

### Requirement: Config file permissions enforcement

On Unix systems, the config file SHALL be created with restrictive permissions from the start — not fixed up after creation. The config directory SHALL be created with mode `0o700` before the config file is written. The config file SHALL be opened with `OpenOptionsExt::mode(0o600)` (or created via a temp-file with `0o600` permissions then atomically renamed) so that it is never world-readable at any point.

#### Scenario: save_config creates file with restrictive permissions
- **WHEN** `save_config()` writes a new config file on Unix
- **THEN** the file is created with permission mode `0o600` from the start (never temporarily world-readable)

#### Scenario: config directory has restrictive permissions
- **WHEN** the config directory is created
- **THEN** it is created with permission mode `0o700`

#### Scenario: existing config file permissions corrected on write
- **WHEN** `save_config()` writes to an existing config file on Unix
- **THEN** the file permissions are set to `0o600` after the write

### Requirement: Sensitive value masking in config display

The `config list`, `config get`, and `config set` confirmation output SHALL mask values for keys matching `api_key`, `token`, or `secret` (case-insensitive substring). The masked format SHALL be `****<last 4 chars>`, or `****` if the value is 4 characters or fewer.

#### Scenario: config get masks api_key
- **WHEN** the user runs `config get ai.api_key`
- **THEN** the output SHALL show `****<last4>` instead of the full value

#### Scenario: non-sensitive key shown in full
- **WHEN** the user runs `config get ai.provider`
- **THEN** the full value SHALL be displayed normally
