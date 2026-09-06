# Subtitle Matching

## Purpose

Hold the CLI half of the subtitle-matching agreement: the `match` command argument surface and input preconditions, mutual-exclusion validation, dry-run and execution framing at the command level, the call-ordering obligation that archive-origin relocation runs before uniqueness allocation, and the structured JSON payload of `match`. Implemented in `src/commands/match_command.rs` and `src/cli/match_args.rs`. The match engine — AI pairing, confidence thresholding, relocation modes, naming, and the operation payload shape — is specified by the same-named `subtitle-matching` capability in `subx-core`.

## Requirements

### Requirement: Dry-Run and Execution Modes

The `match` command SHALL support a `--dry-run` mode that displays planned operations and persists them to the match cache without mutating files, and a default live mode that executes the operations.

The two mechanisms this requirement selects between belong to `subx-core`: cache persistence and reuse are specified by the `cache-management` capability's *Dry-Run Cache Reuse Without AI Calls* and *Cache Reuse Preserves Relocation Mode* requirements in that repository, and the execution of an operation set — including the backup, conflict-resolution and atomicity guarantees — by the `file-operation-safety` capability there. This requirement owns the mode selection, the display of planned operations, and the guarantee that dry-run mutates nothing on disk.

#### Scenario: Dry-run preserves files
- **GIVEN** the user runs `subx match --dry-run <path>`
- **WHEN** the command completes
- **THEN** the planned operations SHALL be printed to the user and saved to the cache, and no file on disk SHALL be created, renamed, copied, moved, or deleted

#### Scenario: Live mode applies operations
- **GIVEN** the user runs `subx match <path>` without `--dry-run`
- **WHEN** the command completes successfully
- **THEN** the engine SHALL execute each operation, renaming subtitle files to match the paired video's base name plus the subtitle extension

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

### Requirement: Match Command Argument Surface and Input Preconditions

The `match` command SHALL own its flag surface and the preconditions it checks before reaching the engine. Every behaviour these flags select is specified by the `subtitle-matching` capability in `subx-core`; this requirement states only what `src/cli/match_args.rs` and `src/commands/match_command.rs` must declare and check.

1. **Confidence.** The command SHALL accept `--confidence` as an integer in the inclusive range 0–100, defaulting to 80, and SHALL convert it to the 0.0–1.0 threshold the engine consumes. A value outside the range SHALL be rejected by argument parsing, not by the engine.
2. **Relocation flags.** The command SHALL expose `--copy` (`-c`) and `--move` (`-m`) as mutually exclusive flags, and SHALL map the selected one — or neither — to the corresponding `FileRelocationMode` value. Supplying both SHALL fail validation with the message `Cannot use --copy and --move together. Please choose one operation mode.`
3. **Backup flag.** The command SHALL expose `--backup` and SHALL forward its value, or `general.backup_enabled` when the flag is absent, into the engine's configuration. Whether a backup is then taken is the engine's decision.
4. **Empty input precondition.** When the resolved input paths contain no video or subtitle files, the command SHALL return an error whose message is `No files found to process` and SHALL NOT call the AI provider.

#### Scenario: Confidence outside valid range is rejected
- **GIVEN** the user passes `--confidence 150`
- **WHEN** the CLI parses the arguments
- **THEN** argument parsing SHALL fail with a validation error from `clap`

#### Scenario: Copy and move are mutually exclusive
- **GIVEN** the user passes both `--copy` and `--move`
- **WHEN** the CLI runs `MatchArgs::validate`
- **THEN** validation SHALL fail with the message `Cannot use --copy and --move together. Please choose one operation mode.`

#### Scenario: No input files available
- **GIVEN** the resolved input paths contain no video or subtitle files
- **WHEN** the match command executes
- **THEN** the command SHALL return an error `No files found to process` without calling the AI provider
### Requirement: Match Command Applies Archive-Origin Relocation Before Uniqueness Allocation

When the `match` command rewrites an operation's relocation target because the subtitle originated from an extracted archive, it SHALL complete every such rewrite **before** invoking the global uniqueness allocator, and SHALL invoke the allocator exactly once over the fully rewritten operation set.

- The rewrite is the `archive_origin` branch in `src/commands/match_command.rs`; the allocator is `apply_unique_target_paths`, specified by the `subtitle-matching` capability's *AI-Driven Language and Globally-Unique Target Naming* requirement in `subx-core`.
- This ordering SHALL NOT be assumed to be enforced by the allocator. The allocator is a free function over a mutable operation slice and has no way to require that its caller has finished rewriting; if it runs first, its uniqueness guarantee holds over the pre-rewrite candidate paths and two operations can still collide at their real destinations.
- The command SHALL NOT rewrite a relocation target after the allocator has run, and SHALL NOT invoke the allocator twice, because the allocator's numeric-suffix probing is stable only over a single pass across one operation set.

#### Scenario: Allocator runs after archive-origin forced relocation
- **GIVEN** an archive-origin scenario where the match command rewrites `relocation_target_path` for one or more operations after the engine returns
- **WHEN** the global uniqueness allocator runs
- **THEN** it SHALL operate on the rewritten relocation paths so the uniqueness guarantee holds at the actual destination paths, not at the engine's pre-rewrite candidates

#### Scenario: The allocator is invoked once, after all rewrites
- **GIVEN** an operation set in which some operations are archive-originated and some are not
- **WHEN** the command prepares the set for execution
- **THEN** every archive-origin rewrite SHALL have been applied before the single allocator invocation, and no relocation target SHALL be modified afterwards