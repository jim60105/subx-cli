## Why

SDR §8 records six API gaps that force the downstream Tauri GUI (`../subx`) to duplicate logic `subx-core` already owns. D1 (`make-core-engines-thread-safe`) closed gaps 1 and 2. This change closes **gaps 3, 4, 5 and 6** — the last four — and with them the last reasons that repository has to re-derive `subx-core`'s internals. SDR D9 puts this change off the critical path: it blocks nothing and nothing waits on it.

**Gap 3 — `ComponentFactory::create_match_engine()` hardcodes `relocation_mode: None`.** The factory derives six of `MatchConfig`'s eight fields from the loaded `Config` (`subx-core/src/core/factory.rs:69-82`) and then pins the two the caller actually chooses. A caller that wants `--copy` or `--move` semantics therefore cannot use the factory at all, and must hand-build the whole struct. Both real callers do: the CLI at `subx-cli/src/commands/match_command.rs:423-433` and the GUI at `../subx/src-tauri/src/commands/match.rs:264-273`, which carries a four-line comment naming `create_match_engine()` as unusable. `MatchConfig` has all-`pub` fields and no `#[non_exhaustive]`, so every hand-built literal is a compile error waiting for the ninth field.

**Gap 4 — convert's output-path resolution is computed inline.** The rule is "beside the input, except archive-extracted files, which land beside the archive rather than inside an extraction directory the user never chose". It is written out inside the per-file loop of `subx-cli/src/commands/convert_command.rs:347-371` and nowhere else, so the GUI re-derives it at `../subx/src-tauri/src/commands/convert.rs:488-506`. The same rule, in a second shape, is re-derived a third time for translate: `translate_command.rs:383-392`'s private `default_output_path` is mirrored at `../subx/src-tauri/src/commands/translate.rs:725-730`, with a module-doc paragraph (`:12-18`) explaining why. C2b classified `input-path-handling`'s *Output Directory Resolution for Archive Files* as CLI-side **because** of this, and its own task 1.7 says so: "Zero hits under `src/commands/` would mean the requirement had migrated into core and must be reclassified."

**Gap 5 — match's archive-origin target rewrite is inline command logic.** `subx-cli/src/commands/match_command.rs:456-466` walks the operation list and, for every subtitle that came out of an archive, forces a `Copy` relocation into the matched video's directory. It must run *before* `apply_unique_target_paths` — a call-order constraint the allocator provably cannot enforce on itself — and `subtitle-matching`'s *AI-Driven Language and Globally-Unique Target Naming* already spells that ordering out, naming `match_command.rs` as the implementor. The GUI copies both the loop and the ordering comment at `../subx/src-tauri/src/commands/match.rs:393-408`.

**Gap 6 — no transport-agnostic progress or cancellation hook.** A1 built the `Reporter` seam with `fn progress(&self, event: &ProgressEvent<'_>)` and shipped `ProgressEvent` as `#[non_exhaustive]` with a single `Message(&str)` variant, stating in its own Non-Goals that the structured progress/cancellation API is this change's job. Today a core-owned loop can say *something happened* and nothing else: `MatchEngine::execute_operations_audit` (`engine.rs:1783`) walks N operations and reports nothing until it returns, so the GUI's `execute_selected_impl` (`../subx/src-tauri/src/commands/match.rs:340-345`) has no progress and no cancel — alone among its four wizards. On the CLI side the absence has produced a live defect: `match_command.rs:807-813` defines a *second*, private `create_progress_bar` that shadows `cli::ui::create_progress_bar`, bypasses `ui::progress_draw_target_for`, and therefore renders progress frames in `--output json` mode whenever `general.enable_progress_bar` is true — which `progress-reporting`'s *Progress Bars Force-Hidden in JSON Output Mode* forbids in the same breath as it says "ad-hoc `ProgressBar::new(...)` calls that bypass this helper SHALL be refactored to go through it".

The four gaps share one shape: a decision or a rule that is correct, tested and settled, but reachable only by copying it. Closing them deletes ~60 lines from `../subx` and gives its match wizard progress and cancellation it cannot build today.

## What Changes

**1. `ComponentFactory` gains a configuration accessor and a config-taking constructor.** Two new methods on `subx-core/src/core/factory.rs`:

```rust
pub fn match_config(&self) -> MatchConfig;
pub fn create_match_engine_with(&self, config: MatchConfig) -> Result<MatchEngine>;
```

`match_config()` returns exactly the struct `create_match_engine()` builds today, unchanged in every field including the `0.8` confidence default. `create_match_engine_with` builds the AI provider and injects the caller's config, propagating the factory's reporter per A1 Decision 6. `create_match_engine()` becomes `self.create_match_engine_with(self.match_config())` and its signature, behaviour and rustdoc contract are unchanged. `ComponentFactory::new`'s signature is untouched — A1's locked constraint.

