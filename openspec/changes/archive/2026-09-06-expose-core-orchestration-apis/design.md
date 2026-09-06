## Context

This is the fourteenth and last change of the crate split, and the second of the two that exist purely for the downstream consumer. SDR §8 verified `../subx`'s real API contract across all sixteen `.rs` files of its `src-tauri/src/` and recorded six gaps that force it to duplicate logic `subx-core` owns. D1 closed gaps 1 and 2 — the missing `Send + Sync` on the format-driving engines, and the VAD-independent manual offset — for a payoff of ~163 lines. This change closes the remaining four.

The four are not variations on one theme, and it is worth saying what each actually is before designing anything.

Gaps 3, 4 and 5 are the same defect three times: a decision that is settled, correct and tested, expressed in a place no library caller can reach. `create_match_engine()` decides six `MatchConfig` fields from configuration and then pins the two the caller chooses; convert decides where an archive-extracted output lands, inside a `for` loop; match decides that an archive-extracted subtitle relocates to the video's directory, in a second `for` loop. Each is small. Each is copied verbatim into `../subx`, with a comment explaining that it is a copy. Fixing all three is function extraction and nothing more — no behaviour changes, no signature changes, no new dependencies.

Gap 6 is different, and the difference matters enough to change how it is scoped. SDR §8 frames it as "the GUI re-implements progress entirely over `tauri::ipc::Channel` and cancels via `AtomicBool` / `tokio::sync::watch`", which reads as another duplication to delete. Reading that code says otherwise. `ConvertProgress` and `TranslateProgress` (`../subx/src-tauri/src/dto.rs:277-299`, `:506-529`) are emitted by the GUI's **own** batch loops, over a transport `subx-core` cannot supply and should not know about, and both carry a comment saying so: *"Per file, never finer: the crate converts a whole file in one call and reports nothing from inside it, so a percentage would be invented."* The three local reporter traits — `ProgressReporter` (`commands/match.rs:139`), `ConversionReporter` (`convert.rs:112`), `TranslateReporter` (`translate.rs:162`) — are Tauri channel adapters and test seams, not copies of anything in core. **None of them can be deleted, and this change does not claim they can.**

What is genuinely missing is narrower and sharper: core-owned loops report nothing from inside themselves, so a caller driving one cannot show progress or stop it. There is exactly one such loop on the GUI's critical path — `MatchEngine::execute_operations_audit` (`engine.rs:1743`, loop at `:1783`) — and `../subx`'s match wizard is, as a direct result, the only one of its four with neither a progress channel nor a cancel command (`commands/match.rs:340-345` simply awaits the whole batch). On the CLI side the same absence has produced a live defect, which is the part of gap 6 with a bug number attached: `match_command.rs:807-813` defines a private `create_progress_bar` that shadows `cli::ui::create_progress_bar`, skips `ui::progress_draw_target_for`, and therefore leaks progress frames into `--output json`.

So gap 6's payoff is a capability the consumer cannot build today plus one CLI defect closed, not a line count. Saying that plainly is more useful to whoever reads this next than repeating SDR §8's framing would be.

## Goals / Non-Goals

**Goals:**

- Close SDR §8 gaps 3, 4, 5 and 6 with additions only: no signature change, no removal, no path reshape, nothing `cargo-semver-checks` calls major.
- Extend A1's `Reporter` seam exactly as A1 designed it to be extended — variants on an already-`#[non_exhaustive]` enum and a provided trait method — rather than redesigning the trait.
- Derive the `ProgressEvent` variant set from call sites that exist, and refuse to add a variant no emitter needs.
- Make the CLI the first adopter of every new item, so the two implementations cannot drift, and delete `match_command.rs:807-813` in the process.
- Leave `../subx` able to delete named line ranges, and say which ones.
- Preserve the GUI's ~40-item consumed surface (SDR §8) and B2's *Public API Path Stability for the Library Surface* exactly.

**Non-Goals:**

- Supplying a progress *transport*. `tauri::ipc::Channel`, `indicatif` and stdout are all transports; `Reporter` is the seam that keeps core ignorant of which one is in use, and that is the whole of core's job here.
- Deleting `../subx`'s three local reporter traits. They are channel adapters and test seams (see Context). SDR §8 gap 6's framing implies otherwise; this change corrects the record rather than acting on it.
- Changing what any existing message says or which stream it lands on. A1 froze the thirteen rewired sites byte-for-byte and locked them with characterisation tests; those tests must still pass unchanged. In particular `translation/engine.rs:395`'s `📊 Translation Progress:` block **keeps** its `ProgressEvent::Message` call and is not promoted to `Advanced` (Decision 4).
- Adding cancellation to `MatchEngine::execute_operations`, `TranslationEngine::translate_subtitle`, or `SyncEngine`. Decision 6 gives the reason for each.
- Making `MatchConfig` `#[non_exhaustive]`. That is itself a breaking change; Decision 2 records why `match_config()` is the migration path that makes it cheap later.
- Exposing translate's `translated_file_name` / `backup_path`, which `../subx` also mirrors (`translate.rs:704-708`, `:735-742`). Decision 3 records the finding and declines it with a reason.
- Touching `src/cli/output.rs`, the clap surface, or anything in `commands/` beyond adopting the new APIs.

## Decisions

### Decision 1: D2's slot is after D1 and **before C2a**, not last — and the reason is that its deltas target core-classified requirements

`tmp/split-implementation-plan.zh-TW.md` §4 puts D2 alone in batch 10, after everything. That placement is wrong for the same class of reason D1's was, and the mechanism is worth stating because it is not obvious.

Four of this change's five delta files target requirements C2b classifies **C** (core) or that C2a migrates wholesale. After C2a and C2b archive, those requirements no longer live in `subx-cli/openspec/specs/` — they live in `subx-core/openspec/specs/`, which this repository's OpenSpec root never descends into (C2a verified that). A change authored under `subx-cli/openspec/changes/` therefore **cannot** delta them at all: `core-reporting` would not exist here, and `input-path-handling`, `subtitle-matching` and `component-factory` would each hold only their CLI residue.

