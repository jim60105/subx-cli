## Context

SubX is today one crate. `src/cli/` owns the clap surface, the terminal UI (`ui.rs`, `table.rs`) and the process-global output mode (`output.rs`). `src/core/` and `src/services/` own the engines. The dependency is supposed to flow one way — CLI depends on core — and for `src/config/**` it genuinely does (SDR §3: zero upward references). For `src/core/**` and `src/services/**` it does not: thirteen non-test sites reach up into `crate::cli`.

Every one of those thirteen sites exists for the same reason. A core engine wants to say something to a human, so it prints. But since `--output json` and `--quiet` were added, printing unconditionally would corrupt the single-JSON-envelope contract (`machine-readable-output`), so each site learned to ask the CLI whether it is allowed to print. The result is that `MatchEngine`, `TranslationEngine`, `WorkerPool` and `FileManager` all know that a terminal exists, that JSON mode exists, and — for three of them — that a `--quiet` flag exists.

Two forces make this urgent rather than merely untidy:

1. **The split (SDR §0–§3).** When `src/core/` moves to the `subx-core` repository, `crate::cli` stops existing in that crate. `clap`, `colored` and `indicatif` are permanently `subx-cli`-only (SDR D8, §4). Thirteen compile errors would have to be redesigned *during* a file move, which is exactly the situation the A0–A2 preparatory changes exist to avoid.
2. **The GUI (SDR §8).** `../subx` already drives `MatchEngine`, `TranslationEngine`, `ComponentFactory`, `FileManager` and `core::matcher::*` directly, and consumes **none** of `cli::{ui, output, table}`. Every message these engines print today lands in the Tauri process's stdout/stderr, unlabelled and uninterceptable. The GUI cannot silence it and cannot surface it in its own UI. SDR §8 gap 6 records that the GUI had to re-implement progress from scratch over `tauri::ipc::Channel` for precisely this reason.

The remedy prescribed by SDR §3 is a `Reporter` trait owned by core, with the mode-gating logic living in the CLI's implementation of it. This change implements that seam and rewires all thirteen sites, entirely inside the existing single crate. Nothing moves between repositories; `Cargo.toml` is not touched. The success criterion is mechanical: `grep -rn "crate::cli" src/core src/services` returns zero non-test hits, and the CLI's observable output is byte-identical.

## Goals / Non-Goals

**Goals:**

- Remove all thirteen `core`/`services` → `cli` edges listed in SDR §3, leaving `grep -rn "crate::cli" src/core src/services` with zero non-test hits.
- Preserve CLI behaviour **exactly**: same text, same stream, same suppression under `--quiet` and `--output json`. Lock it with characterisation tests written *before* the rewiring.
- Preserve every public constructor signature the GUI depends on (SDR §8): `MatchEngine::new`, `SyncEngine::new`, `TranslationEngine::new`, `ComponentFactory::new`.
- Give the seam the shape D2 (`expose-core-orchestration-apis`) will extend for progress and cancellation, so D2 adds variants rather than redesigning the trait.
- Make the boundary self-enforcing so it cannot regress in the changes between A1 and B2.

**Non-Goals:**

- Moving any file between `src/core/`, `src/services/` and `src/cli/`. That is A2 (`relocate-misplaced-core-modules`) and B2 (`move-core-sources-into-subx-core`).
- Introducing a workspace, a submodule, or a second crate. That is B1.
- Changing what any message *says*, or adding/removing any message. This change is a rewiring, not a UX pass.
- Making the engines `Send`/`Sync` overall. `FormatConverter`, `TranslationEngine` and `FormatManager` hold boxed non-`Send` handlers (SDR §8 gap 1); fixing that is D1 (`make-core-engines-thread-safe`). The `Reporter: Send + Sync` bound here does not regress that situation and does not fix it.
- Building a structured progress/cancellation API. `ProgressEvent` ships with one variant and is `#[non_exhaustive]` precisely so D2 can do that work.
- Replacing `tracing`/`log`. Structured logging is orthogonal and stays exactly as it is; `machine-readable-output` already treats it as a separate, `RUST_LOG`-gated channel.
- Touching `src/cli/output.rs`. The `OnceLock` globals keep their current shape and location.

