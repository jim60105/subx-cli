## Context

Every document in this repository was written for one crate in one repository. Ten changes later there are two crates in two repositories, and the documents have been patched incrementally by whichever change made a particular sentence false. That produces a specific failure mode: each document is locally plausible and globally incoherent. `AGENTS.md:87-93` prints a five-layer diagram rooted at `src/`; `AGENTS.md` also carries, courtesy of B1's task 8.3, a `## Repository Layout` section explaining that three of those five layers live in a submodule. Both were written by changes that were correct about their own edit.

The documents split into three classes by *what breaks when they are wrong*:

- **Contract documents** — `AGENTS.md` ×2, `README.md` ×3, `CHANGELOG.md` ×2. Read before work starts, by agents that `AGENTS.md:18` explicitly instructs to trust them over the codebase, and by contributors deciding whether `git clone` was enough. Wrong here produces wrong *work*: a module created in the wrong repository, a clone that will not build, a crate published without a release record.
- **Reference documents** — the six files under `docs/`. Read while working, to answer a question the reader already knows they have. Wrong here produces wrong *belief*, which is worse over time and cheaper to correct at any given moment, because the reader who is already in the code has a second source.
- **Machinery that reads documents** — `.github/skills/update-config-document/SKILL.md`, whose declared source of truth is `src/config/`, and the `spec-governance` drift check, which C2a specified and C2b supplied a record for and which neither implemented.

The third class matters more than its size suggests. It is the only part of the documentation surface that can be *checked*, and this change is the first opportunity to place a check at all — because C1 has just made `scripts/quality_check.sh` workspace-aware and given `subx-core` a script of its own, so the shape of "the gate" is finally settled.