So the slot is: **after C1, after D1, before C2a.** That window is the same one D1 occupies and is permitted for the same reason — the plan's §2 records C1 and C2a/C2b as mutually independent. Concretely this change requires B2 (its edits are in `subx-core/src/**`), A1 (the seam it extends), A2 (`core::input`, which owns `CollectedFiles`), C1 (`subx-core/CHANGELOG.md` exists and `scripts/quality_check.sh` passes `--workspace`) and D1 (file overlap in `matcher/engine.rs` and `core/report/`, which the plan's §3 matrix marks 🔴 between D1 and D2).

**Alternatives considered:**

- *Land last, and author the core-side deltas inside a `subx-core/openspec/changes/` change.* Rejected as the default, but recorded as the fallback: it is what task 1.1 instructs if C2a has already archived. It is worse because it splits one change's artifacts across two OpenSpec roots and two repositories, and because C2b's classification of `input-path-handling`'s *Output Directory Resolution for Archive Files* would then already be committed as **L** — a decision Decision 8 has to correct.
- *Land before D1.* Rejected — the plan's dependency table records D1 → D2, both touch `matcher/engine.rs` and `core/report/`, and D1 has the harder precondition (its supertrait must precede publication). D1 goes first and this change absorbs the sequencing risk, which is nil because nothing waits on it.

### Decision 2: gap 3 is a configuration accessor plus a config-taking constructor, not a relocation-mode parameter and not a builder

```rust
// subx-core/src/core/factory.rs
impl ComponentFactory {
    pub fn match_config(&self) -> MatchConfig { /* today's literal, verbatim */ }
    pub fn create_match_engine_with(&self, config: MatchConfig) -> Result<MatchEngine> { … }
    pub fn create_match_engine(&self) -> Result<MatchEngine> {
        self.create_match_engine_with(self.match_config())
    }
}
```

`match_config()` returns exactly what `create_match_engine()` builds today (`factory.rs:70-79`), including the `confidence_threshold: 0.8` literal and its "Default value, can be configurable" intent, so `create_match_engine()`'s behaviour is bit-identical after the delegation.

**Why an accessor and not a parameter.** The obvious shape is `create_match_engine_with(relocation_mode: FileRelocationMode)`, which is exactly what gap 3's wording suggests. It is wrong on the evidence: **both** real callers override *two* fields, not one. `match_command.rs:423-433` sets `confidence_threshold: args.confidence as f32 / 100.0` and `backup_enabled: args.backup || config.general.backup_enabled` alongside the relocation mode; `../subx/src-tauri/src/commands/match.rs:264-273` sets `confidence_threshold: DEFAULT_CONFIDENCE as f32 / 100.0` and the mode. A one-parameter constructor would have needed a second parameter within the same change that introduced it.

**Why not a `MatchConfigBuilder`.** `MatchConfig`'s eight fields are all `pub`. A builder would be a new public type with eight setters whose only job is to write fields the caller can already write, and it would leave the *actual* problem — knowing which six values come from `Config` — unsolved. `let mut c = factory.match_config(); c.relocation_mode = mode;` solves it in two lines with no new type.

**Why `create_match_engine_with` still earns its place**, given that both current callers build their own AI provider and could stop at `match_config()`: it is the only route to *the factory's own* provider with a caller-chosen config, which is literally what gap 3 says is missing, and it is where A1 Decision 6's reporter propagation happens. Without it, a caller who wants both has to reach for the free `create_ai_provider` and re-do the wiring the factory exists to own. It is three lines.

