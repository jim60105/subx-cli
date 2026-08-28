## Context

After B2 the library lives in `subx-core/src/**` and the Tauri GUI (`../subx`) depends on `subx-core` alone. SDR §8 verified the GUI's consumed surface across all sixteen `.rs` files of `../subx/src-tauri/src/` and recorded six gaps that force it to duplicate logic. This change closes the first two. Neither blocks anything: SDR D9 makes D1 and D2 explicitly non-critical-path, and the plan's conflict matrix shows D1 overlapping only with B2 (which precedes it) and D2 (which follows it).

The two gaps are the same defect wearing different clothes. In both cases a correct, tested, already-public capability is unreachable from an async multi-threaded host because of a property that has nothing to do with what the capability does.

**Gap 1 is one line of code.** `FormatManager`'s only field is `formats: Vec<Box<dyn SubtitleFormat>>` (`manager.rs:23`). `dyn Trait` carries no auto traits unless they are named, so `Box<dyn SubtitleFormat>` is neither `Send` nor `Sync` no matter what the concrete types are — and the concrete types here are four unit structs. The property propagates by value into `FormatConverter { format_manager: FormatManager, config: ConversionConfig }` (`converter.rs:28-31`) and `TranslationEngine { ai_provider: Arc<dyn AIProvider>, format_manager: FormatManager, batch_size: usize }` (`translation/engine.rs:34-38`). Those are the three types SDR §8 gap 1 names, and a `FormatManager`-holder search over `src/` returns exactly those two holders and no others, so the list is complete rather than illustrative.

Everything else in the neighbourhood is already thread-safe and stays that way for free: `AIProvider: Send + Sync` (`services/ai/mod.rs:153`), `ConfigService: Send + Sync` (`config/service.rs:61`), `MatchEngine { Box<dyn AIProvider>, FileDiscovery, MatchConfig }` (`matcher/engine.rs:1264-1268`), `SyncEngine { SyncConfig, Option<VadSyncDetector> }` (`sync/engine.rs:21-24`, whose detector bottoms out in `VadAudioProcessor {}` — a field-less struct), `ComponentFactory { config: Config }` (`factory.rs:42-44`) and `FileManager { operations: Vec<FileOperation> }` (`file_manager.rs:99-101`). A1's `Arc<dyn Reporter>` field, added to several of these, is `Send + Sync` because `Reporter: Send + Sync` — A1's Decision 9 chose that bound deliberately so as not to add "a fourteenth reason those types cannot cross a thread boundary".

**Gap 2 is a constructor precondition that outlives its own scope.** `SyncEngine::new` (`sync/engine.rs:36-68`) builds a `VadSyncDetector` when `config.vad.enabled`, then — outside that branch — refuses to construct at all if it has no detector. `apply_manual_offset` (`:148-204`) reads `self.config.max_offset_seconds` and mutates `subtitle.entries`; it does not look at `self.vad_detector`. So a caller who wants only a manual offset must satisfy a precondition for a subsystem it will never touch. The GUI, whose whole reason for offering manual offset is that automatic detection was unavailable, cannot do that, and mirrors the method instead.

The consequence is measurable in the consuming repository, not in this one: five comments and one module-doc paragraph across three command modules exist solely to explain these two constraints, and one of them (`sync.rs:280-285`) is a written promise that a copied algorithm will not drift.

## Goals / Non-Goals

**Goals:**

- Make `FormatManager`, `FormatConverter` and `TranslationEngine` `Send + Sync`, so the GUI can hold them across an `.await` and put them in `tauri::State`.
- Make the guarantee non-regressible by asserting it at compile time over a named set of types, rather than recording it in prose.
- Give VAD-independent callers a first-class entry point to the manual-offset transform, with exactly one implementation of the transform shared with `SyncEngine::apply_manual_offset`.
- Preserve every public path, every constructor signature and every observable CLI behaviour. `MatchEngine::new`, `SyncEngine::new`, `TranslationEngine::new` and `ComponentFactory::new` are A1's locked constraint and none of them is touched.
- Be honest about the semver consequence and sequence the change so the consequence is zero rather than glossing over it.
- State concretely what `../subx` can delete, because that is the change's entire payoff.

**Non-Goals:**

- Gaps 3–6. `ComponentFactory::create_match_engine`'s hardcoded `relocation_mode: None`, convert's output-path resolution, match's archive-origin rewrite and the progress/cancellation hook are **D2**'s (`expose-core-orchestration-apis`). This change adds no progress plumbing, touches no `Reporter` variant and does not open `factory.rs` except to read it.
- Changing `SyncEngine::new`'s behaviour. Making the VAD initialisation lazy, or relaxing the `vad.enabled == false` refusal, is a separate concern with its own consumers — Decision 9 names it and declines it.
- Editing `../subx`. That repository is read-only reference here. This change states what becomes deletable; the deletion is a PR in that repository, exactly as SDR §7's `subx_cli::` → `subx_core::` migration is.
- Making any *other* type `Send`/`Sync`. The assertion set is the seven engine types plus `Box<dyn SubtitleFormat>`; it is not a sweep of the crate.
- Merging the two `SyncMethod` enums, deleting the dead `SyncEngine::auto_detect_sync_offset` (`sync/engine.rs:113-131`, private and called from nowhere), or any other cleanup that happens to be in the same file. A2 declined the first for the same reason.
- Touching `Cargo.toml` in either repository, or any workflow, script or feature flag.

## Decisions

### Decision 1: The root cause is `Box<dyn SubtitleFormat>`, and the fix is a supertrait on the trait

`FormatManager` has exactly one field. Its type is `Vec<Box<dyn SubtitleFormat>>`. A trait object's auto traits must be spelled out, so this vector is `!Send + !Sync` even though every value ever put in it is a unit struct. That is the whole of gap 1: remove it and all three types named by SDR §8 become `Send + Sync` with no other edit, because every other field in the three structs is either plain data or a trait object whose trait already carries the bounds.

