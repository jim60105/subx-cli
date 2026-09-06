## Context

This change is the second half of a seam C3 drew and measured. C3's Context section partitioned the documentation surface by *what breaks when it is wrong*: contract documents, read before work starts and capable of causing wrong work; reference documents, read while working and capable of causing wrong belief; and machinery that reads documents. C3 took the first and third classes. This change takes the second.

The three documents it holds are the ones whose repair is a re-authoring rather than a re-path, and they fail in three different ways that need three different treatments:

- `docs/tech-architecture.md` is **structurally** wrong. Its organising principle is "here are the layers of the crate" and there is no longer a crate whose layers those are. Twelve of its headings are module paths and four of its top-level sections describe the other repository. Re-pathing it would produce a coherent-looking document organised around a fiction.
- `docs/ai-provider-integration-guide.md` is **procedurally** wrong. It is a twelve-step checklist and ten of the twelve steps now name a file in the other repository. Its `## File Change Summary` table exists precisely to be used as a pre-pull-request checklist, so every unqualified row is an instruction to look in the wrong place.
- `docs/config-usage-analysis.md` is **temporally** wrong, and was before this series started. It is a dated snapshot with a stale date, presented in the same voice as the living documents around it.

Two constraints carry over from C3 and are not re-argued here. First, `docs/` stays wholly in `subx-cli` and `subx-core`'s reference documentation is its rustdoc — C3's Decision 3, which is why this change re-paths three documents instead of moving two. Second, `docs/command-reference.md`, `docs/machine-readable-output.md` and `docs/configuration-guide.md` were repaired by C3 despite living in the same directory, because their edits were mechanical and one of them — `command-reference.md`'s musl instructions — was a user-facing document telling users to invoke a path a normative requirement makes fail. This change must not re-open any of the three.

One new constraint arises from C3 having landed. C3 authored `subx-core/README.md` with absolute URLs into `subx-cli/docs/`, necessarily absolute because a relative path across a submodule boundary resolves on neither the git forge nor docs.rs. This change re-authors two of the documents those URLs target. No continuous-integration job in either repository can see both ends of such a link: core's workflow (C1) cannot see the parent's `docs/`, and the parent's workflow does not parse core's README. That asymmetry is the reason for the one requirement this change adds to a capability C3 already deltaed.

## Goals / Non-Goals

**Goals:**

- Re-scope `docs/tech-architecture.md` around two crates, so its structure describes what exists rather than what existed before B2.
- Crate-qualify every source path in the three remaining `docs/` files, satisfying the acceptance condition C3 set: `grep -rn 'src/' docs/` yields no unqualified path.
- Execute the two decisions C3 made about files it did not open — delete the hand-maintained dependency block (C3 Decision 6) and demote the dated configuration analysis (C3 Decision 10).
- Fix the pre-existing false statements in the sections being rewritten, where the falsity is provable against the code: seven in `tech-architecture.md`, three in the provider guide.
- Close C3's Open Question 4 by deciding what the `update-config-document` skill is for, now that its primary document is a historical record.
- Establish that a cross-repository documentation link is a two-way obligation, and re-verify the links C3 created.

**Non-Goals:**

- Re-litigating any of C3's eleven decisions. Decision 1 below records which bind this change; the rest are C3's and stay there.
- Re-opening `docs/command-reference.md`, `docs/machine-readable-output.md` or `docs/configuration-guide.md`. C3 repaired all three. This non-goal is conditional in exactly one way, recorded in Decision 8: if C3 takes its stated relief valve, those three files arrive here as scope instead, and this bullet is void.
- Touching either `AGENTS.md`, either `README.md` except for the link corrections of task 6.3, `CHANGELOG.md`'s `## [2.0.0]` section, any manifest, any workflow, or `scripts/quality_check.sh`.
- Adding a documentation check to the quality gate. C3 added the one check that is cheap and precise (three predicates over two `openspec/` trees and one marked region). A grep asserting that no document contains a bare `src/` path would fire on every legitimate path inside a future code block, and a check with a routine false-positive rate is a check that gets deleted.
- Re-deriving `docs/config-usage-analysis.md`'s 33 call trees. Decision 6 explains why, and names the headroom where that work would go if the decision is revisited.
- Translating anything. `docs/config-usage-analysis.md` stays in zh-TW; converting it to English is a rewrite of a document being demoted, and its audience is the one maintainer who reads it.