## Decisions

### Decision 1: The seam lives at `src/core/report/`, so it travels with core in B2

The new module is `src/core/report/mod.rs`, reached as `subx_cli::core::report`. It is declared in `src/core/mod.rs` alongside `archive`, `factory`, `file_manager`, … and documented in that module's header list.

**Why under `src/core/`:** SDR §2.1 moves `src/core/**` wholesale to `subx-core/src/core/**`. Putting the seam anywhere else (a new top-level `src/report/`, or inside `src/services/`) means B2 has to move it *separately* and update its import paths independently of the bulk `git mv`. Under `src/core/` it is carried along for free, and every consumer's `crate::core::report::…` path becomes `subx_core::core::report::…` under the same blanket `subx_cli::` → `subx_core::` rewrite B3 already applies to 89 test files.

**Relationship to SDR §2.1's `subx_core::report`:** SDR §2.1 names the eventual public path `subx_core::report`. That is a re-export decision for B2, not a file-layout decision for A1: B2 may add `pub use core::report;` (or `pub use crate::core::report::{Reporter, NoopReporter};`) to `subx-core/src/lib.rs` to surface the shorter path. Nothing in this change forecloses it, and adding the alias later is additive.

**Alternatives considered:**

- *A top-level `src/report/` today.* Rejected — it is one more directory for B2 to move and re-path, for zero benefit while the crate is still monolithic.
- *Put it in `src/services/`.* Rejected — `core` depends on `services` (e.g. `MatchEngine` holds `Box<dyn AIProvider>`), not the other way round; the seam would then be an upward dependency from every core engine.

### Decision 2: Trait shape — four channels, all with default no-op bodies

```rust
// src/core/report/mod.rs
pub trait Reporter: Send + Sync {
    /// Human-oriented detail about work in progress. Not a failure.
    fn diagnostic(&self, message: &str) { let _ = message; }
    /// A non-fatal problem the operation recovered from or worked around.
    fn warn(&self, message: &str) { let _ = message; }
    /// Token accounting for one completed AI API call.
    fn ai_usage(&self, usage: &AiUsage) { let _ = usage; }
    /// An event on the long-running-work progress stream.
    fn progress(&self, event: &ProgressEvent<'_>) { let _ = event; }
}

pub struct NoopReporter;
impl Reporter for NoopReporter {}

pub fn noop() -> std::sync::Arc<dyn Reporter> { std::sync::Arc::new(NoopReporter) }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiUsage {
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEvent<'a> {
    /// Free-form status line emitted while long-running work advances.
    Message(&'a str),
}
```

**Why default bodies:** an implementor that only cares about warnings writes `impl Reporter for MyThing { fn warn(&self, m: &str) { … } }` and gets silence everywhere else. `NoopReporter`'s entire implementation is `impl Reporter for NoopReporter {}`. When D2 adds structured `ProgressEvent` variants or a `cancelled()` query, existing implementors — including the GUI's — keep compiling.

**Why `&str` and not `fmt::Arguments` or a `Display` generic:** the trait must stay object-safe, because it is stored as `Arc<dyn Reporter>`. Call sites build the string with `format!` at the call site, which is what they already do inside `eprintln!`. The cost is one allocation per message on a path that was already doing terminal I/O.

**Why `ProgressEvent` is `#[non_exhaustive]` with one variant:** SDR §3 states this is "the same seam `expose-core-orchestration-apis` later extends for progress/cancel, so design it with that in mind from the start". `#[non_exhaustive]` means D2 can add `Started { total }`, `Advanced { done, total }`, `Finished` and friends without a breaking change, and every existing `match` in an implementor keeps compiling because it already needs a `_ =>` arm.