**Chosen: `pub trait SubtitleFormat: Send + Sync`** at `subx-core/src/core/formats/types.rs:835`.

The whole diff on the mechanism is two words. Nothing else in the crate changes shape: `Box<dyn SubtitleFormat>` stays written as it is, `get_format` and `get_format_by_extension` keep returning `Option<&dyn SubtitleFormat>` (`manager.rs:59-72`), every `impl SubtitleFormat for …` block is untouched, and no call site anywhere needs an annotation.

**Rejected: bound the storage instead of the trait —** `formats: Vec<Box<dyn SubtitleFormat + Send + Sync>>`. It is the more conservative-looking option, and it costs more than the chosen one for a reason worth recording, because it is not obvious and it was checked rather than assumed. The two accessors are written as `self.formats.iter().find(…).map(|f| f.as_ref())` against a declared return type of `Option<&dyn SubtitleFormat>`. Auto-trait removal *is* a legal unsizing coercion, but a coercion site does not propagate through a closure's inferred return type: the closure infers `&(dyn SubtitleFormat + Send + Sync)`, `Option::map` returns `Option<&(dyn SubtitleFormat + Send + Sync)>`, and the function's return type does not match. Verified with a reduced case under `rustc --edition 2024`:

```
error[E0308]: mismatched types
   |     fn get(&self) -> Option<&dyn T> { self.v.first().map(|b| b.as_ref()) }
   |                      --------------   ^^^ expected trait `T`, found trait `T + Send + Sync`
```

The repair is an explicit `as &dyn SubtitleFormat` in each of the two accessors, which compiles — but it means the field-only variant costs two extra casts, leaves `Box<dyn SubtitleFormat>` still meaning "not thread-safe" everywhere *else* it might be written in future, and puts the guarantee in one struct's private field rather than in the trait's contract. It buys nothing in return: the set of implementors is identical either way (Decision 2), so it is not more permissive in practice.

**Rejected: `Arc<dyn SubtitleFormat + Send + Sync>` in place of `Box`.** The brief lists it as a candidate and it addresses a problem this code does not have. The handlers are constructed once in `FormatManager::new` (`manager.rs:34-41`), never shared, never cloned, and never outlive the manager; `FormatConverter::clone` (`converter.rs:32-36`) does not clone the manager at all — it calls `FormatConverter::new(self.config.clone())`, rebuilding the four unit structs, which is free. `Arc` would add a refcount to four zero-sized values and change a field type for no reachability gain.

**Rejected: erase the trait object entirely** — replace `Vec<Box<dyn SubtitleFormat>>` with an enum over the four formats, or with a fixed-size array of concrete types. It would make the thread-safety question disappear along with the dynamic dispatch, and it would be a redesign of `FormatManager`'s registration model, break `get_format`'s and `get_format_by_extension`'s published `&dyn SubtitleFormat` return types (both on the trait-object side of B2's *Public API Path Stability* contract), and land a `format-conversion` API change inside a change whose stated goal is not to. Rejected on scope, not on merit.

### Decision 2: Every `SubtitleFormat` implementor satisfies the new bound, and the implementor set is closed

The bound is only free if every implementor already satisfies it. The complete set, enumerated by `grep -rn "impl SubtitleFormat for" src/`:

| Implementor | Definition | Fields | `Send + Sync`? |
|---|---|---|---|
| `AssFormat` | `src/core/formats/ass/mod.rs:99` | `pub struct AssFormat;` | Yes — unit struct |
| `VttFormat` | `src/core/formats/vtt/mod.rs:35` | `pub struct VttFormat;` | Yes — unit struct |
| `SrtFormat` | `src/core/formats/srt/mod.rs:25` | `pub struct SrtFormat;` | Yes — unit struct |
| `SubFormat` | `src/core/formats/sub/mod.rs:35` | `pub struct SubFormat;` | Yes — unit struct |

Four implementors, all field-less, all trivially `Send + Sync` by the auto-trait rules. There is no `impl SubtitleFormat for` anywhere under `tests/`, and none in `../subx/src-tauri/src/` — the only trait the GUI implements is `AIProvider`, in four test doubles (`commands/match.rs:592`, `:636`, `commands/translate.rs:923`, `:1554`), and `AIProvider` already requires `Send + Sync`, so those four are untouched.

**This is the crux of the change and it turned out to be empty.** The brief anticipated that an implementor failing the bound would be where the difficulty lay; there is none. What remains is not a code problem but a versioning problem (Decision 5) and a governance problem (Decision 6), and the change's risk profile is entirely in those two.

The set is closed in a way worth stating, because it is what makes the bound safe to add rather than merely possible: `SubtitleFormat` is the extension point of a *closed* registry. `FormatManager::new` hardcodes the four (`manager.rs:34-41`), there is no registration API, and `format-conversion`'s *Supported Output Formats* fixes the accepted `--format` set at exactly those four. A fifth format is a change to `FormatManager::new`, and any type an author would write there is a parser — stateless, or holding configuration. A parser that is not `Send + Sync` would hold a `Rc`, a `RefCell` or a raw pointer, which in a subtitle parser is a defect independent of this bound.

### Decision 3: `Send` **and** `Sync`, because they are one clause and one is not cheaper than the other

The brief asks whether `Sync` is over-delivery when `Send` alone unblocks the GUI. It is not, and the reason is mechanical rather than a judgement about generosity.

