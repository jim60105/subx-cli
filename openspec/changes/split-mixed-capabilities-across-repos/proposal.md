## Why

C2a moved the capabilities that are entirely core-owned into `subx-core` and wrote down the rule by which ownership is decided. It deliberately left thirteen capabilities behind, because each of them specifies behaviour implemented on *both* sides of the crate boundary. Those thirteen are now the only reason `subx-cli/openspec/specs/` still contains requirements that no `subx-cli` file can satisfy.

The cost is concrete and it is `spec-governance`'s own language that names it. `spec-governance` requires that every requirement be owned by exactly one repository, that an unqualified path citation resolve in the owning repository, and that a `## Purpose` name only paths that exist there. Today `configuration-management`'s Purpose ends "Implemented in `src/config/`", and after B2 `src/config/` is in the other repository; `component-factory`'s Purpose names `src/core/factory.rs` and `src/commands/match_command.rs` in the same sentence, one of which has moved; `error-handling`'s *Stable Machine-Readable Category and Code* governs `subx_core::error`, which the Tauri GUI at `../subx` consumes directly and which a `subx-core` contributor cannot find. Thirteen capabilities, **134 requirements**, are in this state. C2a shipped the rule; this change is the second half of applying it.

This is series change **C2b**. It splits each of the thirteen along the core/CLI line: the core half migrates into `subx-core/openspec/specs/`, the CLI half stays here. Like C2a it spans both repositories, and its `subx-core` half is a separate change, **`import-split-capability-specs`**, authored in `subx-core/openspec/changes/` and archived there. Neither half is complete alone and each names the other, as `spec-governance`'s *A Change Spanning Both Repositories Is Authored as Two Changes and Archived by Hand* requires.

It runs strictly after C2a. The two share the `openspec/specs/` tree, C2b needs the `subx-core` OpenSpec root C2a creates, and — less obviously — C2b needs C2a's `spec-governance` requirements as the thing it applies rather than as a judgement call it re-derives. It also inherits three specific hand-offs from C2a: the two demoted capabilities (`component-factory`, `parallel-processing`), the three `local-llm-provider` references that must be re-qualified once their targets split, and the recorded split set that C2a's drift check reads and C2a's Open Questions leave to C2b.

**Three corrections to the record, each with its evidence in `design.md` Decision 1.**

- **`supply-chain-hardening` is not split, and cannot be.** After A0 made *Every Declared Dependency Has a Use Site* explicitly per-manifest ("When the project is split across more than one crate, each crate's manifest SHALL satisfy it against its own source trees") and C1 added *Submodule Pointer Is a Supply-Chain Input* and *Each Published Crate Is Audited in Its Own Repository*, not one of its eight requirements is core-only. Every one either constrains both manifests or constrains the superproject alone. Splitting it would require writing the same per-manifest rule twice in two repositories with nothing keeping the copies in agreement — the duplication `spec-governance` forbids. It stays wholesale in `subx-cli`, with two requirements restated so their pre-split single-manifest phrasing reads correctly against a two-manifest project. **Twelve capabilities are split, not thirteen.**
- **`parallel-processing`'s CLI residue is three requirements' worth, not two.** C2a named *Aggregated Result Reporting* and *Progress Reporting Opt-Out*. It missed *Task Scheduler Entry Point*, both of whose scenarios are CLI: one requires the command to "report the number of tasks to be processed and the maximum concurrency to the user", the other requires it to print `No video files found to process` — and that literal is at `src/commands/match_command.rs:718`, inside `execute_parallel_match`, alongside `monitor_batch_execution` at `:765`. The requirement's normative subject, `TaskScheduler::new()`, is core; the requirement is split.
- **`component-factory`'s CLI residue is exactly the one requirement C2a named.** *Commands Consume Services via Dependency Injection* is confirmed. Its neighbour *Tests Use TestConfigService via TestConfigBuilder* looks like a second, but all three integration tests it cites (`tests/openrouter_integration_tests.rs`, `tests/azure_openai_api_integration_tests.rs`, `tests/dependency_injection_integration_tests.rs`) are core-bound under B3's ownership test, and its unit-test scenario cites `src/core/factory.rs`. It is core, carrying one qualified citation.

