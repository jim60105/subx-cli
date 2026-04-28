## 1. Schema and Helper Additions

- [x] 1.1 Add `language: Option<String>` and `target_filename_suffix: Option<String>` to `FileMatch` in `src/services/ai/mod.rs`, both with `#[serde(default)]` and rustdoc explaining they are optional AI-supplied hints
- [x] 1.2 Add `subtitle_file_id: String` to `ContentSample` in `src/services/ai/mod.rs` and update `MatchEngine::extract_content_samples` (`src/core/matcher/engine.rs:845-872`) to populate it from the originating `MediaFile.id`
- [x] 1.3 Add a `sanitize_suffix(&str) -> Option<String>` helper that keeps `[A-Za-z0-9_-]`, truncates to 16 chars, and returns `None` when the result is empty; cover with unit tests for empty input, path-traversal input (`../etc`), Unicode, and overlong input
- [x] 1.4 Add a `normalize_language_code(&str) -> Option<String>` helper backed by `LanguageDetector`'s `language_codes` map: collapse `english`/`eng`/`EN` → `en`, `cht`/`繁中`/`traditional-chinese` → `tc`, `chs`/`简中` → `sc`, treat `und` (case-insensitive) as `None`, and pass through any other `[A-Za-z0-9_-]{1,16}` token verbatim after lower-casing; cover with unit tests for each case

## 2. Engine Naming Logic

- [x] 2.1 Refactor `MatchEngine::generate_subtitle_name` (`src/core/matcher/engine.rs:885`) to take `&FileMatch` and apply the four-step precedence: sanitized + normalized `target_filename_suffix` → sanitized + normalized `language` (with `und` → no tag) → `LanguageDetector::get_primary_language` → no tag
- [x] 2.2 Update the call site inside the AI-analysis loop (around `engine.rs:791`) to thread the matching `FileMatch` into `generate_subtitle_name`
- [x] 2.3 Implement `apply_unique_target_paths(operations: &mut [MatchOperation])` as a free function in `src/core/matcher/engine.rs`: sort by `(target_directory, subtitle_file.relative_path)`, iterate with a `HashSet<PathBuf>` of claimed final target paths, and for each operation probe `<base>.<n>.<ext>` (or `<base>.<lang>.<n>.<ext>` if a language segment is already present) starting at `n=2` until a free path is found
- [x] 2.4 Call `apply_unique_target_paths` from `match_command.rs` **after** the archive-origin `relocation_target_path` rewrite at lines 456-464, passing the post-rewrite operations slice; ensure both `relocation_target_path` and `new_subtitle_name` are updated atomically so downstream `execute_operations` sees consistent values
- [x] 2.5 Update existing engine tests in `src/core/matcher/engine.rs` (`test_generate_subtitle_name_*`) to pass a synthesized `FileMatch` and add new tests covering: AI suffix wins, AI language used, language synonym normalization (`english` → `en`), `und` → no tag, sanitization rejects `../etc`, two same-video duplicates → `movie.srt` + `movie.2.srt`, three-way duplicate where input already contains `movie.2.srt` → resolves to `movie.srt` + `movie.3.srt` + `movie.2.srt`, two distinct languages preserved verbatim, two different videos colliding under copy mode in shared target dir, archive-origin relocation rewrite still produces unique targets

## 3. Prompt Rewrite

