# C2b implementation plan (apply stage of split-mixed-capabilities-across-repos) — FINAL for RD#1

## Authoritative re-derived numbers (measured, not from proposal)
- Donor baseline (13 MIXED caps): **136** requirement titles, not 134.
  - `input-path-handling` 14 (proposal said 13), `timeline-sync` 19 (said 18).
  - The two extras are late additions by A2-series proposals merged after this change was drafted:
    1. `Archive-Aware Output Location Resolution` (input-path-handling, from D2 expose-core-orchestration-apis archive d5b3381) — object: `default_output_dir`/`default_output_path` on `CollectedFiles` in core `src/core/input/mod.rs`. **CORE** per spec-governance ownership test.
    2. `VAD-Independent Manual Offset Application` (timeline-sync, from A2 make-core-engines-thread-safe archive a574120) — object: free fn `shift_subtitle_timing` in core `src/core/sync/mod.rs`. **CORE**.
  - Neither is in this change's delta REMOVED lists. Fix per apply.md step 2: add two REMOVED entries (Reason+Migration) to the donor delta files; carry both into core deltas verbatim with de-qualifications: `subx-core/src/...` self-paths → root-relative; `subx-core::core::sync` crate path KEPT (crate path, not file path); "`subx-core` SHALL expose" → "The library SHALL expose"; "those commands print the resolved path" → named as subx-cli commands.
- Reconciled totals:
  - Delta ops: **84 REMOVED + 12 MODIFIED + 6 ADDED = 102**.
  - Core arrivals: 84 removals + 7 new core-half titles = **91** (82+7=89 was the drafted figure).
  - Donor retained: (136 − 84) untouched-or-modified 52 + 6 ADDED = **58** — per-cap matches tasks 8.2 exactly (input 2 = Output-Dir-Resolution retained + Input-Argument-Structs ADDED; timeline 10).
  - Union 91 + 58 = 149 = 136 + 13. ✓
  - subx-core: 13 → 25 caps; 96 → 187 titles (measured 96; proposal said 93→182).
- **8.1 title-set equality needs a rename map**: removed `Library and Binary Error Surface Split` arrives renamed as new-half `Library Error Surface Holds Only Machine Contracts` (arriving titles: verbatim 7 + renamed 1 + `Display Is the Library's Error Rendering` + `Library Code Surfaces Recoverable Failures as Errors` = 10). `Task Scheduler Entry Point` and `Sync Method Selection` arrive under their own titles. Formula: 84 removals→84 arriving (one renamed) + 7 new = 91.

## Drifted citations (fix design.md Decision 2 + Migration Plan step 2/3)
- match_command.rs: execute_parallel_match :682→**:702**; "No video files found" :718→**:738**; monitor_batch_execution :765→**:781**; archive-origin branch :459→**:486**; allocator call :470→**:490**.
- apply_unique_target_paths defn: subx-core/src/core/matcher/engine.rs:134→**:139**; cache join site engine.rs:2244→**:2541-2544** (dirs::config_dir); cache_command.rs get_config_dir :217, cache path :226-227 (manual XDG_CONFIG_HOME) — disagreement-by-construction still holds.
- translation engine "Processed cues:" engine.rs:405→**:415**.
- config service env-overrides block :222-251→**:287-460** (re-verify range covers SUBX_AI_* application); :31,39,43 permission lines OK; masking.rs:19 OK; match_args.rs:23-25,53 OK; translate_command.rs:163-174 OK.
- Output Directory Resolution for Archive Files: verified still substantive CLI — convert delegates to CollectedFiles::default_output_path (converted), sync keeps conditional Option form (:520,:595) deliberately, translate joins its own name on default_output_dir + refuses --replace (:332). Requirement retained untouched per proposal.

## Tooling policy (deviation declared up front)
- Search: grep/ast-grep tools. Edits to EXISTING files: apply_patch (edit tool) — proposal.md, design.md, tasks.md, the 13 donor delta files, donor main specs (MODIFIED substitution, ADDED append, Purpose/H1 repairs), AGENTS.md, CHANGELOGs, archived core spec repairs.
- New large files (12 core ADDED deltas + local-llm-provider delta): python **byte-move assembler** — verbatim blocks copied from donor main specs by exact title match, scripted deterministic Migration-note edits (subx-cli: qualifiers, path de-qualifications); composed halves hand-written by me into scratch blocks and spliced. Guards: per-file zero-duplicate-title check, arriving-title-set equality vs 3.1(+2) list, sweeps. Rationale: C2a's bug was append-vs-substitute logic, not scripting; retyping ~2500 verbatim lines invites corruption; every byte is guarded after assembly.
- Donor deletions (84): scripted delete by exact `### Requirement: <title>` match, count-guarded (per-cap expected counts), never append. MODIFIED: substitute in place from delta body. ADDED: append under ## Requirements.

