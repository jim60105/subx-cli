## Why

The current AI match prompt (`build_analysis_prompt_base` in `src/services/ai/prompts.rs`) gives the model only an unstructured filename list and a JSON schema. Two recurring failures result:

1. **Language is inferred from filenames only.** When several subtitles share a base name (e.g. `movie.srt`, `movie.1.srt`, `movie.2.srt`), the model has no signal to distinguish them and ignores the strongest available cue — the actual subtitle text in `content_samples`. The post-AI rename step (`MatchEngine::generate_subtitle_name`) also depends solely on `LanguageDetector::get_primary_language(&subtitle.path)`, which inspects the path; if the filename has no language tag the rename produces `<video>.<ext>` regardless of the subtitle's real language.
2. **Multiple subtitles can collapse to identical target names.** When two or more subtitles for the same video carry no recognizable language code, every pair generates the same `<video_base>.<ext>` filename. The journal/rename step then either overwrites the previous file or aborts the batch — and the AI is never asked to disambiguate.

The Anthropic prompt-engineering best-practices guide (Claude Opus 4.7) recommends being clear and direct, structuring complex prompts with XML tags, providing context for *why* an instruction matters, and explicitly stating scope so the model does not silently generalize. The current prompt does none of these.

## What Changes

- Rewrite `build_analysis_prompt_base` and `build_verification_prompt_base` to follow Claude best-practices:
  - Wrap role, instructions, file inventories, content samples, output schema, and worked examples in descriptive XML tags (`<role>`, `<instructions>`, `<video_files>`, `<subtitle_files>`, `<content_samples>`, `<output_schema>`, `<example>`).
  - Use Markdown headings inside the instruction block for readability while keeping XML as the top-level structural delimiter.
  - State explicitly that the model MUST infer subtitle language from **both** the filename *and* the content sample text, preferring content evidence when the two disagree.
  - State explicitly that every output filename across a single response MUST be unique; when several subtitles map to the same video, the model MUST attach a distinguishing language code (or fall back to a numeric suffix) so that no two `matches[]` entries produce the same target name.
  - Keep the existing JSON response keys (`matches[].video_file_id`, `subtitle_file_id`, `confidence`, `match_factors`, top-level `confidence`, `reasoning`) for backward compatibility, and add two new optional per-match fields:
    - `language` — ISO-639-1/2 or project code (`tc`, `sc`, `en`, `ja`, …) inferred from filename + content.
    - `target_filename_suffix` — the disambiguating tag the engine should splice between the video base name and the subtitle extension (e.g. `tc`, `en`, `2`).
- Extend `FileMatch` (`src/services/ai/mod.rs`) with `language: Option<String>` and `target_filename_suffix: Option<String>`, both `#[serde(default)]` so older cached responses still deserialize.
- Extend `ContentSample` with the originating subtitle's stable file ID (`subtitle_file_id: String`) so the model can unambiguously map preview text to the right subtitle, even when two subtitles share a filename in different directories. The XML rendering uses `<sample subtitle_file_id="…">` accordingly.
- Update `MatchEngine::generate_subtitle_name` to prefer `FileMatch.target_filename_suffix` (or `language`) when present and fall back to today's `LanguageDetector` result. Normalize AI-supplied language strings through the same `LanguageDetector` code map so that `english`/`eng` collapse to `en`, `cht`/`繁中` to `tc`, etc. Treat the explicit value `und` as "no language" (skip the tag).
- Replace the simplistic "exact-duplicate cluster" rename with a **global uniqueness allocator**: the engine maintains a `HashSet<PathBuf>` of already-claimed final target paths (parent directory + filename), iterates the proposed operations in path-stable order, and bumps the numeric suffix until each operation owns a unique path. This guarantees uniqueness across **all** operations in the batch (different videos, different parent dirs, copy/move relocations, and pre-existing names like `movie.2.srt`), not just exact-duplicate clusters within the same video.
- Run the global uniqueness allocator **after** `match_command.rs` applies its archive-origin forced relocation (`relocation_target_path` rewrite at `src/commands/match_command.rs:456-464`) so the guarantee holds at the actual destination paths.
- Bump the file-list match cache version (`cache_version: "1.0"` → `"2.0"` in `src/core/matcher/cache.rs` and `src/core/matcher/engine.rs::save_file_list_cache`) and include a hash of the prompt schema in `calculate_config_hash` so old cached operations cannot resurface stale duplicate-name results after the upgrade.
- Update prompt-related unit tests in `src/services/ai/prompts.rs`; engine tests in `src/core/matcher/engine.rs` and the existing prompt-format integration tests `tests/ai_prompts_coverage_tests.rs`, `tests/ai_trait_implementation_tests.rs`, `tests/match_duplicate_rename_conflict_tests.rs`; and the wiremock fixtures under `tests/common/test_data_generators.rs` and `tests/common/mock_openai_helper.rs` to exercise the new XML structure, content-driven language inference, and global uniqueness disambiguation.

