## REMOVED Requirements

### Requirement: Task Scheduler Entry Point

**Reason**: Core half of the split, and the correction to C2a's hand-off. C2a Decision 1 demoted `parallel-processing` to this change's list on the evidence of two requirements; this is the third. The requirement's normative subject — `TaskScheduler::new()` and its contract of accepting boxed `Task + Send + Sync` values and returning a corresponding `Vec<TaskResult>` — is `src/core/parallel/scheduler.rs`, which is `subx-core` after B2. Re-added in `subx-core` by `import-split-capability-specs`, at `openspec/specs/parallel-processing/spec.md` in that repository. It leaves this repository's half of the capability; it does not leave the project.

**Migration**: Both scenarios contain CLI obligations, and they are lifted into *Parallel Match Reports Task Count and Handles an Empty Input Set*, added below. *Parallel match over a directory* is the one scenario in this change restated on both sides: the core half keeps "each video SHALL be processed by the scheduler" and drops the reporting clause; the CLI half keeps the reporting clause. *Empty task list exits early* is wholly CLI — `execute_parallel_match` and the `println!("No video files found to process")` are at `src/commands/match_command.rs:682` and `:718` — and moves entirely.

### Requirement: Bounded Concurrency

**Reason**: Core half of the split. The concurrency limit and `scheduler.get_active_workers()` are in `src/core/parallel/`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Batch Task Submission

**Reason**: Core half of the split. `TaskScheduler::submit_batch_tasks` is in `src/core/parallel/scheduler.rs`; its test citation `tests/parallel_processing_integration_tests.rs` is core-bound under B3's ownership test and moves with it. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: B4 renames the moved `tests/parallel/integration_tests.rs` to `parallel_integration_tests.rs`. That is a different file from the top-level `tests/parallel_processing_integration_tests.rs` this requirement cites, which B3 flattens under its existing basename; the citation SHALL be carried over verbatim, and SHALL be re-checked against `subx-core/tests/` at import time so that the two similarly-named files are not confused.

### Requirement: Task Queue Overflow Strategy

**Reason**: Core half of the split. The requirement already names "Implemented in `src/core/parallel/scheduler.rs::submit_task_with_priority`". Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Optional Task Priority Ordering

**Reason**: Core half of the split. `TaskPriority` is defined in `src/core/parallel/scheduler.rs` and the dispatch order is enforced there. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Non-blocking I/O in async executor

**Reason**: Core half of the split. The `spawn_blocking` obligation applies to the async task executors under `src/core/parallel/`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: The `async-runtime-safety` capability, which C2a moved wholesale to `subx-core`, states a broader form of the same invariant. The overlap is pre-existing and intra-repository once both are in `subx-core`; it is carried over unchanged, because deleting a requirement is not a decision this change has a mandate to make.

### Requirement: Active task accounting correctness

**Reason**: Core half of the split. The RAII guard and the four exit paths it covers are in `src/core/parallel/scheduler.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: UUIDv7 Worker and Task Identifiers

**Reason**: Core half of the split. `src/core/parallel/worker.rs`, `src/core/parallel/scheduler.rs` and `crate::core::uuidv7::Uuidv7Generator` are all `subx-core` after B2. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: Two edits. The path `crate::core::uuidv7::Uuidv7Generator` is correct verbatim inside `subx-core` and SHALL NOT be rewritten. The `Cargo.toml` clause — "the `uuid` crate's `v4` feature SHALL NOT be enabled in `Cargo.toml`", with the *uuid v4 feature is disabled* scenario — SHALL be read as `subx-core`'s own manifest, which is the only one of the two that declares `uuid` at all under SDR §4; on arrival the citation SHALL name that manifest unambiguously so the scenario is not checked against the superproject's.

## ADDED Requirements

### Requirement: Parallel Match Reports Task Count and Handles an Empty Input Set

The `match` command's parallel execution path SHALL make the batch visible to the user before it starts and SHALL exit cleanly when there is nothing to do.

- Before submitting tasks to the scheduler, `execute_parallel_match` SHALL report to the user the number of tasks to be processed and the maximum concurrency in effect.
- When file discovery yields no video files, `execute_parallel_match` SHALL print `No video files found to process` and return successfully without constructing a scheduler or submitting any task.
- Both obligations are the command's, not the scheduler's: the scheduler's contract is specified by the `parallel-processing` capability's *Task Scheduler Entry Point* requirement in `subx-core`, which says nothing about reporting because a library that reports to a terminal is what the `core-reporting` capability exists to forbid.

#### Scenario: Parallel match over a directory
- **GIVEN** a directory containing N video files and `subx match` uses the parallel execution path
- **WHEN** the command prepares the generated `FileProcessingTask` set
- **THEN** before execution the command SHALL report the number of tasks to be processed and the maximum concurrency to the user

#### Scenario: Empty task list exits early
- **GIVEN** no video files are discovered
- **WHEN** `execute_parallel_match` runs
- **THEN** the command SHALL print `No video files found to process` and return successfully without scheduling any work
