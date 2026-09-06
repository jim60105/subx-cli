# Core Reporting

## Purpose

Give `src/core/` and `src/services/` a transport-agnostic way to speak to whoever is driving them without depending on the CLI layer. Core owns the `Reporter` trait seam (`crate::core::report`); the CLI provides the only terminal implementation (`TerminalReporter` in `src/cli/reporter.rs`) that applies output-mode and quiet gating. This is the layering invariant the `subx-core` crate split depends on: no module under `src/core/` or `src/services/` may reference `crate::cli`, and a mechanical guard test enforces it.

## Requirements

### Requirement: Transport-Agnostic Reporter Seam

The crate SHALL expose a transport-agnostic reporting sink at `subx_core::core::report` (`subx-core/src/core/report/mod.rs`) through which every module under `subx-core/src/core/` and `subx-core/src/services/` emits human-oriented messages. The module SHALL define:

- `pub trait Reporter: Send + Sync` with exactly five methods, **each carrying a default body** so that an implementor opts in only to the channels it cares about:
  - `fn diagnostic(&self, message: &str)` — human-oriented detail about work in progress; not a failure.
  - `fn warn(&self, message: &str)` — a non-fatal problem the operation recovered from or worked around.
  - `fn ai_usage(&self, usage: &AiUsage)` — token accounting for one completed AI API call.
  - `fn progress(&self, event: &ProgressEvent<'_>)` — an event on the long-running-work progress stream.
  - `fn cancelled(&self) -> bool` — whether the caller has asked the current operation to stop. Its default body SHALL return `false`.
- `pub struct NoopReporter` whose entire implementation is `impl Reporter for NoopReporter {}`.
- `pub fn noop() -> std::sync::Arc<dyn Reporter>` returning a `NoopReporter` behind an `Arc`.
- `#[non_exhaustive] pub enum ProgressEvent<'a>` with exactly four variants:
  - `Message(&'a str)` — free-form status chatter on the progress stream, retry notices included.
  - `Started { total: u64 }` — a unit-counted progress stream is opening over `total` units.
  - `Advanced { done: u64, total: u64, item: Option<&'a str> }` — `done` of `total` units are complete; `item` names the unit that just finished, when the emitter has a name for it.
  - `Finished { done: u64, total: u64 }` — the stream is closing.

`ProgressEvent` SHALL derive `Debug`, `Clone`, `PartialEq` and `Eq`. No variant SHALL carry a floating-point field, because that would silently prevent the `Eq` derive from applying and removing a derive from a public type is a major-version change; a consumer that wants a percentage SHALL divide `done` by `total` itself.

The trait SHALL remain **object-safe**: it is stored and passed as `std::sync::Arc<dyn Reporter>`, never as a generic parameter. Messages SHALL be passed as `&str`; call sites format at the call site.

`ProgressEvent` SHALL remain `#[non_exhaustive]` so that further variants can be added without a breaking change to any implementor. New capabilities on this seam SHALL be added as `#[non_exhaustive]` enum variants or as trait methods with default bodies; a supertrait SHALL NOT be added to `Reporter`, because `trait_added_supertrait` is a major-version change and every capability this seam has needed so far has been expressible without one.

#### Scenario: Default implementation is silent
- **GIVEN** a type that implements `Reporter` and overrides no method
- **WHEN** `diagnostic`, `warn`, `ai_usage` and `progress` are each invoked on it
- **THEN** every call SHALL return without producing any output on any stream and without panicking

#### Scenario: Default cancellation answer is false
- **GIVEN** a type that implements `Reporter` and overrides no method
- **WHEN** `cancelled()` is invoked on it
- **THEN** it SHALL return `false`, so that a core loop attached to such a reporter runs to completion

#### Scenario: Partial implementation opts into one channel
- **GIVEN** a type that implements `Reporter` and overrides only `warn`
- **WHEN** `warn("careful")` and then `diagnostic("detail")` are invoked on it
- **THEN** the overridden `warn` SHALL receive `"careful"` and the un-overridden `diagnostic` SHALL be a silent no-op