The only obstruction is a trait object's auto-trait list. `pub trait SubtitleFormat: Send` is exactly the same edit as `pub trait SubtitleFormat: Send + Sync` — the same file, the same line, the same four implementors, the same zero call-site churn. There is no smaller version of the work. Choosing `Send` alone would mean deliberately writing a narrower bound to deliver less, and then owning the consequence: `&FormatManager` could not be shared across threads, `Arc<FormatConverter>` would not be `Send`, and `tauri::State<FormatConverter>` — which needs `Send + Sync + 'static` — would still be impossible. That is not "not over-delivering"; that is delivering a fix that does not close the gap.

The GUI needs both. `convert.rs`'s and `translate.rs`'s `spawn_blocking` workaround needs `Send` (the future must cross to a worker). Holding a converter in `AppState` — the shape that lets the engine be built once instead of per file — needs `Sync` as well. SDR §8 gap 1 names both: "neither `Send` nor `Sync`".

Where over-delivery *would* start, and where this change stops: no `Sync`-by-interior-mutability is introduced anywhere, no `Mutex` or `RwLock` is added, no method takes `&mut self` where it took `&self` or vice versa, and no type gains `Clone`. The types become shareable because their fields already were; nothing is made shareable by adding synchronisation.

### Decision 4: The guarantee is asserted at compile time over a named set, not documented

A bound that is satisfied by accident is a bound that regresses silently. Adding one non-`Send` field to `TranslationEngine` — an `Rc`, a `RefCell`, a `dyn` field whose trait forgot the bounds — would restore gap 1 with no test failure and no clippy warning, and the GUI would discover it as a compile error in a different repository.

**Chosen: a `thread_safety` module in `subx-core/src/core/mod.rs`** containing

```rust
#[cfg(test)]
mod thread_safety {
    const fn assert_send_sync<T: Send + Sync + 'static>() {}
    // one line per contracted type
    const _: () = assert_send_sync::<crate::core::formats::manager::FormatManager>();
    // …
}
```

over eight entries: `FormatManager`, `FormatConverter`, `TranslationEngine`, `MatchEngine`, `SyncEngine`, `ComponentFactory`, `FileManager` and `Box<dyn SubtitleFormat>`. It has no runtime body, so it costs nothing at test time and fails at *compile* time, which is the failure mode that cannot be skipped by a filter expression.

**Why four types that are already `Send + Sync` are in the set.** `MatchEngine`, `SyncEngine`, `ComponentFactory` and `FileManager` need no change today (Context). Including them makes the assertion module the *contract* rather than a record of this change's edits, and it costs four lines. A future change that adds a non-`Send` field to `MatchEngine` — plausible: a cache, a handle, a callback — fails here instead of in `../subx`. Excluding them would mean the file answers "what did D1 fix" rather than "what does core guarantee", and the second is the question a reader has.

**Why `#[cfg(test)]` and not an unconditional `const _`.** An unconditional item is compiled for every consumer of the crate and appears in the module tree; `#[cfg(test)]` keeps it out of the published artifact while still failing `cargo check --all-tests` and `scripts/quality_check.sh`. Either would work; this one keeps the crate's compiled surface exactly as it was.

**Rejected: a runtime `#[test] fn` that calls `assert_send_sync::<T>()`.** It compiles the same check but reports as a passing test, which invites someone to skip it with `--filter-expr`. A `const _` cannot be skipped.

**Rejected: `static_assertions::assert_impl_all!`.** It reads better and adds a dependency to `subx-core` for four lines of `const fn`. SDR §4 fixes core's dependency list and A0 spent a whole change removing five dependencies with no use sites; adding one back for sugar is the wrong direction.

**Rejected: prose in `crate-topology`.** A repo-scoped spec requirement cannot fail a build. The assertion and the `async-runtime-safety` requirement together are the right pair: the spec says what is guaranteed, the assertion makes the compiler enforce it.

### Decision 5: The semver verdict — `trait_added_supertrait` is major, and the honest answer is to land before publication

Adding a supertrait to a public trait is a breaking change. `cargo-semver-checks` classifies it as `trait_added_supertrait`, **major**. It is breaking regardless of whether any implementor actually fails the new bound, because a downstream crate's `impl SubtitleFormat for TheirType` now carries an obligation it did not carry before, and a downstream generic bounded by `T: SubtitleFormat` now implies `Send + Sync` in a way that changes inference.

Two facts about the *actual* audience, verified rather than assumed:

1. **`SubtitleFormat` is not on SDR §8's consumed list.** The GUI imports `core::formats::{manager::FormatManager, Subtitle, converter::{FormatConverter, ConversionConfig, ConversionResult}}` and nothing else from that module. It implements the trait nowhere.
2. **There is no other known consumer.** `subx-cli` at 1.9.1 is on crates.io as a binary crate whose library surface was never advertised as an extension point, and `subx-core` has not been published at all.

So the break is formal, not real. But "formal" still means a version number, and the series has been explicit about not papering over version consequences (C1 Decision on the 2.0.0 bump exists for exactly this reason).

**The verdict, stated plainly:**

- **Landed before `subx-core` 1.0.0 is published to crates.io** — the supertraits are part of what 1.0.0 *is*. There is no break, no version consequence, and nothing for C1's publish flow to accommodate. The `[Unreleased]` CHANGELOG entry is folded into `## [1.0.0]` by the release, exactly as C1 task 8.5 folds `subx-cli`'s A0–B4 entries into `## [2.0.0]`.
- **Landed after** — `subx-core` goes to **2.0.0**, `subx-cli`'s dependency line becomes `subx-core = { path = "subx-core", version = "2.0" }`, C1's submodule-pointer CI job (which asserts that the pinned core commit's version matches the `version` in `subx-cli`'s dependency line) needs both sides bumped in the same commit, and `subx-cli` — whose D11 re-export republishes the trait — takes a major bump of its own to 3.0.0 for a two-word change. That is a genuinely bad outcome for a change that is off the critical path and therefore free to move.

