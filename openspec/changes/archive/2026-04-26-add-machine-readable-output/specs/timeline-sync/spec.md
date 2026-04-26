## ADDED Requirements

### Requirement: Sync Command Emits Structured JSON Payload

When the `sync` command runs with the global output mode set to `json`, it SHALL emit a single JSON envelope on stdout (per the `machine-readable-output` capability) and SHALL NOT print free-form progress chatter or status symbols on stdout. The envelope's `data` object SHALL contain:

- `method` (string identifying the active sync method, e.g., `"vad"`, `"manual"`).
- `inputs` (array of objects with `subtitle` (string path), optional `video` (string path), `detected_offset_ms` (integer or `null` when no detection occurred), `applied_offset_ms` (integer), `status` (`"ok"` or `"error"`), and an optional `error` object with `code`, `category`, `message` when `status == "error"`).
- `operations` (array of objects with `subtitle` (string path), `before_ms` (integer), `after_ms` (integer), `applied` (bool), `status` (`"ok"` or `"error"`), and an optional `error` object when `status == "error"`).

When the sync command processes multiple subtitle files in batch mode, individual per-file failures SHALL be represented as entries with `status == "error"` while the top-level envelope SHALL remain `status == "ok"` and the process exit code SHALL be `0`. A top-level error envelope SHALL be emitted only for whole-command failures such as `InvalidSyncConfiguration`, missing required inputs, or fatal I/O before any file is processed.

In `text` mode (the default) the sync command's existing UX is unchanged.

#### Scenario: VAD-based sync reports detected and applied offsets
- **GIVEN** a subtitle/video pair processed via the VAD method
- **WHEN** the user runs `subx-cli --output json sync <args>`
- **THEN** `data.method == "vad"`, `data.inputs[0].detected_offset_ms` SHALL be an integer, and `data.operations[0].applied` SHALL reflect whether the subtitle file was modified

#### Scenario: Manual sync reports applied offset
- **GIVEN** the user passes a manual offset via the CLI
- **WHEN** the command runs with `--output json`
- **THEN** `data.method == "manual"` and `data.inputs[0].applied_offset_ms` SHALL equal the user-provided offset converted to milliseconds

#### Scenario: Invalid sync configuration produces error envelope
- **GIVEN** the user supplies a sync configuration that fails validation
- **WHEN** the command runs with `--output json`
- **THEN** the envelope SHALL satisfy `status == "error"` and `error.category == "invalid_sync_configuration"`, and the process exit code SHALL match `SubXError::exit_code` for that variant
