## Context

`subx-cli/openspec/specs/` is the only specification tree SubX has. It was written when SubX was one crate, and every requirement in it cites paths relative to one repository root. After B2, B3, B4 and C1, that assumption is false for roughly half of it: `src/core/**`, `src/services/**`, `src/config/**` and `src/error.rs` are in `subx-core`, a separately clonable repository mounted as a submodule at `subx-core/`, and the tests that exercise them are there too.

The tree is also larger than SDR §9 records. SDR §9 was written on 2026-08-26 against 29 capabilities. Three more exist by the time this change runs: `crate-topology` (created by B1, extended by B2 and C1), `core-reporting` (created by A1) and `cross-crate-testing` (created by B3, extended by B4). SDR §9's totals are also off against the tree as it stands today — it records 262 requirements and 668 scenarios across the 29; the files hold **252 requirements and 611 scenarios**. Neither discrepancy changes any decision below, but both are recorded because a proposal that quietly restated a wrong number would make the next author trust it.

Two constraints shape everything that follows.

**First, OpenSpec has exactly one root per invocation.** A change resolves against the nearest `openspec/` directory and its deltas can only name capabilities in that root. There is no cross-root delta, no federation, and no supported way to express "remove this capability here and add it there" as one change. Verified empirically: with `subx-core/openspec/` populated, `openspec list --specs` and `openspec validate --all` run at the `subx-cli` root enumerate only `subx-cli`'s capabilities, and the same commands run from inside `subx-core/` enumerate only `subx-core`'s. That isolation is what makes two trees viable at all; it is also what forces the work into two changes.

**Second, `openspec archive` cannot perform either half of this move.** A delta that removes every requirement of a capability rebuilds a spec with an empty `## Requirements` section, which fails validation, and the archive is whole-change atomic — it changes nothing at all, including the deltas that would have applied cleanly. On the receiving side archive does work, but it writes a placeholder H1 and a `TBD` Purpose that both pass `--strict`. Every behaviour asserted in this document was reproduced against `openspec` 1.6.0 in a scratch copy of this tree before it was written down; Decision 2 carries the transcripts.

The result is a change that is small in bytes and unusual in shape: nothing compiles, nothing runs, and almost all the risk is in the two archive steps and in whether the classification it acts on is correct.

## Goals / Non-Goals

**Goals:**

- Verify SDR §9's CORE list against the specs rather than adopting it, and correct it in public where it is wrong.
- Stand OpenSpec up inside `subx-core/` with the smallest artifact set that works, and justify what is deliberately not created.
- Move the verified-core capabilities into `subx-core` with their text unchanged, their citations correct against the new root, and their history intact.
- Author `subx-core`'s first change, `import-core-specs`, in enough detail that its implementer writes the right thirteen files without re-deriving this analysis.
- Write down the ownership rule, so that C2b and every later proposal have something to apply instead of a judgement call.
- Leave both trees passing `openspec validate --strict` and free of dangling cross-repository references, including the one this change itself creates.

**Non-Goals:**

- Splitting any capability owned on both sides. That is C2b, and the three capabilities this change demotes into its list (`component-factory`, `parallel-processing`) or leaves for it (the other nine MIXED) are handed over with the evidence, not half-done.
- Changing the meaning of any requirement. No requirement is added, deleted, weakened, strengthened, merged or renumbered. The union of the two trees after this change is textually identical to `subx-cli`'s tree before it, apart from the fourteen citation edits enumerated in the deltas and the one reference qualification in `configuration-management`.
- Moving `crate-topology` or `cross-crate-testing`. Decision 9.
- Editing any source file, manifest, workflow, script or test in either repository.
- Rewriting `AGENTS.md` for `subx-core`. C3 owns that file; this change writes the OpenSpec paragraphs into `subx-cli`'s `AGENTS.md` and hands C3 the core text.
- Implementing the drift check as running code. `spec-governance` specifies it; C3 places it, because C3 is already rewriting the scripts it would live in.
- Registering either tree with `openspec store`, or adding a `references:` key to either `config.yaml`. Decision 12.

## Decisions

### Decision 1: SDR §9 is re-derived from the specs, and it is wrong in three places

SDR §6 was re-derived from the code while B3 was authored and turned out to be wrong in six places, because it had been built from greps rather than from reading. SDR §9 came from the same kind of survey and gets the same treatment. The test applied to each of the fourteen: **read every requirement, and ask which repository's files must change to satisfy it.** Subject matter, capability title and Purpose wording are not evidence.

Eleven of the fourteen survive unchanged: `ai-provider-integration`, `archive-extraction`, `async-runtime-safety`, `file-operation-safety`, `file-organization`, `input-size-guards`, `language-detection`, `local-llm-provider`, `media-discovery`, `subtitle-parser-hardening`, `subtitle-styling`, `vad-speech-detection` — that is twelve; every path citation in all twelve names `src/core/**`, `src/services/**` or `src/config/**`, plus four `tests/` citations and two `scripts/quality_check.sh` citations that Decision 7 handles.

**`component-factory` is not wholesale core.** Its requirement *Commands Consume Services via Dependency Injection* opens "Command entry points under `src/commands/` SHALL receive a `&dyn ConfigService` …" and its two scenarios cite `src/commands/match_command.rs:176-213`, `src/commands/sync_command.rs:168-173` and `src/commands/convert_command.rs:203-205`. `src/commands/` stays in `subx-cli` permanently — SDR D7 records that the GUI has zero references to `subx_cli::commands` and that moving it would drag `clap` into core for no consumer. Its `## Purpose` names the same three files. This is exactly SDR §9's own CLI-seam family (a): a command-interface requirement inside a capability whose remaining six requirements are core. The capability is MIXED and belongs to C2b.

