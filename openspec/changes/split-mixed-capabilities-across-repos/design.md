## Context

After C2a, `subx-cli/openspec/specs/` holds 20 capability directories. Seven of them are honestly owned by `subx-cli` (`machine-readable-output`, `progress-reporting`, `release-distribution`, `shell-completions`, `crate-topology`, `cross-crate-testing`, `spec-governance`). The other thirteen are the ones C2a deliberately did not touch: capabilities whose requirements are implemented on both sides of the crate boundary. They hold **134 requirements**, more than half the requirement count of the whole project, and every one of them that governs `src/core/**`, `src/services/**`, `src/config/**` or `src/error.rs` is currently a specification of another repository's code, sitting in this one.

C2a's contribution was not the move; it was the rule. `spec-governance` now states, normatively, that a requirement has exactly one owning repository, how that owner is decided, what is *not* evidence for it, that a capability with requirements on both sides is expressed as two capabilities of the same name, that an unqualified path resolves against the owning repository, and that a `## Purpose` names only its own side's paths. This change is the application of that rule to the thirteen. Almost everything difficult about it is classification, and almost everything risky about it is the archive procedure C2a already characterised.

Three constraints carried forward from C2a shape the work, and one is new.

**Carried: OpenSpec has one root per invocation.** There is no cross-root delta. The work is therefore two changes, this one and `import-split-capability-specs` in `subx-core`.

**Carried: `openspec archive` cannot perform the sending half.** A delta that removes every requirement of a capability rebuilds a spec with an empty `## Requirements` section and fails; the failure is whole-change atomic. C2a verified this against `openspec` 1.6.0 and established the procedure: `--skip-specs` plus hand application of every delta.

**New, and specific to a split: no capability in this change is emptied.** All twelve split capabilities keep a CLI half, so no `openspec/specs/<cap>/` directory is deleted and none of the twelve deltas removes *every* requirement of its capability. That means `openspec archive` would not necessarily refuse — the rebuilt specs are non-empty. It is used with `--skip-specs` anyway, for a different reason: the archive cannot rewrite twelve `## Purpose` paragraphs, and a spec whose Purpose cites the other repository's paths is a `spec-governance` violation that `validate --strict` does not detect. Applying the thirteen deltas mechanically and then hand-editing twelve Purposes in the same files is a worse workflow than applying all thirteen by hand, because the intermediate state is neither the old tree nor the new one. Decision 9 records this and the verification that replaces the tool's guarantee.

**New: this change is the first to produce a capability name in both trees.** C2a's task 8.2 asserts capability-name disjointness and notes "at this commit the split set is empty; C2b creates the first entries." Twelve names will be in both trees on purpose, and the drift check that C2a specified cannot distinguish those from an accident unless the deliberate set is written down. Decision 10 writes it down.

## Goals / Non-Goals

**Goals:**

- Classify all 134 requirements individually, from the requirement text and the code it cites, and record the classification where the next reader can check it.
- Apply `spec-governance`'s ownership test as written, including its list of what is *not* evidence, and report the cases where SDR §9's three CLI-seam families gave the wrong answer.
- Split only the requirements that genuinely straddle, and split them so that the union of the two halves is exactly the original obligation — no gap, no overlap.
- Discharge the three hand-offs C2a left: the two demoted capabilities, `local-llm-provider`'s three references, and the recorded split set.
- Leave both trees passing `openspec validate --strict`, with no unqualified cross-repository citation and no `## Purpose` naming the other side.
- Measure the change honestly and name a seam if it does not fit, rather than overflowing silently.

**Non-Goals:**

- Changing the meaning of any requirement. Fifteen are expressed as two requirements instead of one; none is weakened, strengthened, renamed for taste, merged or deleted. The one added scenario and the one retired scenario are enumerated in Decision 3 and Decision 7.
- De-duplicating the pre-existing overlap between `secrets-protection`'s masking requirement and `configuration-management`'s *Sensitive value masking in config display*. It predates this change and is intra-repository, which `spec-governance` does not forbid. Open Questions.
- Fixing the six API gaps of SDR §8. Three of them (convert output-path resolution, match archive-origin rewrite, progress hooks) are exactly why some requirements classify CLI in this change. Moving the logic is D1/D2's work; this change specifies where the logic *is*, not where it should be.
- Implementing the drift check as running code. This change creates the record it reads; C3 places the check, per C2a's first Open Question.
- Editing any source file, manifest, workflow, script or test in either repository.
- Splitting `supply-chain-hardening`. Decision 5.
- Re-arguing C2a's classification of the thirteen wholesale-core capabilities, or moving `crate-topology`, `cross-crate-testing` or `spec-governance`.

## Decisions

### Decision 1: the classification rule applied, and the three cases where SDR §9's heuristic families misled

`spec-governance`'s *Every Requirement Has Exactly One Owning Repository* gives a three-step test — changed files, then the CLI-only dependency test, then scenario citations — plus a list of three things that are **not** evidence. That test is what was applied, and applying it means reading a requirement's normative prose to find its *object*, then locating that object in the source tree, then checking whether the requirement's remaining obligations attach to the same side. SDR §9 additionally offers a shortcut: the CLI seam inside a MIXED capability is usually (a) an "`X` Command Interface" / flags requirement, (b) an "Emits Structured JSON Payload" requirement, or (c) a dry-run / progress / table-rendering requirement.

The shortcut is a good prompt and a bad rule. It held for every (b): all five *Emits Structured JSON Payload* requirements are CLI, without exception, because the envelope is assembled in `src/cli/output.rs` and rendered by the command. It held for most (a). It failed three times, twice in each direction, and the failures are the reason the whole tree was read rather than screened.

**(c) misled toward CLI: `subtitle-translation`'s *Batching and Ordering*.** Its second scenario reads "the command SHALL log the number of processed cues and total cues in the form `Processed cues: <processed>/<total>`" — textbook family (c) vocabulary. The string is at `src/core/translation/engine.rs:405`, inside a `format!` that A1 routes through the `Reporter` seam. The requirement's other scenarios are batch splitting and output ordering, also `src/core/translation/`. It is wholesale core, and a screen for progress vocabulary would have sent it to the CLI half where the GUI — which constructs `TranslationEngine::new` directly (`../subx/src-tauri/src/commands/translate.rs:489`) — could not find it.

**(c) misled toward CLI: `timeline-sync`'s *VAD Padding Chunks Configuration*.** No progress vocabulary, but it reads as a user-facing knob (`sync.vad.padding_chunks`, default 3) and sits between two requirements about CLI overrides. Its object is the `vad.label(..., padding_chunks, ...)` call in `src/services/vad/detector.rs` and the `VadConfig` struct in `src/config/mod.rs`. Both core. Adjacency in a file is not evidence, which is a corollary of `spec-governance`'s "a capability's title, its `## Purpose` wording" exclusion that is worth stating out loud.

**(a) misled toward core: `format-conversion`'s *Supported Output Formats*.** It does not read like a command-interface requirement — it reads like a statement about the conversion pipeline, and its scenario is about output file contents. But its normative object is `OutputSubtitleFormat`, and `OutputSubtitleFormat` is defined in `src/cli/convert_args.rs`, a clap enum that D8 keeps permanently out of core. Half the requirement (the accepted `--format` value set, the matching extension, the `formats.default_output` fallback resolved in `convert_command.rs`) is CLI; half (SRT→VTT producing a `WEBVTT` header and dot timecodes) is `src/core/formats/`. It is split.

**The narrative-framing exclusion did most of the work, and it is the rule most easily got wrong.** `spec-governance` says a scenario that reaches core behaviour through `subx config set …` is not ownership evidence, and that a prose sentence describing where a value originates is not either. Nineteen requirements in `configuration-management`, `input-path-handling` and `subtitle-matching` are phrased as user invocations over core objects. *Value Validation*'s scenario is "the user runs `subx config set sync.max_offset_seconds -5`"; its object is the field validator in `src/config/field_validator.rs`. *Recursive vs Flat Traversal*'s scenario says "`--recursive` passed"; its object is `collect_files`. All nineteen are core, and a reading that took the invocation at face value would have left almost all of `configuration-management` in `subx-cli`.

**The converse trap, and how it was avoided.** The exclusion cuts only one way: an invocation in the GIVEN is not evidence of CLI ownership, but a *print*, an *exit code*, an *argument-parsing failure* or a *flag definition* in the THEN is evidence, because those are obligations no core file can discharge. The operational form of the test used throughout: **read the THEN clause and ask which file would have to change if it became false.** `subtitle-matching`'s *Confidence Threshold Enforcement* has two scenarios; the first THEN is "the engine SHALL omit that candidate", the second is "argument parsing SHALL fail with a validation error from `clap`". Different files, so the requirement is split. Different files, so the requirement's content is divided. That single question resolved twenty-two of the twenty-three divisions; only case 5 below needed more than it.

**Two mechanical screens were run as a cross-check, not as the decision.** A grep for `clap`, `clap_complete`, `colored`, `tabled` and `indicatif` across the thirteen returns hits in `subtitle-matching` (`clap`), `subtitle-matching`'s JSON requirement (`src/cli/table.rs`) and `subtitle-translation` (`clap-based CLI conventions`) — all of which the reading had already classified CLI. A grep for `src/cli/`, `src/commands/` and `src/main.rs` returns hits in every one of the thirteen, which is the definition of the MIXED list and tells you nothing about *which* requirements. Neither screen would have found the three misclassifications above, because none of them contains a CLI-only crate name or a `src/cli/` path in the misleading half.

**Corrections to C2a's hand-off.** C2a Decision 1 named the CLI seams in the two capabilities it demoted. `component-factory`'s is confirmed exactly: *Commands Consume Services via Dependency Injection*, and nothing else — *Tests Use TestConfigService via TestConfigBuilder* is core, because its unit-test scenario cites `src/core/factory.rs` and all three integration tests it names are core-bound under B3's rule 3 (`tests/openrouter_integration_tests.rs` and `tests/azure_openai_api_integration_tests.rs` import only `config`, `core` and `services`; `tests/dependency_injection_integration_tests.rs` is listed core-side at B3 tasks 5.5 and B3 design line 137). `parallel-processing`'s is one requirement larger than C2a recorded: *Task Scheduler Entry Point* is split, on the evidence in the proposal. Neither correction changes any other decision; both are recorded because C2a recorded its corrections to SDR §9 for the same reason.