**Alternatives considered:**

- *One `report(&self, level: Level, message: &str)` method.* Rejected — it collapses `ai_usage`'s structured payload into a pre-rendered string, which is precisely what the GUI cannot use.
- *Separate `Reporter` and `ProgressSink` traits.* Rejected — two `Arc`s to thread through every constructor, and D2 would have to retrofit the second one into all the same call sites this change is already touching.

### Decision 3: Mode gating moves **into** the CLI's `Reporter` impl; core never learns that JSON mode exists

`src/cli/reporter.rs` gains `TerminalReporter`, and it is the **only** consumer of `output::active_mode()` and `output::is_quiet()` on behalf of core:

```rust
pub struct TerminalReporter;

impl Reporter for TerminalReporter {
    fn diagnostic(&self, message: &str) {
        if output::active_mode().is_json() { return; }
        eprintln!("{message}");
    }
    fn warn(&self, message: &str) {
        if output::active_mode().is_json() { return; }
        eprintln!("{message}");
    }
    fn ai_usage(&self, usage: &AiUsage) {
        ui::display_ai_usage(usage);          // already JSON-gated CLI-side
    }
    fn progress(&self, event: &ProgressEvent<'_>) {
        if output::active_mode().is_json() || output::is_quiet() { return; }
        match event { ProgressEvent::Message(m) => eprintln!("{m}"), _ => {} }
    }
}
```

**Why not replicate the gate in core:** the alternative is to give core its own mode enum and have the CLI push it down. That reproduces the coupling in a new shape — core would still branch on "am I in machine-readable mode", still need a global or a threaded-through parameter, and B2 would still have to decide who owns the enum. The whole point of the seam is that core says *what happened* and the transport decides *whether and how to show it*. Core does not know JSON mode exists; it does not know `--quiet` exists; it does not know stdout and stderr are different things.

**Why the globals stay:** `ACTIVE_MODE` and `QUIET` (`src/cli/output.rs:78-79`) are `OnceLock`s installed once by `install_active_mode` during dispatch. They are read by `ui.rs`, `main.rs` and the per-command JSON renderers — all CLI-side, all staying in `subx-cli`. Nothing about this change makes them worse, and threading the mode through every UI helper is a much larger and entirely unrelated refactor. After this change they are read *by core's behalf* only through `TerminalReporter`, which is the correct place for them.

**Why `TerminalReporter` reads the globals rather than capturing the mode at construction:** the mode is installed before any command runs, but `TerminalReporter` values are constructed inside command bodies; reading the `OnceLock` at call time keeps a single source of truth and matches exactly what the thirteen sites do today, so the characterisation tests hold.

### Decision 4: The channel → stream → suppression matrix reproduces today's output byte-for-byte

`TerminalReporter` defines exactly three rules, and each of the thirteen sites is assigned the channel whose rule matches what it does today:

| Channel | Stream | Suppressed by |
|---|---|---|
| `diagnostic` | stderr | JSON mode |
| `warn` | stderr | JSON mode |
| `ai_usage` | stdout (via `ui::display_ai_usage`) | JSON mode |
| `progress` | stderr | JSON mode **or** `--quiet` |

Site assignment:

