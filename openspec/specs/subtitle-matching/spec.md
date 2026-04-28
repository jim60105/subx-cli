# Subtitle Matching

## Purpose

Provide AI-assisted pairing of video files with their correct subtitle files across one or more input paths, producing rename, copy, or move operations while enforcing a user-controlled confidence threshold. Implemented primarily in `src/core/matcher/engine.rs`, `src/commands/match_command.rs`, and `src/cli/match_args.rs`.

## Requirements

### Requirement: AI-Based File Pairing

The system SHALL use a configured AI provider to analyze video and subtitle file names (and optional subtitle content samples) and return candidate video-subtitle pairings with a per-pair confidence score.

#### Scenario: Successful match with sufficient confidence
- **GIVEN** a directory containing at least one video file and one subtitle file, and an AI provider is configured
- **WHEN** the user runs `subx match <path>`
- **THEN** the engine SHALL collect eligible files via `InputPathHandler`, send an `AnalysisRequest` to the AI provider, and generate rename operations for every pair whose confidence is greater than or equal to the configured threshold

#### Scenario: No input files available
- **GIVEN** the resolved input paths contain no video or subtitle files
- **WHEN** the match command executes
- **THEN** the command SHALL return an error `No files found to process` without calling the AI provider

### Requirement: Confidence Threshold Enforcement

The system SHALL accept a user-supplied confidence value in the range 0-100 (default 80) via `--confidence`, convert it to a 0.0-1.0 threshold, and discard any AI-proposed pair whose score falls below that threshold.

#### Scenario: Low-confidence pairs are filtered out
- **GIVEN** the user passes `--confidence 90` and the AI provider returns a candidate match with confidence 0.75
- **WHEN** the engine processes the AI response
- **THEN** the engine SHALL omit that candidate from the generated operations list

#### Scenario: Confidence outside valid range is rejected
- **GIVEN** the user passes `--confidence 150`
- **WHEN** the CLI parses the arguments
- **THEN** argument parsing SHALL fail with a validation error from `clap`

### Requirement: Dry-Run and Execution Modes

The system SHALL support a `--dry-run` mode that displays planned operations and persists them to the match cache without mutating files, and a default live mode that executes the operations.

#### Scenario: Dry-run preserves files
- **GIVEN** the user runs `subx match --dry-run <path>`
- **WHEN** the command completes
- **THEN** the planned operations SHALL be printed to the user and saved to the cache, and no file on disk SHALL be created, renamed, copied, moved, or deleted

#### Scenario: Live mode applies operations
- **GIVEN** the user runs `subx match <path>` without `--dry-run`
- **WHEN** the command completes successfully
- **THEN** the engine SHALL execute each operation through `execute_operations`, renaming subtitle files to match the paired video's base name plus the subtitle extension

### Requirement: File Relocation Modes

The system SHALL expose mutually exclusive `--copy` (`-c`) and `--move` (`-m`) flags that relocate the matched subtitle alongside its paired video; when neither is provided the subtitle SHALL be renamed in place.

#### Scenario: Copy and move are mutually exclusive
- **GIVEN** the user passes both `--copy` and `--move`
- **WHEN** the CLI runs `MatchArgs::validate`
- **THEN** validation SHALL fail with the message `Cannot use --copy and --move together. Please choose one operation mode.`

#### Scenario: Copy relocates matched subtitle
- **GIVEN** a video in directory `A/` and its matched subtitle in directory `B/`, and the user passes `--copy`
- **WHEN** the engine executes operations
- **THEN** the subtitle SHALL be copied into directory `A/` with a name derived from the video's base name, and the original subtitle in `B/` SHALL remain untouched

### Requirement: Optional Backup Before Move

The system SHALL create a backup of the source subtitle file before moving it only when the relocation mode is `Move` and backups are enabled (via `--backup` on the command or `general.backup_enabled` in configuration). The system SHALL NOT create backups in rename-in-place or `Copy` modes.

#### Scenario: Backup before Move
- **GIVEN** the user runs `subx match --move --backup <path>` and a matching subtitle is identified
- **WHEN** the engine executes the operation
- **THEN** a backup of the source subtitle SHALL be created before the file is moved to the video's directory

#### Scenario: No backup when only renaming in place
- **GIVEN** the user runs `subx match --backup <path>` without `--copy` or `--move`
- **WHEN** the engine executes the rename-in-place operation
- **THEN** no backup task SHALL be scheduled

### Requirement: Per-Scan Unique UUIDv7 File Identifiers for Matching

