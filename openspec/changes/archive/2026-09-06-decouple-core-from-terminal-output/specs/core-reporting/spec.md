## ADDED Requirements

### Requirement: Transport-Agnostic Reporter Seam

The crate SHALL expose a transport-agnostic reporting sink at `crate::core::report` (`src/core/report/mod.rs`) through which every module under `src/core/` and `src/services/` emits human-oriented messages. The module SHALL define:

- `pub trait Reporter: Send + Sync` with exactly four methods, **each carrying a default no-op body** so that an implementor opts in only to the channels it cares about:
  - `fn diagnostic(&self, message: &str)` — human-oriented detail about work in progress; not a failure.
  - `fn warn(&self, message: &str)` — a non-fatal problem the operation recovered from or worked around.
  - `fn ai_usage(&self, usage: &AiUsage)` — token accounting for one completed AI API call.
  - `fn progress(&self, event: &ProgressEvent<'_>)` — an event on the long-running-work progress stream.
- `pub struct NoopReporter` whose entire implementation is `impl Reporter for NoopReporter {}`.
- `pub fn noop() -> std::sync::Arc<dyn Reporter>` returning a `NoopReporter` behind an `Arc`.
- `#[non_exhaustive] pub enum ProgressEvent<'a>` with the single variant `Message(&'a str)`, documented as covering free-form status chatter on the progress stream, retry notices included.

The trait SHALL remain **object-safe**: it is stored and passed as `std::sync::Arc<dyn Reporter>`, never as a generic parameter. Messages SHALL be passed as `&str`; call sites format at the call site.

`ProgressEvent` SHALL be `#[non_exhaustive]` so that the `expose-core-orchestration-apis` change can add structured started/advanced/finished variants without a breaking change to any implementor.

#### Scenario: Default implementation is silent
- **GIVEN** a type that implements `Reporter` and overrides no method
- **WHEN** `diagnostic`, `warn`, `ai_usage` and `progress` are each invoked on it
- **THEN** every call SHALL return without producing any output on any stream and without panicking

#### Scenario: Partial implementation opts into one channel
- **GIVEN** a type that implements `Reporter` and overrides only `warn`
- **WHEN** `warn("careful")` and then `diagnostic("detail")` are invoked on it
- **THEN** the overridden `warn` SHALL receive `"careful"` and the un-overridden `diagnostic` SHALL be a silent no-op

#### Scenario: Reporter is usable as a trait object
- **GIVEN** the expression `let r: std::sync::Arc<dyn Reporter> = crate::core::report::noop();`
- **WHEN** the crate is compiled
- **THEN** it SHALL compile, confirming the trait is object-safe and `noop()` yields an `Arc<dyn Reporter>`

#### Scenario: ProgressEvent is extensible
- **GIVEN** a `match` over a `&ProgressEvent<'_>` in an implementor outside the defining module
- **WHEN** that `match` handles `ProgressEvent::Message` and a `_` arm
- **THEN** it SHALL compile, and SHALL continue to compile when further variants are added to the enum

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