**2. `CollectedFiles` gains the two output-location queries.** In `subx-core/src/core/input/mod.rs`, beside `archive_origin`:

```rust
pub fn default_output_dir<'a>(&'a self, input: &'a Path) -> &'a Path;
pub fn default_output_path(&self, input: &Path, extension: &str) -> PathBuf;
```

`default_output_dir` is the general rule: the archive's parent when `input` came from an archive, else `input`'s parent, else `.`. `default_output_path` is convert's resolver, byte-compatible with today's output text (see `design.md` Decision 3 — it is deliberately *not* `default_output_dir(..).join(..)`). Adopted by `convert_command.rs` and `translate_command.rs`; the two `sync_command.rs` sites are deliberately not adopted (Decision 3).

**3. `apply_archive_origin_relocation` becomes public core behaviour.** A free function in `subx-core/src/core/matcher/engine.rs`, beside `apply_unique_target_paths`:

```rust
pub fn apply_archive_origin_relocation(operations: &mut [MatchOperation], collected: &CollectedFiles);
```

Its body is `match_command.rs:456-466` verbatim. `match_command.rs` calls it, so there is one implementation and the "call this before the allocator" pairing is discoverable from one module.

**4. `ProgressEvent` gains three variants and `Reporter` gains one query.** In `subx-core/src/core/report/mod.rs`:

```rust
#[non_exhaustive]
pub enum ProgressEvent<'a> {
    Message(&'a str),                                        // A1
    Started { total: u64 },
    Advanced { done: u64, total: u64, item: Option<&'a str> },
    Finished { done: u64, total: u64 },
}

pub trait Reporter: Send + Sync {
    // … A1's four methods, unchanged …
    fn cancelled(&self) -> bool { false }
}
```

Every field is `u64` or `Option<&str>`, so the enum keeps its `Debug, Clone, PartialEq, Eq` derives. `cancelled()` is a **provided** method: no supertrait is added anywhere, which is the one hazard D1 identified and paid for.

**5. `MatchEngine`'s two execution loops emit progress; the audit loop observes cancellation.** `execute_operations` (`engine.rs:1557`) and `execute_operations_audit` (`:1743`) emit `Started` / `Advanced` / `Finished` around their operation loops. `execute_operations_audit` additionally checks `self.reporter.cancelled()` before starting each operation and, when it is true, stops and pads the remaining slots with `OperationOutcome { applied: false, error: None }` so `outcomes.len() == operations.len()` still holds — the same shape its dry-run branch already returns. `execute_operations` does **not** observe cancellation: its `Result<()>` cannot express "stopped early" without a signature change or a new `SubXError` variant, and AGENTS.md forbids both. No signature changes anywhere.

**6. The CLI adopts all of it, and the duplicate progress bar is deleted.** `cli::TerminalReporter` (A1) becomes the single owner of the batch `indicatif::ProgressBar`, creating it on `Started` through `ui::create_progress_bar` — hence through `ui::progress_draw_target_for` — and honouring `general.enable_progress_bar`. `match_command.rs:807-813`'s private `create_progress_bar` is deleted, `:729-737`'s hand-rolled draw-target block goes with it, and `monitor_batch_execution` reports through the reporter instead of holding a `&ProgressBar`. `ui::create_progress_bar`'s template gains a trailing `{msg}` so the `Active: … | Queued: … | Completed: …` line the parallel path already sets stays visible.

**7. Nothing is removed from the public surface, and no path is reshaped.** All six additions are new items at existing paths. B2's *Public API Path Stability for the Library Surface* is unaffected; the GUI's ~40-item consumed surface (SDR §8) is untouched.

## Capabilities

### New Capabilities

_None._ Every obligation this change creates belongs to a capability that already exists and already names the code it governs. C1 Decision 14 and C2a's `spec-governance` set that bar; `design.md` Decision 9 records the two candidates considered and declined.

### Modified Capabilities

- `core-reporting`: *Transport-Agnostic Reporter Seam* is modified — it is the requirement that enumerates `Reporter`'s methods and `ProgressEvent`'s variants, and both change. Two requirements are added: *Structured Progress Stream Semantics* (who may emit, in what order, and what `done < total` at `Finished` means) and *Cooperative Cancellation Through the Reporter* (`cancelled()`, where it is polled, and what a stopped loop returns). Wholesale core; migrates in C2a.
- `component-factory`: *Match Engine Creation* is modified. It is the requirement that spells out the six config-derived fields and the two hardcoded ones, so it is where a caller learns that `match_config()` exists and what it contains. C2b classifies it **C**.
- `input-path-handling`: *Output Directory Resolution for Archive Files* is modified. C2b classified it **L** on the evidence that it is implemented at five command call sites; this change moves the rule into `CollectedFiles`, so the requirement becomes a **split** — a core half stating the resolution rule and the two query methods, a CLI half keeping the `--output` precedence, the multi-input directory-append rule, `--replace`'s archive refusal and sync's conditional batch redirection. C2b's classification and split counts change; `design.md` Decision 8 carries the hand-off.
- `subtitle-matching`: *AI-Driven Language and Globally-Unique Target Naming* is modified. Its allocator paragraph and its *Allocator runs after archive-origin forced relocation* scenario both name `match_command` as the party that performs the rewrite; after this change the rewrite is `apply_archive_origin_relocation` in core and only the *call* is the command's. C2b classifies the requirement **C** and lifts its call-order clause into a CLI requirement, which this change makes narrower and more precise rather than removing.
- `progress-reporting`: *Progress Bar Styling* is modified (the template gains `{msg}`) and *Progress Bar Visibility Follows Configuration* is modified (the bar is constructed by the CLI reporter, not by the match command, and the flag is honoured there). CLI-only capability; stays in `subx-cli` (SDR §9). Its *Progress Bars Force-Hidden in JSON Output Mode* requirement is **not** modified — deleting the ad-hoc constructor is compliance with it, not a change to it.

