## Context

`subx-cli match` asks an AI provider to pair video files with subtitle files and then renames the subtitles to match the video's base name. The current English prompt (in `src/services/ai/prompts.rs`) was written for a single-language workflow and predates two real-world failure modes seen in user reports:

1. Multiple subtitles for the same video are sent in one request (e.g. multilingual rips with `movie.srt`, `movie.1.srt`, `movie.2.srt`). The model has no instruction to look at `content_samples` text when filenames carry no language code, so it picks language from filename heuristics or skips it altogether.
2. The post-AI rename code (`MatchEngine::generate_subtitle_name`) computes target names purely from `LanguageDetector::get_primary_language(&subtitle.path)`. When two AI-approved subtitles for the same video both lack a detectable language code, both rename to `<video>.<ext>` and the second operation either overwrites the first or aborts the journal.

Anthropic's Claude prompt-engineering guide recommends: clear, direct instructions; XML structure for mixed instruction/data prompts; stating *why* a constraint matters; and explicitly scoping instructions so the model does not silently generalize. The current prompt uses none of these techniques.

The codebase is mid-sized and the prompt sits behind the `AIProvider` trait; all four providers (`openai`, `openrouter`, `azure-openai`, `local`) share the same `build_analysis_prompt_base` / `build_verification_prompt_base` functions, so a single edit reaches every provider. `MatchResult` is cached on disk via `src/services/ai/cache.rs`, so the response schema must remain backward-compatible.

## Goals / Non-Goals

**Goals:**

- Make the analysis prompt explicitly tell the model to infer subtitle language from **both** filename and content sample text.
- Make the analysis prompt explicitly forbid two `matches[]` entries from producing the same target filename, and tell the model how to disambiguate (language code first, numeric suffix as last resort).
- Restructure the prompt with XML tags + Markdown headings per Claude's best-practices.
- Let the AI hand back its language inference (`language`) and disambiguating suffix (`target_filename_suffix`) so the rename code does not have to re-derive them.
- Defensively enforce target-name uniqueness in `MatchEngine` even when the model still emits a duplicate (resilience against older models / non-Claude providers).
- Preserve cache compatibility (existing serialized `MatchResult` payloads still deserialize).

**Non-Goals:**

- Rewriting the verification prompt's *semantics* (it gets the same XML/Markdown polish but no schema change).
- Replacing `LanguageDetector` or changing how non-AI flows decide language.
- Adding a content-sniffing language detector (e.g. whatlang) — the AI is the language oracle.
- Changing CLI flags, configuration keys, or the `AIProvider` trait signature.
- Touching `core/translation/` prompts (separate capability).

## Decisions

### Decision 1: XML-tagged, role-prefixed prompt body

Adopt the following top-level structure for `build_analysis_prompt_base`:

```text
<role>You are an expert subtitle-matching assistant…</role>
<instructions>
## Task
…
## Language Inference Rules
…
## Naming and Uniqueness Rules
…
## Output Schema
…
</instructions>
<video_files>
  <file id="…" name="…" path="…"/>
  …
</video_files>
<subtitle_files>
  <file id="…" name="…" path="…"/>
  …
</subtitle_files>
<content_samples>
  <sample subtitle_id="…">…preview…</sample>
  …
</content_samples>
<example>
  <input_summary>…</input_summary>
  <output>{ JSON }</output>
</example>
```

**Why:** Claude's guide explicitly recommends XML for prompts that mix instructions, file inventories, and examples; consistent tag names reduce misinterpretation. Markdown headings inside `<instructions>` give the model human-readable structure without re-introducing ambiguity at the top level.

**Alternatives considered:**

- *Pure Markdown.* Rejected — Claude best-practices advise XML when instructions and data are interleaved.
- *Pure JSON.* Rejected — JSON is brittle to embed multi-line subtitle previews and harder for the model to scan.

### Decision 2: Language inference is the model's job, with content as primary evidence

The `## Language Inference Rules` section will say (paraphrased): *Use the subtitle preview text in `<content_samples>` as the primary signal. Use the filename only when the content sample is empty or undecidable. When filename and content disagree, trust the content. Output a short language code (`tc`, `sc`, `en`, `ja`, `ko`, `fr`, `de`, `es`, `pt`, `ru`, …). Use `und` when language cannot be determined.*

**Why:** The current implicit behavior (filename-only) is the bug we are fixing. Stating *why* (per Anthropic guide) — "filenames are unreliable for multilingual rips that share a base name" — helps the model generalize.

**Alternatives considered:**

- *Detect language in Rust before calling the AI.* Rejected — duplicates AI capability, adds a heavy dependency, and the AI already reads the content for matching anyway.

### Decision 3: Uniqueness contract in the prompt + global uniqueness allocator in code

The prompt SHALL state: *Across the entire `matches[]` array, no two entries MAY produce the same final filename in the same target directory — not just within the same video. If two subtitles would otherwise collide, supply distinct `language` codes; if both are the same language, supply a numeric `target_filename_suffix` (`2`, `3`, …) starting at `2`.*

