## ADDED Requirements

### Requirement: Non-blocking file I/O in all async functions

All blocking filesystem operations within **any** async function in the codebase SHALL be wrapped in `tokio::task::spawn_blocking()` to prevent blocking the tokio runtime. This includes but is not limited to:
- Parallel task executor functions in `task.rs` (`execute_copy_operation`, `execute_move_operation`, `execute_create_backup_operation`, `execute_rename_file_operation`)
- Match engine async functions in `engine.rs` (`read_to_string` for subtitle content, cache load/save with `fs::read_to_string`/`fs::write`, `create_dir_all` for output directories)
- Any other `async fn` that calls `std::fs::*`, `File::open`, `File::create`, or similar blocking I/O

#### Scenario: large file copy does not block executor

- **WHEN** a copy operation takes 5 seconds on a slow filesystem
- **THEN** other async tasks on the same runtime continue to make progress

#### Scenario: file operations still complete successfully

- **WHEN** a rename or copy is wrapped in spawn_blocking
- **THEN** the operation produces the same result as the blocking version

#### Scenario: match engine file reads do not block runtime

- **WHEN** the match engine reads subtitle content in an async context
- **THEN** the blocking read is offloaded to spawn_blocking

### Requirement: Scheduler active-task cleanup on all code paths

The scheduler SHALL remove entries from `active_tasks` on every code path, including overflow rejection (`Reject` strategy), overflow dropping (`Drop` strategy), and oldest-task eviction (`DropOldest` strategy). An RAII guard struct SHALL ensure cleanup on drop.

#### Scenario: rejected task is cleaned up

- **WHEN** a task is rejected due to queue overflow with `Reject` strategy
- **THEN** the task's entry is removed from `active_tasks`

#### Scenario: dropped task is cleaned up

- **WHEN** a task is dropped due to `Drop` overflow strategy
- **THEN** the task's entry is removed from `active_tasks`

#### Scenario: evicted task is notified

- **WHEN** the oldest task is evicted due to `DropOldest` strategy
- **THEN** a `TaskResult::Failed` is sent to the evicted task's receiver with a descriptive message

### Requirement: Scheduler loop restart on submission

The scheduler loop SHALL not permanently terminate while the `TaskScheduler` instance is alive. If the loop has exited due to idle timeout, submitting a new task SHALL restart the loop.

#### Scenario: task submitted after idle timeout

- **WHEN** the scheduler loop has exited due to idle timeout and a new task is submitted
- **THEN** the loop restarts and the task is processed