## Decisions

### Decision 1: What travels from C3, what stays, and what is restated in both

C3 made eleven decisions. Splitting a change after its design is written creates a specific hazard — a decision that binds both halves being restated inconsistently, or silently owned by neither — so the allocation is explicit.

| C3 Decision | Disposition | Reason |
|---|---|---|
| 1 — `AGENTS.md` shared region + byte-equality check | **Stays in C3** | Both files, the markers and the check are C3's. This change touches no `AGENTS.md`. |
| 2 — Document ownership by where the reader stands | **Restated here, abridged** | Its `docs/**` row is this change's premise; its submodule-contract and shared-prohibition rows are C3's. This change restates only the row it depends on and cites C3 for the argument. |
| 3 — `docs/` stays wholly in `subx-cli`; core's reference doc is its rustdoc | **Stays in C3, operative consequence restated here** | C3 is the change that decided whether `subx-core/docs/` comes into existence at all, because C3 authored core's entire documentation set. The full three-part argument belongs there. What this change restates is the consequence: the two documents that describe mostly-core code are re-pathed in place, not moved (Decision 5 below applies it to the provider guide, where the pull to move is strongest). |
| 4 — Two docs.rs sites, asymmetric link rule, the `--no-deps` gap | **Stays in C3** | Entirely about rustdoc and the doc build. This change edits no `.rs` file and no manifest. |
| 5 — `all-features = true` × `test-support` | **Stays in C3** — see below | |
| 6 — Delete the dependency block | **Travels here** | It edits `docs/tech-architecture.md:509-591`. C3 recorded the decision in order to bound this change's scope; Decision 4 below executes it and states where the replacement text goes. |
| 7 — Drift-check placement | **Stays in C3** | Scripts. This change adds no check (see Non-Goals). |
| 8 — Sizing and the seam | **Restated here with its own numbers** | C3 states the seam and names this change; Decision 8 below states this change's own budget and its own overrun condition. A follow-on that inherits a budget without re-deriving it is how the second overrun happens. |
| 9 — `subx-core/README.md` content boundary | **Stays in C3** | With one consequence travelling: the absolute links C3's Decision 9 mandated are what Decision 7 below makes a two-way obligation. |
| 10 — Demote `config-usage-analysis.md` | **Travels here** | Decision 6 below executes it and closes C3's Open Question 4 in the same breath. |
| 11 — `command-reference.md`'s musl section is C3's | **Stays in C3, as a prohibition here** | This change's Non-Goals restate it as "do not re-open", which is the only form it takes here. |

**Where the `all-features = true` × `test-support` fix lands: C3, and it should not move.** The coordinator asked explicitly, and SDR §4 now records the same answer, so this is a confirmation with its reasoning rather than a fresh choice.

Three arguments, in ascending order of weight:

1. **It is not a reference-documentation edit.** The artifact is `subx-core/Cargo.toml`'s `[package.metadata.docs.rs]` block. Nothing in `docs/` mentions it and no document is corrected by fixing it.
2. **It belongs with the change that established the surrounding contract.** C3 added `release-distribution` § *Published Crate Documentation Sites*, whose "Documentation-site configuration" clauses are the normative statement the fix satisfies — "each crate's `[package.metadata.docs.rs]` block SHALL activate only the features whose documentation belongs on a public API reference". A change that specifies an obligation and leaves its single instance to a successor is how an unowned defect is created; C3's Decision 5 already argued the point at length and it applies to itself.
3. **The deadline is C1's, not this change's.** The defect first becomes visible when `subx-core@1.0.0` is published, which C1 does. C3 is on the critical path before any release; this change is after it. Moving the fix here would move it past the moment it matters.

The counter-argument — that a manifest edit sits oddly inside a change whose title says "docs" — is real and is answered by C3's own framing: the artifact being repaired is a *documentation site*, its defect is invisible in every local build because `cargo doc --all-features` is exactly what `quality_check.sh` runs, and C3 is the change that reads both manifests' documentation metadata. If it does not happen there, nobody looks again.

### Decision 2: One citation convention, applied to all 92 references