`MatchEngine::generate_subtitle_name` produces a *candidate* name using this precedence:

1. Prefer `FileMatch.target_filename_suffix` (after sanitization + language-code normalization).
2. Otherwise prefer `FileMatch.language` (after sanitization + language-code normalization). The literal value `und` (case-insensitive) collapses to "no language tag".
3. Otherwise fall back to `LanguageDetector::get_primary_language`.
4. Otherwise no language tag.

After all candidate operations are computed *and* `match_command.rs` has rewritten any `relocation_target_path` for archive-origin forced relocation, the engine runs a **global uniqueness allocator**:

```text
let mut claimed: HashSet<PathBuf> = HashSet::new();
sort operations by (target_directory, subtitle_file.relative_path)  // stable across reruns
for op in operations:
    let mut candidate = op.final_target_path()
    let mut counter = 2
    while claimed.contains(&candidate):
        candidate = insert_numeric_suffix(op.final_target_path(), counter)
        counter += 1
    op.set_final_target(candidate)
    claimed.insert(candidate)
```

Where `insert_numeric_suffix` injects `.<n>` immediately before the *file* extension (preserving any language-code segments). The pass is deterministic because operations are sorted by canonical relative path, not by `subtitle_file.id` (IDs are regenerated per scan).

**Why a global set, not per-cluster?** Per-cluster renaming (the original draft) reintroduces collisions when an *already-distinct* candidate happens to match the bumped name of another cluster, e.g. inputs `movie.srt`, `movie.srt`, `movie.2.srt` would otherwise collapse to `movie.srt`, `movie.2.srt`, `movie.2.srt`. The global allocator probes against *all* claimed paths and bumps further as needed.

**Why two layers (prompt + code)?** The prompt steers Claude well, but `subx-cli` also runs against `openrouter`, `azure-openai`, and `local` providers whose models may ignore the contract. The allocator guarantees correctness regardless.

**Why path-based ordering?** `subtitle_file.id` is a UUIDv7 regenerated per scan, so it gives determinism within a run but is unstable across reruns. Sorting by canonical relative subtitle path makes naming reproducible across reruns of the same input set.

**Alternatives considered:**

- *Trust the AI fully.* Rejected — non-Claude providers cannot be assumed to honor the contract.
- *Fail fast on duplicates.* Rejected — disambiguation is a better UX than aborting a 50-file batch.

### Decision 3a: Content samples carry a stable subtitle ID

`ContentSample` SHALL gain a `subtitle_file_id: String` field, populated by `extract_content_samples` from the same UUIDv7 that the engine assigns to each `MediaFile`. The XML prompt renders previews as `<sample subtitle_file_id="…">…</sample>` so the model unambiguously knows which subtitle the preview belongs to — even when two subtitles share a filename in different directories.

**Why:** Without this, `<content_samples>` matches subtitles only by `name`, breaking the "language from content" contract for any input where subtitle filenames are not unique across the scanned tree. This is a real case (multi-season folders with `subs.srt` per episode dir).

### Decision 3b: AI language values are normalized through the existing code map

After sanitization, AI-supplied `language` and `target_filename_suffix` strings pass through a normalization function backed by `LanguageDetector`'s `language_codes` map. Inputs like `english`, `eng`, `EN` all resolve to `en`; `cht`, `繁中`, `traditional-chinese` resolve to `tc`; the literal `und` (any case) returns `None` so no language tag is appended. Unrecognized but otherwise valid short codes (e.g. `vi`, `id`) are passed through verbatim after lower-casing — this preserves coverage for languages the existing detector does not know.

**Why:** Without normalization, the AI can produce inconsistent filename styles (`movie.english.srt` next to `movie.tc.srt`). Centralizing through the existing code map keeps post-AI naming aligned with the rename style users already see when no AI is involved.

### Decision 3c: XML body content is escaped, subtitle previews are wrapped in CDATA

All attribute values rendered into the prompt (filenames, paths, IDs) SHALL be XML-attribute-escaped (`&amp;`, `&lt;`, `&gt;`, `&quot;`). Subtitle preview bodies inside `<sample>` SHALL be wrapped in `<![CDATA[…]]>` so common subtitle markup such as `<i>` and `<font color="#fff">` does not break the prompt structure. As a CDATA-safety measure, the literal substring `]]>` SHALL be split into `]]]]><![CDATA[>` inside previews.

**Why:** Without escaping, real subtitle content (which routinely contains `<i>` tags) would corrupt the XML scaffolding the rest of the design depends on.

### Decision 4: Schema additions are optional and backward-compatible

`FileMatch` gains:

```rust
#[serde(default)]
pub language: Option<String>,
#[serde(default)]
pub target_filename_suffix: Option<String>,
```

`ContentSample` gains a non-optional `subtitle_file_id: String`. Because `ContentSample` is constructed only inside the crate (in `extract_content_samples`) and serialized only as part of an outbound `AnalysisRequest`, this change is internally backward-compatible — no on-disk artifact stores `ContentSample` directly.

