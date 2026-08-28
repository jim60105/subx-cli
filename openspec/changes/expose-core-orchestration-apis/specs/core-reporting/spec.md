## MODIFIED Requirements

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

## ADDED Requirements

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