**`parallel-processing` is not wholesale core either.** Two of its ten requirements fail the test:

- *Aggregated Result Reporting* — "SHALL aggregate outcomes of all tasks into success, failure, and partial categories and SHALL report the counts to the user after execution", with the scenario "the command SHALL display a summary". The aggregation loop and the four `println!` calls that display it are at `src/commands/match_command.rs:739-760`, not in `src/core/parallel/`.
- *Progress Reporting Opt-Out* — "when the flag is false, the progress indicator SHALL be hidden", with the scenario "the progress bar SHALL have its draw target set to hidden". The implementation is `src/commands/match_command.rs:729-737`, and `ProgressDrawTarget` is `indicatif`, a permanently CLI-only dependency (SDR D8, §4). `grep -rn 'indicatif\|ProgressDrawTarget' src/` returns hits only under `src/cli/` and `src/commands/`.

Both are SDR §9's CLI-seam family (c). The other eight requirements — the scheduler, bounded concurrency, batch submission, queue overflow, priority ordering, non-blocking I/O, active-task accounting, UUIDv7 identifiers — are `src/core/parallel/**` and nothing else. The capability is MIXED and belongs to C2b.

**`core-reporting` is wholesale core, and SDR §9 could not have known.** A1 created it after 2026-08-26. Its five requirements — *Transport-Agnostic Reporter Seam*, *Reporter Is Send and Sync*, *Core-Owned AI Usage Payload*, *Reporter Attachment Preserves Constructor Signatures*, *No Core or Service Module References the CLI Layer* — contain zero citations of `src/cli/`, `src/commands/` or `src/main.rs`. Their subject is `subx_core::core::report`. Leaving it behind would put the specification of a `subx-core` module in the `subx-cli` tree on the same day `spec-governance` forbids exactly that, so it joins the list.