| Site | Today's gate | Channel |
|---|---|---|
| `engine.rs:1363-1376` `🔍 AI Analysis Results:` block | `!json` | `diagnostic` |
| `engine.rs:1429-1434` `⚠️ Cannot find AI-suggested file pair` | `!json` | `warn` |
| `engine.rs:1570`, `:1583` dry-run `Preview:` lines | `!json` | `diagnostic` (see 4b) |
| `engine.rs:1914-1919` `Warning: Skipping relocation…` | `!json` | `warn` |
| `engine.rs:1962-1965` `Warning: Conflict resolution prompt not implemented…` | `!json` | `warn` |
| `engine.rs:2287-2295` `log_available_files` dump | `!json` | `diagnostic` |
| `engine.rs:2307-2330` `log_no_matches_found` dump | `!json` | `diagnostic` |
| `file_manager.rs:274` `Warning: Cannot restore removed file…` | `!json` | `warn` |
| `worker.rs:132-135` `Waiting for worker … to complete task …` | `!json && !quiet` | `progress` |
| `translation/engine.rs:292-294` unknown-cue-ID retry notice | `!json && !quiet` | `progress` (see 4a) |
| `translation/engine.rs:396-400` `📊 Translation Progress:` block | `!json && !quiet` | `progress` |
| `services/ai/{openai:520, azure_openai:274, openrouter:233, local:245}` | `!json` (inside `display_ai_usage`) | `ai_usage` |