**One inconsistency in B3 is noted and not resolved.** B3 classifies `tests/dependency_injection_integration_tests.rs` as core-bound, but the file imports `subx_cli::App` (line 11, used at `:27`, `:46`, `:63`, `:64`, `:112`), and `App` stays in `subx-cli`'s slim `lib.rs` under SDR §2.2 — so B3's rule 3 ("names only `config`, `core`, `error`, `services` or `Result`") does not strictly fire for it. The classification of *Tests Use TestConfigService via TestConfigBuilder* does not depend on it: its two scenarios cite `src/core/factory.rs`, `tests/openrouter_integration_tests.rs:13-24` and `tests/azure_openai_api_integration_tests.rs:186`, all unambiguously core. The `**Migration**` note therefore qualifies only `docs/testing-guidelines.md` and leaves the three test paths unqualified, with a line recording that if B3 or B4 re-files the DI test, that one citation becomes `subx-cli:`. Raised in Open Questions.

### Decision 2: the per-requirement classification, with evidence

The tables below are the substance of this change. **C** = core, migrates to `subx-core`; **L** = CLI, stays in `subx-cli`; **S** = split, with the two halves named in Decision 3. Line citations were checked against the tree as it stands; where A1, A2 or C1 will have moved a construct the citation is to the construct, not the line.

**`cache-management` — 6 C / 2 L**

| Requirement | | Evidence |
|---|---|---|
| Match Cache Location | C | `save_file_list_cache` and the path join at `src/core/matcher/engine.rs:2244`; the GUI reads the artefact |
| Cache Clear Subcommand | L | `CacheAction::Clear` in `src/cli/cache_args.rs`; the two print strings in `src/commands/cache_command.rs` |
| Configuration-Aware Invalidation | C | `CacheData` fields written by `save_file_list_cache` |
| Cache Reuse Preserves Relocation Mode | C | named "Implemented in `src/core/matcher/engine.rs`"; its two test citations are CLI-bound |
| Dry-Run Cache Reuse Without AI Calls | C | `check_file_list_cache` / `save_file_list_cache`; test citation is CLI-bound |
| Cache Invalidation On Relocation-Affecting Config Change | C | `src/core/matcher/engine.rs::calculate_config_hash` (`:2248`) |
| Cache Scoped To File-List Directory Key | C | the `filelist_<hash>` key in `src/core/matcher/engine.rs` |
| Cache Subcommands Emit Structured JSON Payloads | L | family (b); `--json` alias on `StatusArgs` in `src/cli/cache_args.rs` |

**`component-factory` — 6 C / 1 L**

| Requirement | | Evidence |
|---|---|---|
| ConfigService-Driven Construction | C | `ComponentFactory::new`, `src/core/factory.rs` |
| AI Provider Creation | C | dispatch over `src/services/ai/` |
| Pre-Construction Configuration Validation | C | `validate_ai_config` in `src/core/factory.rs` |
| Match Engine Creation | C | `create_match_engine`, `MatchConfig` |
| VAD and Audio Component Creation | C | `create_vad_sync_detector` etc., `src/services/vad/` |
| Commands Consume Services via Dependency Injection | L | `src/commands/match_command.rs:176-213`, `sync_command.rs:168-173`, `convert_command.rs:203-205` |
| Tests Use TestConfigService via TestConfigBuilder | C | unit scenario cites `src/core/factory.rs`; all three integration tests core-bound (Decision 1) |

**`configuration-management` — 14 C / 4 L (1 split)**

| Requirement | | Evidence |
|---|---|---|
| Unified Configuration Schema | C | `Config` and the five sub-structs, `src/config/mod.rs` |
| Configuration Service Abstraction | C | the `ConfigService` trait and both impls; the command scenario is narrative framing whose CLI obligation is already stated by `component-factory`'s *Commands Consume Services…* — see Decision 3 on why this is **not** split |
| `config` Subcommand Operations | L | `src/cli/config_args.rs`, `src/commands/config_command.rs` |
| Repair Path For Strict-Invalid Configuration | S | tolerant load in `src/config/service.rs`; stderr advisory and `Envelope::warnings` in the command |
| Value Validation | C | `src/config/field_validator.rs` |
| Boolean Value Flexibility | C | `src/config/field_validator.rs` |
| AI Environment Variable Overrides | C | `src/config/service.rs:222-251` |
| Custom Configuration File Path | C | `SUBX_CONFIG_PATH` handling in `ProductionConfigService` |
| Legacy Sync Configuration Rejected | C | the `SyncConfig` schema; `tests/config_migration_tests.rs` core-bound |
| Config Service Reload | C | `ProductionConfigService::reload` |
| Workspace Directory Override | L | named "Implemented in `src/cli/mod.rs`" |
| Compatibility Environment Variables For Third-Party Providers | C | named "Implemented in `src/config/service.rs`" |
| AI Provider Identifier Canonicalization | C | `normalize_ai_provider` in `src/config/field_validator.rs`; the five call sites are `field_validator.rs`, `service.rs`, `validator.rs`, `factory.rs` — all core |
| Local Provider Validation Rules | C | `validate_ai_config` in `src/config/validator.rs`; carries C2a's qualification, which is removed on arrival (Decision 7) |
| Local Provider Environment Variables | C | `ProductionConfigService` env application |
| Config file permissions enforcement | C | `src/config/service.rs:31,39,43` |
| Sensitive value masking in config display | L | the display sites in `src/commands/config_command.rs`; the helper is `secrets-protection`'s core half |

**`encoding-detection` — 2 C / 6 L (1 split)**

| Requirement | | Evidence |
|---|---|---|
| Per-File Encoding Report | L | the four print lines in `src/commands/detect_encoding_command.rs` |
| Input Source Selection | L | positional vs `-i` conflict declared in `src/cli/detect_encoding_args.rs` |
| Verbose Sample Output | L | `--verbose` truncation in the command |
| Robust Handling of Empty and Binary Files | S | detector must not panic (core); batch must not abort and must exit 0 (CLI) |
| Low-Confidence Fallback To Default Encoding | C | `src/core/formats/encoding/detector.rs::select_best_encoding` |
| Legacy Positional File Paths Accepted | L | `DetectEncodingArgs.file_paths`, `src/cli/detect_encoding_args.rs` |
| Detect-Encoding Command Emits Structured JSON Payload | L | family (b) |

**`error-handling` — 10 C / 6 L (3 splits), counted against the post-A2 end state**

| Requirement | | Evidence |
|---|---|---|
| Typed Error Taxonomy | C | the enum and constructors in `src/error.rs`; the GUI consumes six variants |
| Automatic Error Conversions | C | the `From` impls in `src/error.rs` |
| Chained Error Sources | C | `#[from]` / `#[source]` in `src/error.rs` |
| User-Facing Error Formatting | S | `Display` is inherent and core; `user_friendly_message()` is A2's `src/cli/error_ext.rs` |
| Process Exit Code Mapping | L | A2 moved `exit_code()` to `src/cli/error_ext.rs` |
| Top-Level Error Rendering | L | `src/main.rs` |
| API Error Source Enumeration | C | `ApiErrorSource`, `whisper_api` |
| No Panics On Recoverable Errors | S | config loader and match engine (core); command entry points and the rendering pipeline (CLI) |
| Sanitized upstream error messages | C | the 500-char truncation in `src/services/ai/` |
| No sensitive data in error chains | C | URL redaction in `src/services/ai/`; `local-llm-provider` references this requirement |
| Stable Machine-Readable Category and Code | C | `category()`, `machine_code()`, `hint()` inherent on `SubXError`; `category()` is load-bearing for the GUI (SDR §8) |
| Process-Boundary Rendering Honors Output Mode | L | `src/main.rs` and `src/cli/output.rs::ErrorEnvelope::from_error` |
| Library and Binary Error Surface Split | S | defines both halves by construction |

**`format-conversion` — 5 C / 5 L (1 split)**

| Requirement | | Evidence |
|---|---|---|
| Supported Output Formats | S | `OutputSubtitleFormat` in `src/cli/convert_args.rs` (CLI); conversion output shape (core) |
| Input and Output Path Resolution | L | `-i` / `--output` flags and the output-path computation in `src/commands/convert_command.rs` |
| Original File Preservation | L | `--keep-original` and the source removal in the command |
| Per-File Error Isolation | L | the batch loop and the stderr message in the command |
| File size check before parsing | C | `general.max_subtitle_bytes` enforced under `src/core/formats/` (see `src/core/formats/srt/parser.rs:19`) |
| Parser robustness on malformed input | C | the four parsers under `src/core/formats/` |
| Parser/serializer round-trip stability | C | `tests/fixtures/formats/**`, which B3 moves to `subx-core` at the identical relative path |
| Public format API stability across module reorganization | C | `crate::core::formats`; needs one migration note because "other modules in `subx-cli`" now reach it through the D11 re-export |
| Convert Command Emits Structured JSON Payload | L | family (b) |

**`input-path-handling` — 12 C / 2 L (4 CLI clauses gathered into 1), counted against the post-A2 end state**