Two facts constrain every decision below. First, `broken_intra_doc_links = "deny"` is set in both manifests, and the two crates' link graphs are **asymmetric**: `subx-core` is a dependency of `subx-cli`, so `subx-cli`'s rustdoc may name `subx_core::…` and the path resolves; the reverse is not a dependency edge at all, so a link from core to `subx_cli::…` is a hard compile failure — in the repository whose standalone CI (C1's core workflow) is the *only* place that failure surfaces before a pointer bump. Second, a submodule inherits nothing from its parent worktree (SDR §5): no `rustfmt.toml`, no `.gitignore`, no `LICENSE`, and no `docs/`. Anything a reader standing inside a lone `subx-core` clone needs must be physically present there, or reachable by absolute URL.

## Goals / Non-Goals

**Goals:**

- Make every contract document describe the two-crate, two-repository reality, so that an agent trusting `AGENTS.md` places code in the correct repository.
- Author `subx-core`'s `AGENTS.md` and `README.md` as documents in their own right, correct for a library with no binary, no CLI layer, no release matrix and no installer.
- Fix the pre-existing staleness encountered in the files this change opens, where the defect is provable: the "7 targets" claim, the `src/cli/validation.rs` reference, the missing `docs/machine-readable-output.md` inventory entry, and `docs/command-reference.md`'s musl instructions that contradict a normative requirement.
- Decide, once, where every document lives and what mechanism (if any) keeps duplicated content honest — and give the mechanism a home rather than a description.
- Implement the `spec-governance` drift check that C2a and C2b both deferred here.
- State the documentation contract for two docs.rs sites in the specification, including the asymmetric link rule and the verification gap that a `--no-deps` doc build leaves open.

**Non-Goals:**

- Rewriting `docs/tech-architecture.md`, `docs/ai-provider-integration-guide.md` or `docs/config-usage-analysis.md`. Their disposition is **decided** here (Decisions 6 and 10) and **executed** by C4, `refresh-reference-docs-for-two-crates` (Decision 8). Decision 11 explains why `docs/command-reference.md`, despite living in the same directory, is repaired here instead.
- Rewriting content that is merely imperfect. The test applied throughout is whether the split, or a landed sibling change, made a statement false — not whether a better sentence exists.
- Changing any CLI flag, configuration key, JSON envelope field, error variant or exit code. No user-observable behaviour changes.
- Touching `CHANGELOG.md`'s `## [2.0.0]` heading or `subx-core/CHANGELOG.md`'s `## [1.0.0]` heading. C1 owns those and `release.yml:29`'s `awk` extraction depends on their exact shape.
- Introducing a documentation generator, a templating step, or a docs site build. The repository's documentation is hand-written Markdown plus rustdoc, and adding a third mechanism to keep the first two honest would be a fourth thing to keep honest.
- Adding a `context:` block to either `openspec/config.yaml`. `spec-governance` requires the two files to be byte-identical unmodified templates, and C2a's reasoning — that a `context:` block is a second home for facts that live in `AGENTS.md` — is exactly this change's reasoning about duplication.

## Decisions

### Decision 1: The two `AGENTS.md` files share a marked region, checked for byte-equality — which is deliberately *not* the answer `spec-governance` gives for specs

**How much may core's `AGENTS.md` duplicate the parent's?** Exactly the region that is genuinely repository-independent, and no more. Concretely: `## Coding Conventions` (general rules, naming), the `### Critical Rules` of `## Testing Conventions`, `## Documentation Conventions`, `### Changelog Convention`, and the `### CPU-Intensive Operations — Main Agent Only` warning. Those are project conventions about how Rust is written and how tests behave; nothing in them names a crate, a layer, a binary or a release target.

Everything else is per-repository by construction and must **not** match: the build/test/quality command table (core's `scripts/quality_check.sh` is a dozen lines and takes only a profile argument; the parent's takes `-v`, `-p`, `--full` and covers both crates), the module guide, the layer overview, the file-organization tree, the feature table, the CI/CD section, the AI-provider section, and everything about the release matrix and installer, which core has none of.

**The mechanism.** The shared region is delimited in both files by

```markdown
<!-- SHARED_CONVENTIONS_START -->
…
<!-- SHARED_CONVENTIONS_END -->
```

and the governance check (Decision 7) asserts the two extracted regions are byte-identical. This reuses a convention already in the file: `AGENTS.md:412-421` is a tool-managed `<!-- CODEGRAPH_START -->…<!-- CODEGRAPH_END -->` block. Using the same shape means no new concept for a reader and no new syntax for a tool.

**Why this is a different answer from `spec-governance`'s, and why it has to be.** `spec-governance` solves spec drift by making duplication *illegal*: every requirement has exactly one owning repository, a capability never appears in full in both trees, and a name in both trees that is not in `openspec/split-capabilities.txt` is a defect. That answer is available for specs because a spec has no reader who must be standing in exactly one repository — the union of the two trees is the specification, and a person can read both.

It is **not** available for `AGENTS.md`, for the same reason SDR §5 forces `rustfmt.toml`, `.gitignore` and `LICENSE` to be duplicated: an agent working in a standalone `subx-core` clone cannot read the parent's file, because the parent is not there. Single ownership would mean core's conventions are documented in a repository core cannot see. So duplication is mandatory, and the honest response is not to pretend otherwise but to make the duplication *checkable* — which is the one property `spec-governance` says drifting duplicates lack ("Two copies of one requirement drift, and there is no mechanism in either repository that would detect the drift").

The `spec-governance` answer is therefore reused at the level of *principle* — duplication is only acceptable when a mechanism detects divergence — and rejected at the level of *mechanism*, because its mechanism assumes a reader who can see both trees.

**Alternatives considered.**

- *A single `AGENTS.md` in the parent, with core carrying a one-line pointer.* Rejected: it makes the conventions unreadable from the only vantage point a library consumer has. It is also the failure mode C2a already rejected for `openspec/config.yaml`'s `context:` block, inverted.
- *Generate core's file from the parent's at commit time (a hook or a script).* Rejected: it makes the parent's file the source and core's a build artifact, which means core's file must be regenerated by anyone editing conventions in a lone clone — and they cannot, because the source is not there. It also adds a generator to a repository that has none.
- *`git subtree`, a shared submodule holding just the conventions, or a symlink.* Rejected in one breath: a third repository to hold five sections; a symlink that Windows checkouts and `cargo package` both mishandle; and in all cases a mechanism whose failure mode is more obscure than the drift it prevents.
- *Byte-equality is too strict; check semantic equivalence.* Rejected: there is no cheap semantic check, and "roughly the same" is the state the check exists to detect. Byte-equality has a mechanical fix (copy the block) and an unambiguous verdict.

### Decision 2: Document ownership is decided by which repository the reader is standing in, not by which repository the subject matter lives in

| Document | Owner | Rationale |
|---|---|---|
| `AGENTS.md` | **Both** (Decision 1) | Read from inside whichever repository the work happens in. |
| `README.md` (English + zh-TW) | `subx-cli` | The CLI's audience: installers and end users. |
| `subx-core/README.md` | `subx-core` | The library's audience: crate consumers and the GUI (Decision 9). |
| `CHANGELOG.md` | **Both**, independently | Two crates with independent version lines (SDR D6) each publish to crates.io, and crates.io shows the changelog of the crate you installed. |
| `docs/**` (all six) | `subx-cli` only | Decision 3. |
| The submodule / workspace **contract** | `subx-cli` | See below. |
| `subx-core`'s API reference | `subx-core`, as rustdoc | Decision 3. |
| `.github/skills/**` | `subx-cli` | The skills are read by agents operating on the superproject; the one project-specific skill has its declared source path re-qualified to `subx-core/src/config/`. |

**The document that genuinely describes both repositories is the submodule contract, and it lives in `subx-cli` — with one carved-out exception that is duplicated deliberately.** The reasoning is *actionability*, not subject matter. Every operation the contract governs is performed from the superproject: you add the submodule in `subx-cli`, you commit the gitlink in `subx-cli`, you bump the pointer in `subx-cli`, and `cargo publish --workspace` runs in `subx-cli`. A reader standing inside a lone `subx-core` clone cannot perform any of it, so a copy there would be information they cannot act on.

The exception is the set of prohibitions that bind **core's own manifest** — no `[workspace]` table, no workspace inheritance (`version.workspace`, `[workspace.dependencies]`, `[workspace.lints]`), no `[profile.*]` — because those *are* actionable from inside a lone clone, and violating them is exactly the failure mode SDR D3 exists to prevent: a manifest that builds fine as a workspace member and breaks a standalone clone. Those prohibitions therefore go inside the marked shared region of Decision 1, so the drift check covers them and neither file can quietly lose them.

### Decision 3: `docs/` stays wholly in `subx-cli`. Core's reference documentation is its rustdoc, which is a distributed artifact rather than a file

The naive answer is to split `docs/` the way the code was split: `tech-architecture.md` and `ai-provider-integration-guide.md` follow the modules they describe into `subx-core/docs/`, the user-facing four stay. It is wrong for three separate reasons.

**First, the audience does not split the way the code does.** Four of the six documents (`command-reference.md`, `configuration-guide.md`, `machine-readable-output.md`, and `config-usage-analysis.md` insofar as it describes `subx config`) describe *the behaviour of the `subx-cli` binary*. `configuration-guide.md` documents config keys whose implementation is entirely in `subx-core/src/config/` — and its reader is a person editing `~/.config/subx/config.toml`, who has never heard of either crate. Following the implementation would send the most user-facing document in the tree into the library repository. `machine-readable-output.md` is the same shape: the envelope is rendered by `subx-cli/src/cli/output.rs` but the `category` and `machine_code` values it tabulates are core's. Every one of these documents straddles the line, and every one of them has a CLI audience.

**Second, `ai-provider-integration-guide.md` is the case that looks like it should move and must not.** After B2, eleven of its fourteen "File Change Summary" rows are `subx-core` paths. But its remaining rows are `subx-cli/src/cli/config_args.rs`, `README.md` and `README.zh-TW.md`; its Step 9 is titled "Update CLI Documentation"; and its Step 11 tells the author to write an integration test which, under B3's ownership rule, lands in `subx-core/tests/`. It is a *procedure that spans both repositories*. Moving it to core would put a document that instructs the reader to edit `subx-cli` files in the repository that cannot see them, and Decision 2's actionability test then says the same thing it said about the submodule contract: it belongs where the work is coordinated, which is the superproject.

**Third, duplication is the one option no argument supports.** Copying `docs/` into core gives six more files with no detector, in direct violation of the principle Decision 1 established.

**So what does a standalone `subx-core` consumer read?** Its rustdoc, and that is not a consolation prize. Rustdoc is the *right* reference form for a library: it is generated from the code so it cannot drift, its examples are compiled by `cargo test --doc --all-features` on every quality run, `broken_intra_doc_links = "deny"` makes a stale path a build failure rather than a stale sentence, and it is published to `docs.rs/subx-core` where consumers actually look. The three properties `docs/` cannot have — non-drifting, compiled, published — are exactly the three rustdoc has. `subx-core/README.md` then provides orientation and links into `subx-cli/docs/` by **absolute** URL (`https://github.com/jim60105/subx-cli/blob/master/docs/…`), because a relative path from inside a submodule resolves on neither GitHub's blob view nor crates.io nor docs.rs.

**The consequence accepted deliberately:** a reader in a lone core clone must follow a link to a second repository to read the architecture narrative. That is a real cost. It is smaller than either alternative — a document they cannot act on, or six documents that silently disagree with the parent's — and it is the same trade SDR §5 already made when it duplicated eight configuration files and *not* the documentation.

### Decision 4: Two docs.rs sites, one asymmetric link rule, and a verification gap that `--no-deps` hides

The split produces `docs.rs/subx-cli` and `docs.rs/subx-core`. Three rules follow, all of which go into the specification because two of them are hard build failures and the third is silently invisible.

**Rule 1 — core's rustdoc may never name `subx_cli` in an intra-doc link.** `subx-cli` is not a dependency of `subx-core`; the path does not resolve; `broken_intra_doc_links = "deny"` turns it into a compile error. The failure is worst in a standalone clone, where it is the *only* thing that breaks. Where core genuinely must mention CLI behaviour — SDR §2.3 requires exactly this twice, for `hint()`'s CLI-flavoured prose and for `OutputModeUnsupported` being constructed only by the CLI — it is written as plain prose with a bare backticked crate name, never as a bracketed link. Note that `bare_urls = "warn"` means a raw URL is not a free substitute either; a Markdown link is.

**Rule 2 — `subx-cli`'s rustdoc may link into `subx_core::…`, and the D11 re-export block must say where the canonical documentation is.** The path resolves because core is a dependency, so the lint is satisfied. But `subx-cli`'s doc site will show `config`, `core`, `error` and `services` as re-exports of another crate, and a reader landing there from a search engine has no signal that the real documentation is one site over. AGENTS.md forbids new `#[deprecated]`, and SDR D11 already prescribes the instrument: rustdoc prose. The re-export block's doc comment states that these paths exist for compatibility and that `docs.rs/subx-core` is canonical.

**Rule 3 — the boundary is verified by a doc build *with* dependencies, not by the green `--no-deps` run.** This is the non-obvious one. `quality_check.sh:253` runs `cargo doc --all-features --no-deps --document-private-items`. `--no-deps` suppresses generation of dependency documentation but does **not** suppress *resolution* of cross-crate intra-doc paths — so a `[subx_core::config::Config]` link passes the deny lint and then produces an `href` into a directory that was never generated. Locally that is a dead link with a green gate. On docs.rs it happens to work, because docs.rs supplies `--extern-html-root-url` per dependency. A gate that is green locally and correct in production only by the grace of a different flag set is not a gate.

The fix is already half-landed: C1 adds `--workspace` to `quality_check.sh`'s four Cargo invocations including the doc step, which makes both members' documentation generate in the same `target/doc` tree, at which point the cross-crate hrefs resolve locally too. This change writes that down as the reason `--workspace` is load-bearing on the doc step specifically, and adds a task that actually opens the generated `target/doc/subx_cli/index.html` re-export links rather than trusting the exit code.

### Decision 5: `subx-core`'s `[package.metadata.docs.rs] all-features = true` publishes test scaffolding, and the narrower metadata block is preferred over `#[doc(hidden)]`

B1 duplicated `[package.metadata.docs.rs] all-features = true` from `subx-cli/Cargo.toml:37-39` into core "for parity". B3 then added a `test-support` feature that gates `subx_core::test_support::{workspace, file_managers, mock_openai, mock_azure_openai, mock_generators, responses}` — roughly 1,930 LOC of relocated test helpers — and activated `wiremock` and `hound` as optional dependencies behind it. `all-features = true` therefore turns `test-support` **on** for the docs.rs build, so `docs.rs/subx-core` will present wiremock-based mock servers and a `TempDir` fixture builder as part of the library's documented public surface, and will pull two HTTP/audio test dependencies into the documentation build. B3 gave the module `#[allow(missing_docs)]` precisely so that test scaffolding would not dilute the crate's rustdoc contract — publishing it on the doc site defeats that intent by a different route.

**Chosen: narrow core's `[package.metadata.docs.rs]` to name the features that belong on the doc site** (`archive-rar`, which is a real capability a consumer chooses), rather than `all-features = true`.

**Rejected: `#[doc(hidden)]` on `subx_core::test_support`.** It would work, and it is one line. But it lies to a reader who is *supposed* to use those items: SDR D10 makes `TestConfigService`, `TestConfigBuilder` and `TestEnvironmentProvider` unconditional public API specifically because the GUI's own test suite depends on them, and B3 moved the mock helpers into `src/` specifically so that `subx-cli`'s tests could reach them across the crate boundary. Those are documented, supported, intentionally-public items with real out-of-crate consumers; `#[doc(hidden)]` marks an item as an implementation detail. The problem is not that the items are hidden-worthy, it is that the doc site is configured to activate a feature nobody browsing docs.rs wants activated. Fix the configuration, not the items.

**Why a manifest edit belongs to a documentation change at all.** The artifact being repaired is a documentation site, its defect is invisible in every local build (`cargo doc --all-features` is what `quality_check.sh` runs, so locally the scaffolding appears and looks intentional), and it first becomes visible at the moment `subx-core@1.0.0` is published — which C1 does. This is the change that reads both manifests' documentation metadata; leaving it would mean nobody looks at it again.

### Decision 6: The hand-maintained dependency block in `docs/tech-architecture.md` is deleted, and what replaces it is a rule plus a pointer

`docs/tech-architecture.md:509-591` transcribes `Cargo.toml`'s `[dependencies]` and `[dev-dependencies]` into prose. Measured against the real manifest it is wrong in at least ten ways: it lists five crates A0 deletes as dead (`notify`, `once_cell`, `tokio-util`, `winapi`, `libc`), plus `dialoguer` and `md5`, which appear in no manifest at all; it shows `tokio` with `features = ["full"]` against a real five-feature list and `symphonia` with `"all"` against a real nine-codec list with `default-features = false`; it pins `rubato = "0.16.2"` against a real `2.0`; it places `hound` under runtime dependencies when it has zero `src/` use sites; and it omits `zip`, `unrar`, `tar`, `flate2`, `sevenz-rust2`, `tempfile` and `audioadapter-buffers` entirely. `dialoguer` even has a second life at `:70`, where the CLI layer is said to depend on it "for interactive prompts".

A0 re-syncs the block. That is the second documented repair of the same block, and the split is about to double it: two manifests, two blocks, two opportunities to drift, in a document nobody edits when they add a dependency.

**Chosen: delete both TOML blocks.** What replaces them is not nothing, and not a shorter list:

1. **The allocation rule**, which is the part a reader actually cannot get from `cargo tree`: `clap`, `clap_complete`, `colored`, `tabled` and `indicatif` are permanently `subx-cli`-only (SDR D8) and a change that puts any of them in core is a design error, not a dependency addition; `archive-rar` and `slow-tests` are owned in core and pass through from the CLI; and the audio/VAD/archive stack is core's.
2. **A pointer to the two manifests** as the source of truth, plus the one command that answers the question the block was trying to answer (`cargo tree -p subx-core --depth 1`).
3. **The `crate-topology` capability**, which is normative, validated by `openspec validate --strict`, and already carries the dependency-form and feature-ownership contracts.

A prose list of version numbers answers no question that a machine-readable file two directories away does not answer better and more accurately. The rule is the only durable content, and it is content the manifests cannot express.

**Executed by C4**, since it edits `docs/tech-architecture.md`. The decision is recorded here because C4's scope is defined by it: with the block deleted, that document's remaining work is a restructure rather than a restructure *plus* a two-manifest transcription. C4's `design.md` Decision 4 settles the one thing this decision cannot — where the replacement prose goes — and puts it in the new crate-topology section rather than under a surviving `## Dependencies` heading, on the grounds that the heading is the affordance the block regrows through.

### Decision 7: The `spec-governance` drift check lands as a step in the parent's quality script, asserting three predicates

C2a specified the check ("A check SHALL enumerate the capability directory names in both trees and assert that no name appears in both **except** where that capability is deliberately split"), declined to implement it, and recorded the reason: "the natural home is the quality script and C3 is rewriting those." C2b then created the record it reads, `openspec/split-capabilities.txt`, and repeated the deferral. This change places it.

**Where.** `subx-cli/scripts/quality_check.sh`, with a parity implementation in `quality_check.ps1` because `AGENTS.md:28` presents the two as equivalent and CI runs the PowerShell port on Windows. **Only** in the parent: the check compares two `openspec/` trees and two `AGENTS.md` files, and only the superproject can see both. `subx-core/scripts/quality_check.sh` — the dozen-line script C1 created — does not get it, and cannot.

**What it asserts.** Three predicates, each one line of shell:

1. No capability directory name appears in both `openspec/specs/` and `subx-core/openspec/specs/` unless it is present in `openspec/split-capabilities.txt` (`grep -qxF`).
2. Every name in `split-capabilities.txt` is present in **both** trees. C2b's reasoning: "a name in one tree and in the list is also a defect (a split that lost a half)."
3. The region between `<!-- SHARED_CONVENTIONS_START -->` and `<!-- SHARED_CONVENTIONS_END -->` is byte-identical in `AGENTS.md` and `subx-core/AGENTS.md` (Decision 1).

**Why a script assertion and not a `#[test]`.** It reads two `openspec/` trees, two Markdown files and a text record; no Rust symbol is involved, and a Rust test would have to locate the repository root from `CARGO_MANIFEST_DIR` and would run once per crate. C1 set the precedent with its grep assertion that `--allow-dirty` is absent from the workflows: a check whose subject is repository state belongs in the gate, not in the test binary.

**Degradation when the submodule is absent.** If `subx-core/openspec/specs/` does not exist — an uninitialised submodule — the check **skips with a warning** rather than failing. A missing submodule already fails the build at manifest-parse time with a clear message (B1's whole point); a second, more confusing failure from a spec-governance check adds nothing. It skips loudly, not silently.

**The residual gap C2b named, accepted rather than closed.** C2b's open question observes that the name-level check cannot catch two differently-titled requirements in the two halves of one split capability that contradict each other, and floats "require each split capability's two Purposes to name each other" as a mechanical but weak mitigation. It is declined: it is a check on wording, it would pass on two Purposes that name each other and still contradict, and it creates an obligation on twelve capability files for no detection power. The gap is recorded in Open Questions as deliberately open.

### Decision 8: This does not fit one workday. The seam is contract documents here, reference documents in C4

**The measurement.** The surface is 4,292 lines and 178 KB across nine `subx-cli` documents, plus `CHANGELOG.md` (552 lines), plus three documents in `subx-core` that are either unwritten or first-cut. Per-item, with the seam already applied:

| Item | Change | Estimate |
|---|---|---|
| `AGENTS.md` — nine sections re-scoped, three pre-existing defects fixed, coherence pass over six siblings' incremental edits | C3 | 2.0 h |
| `subx-core/AGENTS.md` — read end to end, made correct as a library document, shared region installed | C3 | 1.5 h |
| Governance check in `quality_check.sh` + `quality_check.ps1`, with a deliberately-broken fixture run | C3 | 2.0 h |
| `README.md` + `README.zh-TW.md` + `subx-core/README.md` | C3 | 2.0 h |
| `docs/command-reference.md:10-40` (musl), `docs/machine-readable-output.md:639,748,855-867`, `docs/configuration-guide.md:341` + checklist, and the `update-config-document` skill's path | C3 | 1.5 h |
| `CHANGELOG.md` `### Documentation` + `subx-core/CHANGELOG.md` | C3 | 0.5 h |
| Verification: `cargo doc --workspace`, doctests, cross-crate href inspection, both gates | C3 | 1.25 h |
| **This change** | | **10.75 h** |
| `docs/tech-architecture.md` — restructure 651 lines whose headings are module paths; execute Decision 6; seven pre-existing defects | C4 | 3.0 h |
| `docs/ai-provider-integration-guide.md` — 37 touchpoints across 12 steps, the File Change Summary table, the `display_ai_usage` → `Reporter::ai_usage` correction, and the "only three have runtime implementations" defect (there are four) | C4 | 2.0 h |
| `docs/config-usage-analysis.md` — execute Decision 10, and the skill's instruction change | C4 | 0.75 h |
| C4's verification: the acceptance grep, the cross-repository links, its gate | C4 | 1.0 h |
| **C4** | | **6.75 h** |
| **Total** | | **17.5 h** |

The effective workday in this series is ~9.5 h — C1 measured itself at 9.5 h and B3 at 9 h, and both are booked as "1 d" in the implementation plan. 17.5 h is not one workday by any accounting, which is what makes the seam necessary.

**This change is 10.75 h, which is ~1.25 h over that workday, and the overrun is stated rather than absorbed.** An earlier draft of this decision reported 9.25 h; that figure was an arithmetic slip — the 1.5 h small-corrections row was argued into this change's scope in prose but never added to its column, and the number propagated into SDR §12 before it was checked. The corrected figures are 10.75 h here and 6.75 h in C4, summing to the 17.5 h originally measured. **SDR §12's entry for C3 needs correcting from 9.25 h to 10.75 h.**

Three responses were available and the third is chosen:

- *Absorb it.* Defensible on precedent — the series does not require exactly one day per change (A0 is booked at ~0.5 d, B4 at ~0.7 d) and a 13% overrun on a documentation change carries none of the mid-flight risk B3's did. But absorbing an overrun silently is the specific behaviour this series designs against.
- *Move the 1.5 h small-corrections row to C4 now.* It balances the pair exactly (9.25 h / 8.25 h) and it costs Decision 11's argument: `docs/command-reference.md`'s musl instructions are a contract defect, not a reference-document defect, and shipping the contract half without them leaves a user-facing document telling users to invoke a path a normative requirement makes fail.
- **Chosen: keep the row here, book 10.75 h, and name the relief valve.** If this change runs long, the 1.5 h small-corrections row (tasks 6.1–6.8) is what moves, and it moves to C4, which has 2.75 h of headroom against the same workday. The condition for taking it: task 2's `AGENTS.md` pass exceeding 3 h, which is the only soft item — the other six rows are bounded by files whose contents are already read. Decision 11's argument is the default and this is its measured fallback, in the shape B3 and C2b both used.

**Which side the small-corrections row falls on, and why it is C3's.** `docs/command-reference.md`, `docs/machine-readable-output.md` and `docs/configuration-guide.md` live in the same directory as C4's three files, so the tidy answer would send them there too. They are C3's for two reasons: their edits are mechanical rather than re-authoring, so they do not share C4's risk profile; and one of them is the musl contradiction, a user-facing document instructing users to invoke a path a normative requirement makes fail (Decision 11). That is a contract defect wearing a reference document's clothes, and it belongs with the contract half.

**Chosen seam: contract documents versus reference documents**, drawn along the class boundary from the Context section. Four properties make it the honest seam rather than the convenient one:

1. **The two halves fail differently.** A wrong `AGENTS.md` produces a module in the wrong repository; a wrong `tech-architecture.md` produces a wrong belief in a reader who is already in the code and has a second source. Deferring the second is a decision about *severity*; deferring half of `AGENTS.md` would be a decision about *convenience*.
2. **No file is touched twice.** The two halves are disjoint at file granularity, so C4 is not a second pass over anything and cannot conflict. C3's task 9.1 asserts it.
3. **C4's scope is fully determined before it starts.** Decisions 6 and 10 make its two hard calls and Decision 11 fixes what it must not re-open, so it inherits no open design. C4's own `design.md` Decision 1 records the full inheritance table.
4. **It blocks nothing.** D1 and D2 (`make-core-engines-thread-safe`, `expose-core-orchestration-apis`) touch `subx-core/src/**` and read no documentation. The critical path ends with this change either way.

**Rejected seams.**

- *By repository (`subx-cli` docs now, `subx-core` docs later).* Worst option. It leaves `subx-core/AGENTS.md` in its accreted state, which is the single document most likely to cause wrong work, in the repository with the fewest readers who would notice.
- *By defect origin (split-caused now, pre-existing later).* Superficially principled and operationally bad: it means opening `docs/command-reference.md` to change nothing while its musl instructions contradict a normative requirement, and opening `docs/tech-architecture.md` twice.
- *Ship all sixteen hours as "one change" and let it run long.* Rejected on the series' own precedent. B3 measured itself at 14.5 h and named B4; C2b pre-registered the condition under which it would name `harden-split-capability-specs`. Discovering the overrun at document seven of twelve is the failure being designed out.

**One requirement travels with C4 rather than being dropped.** This change widens `release-distribution` § *Release documentation* to bind "every user-facing document that names a release asset or an installer flag", which is what catches `docs/command-reference.md`'s musl table. It does **not** reach `docs/tech-architecture.md:634-637`, which states a target *count* and four triples while naming no asset and no flag — a matrix disagreement this change cannot fix because it does not open that file. C4 therefore widens the same clause again, to cover a stated matrix or count, and fixes the instance. The alternative — stating the wider obligation here and leaving its only instance to a successor — is how an unowned defect is created.

### Decision 9: `subx-core/README.md` is an API-consumer document, and the things it must *not* contain are the point

B2 already noted that core's README "loses its 'placeholder' note and gains the real module map". This change decides its shape, and the negative list is the substantive part, because copying the CLI's README is the default failure:

**Contains:** what the crate is (the library half of SubX: subtitle parsing and conversion, AI matching, VAD timeline sync, translation, archive extraction, configuration); its two consumers (`subx-cli` and the Tauri GUI at `jim60105/subx`) and the note that a library consumer should depend on `subx-core` and never on `subx-cli`; that a standalone clone is supported and is how crates.io consumers see the crate; the two features (`archive-rar`, `test-support`) and what each is for; the caret relationship to `subx-cli`'s dependency line; a `docs.rs/subx-core` link as the API reference; absolute links into `subx-cli/docs/` for the narrative documents; and GPL-3.0-or-later.

**Must not contain:**

- **The logo.** `README.md:4` is `<img src="assets/logo.svg">`, and `assets/` is listed in `Cargo.toml:19`'s `exclude`. Core has no `assets/` at all and cannot reach the parent's across the submodule boundary. (A task additionally verifies how crates.io renders the parent's own relative reference, since the same `exclude` applies there; if it does not resolve, it becomes an absolute raw URL in both READMEs.)
- **The `subx-cli` badge row.** After C1, core has its own `build-test-audit-coverage.yml`; pointing core's badges at the parent's workflows would report the wrong repository's status. Core's badges are its own workflow, `crates.io/crates/subx-core` and `docs.rs/subx-core`.
- **The install story, the command table, the `--output json` section, the supported-formats table as a *user* feature list.** Core has no binary. A reader who arrives at core wanting to install SubX is sent to `subx-cli` in one line, not served a duplicate quick-start that will drift.
- **A zh-TW translation.** The parent maintains `README.zh-TW.md` because it has end users; core's audience is Rust consumers reading `docs.rs`. Adding a second-language file to core creates a translation pair with no reader and no mechanism, and Decision 1's principle applies: do not duplicate what nothing checks.

### Decision 10: `docs/config-usage-analysis.md` is demoted to a dated record, not re-derived

The document is a snapshot ("**最後更新**：2025-07-07"), the only zh-TW file under `docs/`, 119 lines and 21 KB of wide tables carrying 33 line-precise `file.rs:line` citations. It was already stale before this series — it describes v1.5.x line numbers against a v1.9.1 tree, omits the entire `[translation]` config section that `docs/configuration-guide.md:226` documents, and contradicts itself (its summary says `vad.chunk_size` was "removed as non-existent" while its consistency section lists it as supported by both `get_config_value` and `set_config_value`). The split adds a second kind of wrongness on top: every one of its `src/config/`, `src/core/` and `src/services/` citations now names the wrong repository.

Options: re-derive all 33 call trees against both crates (a day, and stale again within a month); re-path the directory prefixes only (a lie with a corrected address); demote; delete.

**Chosen: demote.** The document gains a header stating the commit it describes, that its call-site line numbers are a historical record and not maintained, and that the current source of truth is `docs/configuration-guide.md` for behaviour and the two manifests' `src/config/` trees for implementation. Its path prefixes are re-qualified to `subx-core/src/…` so the file at least names the correct repository. Its maintenance skill, `.github/skills/update-config-document/SKILL.md`, has its declared source of truth changed from `src/config/` to `subx-core/src/config/` and its "update the line numbers" instruction removed.

**Why not delete.** Its durable content is the coverage matrix — which configuration keys are integrated and which are deprecated-but-retained — and the seven-key deprecated inventory (`correlation_threshold`, `dialogue_detection_threshold`, `min_dialogue_duration_ms`, `dialogue_merge_gap_ms`, `enable_dialogue_detection`, `audio_sample_rate`, `auto_detect_sample_rate`) is recorded nowhere else in the repository. Deleting it loses a fact; demoting it loses only a claim to freshness that the file has not honoured in over a year.

**Executed by C4**, with the decision made here so C4 inherits no open design. C4's `design.md` Decision 6 additionally resolves what the `update-config-document` skill is *for* once its primary document is a historical record — Open Question 4 below.

### Decision 11: `docs/command-reference.md`'s musl section is repaired in *this* change, because a reference document that contradicts a normative requirement is a contract defect

The brief for this change expected `command-reference.md` to be the least affected document, on the reasonable ground that it describes the CLI surface and the CLI surface did not move. That is confirmed: the file contains **zero** `src/` path references, and the split makes not one sentence of it false.

What it does contain, at `:16-40`, is a table of seven release assets including four `-musl` entries, followed by prose telling the user to "Opt into the musl build via the `SUBX_LIBC=musl` environment variable or the `--musl` flag", with a worked `curl` example. `release-distribution` § *Installer musl-input rejection* requires that the installer refuse every one of those inputs with exit code `2` and **no** HTTP request, and that the rejection path is "the *only* legal handling of these values". The archived change `2026-04-28-drop-musl-support` landed that in April; `README.md:124-135` and `README.zh-TW.md:100-111` were updated to five targets and this file was not.

So the most-read user-facing reference instructs users to invoke a path the specification requires to fail. That is not stale prose, it is a documented instruction to do the wrong thing, and it is the same class of defect as `AGENTS.md:18` telling an agent to trust a stale module guide. It is repaired here, with the READMEs and `AGENTS.md`, and not deferred with the rest of `docs/`.

The generalisable finding: "which documents did the split affect" and "which documents are wrong" are different questions, and this file answers *no* to the first and *yes*, worst in the tree, to the second. The brief's instruction to confirm rather than assume was the right instruction and produced the opposite of the expected answer.

## Risks / Trade-offs

- **Risk: the shared-region byte-equality check becomes a nuisance that gets deleted the first time someone edits one file and not the other.** → Mitigation: the region is deliberately small (five sections, none of which changes often — coding conventions, testing critical rules, documentation conventions, changelog convention, the CPU warning) and the fix is mechanical and obvious from the failure message, which names both files and prints the diff. The failure mode being avoided is the opposite one: a check so broad that its red state is normal.
- **Risk: `subx-core/AGENTS.md` drifts anyway, outside the marked region, because six changes each added a section and no future change reads it end to end.** → Mitigation: partially accepted. Outside the shared region the two files are *supposed* to differ, so no mechanical check applies. The mitigation is structural rather than automated: `crate-topology`'s duplication list and the *Published Crate Documentation Sites* requirement added here both bind core's documentation, so a divergence is a spec violation with an owner rather than an oversight.
- **Risk: deleting the dependency block (Decision 6) removes the only place a reader could see the dependency set at a glance, and the replacement rule is not a substitute.** → Mitigation: correct, and deliberate — the block was not a substitute either, having been wrong in ten ways while looking authoritative. The replacement carries the one thing the manifests cannot express (the allocation rule and the permanent CLI-only set) and a command that produces the accurate list on demand.
- **Risk: `subx-core`'s rustdoc as the sole reference documentation for the library (Decision 3) is thinner than `docs/tech-architecture.md` and nobody notices the gap until a consumer complains.** → Mitigation: the gap is bounded and named — core's rustdoc covers API, `subx-cli/docs/tech-architecture.md` covers narrative, and `subx-core/README.md` links to the latter absolutely. `quality_check.sh`'s existing missing-documentation clippy pass (`:296`) already reports undocumented public items in both crates once C1's `--workspace` lands, so thinness is measurable rather than assumed.
- **Risk: narrowing core's `[package.metadata.docs.rs]` (Decision 5) hides an item some consumer expected to find on docs.rs.** → Mitigation: the only items affected are `subx_core::test_support::*`, which B3 already marked `#[allow(missing_docs)]` as scaffolding, and every SDR D10 item the GUI actually depends on (`TestConfigService`, `TestConfigBuilder`, `TestEnvironmentProvider`) lives in `config/`, is unconditional, and is unaffected. Reversible in one line if wrong.
- **Risk: C4 is never scheduled, and three reference documents stay wrong indefinitely.** → Mitigation: the same risk B4 carried and the same answer, taken one step further — C4 is not merely named but authored, with its own `proposal.md`, `design.md`, delta spec and task list, scoped to specific files, independently measured, given a mechanical acceptance condition, and booked in SDR §12. Its two inherited decisions are made here so it cannot stall on an open question. It is also the visibly final item on the series, so its non-completion is conspicuous rather than buried.
- **Risk: `cargo doc --workspace` turns the quality gate red on rustdoc problems in `subx-core` that were never being checked, during a documentation change that did not cause them.** → Mitigation: C1 already lands `--workspace` and verified the doc step green, so the expected surprise count is zero; and this change's tasks run the doc build early (task 1.4) rather than at the gate, so a surprise arrives while there is budget to attribute it.
- **Risk: the governance check's skip-on-missing-submodule behaviour means it silently never runs in some CI job that forgot `submodules: recursive`.** → Mitigation: B1 added `submodules: recursive` to all seven checkout steps and the workspace fails to parse without it, so a job reaching the check with no submodule cannot exist. The skip exists for a developer running the script in a partially-initialised local clone, and it warns rather than passing quietly.
- **Risk: rewriting `README.zh-TW.md` to mirror the English introduces a translation divergence.** → Mitigation: the changes are three localisable units (a badge, a `--recurse-submodules` line, a short two-crate consumption section), all of which have existing parallel structure in the file to follow. There is no mechanical check on the pair and none is added — Decision 1's principle says do not duplicate what nothing checks, and the READMEs are an existing exception this change does not widen.

## Migration Plan

1. **Read the landed state.** Confirm C1 and C2b are merged; read both `AGENTS.md` files, `openspec/split-capabilities.txt`, and C1's `## [2.0.0]` changelog section as they actually exist rather than as siblings described them. Run `scripts/quality_check.sh` once to record the pre-change verdict.
2. **Author `subx-cli/AGENTS.md`**, including the shared-region markers, preserving the CodeGraph block byte-for-byte.
3. **Author `subx-core/AGENTS.md`**, copying the marked region in verbatim.
4. **Land the governance check**, in both scripts, and prove it red on a deliberately-diverged shared region and on a synthetic duplicate capability name before proving it green.
5. **READMEs**, all three, English first then zh-TW, then core's.
6. **The three small `docs/` corrections and the skill path.**
7. **Both changelogs' `### Documentation` entries.**
8. **Decision 5's manifest narrowing**, then `cargo doc --workspace --all-features` and an actual click-through of the re-export links in `target/doc`.
9. **Quality gate**, main agent only.
10. **Rollback** is per-file and independent; the only cross-file coupling is the shared region, and reverting one file's copy makes the check red immediately rather than silently.

## Open Questions

- **Does crates.io resolve `README.md:4`'s relative `assets/logo.svg` when `assets/` is in `Cargo.toml`'s `exclude` list?** Task 5.6 verifies it against the live crates.io rendering of `subx-cli@1.9.1` rather than reasoning about it. If it does not resolve, both READMEs move to an absolute raw URL; if it does, nothing changes. Not blocking either way.
- **C2b's residual coherence gap.** The governance check catches a requirement title present in both trees; nothing catches two differently-titled requirements in the two halves of one split capability that contradict each other. Decision 7 declines C2b's proposed mitigation as weak. **Left deliberately open**, recorded here so it is a known gap rather than an unexamined one.
- **Should `docs/config-usage-analysis.md`'s call trees be re-derived, rather than demoted?** Decision 10 says no, and C4 carries the same answer in its own Decision 6 with ~1.5 h of headroom named as the place that work would go. **Delegated to C4**, which is the change that opens the file; nobody else should revisit it.
- **Whether the `update-config-document` skill should survive at all**, given its primary document is being demoted to a historical record. **Answered in C4** (its `design.md` Decision 6): it survives, with its line-number-refresh instruction removed — an instruction that would reverse the demotion on the skill's next invocation — and its purpose narrowed to the coverage matrix. The path correction is this change's regardless, because it was wrong the moment B2 landed (task 6.8).
