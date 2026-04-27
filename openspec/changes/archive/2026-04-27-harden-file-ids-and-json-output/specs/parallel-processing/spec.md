## ADDED Requirements

### Requirement: UUIDv7 Worker and Task Identifiers

The parallel processing subsystem SHALL identify workers and tasks with UUIDv7 values produced through the `uuid` crate's `Uuid::now_v7()` constructor (or, where strict 1ms monotonicity across IDs is required, through `crate::core::uuidv7::Uuidv7Generator`). The system SHALL NOT call `Uuid::new_v4()` from `src/core/parallel/worker.rs` or `src/core/parallel/scheduler.rs`, and the `uuid` crate's `v4` feature SHALL NOT be enabled in `Cargo.toml`.

#### Scenario: Worker identifier is UUIDv7

- **WHEN** `Worker::new()` constructs a fresh worker
- **THEN** the worker's `id` field SHALL be a `Uuid` whose version nibble equals `7`

#### Scenario: WorkerPool::execute assigns UUIDv7 to dispatched workers

- **WHEN** `WorkerPool::execute` enrolls a new worker for an incoming task
- **THEN** the `worker_id` recorded in the internal `workers` map SHALL be a `Uuid` whose version nibble equals `7`

#### Scenario: Task identifier in test harnesses uses UUIDv7

- **WHEN** the in-tree `CounterTask::task_id` (used by scheduler unit tests in `src/core/parallel/scheduler.rs`) is invoked
- **THEN** the returned string SHALL parse as a `Uuid` whose version nibble equals `7`

#### Scenario: uuid v4 feature is disabled

- **WHEN** `Cargo.toml` is inspected
- **THEN** the `uuid` dependency entry SHALL list `features = ["v7"]` and SHALL NOT include `"v4"`