| Requirement | | Evidence |
|---|---|---|
| Core-Owned Input Collection | C | A2 defines it as `src/core/input/mod.rs` and forbids `clap` there |
| Unified Path Merging | C | `merge_paths_from_multiple_sources`; the "thin adapter" clause is gathered (Decision 4) |
| Extension Filtering | C | `with_extensions`; the per-command whitelist clause is gathered |
| Recursive vs Flat Traversal | C | `scan_directory_flat` / `scan_directory_recursive`; `--recursive` is qualified, not moved |
| Direct File Inputs Pass Through | C | `extract_and_collect` and the archive extension set |
| Mixed File And Directory Inputs | C | `collect_files`; two CLI-bound test citations qualified |
| Directory Deduplication | C | A2 names `src/core/input/mod.rs`; one CLI-bound test citation qualified |
| Invalid Path Surfacing | C | `SubXError::InvalidPath` from `collect_files` |
| CollectedFiles Return Type | C | `CollectedFiles`, its `Deref`, its `TempDir` handles |
| No-Extract CLI Flag | C | A2 already carves it: `with_no_extract` is core, the flag is CLI. Core half retitled *No-Extract Collection Switch*; CLI bullet gathered |
| Archive Origin Mapping | C | `CollectedFiles::archive_origin` |
| Output Directory Resolution for Archive Files | L | `src/commands/convert_command.rs:362`, `match_command.rs:459`, `sync_command.rs:512,585`, `translate_command.rs:330-389`; SDR §8 gaps 4 and 5 exist *because* this is not in core |
| CollectedFiles Additional APIs | C | `into_paths`, `AsRef<[PathBuf]>`; the `DetectEncodingArgs::get_file_paths` sentence is gathered |

**`parallel-processing` — 8 C / 3 L (1 split)**

| Requirement | | Evidence |
|---|---|---|
| Task Scheduler Entry Point | S | `TaskScheduler::new()` (core); the task-count report and `No video files found to process` at `src/commands/match_command.rs:718` (CLI) |
| Bounded Concurrency | C | `get_active_workers`, `src/core/parallel/` |
| Aggregated Result Reporting | L | `monitor_batch_execution` at `src/commands/match_command.rs:765` and the summary prints at `:739-760` |
| Progress Reporting Opt-Out | L | `ProgressDrawTarget` is `indicatif`, CLI-only by D8; `src/commands/match_command.rs:729-737` |
| Batch Task Submission | C | `submit_batch_tasks`; `tests/parallel_processing_integration_tests.rs` core-bound |
| Task Queue Overflow Strategy | C | `src/core/parallel/scheduler.rs::submit_task_with_priority` |
| Optional Task Priority Ordering | C | `TaskPriority` in `src/core/parallel/scheduler.rs` |
| Non-blocking I/O in async executor | C | `spawn_blocking` in the task executors |
| Active task accounting correctness | C | the RAII guard in `src/core/parallel/scheduler.rs` |
| UUIDv7 Worker and Task Identifiers | C | `src/core/parallel/{worker,scheduler}.rs`; the `uuid` features clause becomes `subx-core`'s manifest per SDR §4 |

**`secrets-protection` — 4 C / 1 L (1 split)**

| Requirement | | Evidence |
|---|---|---|
| Mask sensitive config values in CLI output | S | `mask_sensitive_value` at `src/config/masking.rs:19` (core); the `config set`/`list`/`get` display (CLI) |
| Redact API keys in Debug output | C | the three AI clients under `src/services/ai/` and `AIConfig` in `src/config/mod.rs` |
| Restrict config file permissions | C | `src/config/service.rs:31,39,43` |
| Warn on insecure HTTP endpoint | C | the client constructors under `src/services/ai/` |

**`subtitle-matching` — 6 C / 4 L (3 CLI halves gathered into 2)**

| Requirement | | Evidence |
|---|---|---|
| AI-Based File Pairing | C | `MatchEngine::match_file_list`, `AnalysisRequest`; the `No files found to process` scenario is at `src/commands/match_command.rs:446` and is gathered |
| Confidence Threshold Enforcement | C | the 0.0–1.0 filter in the engine; the clap range check at `src/cli/match_args.rs:23-25,123-124` is gathered |
| Dry-Run and Execution Modes | L | the mode selection and the planned-operation display in `src/commands/match_command.rs`; needs restatement because it names `execute_operations` |
| File Relocation Modes | C | `FileRelocationMode` and the copy/move execution in the engine; the mutual-exclusion message at `src/cli/match_args.rs:53` is gathered |
| Optional Backup Before Move | C | the backup task in the engine / `src/core/file_manager.rs` |
| Per-Scan Unique UUIDv7 File Identifiers for Matching | C | `src/core/matcher/discovery.rs::generate_file_id`; `tests/match_engine_id_integration_tests.rs` core-bound |
| Match Command Emits Structured JSON Payload | L | family (b); cites `src/cli/table.rs` |
| AI-Driven Language and Globally-Unique Target Naming | C | `generate_subtitle_name` and `apply_unique_target_paths` at `src/core/matcher/engine.rs:134`; the GUI consumes the allocator. The call-order obligation at `src/commands/match_command.rs:459-470` becomes a separate CLI requirement |

**`subtitle-translation` — 7 C / 6 L (1 split)**

| Requirement | | Evidence |
|---|---|---|
| Translate Command Interface | L | family (a); `src/cli/translate_args.rs` |
| Subtitle Input Collection | L | the subject is the translate command's wiring; the collection algorithm itself is `input-path-handling`'s core half |
| Subtitle Structure Preservation | C | `src/core/translation/` over `src/core/formats/` |
| AI Provider Translation | C | `TranslationEngine` and `src/services/ai/` |
| Stable Cue Mapping | C | UUIDv7 cue IDs and response validation in `src/core/translation/engine.rs` |
| Two-Pass Terminology Consistency | C | the extraction pass and prompt assembly in `src/core/translation/` and `src/services/ai/` |
| Batching and Ordering | C | batch splitting, reassembly, and the `Processed cues:` format at `src/core/translation/engine.rs:405` — the family (c) false positive of Decision 1 |
| Translation Guidance Options | S | `parse_glossary_text` in `src/core/translation/engine.rs` (core); the flags and the glossary file read at `src/commands/translate_command.rs:163-174` (CLI) |
| Translation Configuration | C | the translation config struct and its validator under `src/config/` |
| Safe Output Behavior | L | output naming, overwrite refusal and replace-mode backup in `src/commands/translate_command.rs` |
| Per-File Error Isolation | L | the per-file loop in `src/commands/translate_command.rs` |
| Documentation Coverage | L | `docs/command-reference.md`, `README.md`, `README.zh-TW.md` |

**`supply-chain-hardening` — 0 C / 8 L, counted against the post-A0/post-C1 end state (Decision 5)**