The same argument covers `subx-cli`: its 2.0.0 is declared by C1 but published by a `v2.0.0` tag, so landing before that tag keeps the re-exported trait's bounds inside 2.0.0.

**This is why Decision 6 exists.** The cost of getting the sequencing wrong is an order of magnitude larger than the cost of the change itself, which makes sequencing the substance here.

### Decision 6: D1's real prerequisite set is four items, not one — the slot is after C1 and before C2a

The plan's dependency table (`tmp/split-implementation-plan.zh-TW.md` §2) records one hard prerequisite for D1: **B2 → D1**, "these change `subx-core/src/**`". That is correct and incomplete. Three more prerequisites are derivable from the sibling changes, and they were found by trying to write this change's task list against the plan's batch-8 slot and discovering that two phases could not be written.

| Prerequisite | Why | What breaks without it |
|---|---|---|
| **B2** | the files are at `subx-core/src/core/**` | there is nothing at the paths this change edits |
| **C1** | C1 creates `subx-core/CHANGELOG.md` (its task 8.7) and `subx-core/scripts/quality_check.sh`, sets `default-members = [".", "subx-core"]`, and adds `--workspace` to the four `quality_check.sh` invocations | the documentation phase has no CHANGELOG to write into, and — worse — the quality gate runs `cargo clippy` and `cargo nextest run` **without** `--workspace`, so it does not compile `subx-core` at all. A change whose entire code diff is in core would pass a green gate that never looked at it |
| **before C1's release tag** | Decision 5 | `subx-core` 2.0.0 and `subx-cli` 3.0.0 for a two-word diff |
| **before C2a** | C2a moves `async-runtime-safety` wholesale into `subx-core/openspec/specs/`; C2b splits `format-conversion` and `timeline-sync` and sends their core halves to `subx-core` | all three capabilities this change deltas would be in the *other* repository's spec tree, and the delta files below would have to be authored as a change inside `subx-core/openspec/changes/` instead |

**The slot: after C1, before C2a.** The plan permits it explicitly — §2's "刻意不設依賴" section records C1 and C2a/C2b as mutually independent ("一邊是 `.github/` + `scripts/`，一邊是 `openspec/`"), and §1 records D1 as insertable at will. Concretely, the sequence becomes `… → B4 → C1 → D1 → C2a → C2b → C3`, with D1 taking the batch-7 slot that C2b would otherwise share with C1 and C2a/C2b shifting one batch later. The plan's total is unchanged; only the order within the C block moves.

**Why not do the obvious thing and author the deltas against `subx-core/openspec/`.** Because the series' convention is that all fourteen changes live in `subx-cli/openspec/changes/` (SDR §12), and because it would make this change depend on C2a/C2b — inverting a dependency the plan deliberately does not have, and putting a non-critical-path change downstream of two critical-path ones. Keeping the deltas here and moving the slot earlier costs nothing.

**What C2a and C2b must then carry, stated so it cannot be missed.** This change adds one requirement to `async-runtime-safety` and one to `timeline-sync`, and modifies one in `format-conversion` and three in `timeline-sync`. All six are **core**-classified under C2b's own rules, and the `## Migration Plan` below names them explicitly for both changes' task lists. If this change lands after either of them instead, the same six operations are authored as a change inside `subx-core/openspec/changes/` with the delta text carried over verbatim and citations un-prefixed — `spec-governance`'s *Citation Paths Are Resolved Against the Owning Repository* makes that a mechanical transform, not a rewrite.

### Decision 7: `SyncEngine::new`'s real failure trigger is the config flag, not an initialisation failure — and the SDR and the GUI both say otherwise

SDR §8 gap 2 says `SyncEngine::new` "hard-fails when VAD cannot initialize". The GUI's comment at `sync.rs:191-195` says it "fails whenever the VAD detector cannot be built, which is the *only* thing it can fail on". `timeline-sync`'s *Sync Method Selection* scenario says "VAD is disabled in configuration **or** the VAD detector fails to initialize". Read against the code, the second disjunct is currently unreachable:

- `SyncEngine::new` (`:36-61`): if `config.vad.enabled` is false, `vad_detector` is `None` and the unconditional `if vad_detector.is_none()` at `:56` returns the config error. If it is true, `VadSyncDetector::new` is attempted and a failure is logged and mapped to `None`, reaching the same error.
- `VadSyncDetector::new` (`services/vad/sync_detector.rs:32-36`) is `Ok(Self { vad_detector: LocalVadDetector::new(config)? })`.
- `LocalVadDetector::new` (`services/vad/detector.rs:32-38`) is `Ok(Self { config, audio_processor: VadAudioProcessor::new()? })`.
- `VadAudioProcessor::new` (`services/vad/audio_processor.rs:37-39`) is `Ok(Self {})` over `pub struct VadAudioProcessor {}` (`:11`).

Every link is infallible. **The single reachable trigger today is `config.vad.enabled == false`.** No model is loaded, no device is opened, no file is read; the `voice_activity_detector::VoiceActivityDetector` is constructed per call inside `detect_speech_from_data`, not stored.

This matters for three reasons, which is why it is a decision and not a footnote:

1. It makes gap 2 *worse* than SDR §8 describes, and in a more embarrassing way. A user who sets `sync.vad.enabled = false` cannot use `subx sync --method manual --offset 2.5` — the manual path that exists for exactly that situation — because the engine refuses to exist. `timeline-sync`'s *Manual Offset Without Video* requirement promises that path works.
2. It removes the argument that the free function is a workaround for an unavoidable environmental failure. There is no environmental failure. The precondition is a configuration flag, and the free function does not need a flag.
3. It means the existing spec scenario's wording is imprecise but not wrong, so *Sync Method Selection*'s modification below keeps the disjunction verbatim and adds a clause about the free function rather than rewriting history. If a later change makes VAD initialisation genuinely fallible — loading a model, probing a device — the disjunct becomes live and the scenario is already correct.

