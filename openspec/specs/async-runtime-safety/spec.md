# Async Runtime Safety

## Purpose

Protect the tokio runtime from blocking operations and ensure scheduler
state remains consistent across every control-flow path. Any blocking
filesystem call inside an `async fn` must be offloaded to the blocking
thread pool, and the scheduler must never leak `active_tasks` entries or
deadlock itself after an idle timeout.

## Requirements

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

### Requirement: Library Engine Types Are `Send` and `Sync`

The engine and factory types that make up `subx-core`'s public orchestration surface SHALL be `Send + Sync`, so that a multi-threaded host can hold one across an `.await` point, move one between threads, and share one behind an `Arc` or a framework-managed state container.

- The following eight types SHALL satisfy `Send + Sync + 'static`:
  - `subx_core::core::formats::manager::FormatManager` (`subx-core/src/core/formats/manager.rs`)
  - `subx_core::core::formats::converter::FormatConverter` (`subx-core/src/core/formats/converter.rs`)
  - `subx_core::core::translation::TranslationEngine` (`subx-core/src/core/translation/engine.rs`)
  - `subx_core::core::matcher::MatchEngine` (`subx-core/src/core/matcher/engine.rs`)
  - `subx_core::core::sync::SyncEngine` (`subx-core/src/core/sync/engine.rs`)
  - `subx_core::core::ComponentFactory` (`subx-core/src/core/factory.rs`)
  - `subx_core::core::file_manager::FileManager` (`subx-core/src/core/file_manager.rs`)
  - `Box<dyn subx_core::core::formats::SubtitleFormat>`
- The guarantee SHALL be enforced at compile time by a `const fn assert_send_sync<T: Send + Sync + 'static>()` instantiated once per contracted type in a `thread_safety` module under `subx-core/src/core/mod.rs`. A prose statement SHALL NOT be the only enforcement.
- A trait object stored in any of these types SHALL name its auto-trait bounds, either on the trait itself as a supertrait or at the storage site. A trait object that names neither SHALL NOT be introduced into a contracted type, because `dyn Trait` carries no auto traits by default and the resulting loss of the guarantee is invisible at the definition site.
- Interior mutability SHALL NOT be used to obtain `Sync` for any contracted type. These types are `Sync` because their fields are; adding a lock or a cell to satisfy this requirement SHALL be treated as a change to the type's concurrency model requiring its own proposal.
- When an engine, factory or manager type is added to `subx-core`'s public surface, the change that adds it SHALL add it to the contracted set and to the assertion module.
- Removing `Send` or `Sync` from any contracted type SHALL be treated as a breaking change requiring a major version of `subx-core`, on the same footing as reshaping a public path.

#### Scenario: A conversion engine crosses an await point

- **GIVEN** an `async fn` in a downstream consumer that constructs a `FormatConverter` and awaits `convert_file`
- **WHEN** the resulting future is required to be `Send`, as `tokio::spawn` and a Tauri async command both require
- **THEN** the bound SHALL be satisfied without wrapping the call in `tokio::task::spawn_blocking` and without building a private current-thread runtime to own the converter

#### Scenario: A translation engine is held in shared multi-threaded state

- **GIVEN** a downstream consumer that stores a `TranslationEngine` in a state container requiring `Send + Sync + 'static`
- **WHEN** the container is constructed
- **THEN** the bound SHALL be satisfied, and the engine SHALL be reachable by shared reference from more than one thread

#### Scenario: A non-thread-safe field is added to a contracted type

- **GIVEN** a change that adds a field of a type that is neither `Send` nor `Sync` — an `Rc`, a `RefCell`, or a trait object whose trait declares no auto-trait supertrait — to any of the eight contracted types
- **WHEN** the crate is compiled with its tests
- **THEN** the assertion module SHALL fail to compile, naming the type and the unsatisfied bound, and the failure SHALL occur in `subx-core` rather than in a downstream consumer

#### Scenario: A new engine type is added without a contract entry

- **GIVEN** a proposal adding a new public engine type to `subx-core` that holds a boxed trait object
- **WHEN** the proposal is reviewed against this requirement
- **THEN** it SHALL be required to declare the trait's auto-trait supertraits and to add the type to the contracted set and to the assertion module, or to state why the type is deliberately not part of the orchestration surface
