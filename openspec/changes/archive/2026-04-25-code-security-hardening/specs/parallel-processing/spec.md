# Parallel Processing (Delta)

## ADDED Requirements

### Requirement: Non-blocking I/O in async executor

All blocking filesystem operations within async task execution functions SHALL be offloaded to the tokio blocking thread pool via `spawn_blocking`. Direct `std::fs` calls within `async fn` bodies SHALL NOT occur, because they would stall the tokio runtime and starve other concurrent tasks.

#### Scenario: file copy in async context
- **WHEN** an async task executor copies a file
- **THEN** the blocking I/O SHALL be wrapped in `spawn_blocking` and SHALL NOT block the tokio runtime

## MODIFIED Requirements

### Requirement: Active task accounting correctness

The scheduler SHALL maintain accurate `active_tasks` state across all code paths including normal completion, overflow rejection, overflow dropping, and oldest-task eviction. An RAII guard pattern SHALL ensure that `active_tasks` entries are removed when the task's processing scope ends, regardless of the exit path.

#### Scenario: overflow-rejected task cleanup
- **WHEN** a task submission is rejected due to queue overflow
- **THEN** the task's `active_tasks` entry SHALL be removed before the error is returned

#### Scenario: overflow-dropped task cleanup
- **WHEN** a task is dropped due to overflow strategy
- **THEN** the task's `active_tasks` entry SHALL be removed

#### Scenario: evicted task notification
- **WHEN** the oldest task is evicted by `DropOldest`
- **THEN** a `TaskResult::Failed` with a descriptive message SHALL be sent to the evicted task's channel, and its `active_tasks` entry SHALL be cleaned up

#### Scenario: normal completion cleanup
- **WHEN** a task completes normally
- **THEN** its `active_tasks` entry SHALL be removed