This is **not** a breaking change for external consumers: the JSON response schema only adds optional fields, the CLI surface is unchanged, and configuration keys are unchanged.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `ai-provider-integration`: The "Shared Prompt and Response Schema" requirement changes — prompts SHALL be structured with XML tags, SHALL instruct the model to use both filename and content evidence for language inference, and SHALL require unique target filenames across a single response. The `MatchResult` schema gains optional `language` and `target_filename_suffix` fields per match.
- `subtitle-matching`: The naming behavior in `generate_subtitle_name` changes — it SHALL consume the AI-supplied language/suffix when present, and SHALL guarantee that the operations list emitted for a single batch contains no duplicate target paths.

## Impact

- **Code:** `src/services/ai/prompts.rs`, `src/services/ai/mod.rs` (`FileMatch`, `MatchResult`, `ContentSample`), `src/core/matcher/engine.rs` (`generate_subtitle_name`, batch assembly, uniqueness allocator, `extract_content_samples`, `calculate_config_hash`, `save_file_list_cache`), `src/core/matcher/cache.rs` (`cache_version` bump), `src/commands/match_command.rs` (run uniqueness allocator after archive-origin relocation rewrite). The in-memory `src/services/ai/cache.rs` only stores `MatchResult` in `RwLock<HashMap<…>>`; the additive `#[serde(default)]` fields keep it compatible without code changes.
- **Tests:** `src/services/ai/prompts.rs` unit tests; `src/core/matcher/engine.rs` `generate_subtitle_name` tests; `tests/ai_prompts_coverage_tests.rs`; `tests/ai_trait_implementation_tests.rs`; `tests/match_duplicate_rename_conflict_tests.rs`; `tests/match_engine_id_integration_tests.rs`; `tests/common/test_data_generators.rs`; `tests/common/mock_openai_helper.rs`. New tests cover: content-only language inference, language-code normalization (`english`→`en`, `und`→no tag), three-way duplicate disambiguation, pre-existing `.2` suffix collision, two-different-videos colliding into the same target directory under copy/move relocation, archive-origin forced relocation + uniqueness, and XML escaping of subtitle previews containing `<i>` tags.
- **APIs:** Public `FileMatch` and `ContentSample` structs gain optional fields (additive, non-breaking on read; `ContentSample` is constructed inside the crate so adding a required field is internally backward-compatible).
- **Dependencies:** None.
- **Documentation:** `docs/ai-provider-integration-guide.md` (prompt schema description, optional fields, content-sample mapping) and `docs/tech-architecture.md` (matcher rename rules + uniqueness allocator) updated; AGENTS.md prompt-related sections require no change.
- **Cache compatibility:** The on-disk file-list match cache (`src/core/matcher/cache.rs`) version is bumped from `"1.0"` to `"2.0"`; old cache entries are ignored by `check_file_list_cache` and replaced on next run. The in-memory `MatchResult` cache (`src/services/ai/cache.rs`) requires no migration because the new `FileMatch` fields are `#[serde(default)]`.