The three documents hold roughly 92 source-path and API references between them (22 in `tech-architecture.md`, 37 in the provider guide, 33 in the configuration analysis). They need one convention, decided once, because inconsistency across a re-path pass is worse than the bare paths it replaces — a reader who sees both `subx-core/src/config/mod.rs` and `subx_core::config` cannot tell whether the distinction is meaningful.

**Chosen: two forms with a rule that separates them.**

- **A file being edited, or a line being cited, uses a repository-relative path**: `subx-core/src/config/field_validator.rs:30-35`, `subx-cli/src/cli/config_args.rs`. The prefix is the repository name, which for `subx-cli` is also the superproject root and for `subx-core` is also the submodule mount point — so the path is literally correct from the superproject working directory, which is where a reader following a checklist is standing.
- **An API item being referred to uses the crate path**: `subx_core::core::input::InputPathHandler`, `subx_cli::cli::error_ext::SubXErrorExt`. Underscored crate name, `::` separators, no file extension.

**The rule:** if the reader's next action is to open a file, give them a path; if it is to write a `use` statement or read rustdoc, give them a crate path. `**File:**` lines and the `## File Change Summary` table are paths. Prose describing what a type does is a crate path.

**Why not one form for everything.** A single path form cannot express "this item is publicly reachable at this name", which is what a consumer of `subx-core` needs and is the whole point of the D11 re-exports being documented at all. A single crate-path form cannot express a line range, and this project's house style cites line ranges everywhere — SDR §10 requires backticked `file.rs:line-line` citations in specs, and the documents mirror it.

**Bare `src/…` is never correct after this change**, which is what makes the acceptance condition a one-line grep.

### Decision 3: `docs/tech-architecture.md` is organised by crate at the top level, not by layer with qualified headings

This is the structural call, and the cheap option is tempting: leave `## Core Modules` as one list of twelve headings and qualify each one — `### Configuration Module (`subx-core/src/config/`)`, and so on. It is a smaller diff, it preserves every anchor, and it is wrong.

**Chosen: `## Core Modules` (`:37`) is replaced by two sibling sections, `## The CLI Crate (`subx-cli`)` and `## The Library Crate (`subx-core`)`, with the existing module subsections redistributed under them, plus a short `## Crate Topology` section ahead of both.**

Four reasons, and the fourth is the one that decides it:

1. **The crate boundary is the most consequential fact in the codebase and the flat form buries it.** Under qualified headings, "which repository does this live in" is a detail inside a parenthesis, twelfth in a list. It is the first question a contributor has, because getting it wrong means a commit in the wrong repository and a submodule pointer bump.
2. **The boundary is load-bearing in ways a parenthesis cannot carry.** `subx-core` may not depend on `subx-cli`, may not name it in an intra-doc link, and may not use workspace inheritance. Those are properties of the *section*, not of individual modules, and a two-section structure gives them somewhere to be stated once.
3. **The existing subsections already partition cleanly.** `### CLI Layer` (`:39`) is `subx-cli`; `### Configuration Module` (`:73`), `### Core Engine` and its six subsections (`:118-263`), and `### External Services` and its two (`:265-326`) are `subx-core`. There is no subsection that straddles, so the redistribution is a move rather than a rewrite. Only two new subsections are needed: the `Reporter` seam (A1) and `core::input` (A2), neither of which exists in the document at all.
4. **The flat form silently re-creates the defect this change exists to fix.** A document organised as one list of layers implies the layers belong to one thing. Six changes patched that document by qualifying individual sentences, and the result was C3's finding: "each document is locally plausible and globally incoherent." Qualifying twelve headings is the same operation at a larger granularity, and it would leave the next reader with the same impression the current one has.

**The cost, accepted:** every in-document anchor under `## Core Modules` changes, so `README.md` and `README.zh-TW.md` link targets and `subx-core/README.md`'s absolute links must be re-checked. That is exactly what task 6.2 and 6.3 do, and Decision 7 makes it an obligation rather than a courtesy. The cost is bounded and it is the cost the alternative pays too, one section at a time, forever.

### Decision 4: Executing C3's Decision 6 — the dependency block is deleted and the rule replaces it inside the crate-topology section

C3 decided to delete `docs/tech-architecture.md:509-591` and specified the three-part replacement: the allocation rule, a pointer to the two manifests, and the `crate-topology` capability. What C3 could not decide, because it did not open the file, is *where the replacement goes*.

