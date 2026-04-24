## ADDED Requirements

### Requirement: Subtitle file size limit

The system SHALL check `fs::metadata(path)?.len()` before reading any subtitle file into memory. If the file exceeds the configured `general.max_subtitle_bytes` limit (default: 50 MiB), the system SHALL return an error and skip that file.

#### Scenario: oversized subtitle is rejected

- **WHEN** a subtitle file is 100 MiB and the limit is 50 MiB
- **THEN** the system returns a descriptive error and does not read the file

#### Scenario: normal subtitle is processed

- **WHEN** a subtitle file is 500 KiB
- **THEN** the file is read and processed normally

#### Scenario: custom limit is respected

- **WHEN** the user sets `general.max_subtitle_bytes` to `104857600` (100 MiB)
- **THEN** files up to 100 MiB are accepted

### Requirement: Audio file size limit

The system SHALL check file size before decoding audio. If the file exceeds `general.max_audio_bytes` (default: 2 GiB), the system SHALL return an error.

#### Scenario: oversized audio is rejected

- **WHEN** an audio file is 5 GiB and the limit is 2 GiB
- **THEN** the system returns an error without attempting to decode

#### Scenario: normal audio is processed

- **WHEN** an audio file is 500 MiB
- **THEN** decoding proceeds normally

### Requirement: AI response body size limit

Before calling `.text()` on an AI API response, the system SHALL check `Response::content_length()`. If the content length exceeds 10 MiB, the system SHALL return an error. If `content_length()` returns `None`, the system SHALL read the body in chunks up to 10 MiB and return an error if the limit is exceeded.

#### Scenario: oversized AI response is rejected

- **WHEN** the AI API returns a response with Content-Length: 50 MiB
- **THEN** the system returns an error without reading the body

#### Scenario: normal AI response is processed

- **WHEN** the AI API returns a 2 KiB JSON response
- **THEN** the response is read and parsed normally
