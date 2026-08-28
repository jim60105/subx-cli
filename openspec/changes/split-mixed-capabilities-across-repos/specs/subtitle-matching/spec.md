## REMOVED Requirements

### Requirement: AI-Based File Pairing

**Reason**: Core half of the split. `MatchEngine::match_file_list`, the `AnalysisRequest` it sends and the operation generation are all in `src/core/matcher/engine.rs`, which is `subx-core` after B2, and the GUI constructs `MatchEngine::new` directly (SDR §8). Re-added in `subx-core` by `import-split-capability-specs`, at `openspec/specs/subtitle-matching/spec.md` in that repository. It leaves this repository's half of the capability; it does not leave the project.

**Migration**: The *No input files available* scenario is CLI: the literal `No files found to process` is constructed at `src/commands/match_command.rs:446`, before the engine is reached. It is lifted into *Match Command Argument Surface and Input Preconditions*, added below. The *Successful match with sufficient confidence* scenario stays with the core half; its reference to `InputPathHandler` resolves inside `subx-core` after A2, and its `subx match <path>` phrasing SHALL be named as `subx-cli`'s invocation.

### Requirement: Confidence Threshold Enforcement

**Reason**: Core half of the split. Converting the user-supplied value to a 0.0–1.0 threshold and discarding sub-threshold candidates happens in `src/core/matcher/engine.rs`. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The clause naming the range 0–100, the default 80 and the `--confidence` flag is a `subx-cli` obligation — the field and its clap range are at `src/cli/match_args.rs:23-25` — and is lifted into bullet 1 of *Match Command Argument Surface and Input Preconditions*, together with the *Confidence outside valid range is rejected* scenario, whose THEN is a `clap` validation failure. The arriving requirement SHALL state the threshold as a 0.0–1.0 value supplied by the caller, with the percentage surface named as `subx-cli`'s.

### Requirement: File Relocation Modes

**Reason**: Core half of the split. `FileRelocationMode` and the copy/move execution are in `src/core/matcher/engine.rs`, and the GUI consumes `FileRelocationMode` directly (SDR §8). Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The mutual-exclusion obligation and its exact message — `Cannot use --copy and --move together. Please choose one operation mode.`, produced by `MatchArgs::validate` at `src/cli/match_args.rs:53` — is a `subx-cli` obligation and is lifted into bullet 2 of *Match Command Argument Surface and Input Preconditions*, together with the *Copy and move are mutually exclusive* scenario. The arriving requirement SHALL state the three relocation modes as values of `FileRelocationMode`, with the `--copy` / `--move` / neither surface named as `subx-cli`'s.

### Requirement: Optional Backup Before Move

**Reason**: Core half of the split. The backup decision and the backup task are in the match engine and `src/core/file_manager.rs`; the `general.backup_enabled` value reaches them through `MatchConfig`. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: Both scenarios are phrased as `subx match --move --backup <path>` invocations whose THEN is about whether a backup task is scheduled — narrative framing over core behaviour. They SHALL be carried over with the invocation named as `subx-cli`'s match command; the `--backup` flag's definition and forwarding are bullet 3 of *Match Command Argument Surface and Input Preconditions*.

### Requirement: Per-Scan Unique UUIDv7 File Identifiers for Matching

**Reason**: Core half of the split. The requirement already names "Implemented in `src/core/matcher/discovery.rs` (`generate_file_id`, `Uuidv7Generator` integration)", and `tests/match_engine_id_integration_tests.rs` is core-bound under B3's ownership test. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: The path `crate::core::uuidv7::Uuidv7Generator` is correct verbatim inside `subx-core`. The sentence about cross-invocation correlation "between the match cache and a later `cache apply`" references a `subx-cli` subcommand; on arrival `cache apply` SHALL be named as `subx-cli`'s subcommand, leaving canonical filesystem paths as the normative correlation key.

### Requirement: AI-Driven Language and Globally-Unique Target Naming

**Reason**: Core half of the split. `MatchEngine::generate_subtitle_name`, the `LanguageDetector` code map and the global uniqueness allocator `apply_unique_target_paths` (`src/core/matcher/engine.rs:134`) are all `subx-core` after B2, and the GUI consumes `apply_unique_target_paths` directly (SDR §8). Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The requirement states its allocator obligation as running "After all operations have been generated **and** `match_command` has applied any archive-origin forced relocation". That ordering is a `subx-cli` obligation — `apply_unique_target_paths` is called at `src/commands/match_command.rs:470`, immediately after the archive-origin rewrite at `:459` — and the core allocator cannot enforce it, because a function cannot require its caller to have done something first. It is lifted into *Match Command Applies Archive-Origin Relocation Before Uniqueness Allocation*, added below, together with the *Allocator runs after archive-origin forced relocation* scenario. On arrival the core half SHALL state the ordering as a **precondition on its input** — that the operations passed to the allocator already carry their final relocation targets — rather than as an obligation on a caller it cannot name, and SHALL name the `subx-cli` requirement that discharges it. The *Cross-video duplicates in the same target directory are disambiguated* scenario mentions the same rewrite in its GIVEN and SHALL be carried over with `match_command` named as `subx-cli`'s.

## MODIFIED Requirements

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

## ADDED Requirements

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