#### Scenario: Reporter is usable as a trait object
- **GIVEN** the expression `let r: std::sync::Arc<dyn Reporter> = subx_core::core::report::noop();`
- **WHEN** the crate is compiled
- **THEN** it SHALL compile, confirming the trait is object-safe and `noop()` yields an `Arc<dyn Reporter>`

#### Scenario: ProgressEvent is extensible
- **GIVEN** a `match` over a `&ProgressEvent<'_>` in an implementor outside the defining module
- **WHEN** that `match` handles `ProgressEvent::Message` and a `_` arm
- **THEN** it SHALL compile, and SHALL continue to compile when further variants are added to the enum

#### Scenario: Progress events compare by value
- **GIVEN** two `ProgressEvent::Advanced { done: 3, total: 7, item: Some("a.srt") }` values
- **WHEN** they are compared with `==`
- **THEN** they SHALL be equal, confirming the `PartialEq` and `Eq` derives still apply after the variant additions

### Requirement: Reporter Is Send and Sync

The `Reporter` trait SHALL declare `Send + Sync` as supertraits. Core and service types SHALL store the reporter as `std::sync::Arc<dyn Reporter>` so that a single reporter can be shared by every component a factory builds and cloned into spawned tasks for the cost of a reference-count bump.

This bound is required because the engines are driven from spawned tasks: `WorkerPool::execute` (`src/core/parallel/worker.rs`) `tokio::spawn`s its work and `WorkerPool` is `Clone`; `MatchEngine`, `TranslationEngine` and `SyncEngine` expose `async` methods whose futures are held across `.await` points on a multi-threaded Tokio runtime; and downstream GUI consumers drive the same engines from `spawn_blocking` closures.

Adding an `Arc<dyn Reporter>` field SHALL NOT make any type less thread-safe than it is today.

#### Scenario: Arc<dyn Reporter> crosses a thread boundary
- **GIVEN** a `std::sync::Arc<dyn Reporter>` value
- **WHEN** it is cloned and moved into a `tokio::spawn`ed task that calls `diagnostic` on it
- **THEN** the code SHALL compile and the call SHALL be observable by the reporter implementation

#### Scenario: Static assertion of the bound
- **GIVEN** a compile-time assertion that `std::sync::Arc<dyn Reporter>` is `Send` and `Sync`
- **WHEN** the crate is compiled
- **THEN** the assertion SHALL hold

### Requirement: Core-Owned AI Usage Payload

`crate::core::report` SHALL define a plain, core-owned value type carrying the token counts for one AI API call:

```rust
pub struct AiUsage {
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
```

It SHALL derive `Debug`, `Clone`, `PartialEq` and `Eq`, and SHALL NOT depend on any presentation crate, any AI-provider type, or any CLI module.

`crate::services::ai::AiUsageStats` SHALL become a re-export alias of this type (`pub use crate::core::report::AiUsage as AiUsageStats;`) so that exactly one struct exists. Existing consumers — `AiResponse::usage`, `crate::cli::ui::display_ai_usage`, the four AI clients' struct literals, and every test naming `AiUsageStats` — SHALL keep compiling unchanged. Per the project rule forbidding new `#[deprecated]` items, the alias SHALL be documented as legacy in rustdoc prose only.

Each AI provider client SHALL report its token counts by calling `self.reporter.ai_usage(&usage)` after a successful API response, and SHALL NOT import or call any CLI display helper.

#### Scenario: AiUsageStats resolves to AiUsage
- **GIVEN** the expression `let u: crate::core::report::AiUsage = crate::services::ai::AiUsageStats { model: "gpt-4".into(), prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 };`
- **WHEN** the crate is compiled
- **THEN** it SHALL compile, confirming the two paths name the same type

#### Scenario: Provider reports usage through the seam
- **GIVEN** an AI provider client with a recording `Reporter` attached, and an API response whose body carries a `usage` object with `prompt_tokens`, `completion_tokens` and `total_tokens`
- **WHEN** the client completes the call
- **THEN** the recording reporter SHALL have received exactly one `ai_usage` invocation whose `AiUsage` carries the model name and the three token counts from the response

