## Why

By the time this change runs, the code has been split and the specifications have not. B2 moved `src/core/**`, `src/services/**`, `src/config/**` and `src/error.rs` into `subx-core`; B3 and B4 moved the tests that exercise them; C1 gave the core repository its own CI and its own release path. What is left in `subx-cli/openspec/specs/` is a tree that describes, in the present tense and with normative `SHALL`s, the behaviour of files that are no longer in the repository. `subx-core` — a separately clonable crate that the Tauri GUI at `../subx` will depend on directly — has no `openspec/` at all, and therefore no specification of its own behaviour.

That is not merely untidy. It has three concrete costs:

1. **The specification is unreachable from the crate it governs.** A contributor cloning `subx-core` standalone, or an agent working inside the submodule, resolves the nearest OpenSpec root and finds nothing. The requirements governing `subx_core::core::matcher`, `subx_core::services::vad` and `subx_core::core::report` are in a different repository, findable only by knowing to look for it.
2. **Every citation in those specs is wrong by one repository.** `vad-speech-detection`'s Purpose ends "Implemented in `src/services/vad/`", and `src/services/vad/` does not exist in `subx-cli` after B2. The path is not stale — it is correct, but against a root the reader does not have.
3. **There is no rule for what happens next.** SDR §9 partitions today's capabilities by hand. It says nothing about the capability someone adds next month, or about what to do when a core-owned capability grows a requirement that only the CLI can satisfy. Without a written rule, the second split is decided ad hoc, and the two trees begin to drift on the day they are created.

This change is series change **C2a**. It moves the capabilities that are entirely core-owned into `subx-core`, stands OpenSpec up inside the submodule, and writes down the rule by which every future assignment is made. It is strictly sequenced before **C2b** (`split-mixed-capabilities-across-repos`), which splits the capabilities owned on both sides: the two changes share the `openspec/specs/` tree and cannot run in parallel, and C2b needs both the `subx-core` OpenSpec root and the `spec-governance` capability that this change creates before it has anywhere to put its core halves.

It spans both repositories. Its `subx-core` half is a separate change, **`import-core-specs`**, authored in `subx-core/openspec/changes/` and archived there; SDR D12 names it, and this proposal specifies its full artifact set. Neither half is complete alone, and each names the other.

**SDR §9's classification was re-derived from the specs, and it was wrong in three places.** SDR §6 received the same treatment while B3 was authored, and was also wrong; the lesson is that a capability's subject matter is not evidence of where its requirements are implemented. The corrections are in `design.md` Decision 1 with the citations; in summary:

- **`component-factory` is not wholesale core.** Its requirement *Commands Consume Services via Dependency Injection* constrains `src/commands/`, which stays in `subx-cli` permanently (SDR D7), and cites `src/commands/match_command.rs:176-213`, `sync_command.rs:168-173` and `convert_command.rs:203-205`. Its `## Purpose` names those three files as well. It moves to C2b's list.
- **`parallel-processing` is not wholesale core.** *Aggregated Result Reporting* and *Progress Reporting Opt-Out* both describe code at `src/commands/match_command.rs:725-760` and are satisfiable only through `indicatif`, a permanently CLI-only dependency (SDR D8, §4). They move to C2b's list.
- **`core-reporting` is wholesale core and SDR §9 does not mention it,** because A1 created it after the SDR was written. Its five requirements cite `src/core/` and `src/services/` and nothing else. It joins this change's list.

The net is **13 capabilities, 93 requirements, 239 scenarios**, not 14.

## What Changes

**1. OpenSpec is stood up inside the submodule.** `openspec init --tools none` is run in `subx-core/`, producing `subx-core/openspec/{config.yaml,specs/,changes/archive/}`. `--tools none` rather than `--tools claude`: the parent repository ships no `.claude/skills/openspec-*` either, and eight duplicated skill files inside a submodule is exactly the duplication SDR §5 exists to bound. `config.yaml` is left as the unmodified template with its `context:` block commented out, byte-identical to the parent's — the reasons are Decision 6.

**2. Thirteen capabilities move wholesale.**

| Capability | Reqs | Scenarios | Lines |
|---|---:|---:|---:|
| `ai-provider-integration` | 12 | 40 | 286 |
| `archive-extraction` | 10 | 36 | 267 |
| `async-runtime-safety` | 3 | 7 | 61 |
| `core-reporting` | 5 | 18 | — |
| `file-operation-safety` | 4 | 10 | 77 |
| `file-organization` | 7 | 13 | 96 |
| `input-size-guards` | 3 | 7 | 58 |
| `language-detection` | 5 | 14 | 125 |
| `local-llm-provider` | 7 | 20 | 141 |
| `media-discovery` | 8 | 15 | 127 |
| `subtitle-parser-hardening` | 8 | 26 | 270 |
| `subtitle-styling` | 11 | 16 | 131 |
| `vad-speech-detection` | 10 | 17 | 132 |
| **Total** | **93** | **239** | **~1,930** |

Twelve of the thirteen are measured from `openspec/specs/` today; `core-reporting` is measured from A1's delta, because it enters the main tree only when A1 archives. None of the thirteen is touched by A0–C1, so the counts are stable through the series.

**3. `subx-cli`'s side of the move is fifteen delta files.** Thirteen `## REMOVED Requirements` deltas naming all 93 requirements by exact title; one `## ADDED Requirements` delta creating `spec-governance`; one `## MODIFIED Requirements` delta repairing `configuration-management`'s now-cross-repository reference to `ai-provider-integration`.