Every message keeps its exact bytes. Multi-line blocks that are currently a run of consecutive `eprintln!` calls (the `🔍 AI Analysis Results:` block, `log_available_files`, `log_no_matches_found`) become a single `diagnostic` call carrying an embedded-`\n` string: N `eprintln!("a")…eprintln!("c")` and one `eprintln!("a\nb\nc")` emit identical bytes, and the single call additionally makes the block atomic against interleaving. Leading blank lines (`log_no_matches_found`'s `eprintln!("\n❌ …")`) are preserved by keeping the leading `\n` in the string.

### Decision 4a: The three `--quiet`-gated sites become the `progress` channel

Three sites (`worker.rs:131`, `translation/engine.rs:291`, `translation/engine.rs:395`) are gated on `is_quiet() || is_json()` while the other six core sites are gated on `is_json()` alone. That asymmetry is real, pre-existing, and CLI-observable, so it must be preserved.

The rule that reproduces it exactly and is still principled: **`--quiet` silences the progress stream; it does not silence diagnostics or warnings.** All three quiet-gated sites are events on a long-running-work stream — a worker-pool shutdown drain, a per-batch retry notice interleaved with the translation progress lines, and the translation progress lines themselves. All six JSON-only-gated sites are one-shot diagnostics or warnings about a decision the engine made.

The retry notice at `translation/engine.rs:291` is the one arguable case: its text opens with `⚠`, which reads like a `warn`. It is assigned to `progress` because it is emitted from inside the per-batch translation loop, interleaved between `📊 Translation Progress:` blocks, and shares their gate. `ProgressEvent::Message` is documented as covering free-form status chatter on the progress stream, retry notices included; D2 may promote it to a dedicated variant.

**Alternative considered:** *make `TerminalReporter::warn` honour `--quiet` too, and route the retry notice through `warn`.* Rejected — it would newly silence `Warning: Skipping relocation`, `Warning: Conflict resolution prompt not implemented` and `⚠️ Cannot find AI-suggested file pair` under `subx-cli --quiet match …` in text mode. Those are exactly the messages a user running `--quiet` still needs to see, and silencing them is an observable behaviour change this change has ruled out.

### Decision 4b: The dry-run `Preview:` lines move stdout → stderr, and that is not CLI-observable

`MatchEngine::execute_operations` (`src/core/matcher/engine.rs:1562-1594`) prints its dry-run preview with `println!` — stdout — while every other core site uses `eprintln!`. Routing it through `diagnostic` (stderr) would be a stream change.

It is not observable from the CLI, because **the CLI never reaches that branch**. `src/commands/match_command.rs:576-584` calls `engine.execute_operations(&operations, args.dry_run)` inside `if !args.dry_run { … }`, so `dry_run` is always `false` there; the JSON path at `:484` uses `execute_operations_audit`, whose dry-run branch prints nothing; and `apply_cached_operations` (`engine.rs:2354`) passes `false`. The only way to reach the preview loop is as a library caller — i.e. the GUI — which does not want stdout writes in its process at all.

So: route it through `diagnostic` like every other engine site, and record the stream change here rather than inventing a fifth channel or a stdout variant to preserve a code path no user can trigger. The `machine-readable-output` guarantee is unaffected: in JSON mode the reporter emits nothing on either stream, which is strictly what that spec requires.

**Alternative considered:** *hoist the preview loop into `src/cli/ui.rs` as `display_dry_run_preview` and call it from `match_command`'s dry-run path.* Rejected — it would *add* `Preview:` lines to `subx-cli match --dry-run` output, which no user sees today. That is a UX change, and this change is a rewiring.

### Decision 5: Constructors keep their signatures; the reporter attaches through `with_reporter`

Every affected type keeps its current constructor and gains a builder:

```rust
impl MatchEngine {
    pub fn new(ai_client: Box<dyn AIProvider>, config: MatchConfig) -> Self { /* reporter: report::noop() */ }
    pub fn with_reporter(mut self, reporter: Arc<dyn Reporter>) -> Self { self.reporter = reporter; self }
}
```

applied identically to `SyncEngine::new(SyncConfig) -> Result<Self>`, `TranslationEngine::new(Arc<dyn AIProvider>, usize) -> Result<Self>`, `ComponentFactory::new(&dyn ConfigService) -> Result<Self>`, `FileManager::new() -> Self`, `WorkerPool::new(usize) -> Self`, and the four AI clients' `new*`/`from_config` constructors. The field is always `Arc<dyn Reporter>`, always initialised to `report::noop()`.

**Why this is a hard requirement, not a style preference:** SDR §8 verified the GUI's real API contract across all 16 `.rs` files of `../subx/src-tauri/src/`. It constructs `MatchEngine::new` at `../subx/src-tauri/src/state.rs:992` and `TranslationEngine::new` at `../subx/src-tauri/src/commands/translate.rs:489`, and it uses `ComponentFactory` and `SyncEngine::new` as well. Adding a required `reporter` parameter to any of those four would break the GUI at exactly the moment the split is meant to make the GUI's life easier — a gratuitous break, for a value the GUI would pass as "none" until it implements its own reporter. `Result`-returning constructors (`SyncEngine`, `TranslationEngine`, `ComponentFactory`) chain as `SyncEngine::new(cfg)?.with_reporter(r)`, which reads cleanly.

**Why `Arc` and not `Box`:** `ComponentFactory` hands the same reporter to a `MatchEngine`, a `TranslationEngine`, a `FileManager` and an AI client from one call; `WorkerPool` is `Clone` (`src/core/parallel/worker.rs:155-162`). `Arc<dyn Reporter>` clones for the price of a refcount bump. `Box` would force either a `Clone`-on-`dyn` dance or one reporter per consumer.

**Why not `Option<Arc<dyn Reporter>>`:** `NoopReporter`'s methods are empty default bodies, so a call through the vtable is a no-op the optimiser can flatten; an `Option` would add a branch at every site and an `unwrap_or` dance in every constructor for nothing.

**`SyncEngine` has no print sites today** — it is included because SDR §3 names it in the constructor-compatibility list and because D2 attaches its progress hook there. It stores the reporter and does not yet use it; `src/core/mod.rs:17` already carries `#![allow(dead_code)]`.

### Decision 6: `ComponentFactory` propagates its reporter into everything it builds

`ComponentFactory` (`src/core/factory.rs:42-58`) gains the same `reporter: Arc<dyn Reporter>` field and `with_reporter` builder, and passes it down in `create_match_engine` (`:69-82`), `create_file_manager` (`:86-92`), `create_translation_engine` (`:146-160`) and `create_ai_provider` (`:103-105`).

The free function `create_ai_provider(&AIConfig) -> Result<Box<dyn AIProvider>>` (`src/core/factory.rs:213-240`) keeps its signature and delegates to a new `create_ai_provider_with_reporter(&AIConfig, Arc<dyn Reporter>)`; the factory method calls the latter with its own reporter. The reporter must be attached to the *concrete* client before it is boxed — `with_reporter` cannot be called on a `Box<dyn AIProvider>`.

**Why this matters:** `match_command.rs:283-284` and `:319-320` obtain their AI client from `factory.create_ai_provider()`, and that client is then moved into `MatchEngine::new` at `:436`. Without factory propagation the command would have to know which concrete provider it got in order to attach the reporter. One `factory.with_reporter(cli::terminal_reporter())` at the top of a command wires the whole command.

### Decision 7: `FileManager` and `WorkerPool` take the reporter as a constructor-optional field, not per call

Both are constructed in more places than the engines, and both are the reason the question arises: `FileManager::new()` appears at `src/core/factory.rs:91`, `src/commands/convert_command.rs:385` and in eight rustdoc examples plus unit tests; `WorkerPool::new(n)` appears only in `src/core/parallel/worker.rs`'s own unit tests.

**Chosen: a constructor-optional field with a `with_reporter` builder**, identical to Decision 5. `FileManager::new()`, `impl Default for FileManager` (`:300-304`) and `WorkerPool::new(max_workers)` keep their exact signatures, so all eight rustdoc examples and every unit test keep compiling untouched — which matters, because `broken_intra_doc_links = "deny"` and all doc examples must compile (AGENTS.md).

**Rejected: passing `&dyn Reporter` per call.** It would change `FileManager::rollback(&mut self) -> Result<()>` and `WorkerPool::shutdown(&self)`, both of which the GUI can reach (`core::file_manager::FileManager` is on the SDR §8 consumed list), and it would force every rustdoc example that calls `rollback()` to construct a reporter. It also scales badly: D2 will want a reporter on `execute`/`execute_operations` too, and per-call threading multiplies with each such method.

**On the behaviour of these two under a `NoopReporter`:** neither message is reachable from a CLI command today. `FileManager::rollback` is called only from its own unit test (`src/core/file_manager.rs:123`) — `convert_command.rs:385` calls `remove_file` and never rolls back — and `WorkerPool` is constructed only in unit tests. No test asserts on either string. Wiring `TerminalReporter` into `ComponentFactory::create_file_manager` therefore restores the message for the one production `FileManager` that exists, and the `WorkerPool` message stays where it is: silent, because nothing in production constructs a `WorkerPool`. Both facts are recorded in `tasks.md` so the phase-1 characterisation tests do not chase output that no invocation can produce.

### Decision 8: `AiUsage` is core-owned, and `services::ai::AiUsageStats` becomes an alias for it

`AiUsage` is defined in `src/core/report/mod.rs` with the four fields `display_ai_usage` prints. `src/services/ai/mod.rs:322-333` — which today *defines* `AiUsageStats` — instead re-exports it:

```rust
pub use crate::core::report::AiUsage as AiUsageStats;
```

There is exactly one struct. `AiResponse.usage: Option<AiUsageStats>` (`:341`), `ui::display_ai_usage(usage: &crate::services::ai::AiUsageStats)` (`src/cli/ui.rs:303`), the four clients' `AiUsageStats { … }` literals and every test that names `AiUsageStats` keep compiling unchanged — a `pub use … as` alias supports struct-literal construction and pattern matching identically to the original path.

**Why core-owned rather than `Reporter::ai_usage(&AiUsageStats)`:** the seam should not force `core::report` to depend on `services::ai`. Core already depends on services elsewhere (`MatchEngine` holds a `Box<dyn AIProvider>`), so it would compile — but `report` is the one module every other module reports *through*, and giving it a dependency on the AI service layer makes it the wrong kind of hub. A four-`u32`-and-a-`String` value type has no business living behind an AI provider module.

**Why an alias rather than two structs plus `From`:** duplication with a conversion is strictly worse — every provider would build one struct and convert, `AiResponse` would have to pick a side, and the two would drift. AGENTS.md forbids introducing new `#[deprecated]`, so `AiUsageStats` is documented as a legacy alias in rustdoc prose only, mirroring SDR D11's treatment of the back-compat re-exports.

`AiUsage` derives `Debug, Clone, PartialEq, Eq` — `PartialEq` is new and lets the recording test double assert on reported usage without hand-written comparisons.

### Decision 9: `Reporter: Send + Sync`

The supertrait bound is required, not decorative:

- `WorkerPool::execute` (`src/core/parallel/worker.rs:49`) `tokio::spawn`s tasks; anything the pool holds must be `Send + Sync` to be observed from a spawned task, and `WorkerPool` is `Clone`d across them.
- `MatchEngine::match_file_list_with_audit`, `TranslationEngine::translate_*` and `SyncEngine`'s methods are `async` and are held across `.await` points; the whole future must be `Send` for `tokio::spawn` and for the multi-threaded runtime the CLI uses (`tokio` `rt-multi-thread`, SDR §4).
- The GUI drives these engines from `spawn_blocking` closures and Tauri command handlers, which require `Send` captures (SDR §8 gap 1).

`Arc<dyn Reporter>` is `Send + Sync` automatically once `Reporter: Send + Sync`, so adding the field does not itself make any type less thread-safe. It also does not *fix* SDR §8 gap 1 — the non-`Send` boxed handlers inside `FormatConverter`/`TranslationEngine`/`FormatManager` are D1's problem — but it deliberately avoids adding a fourteenth reason those types cannot cross a thread boundary.

A `TerminalReporter` is a unit struct that writes with `eprintln!`/`println!`, both of which take the corresponding `std` stream lock; it is trivially `Send + Sync`.

### Decision 10: The boundary is enforced by a guard test, not by a lint

`tests/core_cli_boundary.rs` walks every `.rs` file under `src/core/` and `src/services/`, resolved from `CARGO_MANIFEST_DIR` (never a CWD-relative path — SDR §6 records that CWD assumptions break when files move), and fails with the offending `file:line` list if any line contains `crate::cli`.

**Why a test and not a clippy lint or `#![deny]`:** there is no stable rustc or clippy lint for "module A may not name module B" within one crate. Tools that can express it (`cargo-deny`'s bans, architecture-lint crates) operate at crate granularity, which is exactly the granularity this crate does not yet have. A ten-line test is deterministic, needs no new dependency (SDR §4 already prunes dead ones in A0), and is parallel-safe with no global state (AGENTS.md).

**Why it matters between now and B2:** A2, B1 and B3 all touch these files. Without a guard, one reflexive `use crate::cli::…` re-introduces the coupling and B2 discovers it mid-`git mv` — the precise failure mode the A-series exists to prevent. The guard is also the executable form of the `core-reporting` capability's layering requirement.

The test tolerates the string inside `//!`/`///` doc comments only if it is part of a fenced path in prose; to keep it simple and unambiguous the implementation matches on the token `crate::cli` anywhere in a non-comment line, and the rewiring in phases 4–5 leaves zero occurrences of any kind, comments included.

## Risks / Trade-offs

- **Risk: a message silently changes stream or disappears during the rewiring.** → Mitigation: phase 1 writes characterisation tests *before* any source change — text-mode assertions for the matcher chatter (extending `tests/cli/match_command_json_silence.rs`, which already has `human_mode_dry_run_still_prints_ai_analysis_results`), a new `ai_usage_output_characterization` test asserting the `🤖 AI API Call Details:` block lands on **stdout**, and a `translation_progress_characterization` test asserting `📊 Translation Progress:` on stderr in text mode and absent under `--quiet`. Every one of them must still pass, unchanged, after phase 6.
- **Risk: `--quiet` semantics drift for the six JSON-only-gated sites.** → Mitigation: Decision 4a fixes the rule as "`--quiet` silences `progress` only" and the characterisation tests assert that `subx-cli --quiet match …` in text mode still prints the matcher's warnings.
- **Risk: the `AiUsageStats` → `AiUsage` alias breaks a downstream consumer.** → Mitigation: it is a `pub use … as` type alias, so the old path resolves to the same type for construction, matching and trait impls. SDR §8's verified GUI consumption list does not include `AiUsageStats` at all, so nothing downstream even names it.
- **Risk: a required-parameter constructor sneaks in and breaks the GUI.** → Mitigation: Decision 5 is a hard constraint with named GUI call sites; `tasks.md` phase 3 states "signature unchanged" on every constructor task, and phase 9's `cargo test --doc --all-features` catches any rustdoc example that had to change shape — a changed example is the signal that a signature moved.
- **Risk: `#[non_exhaustive] ProgressEvent` with a single variant forces a `_ =>` arm on every implementor for no present benefit.** → Mitigation: accepted deliberately. Paying one wildcard arm now is cheaper than D2 shipping a breaking enum change to the GUI's reporter later.
- **Risk: multi-line blocks collapsed into one `diagnostic` call change the bytes.** → Mitigation: N consecutive `eprintln!` calls and one `eprintln!` with embedded `\n` produce identical bytes; the characterisation tests assert on the full block text including its leading `\n` where one exists (`log_no_matches_found`).
- **Risk: `FileManager` and `WorkerPool` become silent because nothing wires a reporter.** → Mitigation: Decision 7 records that neither message is reachable from a CLI command today and that no test asserts either string; `ComponentFactory::create_file_manager` is wired so the one production `FileManager` does report.
- **Risk: someone re-introduces `crate::cli` in core between A1 and B2.** → Mitigation: Decision 10's guard test, and the `core-reporting` capability requirement it enforces.
- **Risk: scope creep into D1 (thread safety) or D2 (progress API).** → Mitigation: the non-goals are explicit; `ProgressEvent` ships with one variant and `SyncEngine` stores a reporter it does not yet use, both marked as D2's extension points.

## Migration Plan

1. Land the phase-1 characterisation tests against **unmodified** source; confirm they pass and describe today's behaviour.
2. Land `src/core/report/mod.rs` (`Reporter`, `NoopReporter`, `noop`, `AiUsage`, `ProgressEvent`) with its own unit tests, declared from `src/core/mod.rs`. Nothing consumes it yet.
3. Turn `services::ai::AiUsageStats` into the alias (Decision 8). The crate still compiles with no other change.
4. Add the `reporter` field and `with_reporter` builder to `MatchEngine`, `SyncEngine`, `TranslationEngine`, `ComponentFactory`, `FileManager`, `WorkerPool` and the four AI clients. All default to `report::noop()`; no call site changes yet.
5. Rewire the nine `src/core/**` sites to the reporter channels per Decision 4; delete every `crate::cli::output::…` read.
6. Rewire the four `src/services/ai/**` sites to `Reporter::ai_usage`; delete every `use crate::cli::display_ai_usage;`.
7. Land `src/cli/reporter.rs` with `TerminalReporter`, export it from `src/cli/mod.rs`, and wire it in at the five command construction sites. At this point the characterisation tests from step 1 must pass unchanged — that is the gate for the whole change.
8. Land the boundary guard test and confirm `grep -rn "crate::cli" src/core src/services` is empty.
9. Documentation, `[Unreleased]` CHANGELOG entry, then the quality gate on the main agent only.
10. Rollback: revert in reverse order. Steps 2–4 are additive and safe to leave in place if only the rewiring needs reverting; the alias in step 3 is the only item with a public-API footprint, and reverting it is symmetric.

## Open Questions

_None._ SDR §3 fixes the seam's shape, SDR §8 fixes the constructor-compatibility constraint, and the stream/suppression matrix in Decision 4 is derived directly from the thirteen call sites as they stand.
