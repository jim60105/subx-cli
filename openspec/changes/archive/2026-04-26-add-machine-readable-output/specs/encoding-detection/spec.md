## ADDED Requirements

### Requirement: Detect-Encoding Command Emits Structured JSON Payload

When the `detect-encoding` command runs with the global output mode set to `json`, it SHALL emit a single JSON envelope on stdout (per the `machine-readable-output` capability) and SHALL NOT render the human-friendly result table on stdout. The envelope's `data` object SHALL contain:

- `files` (array of objects with `path` (string path), `encoding` (string, e.g., `"UTF-8"`, `"GBK"`, `"Big5"`), `confidence` (number in `[0.0, 1.0]`), `has_bom` (bool), `status` (`"ok"` or `"error"`), and an optional `error` object with `code`, `category`, `message` when `status == "error"`).

When the command processes multiple paths and an individual file cannot be read or decoded, the affected entry SHALL carry `status == "error"` while the top-level envelope SHALL remain `status == "ok"` and the process exit code SHALL be `0`. A top-level error envelope SHALL be emitted only when the command receives a fatal error before any path is processed (e.g., a single missing path passed as the sole argument, or a fatal configuration error).

In `text` mode (the default) the existing per-file table output is unchanged.

#### Scenario: Single file UTF-8 with BOM
- **GIVEN** a subtitle file encoded in UTF-8 with BOM
- **WHEN** the user runs `subx-cli --output json detect-encoding <path>`
- **THEN** `data.files` SHALL contain exactly one element whose `encoding` matches `UTF-8` (case-insensitive permitted), `has_bom == true`, and `status == "ok"`

#### Scenario: Multiple files reported in array
- **GIVEN** three subtitle files passed via globs or `-i`
- **WHEN** the user runs `subx-cli --output json detect-encoding <paths>`
- **THEN** `data.files` SHALL contain exactly three entries, each populated with `path`, `encoding`, `confidence`, `has_bom`, and `status`

#### Scenario: Unreadable file in batch yields per-item error
- **GIVEN** two readable subtitle files and one path that does not exist passed together
- **WHEN** the user runs `subx-cli --output json detect-encoding <paths>`
- **THEN** the top-level envelope SHALL satisfy `status == "ok"`, `data.files` SHALL contain three entries (two with `status == "ok"`, one with `status == "error"` carrying an `error.category` of `path_not_found` or `file_not_found`), AND the process SHALL exit with status `0`

#### Scenario: Single missing path produces top-level error envelope
- **GIVEN** a single non-existent path passed as the only input
- **WHEN** the user runs `subx-cli --output json detect-encoding <missing>`
- **THEN** the envelope SHALL satisfy `status == "error"`, `error.category` SHALL be `"path_not_found"` or `"file_not_found"`, and the process exit code SHALL equal `SubXError::exit_code` for the underlying variant