Recorded here rather than acted on: this change does not repair the flag behaviour (Decision 9).

### Decision 8: The VAD-independent entry point is a free function in `core::sync`, and the offset guard stays on the method

**Chosen:**

```rust
// subx-core/src/core/sync/mod.rs
pub fn shift_subtitle_timing(subtitle: &mut Subtitle, offset_seconds: f32) -> Result<SyncResult>;
```

and `SyncEngine::apply_manual_offset` becomes:

```rust
pub fn apply_manual_offset(&self, subtitle: &mut Subtitle, offset_seconds: f32) -> Result<SyncResult> {
    if offset_seconds.abs() > self.config.max_offset_seconds { /* today's error, verbatim */ }
    shift_subtitle_timing(subtitle, offset_seconds)
}
```

Four properties, each of which was the reason to prefer this over the alternatives:

**It goes where A2 already put the module's free functions.** `core::sync::mod.rs` gains `resolve_sync_pairing` and `create_default_output_path` in A2 (SDR §2.1). A third free function in the same module needs no new module, no new public path segment, and no re-export decision. The name follows the same verb-first shape and does not collide with the method, so `use subx_core::core::sync::shift_subtitle_timing;` and `engine.apply_manual_offset(…)` coexist unambiguously in one scope.

**The split is exactly where the duplication is.** The GUI's `shift_subtitle` (`../subx/src-tauri/src/commands/sync.rs:286-300`) contains the shift and *not* the guard: it checks `max_offset_seconds` separately, thirty lines earlier (`:241-244`), and deliberately in whole milliseconds rather than seconds because the frontend was handed a millisecond limit and re-deriving it from the `f32` would put the two checks on opposite sides of a rounding boundary. So the GUI has a considered reason to own its guard and no reason at all to own the shift. A free function that is guard-free is a precise fit; one that clamped would be unusable and the duplication would survive.

**There is exactly one implementation of the shift.** The two semantics that the GUI's comment calls "the two rules that must not drift" — `checked_add` erroring on positive overflow, and clamping to `Duration::ZERO` on negative underflow — exist once, in core, and both callers reach them. Worth recording: the two implementations agree today. Core writes `if entry.start_time > offset_dur { entry.start_time - offset_dur } else { Duration::ZERO }` (`:180-190`); the GUI writes `entry.start_time.saturating_sub(magnitude)`. These are the same function over `Duration` for every input, including the `==` case. The duplication was faithful; it just should not have been necessary.

**The `SyncResult` comes back.** Returning `Result<SyncResult>` rather than `Result<()>` means the method body is a guard plus a delegation with no post-processing, so `method_used`, `confidence`, `correlation_peak`, `additional_info` and `processing_duration` are constructed in one place. A `Result<()>` free function would force the method to rebuild the `SyncResult`, which is a second implementation of the part most likely to drift silently — `additional_info`'s JSON shape is consumed by `timeline-sync`'s *First-Sentence Offset Annotation* and by the CLI's JSON envelope.

**Rejected: `SyncEngine::without_vad(config)` (or `for_manual_offset`).** It keeps everything on the type, which reads tidier. It is worse in two ways. First, it creates an engine on which `detect_sync_offset` is a runtime failure — a second, less obvious version of exactly the defect being fixed, where a caller holds a `SyncEngine` that cannot do the thing `SyncEngine` is named for. Second, once `shift_subtitle_timing` exists, this constructor has no users: nothing else on `SyncEngine` is VAD-free. Adding a constructor whose only purpose is to reach one method that no longer needs it is dead API on arrival.

**Rejected: a fallible-lazy `vad_detector` field** — `OnceCell<Result<VadSyncDetector>>`, initialised on first detection, with `SyncEngine::new` becoming infallible-in-practice. It removes the precondition at its source, which is genuinely the more principled fix, and it is out of scope for three reasons. (a) It changes `SyncEngine::new`'s *behaviour* while keeping its signature, and the GUI depends on the behaviour: `run_detection` (`sync.rs:196`) maps the constructor's error to a specific user-facing `vad_unavailable` message, and moving that failure to `detect_sync_offset` moves it past the point where the GUI reports it. (b) It makes `timeline-sync`'s *VAD detector is required unconditionally* scenario false, which is a requirement change with the CLI on the other side of it. (c) `OnceCell` in a shared field is where the `Sync` question gets genuinely interesting, and mixing that into the change whose other half is "make things `Sync`" is how a one-day change becomes three. Decision 9 states the condition under which it is taken up.

**Rejected: a `ManualOffset` value type** with a `new(config) -> Result<Self>` and an `apply` method, so the guard travels with its configuration. It is the shape a larger API would want, and here it is a struct with one `f32` field wrapping a function call. No caller asked for it: the CLI has a `SyncEngine`, and the GUI has its own guard.

### Decision 9: Eager VAD initialisation is a defect, and repairing it is a separate change

Is `SyncEngine::new`'s eager, unconditional VAD requirement itself the bug, with the free function merely routing around it?

**Partly yes, and that is stated rather than hedged.** Decision 7 shows the only reachable trigger is `config.vad.enabled == false`, and a constructor that refuses because a feature is *switched off* is not enforcing a precondition — it is refusing to represent a valid configuration. The `if vad_detector.is_none()` at `:56-61` makes the `config.vad.enabled` branch at `:37-54` pointless: both arms end in the same error. That is a defect on its own terms, independent of the GUI.