**Why:** Cached `MatchResult` JSON written by previous versions must still deserialize — `#[serde(default)]` makes both new `FileMatch` fields optional on read. Old code paths that ignore the fields keep working.

### Decision 4a: Bump the on-disk match cache version

`src/core/matcher/cache.rs` defines a `cache_version: String` field (currently `"1.0"`). The on-disk cache key uses this together with `config_hash`. We will:

- Bump the constant `cache_version` to `"2.0"` in both `engine.rs::save_file_list_cache` and the test fixtures in `cache.rs`.
- Include a short prompt-schema version tag (e.g. `"prompt_v2"`) in `calculate_config_hash` so subtle prompt rewrites invalidate the cache going forward.

`check_file_list_cache` already rejects entries whose `cache_version` mismatches the current code, so old cache files are silently discarded on the first post-upgrade run.

**Why:** Without this bump, a user who upgrades while a stale cache exists keeps receiving the old (potentially duplicate-name) operations indefinitely, even though the prompt and code are now correct.

### Decision 5: Verification prompt gets cosmetic polish only

`build_verification_prompt_base` is restructured with the same `<role>/<instructions>/<match>/<output_schema>` XML scaffolding but its JSON contract (`{score, factors}`) is unchanged.

## Risks / Trade-offs

- **[Risk] Larger prompt → more tokens.** XML tags add ~20% tokens. → **Mitigation:** Short tag names, terse instructions, one example. Stay well below `gpt-4.1-mini` 128k / `claude-sonnet-4` 200k context.
- **[Risk] Older cached `MatchResult` (in-memory) entries lack `language`.** → **Mitigation:** `#[serde(default)]` on both new fields; the engine's four-step fallback keeps producing the same names as before for cached responses.
- **[Risk] Local LLMs may not honor the uniqueness contract.** → **Mitigation:** The global uniqueness allocator is the safety net and is unit-tested independently of any AI.
- **[Risk] Model emits `target_filename_suffix` containing path separators or invalid characters.** → **Mitigation:** Sanitize the suffix in Rust — keep only `[A-Za-z0-9_-]`, truncate to 16 chars, drop the field if empty after sanitization.
- **[Risk] AI returns inconsistent language strings (`english`/`eng`/`zh-hant`).** → **Mitigation:** Normalize through `LanguageDetector`'s code map before use (Decision 3b).
- **[Risk] Subtitle previews containing `<i>` or `]]>` corrupt the XML scaffolding.** → **Mitigation:** Wrap previews in CDATA with `]]>` neutralization (Decision 3c).
- **[Risk] Pre-existing `movie.2.srt` collides with a numerically-bumped name.** → **Mitigation:** The global allocator keeps probing (`.3`, `.4`, …) against the claimed-paths set until a free name is found.
- **[Risk] Two different videos resolve to the same target directory + filename under copy/move relocation.** → **Mitigation:** The allocator operates on final target paths post-relocation, not on `(video_id, filename)` pairs, so cross-video collisions are also resolved.
- **[Risk] Tests that string-match the old prompt body break.** → **Mitigation:** Tasks now enumerate every affected test file; assertions will be updated to the new XML anchors (`<role>`, `<instructions>`, `Language Inference Rules`, `Naming and Uniqueness Rules`, `<output_schema>`).
- **[Risk] On-disk file-list match cache returns stale duplicate-name results after upgrade.** → **Mitigation:** Bump `cache_version` from `"1.0"` to `"2.0"` (Decision 4a).

## Migration Plan

1. Land schema additions to `FileMatch` (purely additive, both serde-defaulted) and `ContentSample` (`subtitle_file_id` is internal-only).
2. Land the language-code normalization helper (`normalize_language_code(&str) -> Option<String>`) backed by `LanguageDetector`'s code map; unit-test independently.
3. Land the global uniqueness allocator (`apply_unique_target_paths(ops: &mut [MatchOperation])`); unit-test on the high-risk inputs from spec scenarios.
4. Land the new prompt body and update prompt-format tests in `prompts.rs`, `tests/ai_prompts_coverage_tests.rs`, `tests/ai_trait_implementation_tests.rs`, and `tests/match_duplicate_rename_conflict_tests.rs`.
5. Bump `cache_version` to `"2.0"` and add prompt-schema tag to `calculate_config_hash`.
6. Wire the allocator call into `match_command.rs` *after* archive-origin relocation rewrite at lines 456-464, so the guarantee holds at the actual destination paths.
7. Update `tests/common/mock_openai_helper.rs` and `tests/common/test_data_generators.rs` to optionally emit `language` / `target_filename_suffix` for fixture realism.
8. Update `docs/ai-provider-integration-guide.md` and `docs/tech-architecture.md`.
9. Run `scripts/quality_check.sh` on the main agent only.
10. Rollback: revert in reverse order. The `cache_version` bump means rolling back also invalidates caches written under v2.0 — acceptable since they'd be re-derivable. The `MatchResult` schema change is symmetric (`#[serde(default)]`).

## Open Questions

_None._ All decisions above are intentional and backed by the linked best-practices doc.