`format-conversion` and `parallel-processing` were considered and excluded; `design.md` Decision 9 gives the evidence for each.

## Impact

- **Code:** `subx-core/src/core/factory.rs` (`match_config`, `create_match_engine_with`, `create_match_engine` delegates), `subx-core/src/core/input/mod.rs` (`default_output_dir`, `default_output_path`), `subx-core/src/core/matcher/engine.rs` (`apply_archive_origin_relocation`; progress emission and the cancellation check in the two execution loops), `subx-core/src/core/report/mod.rs` (three `ProgressEvent` variants, `Reporter::cancelled`). In `subx-cli`: `src/cli/reporter.rs` (`TerminalReporter` owns the bar), `src/cli/ui.rs` (`create_progress_bar` template), `src/commands/match_command.rs` (adopts `match_config`, `apply_archive_origin_relocation`; loses the duplicate `create_progress_bar` and the local draw-target block), `src/commands/convert_command.rs` and `src/commands/translate_command.rs` (adopt the two `CollectedFiles` queries).
- **Tests:** new unit tests in `subx-core` for `match_config()` field-for-field against `Config`, for `create_match_engine_with` honouring a non-`None` relocation mode, for both `CollectedFiles` queries including the no-parent and no-extension edges and the documented divergence between them, for `apply_archive_origin_relocation` (archive origin present / absent, `requires_relocation` already true, video with no parent), for the three new `ProgressEvent` variants' derives, and for `Reporter::cancelled`'s default. New integration-shaped tests for `execute_operations_audit`: the event sequence for N operations, cancellation before item k padding the remaining outcomes, and `outcomes.len() == operations.len()` in every case. In `subx-cli`: a characterisation test that `--output json` produces no progress frame on either stream through the reporter-owned bar (the assertion the deleted duplicate would have failed), and one that `general.enable_progress_bar = false` hides it.
- **APIs:** **Additive, all of it.** `subx_core::core::factory::ComponentFactory::{match_config, create_match_engine_with}`; `subx_core::core::input::CollectedFiles::{default_output_dir, default_output_path}`; `subx_core::core::matcher::engine::apply_archive_origin_relocation`; `ProgressEvent::{Started, Advanced, Finished}` on an already-`#[non_exhaustive]` enum; `Reporter::cancelled` as a provided method. `cargo-semver-checks` reports none of these as major. All six are reachable from `subx_cli::` through SDR D11's re-exports at the mirrored paths. **Not additive but not a semver event either:** `ui::create_progress_bar`'s rendered template gains `{msg}` — a behaviour change to a `subx-cli` public helper with an unchanged signature. `design.md` Decision 7 records the verdict for each item and names the two shapes that *would* have been major.
- **Dependencies:** None. No manifest in either repository changes. `tokio-util`'s `CancellationToken` was considered and rejected — A0 deleted that dependency as unused and re-adding it for one bool would reverse A0 (`design.md` Decision 6).
- **Documentation:** rustdoc with `# Arguments` / `# Returns` / `# Errors` / `# Examples` on all six new items; `ProgressEvent`'s enum-level rustdoc gains the stream contract (one open stream per reporter, monotonic `done`, `done ≤ total`); `Reporter::cancelled`'s rustdoc states that mid-`.await` cancellation is achieved by dropping the future and needs no API. `subx-core/CHANGELOG.md` gains `[Unreleased]` → `### Added` for the six items and `### Fixed` for nothing (core has no defect here); **`subx-cli/CHANGELOG.md`** gains `### Fixed` for the JSON-mode progress-frame leak and `### Changed` for the progress-bar template and the reporter-owned bar — the defect and the visible change are both `subx-cli`'s. `docs/configuration-guide.md`'s `general.enable_progress_bar` entry gains one sentence naming the CLI reporter as the enforcement point. `docs/tech-architecture.md` gains one sentence on the progress/cancellation seam, flagged for C3.