The system SHALL assign each discovered media file a per-scan unique identifier of the form `file_<uuid-v7-hyphenated>` (total length 41) generated through the shared `crate::core::uuidv7::Uuidv7Generator` with strict 1ms spacing. Identifiers SHALL be unique within a single discovery scan but SHALL NOT be guaranteed to remain stable across separate invocations of the binary, because UUIDv7 IDs are intrinsically time-based. The match pipeline SHALL reference video and subtitle files by these identifiers (rather than by filename) when sending requests to the AI provider and when correlating the AI response back to disk paths within the same invocation. Cross-invocation correlation (e.g., between the match cache and a later `cache apply`) SHALL use canonical filesystem paths rather than identifiers. Implemented in `src/core/matcher/discovery.rs` (`generate_file_id`, `Uuidv7Generator` integration) and exercised by `tests/match_engine_id_integration_tests.rs`.

#### Scenario: All discovered files receive unique IDs

- **GIVEN** a directory containing several video and subtitle files, including entries with complex non-ASCII filenames
- **WHEN** `FileDiscovery::scan_directory` runs
- **THEN** every returned file SHALL have a non-empty `id` beginning with `file_` and of length 41, the embedded UUIDv7 version nibble SHALL equal `7`, and the full set of IDs SHALL be unique

#### Scenario: AI response is correlated via IDs within the same invocation

- **GIVEN** an AI provider returns `MatchResult.matches` entries referencing `video_file_id` and `subtitle_file_id` shaped as `file_<uuid-v7>`
- **WHEN** `MatchEngine::match_file_list` processes the response
- **THEN** the generated `MatchOperation` set SHALL resolve each ID back to the corresponding `MediaFile` and SHALL produce operations whose `video_file.id` and `subtitle_file.id` match the AI-supplied identifiers

#### Scenario: IDs are not stable across invocations

- **GIVEN** the same directory is scanned in two separate invocations of the binary
- **WHEN** `FileDiscovery::scan_directory` runs in each invocation
- **THEN** the returned `MediaFile.id` values for the same on-disk file MAY differ between the two invocations, AND any cross-invocation correlation SHALL be performed by canonical path rather than by ID

### Requirement: Match Command Emits Structured JSON Payload

When the `match` command runs with the global output mode set to `json`, it SHALL emit a single JSON envelope on stdout (per the `machine-readable-output` capability) and SHALL NOT render the human-friendly result table from `src/cli/table.rs` nor any progress bar. The envelope's `data` object SHALL contain:

- `dry_run` (bool) reflecting the effective `--dry-run` flag.
- `confidence_threshold` (integer in `[0, 100]`) reflecting the effective `--confidence` value.
- `candidates` (array of objects with `video` (string path), `subtitle` (string path), `confidence` (integer 0–100), `accepted` (bool), and an optional `reason` (string) when `accepted == false`).
- `operations` (array of objects with `kind` in `{"rename", "copy", "move"}`, `source` (string path), `target` (string path), `applied` (bool), `status` (`"ok"` or `"error"`), and an optional `error` object with `code`, `category`, `message` when `status == "error"`).
- `summary` (object with integer fields `total_candidates`, `accepted`, `applied`, `skipped`, `failed`).

When the match operation loop applies multiple file operations and an individual operation fails, the affected `operations[i]` entry SHALL carry `status == "error"` and `applied == false` while the top-level envelope MAY remain `status == "ok"` provided at least one prior operation succeeded; alternatively the command MAY abort the loop and emit a top-level error envelope whose `error.details.partial_results` records the operations already applied. A top-level error envelope SHALL be emitted for whole-command failures (AI service failure before any operation is computed, configuration errors, missing inputs).

In `text` mode (the default) the match command's existing UX — colored result table, progress bar, status symbols — is unchanged.

#### Scenario: JSON mode emits payload instead of table
- **GIVEN** an input directory with one accepted video/subtitle pair and an AI provider configured
- **WHEN** the user runs `subx-cli --output json match <path>`
- **THEN** stdout SHALL contain a single JSON envelope with `command == "match"`, `status == "ok"`, and `data.candidates`/`data.operations` populated, and SHALL NOT contain the formatted match table

#### Scenario: Dry-run flag surfaced in payload
- **WHEN** the user runs `subx-cli --output json match --dry-run <path>` with at least one accepted candidate
- **THEN** `data.dry_run == true` and every entry in `data.operations` SHALL satisfy `applied == false`

#### Scenario: Sub-threshold candidates are reported as not accepted
- **GIVEN** an AI provider returns a candidate whose confidence is below `--confidence`
- **WHEN** the user runs `subx-cli --output json match --confidence 90 <path>`
- **THEN** `data.candidates` SHALL include the candidate with `accepted == false` and `data.summary.skipped` SHALL count it

#### Scenario: AI failure surfaces as error envelope
- **GIVEN** the AI provider fails with a network error
- **WHEN** the user runs `subx-cli --output json match <path>`
- **THEN** the envelope SHALL satisfy `status == "error"`, `error.category == "ai_service"`, `error.exit_code == 3`, and the process SHALL exit with status `3`

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
