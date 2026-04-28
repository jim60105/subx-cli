## ADDED Requirements

### Requirement: AI-Driven Language and Globally-Unique Target Naming

The system SHALL derive each renamed subtitle's filename by consuming the AI's optional `FileMatch.target_filename_suffix` and `FileMatch.language` fields when present, and SHALL guarantee that the operations list emitted for a single match batch contains no two operations whose final target path (parent directory plus filename, after any archive-origin or `--copy`/`--move` relocation) is identical — across the entire batch, not only within a single video.

The naming precedence in `MatchEngine::generate_subtitle_name` SHALL be:

1. If `FileMatch.target_filename_suffix` is `Some(value)` after sanitization (keep `[A-Za-z0-9_-]`, truncate to 16 characters, drop if empty) and language-code normalization, use `<video_base>.<value>.<subtitle_extension>`.
2. Else if `FileMatch.language` is `Some(value)` after the same sanitization and normalization, use `<video_base>.<value>.<subtitle_extension>`. The literal value `und` (case-insensitive) SHALL collapse to "no language tag".
3. Else if `LanguageDetector::get_primary_language(&subtitle.path)` returns `Some(code)`, use `<video_base>.<code>.<subtitle_extension>`.
4. Else use `<video_base>.<subtitle_extension>`.

Language-code normalization SHALL pass the AI value through `LanguageDetector`'s code map so that synonyms (`english`/`eng`/`EN` → `en`; `cht`/`繁中`/`traditional-chinese` → `tc`; `chs`/`简中` → `sc`) collapse to the canonical short code; unrecognized but otherwise valid `[A-Za-z0-9_-]{1,16}` tokens SHALL be passed through verbatim after lower-casing.

After all operations have been generated **and** `match_command` has applied any archive-origin forced relocation, the engine SHALL run a deterministic global uniqueness allocator:

1. Sort operations ascending by `(target_directory, subtitle_file.relative_path)` for stability across reruns.
2. Maintain a `claimed: HashSet<PathBuf>` of already-allocated final target paths.
3. For each operation, take its candidate final path; while the candidate is in `claimed`, replace the filename's pre-extension portion with `<base>.<n>.<ext>` (or `<base>.<lang>.<n>.<ext>` if a language segment is present), where `n` is the smallest integer ≥ 2 not yet used for that base.
4. Insert the resolved path into `claimed` before processing the next operation.

The allocator SHALL produce stable results across reruns of the same input set and SHALL probe past pre-existing files-on-disk patterns (e.g. an input that already contains `movie.2.srt`) when those filenames appear among the candidates.

#### Scenario: AI-supplied suffix wins over filename heuristic
- **GIVEN** a `FileMatch` whose `target_filename_suffix` is `Some("tc")` and whose subtitle path is `subs/movie.srt` (no language tag detectable from the path)
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the produced name SHALL be `movie.tc.srt`

#### Scenario: AI-supplied language used when no suffix is present
- **GIVEN** a `FileMatch` whose `target_filename_suffix` is `None`, whose `language` is `Some("ja")`, and whose subtitle path has no detectable language tag
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the produced name SHALL be `movie.ja.srt`

#### Scenario: AI language synonyms are normalized
- **GIVEN** a `FileMatch` whose `language` is `Some("english")` (or `"eng"`, or `"EN"`) and whose subtitle path has no detectable language tag
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the produced name SHALL be `movie.en.srt` for every variant

#### Scenario: AI value `und` is treated as no language
- **GIVEN** a `FileMatch` whose `language` is `Some("und")` and whose `target_filename_suffix` is `None`, and whose subtitle path has no detectable language tag
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the produced name SHALL be `movie.srt` (no language segment appended)

#### Scenario: Falls back to LanguageDetector when AI omits both fields
- **GIVEN** a `FileMatch` whose `target_filename_suffix` and `language` are both `None`, and whose subtitle path is `subs/movie.tc.srt`
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the produced name SHALL be `movie.tc.srt`

#### Scenario: Sanitization rejects unsafe AI-supplied suffix
- **GIVEN** a `FileMatch` whose `target_filename_suffix` is `Some("../etc")` and whose subtitle path has no detectable language tag
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the suffix SHALL be discarded by sanitization and the produced name SHALL fall through to the next precedence rule, yielding `movie.srt`

#### Scenario: Two duplicate-target operations get globally unique names
- **GIVEN** an AI response whose two `FileMatch` entries both pair the same video with subtitles that produce candidate target filename `movie.srt` (same parent directory)
- **WHEN** the engine assembles the operations list and runs the global uniqueness allocator
- **THEN** sorted by canonical relative subtitle path, the first SHALL keep target filename `movie.srt` and the second SHALL be renamed to `movie.2.srt`

#### Scenario: Three-way duplicates and a pre-existing numeric suffix
- **GIVEN** candidate target filenames `movie.srt`, `movie.srt`, and `movie.2.srt` for the same target directory (the third one is a pre-existing distinct candidate)
- **WHEN** the engine runs the global uniqueness allocator
- **THEN** the resolved names SHALL be `movie.srt`, `movie.3.srt`, and `movie.2.srt` (sorted by canonical path, the second clone probes past the already-claimed `movie.2.srt`)

#### Scenario: Language-disambiguated pairs do not need numeric suffixes
- **GIVEN** an AI response that pairs two subtitles with the same video and supplies distinct `language` values `"tc"` and `"sc"`
- **WHEN** the engine assembles the operations list
- **THEN** the produced names SHALL be `movie.tc.srt` and `movie.sc.srt` and the allocator SHALL NOT modify them

#### Scenario: Cross-video duplicates in the same target directory are disambiguated
- **GIVEN** two different videos `a.mkv` and `b.mkv` whose AI-paired subtitles both relocate via `--copy` into a shared target directory and both happen to produce the same candidate filename `subs.srt` after step 1-4
- **WHEN** the engine runs the global uniqueness allocator after `match_command` applies the relocation rewrite
- **THEN** the two final target paths SHALL differ — sorted by canonical subtitle path, the first keeps `subs.srt` and the second becomes `subs.2.srt`

#### Scenario: Allocator runs after archive-origin forced relocation
- **GIVEN** an archive-origin scenario where `match_command.rs` rewrites `relocation_target_path` for one or more operations after the engine returns
- **WHEN** the global uniqueness allocator runs
- **THEN** it SHALL operate on the rewritten relocation paths so the uniqueness guarantee holds at the actual destination paths, not at the engine's pre-rewrite candidates