**The semver hazard this leaves standing, stated so nobody thinks it was missed.** `MatchConfig` is `pub` with all-`pub` fields and no `#[non_exhaustive]`, so adding a ninth field is a major break for every struct-literal construction — of which there are three today (core's factory, `match_command.rs`, and `../subx`). Adding `#[non_exhaustive]` would fix that going forward and is **itself** a major break right now, breaking all three. It is therefore not done here. `match_config()` is the migration path: once no caller writes a literal, `#[non_exhaustive]` costs nothing, and a future change can take it. Recorded in `subx-core/CHANGELOG.md`'s entry so the option is not lost.

`ComponentFactory::new`'s signature is untouched, which A1 Decision 5 makes a hard constraint on the strength of the GUI's call sites.

### Decision 3: gap 4 is two queries on `CollectedFiles`, and they deliberately are not defined in terms of each other

```rust
// subx-core/src/core/input/mod.rs, beside `archive_origin`
impl CollectedFiles {
    pub fn default_output_dir<'a>(&'a self, input: &'a Path) -> &'a Path;
    pub fn default_output_path(&self, input: &Path, extension: &str) -> PathBuf;
}
```

`default_output_dir` is the general rule, and its body is `translate_command.rs:387-391` verbatim: `self.archive_origin(input).and_then(Path::parent).or_else(|| input.parent()).unwrap_or(Path::new("."))`.

`default_output_path` is convert's resolver, and its body is `convert_command.rs:362-370` verbatim:

```rust
match self.archive_origin(input) {
    Some(archive) => {
        let dir = archive.parent().unwrap_or(Path::new("."));
        let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        dir.join(format!("{stem}.{extension}"))
    }
    None => input.with_extension(extension),
}
```

**Why methods on `CollectedFiles` and not free functions.** Both queries need `archive_origin`, which is `CollectedFiles`' own data; `CollectedFiles` is already on the GUI's consumed list; and an inherent method sits one line from `archive_origin` in the rustdoc, which is where a caller looking for "where does the output go" will actually look. A free function taking `&CollectedFiles` would put the same code one module away for no gain.

**Why `default_output_path` is not `default_output_dir(input).join(name)`, which is the thing a reviewer will want to "fix".** In the no-archive case today's code is `input.with_extension(ext)`. When `input` has no parent — a bare relative filename, which `subx convert a.srt` produces — `with_extension` yields `a.vtt` while `default_output_dir` yields `Path::new(".")` and the join yields `./a.vtt`. Same file, different `PathBuf`, and `convert_command.rs:379` prints it: `✓ Conversion completed: a.srt -> ./a.vtt` instead of `-> a.vtt`. That is a CLI-observable text change for a cosmetic refactor, so the two functions stay distinct, the divergence is stated in both rustdocs, and a spec scenario asserts it. This is the one place in the change where the obvious simplification is wrong, and it is the reason the requirement's core half names both methods rather than one.

**Which of the five call sites adopt, and why two do not.** C2b's evidence for classifying *Output Directory Resolution for Archive Files* as **L** is five sites in four files. Three adopt:

| Site | Adopts | Note |
|---|---|---|
| `convert_command.rs:362-370` | `default_output_path(input, &fmt)` | the `else`/`else if` arms of the three-way `if`; the `--output` arm at `:348-361` stays in the CLI |
| `translate_command.rs:383-392` `default_output_path` | `default_output_dir(input).join(translated_file_name(..))` | body becomes one line; the private fn stays as the naming half |
| `../subx` `convert.rs:494-506`, `translate.rs:725-730` | both, in the follow-up PR | Decision 10 |

`sync_command.rs:511-519` and `:583-591` do **not** adopt. Their shape is not "resolve a path" but "conditionally overwrite an `Option<PathBuf>` when, and only when, the subtitle came from an archive" — if the subtitle did not, `single_args.output` must stay `None` so `run_single` derives it downstream through `create_default_output_path`. Rewriting them to always compute a path would turn `output: None` into `output: Some(..)` on the non-archive path, and `sync_command`'s force/overwrite handling distinguishes those two states. The gain would be replacing `archive_path.parent()` with a call; the risk is a behaviour change in the `--force` interaction. Declined, and named here so the next reader does not think the sites were overlooked. This is also why the requirement becomes a **split** rather than migrating wholesale (Decision 8).

**A seventh duplication, found while sizing this and not in SDR §8.** `../subx` also mirrors translate's private `translated_file_name` (`translate.rs:704-708`) and `backup_path` (`:735-742`), and says so in its module doc (`:12-18`). It is the same class of gap as 4. It is **out of scope**: both are naming functions rather than location functions, they belong to `subtitle-translation`'s *Safe Output Behavior*, which C2b classifies **L**, and moving them would open a sixth capability and a second reclassification hand-off for ~14 lines. Recorded in `## Open Questions` as the follow-up it is.

### Decision 4: gap 5 is a free function beside `apply_unique_target_paths`, because the pairing is the point

```rust
// subx-core/src/core/matcher/engine.rs
pub fn apply_archive_origin_relocation(
    operations: &mut [MatchOperation],
    collected: &CollectedFiles,
);
```

Body: `match_command.rs:456-466` verbatim — for each operation whose subtitle has an `archive_origin` and which does not already require relocation, set `relocation_target_path = video_dir.join(&op.new_subtitle_name)`, `requires_relocation = true`, `relocation_mode = FileRelocationMode::Copy`.

**Why here and not in `core::input`.** The function mutates `MatchOperation`, which is the matcher's type. Putting it in `core::input` would make the input module depend on the matcher, inverting the direction that holds today. Putting it in the matcher means `engine.rs` gains a `core::input::CollectedFiles` import, which is a sideways edge inside `core/` and harmless.

**Why a free function and not a `MatchEngine` method.** `apply_unique_target_paths` is already a free function over `&mut [MatchOperation]` and is already on the GUI's consumed list. The two must be called in that order, by the same caller, over the same slice. Making one a method and one a free function would hide the pairing; as neighbours in one module with cross-referencing rustdoc, a caller who finds either finds both.

**Rejected: fold it into `apply_unique_target_paths`.** One call instead of two is tempting and would make the ordering unforgettable. It is rejected twice over: `apply_unique_target_paths(&mut [MatchOperation])` is consumed by `../subx` at its current arity, so adding a `&CollectedFiles` parameter is a signature break; and the allocator is meaningful without a `CollectedFiles` at all (the CLI's parallel path and the engine's own tests call it on operations that never touched an archive).

**Rejected: do it inside `match_file_list_with_audit` so no caller has to.** The engine receives `&[PathBuf]`, not a `CollectedFiles`; it has no way to know a path's provenance. Threading `CollectedFiles` into the engine would change `match_file_list_with_audit`'s signature, which the GUI calls at `commands/match.rs:342`.

**What this does to `subtitle-matching`.** The requirement's allocator paragraph says the allocator runs "after all operations have been generated **and** `match_command` has applied any archive-origin forced relocation", and its last scenario says "`match_command.rs` rewrites `relocation_target_path`". After this change the *rewrite* is core's and only the *call* is the command's, so both sentences need restating — and the restatement is strictly better, because "a named core function, called before the allocator" is checkable while "the command has applied any" is not. C2b lifts that clause into a CLI requirement (*Match Command Applies Archive-Origin Relocation Before Uniqueness Allocation*, Decision 4's one deliberately-ungathered clause); that requirement survives this change with a named function in it instead of an inline loop.

### Decision 5: the `ProgressEvent` variant set, derived from the four emitters that exist

The rule applied: **a variant is added only if a core-owned loop can emit it today with data it already has.** Four candidate emitters were examined.

| Candidate emitter | Data it holds | Verdict |
|---|---|---|
| `MatchEngine::execute_operations_audit` loop (`engine.rs:1783`) | `operations.len()`, the loop index, `op.subtitle_file.name` | **emits.** N discrete units, known up front, each with a name |
| `MatchEngine::execute_operations` loop (`engine.rs:1626`) | same | **emits** (progress only — Decision 6) |
| `TranslationEngine`'s per-batch loop (`translation/engine.rs:395`) | completed/total batches | **does not emit** — its bytes are frozen (see below) |
| `services::ai` retry notices (`translation/engine.rs:291`) | a string | **already covered** by A1's `Message` |

That yields:

```rust
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEvent<'a> {
    /// A1: free-form status chatter, retry notices included.
    Message(&'a str),
    /// A unit-counted stream is opening. `total` is the number of units.
    Started { total: u64 },
    /// `done` of `total` units are complete. `item` names the unit just finished.
    Advanced { done: u64, total: u64, item: Option<&'a str> },
    /// The stream is closing. `done < total` means it stopped early.
    Finished { done: u64, total: u64 },
}
```

