## ADDED Requirements

### Requirement: Match Command Emits Structured JSON Payload

When the `match` command runs with the global output mode set to `json`, it SHALL emit a single JSON envelope on stdout (per the `machine-readable-output` capability) and SHALL NOT render the human-friendly result table from `src/cli/table.rs` nor any progress bar. The envelope's `data` object SHALL contain:

- `dry_run` (bool) reflecting the effective `--dry-run` flag.
- `confidence_threshold` (integer in `[0, 100]`) reflecting the effective `--confidence` value.
- `candidates` (array of objects with `video` (string path), `subtitle` (string path), `confidence` (integer 0–100), `accepted` (bool), and an optional `reason` (string) when `accepted == false`).
- `operations` (array of objects with `kind` in `{"rename", "copy", "move"}`, `source` (string path), `target` (string path), `applied` (bool), `status` (`"ok"` or `"error"`), and an optional `error` object with `code`, `category`, `message` when `status == "error"`).
- `summary` (object with integer fields `total_candidates`, `accepted`, `applied`, `skipped`, `failed`).

When the match operation loop applies multiple file operations and an individual operation fails, the affected `operations[i]` entry SHALL carry `status == "error"` and `applied == false` while the top-level envelope MAY remain `status == "ok"` provided at least one prior operation succeeded; alternatively the command MAY abort the loop and emit a top-level error envelope whose `error.details.partial_results` records the operations already applied. A top-level error envelope SHALL be emitted for whole-command failures (AI service failure before any operation is computed, configuration errors, missing inputs).

In `text` mode (the default) the match command's existing UX — colored result table, progress bar, status symbols — is unchanged.

#### Scenario: JSON mode emits payload instead of table
- **GIVEN** an input directory with one accepted video/subtitle pair and an AI provider configured
- **WHEN** the user runs `subx-cli --output json match <path>`
- **THEN** stdout SHALL contain a single JSON envelope with `command == "match"`, `status == "ok"`, and `data.candidates`/`data.operations` populated, and SHALL NOT contain the formatted match table

#### Scenario: Dry-run flag surfaced in payload
- **WHEN** the user runs `subx-cli --output json match --dry-run <path>` with at least one accepted candidate
- **THEN** `data.dry_run == true` and every entry in `data.operations` SHALL satisfy `applied == false`

#### Scenario: Sub-threshold candidates are reported as not accepted
- **GIVEN** an AI provider returns a candidate whose confidence is below `--confidence`
- **WHEN** the user runs `subx-cli --output json match --confidence 90 <path>`
- **THEN** `data.candidates` SHALL include the candidate with `accepted == false` and `data.summary.skipped` SHALL count it

#### Scenario: AI failure surfaces as error envelope
- **GIVEN** the AI provider fails with a network error
- **WHEN** the user runs `subx-cli --output json match <path>`
- **THEN** the envelope SHALL satisfy `status == "error"`, `error.category == "ai_service"`, `error.exit_code == 3`, and the process SHALL exit with status `3`