#### Scenario: Provider with no reporter attached is silent
- **GIVEN** an AI provider client constructed through its existing constructor with no reporter attached
- **WHEN** it completes a call whose response carries a `usage` object
- **THEN** no bytes SHALL be written to stdout or stderr by the usage-reporting path

### Requirement: Reporter Attachment Preserves Constructor Signatures

Attaching a reporter SHALL NOT change any existing constructor signature. Every affected type SHALL keep its current constructor, SHALL default its reporter field to `crate::core::report::noop()`, and SHALL expose a builder method:

```rust
pub fn with_reporter(mut self, reporter: std::sync::Arc<dyn Reporter>) -> Self
```

This applies to `MatchEngine::new`, `SyncEngine::new`, `TranslationEngine::new`, `ComponentFactory::new`, `FileManager::new` (and its `Default` impl), `WorkerPool::new`, and the constructors of the four AI provider clients. `Result`-returning constructors compose as `Type::new(args)?.with_reporter(reporter)`.

`ComponentFactory` SHALL propagate its reporter into every component it builds — `create_match_engine`, `create_translation_engine`, `create_file_manager` and `create_ai_provider` — so that one `with_reporter` call at a command boundary wires an entire command. The free function `create_ai_provider(&AIConfig)` SHALL keep its signature and delegate to a reporter-aware variant; the reporter SHALL be attached to the concrete client before it is boxed as `Box<dyn AIProvider>`.

`FileManager` and `WorkerPool` SHALL receive the reporter as a constructor-optional field through the same builder, NOT as a per-call parameter, so that `FileManager::rollback` and `WorkerPool::shutdown` keep their current signatures and all existing rustdoc examples keep compiling.

#### Scenario: Existing constructor call still compiles
- **GIVEN** the call `MatchEngine::new(ai_client, match_config)` with no reporter argument
- **WHEN** the crate is compiled
- **THEN** it SHALL compile and the resulting engine SHALL behave as if a `NoopReporter` were attached

#### Scenario: Builder attaches a reporter
- **GIVEN** the call `MatchEngine::new(ai_client, match_config).with_reporter(reporter)`
- **WHEN** the engine emits a diagnostic during a match run
- **THEN** the supplied reporter SHALL receive it

#### Scenario: Fallible constructor composes with the builder
- **GIVEN** the call `SyncEngine::new(sync_config)?.with_reporter(reporter)`
- **WHEN** the crate is compiled
- **THEN** it SHALL compile, confirming `with_reporter` consumes and returns `Self` after the `?`

#### Scenario: Factory propagates its reporter
- **GIVEN** a `ComponentFactory::new(config_service)?.with_reporter(reporter)`
- **WHEN** `create_match_engine`, `create_translation_engine`, `create_file_manager` and `create_ai_provider` are each called on it
- **THEN** every produced component SHALL carry the same reporter, and a diagnostic emitted by any of them SHALL reach it

#### Scenario: FileManager rollback signature is unchanged
- **GIVEN** the rustdoc example `let mut manager = FileManager::new(); manager.rollback()?;`
- **WHEN** `cargo test --doc` runs
- **THEN** the example SHALL compile and pass without mentioning a reporter

### Requirement: No Core or Service Module References the CLI Layer

No module under `src/core/` or `src/services/` SHALL reference `crate::cli` in any form — no `use crate::cli::…`, no fully-qualified `crate::cli::output::active_mode()`, no `crate::cli::display_ai_usage`. Core and service code SHALL NOT read the CLI's process-global output mode or quiet flag, SHALL NOT know that a machine-readable output mode exists, and SHALL NOT call `println!` or `eprintln!` for human-oriented status, diagnostic, warning, progress or usage output. All such output SHALL be routed through `Reporter`.

The rule SHALL be enforced by an automated guard test that walks every `.rs` file under `src/core/` and `src/services/`, resolved from `CARGO_MANIFEST_DIR` rather than the current working directory, and fails with the offending `file:line` list when any occurrence is found.