## What Changes

**1. Twelve capabilities are split along the core/CLI line.** Every one of the 134 requirements is classified individually; `design.md` Decision 2 carries the per-requirement table with the evidence. The resulting allocation:

| Capability | Reqs before | → core | → CLI | Divided | New reqs |
|---|---:|---:|---:|---:|---:|
| `cache-management` | 8 | 6 | 2 | 0 | 0 |
| `component-factory` | 7 | 6 | 1 | 0 | 0 |
| `configuration-management` | 17 | 14 | 4 | 1 | +1 |
| `encoding-detection` | 7 | 2 | 6 | 1 | +1 |
| `error-handling` | 13 | 10 | 6 | 3 | +3 |
| `format-conversion` | 9 | 5 | 5 | 1 | +1 |
| `input-path-handling` | 13 | 12 | 2 | 6 | +1 |
| `parallel-processing` | 10 | 8 | 3 | 1 | +1 |
| `secrets-protection` | 4 | 4 | 1 | 1 | +1 |
| `subtitle-matching` | 8 | 6 | 4 | 5 | +2 |
| `subtitle-translation` | 12 | 7 | 6 | 1 | +1 |
| `supply-chain-hardening` | 8 | 0 | 8 | 0 (stays) | 0 |
| `timeline-sync` | 18 | 9 | 10 | 3 | +1 |
| **Total** | **134** | **89** | **58** | **23** | **+13** |

Counts are the **end state after A0, A1, A2 and C2a**, not today's files: A2 adds one requirement to `input-path-handling` and two to `timeline-sync` and modifies four in `error-handling`, A0 and C1 add five to `supply-chain-hardening`, and C2a modifies `configuration-management`'s *Local Provider Validation Rules*. 89 + 58 = 147, thirteen more than 134.

**2. Twenty-three requirements straddle the line and have their content divided; the project gains thirteen requirements, not twenty-three.** They are named, with their reason and their halves, in `design.md` Decision 3. Nine become two requirements each — one per repository — which accounts for +9. The other fourteen contribute a single CLI clause apiece, and those fourteen clauses are **gathered** into four CLI requirements rather than producing fourteen one-sentence halves: one each in `input-path-handling` and `timeline-sync`, two in `subtitle-matching`. That accounts for +4, and it is Decision 4.

**3. `subx-cli`'s side of the split is thirteen delta files** — one per MIXED capability — holding **82 `## REMOVED Requirements`** entries, **12 `## MODIFIED Requirements`** entries and **6 `## ADDED Requirements`** entries. Forty retained requirements are genuinely untouched and are not restated; `design.md` Decision 6 states the test applied to decide that. `supply-chain-hardening`'s delta is the one file with no removals at all.

**4. `subx-core`'s side is one change, `import-split-capability-specs`,** with `.openspec.yaml`, `proposal.md`, `design.md`, twelve `## ADDED Requirements` delta specs carrying 89 requirements, one `## MODIFIED Requirements` delta re-qualifying three references in the already-imported `local-llm-provider`, and `tasks.md`. Its artifact set is specified file by file in `design.md` Decision 8, following the shape C2a established for `import-core-specs` — including the `# <name> Specification` H1 and `TBD` Purpose that `openspec archive` emits on the receiving side and that must be rewritten by hand.