| Requirement | | Evidence |
|---|---|---|
| Replace unmaintained md5 crate | L | a manifest rule; restated as per-manifest |
| Narrow dependency feature flags | L | a manifest rule over `tokio` (both manifests) and `symphonia` (core's); restated as per-manifest |
| CI cargo audit gate | L | C1's end state already governs the workspace lockfile and both repositories |
| Every Declared Dependency Has a Use Site | L | A0 states it per-manifest for a multi-crate project |
| Dependency Manifest Layout and Comment Language | L | per-manifest |
| Published Crate Contains No Unreachable Source Files | L | per-manifest |
| Submodule Pointer Is a Supply-Chain Input | L | superproject only |
| Each Published Crate Is Audited in Its Own Repository | L | a statement about the pair |

**`timeline-sync` — 9 C / 10 L (2 CLI halves gathered into 1), counted against the post-A2 end state**

| Requirement | | Evidence |
|---|---|---|
| Sync Method Selection | C | `sync.default_method` fallback and `SyncEngine::new`'s unconditional VAD requirement; the `--method` flag and `Manual method requires --offset parameter.` are gathered |
| Offset Clamping Against Maximum | C | `apply_manual_offset` and `vad_detect_sync_offset` in `src/core/sync/engine.rs` |
| Subtitle Timing Application | C | the entry shift and zero clamp in `src/core/sync/engine.rs` |
| Single-File and Batch Modes | L | the flag surface and `SyncArgs::validate`; needs restatement because it names `crate::core::sync::resolve_sync_pairing` |
| Dry-Run Mode | L | family (c); the command's print-without-write path |
| Manual Offset Without Video | L | validation and the command flow; the core relaxation is *Core-Owned Sync Pairing Resolution* step 5 |
| Force Overwrite of Existing Output | L | named "Implemented in `src/commands/sync_command.rs`" |
| Batch Prefix-Match Pairing | L | A2 states the batch pairing heuristic is "a separate, command-level concern" inside `src/commands/sync_command.rs` |
| Batch Skip Directories Without Videos | L | the `✗ Skip sync for …` message in the command |
| Batch Single-Pair Override | L | the same command-level pairing block |
| CLI VAD Parameter Overrides | L | `--vad-sensitivity` / `--window` in `src/cli/sync_args.rs` |
| VAD Audio Processing | C | `VadAudioProcessor`, `src/services/vad/`; both cited tests core-bound |
| VAD Detector Behavior | C | `LocalVadDetector`, `VadSyncDetector` |
| First-Sentence Offset Annotation | C | `SyncResult.additional_info`; `tests/sync_first_sentence_offset_integration_tests.rs` core-bound |
| VAD Padding Chunks Configuration | C | `src/services/vad/detector.rs`, `src/config/mod.rs::VadConfig` — the second family (c) false positive |
| Sync Command Emits Structured JSON Payload | L | family (b) |
| Core-Owned Sync Pairing Resolution | C | A2 defines `resolve_sync_pairing` in `src/core/sync/mod.rs`; the `SyncArgs::get_sync_mode` adapter paragraph is gathered |
| Core-Owned Default Output Path Derivation | C | A2 defines `create_default_output_path` in `src/core/sync/mod.rs`; the `crate::cli::sync_args` legacy re-export clause is gathered |

### Decision 3: the twenty-three requirements whose content is divided, and how the halves avoid a gap and an overlap

A requirement's content is divided only when the answer to "which file must change if this becomes false" is *both repositories*, and when neither obligation can be dropped without losing something the union previously guaranteed. Twenty-three qualify. For each, the invariant enforced is: **every sentence and every scenario of the original appears in exactly one half**, and each half's prose states only obligations its own repository can discharge.

**Nine become one requirement per repository** — a genuine 1→2 split, +9 requirements:

| # | Original | `subx-core` half | `subx-cli` half |
|---|---|---|---|
| 1 | `configuration-management` *Repair Path For Strict-Invalid Configuration* | **Tolerant Configuration Load Path** (new title) | *Repair Path For Strict-Invalid Configuration* (keeps title) |
| 2 | `encoding-detection` *Robust Handling of Empty and Binary Files* | **Detector Tolerates Empty and Binary Input** | *Robust Handling of Empty and Binary Files* |
| 3 | `error-handling` *User-Facing Error Formatting* | **Display Is the Library's Error Rendering** | *User-Facing Error Formatting* |
| 4 | `error-handling` *No Panics On Recoverable Errors* | **Library Code Surfaces Recoverable Failures as Errors** | *No Panics On Recoverable Errors* |
| 5 | `error-handling` *Library and Binary Error Surface Split* | **Library Error Surface Holds Only Machine Contracts** | **Binary Error Surface Adds Presentation Through an Extension Trait** |
| 6 | `format-conversion` *Supported Output Formats* | **Target Format Conversion Semantics** | *Supported Output Formats* |
| 7 | `parallel-processing` *Task Scheduler Entry Point* | *Task Scheduler Entry Point* (keeps title) | **Parallel Match Reports Task Count and Handles an Empty Input Set** |
| 8 | `secrets-protection` *Mask sensitive config values in CLI output* | **Sensitive Value Masking Helper** | *Mask sensitive config values in CLI output* |
| 9 | `subtitle-translation` *Translation Guidance Options* | **Translation Prompt Guidance Inputs** | *Translation Guidance Options* |

**Fourteen contribute one CLI clause apiece into four gathered requirements** — +4 requirements, Decision 4:

| Capability | Requirements whose CLI clause is lifted | Gathered into |
|---|---|---|
| `input-path-handling` (6) | *Core-Owned Input Collection*, *Unified Path Merging*, *Extension Filtering*, *Recursive vs Flat Traversal*, *No-Extract CLI Flag* (core half retitled **No-Extract Collection Switch**), *CollectedFiles Additional APIs* | **Input Argument Structs Are Thin Adapters Over Core Collection** (5 bullets) |
| `timeline-sync` (3) | *Sync Method Selection*, *Core-Owned Sync Pairing Resolution*, *Core-Owned Default Output Path Derivation* | **Sync Argument Struct Is a Thin Adapter Over Core Pairing** (4 bullets) |
| `subtitle-matching` (5) | *AI-Based File Pairing*, *Confidence Threshold Enforcement*, *File Relocation Modes*, *Optional Backup Before Move* → **Match Command Argument Surface and Input Preconditions** (4 bullets); *AI-Driven Language and Globally-Unique Target Naming* → **Match Command Applies Archive-Origin Relocation Before Uniqueness Allocation** | two requirements |

Every one of the fourteen keeps its original title on the core side, except *No-Extract CLI Flag*, whose title stops being true once the flag clause leaves it.

**Which side keeps the original title.** The half that still answers the question the title asks. *Supported Output Formats* asks which `--format` values are accepted — CLI. *Task Scheduler Entry Point* asks what the entry point is — core. Where neither half answers it — case 5, whose title names the split itself — both halves get new titles and the original is removed outright. The rule matters because a retained title makes the `subx-cli` delta a `## MODIFIED Requirements` entry, and a new title makes it a `## REMOVED Requirements` plus `## ADDED Requirements` pair; getting it wrong produces a delta that validates and reads as a deletion. It also has a downstream consequence worth naming: `configuration-management`'s *`config` Subcommand Operations* references *Repair Path For Strict-Invalid Configuration* by title, and that reference keeps resolving only because the CLI half kept the title.

**The arithmetic of the two tables**, since it is the one thing in this change a reader is most likely to want to check: 82 `## REMOVED Requirements` entries plus 7 new core-half titles gives the 89 requirements arriving in `subx-core`; 40 untouched plus 12 `## MODIFIED Requirements` plus 6 `## ADDED Requirements` gives the 58 staying here. The 7 new core-half titles are cases 1, 2, 3, 4, 6, 8 and 9 above — case 5's core half replaces a removed title and case 7's keeps its own, so neither adds one.

**Gap avoidance, worked on the three hardest cases.**

- **Case 5** is the one that could not be got wrong quietly, because A2 wrote the requirement as two labelled halves — "Library half" and "Binary half" — with three additional constraints underneath. The core half takes the enum, the `From` impls, the constructors, `ApiErrorSource`, `category()`, `machine_code()`, `hint()`, the prohibition on core calling the presentation methods, the `hint()` rustdoc obligation, and the `OutputModeUnsupported` retention. The CLI half takes `SubXErrorExt` in `src/cli/error_ext.rs`, its two methods, the "bodies unchanged" guarantee, and the four import sites. The one sentence that belongs to both — "no exit code, message, prefix, or `Hint:` line differs from before the split" — is a statement about the CLI methods' bodies, so it goes with the CLI half; the core half instead carries the narrower guarantee that `Display` is unchanged. Scenarios: *Presentation methods require the extension trait* → CLI; *Machine contracts need no import*, *Core does not depend on presentation*, *Core renders operation errors through Display* → core.
- **Case 3** is where a gap was nearly created. Splitting off `Display` looks like it leaves the English-language rule ("All messages, prefixes, and hints SHALL be written in English") on whichever side you happen to put it, and the hints are core prose while the messages are rendered by both. It is stated on **both** halves, deliberately, as the single exception to the no-overlap rule: it is a project-wide editorial constraint from AGENTS.md, not an obligation on a specific function, and dropping it from either half would let that side's text drift into another language without violating anything. Recorded here so it is not later "fixed" as a duplication.
- **Case 4** splits cleanly along its own prose. The original names three subjects: subcommands under `src/commands/`, the configuration loader `src/config/`, and the match engine `src/core/matcher/`. The core half takes the last two and the *Invalid configuration value is reported, not panicked* scenario, and cites `tests/config_validation_tests.rs` and `tests/match_engine_error_display_integration_tests.rs`, both core-bound. The CLI half takes the subcommands and the *Match-engine failure renders through the unified pipeline* scenario — which is CLI because its THEN is about stderr and the process exit code — and cites `tests/match_engine_error_handling_integration_tests.rs`, which is CLI-bound.

**Overlap avoidance, and the one place a split was rejected because of it.** `configuration-management`'s *Configuration Service Abstraction* reads like a split: the trait is core, and "the command handler SHALL obtain configuration by calling `config_service.get_config()` on the injected service rather than reading a global or static" is CLI. It is **not** split, because that exact obligation, over the same three command files, is already the whole of `component-factory`'s *Commands Consume Services via Dependency Injection*, which stays in `subx-cli`. Splitting would put two requirements in one repository saying the same thing about the same lines, and `spec-governance`'s duplication prohibition is aimed at cross-repository copies precisely because intra-repository ones are at least visible in one `grep`. So the requirement goes wholesale to core, and the CLI obligation is left where it already lives. The scenario *Command receives an injected service* travels with it as narrative framing, which is what `spec-governance`'s non-evidence list says it is.

**Scenario accounting.** Every scenario of every split requirement is allocated to exactly one half, verbatim, with five exceptions. Because a requirement half must carry at least one scenario to pass `--strict`, a split whose scenarios all land on one side forces the other side to gain one; that is the origin of three of the five.

1. **Restated on both sides.** `parallel-processing`'s *Parallel match over a directory* asserted two obligations on two sides in one scenario. It is restated on each side with its obligation divided: the core half keeps "each video SHALL be processed by the scheduler", the CLI half keeps "the command SHALL report the number of tasks to be processed and the maximum concurrency to the user". Same GIVEN and WHEN, disjoint THENs. This is the only scenario in the change that appears in both trees.
2. **Rephrased, not duplicated.** `input-path-handling`'s *Extension Filtering* has one scenario, phrased over `ConvertArgs::get_input_handler().collect_files()`. The core half rephrases it over a handler built directly with `with_extensions(&["srt", "ass", "vtt", "sub", "ssa"])`; the CLI-invocation form travels to the gathered CLI requirement.
3. **Two added because the core half would have none.** `encoding-detection`'s *Robust Handling of Empty and Binary Files* has two scenarios, both phrased as "the command SHALL not panic and SHALL exit successfully", both CLI. The core half — *Detector Tolerates Empty and Binary Input* — gains two scenarios phrased over the detector's return value for a zero-byte and a binary input.
4. **One added because the CLI half would otherwise assert only half its own contract.** `error-handling`'s *Binary Error Surface Adds Presentation Through an Extension Trait* keeps A2's *Presentation methods require the extension trait* and gains *The trait lives in the binary crate*, asserting that a consumer depending on `subx-core` alone finds neither method — the obligation the split exists to create, which no A2 scenario stated because before the split there was no such consumer.
5. **Three added for the gathered requirements' own obligations.** Each gathered CLI requirement receives the scenarios lifted with its clauses, and gains one asserting the "no logic" property that is the point of gathering them: `input-path-handling` gains *The adapter adds no behaviour*, `timeline-sync` gains *Legacy sync aliases still resolve*, and `subtitle-matching`'s second addition gains *The allocator is invoked once, after all rewrites*.
6. **One added as a repair.** `cache-management`'s *Cache Clear Subcommand* gains one scenario asserting that the CLI's `cache_path()` resolves to the same path as the core producer. Justified in Decision 6.
7. **One added and one retired in the same capability.** `supply-chain-hardening`'s *Replace unmaintained md5 crate* gains *neither manifest re-introduces the unmaintained crate*, because its single existing scenario is about a hash call site and says nothing about the manifests the restated requirement now governs. `configuration-management`'s *Local Provider Validation Rules* loses C2a's *The cross-repository reference resolves*, whose GIVEN can no longer occur — Decision 7.

Net: **eight scenarios added, one retired, one restated on both sides, one rephrased.** The English-language sentence of 1→2 split case 3 is the only *prose* stated twice.

### Decision 4: fourteen CLI clauses are gathered into four requirements instead of producing fourteen one-sentence halves

A2 wrote several requirements in `input-path-handling` and `timeline-sync` in the same shape: a core mechanism, then one sentence obliging the clap struct to be a thin adapter with no logic of its own. Splitting each of those on the line A2 drew would produce, in `input-path-handling` alone, five CLI requirements reading "the `--no-extract` and `--recursive` flags SHALL be defined and forwarded", "each command SHALL supply its own extension whitelist", "`*Args::get_input_handler` SHALL be a thin adapter", "`DetectEncodingArgs::get_file_paths` SHALL be updated accordingly", "`crate::cli` SHALL keep the legacy re-exports". Each is one sentence, none is independently reviewable, and a reader asking "what does the CLI still own here" would have to read five headings to assemble one rule.

So those clauses are **gathered**: one CLI requirement per capability, whose bullets are the lifted clauses. Three capabilities, four requirements, fourteen source clauses:

- `input-path-handling` → *Input Argument Structs Are Thin Adapters Over Core Collection*, five bullets, from six requirements: the `--no-extract` clause of *No-Extract CLI Flag* and the `--recursive` clause of *Recursive vs Flat Traversal* share bullet 1; *Extension Filtering* gives bullet 2, *CollectedFiles Additional APIs* bullet 3, *Unified Path Merging* bullet 4, and *Core-Owned Input Collection*'s legacy-re-export clause bullet 5.
- `timeline-sync` → *Sync Argument Struct Is a Thin Adapter Over Core Pairing*, four bullets, from three requirements: *Sync Method Selection* (the `--method` flag and the `Manual method requires --offset parameter.` validation) gives bullet 1, *Core-Owned Sync Pairing Resolution* bullet 2, *Core-Owned Default Output Path Derivation* bullet 3, and the two legacy aliases those last two define share bullet 4.
- `subtitle-matching` → *Match Command Argument Surface and Input Preconditions*, four bullets, from *Confidence Threshold Enforcement* (the clap 0–100 range), *File Relocation Modes* (the `--copy`/`--move` exclusion and its exact message), *Optional Backup Before Move* (the `--backup` flag and its forwarding) and *AI-Based File Pairing* (the empty-input error).

**One CLI clause is deliberately not gathered.** `subtitle-matching`'s *AI-Driven Language and Globally-Unique Target Naming* contributes its own standalone requirement, *Match Command Applies Archive-Origin Relocation Before Uniqueness Allocation*, because it is not an argument-surface obligation at all: it is a call-order constraint at `src/commands/match_command.rs:459-470` that the core allocator provably cannot enforce — a free function over a mutable slice cannot require its caller to have finished rewriting — and burying it among flag rules is how it gets violated.

**Why this is not a licence to merge freely.** The gather is admissible exactly when the clauses are the same *kind* of obligation — "the argument layer defines flags and adapts, and contains no logic" — so that the resulting requirement has one subject and reads as one rule. Where the CLI residue is heterogeneous it stays separate: `parallel-processing`'s CLI residue stays as three requirements, because reporting a summary, hiding a progress bar and announcing a task count are three obligations on three code paths.

**No gap, checked mechanically.** For each gathered clause, the clause is deleted from the core half's prose and its text appears as a bullet of the gathered requirement, with the `**Migration**` note on the removed requirement naming which bullet received it. The task list requires that check in both directions: fourteen lifted clauses, fourteen `**Migration**` notes, thirteen bullets and one standalone requirement, with no bullet lacking a source and no note lacking a destination.

### Decision 5: `supply-chain-hardening` is not split, and `encoding-detection` is split despite being 2 against 6

The brief asks whether any capability is lopsided enough that splitting is worse than moving or leaving it wholesale. Two candidates, and they resolve in opposite directions.

**`supply-chain-hardening` stays wholesale in `subx-cli`.** SDR §9 lists it as MIXED, and against the tree as written in April that was defensible: *Replace unmaintained md5 crate* names cache hashing, *Narrow dependency feature flags* names `symphonia`, and both of those are core subjects. A0 and C1 changed the capability's character. A0 added *Every Declared Dependency Has a Use Site* with the sentence "This requirement applies per manifest. When the project is split across more than one crate, each crate's manifest SHALL satisfy it against its own source trees" — a rule about every manifest the project has. C1 added *Submodule Pointer Is a Supply-Chain Input*, which is about the superproject alone, and *Each Published Crate Is Audited in Its Own Repository*, which is about the pair, and rewrote *CI cargo audit gate* to govern the workspace lockfile plus each crate's own. After that, the end state has **no core-only requirement**. Every one of the eight either constrains both manifests or constrains the superproject.

Splitting it would mean writing the per-manifest rules twice, once per repository, in two files with nothing comparing them — the exact drift `spec-governance` forbids and the exact reason C2a declined to mirror `crate-topology`. Moving it wholesale to `subx-core` would strand *Submodule Pointer Is a Supply-Chain Input* in the child, where `.gitmodules` does not exist. Leaving it wholesale in `subx-cli` is correct under `spec-governance`'s *Capabilities Governing the Repository Relationship Live in `subx-cli`*, whose stated reason — "each of them is a statement about both repositories at once" — now describes this capability as accurately as it describes `crate-topology`.

What is *not* acceptable is leaving it untouched, because two of its requirements are phrased against a single manifest and read wrongly against two. So it gets a two-requirement `## MODIFIED Requirements` delta and no removals:

- *Replace unmaintained md5 crate* is restated to say that the rule applies to every manifest the project publishes, and that at the time of writing no manifest declares `md5`; the cache-hashing scenario is qualified to name `subx-core:src/services/ai/cache.rs:177` and `subx-core:src/core/matcher/engine.rs` as the hash sites, so the scenario keeps a referent.
- *Narrow dependency feature flags* is restated per manifest: `tokio` appears in both and each declaration is separately constrained (SDR §4 fixes the two feature sets), `symphonia` appears only in `subx-core`'s.

`spec-governance`'s *Capabilities Governing the Repository Relationship Live in `subx-cli`* enumerates three capabilities by name. It is left unamended: the requirement's normative sentence is general ("A capability whose subject is the relationship between the two repositories SHALL live in `subx-cli`"), the three names are given as instances, and adding a fourth name would mean a `## MODIFIED Requirements` delta on `spec-governance` in the change that lands the first split — which is a lot of ceremony for an example list. Noted in Open Questions instead, because the *next* author to add a repository-relationship capability should probably fold the list into a maintained one or delete it.

**`encoding-detection` is split, 2 core against 6 CLI.** The temptation is to call it CLI wholesale: it is a diagnostic command, five of its seven requirements are about printing, and the two core ones are small. Rejected. `select_best_encoding` in `src/core/formats/encoding/detector.rs` is a `subx-core` function with a precisely specified contract — two fallback confidences, `0.5` and `0.1`, and two exact sample-text prefixes — and *Low-Confidence Fallback To Default Encoding* is the only place that contract is written down. Leaving it in `subx-cli` would put a `subx-core` function's numeric contract in a repository where the person changing the function does not have the spec, which is the failure `spec-governance` exists to prevent. Two requirements is a capability: `async-runtime-safety`, which C2a moved wholesale, has three.

The other direction is checked too. `input-path-handling` is 12 core against 2 CLI, and moving it wholesale would strand *Output Directory Resolution for Archive Files* — implemented at four command sites and duplicated by the GUI precisely because it is not in core. Its CLI half is small but real, so it splits.

### Decision 6: forty retained requirements are not restated, and the test for that is textual, not topical

`spec-governance` and SDR §10 require a `## MODIFIED Requirements` entry to restate the requirement in full. Restating a requirement that does not change is not free: it produces a delta that reads as a change, and a reviewer must diff it against the main spec to discover that nothing happened. So a retained CLI requirement is restated only if its **text** must change. The test, applied to all 58 retained requirements:

1. Does it cite a path that no longer exists in `subx-cli`? (`src/core/…`, `src/services/…`, `src/config/…`, `src/error.rs`, `tests/` files B3 moves, `tests/fixtures/formats/**`.)
2. Does it name a crate path — `crate::core::…`, `crate::config::…` — that now resolves only through the D11 re-export?
3. Does it reference a capability that this change moves, or a requirement whose title this change changes?
4. Is it one half of a split?
5. Does the other half's departure leave one of its sentences without a referent?

Twelve requirements answer yes to at least one and are restated. The twelve, with which test fired:

| Capability | Requirement | Test |
|---|---|---|
| `cache-management` | Cache Clear Subcommand | 5 — the cache path it deletes is now produced in the other repository |
| `configuration-management` | Repair Path For Strict-Invalid Configuration | 4 |
| `encoding-detection` | Robust Handling of Empty and Binary Files | 4 |
| `error-handling` | User-Facing Error Formatting | 4 |
| `error-handling` | No Panics On Recoverable Errors | 1, 4 — cites three `tests/` files, two of which move |
| `format-conversion` | Supported Output Formats | 4 |
| `secrets-protection` | Mask sensitive config values in CLI output | 4 |
| `subtitle-matching` | Dry-Run and Execution Modes | 3 — names `execute_operations`, whose specification leaves |
| `subtitle-translation` | Translation Guidance Options | 4 |
| `supply-chain-hardening` | Replace unmaintained md5 crate | 1 — the hash sites move |
| `supply-chain-hardening` | Narrow dependency feature flags | 1 — `symphonia` moves; `tokio` becomes two declarations |
| `timeline-sync` | Single-File and Batch Modes | 2 — names `crate::core::sync::resolve_sync_pairing` |

Forty answer no to all five and are left alone. The ones it is worth naming because they look like they should be restated and are not: `component-factory`'s *Commands Consume Services via Dependency Injection* mentions `ComponentFactory`, but as a type name, not a path, and `subx_cli::core::ComponentFactory` still resolves through D11; `subtitle-matching`'s *Match Command Emits Structured JSON Payload* cites `src/cli/table.rs`, which stays; `configuration-management`'s *`config` Subcommand Operations* references *Repair Path For Strict-Invalid Configuration* by title, and that title is retained on the `subx-cli` side precisely so this reference keeps resolving — which is one of the reasons Decision 3's title rule is not arbitrary.

**The one addition beyond the split.** *Cache Clear Subcommand* gains a sentence and a scenario. The cache path is resolved twice in the codebase today: `src/core/matcher/engine.rs:2244` produces it, `src/commands/cache_command.rs:224` (via `get_config_dir()` at `:218`) reproduces it for the `cache` subcommand. After the split those two lines are in different repositories, with no compiler and no test forcing them to agree, and the failure mode is silent: `cache clear` prints `No cache file found` while a cache sits on disk. `spec-governance` requires the moving change to repair what its own move breaks; this is that repair, and it is one sentence plus one scenario rather than a new requirement.

### Decision 7: the two cross-repository references this change must move, in opposite directions

**C2a's `configuration-management` qualification is removed, and that is not the reversion C2a feared.** C2a's Decision 10 restated *Local Provider Validation Rules* so its reference reads "the `ai-provider-integration` capability **in `subx-core`**", added a scenario asserting that the qualification reads correctly to someone holding only `subx-cli`, and recorded as a risk that C2b "restates the requirement from the pre-C2a text and silently reverts the qualification".

This change restates the **post-C2a** text — the version with the qualification and the added scenario — and then does two things to it, both required by `spec-governance`:

- The qualification is removed, because the requirement itself moves to `subx-core`, and `spec-governance`'s *Cross-Repository Capability References Are Qualified and Re-Qualified When They Move* says an unqualified capability reference is read as a reference within the same repository. Once both capabilities are in `subx-core`, "in `subx-core`" is noise at best and wrong at worst, because it implies the reader is elsewhere.
- C2a's added scenario, *The cross-repository reference resolves*, is retired with it. Its GIVEN is "a reader of this requirement holding only a `subx-cli` checkout", and after this change there is no such reader: the requirement is not in `subx-cli`. Retaining it would leave a scenario whose GIVEN cannot occur.

The distinction from a reversion is visible in the artifact: a reversion would produce text identical to the pre-C2a main spec, including the missing qualification *and* the missing scenario. What this change produces is text that differs from both — it carries C2a's edit forward and then supersedes it for a stated reason. The `**Migration**` note on the removal says exactly this, and the `subx-core` delta's version of the requirement is the un-qualified one, so the two artifacts cannot disagree.

**`local-llm-provider`'s three references are re-qualified to `subx-core`, discharging C2a's fourth Open Question.** `local-llm-provider` is already in `subx-core`, and C2a qualified three of its references to `subx-cli` because that was correct at C2a's commit:

- *Local LLM Provider Identifier* refers to `normalize_ai_provider` as "defined in the `configuration-management` capability".
- *Local Provider Environment Variable Overrides* refers to the `SUBX_AI_*` precedence rule "as defined by the `configuration-management` capability".
- *Actionable Local-Endpoint Error Mapping* refers to the URL-redaction rule "in conformance with the `error-handling` capability's ... requirement".

All three targets are core-owned in this change: *AI Provider Identifier Canonicalization*, *AI Environment Variable Overrides* and *No sensitive data in error chains* all migrate. So all three references become same-repository references and the `subx-cli` qualification is removed. This is a `## MODIFIED Requirements` delta on `local-llm-provider` inside `import-split-capability-specs`, restating three requirements in full — the only reason that change touches a capability it did not import.

Note the asymmetry, because it is the general shape of the problem: a reference is re-qualified by the change that moves its **target**, not by the change that moves the referring spec. C2a moved the referrer and qualified; C2b moves the target and un-qualifies. If a later change splits `local-llm-provider` itself, it will have to check these three again.

### Decision 8: `import-split-capability-specs`'s artifact set, file by file

C2a Decision 4 fixed the shape for `import-core-specs` because a change with thirteen ADDED capability files and no code would otherwise be guessed at. The same applies, with two differences: this change's receiving side carries a `## MODIFIED Requirements` delta as well, and its twelve capabilities *already exist* nowhere in `subx-core` but share their names with twelve capabilities that will still exist in `subx-cli`.

| Path (relative to `subx-core/`) | Content |
|---|---|
| `openspec/changes/import-split-capability-specs/.openspec.yaml` | Exactly two lines: `schema: spec-driven` and `created: <date of authoring>`. |
| `openspec/changes/import-split-capability-specs/proposal.md` | The four H2s of SDR §10. `## Why` follows C2a Decision 3's substance — it adds no behaviour, it exists because an archived change is OpenSpec's only mechanism for populating `openspec/specs/`, and the provenance of these 89 requirements has to be readable from this repository — and additionally states that each of the twelve capabilities is **one half of a capability of the same name in `subx-cli`**, names `split-mixed-capabilities-across-repos` as its other half, and states that it is not independently reviewable. `## What Changes` lists the twelve with requirement and scenario counts, the twelve Purposes written by hand, the citation edits, and the `local-llm-provider` re-qualification. `## Capabilities`: twelve under `### New Capabilities`, `local-llm-provider` under `### Modified Capabilities`. `## Impact`: `**Code:**`, `**Tests:**`, `**APIs:**`, `**Dependencies:**` all `None.`; `**Documentation:**` names `subx-core/CHANGELOG.md`. |
| `openspec/changes/import-split-capability-specs/design.md` | `## Context`, `## Goals / Non-Goals`, `## Decisions`, `## Risks / Trade-offs`. Four decisions and no more: (1) the arriving text is the `subx-cli` deltas' text and the expected diff against the sending change is empty apart from the enumerated edits; (2) the twelve `## Purpose` paragraphs and their exact wording, including that each names only `subx-core` paths and states the CLI half's role in prose without a path; (3) the post-archive H1/Purpose repair, including that `subtitle-translation`'s H1 must be `# Subtitle Translation` and not the `# Subtitle Translation Specification` its `subx-cli` ancestor carries; (4) the `local-llm-provider` re-qualification per Decision 7. It SHALL NOT re-argue the classification — that argument is this file, and it cites `subx-cli`'s archived copy of it. |
| `openspec/changes/import-split-capability-specs/specs/<capability>/spec.md` × 12 | Each starts with `## ADDED Requirements`, no H1 and no `## Purpose`, then carries the core half's requirements in the source order. |
| `openspec/changes/import-split-capability-specs/specs/local-llm-provider/spec.md` | `## MODIFIED Requirements` with the three re-qualified requirements restated in full. |
| `openspec/changes/import-split-capability-specs/tasks.md` | Numbered H2 phases per SDR §10, ending with a documentation phase and a quality-gate phase run against `subx-core`'s own quality script (C1 Decision 4). |

**The twelve delta files are produced mechanically from this change's own deltas, not from the main specs.** For a requirement that migrates whole, the source is `openspec/specs/<cap>/spec.md`'s block, unchanged. For a split, the source is the core half as written for this change. Both are assembled once, here, so that the sending change's `## REMOVED Requirements` titles and the receiving change's `## ADDED Requirements` titles are generated from one list and cannot disagree — the title-set equality check in the task list is then a real check rather than a comparison of two independent transcriptions.

**No filtered git history.** C2a moved twelve capability *directories* and could hand `git filter-repo` a `--path` per directory. Here no directory moves: each of the twelve `openspec/specs/<cap>/spec.md` files is *edited* in `subx-cli` and a *new* file with a subset of its requirements appears in `subx-core`. `filter-repo` cannot express "the requirements matching these titles"; a path-filtered history would carry the whole file including the requirements that stay, which is worse than no history because it would present the CLI half as having lived in `subx-core`. So the core-side files are authored, and provenance rests where C2a's Decision 8 observed it already rests: `subx-cli/openspec/changes/archive/**` keeps the complete authoring record of all 134 requirements, and `import-split-capability-specs` names the change and the date they were divided. This is stated rather than left implicit because C2a established filtered history as the series' default and a silent departure from it would look like an omission.

### Decision 9: `--skip-specs` is used even though no capability is emptied, and the twelve `subx-cli` Purposes are the reason

C2a used `--skip-specs` because it had to: its deltas emptied thirteen capabilities and `openspec archive` refused atomically. This change's deltas empty nothing — all twelve capabilities keep a CLI half, so the rebuilt specs are non-empty and `openspec archive` would very likely succeed.

It is used anyway.

**What the tool cannot do.** `openspec archive` applies REMOVED, MODIFIED and ADDED operations to the requirement blocks of a main spec. It does not touch the H1 or the `## Purpose`. After a successful archive, `openspec/specs/component-factory/spec.md` would hold exactly one requirement — *Commands Consume Services via Dependency Injection* — under a Purpose reading "Provide a centralized dependency-injection factory, `ComponentFactory` … Implemented in `src/core/factory.rs` and consumed by `src/commands/match_command.rs`". That is a `spec-governance` violation of two clauses at once (a Purpose citing the other repository, and a Purpose describing behaviour the spec no longer contains), and `openspec validate --specs --strict` passes on it, exactly as it passes on the `TBD` placeholder C2a documented.

**Why not archive and then fix.** Because the twelve Purposes are the same twelve files the archive just rewrote, and an intermediate commit in which the requirement blocks are new and the Purposes are stale is a state that validates, reads plausibly, and is the most likely thing to get committed if the work is interrupted. Applying everything by hand keeps each file in exactly one of two states.

**The procedure**, in the order the tasks perform it, mirroring C2a:

1. `subx-core`: author `import-split-capability-specs`, `openspec validate import-split-capability-specs --strict`, `openspec archive import-split-capability-specs -y` (no flag — the receiving side succeeds), then hand-repair twelve H1s and twelve Purposes, then `openspec validate --specs --strict`. Commit.
2. `subx-cli`: `openspec archive split-mixed-capabilities-across-repos -y --skip-specs`, then hand-apply all thirteen deltas — 82 removals, 12 restatements, 6 additions — then rewrite twelve `## Purpose` paragraphs, then write `openspec/split-capabilities.txt`, then `openspec validate --specs --strict`. Commit **together with the moved submodule gitlink**.

Receiving side first, for C2a's reason: the transient state is duplication, which one `git log` explains, rather than absence, which nothing recovers.

**What replaces the tool's guarantee.** Three checks, all in the task list and all independent of how the edits were made: the set of `### Requirement:` titles under each of the twelve capabilities in `subx-core` equals the set named in this change's REMOVED deltas plus the seven new core-half titles; the set in `subx-cli` equals 58 titles enumerated in the task list; and the intersection of the two sets, per capability, is empty. The third is the one that catches a hand-application slip — a requirement pasted into `subx-core` and not deleted from `subx-cli` — and it is the check `spec-governance`'s *A requirement is never in both trees* scenario asks for.

### Decision 10: the recorded split set is a committed twelve-line file, not a table inside `spec-governance`

`spec-governance`'s *The Two Specification Trees Are Isolated by Construction and Checked for Drift* requires that no capability name appear in both trees "**except** where that capability is deliberately split, and that the deliberately-split set is recorded in one place". C2a left where as an Open Question for C2b, listing two candidates.

**Chosen: `subx-cli/openspec/split-capabilities.txt`**, twelve lines, one capability name per line, sorted, no comments. Committed in `subx-cli` because that is where `spec-governance` lives and where the drift check will run.

**Rejected: a table inside `spec-governance`'s own spec.** It reads better and it costs more than it looks. Every future split — and *A Capability That Grows a Requirement on the Other Side of the Line Is Split, Not Moved* guarantees there will be future splits — would need a `## MODIFIED Requirements` delta on `spec-governance` restating the whole requirement to add one row. That is the shape of overrun C1 and C2a each spent a decision on, recurring on every split forever. Worse, a check that parses a markdown table inside a normative requirement makes the requirement's formatting load-bearing: reflowing the table would break CI.

**Why a bare list and not a structured file.** The check needs one predicate — "is this name in the deliberate set" — and a newline-delimited list answers it in one `grep -qxF`. `spec-governance` additionally asks that "every capability name in either tree is reachable from the split-ownership record"; that is satisfiable against a bare list plus the two directory listings, because a name in both trees and not in the list is a defect and a name in one tree and in the list is also a defect (a split that lost a half). Both are one-line assertions. Adding structure now, before the check exists, would be guessing at what C3 needs.

**The file's content after this change**, which is also the twelve capabilities this change splits:

```
cache-management
component-factory
configuration-management
encoding-detection
error-handling
format-conversion
input-path-handling
parallel-processing
secrets-protection
subtitle-matching
subtitle-translation
timeline-sync
```

`supply-chain-hardening` is not in it, which is the mechanically visible consequence of Decision 5.

### Decision 11: how two halves of one capability stay coherent, and what makes them incoherent

The brief asks how a capability that exists in both repositories keeps its halves coherent over time. `spec-governance` already answers most of it; this decision states which of its requirements bite here and adds the one thing it does not cover.

**What `spec-governance` already guarantees, and what this change does to satisfy it.**

- *A Capability Moves Wholesale, Splits, or Stays* requires the two halves to share a name and each to have its own `## Purpose` naming the paths it actually covers. Satisfied by the twenty-four hand-written Purposes; the shared name is why `openspec/split-capabilities.txt` has to exist at all.
- *A requirement is never in both trees* is the invariant that makes the halves a partition rather than two overlapping documents. Enforced by the per-capability title-intersection check of Decision 9, and by the one deliberate exception being a *scenario*, not a requirement (Decision 3's accounting item 1).
- *A Capability That Grows a Requirement on the Other Side of the Line Is Split, Not Moved* is what keeps the halves from re-merging by accretion: someone adding a CLI requirement to `input-path-handling` adds it to the `subx-cli` half, not to whichever half they happened to open. Twelve pre-existing pairs make that rule cheap to follow, because both halves already exist and neither has to be created.
- *Citation Paths Are Resolved Against the Owning Repository* is what keeps a half readable in isolation. Every citation crossing the line in this change is qualified `subx-cli:` or `subx-core:`, and the task list sweeps for unqualified `src/cli/`, `src/commands/` and `src/main.rs` in `subx-core/openspec/specs/` and for unqualified `src/core/`, `src/services/`, `src/config/` and `src/error.rs` in the twelve `subx-cli` halves. That second sweep is new — C2a needed only the first, because it moved nothing into a repository that kept a half behind.
- *Cross-Repository Capability References Are Qualified and Re-Qualified When They Move* is Decision 7's subject.

**The one thing `spec-governance` does not cover: a split pair can drift without either half becoming wrong.** Nothing forbids `subx-cli`'s `timeline-sync` half from growing a requirement that contradicts a `subx-core` requirement, or from re-specifying one. The check of Decision 9 catches identical titles; it does not catch two differently-titled requirements that disagree. This change does not solve that, and says so rather than pretending the title check is sufficient. The mitigation available at zero cost is the shared name: `spec-governance` chose it so that "a reader asking 'what does SubX specify about *X*' reads both files", and the twelve Purposes are written to make the other half's existence explicit — each ends by naming its counterpart's repository and role in prose. A reviewer who opens one half is told, in the first paragraph, that there is another. Raised in Open Questions as the residual gap.

### Decision 12: the obvious seam is the capability boundary; it is named and declined, and the condition under which it is taken is stated

Three sibling changes measured themselves, found an overrun and named a seam: B3 at ~14.5 h handed three phases to B4, C1 named its own division of labour with B1 and B3, and C2a measured 9.25 h and explicitly *declined* a seam. This change measures **~10.25 h** (see `## Sizing`) — at the top of the band the series has been running in, above C2a's 9.25 h and B3's retained 9 h, below the 14.5 h that forced B3 to split.

**The obvious seam is the capability boundary,** and on the surface it is a good one. Capabilities are independent: no delta in one names a requirement in another, and the verification checks are per-capability. A plausible division would be eight capabilities here (`cache-management`, `component-factory`, `configuration-management`, `encoding-detection`, `error-handling`, `secrets-protection`, `supply-chain-hardening`, `format-conversion` — ~6.5 h, carrying the whole two-repository mechanism and the three hardest judgements) and five forward (`input-path-handling`, `parallel-processing`, `subtitle-matching`, `subtitle-translation`, `timeline-sync` — ~3.75 h, carrying all three gather decisions).

**It is declined, for a reason specific to this change: the partition is what the project's governance rule is defined over.** `spec-governance`'s drift check asserts that no capability name appears in both trees *except* where the capability is deliberately split, and that the split set is recorded in one place. Between the two halves of a seamed C2b, five capability names would exist in both trees while `openspec/split-capabilities.txt` listed only eight — so the check would have to be either absent, or wrong, or relaxed to tolerate exactly the condition it exists to detect. C2a landed that rule and this change is the first to exercise it; landing it in a state where it cannot be enforced, for the duration of a second change, is the failure C1's Decision 14 and C2a's Decision 10 both refused in smaller form. The three title-set checks have the same property: their totals (89 core, 58 CLI, empty per-capability intersection) are properties of the completed partition, and against a partial one they degrade from assertions to arithmetic nobody can check.

The secondary reason is smaller but real: `configuration-management` and `secrets-protection` specify the same masking behaviour (Open Questions), so they cannot be separated across a seam without the risk that one half is de-duplicated and the other is not.

**The condition under which the seam is taken anyway.** This is a measurement, not a certainty, and the estimate's two soft items are the twelve core-side deltas (2.5 h if the twenty-three divided requirements' core halves compose cleanly from this document, 4 h if they have to be re-derived) and the hand application of 100 delta operations (2 h if scripted per operation type, 3.5 h if done one at a time). If task 1's baseline pass shows either running long, the seam above is taken as specified, and the follow-on is named **`harden-split-capability-specs`** in the shape of `harden-split-test-suite`: it extends `import-split-capability-specs` rather than authoring a second receiving change, appends five lines to `split-capabilities.txt`, and re-runs the verification over the full set. In that case the drift check is not implemented until the follow-on lands, and that must be written into C3's hand-off. Stating the condition in advance is the point: B3's overrun was found by measuring, and the failure mode being designed out is discovering it at operation 60 of 100.

**Why not split within a capability.** A capability's delta is atomic in the only sense that matters: applying half of `error-handling`'s eight removals leaves a main spec that validates and is wrong. The capability is the smallest unit whose completion can be checked, so it is the only admissible seam.

## Risks / Trade-offs

- **Risk: a requirement is classified CLI because its scenario prints something, when its normative object is core.** → Mitigation: this is the single most likely classification error, and Decision 1's operational test targets it — read the THEN and ask which file must change. Two requirements were caught by it (`subtitle-translation`'s *Batching and Ordering*, `timeline-sync`'s *VAD Padding Chunks Configuration*), and both would have been misfiled by the family (c) screen. The task list re-runs the check on the four requirements in the set whose THEN mentions output but whose object is core, and requires the citation to resolve inside `subx-core`.
- **Risk: a split leaves a gap — an obligation that was in the original and is in neither half.** → Mitigation: every split is authored by moving text, never by rewriting from the title, and the task list requires a clause-level accounting per split: each sentence and each scenario of the original is checked off against exactly one half. Decision 3 works the three hardest cases in the open, including the one sentence deliberately stated twice, so a later reader does not mistake it for an error.
- **Risk: a split leaves an overlap, and the two halves drift into disagreement.** → Mitigation: the per-capability title-intersection check of Decision 9 catches identical titles in both trees. It does not catch differently-titled disagreement, which Decision 11 states as a residual gap rather than papering over. One rejected split (`Configuration Service Abstraction`) exists specifically because splitting it would have created an intra-repository overlap.
- **Risk: `openspec archive` is run without `--skip-specs`, succeeds, and the twelve stale Purposes are committed.** → Mitigation: this is a new hazard that C2a did not have — C2a's archive *failed* loudly, this one would succeed. It is pre-empted in three places: Decision 9 states it, the task carrying the command names the flag and the reason on the same line, and the verification phase greps the twelve `subx-cli` halves for `src/core/`, `src/services/`, `src/config/` and `src/error.rs` outside a `subx-core:` qualification and requires zero hits, which a stale Purpose cannot survive.
- **Risk: C2a's `configuration-management` qualification is dropped and it looks like the reversion C2a warned about.** → Mitigation: Decision 7 states the difference and the `**Migration**` note on the removal repeats it, so the artifact carries its own explanation. The distinguishing evidence is textual and checkable: a reversion would also silently lose C2a's added scenario, and this change retires that scenario explicitly with a stated reason.
- **Risk: the gather of fourteen CLI clauses loses one.** → Mitigation: fourteen clauses, fourteen `**Migration**` notes naming the destination of each, thirteen bullets and one standalone requirement, and a task that checks the correspondence in both directions — no bullet without a source, no note without a destination. The gather is the one place in this change where text is *moved between requirements* rather than between repositories, so it gets its own check.
- **Risk: `input-path-handling`'s CLI residue is judged to be "only the flag surface" and the capability is moved wholesale.** → Mitigation: *Output Directory Resolution for Archive Files* is implemented at five call sites in four command files (`convert_command.rs:362`, `match_command.rs:459`, `sync_command.rs:512`, `:585`, `translate_command.rs:330-389`) and is duplicated by the GUI at `../subx/src-tauri/src/commands/convert.rs:490-506` and `match.rs:394-410` — SDR §8 gaps 4 and 5 — precisely because it is not in core. Moving the requirement to `subx-core` would specify behaviour no `subx-core` file implements. The task list re-greps `archive_origin` before the split to confirm the call sites are still where the classification says.
- **Risk: `supply-chain-hardening` is split anyway by an implementer following the brief's list of thirteen.** → Mitigation: Decision 5 argues it, the proposal states it in the `## Why`, its delta file exists and contains only `## MODIFIED Requirements`, and `openspec/split-capabilities.txt` has twelve lines. Four artifacts have to agree to get it wrong.
- **Risk: the seam is taken as advisory and one change attempts all thirteen.** → Mitigation: Decision 12 gives the arithmetic and the composition of both halves. The failure mode of ignoring it is the one C2a named for its own overrun risk: mechanical work done at 2 a.m. across 100 delta operations, in files where a mistake validates.
- **Trade-off: twenty-three requirements have their content divided, and the project's requirement count rises by thirteen for no behaviour change.** → Accepted. The alternative is twenty-three requirements each of which one of the two repositories cannot satisfy, which is the condition this change exists to end. The count is stated in the proposal so it is not later read as scope creep.
- **Trade-off: no filtered git history for the moved requirement text.** → Accepted (Decision 8). `filter-repo` cannot express a subset of a file's requirements, and a path-filtered history would misrepresent the CLI half as having lived in `subx-core`. The authoring record stays in `subx-cli/openspec/changes/archive/**`, which C2a already established as richer than blame for spec files.
- **Trade-off: twelve capability names now exist in both trees, and a reader of one repository sees an incomplete capability.** → Accepted; it is `spec-governance`'s own chosen design ("a reader asking 'what does SubX specify about *X*' reads both files"). The mitigations are the shared name, the twelve Purposes that each name their counterpart, and `split-capabilities.txt` as the enumerable record.
- **Trade-off: `spec-governance`'s example list of relationship capabilities is now incomplete by one.** → Accepted (Decision 5). Amending an example list costs a full `## MODIFIED Requirements` restatement of an eight-scenario requirement; the normative sentence is general and already covers `supply-chain-hardening`. Open Questions.

## Sizing

Estimated at **~10.25 h**, in the accounting convention C2a used: the estimate covers the work the task list performs, not the classification and delta authoring that produced this document and the twelve `subx-cli` delta files, which are artifacts of the proposal rather than steps of the change.

| Work | Estimate |
|---|---|
| Baseline: confirm C2a landed and archived, both roots green, and re-verify the classification's load-bearing citations | 1.0 h |
| `import-split-capability-specs` framing artifacts (`.openspec.yaml`, `proposal.md`, `design.md`, `tasks.md`) | 1.5 h |
| Twelve core-side ADDED deltas (89 requirements, of which twenty-three are composed halves rather than verbatim copies) plus the `local-llm-provider` MODIFIED delta | 2.5 h |
| Core-side archive, twelve H1/Purpose repairs, `validate --specs --strict` | 1.0 h |
| CLI-side archive with `--skip-specs`, hand-applying 100 delta operations, twelve Purpose rewrites, `split-capabilities.txt` | 2.0 h |
| Verification: three title-set checks, per-capability intersection, two citation sweeps, `TBD` grep, fresh-clone validation | 1.25 h |
| Documentation: `AGENTS.md`, two CHANGELOGs, the hand-off record | 0.5 h |
| Quality gate | 0.5 h |

The two soft items and the seam they would trigger are Decision 12's. For comparison within the series: C2a moved 93 requirements verbatim in 9.25 h; this change moves 89, of which twenty-three are composed rather than copied, and additionally rewrites twelve `subx-cli` Purposes that C2a did not have — which is most of the extra hour.

## Migration Plan

Each step leaves both repositories in a state that either validates or fails loudly, and the one window in which a requirement exists twice is deliberate and short.

1. **Baseline.** Confirm C2a has landed and archived, that `openspec list --specs` reports 20 at the `subx-cli` root and 13 inside `subx-core/`, and that `openspec validate --all` is green in both. Confirm the end states this change classifies against: A2's three additions and five modifications are in the main specs, A0's three additions and C1's two are in `supply-chain-hardening`, and C2a's `configuration-management` restatement is in place.
2. **Re-verify the classification's load-bearing citations.** The two demotions C2a handed over; `match_command.rs:718` and `:765`; `masking.rs:19`; `translation/engine.rs:405`; `match_args.rs:53`; `match_command.rs:459-470`; the `archive_origin` call sites; `cache_command.rs:224` against `engine.rs:2244`. Any that has moved is corrected in Decision 2 before anything is written.
3. **Author this change's twelve `subx-cli` deltas.** 82 removals, 12 restatements, 6 additions. `openspec validate split-mixed-capabilities-across-repos --strict` green.
4. **Author `import-split-capability-specs` in `subx-core`,** generating the twelve core-side deltas from the same title list as step 3, plus the `local-llm-provider` MODIFIED delta. `openspec validate import-split-capability-specs --strict` green from inside `subx-core/`.
5. **Land the receiving side.** Archive, repair twelve H1s and twelve Purposes, `grep -r 'TBD - created by archiving'` returns nothing, `openspec validate --specs --strict` green, 25 capabilities. Commit in `subx-core`. **From here the 89 core-half requirements exist in both trees.**
6. **Land the sending side.** Archive with `--skip-specs`, hand-apply all 100 delta operations, rewrite twelve Purposes, repair `subtitle-translation`'s H1, write `openspec/split-capabilities.txt`, `openspec validate --specs --strict` green, 20 capabilities and 58 requirements. Commit in `subx-cli` **with the moved gitlink**. The duplication window closes here.
7. **Verify the pair.** Three title-set checks, the two citation sweeps, the per-capability intersection check, and a fresh `git clone --recurse-submodules` validated at both roots.
8. **Document.** `AGENTS.md`, both CHANGELOGs, and the hand-off to `harden-split-capability-specs`.

**Rollback** is `git revert` of the `subx-cli` commit — which restores the twelve full capabilities and the old gitlink in one step — plus, optionally, `git revert` of the `subx-core` commit. Nothing outside `openspec/` is affected. The union of the two trees is textually equivalent before and after apart from the twenty-three divided requirements, the eight added scenarios and the one retired scenario, all enumerated, so a revert cannot lose an obligation.

## Open Questions

- **The residual coherence gap of Decision 11.** The title-intersection check catches a requirement present in both trees. Nothing catches two differently-titled requirements in the two halves of one capability that contradict each other. The cheapest candidate mitigation is to have the drift check additionally require each split capability's two Purposes to name each other, which is mechanical and weak. **Resolve when the drift check is implemented (C3), or accept the gap deliberately.**
- **The pre-existing masking overlap.** `secrets-protection`'s *Mask sensitive config values in CLI output* and `configuration-management`'s *Sensitive value masking in config display* specify the same behaviour over the same code, and have since both were written. This change preserves the overlap exactly — both CLI halves, both restated or retained unchanged — because de-duplicating it is a decision about which capability owns config-display masking and this change has no mandate to make it. **Resolve in a later change.**
- **`spec-governance`'s example list of relationship capabilities.** It names `crate-topology`, `cross-crate-testing` and `spec-governance`; after Decision 5, `supply-chain-hardening` belongs with them. Amending it costs a full restatement of an eight-scenario requirement. The better fix is probably to delete the example list and rely on the normative sentence. **Resolve in the next change that touches `spec-governance`.**
- **Whether B3's core-side filing of `tests/dependency_injection_integration_tests.rs` is right.** It imports `subx_cli::App`, which stays in `subx-cli`, so B3's rule 3 does not strictly fire. Nothing in this change depends on it (Decision 1), but if B4 re-files the test to `subx-cli`, `component-factory`'s *Tests Use TestConfigService via TestConfigBuilder* needs one more `subx-cli:` qualification. **Resolve during B3 or B4; re-check at step 2 of the Migration Plan.**
- **Whether `subtitle-translation`'s *Documentation Coverage* should exist at all after the split.** It requires `docs/command-reference.md`, `README.md` and `README.zh-TW.md` to document translation. All three are `subx-cli` files, so it classifies CLI without difficulty — but C3 is about to rewrite all three, and a requirement that a document mention a feature is the kind of thing C3 may want to restate against the two-crate documentation set. Carried over unchanged here. **Flag for C3.**
