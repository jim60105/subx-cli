# Parallel Processing

## Purpose

Execute batch subtitle operations (e.g., parallel match, conversion) across multiple files concurrently using a worker-pool task scheduler with bounded concurrency and aggregated result reporting. Implemented in `src/core/parallel/` (`scheduler.rs`, `task.rs`, `worker.rs`, `pool.rs`, `load_balancer.rs`, `config.rs`).

## Requirements

### Requirement: Task Scheduler Entry Point

The system SHALL expose `TaskScheduler::new()` as the primary entry point for batch execution, and the scheduler SHALL accept a collection of boxed `Task + Send + Sync` values and return a corresponding `Vec<TaskResult>`.

#### Scenario: Parallel match over a directory
- **GIVEN** a directory containing N video files and `subx match` uses the parallel execution path
- **WHEN** the scheduler runs the generated `FileProcessingTask` set
- **THEN** each video SHALL be processed by the scheduler and, before execution, the command SHALL report the number of tasks to be processed and the maximum concurrency to the user

#### Scenario: Empty task list exits early
- **GIVEN** no video files are discovered
- **WHEN** `execute_parallel_match` runs
- **THEN** the command SHALL print `No video files found to process` and return successfully without scheduling any work

### Requirement: Bounded Concurrency

The system SHALL limit the number of concurrently running tasks to the active worker count configured from `config.parallel`, preventing unbounded task fan-out.

#### Scenario: Concurrency is reported
- **GIVEN** the parallel matcher has been initialized
- **WHEN** the scheduler begins running tasks
- **THEN** it SHALL expose the active worker count via `scheduler.get_active_workers()` and SHALL not run more tasks simultaneously than that count

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
