# Language Detection

## Purpose

Detect the language of subtitle (and related media) files from their filesystem paths and filenames, producing a normalized language code plus a confidence score and source label, so the match engine can use language information as matching metadata and as input to the output filename rendering. Implemented in `src/core/language.rs` and consumed by `src/core/matcher/mod.rs` (`MediaFile` construction) and `src/core/matcher/engine.rs` (subtitle output naming).

## Requirements

### Requirement: Language Code Normalization Table

`LanguageDetector::new` SHALL initialize a case-insensitive mapping from raw tokens to normalized language codes. The mapping SHALL include at minimum the following entries (see `src/core/language.rs:58-75`):

- Traditional Chinese (`tc`): `tc`, `繁中`, `繁體`, `cht`
- Simplified Chinese (`sc`): `sc`, `简中`, `简体`, `chs`
- English (`en`): `en`, `英文`, `english`

Tokens not present in the table SHALL NOT be recognized as a language.

#### Scenario: Keyword-based detection for Traditional Chinese

- **GIVEN** the file path `繁體/subtitle.srt`
- **WHEN** `LanguageDetector::get_primary_language` is called
- **THEN** it SHALL return `Some("tc")`

#### Scenario: Keyword-based detection for Simplified Chinese

- **GIVEN** the file path `简体/subtitle.srt`
- **WHEN** `LanguageDetector::get_primary_language` is called
- **THEN** it SHALL return `Some("sc")`

#### Scenario: Unknown token not recognized

- **GIVEN** the file path `subtitle.xx.srt` where `xx` is not in the mapping table
- **WHEN** `LanguageDetector::get_primary_language` is called
- **THEN** it SHALL return `None`

### Requirement: Directory-Name Detection

`LanguageDetector::detect_from_directory` SHALL iterate over the path components, lowercase each component, look it up in the language-code table, and return the first match as a `LanguageInfo` with `source = LanguageSource::Directory` and `confidence = 0.9`.

#### Scenario: Directory component matches a language code

- **GIVEN** the path `tc/movie.srt`
- **WHEN** `detect_from_path` is called
- **THEN** it SHALL return `Some(LanguageInfo { code: "tc", source: LanguageSource::Directory, confidence: 0.9 })`

#### Scenario: Case-insensitive directory matching

- **GIVEN** the path `EN/movie.srt`
- **WHEN** `detect_from_path` is called
- **THEN** it SHALL return `Some(LanguageInfo { code: "en", source: LanguageSource::Directory, confidence: 0.9 })`

### Requirement: Filename-Pattern Detection

`LanguageDetector::detect_from_filename` SHALL match the file name (the final path component) against the following regular expressions, in order (see `src/core/language.rs:77-81`):

1. `\.([a-z]{2,3})\.` — e.g., `movie.en.srt`, `movie.tc.srt`, `movie.sc.srt`
2. `_([a-z]{2,3})\.` — e.g., `movie_en.srt`, `movie_sc.srt`
3. `-([a-z]{2,3})\.` — e.g., `movie-en.srt`, `movie-tc.srt`

For the first pattern whose captured token is present in the language-code table, the detector SHALL return a `LanguageInfo` with `source = LanguageSource::Filename` and `confidence = 0.8`.

#### Scenario: Dot-delimited language tag

- **GIVEN** the path `movie.en.srt`
- **WHEN** `detect_from_path` is called
- **THEN** it SHALL return `Some(LanguageInfo { code: "en", source: LanguageSource::Filename, confidence: 0.8 })`

#### Scenario: Underscore-delimited language tag

- **GIVEN** the path `movie_sc.srt`
- **WHEN** `detect_from_path` is called
- **THEN** it SHALL return `Some(LanguageInfo { code: "sc", source: LanguageSource::Filename, confidence: 0.8 })`

#### Scenario: Hyphen-delimited language tag

- **GIVEN** the path `movie-tc.ass`
- **WHEN** `detect_from_path` is called
- **THEN** it SHALL return `Some(LanguageInfo { code: "tc", source: LanguageSource::Filename, confidence: 0.8 })`

#### Scenario: No recognizable tag

- **GIVEN** the path `subtitle.ass`
- **WHEN** `get_primary_language` is called
- **THEN** it SHALL return `None`

### Requirement: Directory Evidence Outranks Filename Evidence

`LanguageDetector::detect_from_path` SHALL attempt directory-name detection before filename-pattern detection and SHALL return the first successful result. `detect_all_languages` SHALL collect both directory and filename evidence, sort the results by `confidence` in descending order, and deduplicate by `code`, so that a directory hit (confidence `0.9`) precedes a filename hit (confidence `0.8`) for the same code.

#### Scenario: Directory evidence wins over filename evidence

- **GIVEN** the path `tc/movie.en.srt`
- **WHEN** `detect_from_path` is called
- **THEN** it SHALL return `Some(LanguageInfo { code: "tc", source: LanguageSource::Directory, confidence: 0.9 })`

#### Scenario: Multiple-evidence aggregation is sorted and deduplicated

- **GIVEN** the path `tc/movie.en.srt`
- **WHEN** `detect_all_languages` is called
- **THEN** the first element of the returned vector SHALL have `code = "tc"` with `confidence = 0.9`
- **AND** the second element SHALL have `code = "en"` with `confidence = 0.8`
- **AND** no two elements SHALL share the same `code`

### Requirement: Integration as Match-Engine Metadata

The match engine SHALL populate a per-file `language: Option<LanguageInfo>` field on each `MediaFile` using `LanguageDetector::detect_from_path` during `MediaFile` construction (see `src/core/matcher/mod.rs:343-344`). The match engine SHALL also use `LanguageDetector::get_primary_language` when computing the renamed subtitle filename so the language code is propagated into the final output name (see `src/core/matcher/engine.rs:766-782`).

#### Scenario: MediaFile carries detected language

- **GIVEN** a subtitle file `tc/episode.srt`
- **WHEN** a `MediaFile` is constructed for it
- **THEN** its `language` field SHALL equal `Some(LanguageInfo { code: "tc", source: LanguageSource::Directory, confidence: 0.9 })`

#### Scenario: Renamed subtitle includes language code

- **GIVEN** a matched pair with video base name `episode` and subtitle path `episode.sc.srt`
- **WHEN** `MatchEngine::generate_subtitle_name` runs
- **THEN** the resulting filename SHALL be `episode.sc.srt`

#### Scenario: Renamed subtitle omits language when undetected

- **GIVEN** a matched pair with video base name `episode` and subtitle path `episode.srt` (no language hint)
- **WHEN** `MatchEngine::generate_subtitle_name` runs
- **THEN** the resulting filename SHALL be `episode.srt` (no language segment inserted)