**Why demote rather than carve.** It would be possible for this change to lift the three CLI requirements out and move the remaining fifteen. It is rejected: carving a capability in half *is* the work C2b exists to do, C2b already has a stated method for it (SDR §9's three CLI-seam families), and doing two of the eleven here would leave C2b's list at nine with two done differently by a different author. The demotion costs C2b nothing — it gains two capabilities whose seams are already located, cited and named in this document.

**The corrected census.** Thirteen capabilities, 93 requirements, 239 scenarios move. Nineteen capabilities stay, plus `spec-governance` arriving, for twenty. None of the thirteen is touched by A0, A1, A2, B1, B2, B3, B4 or C1 — A0 and C1 touch `supply-chain-hardening`; A1 touches `machine-readable-output` and `progress-reporting` and creates `core-reporting`; A2 touches `error-handling`, `input-path-handling` and `timeline-sync`; B1, B2 and C1 touch `crate-topology`; B3 and B4 touch `cross-crate-testing`; C1 touches `release-distribution`. The counts are therefore stable from today through to the moment this change runs, which is why they can be stated as fixed numbers rather than estimates.

### Decision 2: `openspec archive` performs neither half of this move; the procedure is `--skip-specs` plus an enumerated set of hand steps

The brief asks how `openspec archive` behaves for a change whose specs land in a different repository. The answer is that it has no concept of the other repository at all, and the interesting behaviour is what it does with each half locally. All three findings below were reproduced against `openspec` 1.6.0 in a scratch copy of this tree.

**(a) The removing half refuses, atomically.** A change whose delta removes every requirement of `async-runtime-safety` validates cleanly (`Change 'try-removal' is valid`) and then fails to archive:

```
{ "archive": null,
  "status": [{ "severity": "error", "code": "archive_spec_validation_failed",
    "message": "Rebuilt spec for 'async-runtime-safety' failed validation. No files were changed." }] }
```

The cause is that the rebuilt spec has an empty `## Requirements` section, and `openspec validate <spec> --strict` reports `Spec must have at least one requirement`. "No files were changed" is literal: the archive is whole-change atomic, so a mixed change carrying both a REMOVED-everything delta and an ADDED delta for a brand-new capability applies neither. That was verified directly — a scratch change with a `spec-governance` ADDED delta alongside the `async-runtime-safety` REMOVED delta left `openspec/specs/spec-governance/` uncreated.

**(b) `--skip-specs` succeeds and does exactly nothing to the specs.** `openspec archive <change> -y --skip-specs` reports `"specsUpdated": false`, moves the change directory to `openspec/changes/archive/<date>-<name>/`, and leaves all 32 capability directories in place. So `--skip-specs` is the correct flag, and the cost of using it is that **every** delta in the change must then be applied by hand — the thirteen removals, the `spec-governance` addition, and the `configuration-management` modification alike.

**(c) The receiving half succeeds, and produces a spec that is conformant and wrong.** Archiving an ADDED-only delta for a capability that does not yet exist writes:

```markdown
# async-runtime-safety Specification

## Purpose
TBD - created by archiving change import-core-specs. Update Purpose after archive.
## Requirements
### Requirement: Non-blocking file I/O in all async functions
```

Three defects, none of which any check will catch: the H1 is `# <name> Specification` rather than the Title Case name SDR §10 requires; the Purpose is a placeholder; and there are no blank lines after the two H2s. `openspec validate --specs --strict` passes on all of it — the placeholder is 79 characters, above the 50-character "Purpose too brief" warning threshold. This is why `import-core-specs`'s `tasks.md` carries an explicit post-archive phase rather than trusting the tool, and why `spec-governance` states the replacement as an obligation.

**The resulting procedure**, in the order the tasks perform it:

1. `subx-core`: author `import-core-specs`, `openspec validate import-core-specs --strict`, `openspec archive import-core-specs -y` (no flag — it succeeds), then hand-repair thirteen H1s and thirteen Purposes, then `openspec validate --specs --strict`. Commit.
2. `subx-cli`: `openspec archive move-core-capabilities-to-subx-core -y --skip-specs`, then `git rm -r` the thirteen capability directories, hand-write `openspec/specs/spec-governance/spec.md` from this change's ADDED delta with an H1 and a Purpose added, hand-apply the `configuration-management` MODIFIED requirement into its main spec, then `openspec validate --specs --strict`. Commit **together with the moved submodule gitlink** so the two trees are consistent at every commit of `subx-cli`.

The ordering is not arbitrary: the receiving side goes first so that at no point does a requirement exist in neither tree. Between the two commits it exists in both, which is a duplication that lasts minutes and is visible in one `git log`; the alternative window, in which 93 requirements exist nowhere, is not recoverable by reading either repository.

### Decision 3: `import-core-specs` is a genuine OpenSpec change, and its `## Why` says so without pretending it adds behaviour

The honest objection is that OpenSpec changes exist to propose behaviour, and this one proposes none: it adds thirteen capabilities whose every requirement is already satisfied by code that already exists and already passes its tests. Calling it a change looks like ceremony.

It is not, for three reasons.

**It is the only mechanism there is.** OpenSpec has no supported command that writes a capability into `openspec/specs/` other than archiving a change that adds it. Hand-writing thirteen files into `subx-core/openspec/specs/` and committing them would produce the same tree and no record of where it came from — and `openspec/changes/archive/` in `subx-core` would be empty, so the first thing a reader of that repository learns about its specification is nothing.

**The provenance has to be in the receiving repository.** `subx-cli` keeps the archived record of every change that authored these 93 requirements, going back to `2026-04-25-archive-input-extraction`. A standalone clone of `subx-core` — which is how crates.io consumers, `docs.rs` and the Tauri GUI see the crate — has none of it. `import-core-specs` is the one artifact that tells that reader the requirements were migrated from `subx-cli` on a date, by a named change, as part of a crate split, without alteration. That is provenance, and provenance is the thing a spec-driven repository cannot regenerate.

**It carries real decisions.** Thirteen Purposes are rewritten, fourteen citations are edited, three line ranges are re-verified, and the placeholder H1/Purpose that `openspec archive` generates are replaced. Those are choices with a right and a wrong answer, and a change is where they are recorded and reviewed.

Its `## Why` therefore says, in substance: *this change adds no behaviour and satisfies no new requirement. It exists because `subx-core` is now a separately clonable crate whose specification lives in another repository, because OpenSpec's only mechanism for populating `openspec/specs/` is an archived change, and because the record of where these requirements came from has to be readable from this repository. Every requirement here was written elsewhere and is reproduced verbatim; the only editorial changes are the thirteen `## Purpose` paragraphs and the fourteen citations enumerated in the sending change's `**Migration**` notes.* It names `move-core-capabilities-to-subx-core` in `subx-cli` as its other half, and it states plainly that it is not independently reviewable.

### Decision 4: `import-core-specs`'s artifact set, file by file

A change with thirteen ADDED capability files and no code is unusual enough that leaving its shape to the implementer would produce something wrong. The complete set:

| Path (relative to `subx-core/`) | Content |
|---|---|
| `openspec/changes/import-core-specs/.openspec.yaml` | Exactly two lines: `schema: spec-driven` and `created: <date of authoring>`. No other keys — this matches every archived change in `subx-cli`. |
| `openspec/changes/import-core-specs/proposal.md` | The four H2s of SDR §10. `## Why` per Decision 3. `## What Changes` listing the thirteen capabilities with their requirement and scenario counts, the thirteen Purpose rewrites, and the fourteen citation edits. `## Capabilities` with all thirteen under `### New Capabilities` and the literal `_None._` under `### Modified Capabilities`. `## Impact` with `**Code:**`, `**Tests:**`, `**APIs:**` and `**Dependencies:**` all `None.` and `**Documentation:**` naming `subx-core/CHANGELOG.md`. |
| `openspec/changes/import-core-specs/design.md` | `## Context`, `## Goals / Non-Goals`, `## Decisions`, `## Risks / Trade-offs`. Four decisions, no more: (1) the text is reproduced verbatim and the diff against the source is expected to be empty apart from the enumerated edits; (2) the thirteen Purposes and their exact wording; (3) the post-archive H1/Purpose repair and why the tool's output cannot be trusted; (4) the unrelated-histories merge, or its fallback, per Decision 8. It SHALL NOT re-argue the ownership classification — that argument lives here, and duplicating it is how two documents start disagreeing. It cites this file instead. |
| `openspec/changes/import-core-specs/specs/<capability>/spec.md` × 13 | Each starts with `## ADDED Requirements` and no H1 and no `## Purpose`, then carries every `### Requirement:` and `#### Scenario:` block of the source capability **verbatim**, in the source order, with only the edits named in the sending change's `**Migration**` notes applied. |
| `openspec/changes/import-core-specs/tasks.md` | Numbered H2 phases per SDR §10, ending with a documentation phase and the quality-gate phase. The gate is `subx-core`'s own — C1 Decision 4 gives that repository a smaller quality script of its own — and its meaning here is Decision 11's. |

The thirteen delta files are produced mechanically, not retyped: for each capability, take `subx-cli/openspec/specs/<cap>/spec.md`, drop everything up to and including the `## Requirements` line, and prepend `## ADDED Requirements`. `core-reporting` is the exception — it is sourced from A1's archived delta at `openspec/changes/archive/<date>-decouple-core-from-terminal-output/specs/core-reporting/spec.md`, whose body is already in delta shape and needs only its `## ADDED Requirements` header kept.

### Decision 5: the thirteen `## Purpose` paragraphs are written by hand, and eight of them are the pre-split text unchanged

`openspec archive` writes a `TBD` placeholder, so each Purpose is authored. Eight of the thirteen need no editorial change at all, because their pre-split Purpose already names only core paths: `ai-provider-integration`, `archive-extraction`, `file-organization`, `language-detection`, `local-llm-provider`, `media-discovery`, `subtitle-styling`, `vad-speech-detection`. Four have no path citation in their Purpose at all and are likewise carried over as-is: `async-runtime-safety`, `file-operation-safety`, `input-size-guards` and `subtitle-parser-hardening`.

That leaves `core-reporting`, which has never had a main-spec Purpose because A1's delta correctly omits one. It is written new. Proposed text, to be used unless review improves it:

> Provide a transport-agnostic reporting sink through which every module under `src/core/` and `src/services/` emits human-oriented status, diagnostic, warning, progress and AI-usage output, so that the library never writes to a terminal and never learns that a machine-readable output mode exists. The consumer supplies the sink; the default is silent. Implemented in `src/core/report/mod.rs` (`Reporter`, `NoopReporter`, `AiUsage`, `ProgressEvent`, `noop()`) and attached to `MatchEngine`, `TranslationEngine`, `SyncEngine`, `FileManager`, `WorkerPool` and `ComponentFactory` through `with_reporter`.

The rule the Purposes follow is Decision 7's: a Purpose ends with "Implemented in `…`" naming paths that exist in the owning repository, and where the pre-split Purpose named the other side's role it says so in prose without a path.

### Decision 6: `openspec init --tools none`, and `config.yaml` carries no `context:` block

`openspec init` in `subx-core/` creates `openspec/config.yaml`, `openspec/specs/` and `openspec/changes/archive/`. Two sub-decisions.

**`--tools none`, not `--tools claude`.** `--tools claude` additionally writes eight `.claude/skills/openspec-*/SKILL.md` files. `subx-cli` has none of them — its `.claude/` holds only `CLAUDE.md`, `settings.json` and `settings.local.json` — because the OpenSpec skills are available to the agent already. Writing eight duplicated skill files into a submodule would add a per-repository copy of a shared tool's instructions, which is the class of duplication SDR §5 exists to enumerate and bound, and they would drift silently because nothing compares them.

**No `context:` block.** The brief asks the question because the parent's is entirely commented out and a fresh submodule is the obvious place to want one. It is declined:

- The context an agent needs when working inside `subx-core` is the project's conventions, and those live in `AGENTS.md`, which C3 authors for that repository. A `context:` block would be a second home for the same facts with no mechanism keeping them in agreement.
- Leaving both files as the unmodified `openspec init` template makes them `diff`-able. C1 Decision 9 established that discipline for the two CI workflows: keep the files identical so that `diff` is the review tool and a divergence is a spec violation rather than a judgement call. The same reasoning applies here at zero cost.
- `spec-governance` makes it normative — the two `config.yaml` files SHALL be byte-identical, and a future `rules:` block SHALL land in both in the same change — so this is not merely today's preference.

**One implementation detail `init` does not handle.** `openspec init` creates `openspec/changes/archive/` as an empty directory, and git does not track empty directories. `subx-core/openspec/specs/` and `openspec/changes/` will hold real files immediately, but `changes/archive/` will not until the first change is archived — and `import-core-specs` is archived in this very change, so it does. The task list still verifies the directory exists in a fresh clone rather than assuming it.

### Decision 7: the citation rewrite rule is identity for core paths, qualification for everything else — and the reason is B2's

The brief asks for the `## Purpose` path-rewrite rule and how it stays true after B2 moved the sources. The answer is smaller than expected, and it is a consequence of a decision B2 already made.

B2's Decision 1 chose `git filter-repo --path src/core --path src/services --path src/config --path src/error.rs` precisely because **no path rewriting was required**: those paths are already the paths they should have under `subx-core`'s root. `src/core/matcher/engine.rs` in `subx-cli` became `src/core/matcher/engine.rs` in `subx-core`. Therefore:

> **A citation of `src/core/…`, `src/services/…`, `src/config/…` or `src/error.rs` in a moved spec is already correct inside `subx-core` and SHALL NOT be edited.**

That covers the overwhelming majority: every path citation in twelve of the thirteen capabilities, and `core-reporting`'s `src/core/report/mod.rs` and `src/core/parallel/worker.rs`. Four classes remain, and all fourteen instances are enumerated as `**Migration**` notes in the REMOVED deltas so the implementer does not have to find them again.

**(a) A citation naming a file that stays in `subx-cli`** is rewritten to the qualified form `subx-cli:<path>`. There is exactly one: `file-organization`'s *AutoRename Sequential Suffixes* cites `tests/match_duplicate_rename_conflict_tests.rs`, which imports `subx_cli::cli::MatchArgs` and `subx_cli::commands::match_command` and is therefore CLI-bound under B3's ownership test.

**(b) A citation naming a file that moved to `subx-core` under a different name.** There are none among the thirteen. `vad-speech-detection` cites `tests/vad_audio_processor_tests.rs`, `tests/vad_detector_tests.rs` and `tests/vad_integration_tests.rs`; B3 preserves basenames when it flattens into `subx-core/tests/`, and none of the three is among the three files B4 renames (`input_handler_tests.rs`, `parallel/integration_tests.rs` → `parallel_integration_tests.rs`, `sync/integration_tests.rs` → `sync_integration_tests.rs`). The rule is stated anyway, because C2b will need it.

**(c) A citation naming a per-repository artefact that both repositories now have.** `subtitle-parser-hardening` cites `scripts/quality_check.sh` twice and `Cargo.lock` once; those become `subx-core`'s own — C1 Decision 4 gives that repository a smaller quality script, and B1 Decision 5b commits its own lockfile. Its *Optional cargo-fuzz harness lives outside default workspace* requirement is the interesting one: it says the harness lives in "a sibling `fuzz/` directory ... excluded from the main `Cargo.toml` workspace members", and after the split the parsers are in `subx-core` while the *main* workspace root is `subx-cli/Cargo.toml`. The requirement is restated to name `subx-core/fuzz/` and to require exclusion from both roots. Nothing is lost: no `fuzz/` directory exists today, and the requirement is conditional on one being added.

**(d) A `file.rs:line-line` range crossing the boundary is re-verified, not carried.** There are exactly four, all in `language-detection`: `src/core/language.rs:58-75`, `src/core/language.rs:77-81`, `src/core/matcher/mod.rs:343-344` and `src/core/matcher/engine.rs:766-782`. The paths are correct by the identity rule; the ranges are not guaranteed, because A1 edited `engine.rs` in six places. `language.rs` and `matcher/mod.rs` were untouched by A0–C1 and are expected to be exact, but all four are checked rather than assumed — a spec citing a line range that no longer covers the construct is worse than one citing no range at all.

**How the rule stays true.** It stays true because it is not a mapping table but a statement about roots: an unqualified path is relative to the repository owning the spec, and a path naming the other repository carries its name. `spec-governance` states it that way, so a later relocation inside either repository invalidates the specific line ranges but never the rule.

### Decision 8: history for the moved spec files is preserved by B2's mechanism, and B2's own fallback is available here for a reason B2 did not have

B2 faced the identical problem for 36,687 LOC and answered it in its Decision 1: a cross-repository move cannot be a `git mv`, because `subx-core/` is a gitlink and `subx-cli`'s index holds one entry for it. Its chosen mechanism was `git filter-repo` on a scratch clone, fetched into `subx-core` and merged with `--allow-unrelated-histories`. The same mechanism applies here without modification, and — as in B2 — **no path rewriting is needed**, because `openspec/specs/<cap>/` is already the path those directories should have under `subx-core`'s root:

```bash
git clone https://github.com/jim60105/subx-cli /tmp/spec-history
cd /tmp/spec-history
git filter-repo \
  --path openspec/specs/ai-provider-integration --path openspec/specs/archive-extraction \
  --path openspec/specs/async-runtime-safety     --path openspec/specs/core-reporting \
  --path openspec/specs/file-operation-safety    --path openspec/specs/file-organization \
  --path openspec/specs/input-size-guards        --path openspec/specs/language-detection \
  --path openspec/specs/local-llm-provider       --path openspec/specs/media-discovery \
  --path openspec/specs/subtitle-parser-hardening --path openspec/specs/subtitle-styling \
  --path openspec/specs/vad-speech-detection
```

then, in `subx-core`, `git remote add spec-history /tmp/spec-history && git fetch spec-history && git merge --allow-unrelated-histories spec-history/main`. The merge is conflict-free by construction: the filtered tree contains only `openspec/specs/**`, and `subx-core`'s `openspec/specs/` is empty until this change. `subx-core` already carries one unrelated-histories merge from B2, so a second root is not a new kind of oddity in its `git log --graph`; it is worth one more sentence in the note B2 added to `subx-core/README.md`.

**Why reuse rather than re-decide.** Two mechanisms for one problem in one series is how the second one gets applied to the wrong case. B2 also recorded a fallback — plain copy on `main`, filtered history pushed to an orphan branch `pre-split-history` for archaeology — explicitly marked "strictly worse", to be reached for only if `filter-repo` cannot be installed.

**Where this case honestly differs, and what follows.** For `engine.rs`, `git blame` is the only way to answer "why is this line here"; losing it is a permanent loss. For a spec file it is not: `subx-cli/openspec/changes/archive/**` holds the complete authoring record of all 93 requirements — thirteen archived changes with their proposals, designs and deltas — and that record **stays in `subx-cli`** and is not moved by this change. So a reader who cannot `git blame` the moved spec can still find the change that wrote any requirement, in more detail than blame would give. The consequence is not that the filtered history is skipped; it is that **B2's fallback is genuinely acceptable here where it was not acceptable in B2**, so an implementer blocked on `filter-repo` availability may take it without escalating, provided the orphan branch is pushed and the CHANGELOG says so. The task list carries both paths and the verification (`git log --oneline -- openspec/specs/vad-speech-detection/spec.md` inside `subx-core` returns more than one commit) applies to the primary path only.

### Decision 9: `spec-governance` is a new capability; `crate-topology` and `cross-crate-testing` stay in `subx-cli` and are not extended

C1's Decision 14 set the bar for adding a capability rather than extending `crate-topology`: a new capability is justified only when the alternative would split one readable contract across two documents. Applying that bar honestly here gives two answers, in opposite directions.

**`crate-topology` is the wrong home for the ownership rules, and a new capability is right.** `crate-topology` says what the two repositories *are*: the submodule mount, the workspace shape, the manifest prohibitions, module ownership, the re-export surface, the dependency allocation, the per-repository configuration files, submodule checkout in CI. Every one of those is a statement about build and packaging structure, enforceable by Cargo or by a script. What this change adds is a statement about **the specification process**: how a requirement's owner is decided, what is and is not evidence for that decision, how a change spanning two repositories is authored and archived, and what `openspec archive` does when a delta empties a spec. Those rules bind proposal authors, not builds. Folding them into `crate-topology` would put "`subx-core/Cargo.toml` SHALL NOT contain a `[workspace]` table" and "a scenario reaching core behaviour through `subx config set` is not ownership evidence" under one heading, and the reader looking for either would have to read past the other. That is the failure C1's Decision 14 named, in mirror image: not one contract split across two documents, but two contracts fused into one.

The size test confirms it rather than driving it: eight requirements and 24 scenarios is a capability, not an appendix.

**`crate-topology` and `cross-crate-testing` nevertheless stay where they are, and are not touched.** Each is a statement about both repositories at once. Splitting `crate-topology` would separate "every CI job checks the submodule out" from "the core repository has its own CI", which are one contract; splitting `cross-crate-testing` would separate "test ownership follows the crate under test" from "shared helpers live in `subx-core` behind a `test-support` feature", likewise. Mirroring either into `subx-core` would create two copies with nothing keeping them in agreement. They stay in the parent, because the parent is where the workspace root, `.gitmodules`, the pointer and the release pipeline are, and a rule about the pair is enforceable from there. `spec-governance` joins them and states the rule for all three. The consequence — that a standalone clone of `subx-core` does not carry these rules — is accepted and written down: a standalone clone is a build target and a crates.io source, not where the two-repository contract is decided.

### Decision 10: `configuration-management` gets a `## MODIFIED Requirements` delta, a fifteenth file the brief did not name

Moving `ai-provider-integration` breaks a reference. `configuration-management`'s *Local Provider Validation Rules* says the HTTPS-required rule "documented for hosted providers in the `ai-provider-integration` capability SHALL NOT apply to `local`" — and after this change, a reader holding only `subx-cli` finds no such capability.

Three options were weighed.

**Leave it for C2b.** Defensible: C2a and C2b are strictly sequential, C2b must restate that requirement anyway when it splits `configuration-management`, and the window is one change long. Rejected because this change introduces the `spec-governance` requirement *Cross-Repository Capability References Are Qualified and Re-Qualified When They Move*, which says in terms that the moving change owns the repair. Landing a rule and a violation of it in the same commit is the specific failure C1 Decision 14 refused for `.llvm-cov.toml`.

**Edit the main spec directly.** Rejected outright: `spec-governance` forbids repairing a reference outside the delta mechanism, for the same reason the delta mechanism exists.

**Restate the one requirement in a MODIFIED delta.** Chosen. Thirty-five lines, one word changed in the prose plus one added scenario asserting that the qualification reads correctly to someone holding only `subx-cli`. C1 spent its Decision 14 on exactly this shape of overrun and concluded it was unavoidable and small; the same conclusion holds, and it brings the delta count to fifteen.

The symmetric repairs in the other direction — `local-llm-provider`'s three references to `configuration-management` and `error-handling` — are **not** `subx-cli` deltas. They are edits to the text arriving in `subx-core`, and they are carried in the `**Migration**` notes so `import-core-specs` applies them. They are qualified to `subx-cli` here, and `spec-governance` obliges C2b to re-qualify them to `subx-core` when it sends `normalize_ai_provider`'s and the URL-redaction requirement's halves across.

### Decision 11: the REMOVED deltas name all 93 requirements with a one-line reason, and only fourteen carry a `**Migration**` note

The archive uses `## REMOVED Requirements` exactly twice, both in `2026-04-27-harden-file-ids-and-json-output`, and both do the same thing: a `### Requirement: <exact title>` header, a `**Reason**:` paragraph, and a `**Migration**:` paragraph. Neither restates the removed requirement's body. That convention is followed here, with one deliberate narrowing.

**All 93 titles are named.** The alternative — one synthetic entry per capability reading "all requirements" — was rejected. The titles are the audit trail: they are what makes it mechanically checkable that the receiving repository got exactly what left, by comparing the set of `### Requirement:` titles in this change's deltas against the set in `subx-core/openspec/specs/`. The task list performs that comparison and requires the sets to be equal, which is only possible because the titles are written out. At 426 lines across thirteen files, the artifact is well within reason; restating the 93 bodies as well — roughly 1,930 lines, duplicated verbatim in `import-core-specs` — would not be.

**The reason is one line and identical within a file.** In the archive's two instances each removal had a distinct rationale, because each was a distinct decision. Here one decision is applied 93 times, and writing 93 differently-phrased versions of it would imply distinctions that do not exist.

**`**Migration**` appears only where something is actually edited on arrival.** Fourteen requirements carry one, corresponding exactly to Decision 7's four classes plus the three cross-capability references and the two `core-reporting` readings. The other 79 have nothing to migrate — their text is byte-identical on both sides — and a boilerplate migration note on each would bury the fourteen that matter. `openspec validate --strict` accepts a removal entry with `**Reason**` alone; that was verified.

### Decision 12: the two roots are not linked through `openspec store` or a `references:` key

`openspec doctor` reports a "References" section, and a committed `references:` key in `config.yaml` is accepted:

```yaml
references:
  - id: subx-core
    path: ../subx-core
```

Declared this way, `openspec doctor --json` returns `reference_unresolved` — *"Referenced store 'subx-core' is not registered on this machine"* — with the fix being `openspec store register <path> --id subx-core`. That is the whole mechanism: a `references:` entry names a **store**, and a store is a per-machine registration. Registering `subx-core` would additionally write `subx-core/.openspec-store/store.yaml` into the submodule.

Declined on all three counts. The registration is not committable, so every developer and every CI runner that has not run the command sees a `doctor` warning — a warning everyone must ignore is a warning that will be ignored when it matters. The `store.yaml` file is a second per-repository artefact for SDR §5's list to carry. And the thing it buys — cross-tree resolution — is not needed, because the two trees deliberately do not reference each other's requirements; they reference each other's *capabilities*, in prose, which Decision 7's qualified forms already make unambiguous to a human and which no tool needs to resolve.

The isolation is relied on instead, and it is verified rather than assumed: with `subx-core/openspec/specs/` populated, `openspec list --specs` at the `subx-cli` root listed 28 capabilities and not the submodule's, and the same command inside `subx-core/` listed only the submodule's. `spec-governance` records that isolation as something not to be defeated, and replaces the tooling link with a drift check over capability names.

## Risks / Trade-offs

- **Risk: the classification is wrong again, and a CLI requirement lands in `subx-core` where nobody will find it.** → Mitigation: every requirement of all fourteen SDR §9 candidates was read, and the three corrections in Decision 1 carry file-and-line evidence that was checked against the source (`src/commands/match_command.rs:725-760`, the `indicatif` grep, `src/commands/{match,sync,convert}_command.rs`). The task list re-runs the two mechanical screens — a grep for `src/cli/`, `src/commands/`, `src/main.rs` and terminal vocabulary across the thirteen, and a grep for the five CLI-only crate names — before anything moves, and requires both to be clean.
- **Risk: `openspec archive` is run without `--skip-specs`, fails, and the failure is read as a broken delta.** → Mitigation: the failure is atomic and changes no file, so the cost is confusion rather than damage. It is pre-empted in three places — `spec-governance` states the expected message verbatim, `design.md` Decision 2 reproduces the JSON, and the task carrying the command names the flag and the reason in the same line.
- **Risk: `--skip-specs` is used and the hand steps are partly forgotten, leaving `spec-governance` unwritten or a capability directory undeleted.** → Mitigation: this is the single most likely way the change goes wrong, because the tool reports success. Every hand step is its own checkbox, and the verification phase asserts the end state independently of how it was reached: exactly 20 capability directories in `subx-cli`, exactly 13 in `subx-core`, the two title sets disjoint, and `openspec validate --specs --strict` green in both roots.
- **Risk: the thirteen generated Purposes are left as `TBD` because `validate --strict` passes on them.** → Mitigation: verified and stated as a defect the tool will not catch. The task list greps both trees for the literal string `TBD - created by archiving` and requires zero hits, which is a check the tool does not provide and this change adds.
- **Risk: `git filter-repo` is unavailable and the history is silently dropped.** → Mitigation: Decision 8 records the fallback explicitly, states the condition under which it is acceptable here (the archived change record stays in `subx-cli` and is richer than blame), and requires the CHANGELOG to say which path was taken. Silence is the failure mode being designed out.
- **Risk: the two commits land separately and `subx-cli` briefly points at a `subx-core` commit without the specs.** → Mitigation: Decision 2 fixes the order — receiving side first — so the transient state is duplication rather than absence, and the `subx-cli` commit carries the moved gitlink, per B1's *Submodule pointer accompanies a dependent change*.
- **Risk: C2b restates `configuration-management`'s *Local Provider Validation Rules* from the pre-C2a text and silently reverts the qualification.** → Mitigation: it is called out in Open Questions and in the CHANGELOG entry, and `spec-governance`'s re-qualification requirement makes the revert a spec violation rather than an oversight. The added scenario in the MODIFIED delta also fails to make sense if the qualification is dropped, which makes the revert visible in review.
- **Trade-off: two capabilities are demoted to C2b rather than carved here, so C2b grows from eleven to thirteen.** → Accepted (Decision 1). Both seams are already located, cited and named, so C2b gains work it does not have to find; splitting two of eleven here by a different author and a different method is the worse outcome.
- **Trade-off: a fifteenth delta file beyond the brief.** → Accepted (Decision 10). The alternative is a rule and a violation of it in the same commit.
- **Trade-off: `subx-core` does not carry `crate-topology`, `cross-crate-testing` or `spec-governance`, so a standalone clone has an incomplete picture.** → Accepted (Decision 9). Those three describe the pair; a clone of one half is not the audience for them, and mirroring them would create copies with nothing keeping them in agreement.
- **Trade-off: the 93 removal entries are 426 lines of near-boilerplate.** → Accepted (Decision 11). They are the mechanically checkable audit trail that the receiving tree got exactly what left, and the check that uses them is in the task list.

## Sizing

Estimated at **~9.25 h** — one long workday, comparable to B3's measured 9 h and C1's 9.5 h, and it fits without a seam.

| Work | Estimate |
|---|---|
| Baseline: verify the classification screens, count the tree, confirm A1–C1 landed | 0.75 h |
| `openspec init` in `subx-core`, config verification, `.gitkeep`/fresh-clone check | 0.5 h |
| Filtered spec history (`filter-repo`, fetch, unrelated-histories merge, verify) | 0.5 h |
| `import-core-specs`: `.openspec.yaml`, `proposal.md`, `design.md`, `tasks.md` | 1.5 h |
| The thirteen ADDED delta specs: mechanical extraction, 14 citation edits, 13 Purposes | 2.0 h |
| Core-side archive, thirteen H1/Purpose repairs, `validate --specs --strict` | 1.0 h |
| CLI-side archive with `--skip-specs`, thirteen `git rm -r`, `spec-governance` main spec, `configuration-management` edit | 1.0 h |
| Verification: title-set equality, disjointness, `TBD` grep, both roots green, fresh-clone check | 0.75 h |
| Documentation: `AGENTS.md`, two CHANGELOGs, the core `AGENTS.md` handoff to C3 | 0.75 h |
| Quality gate | 0.5 h |

The two items that could overrun are the thirteen ADDED delta specs — if the mechanical extraction is done by hand rather than scripted, 2 h becomes 4 — and the `filter-repo` step if the tool is not installed, which is why Decision 8 gives it a named fallback rather than leaving it to be improvised. **Neither is a seam**: if the extraction overruns, the residue is one more hour of mechanical work in the same files, not a separable body of work with its own verification. Two sibling changes in this series named seams and were right to; this one has nothing to hand forward that would not immediately have to be handed back, so it stays whole.

## Migration Plan

Each step leaves both repositories in a state that either validates or fails loudly, and the one window in which a requirement exists twice is deliberate and short.

1. **Baseline.** Confirm C1 has landed and the submodule is initialised. Run `openspec validate --all` at the `subx-cli` root and record it green. Re-run the two classification screens and confirm the thirteen are clean and the two demotions still hold.
2. **Prepare the destination.** `openspec init --tools none` in `subx-core/`, verify `config.yaml` is byte-identical to the parent's, and produce the filtered spec history into a scratch clone — before either worktree is touched, because it derives from `subx-cli`'s pre-move HEAD.
3. **Author `import-core-specs` in `subx-core`.** Extract the thirteen delta specs mechanically, apply the fourteen citation edits, write the four artifacts. `openspec validate import-core-specs --strict` green before anything is archived.
4. **Land the receiving side.** Merge the filtered history, archive `import-core-specs`, repair thirteen H1s and thirteen Purposes, `openspec validate --specs --strict` green, `grep -r 'TBD - created by archiving'` returns nothing. Commit in `subx-core`. **From here the 93 requirements exist in both trees.**
5. **Land the sending side.** Archive with `--skip-specs`, `git rm -r` the thirteen directories, write `spec-governance`'s main spec, apply the `configuration-management` edit, `openspec validate --specs --strict` green. Commit in `subx-cli` **with the moved gitlink**. The duplication window closes here.
6. **Verify the pair.** Title-set equality between this change's REMOVED deltas and `subx-core`'s specs. Capability-name disjointness across the two trees. Fresh `git clone --recurse-submodules` into a scratch directory, then `openspec validate --all` at both roots.
7. **Document.** `AGENTS.md`, both CHANGELOGs, and the core `AGENTS.md` text handed to C3.

**Rollback** is `git revert` of the `subx-cli` commit — which restores the thirteen directories and the old gitlink in one step — plus, optionally, `git revert` of the `subx-core` commit. Nothing outside `openspec/` is affected, no code, test, manifest or workflow changes, and the union of the two trees is textually identical before and after, so a revert cannot lose a requirement. The one non-reversible artifact is `subx-core`'s history merge, which is additive and harmless if the specs are reverted on top of it.

## Open Questions

- **Where the drift check actually runs.** `spec-governance` requires a check that the two trees' capability names are disjoint except for a recorded split set. This change specifies it and does not implement it, because the natural home is the quality script and C3 is rewriting those. If C3 declines, the fallback is a step in `subx-cli`'s CI that runs only when `openspec/` or the submodule pointer changes. **Resolve during C3.**
- **Where the recorded split set lives once C2b creates thirteen split capabilities.** The obvious candidates are a table in `spec-governance`'s own spec — which would then need a `## MODIFIED Requirements` delta on every future split — or a small committed file the check reads. C2b creates the first thirteen entries and is the change that has to answer this. **Resolve during C2b.**
- **Whether `core-reporting`'s *No Core or Service Module References the CLI Layer* survives as a separate requirement.** B2's `crate-topology` requirement *`subx-core` Never References `subx-cli`* states a strictly broader invariant, enforced in the post-split topology, and stays in `subx-cli`. The moved copy is therefore partly redundant. It is carried over unchanged here, with a `**Migration**` note recording the overlap, because deleting a requirement is a decision this change has no mandate to make and `crate-topology` belongs to C1's lineage. **Resolve in a later change, or accept the redundancy deliberately.**
- **Whether C2b re-qualifies `local-llm-provider`'s three references, or leaves them pointing at `subx-cli`.** They are qualified to `subx-cli` here, which is correct at this commit. After C2b splits `configuration-management` and `error-handling`, the referenced halves — `normalize_ai_provider` and the URL-redaction rule — are both core-owned, so both should become `subx-core`. `spec-governance` obliges it; this note is here so C2b's author does not have to rediscover which three they are.
