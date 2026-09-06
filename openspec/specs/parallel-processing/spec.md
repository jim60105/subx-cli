# Parallel Processing

## Purpose

Hold the CLI half of the parallel-processing agreement: aggregated result reporting, progress reporting opt-out, and the parallel-match task-count reporting with its empty-input-set handling. Implemented in `src/commands/`. The `TaskScheduler` entry point, bounded concurrency, async-execution rules, task accounting, and UUIDv7 identifiers are specified by the same-named `parallel-processing` capability in `subx-core`.

## Requirements

### Requirement: Aggregated Result Reporting

The system SHALL aggregate outcomes of all tasks into success, failure, and partial categories and SHALL report the counts to the user after execution.

#### Scenario: Mixed results are summarized
- **GIVEN** a batch of tasks where some succeed, some fail, and some complete partially
- **WHEN** `monitor_batch_execution` returns
- **THEN** the command SHALL display a summary including the number of successful, failed, and partial tasks

### Requirement: Progress Reporting Opt-Out

The system SHALL respect the `general.enable_progress_bar` configuration; when the flag is false, the progress indicator SHALL be hidden.

#### Scenario: Progress bar disabled
- **GIVEN** `general.enable_progress_bar = false`
- **WHEN** a parallel batch executes
- **THEN** the progress bar SHALL have its draw target set to hidden and no progress animation SHALL appear on the terminal

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