- [x] 3.1 Rewrite `build_analysis_prompt_base` in `src/services/ai/prompts.rs` to emit the XML structure described in `design.md` Decision 1, including `<role>`, `<instructions>` (with `## Task`, `## Language Inference Rules`, `## Naming and Uniqueness Rules`, `## Output Schema` Markdown sections), `<video_files>`, `<subtitle_files>`, `<content_samples>`, `<output_schema>`, and one `<example>` block
- [x] 3.2 Render each video / subtitle file as `<file id="…" name="…" path="…"/>` with all attribute values XML-attribute-escaped (escape `&`, `<`, `>`, `"`, `'`); parse the existing `"ID:… | Name:… | Path:…"` strings tolerantly via simple `split` so `MatchEngine` callers do not need to change shape
- [x] 3.3 Render each `ContentSample` as `<sample subtitle_file_id="…"><![CDATA[…preview…]]></sample>` and neutralize any literal `]]>` substring inside the preview by splitting it as `]]]]><![CDATA[>`
- [x] 3.4 Update `build_verification_prompt_base` to use the same XML scaffolding (`<role>`, `<instructions>`, `<match>`, `<output_schema>`) without changing the JSON contract
- [x] 3.5 Update `get_analysis_system_message` and `get_verification_system_message` to be one short, role-only sentence (the heavy lifting moves into `<role>` / `<instructions>` blocks)
- [x] 3.6 Update unit tests in `src/services/ai/prompts.rs` to assert the new anchors: `<role>`, `<instructions>`, `<video_files>`, `Language Inference Rules`, `Naming and Uniqueness Rules`, `<output_schema>`, `<sample subtitle_file_id="…">`; remove or replace assertions tied to old free-text wording while keeping the legacy ID-presence assertions; add a test covering XML escaping (filename containing `&`) and CDATA preservation of `<i>` markup including a preview that contains `]]>`

## 4. Cache Invalidation

- [x] 4.1 Bump the `cache_version` constant from `"1.0"` to `"2.0"` in `src/core/matcher/engine.rs::save_file_list_cache` and update the test fixtures in `src/core/matcher/cache.rs`
- [x] 4.2 Add a short prompt-schema tag (e.g. `"prompt_v2"`) to `MatchEngine::calculate_config_hash` so future prompt rewrites invalidate the cache automatically
- [x] 4.3 Add a unit test that an entry written with `cache_version: "1.0"` is rejected by `check_file_list_cache` after the bump

## 5. Test Fixtures and Mocks

- [x] 5.1 Update `tests/common/test_data_generators.rs` (`MatchResponseGenerator`) so its emitted JSON optionally includes `language` and `target_filename_suffix`; keep at least one fixture that omits them to exercise the legacy path
- [x] 5.2 Update `tests/common/mock_openai_helper.rs` synthesizer to populate `language` from an explicit caller-supplied map (default empty) so individual tests can opt in
- [x] 5.3 Add a unit test in `src/services/ai/prompts.rs` that round-trips a JSON `MatchResult` containing `language` and `target_filename_suffix` and verifies they survive `parse_match_result_base`
- [x] 5.4 Update prompt-format integration tests that string-match the old prompt body: `tests/ai_prompts_coverage_tests.rs`, `tests/ai_trait_implementation_tests.rs`, `tests/match_duplicate_rename_conflict_tests.rs` (which today parses old `Video files:` / `Subtitle files:` headers), plus any provider tests asserting exact system messages
- [x] 5.5 Add an integration test (extend `tests/match_engine_id_integration_tests.rs` or add a new file) that:
  - sends two subtitles whose only differentiator is content (filenames identical except for `.1` / `.2` suffix) and verifies the engine emits unique target names
  - sends two subtitles in *different* directories sharing the same filename `subs.srt` to confirm the `subtitle_file_id` mapping in `<content_samples>` works end-to-end

## 6. Documentation

- [x] 6.1 Update `docs/ai-provider-integration-guide.md` to describe the new XML prompt structure, the `subtitle_file_id`-keyed content samples, and the optional `language` / `target_filename_suffix` response fields
- [x] 6.2 Update `docs/tech-architecture.md` matcher section to describe the four-step naming precedence and the global uniqueness allocator (including its post-relocation invocation point)
- [x] 6.3 Add a `### Changed` entry under `[Unreleased]` in `CHANGELOG.md` summarizing the prompt rewrite, content-driven language inference, language-code normalization, and global uniqueness guarantee; add a `### Migration` note about the `cache_version` bump

## 7. Quality Gate

- [x] 7.1 Run `cargo fmt` and `cargo clippy -- -D warnings` and fix all warnings
- [x] 7.2 Run `cargo nextest run --filter-expr 'test(prompt) + test(generate_subtitle_name) + test(unique_target) + test(match_engine_id) + test(match_duplicate) + test(cache)' || true` and confirm only the targeted modules' tests pass before broader runs
- [x] 7.3 Run `scripts/quality_check.sh` once at the end (main agent only — do not invoke from sub-agents) and ensure it is green
- [x] 7.4 Run `cargo test --doc --all-features` to confirm rustdoc examples still compile