This is the layering invariant the crate split depends on: `clap`, `colored` and `indicatif` are permanently CLI-only dependencies, and `crate::cli` will not exist in the crate that `src/core/` and `src/services/` move into.

#### Scenario: Boundary grep is clean
- **GIVEN** the working tree after this change
- **WHEN** `grep -rn "crate::cli" src/core src/services` is run
- **THEN** it SHALL return zero non-test hits

#### Scenario: Guard test fails on a re-introduced edge
- **GIVEN** a file under `src/core/` that contains the token `crate::cli`
- **WHEN** the boundary guard test runs
- **THEN** it SHALL fail and its message SHALL name the offending file and line

#### Scenario: Guard test resolves paths from the manifest directory
- **GIVEN** the boundary guard test executed by a runner whose current working directory is not the crate root
- **WHEN** the test resolves `src/core/` and `src/services/`
- **THEN** it SHALL resolve them relative to `CARGO_MANIFEST_DIR` and SHALL still find and scan every file

#### Scenario: Core engines emit no direct terminal output
- **GIVEN** a `MatchEngine`, `TranslationEngine`, `FileManager` or `WorkerPool` constructed with no reporter attached
- **WHEN** it executes a path that would previously have printed status, diagnostic, warning, progress or AI-usage text
- **THEN** no bytes SHALL be written to stdout or stderr by that path

### Requirement: Structured Progress Stream Semantics

A core-owned loop that reports unit-counted progress SHALL do so as a **stream**: exactly one `ProgressEvent::Started`, then zero or more `ProgressEvent::Advanced`, then exactly one `ProgressEvent::Finished`, all through `Reporter::progress`.

- `Started { total }` SHALL be emitted before the first unit of work begins, and `total` SHALL be the number of units the loop intends to process.
- `Advanced { done, total, item }` SHALL be emitted after each unit completes. `done` SHALL be monotonically non-decreasing across one stream, SHALL never exceed `total`, and SHALL equal the number of units completed so far. `total` SHALL equal the value reported by `Started`. `item` SHALL name the unit that just completed when the emitter has a human-meaningful name for it, and SHALL be `None` otherwise.
- `Finished { done, total }` SHALL be emitted exactly once when the loop stops, whether it completed or stopped early. `done < total` SHALL mean the loop stopped before processing every unit; `done == total` SHALL mean it processed all of them.
- A stream with `total == 0` SHALL still emit `Started { total: 0 }` and `Finished { done: 0, total: 0 }` with no `Advanced` between them, so a consumer sees the batch open and close.

At most **one** progress stream SHALL be open per `Reporter` at any time; core SHALL NOT open a stream from inside another. A consumer that receives a `Started` while a stream is open SHALL treat it as replacing the previous stream rather than nesting it.

The emitters SHALL be exactly:

- `MatchEngine::execute_operations` (`subx-core/src/core/matcher/engine.rs`), over its operation loop.
- `MatchEngine::execute_operations_audit` (`subx-core/src/core/matcher/engine.rs`), over its operation loop, with `item` set to the operation's `subtitle_file.name`.

Existing `ProgressEvent::Message` call sites SHALL NOT be converted to `Advanced`. In particular `TranslationEngine`'s per-batch progress block (`subx-core/src/core/translation/engine.rs`) SHALL keep emitting `Message`, because its rendered bytes are fixed by the `core-reporting` *Reporter Attachment Preserves Constructor Signatures* work and by the CLI characterisation tests that lock them.

A dry-run invocation SHALL emit no progress stream, because it performs no units of work.

#### Scenario: Stream shape over three operations
- **GIVEN** a `MatchEngine` with a recording `Reporter` attached and three operations to execute
- **WHEN** `execute_operations_audit(&operations, false)` runs to completion
- **THEN** the reporter SHALL have received, in order, `Started { total: 3 }`, three `Advanced` events with `done` equal to `1`, `2` and `3` and `total` equal to `3`, and `Finished { done: 3, total: 3 }`