**5. Twenty-four `## Purpose` paragraphs are rewritten by hand.** Twelve in `subx-core` (replacing the archive's `TBD`) and — new to this change, and the part C2a did not have to do — twelve in `subx-cli`, because a split capability's retained half inherits a Purpose that names the other repository's paths. `spec-governance`'s scenario *A Purpose names only the owning repository's paths* uses `component-factory` as its worked example; this change is where that scenario is discharged. Two pre-existing H1 defects are repaired at the same time: `subtitle-translation`'s main spec is titled `# Subtitle Translation Specification`, and its `subx-core` counterpart must not inherit that.

**6. C2a's `configuration-management` qualification is deliberately un-qualified, not reverted.** C2a restated *Local Provider Validation Rules* so that its reference to `ai-provider-integration` names `subx-core`, and recorded as a risk that C2b might silently revert it. This change does the opposite of reverting: it restates the **post-C2a** text and then removes the qualification, because the requirement itself now moves to `subx-core`, where a reference to `ai-provider-integration` is a same-repository reference and `spec-governance` requires unqualified form. The added scenario C2a wrote for that qualification is retired with it. Symmetrically, `local-llm-provider`'s three references — qualified to `subx-cli` by C2a — are re-qualified to `subx-core`, discharging C2a's fourth Open Question.

**7. The recorded split set is created.** `spec-governance`'s drift check asserts that no capability name appears in both trees except where the capability is deliberately split, and that the split set is recorded in one place. C2a left the location of that record as an Open Question for C2b. This change answers it: a committed `openspec/split-capabilities.txt` in `subx-cli`, twelve lines, one capability name per line — resolved in `design.md` Decision 10, which also says why it is not a table inside `spec-governance`'s own spec.

**8. The change stays whole at ~10.25 h, and the seam it declines is named with the condition that triggers it.** The obvious seam is the capability boundary, and it is rejected for a reason specific to this change: between the two halves of a seamed C2b, five capability names would exist in both trees while the recorded split set listed only eight, so `spec-governance`'s drift check — the rule C2a landed and this change is the first to exercise — would have to be absent, wrong, or relaxed to tolerate exactly the condition it exists to detect. `design.md` Decision 12 gives the division that would be taken if task 1's measurement runs long, and names the follow-on `harden-split-capability-specs`.

No source file, test, manifest, workflow or script is edited in either repository. `subx-cli` stays at the version C1 set.

## Capabilities

### New Capabilities

- _None._

### Modified Capabilities

- `cache-management` — six requirements removed to `subx-core` (cache location, invalidation, key derivation and reuse, all implemented in `src/core/matcher/{cache,engine}.rs`); *Cache Clear Subcommand* restated because the CLI resolves the cache path independently at `src/commands/cache_command.rs:224` and must be held to agreement with the core resolver at `src/core/matcher/engine.rs:2244`.
- `component-factory` — six requirements removed to `subx-core`; *Commands Consume Services via Dependency Injection* retained untouched as the sole CLI half, confirming C2a's demotion.
- `configuration-management` — thirteen requirements removed to `subx-core`; *Repair Path For Strict-Invalid Configuration* restated as the CLI half of a split whose core half becomes *Tolerant Configuration Load Path*.
- `encoding-detection` — *Low-Confidence Fallback To Default Encoding* removed to `subx-core`; *Robust Handling of Empty and Binary Files* restated as the CLI half of a split whose core half becomes *Detector Tolerates Empty and Binary Input*. This is the most lopsided split in the set, two core requirements against six, and Decision 5 argues why it is still a split.
- `error-handling` — eight requirements removed to `subx-core`, including A2's *Library and Binary Error Surface Split*, which is split in two because it defines both halves by construction; *User-Facing Error Formatting* and *No Panics On Recoverable Errors* restated as CLI halves; *Binary Error Surface Adds Presentation Through an Extension Trait* added as the CLI counterpart of the surface-split requirement.
- `format-conversion` — four requirements removed to `subx-core`; *Supported Output Formats* restated as the CLI half (the `--format` value set and `OutputSubtitleFormat`, which lives in `src/cli/convert_args.rs`) of a split whose core half becomes *Target Format Conversion Semantics*.
- `input-path-handling` — twelve requirements removed to `subx-core`, which is almost the whole capability; *Input Argument Structs Are Thin Adapters Over Core Collection* added, gathering, in five bullets, the CLI clauses lifted out of six core requirements. The honest CLI residue is **not** only the flag surface: *Output Directory Resolution for Archive Files* is retained untouched and is a substantive requirement, implemented inline at `src/commands/convert_command.rs:362`, `match_command.rs:459`, `sync_command.rs:512,585` and `translate_command.rs:330-389`.
- `parallel-processing` — eight requirements removed to `subx-core`, including *Task Scheduler Entry Point* whose core half keeps the title; *Parallel Match Reports Task Count and Handles an Empty Input Set* added as its CLI half. *Aggregated Result Reporting* and *Progress Reporting Opt-Out* retained untouched, as C2a predicted.
- `secrets-protection` — three requirements removed to `subx-core`; *Mask sensitive config values in CLI output* restated as the CLI half of a split whose core half becomes *Sensitive Value Masking Helper*.
- `subtitle-matching` — six requirements removed to `subx-core`; *Dry-Run and Execution Modes* restated because it named `execute_operations`, now core; *Match Command Argument Surface and Input Preconditions* and *Match Command Applies Archive-Origin Relocation Before Uniqueness Allocation* added, the second because `apply_unique_target_paths` is invoked at `src/commands/match_command.rs:470` immediately after the archive-origin rewrite at `:459`, and that call order is a real CLI obligation the core allocator cannot enforce.
- `subtitle-translation` — six requirements removed to `subx-core`; *Translation Guidance Options* restated as the CLI half (flag definitions and glossary file reading at `src/commands/translate_command.rs:163-174`) of a split whose core half becomes *Translation Prompt Guidance Inputs*.
- `supply-chain-hardening` — **nothing removed.** *Replace unmaintained md5 crate* and *Narrow dependency feature flags* restated so that they read against two manifests rather than one; the capability stays wholesale in `subx-cli` under `spec-governance`'s *Capabilities Governing the Repository Relationship Live in `subx-cli`*.
- `timeline-sync` — nine requirements removed to `subx-core`, including A2's two additions and *Sync Method Selection* whose core half keeps the title; *Single-File and Batch Modes* restated because its reference to `crate::core::sync::resolve_sync_pairing` no longer resolves in this crate; *Sync Argument Struct Is a Thin Adapter Over Core Pairing* added, gathering the CLI clauses A2 wrote into three core requirements.

## Impact

- **Code:** None. No `.rs` file, `Cargo.toml`, workflow, or script is touched in either repository.
- **Tests:** None as code. This change creates the data file the `spec-governance` drift check reads (`openspec/split-capabilities.txt`); the check itself remains C3's to place, per C2a's first Open Question.
- **APIs:** None.
- **Dependencies:** None. `openspec` 1.6.0, already in use; no crate, feature or lockfile entry changes.
- **Documentation:** `AGENTS.md`'s "OpenSpec and Project Skills" section gains one paragraph on split capabilities — that twelve names appear in both trees by design, that the record of which twelve is `openspec/split-capabilities.txt`, and that adding a requirement to a split capability means deciding its side first. `CHANGELOG.md` in both repositories gains `[Unreleased]` entries under `### Added`, `### Changed`, `### Removed` and `### Documentation`.
- **Specifications:** `subx-cli/openspec/specs/` keeps 20 capability directories and loses 76 requirements net (134 → 58). `subx-core/openspec/specs/` goes from 13 capabilities to 25, and from 93 requirements to 182. Twelve capability names then exist in both trees, which is the first non-empty split set the project has had. The union of the two trees preserves every obligation: twenty-three requirements have their content divided, eight scenarios are added and one retired (all ten changes enumerated in `design.md` Decision 3's scenario accounting), and no requirement is weakened, strengthened, merged or deleted.