**Chosen: the allocation rule goes into the new `## Crate Topology` section from Decision 3, not into a shrunken `## Dependencies` section.**

The rule is a statement about the *boundary* — which crate may hold `clap`, which owns `archive-rar`'s real gate, which holds the audio stack. It is the same subject matter as the crate topology and a different subject matter from "here is a list of dependencies". Leaving a `## Dependencies` heading with three paragraphs under it invites the next contributor to add "just one" version number, and the block regrows. Deleting the heading removes the affordance.

`### Dev Dependencies` (`:578-590`) goes with it. It has the same drift profile — it already omits `pretty_assertions` and the dev-only `regex` entry — and B3 redistributed the eight dev-dependencies across two manifests, so a single list cannot be correct for either.

What survives verbatim: `### Release Profile` (`:613-622`), because `[profile.release]`'s root-only ownership is a real constraint that SDR §5 and B1's `crate-topology` both pin, and a reader wants to see the five settings without opening a manifest. It moves under `## Crate Topology` with a sentence stating that `[profile.*]` is declared only in the workspace root and that a copy inside `subx-core/Cargo.toml` would be ignored with a warning on every build.

### Decision 5: The AI-provider guide stays one document, despite ten of twelve steps being core-side

The evidence for splitting looks strong. Step-by-step: Steps 1–8 and 10 name files under `subx-core` (`:34`, `:114`, `:125`, `:147`, `:194`, `:231`, `:251`, `:272`, `:312`); Step 11 (`:319`) creates a test that lands in `subx-core/tests/`. That is ten of twelve steps plus a test, against Step 9 (`:305`, `subx-cli/src/cli/config_args.rs`) and Step 12 (`:347-355`, `subx-cli`'s `docs/` and both READMEs).

**It stays one document.** C3's Decision 3 already argued this file specifically, and executing that argument here means recording what the split would actually cost:

- **A split produces two documents neither of which is a procedure.** The value of this file is that it is a complete checklist for a task nobody performs often. Cut at the repository boundary, the core half ends with a working provider that the CLI cannot select and that no README mentions, and the CLI half is two steps long. A reader following the core half to completion would believe they were done.
- **The two halves would have to reference each other across a submodule boundary**, by absolute URL in one direction and relative in the other, with no mechanism checking either — which is precisely the failure mode Decision 7 below adds a requirement to contain, multiplied by twelve steps.
- **The `## File Change Summary` table is the artifact that most needs to stay whole.** It exists to be read once, before opening a pull request, to confirm nothing was missed. Its whole function is completeness across layers; splitting it by repository destroys the property it exists for.
- **`spec-governance`'s reasoning applies by analogy and points the same way.** It requires a split capability to keep *one name in two repositories* specifically so that "a reader asking 'what does SubX specify about X' reads both files". Where the artifact is a *sequence* rather than a set of requirements, the equivalent of that guarantee is not two files under one name — it is one file that names the repository at each step.

**What the guide gains instead:** a **Repo** column in the `## File Change Summary` table (`:496-516`), a repository prefix on each of the twelve `**File:**` lines, and one new sentence at Step 11 stating that the integration test lands in `subx-core/tests/` because B3's ownership rule assigns a test by what it drives, and this one drives `ComponentFactory` and `TestConfigService`.

### Decision 6: Executing C3's Decision 10 — demote, and give the skill a purpose that survives the demotion

C3 decided `docs/config-usage-analysis.md` is demoted rather than re-derived, deleted, or silently re-pathed. Executing it raises the question C3 left open as its fourth Open Question: what is `.github/skills/update-config-document/SKILL.md` *for*, once its primary document is explicitly not maintained?

**Chosen: the skill survives, with its line-number instruction removed and its purpose narrowed to the coverage matrix.**

The document has two kinds of content and only one of them is being demoted:

- **The call-hierarchy line numbers** — 33 citations of the form `src/services/ai/openai.rs:252` — are a maintenance liability with no reader. They were stale before this series; keeping them accurate would require re-deriving them on every refactor of two crates; and the reader who needs to know where `api_key` is consumed is better served by `cargo` and a grep than by a number recorded a year ago. These are what the demotion header disclaims, and the skill's instruction to refresh them is removed. Left in place, that instruction would undo the demotion on its next invocation, which is a self-reversing change.
- **The coverage matrix** — which configuration keys are integrated, which are defined-but-unused, and the seven-key deprecated-but-retained inventory — is durable, is genuinely useful when adding a configuration key, and exists nowhere else in either repository. This is what the skill is re-pointed at: given a new or changed key in `subx-core/src/config/`, confirm the matrix has a row for it and that its status is right.

**Why not delete the skill along with the freshness claim.** The five-place checklist for adding a configuration key (`AGENTS.md:342-351`, mirrored in `docs/configuration-guide.md`) has no mechanical enforcement, and the matrix is the only artifact that would reveal a key added in four places out of five. A weak audit over a durable matrix is worth more than nothing, which is what deleting both would leave.

**Why not translate the document to English while opening it.** It is being demoted; a translation is a rewrite, its audience is the maintainer who wrote it, and the project's English-only rule governs code, rustdoc, comments and commit messages rather than a pre-existing zh-TW analysis document. Recorded as a deliberate non-goal rather than an oversight.

### Decision 7: A cross-repository documentation link is a two-way obligation, and that is why this change deltas a requirement C3 already deltaed

C3's Decision 9 required `subx-core/README.md` to link into `subx-cli/docs/` by absolute URL, because a relative path across a submodule boundary resolves on neither the git forge, the registry, nor docs.rs. That decision is correct and this change does not disturb it. It has an unstated consequence that this change is the first to encounter:

**The repository that owns the link is not the repository that can break it.** Decision 3 renames every anchor under `docs/tech-architecture.md`'s `## Core Modules`. If `subx-core/README.md` deep-links to one of them, the link dies — in `subx-core`, from an edit in `subx-cli`, with no signal in either place. Neither continuous-integration configuration can detect it: core's workflow (C1) has no checkout of the parent, and the parent's workflow does not parse core's README. This is structurally different from an intra-doc link, which `broken_intra_doc_links = "deny"` turns into a hard build failure, and different again from a relative Markdown link, which a forge will at least render as a 404 the reader can see.

So the obligation has to sit on the *editing* change rather than on a check: whoever re-authors a document that is the target of a cross-repository link re-verifies that link, in the repository where it lives, in the same unit of work. That is a requirement, it is not currently stated anywhere, and it is one C3 could not have satisfied — C3 created the links and this change is the first that can invalidate them.

Two obligations are added, not one. The second is smaller and was found by auditing what C3's own clause actually binds: C3 widened *Release documentation* to reach "every user-facing document that names a release asset or an installer flag", which catches `docs/command-reference.md`'s musl table and instructions — the defect C3 repairs. It does not catch `docs/tech-architecture.md:634-637`, which states a target *count* and four triples while naming no asset and no flag. That is a matrix disagreement C3's requirement cannot reach and C3 does not fix, so the clause is widened here to cover a stated matrix or count. The pattern is worth naming: three documents in this repository have carried three different wrong numbers for the same five-target matrix — `AGENTS.md` said seven, `docs/command-reference.md` said seven, this file says four — and a count drifts more quietly than a name because no reader is looking it up.

Both are expressed as a **MODIFIED** *Release documentation* rather than as new requirements, because that requirement already owns the cross-repository link rule ("Links into the superproject's documentation SHALL be absolute") and splitting a rule from its consequence across two requirements is how the pair drifts. Restating it in full on C3's end state, with C3's five scenarios carried over verbatim, is the mechanism the house style provides for exactly this.

**Why not a link checker in the gate.** It would need network access to resolve a GitHub blob URL with an anchor, it would fail on every unrelated network flake, and it would be the third check in a gate whose credibility C3 spent a decision protecting. The narrow, offline form — grep core's README for `blob/master/docs/` targets and confirm each anchor exists in the corresponding file — is a task in this change (6.2) and is worth doing once, by the change that changed the anchors, rather than on every commit forever.

### Decision 8: 6.75 h, itemised, with a stated overrun condition

C3 measured this change at 6.75 h from the outside, and that figure survives re-derivation — unlike C3's own, which was 9.25 h in an earlier draft and is 10.75 h once its small-corrections row is actually added to its column. Inheriting a number without re-deriving it is how a second overrun happens, so this one is re-derived here at task granularity:

| Item | Estimate |
|---|---|
| `docs/tech-architecture.md` — the Decision 3 restructure: two crate sections, `## Crate Topology`, redistribute twelve subsections, add the `Reporter` and `core::input` subsections | 1.75 h |
| `docs/tech-architecture.md` — Decision 4's deletion and replacement, plus the seven pre-existing defects at `:51-60`, `:70`, `:144-147`, `:267-283`, `:298`, `:634-637`, `:648` | 1.25 h |
| `docs/ai-provider-integration-guide.md` — twelve `**File:**` lines, the fourteen-row table plus its new column, the three-providers correction, the two `display_ai_usage` instructions, Step 11's ownership sentence | 2.0 h |
| `docs/config-usage-analysis.md` — the demotion header and footer, 33 path prefixes, and the skill's instruction change | 0.75 h |
| Cross-repository link re-verification and any correction in `subx-core` | 0.25 h |
| Changelog | 0.25 h |
| Acceptance verification: the `grep` condition, the doc build, the gate | 0.5 h |
| **Total** | **6.75 h** |

Against a ~9.5 h effective workday — C1's own accepted figure — that leaves **2.75 h of headroom**, and it has two named claims on it, in priority order:

1. **C3's relief valve.** C3 measures at 10.75 h, ~1.25 h over the same workday, and its stated fallback is to move its 1.5 h small-corrections row (`docs/command-reference.md`, `docs/machine-readable-output.md`, `docs/configuration-guide.md` and the `update-config-document` skill path) into this change if its `AGENTS.md` pass exceeds 3 h. Taking it puts this change at 8.25 h, still inside the workday. If that happens, `design.md` Decision 5's prohibition on re-opening those three files is lifted for this change only, and C3's Decision 11 is recorded as having been overtaken by measurement rather than by argument.
2. **Re-deriving `config-usage-analysis.md`'s 33 call trees**, ~1.5 h, if Decision 6's demotion reads badly once written.

Both cannot be taken. The first has priority, because it is a defect in a published user-facing document and the second is an improvement to a document being demoted.

**The overrun condition, stated in advance** in the shape B3 and C2b both used. The soft item is the first row: 1.75 h if the twelve subsections redistribute as cleanly as their current heading structure suggests, and up to 3.5 h if `### Core Engine`'s six subsections turn out to interleave CLI and core prose at the paragraph level rather than the section level. If task 2.1's read-through finds that interleaving, the seam is `docs/ai-provider-integration-guide.md`, lifted out whole into a change named `refresh-ai-provider-guide-for-two-crates` — it is the one item that is fully independent of the other two, shares no file with them, and is a pure re-path once Decision 5 has settled that it stays one document. Stating the condition in advance is the point: B3's overrun was found by measuring, and the failure being designed out is discovering it at subsection nine of twelve.

## Risks / Trade-offs

- **Risk: Decision 3's restructure breaks in-document anchors that something outside this repository links to.** → Mitigation: this is the risk Decision 7 exists to contain, and tasks 6.1–6.3 discharge it — the two READMEs' `docs/…` links, `subx-core/README.md`'s absolute links, and the `docs/`-internal cross-references in `machine-readable-output.md` and `command-reference.md` are all resolved against the post-change file. What cannot be checked is a link from outside both repositories; that is accepted, and the section names chosen are the durable ones (crate names) rather than the volatile ones (module paths).
- **Risk: deleting `## Dependencies` and `### Dev Dependencies` (Decision 4) removes the only at-a-glance view of the dependency set, and the replacement rule is not a substitute.** → Mitigation: correct and deliberate, and C3's Decision 6 already carries the full argument — the block was not a substitute either, having been wrong in ten ways while looking authoritative. The one command that produces an accurate list on demand is given in the replacement text.
- **Risk: the provider guide stays one document (Decision 5) and a `subx-core` contributor in a standalone clone cannot read it.** → Mitigation: partially accepted, and it is the acknowledged cost of C3's Decision 3. `subx-core/README.md` links to it absolutely, and Decision 5's own reasoning is that a core-only half would be a procedure that cannot be completed — a reader who cannot reach the document is better off than one who reaches a truncated version and believes they are finished.
- **Risk: demoting `docs/config-usage-analysis.md` (Decision 6) reads as abandonment, and the next contributor deletes it.** → Mitigation: the demotion header states positively what the document is *for* — the coverage matrix and the deprecated-key inventory — rather than only what it is not, and the skill is re-pointed at that purpose so the document retains an active consumer. The seven-key inventory's uniqueness is recorded in the header itself, so a future deletion has to argue against a stated fact.
- **Risk: the seven pre-existing `tech-architecture.md` defects are fixed as a side effect and the fix scope creeps into a rewrite of prose that is merely imperfect.** → Mitigation: the test C3 applied applies here — is the statement provably false against the code — and all seven are individually verifiable: eight enum variants at `src/cli/mod.rs:101-123` against seven documented, `dialoguer` absent from the manifest, `LocalLLMClient` at `src/core/factory.rs:232-234`, `display_ai_usage` removed by A1, five release targets, `aarch64-unknown-linux-gnu` published. A sentence that is unclear but true is out of scope.
- **Risk: the citation convention (Decision 2) is applied inconsistently across 92 references, which is worse than the bare paths.** → Mitigation: the acceptance condition greps for the failure directly (`grep -rn 'src/' docs/` finding an unqualified hit), and the two forms are distinguished by a rule about the reader's next action rather than by taste, so a reviewer can check any single citation without knowing the other 91.
- **Risk: this change is scheduled last on the series and never happens, leaving three documents wrong indefinitely.** → Mitigation: the same risk C3 carried and the same answer — it is named in SDR §12, scoped to three files, measured, given a mechanical acceptance condition, and its design decisions are all made. It is also the visibly final item, so its absence is conspicuous rather than buried.
- **Risk: the changelog entry lands under `## [2.0.0]` and amends notes that have already been published to a GitHub Release.** → Mitigation: the entry goes under `[Unreleased]`, stated in the proposal's Impact and enforced by task 7.1, which names the heading it must not touch.

## Migration Plan

1. **Read the landed state.** Confirm C3 is merged. Read `subx-core/README.md`'s absolute links and both READMEs' `docs/…` links *before* any restructure, and record every anchor they target — after Decision 3's rename, the original targets are unrecoverable from the new file.
2. **Read `docs/tech-architecture.md` end to end** and confirm the twelve subsections partition by crate at the section level rather than the paragraph level. This is Decision 8's overrun test and it happens first, while there is budget to act on it.
3. **Restructure `tech-architecture.md`** — topology section, two crate sections, redistribute, add the two missing subsections — then delete the dependency blocks, then fix the seven defects. In that order: the defects are in sections whose final location the restructure decides.
4. **The provider guide**, mechanically: `**File:**` lines, then the table, then the three content corrections.
5. **The configuration analysis and the skill**, together, because Decision 6 couples them.
6. **Re-verify the cross-repository links** against the finished documents, and correct in `subx-core` if needed.
7. **Changelog**, under `[Unreleased]`.
8. **Acceptance:** the grep condition, the `--workspace` doc build, the gate.
9. **Rollback** is per-file and independent. The one coupling is step 6 — a link correction in `subx-core` that outlives a reverted restructure in `subx-cli` would point at an anchor that no longer exists, so step 6 reverts with step 3 or not at all.

## Open Questions

- **Do the twelve module subsections of `## Core Modules` partition by crate at the section level?** Every heading says they do, and the sizing in Decision 8 assumes it. Task 2.1 verifies it by reading rather than by inspecting headings, because a paragraph inside `### Core Engine` that describes how `match_command.rs` drives the engine would straddle. Resolved at task 2.1; the overrun seam is named in advance either way.
- **Does anything outside these two repositories link to a `docs/tech-architecture.md` anchor?** The Tauri GUI at `../subx` is the only known candidate and is out of scope for edits in this series. Not blocking — the restructure proceeds regardless, and a broken external link is the accepted residual of Decision 3.
- **Should the coverage matrix in `docs/config-usage-analysis.md` eventually move into `docs/configuration-guide.md`**, letting the analysis document be deleted outright? It is the only argument for deletion that survives Decision 6, because it relocates the durable content rather than losing it. It is a larger edit than this change's budget allows and it touches a document C3 repaired. **Deliberately deferred**, with no successor named — this is a question about documentation shape, not about the crate split, and it should not be attached to this series.