## Phases
A. Baseline tasks 1.1–1.10 recorded (1.1 green both roots; 1.10 seam: proceed whole).
B. Documentation corrections (apply.md step 2): two new REMOVED delta entries; proposal/design/tasks numeric re-derivation + drifted line numbers (per plan.md numbers above). tasks.md: 3.1 82→84/89→91+rename note; 3.9 input 13 (6 composed, 7 verbatim incl. Archive-Aware de-qualified); 3.14 timeline 10 (6 verbatim + VAD-Independent verbatim-dequalified + 3 composed); 4.1 →48 notes; 6.2 84 deletions; 8.1 84+7=91; 1.x evidence fixes.
C. Archive policy: subx-core receiving side `openspec archive import-split-capability-specs -y` DURING apply (C2a precedent — only mechanism that populates specs/); donor NOT archived — hand-apply all 102 ops; tasks.md 6.1 rewritten as deferred instruction: `openspec archive split-mixed-capabilities-across-repos -y --skip-specs` at archive stage (deltas already applied, do not re-apply). Probe: `openspec validate --all --strict` stays green with live change whose REMOVED targets are gone (C2a precedent; if not → message Main).
D. Core side (tasks 2–5): framing artifacts (.openspec.yaml 2 lines, proposal 4 H2s, design 4 decisions, tasks phases + own quality script); 13 delta files; 4.x edits (48 Migration notes, 4.2 qualifiers, 4.3 non-qualifiers, 4.4 line ranges, 4.5 Local Provider un-qualify (must differ from both pre/post-C2a text), 4.6 14 composed, 4.7 7 new halves, 4.8 +2 scenarios, 4.9 both-tree scenario, 4.10 extension scenario, 4.11 precondition phrasing, 4.12 sweep, 4.13 strict validate); archive → 12 H1 + 12 Purpose repairs, blank lines, TBD zero, local-llm-provider zero-subx-cli-hits, validate --specs --strict 25/25, dedup guard, commit + push origin main.
E. Donor side (tasks 6–7): 84 deletions, 12 substitutions, 6 appends, 12 Purpose rewrites (own-repo paths only, prose pointer to core half; component-factory worked example), subtitle-translation H1 fix, split-capabilities.txt (12 lines sorted, no SH), dedup guard, sweeps, validate --specs --strict 20/20, commit + gitlink bump.
F. Verify (tasks 8.x): 91/58 title equalities (with rename map), per-cap disjointness (12 caps), TBD zero, split record bidirectional, `git diff --stat HEAD~1 -- . ':!openspec'` empty in both.
   - **8.5 sweep scoping deviation**: require zero unqualified `src/core/|src/services/|src/config/|src/error.rs` hits in the 13 touched capability files (only `subx-core:`-prefixed). Wider tree (`crate-topology`, `machine-readable-output`, `progress-reporting`, `spec-governance` examples) has pre-existing B2-era citations untouched by this change → enumerate as known debt + C3 handoff, do NOT edit (scope). Record honestly in tasks.md/report.
   - 8.9/8.10 fresh-clone: attempt locally (clone file:// worktree + submodule); if submodule recursion is awkward, use `--reference`/direct clone; must actually run.
G. Docs (9.x): AGENTS.md 3 paragraphs; both CHANGELOGs; 9.5–9.7 handoffs into design.md Open Questions (live file moves with archive).
H. tasks.md checkbox pass with honest evidence (10.1–10.4 unchecked = main agent; 6.1 unchecked = deferred archive task).
I. Rubber-Duck #2 on finished changes → address → re-verify.

## Assumptions challenged (RD#1 targets)
1. Classifying the 2 late additions as CORE and folding them into this change's deltas; tasks 8.2 per-cap corroborates.
2. Receiving-side archive during apply; donor archive deferred per coordinator steer.
3. `openspec validate --all --strict` tolerates a live change whose REMOVED targets no longer exist in main specs.
4. Composed halves: every sentence/scenario of the 23 divided originals lands in exactly one half (two deliberate duplicates: error-handling Display sentence; parallel-processing "Parallel match over a directory" scenario).
5. Byte-move assembler + guards is the safe execution vehicle at this scale; all judgment edits via apply_patch/write.
6. Donor `src/` has only commands/, cli/, lib.rs, main.rs (crate-topology) — so donor-side unqualified src/core cites are already dead pointers post-B2; hence 8.5 scoping.