**It is not repaired here, for reasons that are about consumers rather than effort.** Repairing it means deciding what `subx sync --method vad` does when `vad.enabled = false` (fail at detection, or ignore the flag), what `SyncEngine::new` returns instead of `Err`, and how `timeline-sync`'s *Sync Method Selection* and *Manual Offset Without Video* requirements are restated to match. Each of those has the CLI's `sync_command.rs` and the GUI's `run_detection` error mapping on the other side. That is a behaviour change to a shipped command, and it belongs in a change whose subject *is* that behaviour — not in one whose subject is thread safety and whose second half is an additive function.

**What this change does instead:** it makes the defect harmless for the caller who was blocked by it, and records the defect where the next reader will find it. `shift_subtitle_timing`'s rustdoc states that it exists because `SyncEngine::new` requires a VAD detector even for manual offsets and links the requirement; `SyncEngine::new`'s rustdoc states that the requirement is unconditional and that manual-offset-only callers should use the free function. Neither says "this is a bug" — a rustdoc comment is the wrong place for that — and the `## Open Questions` section carries it forward as a named candidate.

**The condition under which it is taken up:** if a later change makes VAD initialisation genuinely fallible (loading a model file, opening a device, probing a codec), the second disjunct of Decision 7 becomes live, the eager path starts failing for environmental reasons rather than configuration ones, and the lazy-field design rejected in Decision 8 becomes the right answer rather than an over-reach. `shift_subtitle_timing` is unaffected either way, which is the point of it being a free function.

### Decision 10: Three delta specs — and why `subtitle-translation` and `component-factory` are not among them

The set is `async-runtime-safety`, `format-conversion` and `timeline-sync`, chosen against C2b's classification tables so that every requirement touched is one C2b marks **C** and therefore one that travels to `subx-core` intact.