#### Scenario: Advanced names the completed unit
- **GIVEN** a `MatchEngine` with a recording `Reporter` attached and one operation whose `subtitle_file.name` is `"movie.srt"`
- **WHEN** `execute_operations_audit` completes that operation
- **THEN** the `Advanced` event SHALL carry `item: Some("movie.srt")`

#### Scenario: Empty batch still opens and closes
- **GIVEN** a `MatchEngine` with a recording `Reporter` attached and an empty operations slice
- **WHEN** `execute_operations_audit(&[], false)` runs
- **THEN** the reporter SHALL have received exactly `Started { total: 0 }` followed by `Finished { done: 0, total: 0 }` and no `Advanced` event

#### Scenario: Dry run emits no stream
- **GIVEN** a `MatchEngine` with a recording `Reporter` attached and two operations
- **WHEN** `execute_operations_audit(&operations, true)` runs
- **THEN** the reporter SHALL have received no `ProgressEvent` of any variant

#### Scenario: A reporter that ignores progress is unaffected
- **GIVEN** a `MatchEngine` constructed with no reporter attached
- **WHEN** `execute_operations_audit` executes three operations
- **THEN** no bytes SHALL be written to stdout or stderr by the progress path and the returned outcomes SHALL be identical to those produced before progress emission was added

### Requirement: Cooperative Cancellation Through the Reporter

A caller SHALL be able to stop a core-owned execution loop between units of work by returning `true` from `Reporter::cancelled`. The mechanism SHALL be a **poll**, not a notification: core SHALL call `cancelled()` and SHALL NOT require any channel, waker, or additional dependency. `subx-core` SHALL NOT declare a cancellation-primitive dependency for this purpose.

- `MatchEngine::execute_operations_audit` (`subx-core/src/core/matcher/engine.rs`) SHALL call `self.reporter.cancelled()` immediately before starting each operation. When it returns `true`, the method SHALL stop, SHALL NOT begin that operation or any later one, and SHALL pad the remaining slots with `OperationOutcome { applied: false, error: None }` so that `outcomes.len() == operations.len()` still holds. It SHALL return `Ok(..)`, never `Err`, for a cancellation.
- The stream's closing `ProgressEvent::Finished { done, total }` SHALL carry `done < total` in that case, which is the transport-agnostic signal that the batch stopped early.
- Cancellation SHALL be observed only between operations, never inside one, so a cancelled run SHALL NOT leave a partially written or partially renamed file.
- `MatchEngine::execute_operations` SHALL NOT observe cancellation. Its `Result<()>` return type cannot express a short batch, and neither changing its signature nor adding an error variant is permitted.
- Mid-`await` cancellation SHALL NOT be added to this seam. A caller that needs to abandon an in-flight future SHALL drop it — which is what `tokio::select!` and `tokio::task::AbortHandle` already do — and core SHALL remain safe to drop at every `.await` point.

#### Scenario: Cancellation stops before the next operation
- **GIVEN** a `MatchEngine` with a `Reporter` whose `cancelled()` returns `false` for the first two calls and `true` afterwards, and four operations
- **WHEN** `execute_operations_audit(&operations, false)` runs
- **THEN** exactly two operations SHALL have been applied, and no filesystem change SHALL have been made for the third or fourth

#### Scenario: Cancelled run preserves outcome arity
- **GIVEN** the run above
- **WHEN** it returns
- **THEN** it SHALL return `Ok(outcomes)` with `outcomes.len() == 4`, whose last two entries SHALL each be `OperationOutcome { applied: false, error: None }`

#### Scenario: Cancellation is visible on the progress stream
- **GIVEN** the run above with a recording `Reporter`
- **WHEN** it returns
- **THEN** the final event SHALL be `Finished { done: 2, total: 4 }`

#### Scenario: A reporter that never cancels changes nothing
- **GIVEN** a `MatchEngine` with a `Reporter` that does not override `cancelled`
- **WHEN** `execute_operations_audit` executes four operations
- **THEN** all four SHALL be attempted and the outcomes SHALL be identical to those produced before the cancellation check was added