**Why `Started`/`Advanced`/`Finished` and not a single event.** `indicatif` needs the total before it can draw a bar, and `../subx`'s `ConvertProgress`/`TranslateProgress` both carry `index`, `total` and a name — an independently-arrived-at confirmation that a consumer wants exactly these three fields. A single `Advanced { done, total }` without `Started` would force a renderer to create its bar lazily on the first item, which loses the empty-batch case (`total: 0`, no items, but the user should still see the batch open and close).

**Why `item: Option<&'a str>` and not a required `&'a str`.** The audit loop has `op.subtitle_file.name`; the CLI's `monitor_batch_execution` (`match_command.rs:764-805`) advances on task completion and has no per-task name to hand out. `Option` covers both without a second variant.

**Why `Finished { done, total }` and not a bare `Finished`.** `done < total` at close is exactly the signal "this stopped early", which is what a cancelled audit loop produces (Decision 6). A bare `Finished` would force a consumer to track the counts itself to notice, and `../subx`'s convert/translate reports already carry a `cancelled: bool` for precisely this distinction (`dto.rs:339`, `:569`).

**Why no `label`, no stream id, no `Failed` variant.** No emitter has a label to give that the CLI does not already print itself (`match_command.rs:726-728`); a stream id would only earn its place with concurrent streams, which do not exist (see the requirement's one-open-stream rule and the rejected alternative below); and a failure is already an `Err` or an `OperationOutcome.error`, so a `Failed` variant would be a second, weaker channel for something the return type says better.

**Why the translation site is not promoted to `Advanced`, even though it is the one place with a natural `done`/`total`.** `translation/engine.rs:395`'s `📊 Translation Progress:` block is one of A1's thirteen rewired sites. A1 Decision 4 assigned it to `progress`, froze its bytes, and required a `translation_progress_characterization` test asserting it on stderr in text mode and absent under `--quiet`, which "must still pass, unchanged". Promoting it to `Advanced` would make the CLI render a bar where a text block is today — a UX change to a frozen site, breaking A1's test, for a consumer (`../subx`) whose own comment says it wants per-file and not finer granularity. Left alone. If a later change wants mid-file translation progress, `Advanced` is already there for it to use, which is the whole benefit of `#[non_exhaustive]`.

**The `Eq` trap, worth one sentence.** A1 derived `Debug, Clone, PartialEq, Eq` on `ProgressEvent`. Every field added here is `u64` or `Option<&str>`. Had any variant carried an `f32` percentage — the obvious thing to want on a progress event — the `Eq` derive would have failed to apply, and `cargo-semver-checks` reports a removed derive as **major**. Counts, not fractions; a renderer divides.

**Alternative considered and rejected: a `stream: u64` token on all three variants,** so a reporter can demultiplex concurrent progress streams. Rejected on the evidence that no two streams can be open at once today — the CLI's parallel path and its sequential execution path are mutually exclusive branches of one command, and core opens no stream from inside another. The requirement states the one-open-stream rule normatively so a future emitter that breaks it has to say so; adding the token then is additive.

### Decision 6: cancellation is `fn cancelled(&self) -> bool` on `Reporter`, and it is polled between items only

```rust
pub trait Reporter: Send + Sync {
    // … A1's diagnostic / warn / ai_usage / progress, unchanged …
    fn cancelled(&self) -> bool { false }
}
```

**Weighed against the two mechanisms `../subx` already uses.** They are not two designs for one problem; they are two granularities, and the split between them is the whole answer.

| Consumer | Mechanism | Granularity | Does core need to help? |
|---|---|---|---|
| convert (`commands/convert.rs:255`, `:300`) | `Arc<AtomicBool>`, `load(SeqCst)` | **between files**, in the GUI's own loop | No — the loop is the GUI's |
| translate (`commands/translate.rs:397`, `:500`, `:524-537`) | `tokio::sync::watch::Receiver<bool>`, `tokio::select!` against `translate_one` | **mid-file**, inside a core `.await` | No — dropping the losing branch cancels it |
| match (`state.rs:163`, `:192`) | `AbortHandle` + epoch guard | whole task | No — abort drops the future |
| sync (`commands/sync.rs:143-169`) | `AbortHandle` + epoch guard | whole task | No |

The load-bearing observation is in the translate row. Mid-`.await` cancellation in Rust is achieved by **dropping the future**, and `tokio::select!` does exactly that — the GUI's comment says so: *"Dropping the losing `translate_one` branch cancels its pending HTTP request."* Core needs no API for that case and cannot improve on it; all it must do is be drop-safe, which it already is. The same is true of the two `AbortHandle` cases.

So the only case core can serve is the one none of the four covers: **stopping between the items of a loop core owns.** For that, a poll is not a compromise, it is the right shape — the check happens at the top of an iteration that is about to do filesystem work, there is nothing to wake, and `AtomicBool::load` is what the GUI itself uses at that granularity.

**Why on `Reporter` rather than a separate `CancellationToken`.** A1 Decision 2 already rejected splitting `Reporter` into `Reporter` + `ProgressSink`, and named the cost: "two `Arc`s to thread through every constructor, and D2 would have to retrofit the second one into all the same call sites". A separate token is that same cost, arriving one change later — a second `Arc<dyn …>` field, a second `with_*` builder, on `MatchEngine`, `TranslationEngine`, `SyncEngine`, `ComponentFactory`, `FileManager`, `WorkerPool` and four AI clients. A1 also anticipated this exact method by name: its Decision 2 says *"When D2 adds structured `ProgressEvent` variants **or a `cancelled()` query**, existing implementors keep compiling."* SDR §3 says the same at the seam level. The design was made for it.

**Rejected: `tokio_util::sync::CancellationToken`.** A0 deleted `tokio-util` as one of five zero-use-site dependencies. Re-adding a dependency A0 removed, to carry one bool, reverses a change in the same series — the same argument D1 made when it declined `static_assertions` for four `const fn` lines.

**Rejected: a supertrait, `pub trait Reporter: Send + Sync + Cancellable`.** This is the shape that reads best and it is the one thing this change must not do. D1 established that `trait_added_supertrait` is a **major** semver event that had to precede `subx-core` 1.0.0's publication, and paid for it with four preconditions and a stop-the-change check. A provided method on the existing trait buys the same expressiveness for zero semver cost. Decision 7 states the verdict; the answer to "does anything here carry D1's hazard" is *no, and this is where it was avoided on purpose*.

**Where it is polled, and what a stopped loop returns.** Exactly one site: `execute_operations_audit`'s loop (`engine.rs:1783`), checked before each operation. On `true` it stops and pads the remaining slots with `OperationOutcome { applied: false, error: None }`, so `outcomes.len() == operations.len()` still holds — which callers rely on, `../subx` by zipping the two in `build_report` (`commands/match.rs:475`) — and the shape matches the method's own dry-run branch (`:1748-1756`), which already returns all-`applied: false`. It never returns `Err`, and it never leaves a half-written file: the check is between operations, not inside one.

**Three places cancellation is deliberately not added.**

- `execute_operations` returns `Result<()>`. It cannot express "stopped early" without changing its signature (breaking; the GUI does not call it but the path-stability requirement governs it) or minting a new `SubXError` variant (AGENTS.md: existing variants only, each mapped to an exit code). It emits progress and ignores `cancelled()`. The CLI's cancellation for that path is Ctrl-C, as today.
- `TranslationEngine::translate_subtitle` returns one `TranslationResult`; a partial has no representation. `../subx` cancels it by dropping the future and is satisfied.
- `SyncEngine` — `../subx` uses `AbortHandle` and D1 examined this code most recently; no loop, nothing to poll.

### Decision 7: the semver verdict, item by item — all additive, and the one shape that would not have been

| Addition | `cargo-semver-checks` class | Verdict |
|---|---|---|
| `ComponentFactory::match_config` | new inherent method | minor |
| `ComponentFactory::create_match_engine_with` | new inherent method | minor |
| `CollectedFiles::default_output_dir` | new inherent method | minor |
| `CollectedFiles::default_output_path` | new inherent method | minor |
| `apply_archive_origin_relocation` | new public free function | minor |
| `ProgressEvent::{Started, Advanced, Finished}` | `enum_variant_added` on an enum that is **already** `#[non_exhaustive]` | minor |
| `Reporter::cancelled` | `trait_method_added` **with a default body** | minor |
| `ui::create_progress_bar` template gains `{msg}` | no signature change; `subx-cli` only | not a semver event; is a CHANGELOG `### Changed` |
| `execute_operations_audit` stops early when `cancelled()` | behaviour reachable only by a caller that returns `true` from a method that did not exist | not observable by any existing consumer |

**Nothing here carries D1's hazard.** D1's `SubtitleFormat: Send + Sync` was `trait_added_supertrait`, which is major, which is why it needed the four preconditions and the falsifiable "has the tag been cut" check. Every addition above is a minor-version event, and the two that could have been major were designed around rather than accepted:

1. `ProgressEvent` is `#[non_exhaustive]` **because A1 made it so for this change** — A1 accepted a wildcard-arm cost on a one-variant enum precisely so these three variants would be free. That decision is now cashed in.
2. `Reporter::cancelled` is a provided method, not a supertrait (Decision 6).

**Consequences for sequencing.** Because it is all minor, this change does *not* have to precede publication. It could land against a published `subx-core` 1.0.0 as 1.1.0. Decision 1's window is required for a spec-authoring reason, not a semver one, and that is a meaningfully weaker constraint than D1's — worth stating because a reader who has just read D1 will assume otherwise.

**Two residual hazards, named rather than fixed.** `MatchConfig` stays exhaustively constructible, so its ninth field is still a major break (Decision 2). And a downstream type that implements `Reporter` *and* another trait with a `cancelled()` method would face an ambiguous unqualified call; no such type exists in either repository or in `../subx`, and the fix is one turbofish at the call site.

### Decision 8: `input-path-handling`'s *Output Directory Resolution for Archive Files* moves from **L** to a split — and C2b must be told

C2b classified this requirement **L**, with the evidence written out in three places: its Decision 2 table ("SDR §8 gaps 4 and 5 exist *because* this is not in core"), its Decision 5 ("its CLI half is small but real, so it splits" — of the capability, not this requirement), and its risk register ("moving the requirement to `subx-core` would specify behaviour no `subx-core` file implements"). Every word of that was true when written.

This change makes it false, and C2b anticipated exactly that: its **task 1.7** re-greps `archive_origin` under `src/commands/` and says "Zero hits would mean the requirement had migrated into core and must be reclassified."

The grep will not go to zero. After this change `src/commands/` still holds the `--output` precedence arm (`convert_command.rs:348-361`), `--replace`'s archive refusal (`translate_command.rs:330-336`), and sync's two conditional batch redirections (`sync_command.rs:511-519`, `:583-591`, deliberately not adopted per Decision 3). So the honest verdict is **not** C→ or L→; it is **L → S**, a 1→2 split in exactly the shape C2b's Decision 3 already has machinery for:

| Half | Title | Content |
|---|---|---|
| core | **Archive-Aware Output Location Resolution** (new title) | the resolution rule; `default_output_dir` and `default_output_path`; the documented divergence between them; the scenario that convert's output lands beside the archive |
| CLI | *Output Directory Resolution for Archive Files* (keeps the title) | `-o`/`--output` precedence and the multi-input directory-append rule; `--replace` refused for archive-extracted subtitles; sync's conditional batch redirection; the scenario that `-o` overrides archive origin |

C2b's rule for which half keeps the title — "the half that still answers the question the title asks" — gives the title to the CLI: the question is *which directory does the output go in for this command invocation*, and after this change the answer is still assembled by the command. So the `subx-cli` delta is a `## MODIFIED Requirements` entry and the core half is a new title, which is C2b's own case pattern.

**The hand-off, which task 6.4 writes into this change's completion notes.** C2b's `input-path-handling` table changes from 12 C / 2 L to **13 C / 2 L (1 split)**; its Decision 3 nine-way split table gains a tenth row; its arithmetic in Decision 3 ("82 removed plus 7 new core-half titles gives 89") becomes 83 + 8 = 91 core and 59 CLI; and its task 1.7 grep expectation changes from "must hit convert_command.rs, match_command.rs, sync_command.rs (twice) and translate_command.rs" to "must hit convert_command.rs, sync_command.rs (twice) and translate_command.rs, and must **not** hit match_command.rs" — because gap 5 removes match's `archive_origin` call entirely.

This is the same species of hand-off D1's Migration Plan step 6 wrote for C2a and C2b, and it exists for the same reason: a change that lands before the split is the only party that can tell the splitter its evidence moved.

### Decision 9: five delta specs — and the evidence for excluding `format-conversion` and `parallel-processing`

The set is `core-reporting`, `component-factory`, `input-path-handling`, `subtitle-matching`, `progress-reporting`. Four of the five touch only requirements C2b marks **C** or that C2a migrates wholesale; the fifth is CLI-only and stays in `subx-cli`. The brief's two remaining candidates are excluded, each for a different reason, and both reasons are the overlap C2b's Decisions 3 and 11 warn about.

**`core-reporting` — 1 MODIFIED + 2 ADDED.** *Transport-Agnostic Reporter Seam* is the requirement that enumerates `Reporter`'s methods ("exactly four methods") and `ProgressEvent`'s variants ("the single variant `Message(&'a str)`"); both statements become false, so it is restated in full with five methods and four variants. The two additions are subjects the seam requirement does not cover and should not: *Structured Progress Stream Semantics* (ordering, one open stream, monotonic `done`, `done ≤ total`, and what `done < total` at close means) is about emitters and consumers rather than about the trait's shape, and *Cooperative Cancellation Through the Reporter* is about where `cancelled()` is polled and what a stopped loop returns. Splitting them keeps the seam requirement readable as a type declaration and puts each behavioural contract where a grep for it will land.

**`component-factory` — 1 MODIFIED.** *Match Engine Creation* is the requirement that already enumerates the six config-derived fields and the two hardcoded ones, in the order the code writes them. It is the only place a caller learns what `match_config()` contains, and its first scenario already asserts two of those fields. C2b marks it **C**.

**`input-path-handling` — 1 MODIFIED.** Decision 8.

**`subtitle-matching` — 1 MODIFIED.** Decision 4. *AI-Driven Language and Globally-Unique Target Naming* is restated in full — eleven scenarios, ten of them carried over verbatim — because two sentences and one scenario name `match_command` as the party that performs the archive-origin rewrite.

**`progress-reporting` — 2 MODIFIED.** *Progress Bar Styling* pins the template string exactly, and the template gains a trailing `{msg}` so the `Active: … | Queued: … | Completed: …` line that *Batch Progress Updates* requires stays visible once the parallel path stops using its own `{msg}`-bearing template. *Progress Bar Visibility Follows Configuration* says "The match command SHALL construct a progress bar … Implemented in `src/commands/match_command.rs`", and after this change the CLI reporter constructs it; the flag obligation is unchanged but its address is not. Its third requirement, *Progress Bars Force-Hidden in JSON Output Mode*, is **not** modified: it already says "ad-hoc `ProgressBar::new(...)` calls that bypass this helper SHALL be refactored to go through it", so deleting `match_command.rs:807-813` is compliance with a requirement that has been unmet since it was written, not a change to it. Recording it as a `### Fixed` CHANGELOG line and not as a spec delta is the honest shape. *Batch Progress Updates* is also not modified — it constrains what the message says and when, both of which survive intact.

**`format-conversion` — excluded.** *Input and Output Path Resolution* is C2b-**L** and cites "the output-path computation in `src/commands/convert_command.rs`", which is exactly what Decision 3 moves. A delta here is the obvious move and it is declined: the resolution rule would then be stated in two capabilities, which is the "two requirements in two capabilities asserting one property" failure C2b Decision 3 spends a page avoiding and C2b Decision 11 names as the drift a split pair cannot detect. `input-path-handling` is the right single home — it owns `CollectedFiles`, it owns `archive_origin`, and C2b's own evidence table filed all five call sites there. What remains in `format-conversion`'s requirement after this change — the `-i` and `--output` flags — is still exactly true, so there is no gap either.

**`parallel-processing` — excluded, and this one is worth the sentence.** Two of its requirements look like they must change. *Progress Reporting Opt-Out* reads in full: "The system SHALL respect the `general.enable_progress_bar` configuration; when the flag is false, the progress indicator SHALL be hidden", with a scenario phrased over "a parallel batch executes". It carries **no file citation and no construction site**, so moving the bar into the CLI reporter leaves every word true. *Aggregated Result Reporting* constrains the summary counts `monitor_batch_execution` returns, which this change does not touch. Restating either would produce a delta that validates and says nothing — D1 Decision 10 declined a `component-factory` delta on exactly that test. There is also a positive reason to leave it alone: `progress-reporting`'s Purpose already states the division ("Configuration coupling with `general.enable_progress_bar` is covered here from the UI-behaviour perspective; the parallel-processing spec owns the scheduler side of the same flag"), so the UI-side change belongs to `progress-reporting` and only to it.

### Decision 10: what `../subx` can delete, file by file

The payoff, concretely, and with the honest zero in it. None of this is done by this change — `../subx` is read-only reference here, and the deletion is a follow-up PR in that repository, exactly as D1's Decision 11 and SDR §7's `subx_cli::` → `subx_core::` migration are. Line ranges are against that repository as it stands.

**Gap 3 — `MatchConfig` hand-building:**

| Location | Lines | What it is |
|---|---|---|
| `src-tauri/src/commands/match.rs:259-273` | 15 | the four-line comment naming `create_match_engine()` unusable, plus the eight-field `MatchConfig` literal. Becomes `let mut config = factory_config.match_config(); config.confidence_threshold = …; config.relocation_mode = mode;` — three lines, and immune to a ninth field |

**Gap 4 — output-path resolution:**

| Location | Lines | What it is |
|---|---|---|
| `src-tauri/src/commands/convert.rs:488-506` | 19 | `resolve_output_path` in full, including the six-line doc comment whose subject is that it mirrors `convert_command.rs`. Becomes `collected.default_output_path(input, format_id)` |
| `src-tauri/src/commands/translate.rs:725-730` | 6 | the `base_dir` chain inside `resolve_output_path`. Becomes `collected.default_output_dir(input)`; the replace-mode branch and the naming call stay |
| `src-tauri/src/commands/translate.rs:12-18` | 4 | the sentences of the module doc whose subject is that the CLI's output resolution is private and therefore mirrored. The `backup_path` half of that paragraph stays until the Open Questions item is taken |

**Gap 5 — archive-origin rewrite:**

| Location | Lines | What it is |
|---|---|---|
| `src-tauri/src/commands/match.rs:393-408` | 16 | `rewrite_archive_origins` in full, including the doc comment that says it mirrors `match_command.rs` and the "(design D5)" ordering note. Becomes `apply_archive_origin_relocation(&mut operations, &collected)` immediately above the `apply_unique_target_paths` call it already sits above at `:428-430` |

**~60 lines**, of which ~29 are prose whose only subject is that the code below it is a copy.

**Gap 6 — zero lines, and a capability instead.** No deletion. `ProgressReporter`, `ConversionReporter` and `TranslateReporter` are Tauri `Channel` adapters and test seams; `AtomicBool` and `watch` are the right mechanisms at the granularities they serve (Decision 6). What becomes possible is what `../subx` cannot build today: `execute_selected_impl` (`commands/match.rs:301-346`) awaits the whole batch with no progress and no cancel, alone among that repository's four wizards. With `Started`/`Advanced`/`Finished` and `cancelled()` it can gain a `MatchProgress` execution stage and a `cancel_execution` command whose `AtomicBool` is read by core between operations — the same shape its convert wizard already has. That is the gap-6 payoff, and stating it as a capability rather than a line count is the correction this change makes to SDR §8's framing.

**What is verified here rather than assumed.** Task phase 8 builds `../subx` against the patched `subx-core` in a scratch worktree with the four deletions applied, and confirms it compiles and its existing tests pass — in particular that `default_output_path` and `default_output_dir` reproduce the two mirrored functions bit-for-bit, which that repository's own tests at `translate.rs:1240`, `:1286`, `:1308` already assert over concrete paths. The worktree is discarded; nothing is committed to `../subx`.

### Decision 11: sizing — this one does **not** fit a workday, the seam is named, and it is deliberately not taken

Six siblings measured themselves. B3 found ~14.5 h and lifted three phases into B4; C3 found 10.75 h and lifted three files into C4; C1, C2a and C2b each named their own division; D1 measured 7.5 h and needed no seam. This change measures **~12.75 h** — about 1.7 workdays.

| Work | Estimate |
|---|---|
| Preconditions and baseline: confirm B2, A1, A2, C1 and D1 landed and C2a has not; re-verify the five `archive_origin` sites and the two `MatchConfig` literals against the post-B2 tree | 0.5 h |
| Gap 3: `match_config`, `create_match_engine_with`, `create_match_engine` delegation, rustdoc, three unit tests, CLI adoption at `match_command.rs:423-436` | 1.5 h |
| Gap 4: both `CollectedFiles` queries, rustdoc including the divergence note, six unit tests, CLI adoption at `convert_command.rs:362-370` and `translate_command.rs:383-392` | 2.0 h |
| Gap 5: `apply_archive_origin_relocation`, rustdoc, four unit tests, CLI adoption at `match_command.rs:456-466` | 1.25 h |
| Gap 6a: three `ProgressEvent` variants, `Reporter::cancelled`, derive and default-body tests | 0.75 h |
| Gap 6b: emission from both execution loops, the cancellation check and the outcome padding, five tests | 1.75 h |
| Gap 6c: `TerminalReporter` bar ownership, `ui::create_progress_bar`'s `{msg}`, deleting `match_command.rs:807-813` and `:729-737`, rewiring `monitor_batch_execution`, two characterisation tests | 2.0 h |
| Five delta specs — one with 1 MODIFIED + 2 ADDED, one with an eleven-scenario full restatement — and `openspec validate --strict` | 1.5 h |
| Documentation, two CHANGELOGs, the C2b hand-off note | 0.75 h |
| Quality gate | 0.75 h |

**The seam, named:** phases 5, 6 and 7 of `tasks.md` (gap 6), plus the `core-reporting` and `progress-reporting` delta files and their two documentation lines. That is ~5 h and lifts verbatim into a fifteenth change, `stream-core-progress-and-cancellation`, leaving gaps 3–5 at ~7.75 h — a clean workday.

**And it is deliberately not taken.** Three reasons, in order of weight:

1. **The halves collide on the same files.** B3's split into B4 worked because B4's three phases "touch no file the move touches". Here they do: gap 3 and gap 5 both edit `subx-core/src/core/matcher/engine.rs` and `subx-cli/src/commands/match_command.rs`, and gap 6b/6c edit both again. Pre-splitting would manufacture exactly the 🔴 overlap the plan's §3 conflict matrix exists to prevent, and would force one half to rebase on the other for no benefit.
2. **Nothing waits.** B3 and C3 split because they were on the critical path and successors were blocked behind them. This change is the last in the series and has zero dependants (SDR D9, plan §2). An overrun costs a second sitting and nothing else — which is a materially different trade from the one the earlier five faced.
3. **The three shared artifacts would be duplicated.** One precondition set (Decision 1's window, which is the tightest constraint in the change), one hand-off note to C2b (Decision 8, which gap 4 alone triggers), and one `[Unreleased]` entry per repository. Splitting writes all three twice and creates a second chance to get the C2b hand-off wrong.

So: the measurement is stated, the seam is named at a real phase boundary, and the condition for taking it is explicit — **if phase 4 is not complete by the end of the day, phases 5–7 and their two delta files lift into `stream-core-progress-and-cancellation` rather than being rushed.** Task 8.5 makes that a decision point rather than a drift.

## Risks / Trade-offs

- **Risk: gap 6c changes CLI-visible output — text-mode `subx match` gains a progress bar during execution, which no user sees today.** → Mitigation: this is the one intentional UX change and it is confined to a channel `progress-reporting` already governs. It is suppressed in JSON mode (that is the defect being fixed), suppressed by `general.enable_progress_bar = false` (the *Progress Bar Visibility* restatement), and suppressed by `--quiet` because A1 Decision 4a routes the whole `progress` channel through the quiet gate. The two new characterisation tests assert the first two. If review rejects the bar, `TerminalReporter` can render `Started`/`Advanced`/`Finished` as nothing at all and the rest of the change is unaffected — core's emission is silent under `NoopReporter` either way.
- **Risk: `default_output_path` is "simplified" into `default_output_dir(input).join(..)` by a later reader, changing `a.srt -> a.vtt` into `a.srt -> ./a.vtt`.** → Mitigation: Decision 3 states the divergence, both rustdocs state it, and the `input-path-handling` core half carries a scenario that asserts the bare-filename case explicitly. It is the only trap in the change and it is guarded in three places.
- **Risk: `execute_operations_audit`'s padding on cancellation is read as success by an existing caller.** → Mitigation: the padded outcome is `{ applied: false, error: None }`, which is exactly what the dry-run branch has always returned, so no caller can have been treating that shape as "applied". The distinguishing signal is `Finished { done, total }` with `done < total`, which only a caller that implemented `cancelled()` can receive — and a caller that never returns `true` never sees a short batch. No existing consumer's behaviour can change.
- **Risk: the one-open-stream rule is violated by a future emitter and two bars fight over one terminal.** → Mitigation: it is stated normatively in *Structured Progress Stream Semantics* rather than left as an assumption, and Decision 5 records the `stream: u64` token as the additive fix. The CLI reporter's own behaviour on a second `Started` is specified (replace, do not nest), so the failure is a wrong bar rather than a panic or a leak.
- **Risk: this change lands after C2a and the four core-side deltas have nowhere to go.** → Mitigation: task 1.1 checks `openspec/specs/core-reporting/spec.md` still exists in this repository and stops the change if it does not, with Decision 1's fallback named. Unlike D1's equivalent precondition this one is fully recoverable — the deltas move to a `subx-core/openspec/changes/` change and no code decision changes — because Decision 7 establishes that nothing here is a semver break.
- **Risk: a reader takes SDR §8 gap 6 at face value, expects the GUI's three reporter traits to be deleted, and reads Decision 10's zero as an incomplete job.** → Mitigation: the Context section and Decision 10 both state the correction with the GUI's own comments as evidence. This is the last change in the series, so the correction has to live here or nowhere.
- **Risk: `Reporter::cancelled` is polled somewhere hot and costs a virtual call per item.** → Mitigation: one poll per *file operation* — a rename, a copy or a backup — is unmeasurable against the syscall it precedes. The requirement names the single call site so a later change that adds a poll inside a tight loop has to argue for it.
- **Risk: scope creep into the seventh duplication (`translated_file_name`, `backup_path`) or into `SyncEngine`.** → Mitigation: both are in Non-Goals with a reason, and the first is in Open Questions with the capability it would open named.

## Migration Plan

1. **Preconditions.** B2, A1, A2, C1 and D1 landed; C2a has **not** archived (`openspec/specs/core-reporting/spec.md` still exists here); both roots green. Decision 1 explains why the window matters and Decision 7 explains why missing it is recoverable.
2. **Gaps 3, 4, 5 first, in that order** — three independent function extractions, each with its CLI adoption in the same phase so the two implementations never both exist. After phase 4 the change is coherent and shippable on its own, which is what makes Decision 11's seam real rather than rhetorical.
3. **Decision point.** If the day is gone, stop here and lift phases 5–7 into `stream-core-progress-and-cancellation` (task 8.5).
4. **Gap 6, core side** — variants, `cancelled()`, emission from both loops, cancellation and padding in the audit loop. Silent under `NoopReporter`, so the CLI is unaffected until step 5.
5. **Gap 6, CLI side** — `TerminalReporter` takes the bar, `ui::create_progress_bar` gains `{msg}`, the duplicate constructor and the local draw-target block are deleted, `monitor_batch_execution` reports through the reporter. A1's characterisation tests must still pass unchanged; that is the gate for this phase.
6. **Downstream verification.** Scratch worktree of `../subx`, Decision 10's four deletions applied, `cargo check` and its test suite. Discard.
7. **Specs and docs.** Five delta files, `openspec validate expose-core-orchestration-apis --strict`, two CHANGELOGs, the C2b hand-off note from Decision 8.
8. **Rollback.** Every step is additive on the core side, so steps 4 and 2 can be left in place if only the CLI adoption needs reverting. Step 5 is the only one with CLI-observable output, and reverting it is symmetric: restore the local `create_progress_bar` and the draw-target block, and `TerminalReporter` falls back to A1's behaviour.
9. **The follow-up PR in `../subx`**, which is not part of this change: the four deletions of Decision 10, then a `cancel_execution` command and an execution stage on `MatchProgress`.

## Open Questions

- **Should `translated_file_name` and `backup_path` move to core too?** Decision 3 found them mirrored at `../subx/src-tauri/src/commands/translate.rs:704-708` and `:735-742` — a seventh duplication SDR §8 does not record. They are declined here because they are naming rather than location functions and belong to `subtitle-translation`'s *Safe Output Behavior*, which C2b classifies **L**; taking them would open a sixth capability and a second reclassification hand-off for ~14 lines. Recorded so the finding is not lost with this change.
- **Should `MatchConfig` become `#[non_exhaustive]`?** Yes on the merits and not here: it is a major break against three current struct-literal sites (Decision 2). It becomes free once all three adopt `match_config()`, which this change makes possible for two of them. The natural taker is whichever change next bumps `subx-core`'s major version.
- **Does `execute_operations` deserve a cancellable sibling?** Its `Result<()>` cannot express a short batch (Decision 6). An `execute_operations_audit`-shaped return would be the answer, and it already exists — so the real question is whether `execute_operations` should be retired in favour of it. That is a removal, which needs a major version and a survey of `subx-cli`'s own call sites; out of scope, raised because the asymmetry will read as an oversight.
- **Should the CLI's `monitor_batch_execution` move into `core::parallel` so the scheduler emits its own progress?** It would put `Started`/`Advanced` next to the counts they describe and let `../subx` drive a parallel batch with progress. It is out of scope because C2b classifies *Aggregated Result Reporting* and the CLI half of *Task Scheduler Entry Point* as **L** on the strength of those exact lines, and moving them would reopen a split C2b has already reasoned through.