**`async-runtime-safety` — one ADDED requirement, and it is the right home for the cross-cutting guarantee.** The capability moves *wholesale* to `subx-core` in C2a (SDR §9's CORE list), so it has no CLI half to worry about, and its Purpose — "Protect the tokio runtime from blocking operations and ensure scheduler state remains consistent across every control-flow path" — is already about what the library must be for a multi-threaded runtime to hold it. Its three existing requirements are about `spawn_blocking` and scheduler bookkeeping; "the types a runtime holds must be `Send + Sync`" is the same subject one level up. Putting the cross-cutting statement here, once, is what lets the other two deltas stay narrow.

**`format-conversion` — one MODIFIED requirement.** *Public format API stability across module reorganization* is the requirement that enumerates `SubtitleFormat` among the frozen public items and pins its "full method signatures … in arity, parameter types, return types, and default-method semantics". A supertrait list is part of that same surface and is the thing an implementor must satisfy, so this is where it belongs and where an implementor will look. C2b marks the requirement **C** with a note that it "needs one migration note because 'other modules in `subx-cli`' now reach it through the D11 re-export"; the restatement below keeps that clause intact so C2b's migration note still applies to the same sentence.

**`timeline-sync` — one ADDED, three MODIFIED.** The ADDED requirement is the new entry point. The three modifications are not cosmetic: *Subtitle Timing Application* and *Offset Clamping Against Maximum* currently locate both the shift and the guard on `apply_manual_offset`, and after this change those two obligations sit on different items — which is precisely the distinction someone would otherwise "tidy up" by moving the guard into the free function, silently breaking the GUI's millisecond-boundary reasoning. *Sync Method Selection* is modified because its second scenario is the recorded reason the duplication exists, and leaving it unqualified would make the capability read as self-contradictory next to the new requirement. All four are **C** in C2b's `timeline-sync` table.

**`subtitle-translation` — excluded, deliberately.** `TranslationEngine` is one of the three types gap 1 names, and *AI Provider Translation* is its core-classified requirement, so a delta here is the obvious move. It is declined because the `async-runtime-safety` requirement names `TranslationEngine` explicitly: a second statement would be two requirements in two capabilities asserting one property, which is the overlap C2b Decision 3 spends a page avoiding and which C2b Decision 11 names as the failure mode a split pair cannot detect. Nothing in `subtitle-translation`'s twelve requirements says anything about construction context, threading or storage, so there is no gap either — the property is stated once, in the capability whose subject it is.

**`component-factory` — excluded, on evidence.** `ComponentFactory { config: Config }` (`factory.rs:42-44`) is already `Send + Sync`; no factory method's signature changes; `ComponentFactory::new`'s signature is A1's locked constraint and is not touched. Its six core requirements — *ConfigService-Driven Construction*, *AI Provider Creation*, *Pre-Construction Configuration Validation*, *Match Engine Creation*, *VAD and Audio Component Creation*, *Tests Use TestConfigService via TestConfigBuilder* — are all still exactly true afterwards. `ComponentFactory` does appear in the assertion set of Decision 4, which is a statement about it, and that statement lives in `async-runtime-safety` with the other seven. Adding a `component-factory` delta to restate a requirement that does not change would be a delta that validates and says nothing.

**`crate-topology` — considered and excluded.** B2's *Public API Path Stability for the Library Surface* governs paths and says reshaping them "SHALL be treated as a breaking change requiring a major version of `subx-core`". This change adds no path, removes none and reshapes none; the trait is reached at exactly `subx_core::core::formats::SubtitleFormat` before and after. The semver rule for auto-trait guarantees is stated in the `async-runtime-safety` requirement instead, where it sits next to the guarantee it qualifies.

### Decision 11: What `../subx` can delete, file by file

The payoff, concretely. None of this is done by this change — `../subx` is read-only reference here, and the deletion is a follow-up PR in that repository, exactly as SDR §7's `subx_cli::` → `subx_core::` migration is. Line ranges are against that repository as it stands.

**Gap 1 removals:**

| Location | Lines | What it is |
|---|---|---|
| `src-tauri/src/commands/convert.rs:394-423` | 30 | `convert_off_thread` in full — the 9-line rationale plus the `spawn_blocking` + `new_current_thread` + `block_on` wrapper. `convert_one` (`:342`) awaits `FormatConverter::convert_file` directly, and the converter can be built once in `run_batch` (`:249`) instead of per file |
| `src-tauri/src/commands/translate.rs:459-470` | 12 | `translate_batch_off_thread`'s rationale paragraph |
| `src-tauri/src/commands/translate.rs:482-487`, `:552-554` | 10 | the `spawn_blocking` + `new_current_thread` + `block_on` wrapper itself. The per-item loop it encloses is real logic and moves up into `run_batch` (`:390`) unchanged; `BatchResult` (`:453-457`) becomes unnecessary once the loop is no longer behind a `JoinHandle` |
| `src-tauri/src/commands/translate.rs:193-195` | 3 | the "Not `async`: `FormatManager` holds boxed format handlers that are not `Send`" note on `scan_translate_inputs`, which can then become `async` |
| `src-tauri/src/commands/translate.rs:283-288` | 6 | the block scope and comment that exist only to drop a `FormatManager` before an `.await` |
| `src-tauri/src/commands/sync.rs:344-347` | 4 | the same rationale on `load_subtitle`; a single `FormatManager` can then be shared by `load_subtitle` and `save_subtitle` instead of built and dropped twice |

**Gap 2 removals:**

| Location | Lines | What it is |
|---|---|---|
| `src-tauri/src/commands/sync.rs:278-300` | 23 | `shift_subtitle` in full. `apply_sync_offset_impl:269` calls `shift_subtitle_timing(&mut subtitle, offset_seconds)` instead, keeping its own millisecond guard at `:241-244` |
| `src-tauri/src/commands/sync.rs:604-630` | 27 | the two drift-guard tests — `a_negative_offset_clamps_at_zero_exactly_as_the_crate_does` and `a_positive_offset_beyond_the_representable_range_is_rejected`. Both assert core's semantics through a copy; once there is no copy, core's own unit tests are the assertion |
| `src-tauri/src/commands/sync.rs:13-17` | 5 | the module-doc paragraph explaining why one function is mirrored rather than called |
| `src-tauri/src/commands/sync.rs:191-195` | 5 | the `SyncEngine::new` "only thing it can fail on … design D6" comment, which Decision 7 shows is imprecise as well as no longer load-bearing |

**~163 lines**, of which ~50 are prose whose only subject is a constraint that will no longer exist, and 23 are an algorithm that exists twice. Two of that repository's design decisions — its D1 ("nothing is held between detect and apply") and D6 (the mirrored offset) — are stated in its own docs as consequences of these gaps and can be revisited.

**What is verified here rather than assumed.** Task phase 5 builds `../subx` against the patched `subx-core` in a scratch worktree, with the four `spawn_blocking` wrappers removed, and confirms it compiles. That is the only real test of gap 1: `Send`-ness is a property of the *consumer's* futures, and an assertion inside `subx-core` proves the fields are right without proving Tauri's bound is met. The scratch worktree is discarded; nothing is committed to that repository.

### Decision 12: Sizing — this one fits, and the measurement is stated for the same reason five siblings stated theirs

Five siblings measured themselves, found an overrun and named a seam (B3 → B4 at ~14.5 h; C1, C2a and C2b each naming their own division; C3 → C4). This change measures **~7.5 h** and needs no seam, which is worth saying explicitly because a bare absence of a `## Sizing` section reads as an omission in this series rather than as a claim.

| Work | Estimate |
|---|---|
| Baseline: confirm B2 and C1 landed, C2a has not, both roots green, re-verify the four implementors and the `FormatManager`-holder set against the post-B2 tree | 0.75 h |
| The supertrait, plus the rustdoc and the `rust,ignore` example, plus whatever fallout `cargo check --workspace --all-targets` finds (expected: none) | 1.0 h |
| The `thread_safety` assertion module, eight entries | 0.5 h |
| `shift_subtitle_timing`: extract the body, rewrite `apply_manual_offset` as guard + delegation, six unit tests plus the delegation-equivalence test | 1.75 h |
| Downstream verification: scratch worktree of `../subx`, remove the four wrappers, `cargo check` | 1.25 h |
| Three delta specs (one ADDED requirement, one restated requirement, one ADDED + three restated) and `openspec validate --strict` | 1.0 h |
| Documentation: two CHANGELOGs, `docs/tech-architecture.md`, the C2a/C2b hand-off note | 0.5 h |
| Quality gate | 0.75 h |

The soft item is the downstream verification, which depends on how cleanly `../subx`'s `run_batch` functions absorb the loops the wrappers currently enclose. If that runs long it is **cut, not extended**: it is a confirmation of a property the assertion module already establishes on this side of the boundary, and it produces no committed artifact. Nothing else in the list is uncertain — the code diff is two words, one moved function body and eight `const _` lines.

## Risks / Trade-offs

- **Risk: the supertrait is a formal semver break and the sequencing slips, landing it after `subx-core` 1.0.0 ships.** → Mitigation: Decision 6 makes "C1 landed, release tag not yet cut, C2a not yet landed" an explicit precondition checked in task 1.1, and task 1.2 makes it falsifiable rather than asserted — `git tag --list 'v*'` in both repositories and `cargo search subx-core`. If the tag exists, the change stops there and is re-proposed as a 2.0.0 change with C1's dependency line and pointer-check job in scope. This is the one risk that cannot be repaired after the fact.
- **Risk: a fifth `SubtitleFormat` implementor is added later and is not `Send + Sync`.** → Mitigation: it fails at compile time in `FormatManager::new`, in `subx-core`, with a clear error naming the missing bound — not in `../subx` and not at runtime. The trait rustdoc states the obligation under `# Implementation Notes`, and Decision 2 records why a subtitle parser that cannot satisfy it is a defect on its own terms.
- **Risk: the assertion module rots into a list nobody updates, so a new engine type silently escapes the contract.** → Mitigation: it cannot silently escape, because a type not in the list was never guaranteed — the failure is a missing guarantee, not a false one. The `async-runtime-safety` requirement names the eight and requires that a new engine type be added, so the review question ("did you add it to the assertion module?") is anchored in a spec rather than in habit.
- **Risk: extracting `apply_manual_offset`'s body changes its behaviour.** → Mitigation: the body moves verbatim — the `checked_add` pair, the `if start_time > offset_dur` clamp, and the whole `SyncResult` literal including `additional_info`'s two JSON keys. The two existing tests (`engine.rs:357`, `:375`) are **not** rewritten to target the free function; they keep calling `apply_manual_offset` and are the regression check. The new delegation test asserts the two paths produce identical results for the same input, so a divergence fails rather than being reviewed by eye.
- **Risk: `shift_subtitle_timing` is guard-free, and a future caller uses it where a guard was expected, applying an unbounded offset.** → Mitigation: this is a real trade-off, not an oversight, and it is why *Offset Clamping Against Maximum* is restated rather than left alone — the restatement says which item owns the guard and that the free function deliberately does not. The rustdoc's `# Errors` section states that `sync.max_offset_seconds` is **not** enforced and names `SyncEngine::apply_manual_offset` as the entry point that does. The CLI reaches the transform only through the method.
- **Risk: `subtitle-translation` gets no delta, and a reader of that capability alone never learns `TranslationEngine` is `Send + Sync`.** → Mitigation: accepted, and it is the lesser of the two failures C2b Decision 11 describes. A missing cross-reference is recoverable by reading one more capability; two capabilities independently asserting one property is the drift that C2b's title-intersection check cannot detect. The `async-runtime-safety` requirement names `TranslationEngine` and `subx-core/src/core/translation/engine.rs` explicitly, so a grep for either finds it.
- **Risk: the downstream verification gives false confidence because it patches `../subx` in a scratch worktree that is then discarded.** → Mitigation: what it proves is narrow and stated as such — that Tauri's `Send` bound on an async command is satisfiable with the wrappers gone. It is not a claim that the deletions in Decision 11 are complete or that they are the best shape for that repository; those are decisions for the PR in it. If the scratch check is cut for time, the change still lands on the strength of the assertion module.

## Migration Plan

1. **Preconditions.** Confirm B2 and C1 landed; confirm C2a has **not** archived (`openspec/specs/async-runtime-safety/` still exists in this repository); confirm no `v*` release tag exists in either repository and `subx-core` is not on crates.io. Any of these failing changes the shape of the change — see Decision 6's table and the first risk.
2. **Gap 1 first, because it is the two-word half.** Supertrait, rustdoc, assertion module, `cargo check --workspace --all-targets`, `cargo clippy --workspace -- -D warnings`. This half is done or obviously broken within the first hour.
3. **Gap 2 second.** Extract `shift_subtitle_timing`; rewrite `apply_manual_offset` as guard + delegation; add the six unit tests and the delegation test; confirm the two existing tests still pass unmodified.
4. **Downstream verification.** Scratch worktree of `../subx`, wrappers removed, `cargo check`. Discard.
5. **Specs and docs.** Three delta files, `openspec validate make-core-engines-thread-safe --strict`, two CHANGELOGs, `docs/tech-architecture.md`.
6. **Hand-off to C2a and C2b, written into this change's completion notes.** C2a's `async-runtime-safety` migration must carry **four** requirements, not three — *Library Engine Types Are `Send` and `Sync`* is the fourth, and it belongs in `import-core-specs`'s ADDED delta with its citations un-prefixed. C2b's `format-conversion` table must show *Public format API stability across module reorganization* restated from **this** change's text (it already carries a migration note on the same requirement, so the restatement is one it was going to make anyway), and its `timeline-sync` table gains *VAD-Independent Manual Offset Application* as a fifth **C** entry, taking that capability's split from 9 C / 10 L to 10 C / 10 L. All five requirements are core; none creates a CLI half; the split counts in C2b's Decision 2 and its `## Sizing` shift by one requirement.
7. **The follow-up PR in `../subx`**, which is not part of this change: switch the four call sites, delete the ~163 lines of Decision 11, and revisit that repository's design notes D1 and D6.

## Open Questions

- **Should `SyncEngine::new` stop refusing when `sync.vad.enabled = false`?** Decision 9 says yes on the merits and not here. It is a behaviour change to `subx sync` with both the CLI and the GUI's error mapping on the other side, and it needs `timeline-sync`'s *Sync Method Selection* and *Manual Offset Without Video* restated together. Deferred with the condition named: it becomes urgent the moment VAD initialisation acquires a genuinely fallible step.
- **`SyncEngine::auto_detect_sync_offset` (`sync/engine.rs:113-131`) is dead.** It is private, and `detect_sync_offset` routes `SyncMethod::Auto` straight to `vad_detect_sync_offset` (`:97-99`), so its `"No detector available in auto mode"` branch is unreachable. Deleting it is correct and is not this change's business; noted because it sits four lines from code this change edits and will look like an omission to the next reader.
- **Does `subx-core` want a compile-fail test asserting the *negative* — that a non-`Send` `SubtitleFormat` implementor is rejected?** It would need `trybuild`, a new dev-dependency, for one case whose failure mode is already a plain compile error. Declined here; raised because Decision 2's "the set is closed" argument is the only thing standing between this bound and a future implementor, and a `trybuild` case would turn that argument into a test.
- **`docs/tech-architecture.md` lives in `subx-cli` and describes core's format system.** This change adds one sentence to it about the thread-safety guarantee. C3 rewrites the file for the two-crate world and may decide the sentence belongs in `subx-core`'s own documentation instead; if so, C3 moves it. Flagged so the sentence is not written twice.