**4. `subx-core`'s side is one change, `import-core-specs`,** with `.openspec.yaml`, `proposal.md`, `design.md`, thirteen `## ADDED Requirements` delta specs carrying the full text, and `tasks.md`. Its artifact set is specified requirement by requirement in `design.md` Decisions 3, 4 and 5, because a change with thirteen ADDED capability files and no behaviour change is unusual enough that guessing at it would produce something wrong.

**5. Citations are resolved against the owning repository, and the rewrite rule is narrower than it looks.** B2 relocated the core sources at *identical relative paths*, so `src/core/…`, `src/services/…`, `src/config/…` and `src/error.rs` are already correct verbatim inside `subx-core` and are carried over untouched. Only four classes of citation are edited: the fourteen enumerated in the REMOVED deltas' `**Migration**` notes — three stale-risk line ranges in `language-detection`, one CLI-bound test file in `file-organization`, the `fuzz/` and quality-script citations in `subtitle-parser-hardening`, three cross-capability references in `local-llm-provider`, the `-i` and feature-gate framing in `archive-extraction`, the `--recursive` sentence in `media-discovery`, and the two `crate::`-path readings in `core-reporting`.

**6. `spec-governance` is created** — eight requirements: how a requirement is assigned to a repository and what is not evidence for it; how a capability moves wholesale, splits or stays, and why it is never duplicated or left as a pointer; what happens when a capability later grows a requirement on the other side of the line; where the capabilities that govern the *relationship* live; how path citations are resolved against the owning repository; how cross-capability references are qualified and re-qualified when their target moves; how a change spanning both repositories is authored and archived; and how the two trees are kept from drifting.

**7. The archive procedure is not the default one, and that is specified rather than discovered.** `openspec archive move-core-capabilities-to-subx-core -y` refuses: a delta that removes every requirement of a capability rebuilds a spec with zero requirements, which fails `Spec must have at least one requirement`, and the refusal is whole-change atomic. The change is archived with `--skip-specs` and all fifteen deltas are then applied by hand. On the `subx-core` side `openspec archive` succeeds, but writes `# <capability> Specification` and a `TBD` Purpose for each of the thirteen, both of which pass `validate --strict` and both of which must be replaced by hand. Every one of these behaviours was verified against `openspec` 1.6.0 in a scratch tree; the transcripts are summarised in `design.md` Decision 2.

No source file, test, manifest, workflow or script is edited in either repository. `subx-cli` stays at the version C1 set.

## Capabilities

### New Capabilities

- `spec-governance` — the rules by which specification ownership is partitioned across two repositories: the ownership test and what is not evidence for it, the move/split/stay disposition of a capability, the treatment of a capability that later grows a requirement on the other side, the placement of capabilities that govern the repository relationship, the qualification of cross-repository paths and capability references, the two-change protocol and manual archive procedure for work spanning both repositories, and the isolation and drift-checking of the two OpenSpec roots.

### Modified Capabilities

- `ai-provider-integration`, `archive-extraction`, `async-runtime-safety`, `core-reporting`, `file-operation-safety`, `file-organization`, `input-size-guards`, `language-detection`, `local-llm-provider`, `media-discovery`, `subtitle-parser-hardening`, `subtitle-styling`, `vad-speech-detection` — each as a `## REMOVED Requirements` delta naming every requirement by exact title with a reason, and with a `**Migration**` note on the fourteen requirements whose text is edited on arrival. The full text is re-added verbatim in `subx-core` by `import-core-specs`; the archive's own convention for removals is a title plus a reason, not a restatement, and restating 93 requirements twice would be an audit trail nobody could read.
- `configuration-management` — one requirement, *Local Provider Validation Rules*, restated in full so that its reference to the `ai-provider-integration` capability names `subx-core`. This change creates that dangling reference and therefore repairs it; leaving it would put a `spec-governance` violation in the tree on the day `spec-governance` lands.

## Impact

- **Code:** None. No `.rs` file, `Cargo.toml`, workflow, or script is touched in either repository.
- **Tests:** None. The one mechanical check this change introduces — the capability-name drift check required by `spec-governance` — is specified here and implemented by C3 alongside the rest of the two-repository documentation work, because it needs a home in a script that C3 is already rewriting.
- **APIs:** None.
- **Dependencies:** None. `openspec` is a development tool, already in use, at version 1.6.0; no crate, feature or lockfile entry changes.
- **Documentation:** `AGENTS.md:157` ("`openspec/` OpenSpec changes, specs, and workflow config") and its "OpenSpec and Project Skills" section (`:379-383`) gain the two-root rule and the manual-archive procedure. `subx-core` gains the same paragraphs in its own `AGENTS.md`, which C3 authors — this change writes the paragraph into `subx-cli`'s and hands the core copy forward with the text fixed. `CHANGELOG.md` in both repositories gains `[Unreleased]` entries under `### Added`, `### Changed`, `### Removed` and `### Documentation`.
- **Specifications:** `subx-cli/openspec/specs/` goes from 32 capabilities to 20 — the 13 leave, `spec-governance` arrives. `subx-core/openspec/specs/` goes from not existing to 13 capabilities, 93 requirements and 239 scenarios. The union is unchanged in content: no requirement is added, deleted, weakened or strengthened by this change.
