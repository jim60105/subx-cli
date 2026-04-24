# Error Handling (Delta)

## ADDED Requirements

### Requirement: Sanitized upstream error messages

When an AI API returns an error response, the system SHALL truncate the error body to a maximum of 500 characters before including it in `SubXError`. The system SHALL strip any HTTP headers or request metadata from the error message to avoid leaking sensitive or excessive information to end users.

#### Scenario: long error body is truncated
- **WHEN** the AI API returns a 10 KiB error body
- **THEN** the `SubXError` message SHALL contain at most 500 characters of the body followed by `... (truncated)`

#### Scenario: short error body is preserved
- **WHEN** the AI API returns a 200-character error body
- **THEN** the full body SHALL be included in the error message

### Requirement: No sensitive data in error chains

Error types and messages SHALL NOT include API keys, authentication tokens, or full request/response URLs that contain query parameters. If a URL must be included, only the scheme, host, and path components SHALL be shown.

#### Scenario: error with URL strips query params
- **WHEN** an HTTP error includes a URL with query parameters
- **THEN** the error message SHALL show only `scheme://host/path`

#### Scenario: API key never in error message
- **WHEN** an AI service error occurs
- **THEN** the error message and its chain SHALL NOT contain any API key value
