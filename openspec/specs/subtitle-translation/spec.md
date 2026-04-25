# Subtitle Translation Specification

## Purpose

Provide a `subx translate` command that rewrites subtitle cue text into a
target language while preserving cue timing, ordering, and format metadata
supported by the existing parser/writer pipeline. Translation reuses the
configured AI provider, structured prompts with stable UUIDv7 cue IDs, and a
two-pass terminology consistency workflow so recurring proper nouns are
translated consistently. Implemented in `src/core/translation/`,
`src/commands/translate_command.rs`, `src/cli/translate_args.rs`, and
supporting prompt helpers in `src/services/ai/`.

## Requirements

### Requirement: Translate Command Interface

The system SHALL provide a `translate` command that accepts subtitle inputs and
a required target language, using the same clap-based CLI conventions as other
SubX commands.

#### Scenario: target language is required

- **GIVEN** the user runs `subx translate movie.srt` without a target language
- **WHEN** CLI argument validation runs
- **THEN** the command SHALL fail with a usage error describing the missing
  target language

#### Scenario: translate command accepts target language

- **GIVEN** the user runs `subx translate movie.srt --target-language zh-TW`
- **WHEN** CLI argument validation runs
- **THEN** the command SHALL accept the invocation and pass `zh-TW` to the
  translation command handler

### Requirement: Subtitle Input Collection

The translate command SHALL collect subtitle files from direct file paths,
directories, repeated `-i/--input` flags, recursive traversal, and supported
archive inputs using the shared input path handling behavior.

#### Scenario: directory input filters subtitle files

- **GIVEN** a directory containing `movie.srt`, `movie.ass`, and `movie.mp4`
- **WHEN** the user runs `subx translate <dir> --target-language ja`
- **THEN** the command SHALL process `movie.srt` and `movie.ass`
- **AND** the command SHALL NOT process `movie.mp4`

#### Scenario: archive input is expanded

- **GIVEN** `subs.zip` contains `episode.srt`
- **WHEN** the user runs `subx translate subs.zip --target-language en`
- **THEN** the command SHALL translate the extracted `episode.srt`
- **AND** write the translated output relative to the original archive location
  when no explicit output directory is provided

### Requirement: Subtitle Structure Preservation

The translate command SHALL preserve subtitle cue timing, cue ordering, cue
count, and format metadata that is representable by the existing parser/writer
pipeline while replacing only translatable user-visible text.

#### Scenario: SRT timing is preserved

- **GIVEN** an SRT file with two cues and valid time ranges
- **WHEN** the command translates it successfully
- **THEN** the output SHALL contain two cues in the same order
- **AND** each cue SHALL retain its original start and end timestamp

#### Scenario: ASS supported style fields are preserved

- **GIVEN** an ASS file with style definitions and dialogue lines containing
  visible text
- **WHEN** the command translates it successfully
- **THEN** the output SHALL retain style definitions supported by the current
  ASS parser/writer
- **AND** dialogue timing/style fields SHALL remain unchanged except for the
  translated visible text field

#### Scenario: unsupported styling is not promised

- **GIVEN** a subtitle file contains styling constructs that the current
  parser/writer cannot represent
- **WHEN** the command translates it
- **THEN** the command SHALL NOT promise preservation beyond the existing
  format parser/writer behavior

### Requirement: AI Provider Translation

The translate command SHALL use the configured AI provider and existing AI
request safety behavior to translate cue text through structured prompts and
parseable responses.

#### Scenario: configured provider is used

- **GIVEN** `ai.provider = "openrouter"` in configuration
- **WHEN** the user runs `subx translate movie.srt --target-language ko`
- **THEN** translation requests SHALL be sent through the OpenRouter provider
  constructed by `ComponentFactory`

#### Scenario: malformed AI response is rejected

- **GIVEN** the AI provider returns a response that is not valid translation
  JSON
- **WHEN** the command parses the response
- **THEN** the command SHALL return a typed AI service error for that file
- **AND** SHALL NOT write a partial translated subtitle for that file

### Requirement: Stable Cue Mapping

The system SHALL include stable cue identifiers in translation requests and
SHALL validate AI responses against requested cue IDs before applying translated
text.

#### Scenario: cue IDs use UUIDv7

- **GIVEN** a subtitle file contains multiple cues
- **WHEN** the command assigns cue IDs for translation requests
- **THEN** each cue ID SHALL be a UUIDv7 value
- **AND** the IDs SHALL be assigned in subtitle cue order

#### Scenario: UUIDv7 timestamps strictly increase

- **GIVEN** cue ID generation assigns IDs to adjacent subtitle cues
- **WHEN** the generator creates the next UUIDv7 cue ID
- **THEN** the generator SHALL intentionally wait at least 1ms after the
  previous UUIDv7 cue ID generation
- **AND** the next UUIDv7 `unix_time_ts` SHALL be greater than the previous
  cue ID's `unix_time_ts`

#### Scenario: missing cue translation is retried once after initial batches

- **GIVEN** a translation batch contains UUIDv7 cue IDs `A`, `B`, and `C`
- **WHEN** the AI response omits cue ID `B`
- **THEN** the command SHALL keep translations for `A` and `C`
- **AND** continue translating all remaining initial batches
- **AND** resend cue `B` for translation once after all initial batches complete

#### Scenario: missing cue retry falls back to empty text

- **GIVEN** cue ID `B` was omitted from the initial translation response
- **AND** the one retry response also omits cue ID `B`
- **WHEN** the command applies translated text
- **THEN** cue `B` SHALL be written with an empty text value
- **AND** the translated subtitle output SHALL still be generated

#### Scenario: unknown cue ID retries the discarded batch once

- **GIVEN** a translation batch contains UUIDv7 cue IDs `A` and `B`
- **WHEN** the AI response includes unknown UUIDv7 cue ID `Z`
- **THEN** the command SHALL treat the response as hallucinated
- **AND** discard the entire response for that batch
- **AND** retry the same batch once

#### Scenario: unknown cue ID after retry fails the file

- **GIVEN** a translation batch response included unknown cue ID `Z`
- **AND** the one retry response for the same batch also includes an unknown cue
  ID
- **WHEN** translation validates the retry response
- **THEN** the command SHALL fail that file with an AI service error
- **AND** no translated output SHALL be written for that file

### Requirement: Two-Pass Terminology Consistency

Before translating subtitle cue batches, the translate command SHALL perform a
terminology extraction pass that identifies proper nouns such as person names
and place names, returns a structured source-to-target terminology map, and
then includes that map in subsequent translation prompts.

#### Scenario: terminology extraction runs before translation

- **GIVEN** a subtitle file contains recurring names such as `Alice` and
  `Wonderland`
- **WHEN** the command translates the file
- **THEN** the command SHALL send a terminology-extraction request before the
  cue translation requests
- **AND** the extraction response SHALL be parsed as a structured terminology
  map

#### Scenario: translation prompt includes terminology map

- **GIVEN** the terminology extraction pass returns `Alice -> 愛麗絲`
- **WHEN** the command builds a cue translation prompt
- **THEN** the prompt SHALL include the terminology map
- **AND** SHALL instruct the provider to use `愛麗絲` whenever translating
  `Alice`

#### Scenario: established translations are preferred

- **GIVEN** a proper noun has an established conventional translation in the
  target language
- **WHEN** the command builds the terminology extraction prompt
- **THEN** the prompt SHALL instruct the provider to use the established
  conventional translation

#### Scenario: coined translations prefer transliteration

- **GIVEN** a proper noun has no established conventional translation
- **WHEN** the command builds the terminology extraction prompt
- **THEN** the prompt SHALL instruct the provider to prefer phonetic
  transliteration before semantic translation

#### Scenario: explicit glossary overrides generated terminology

- **GIVEN** the user-provided glossary maps `Alice -> 艾莉絲`
- **AND** the generated terminology map contains `Alice -> 愛麗絲`
- **WHEN** the command builds the cue translation prompt
- **THEN** the prompt SHALL use the glossary mapping `Alice -> 艾莉絲` as the
  effective terminology entry

#### Scenario: empty terminology map is allowed

- **GIVEN** the terminology extraction pass returns an empty valid map
- **WHEN** the command translates cue batches
- **THEN** translation SHALL proceed without terminology entries

### Requirement: Batching and Ordering

The translate command SHALL split large subtitles into configurable cue batches
for AI requests and SHALL reassemble translated entries in their original order.

#### Scenario: multiple batches preserve final order

- **GIVEN** a subtitle file has 250 cues and translation batch size is 100
- **WHEN** translation completes successfully for all batches
- **THEN** the output SHALL contain all 250 cues in the original order

#### Scenario: translation progress logs processed cue count

- **GIVEN** a subtitle file has 250 cues and translation batch size is 100
- **WHEN** each translation response is accepted
- **THEN** the command SHALL log the number of processed cues and total cues in
  the form `Processed cues: <processed>/<total>`

#### Scenario: one malformed batch prevents partial output

- **GIVEN** a subtitle file requires three translation batches
- **WHEN** the second batch fails validation due to malformed JSON, duplicate
  IDs, or unknown IDs after its one retry
- **THEN** the command SHALL report the file as failed
- **AND** SHALL NOT write an output file containing only the first batch
  translations

### Requirement: Translation Guidance Options

The translate command SHALL support optional source language, glossary file, and
inline context guidance that is included in the translation prompt without
changing subtitle timing or file discovery behavior.

#### Scenario: glossary file is included in prompt

- **GIVEN** the user provides `--glossary glossary.txt`
- **WHEN** the command builds the translation prompt
- **THEN** the command SHALL read `glossary.txt` as a UTF-8 text file
- **AND** the prompt SHALL include the file content as terminology guidance
- **AND** the command SHALL still require the AI response to use the structured
  cue ID mapping

#### Scenario: missing glossary file is rejected

- **GIVEN** the user provides `--glossary missing.txt`
- **WHEN** the command validates translation inputs
- **THEN** the command SHALL return an invalid path or input error
- **AND** SHALL NOT send an AI translation request

#### Scenario: inline context is included in prompt

- **GIVEN** the user provides `--context "Use formal business tone"`
- **WHEN** the command builds the translation prompt
- **THEN** the prompt SHALL include that text as domain or tone guidance
- **AND** SHALL NOT interpret the context value as a filesystem path

#### Scenario: source language is optional

- **GIVEN** the user omits `--source-language`
- **WHEN** the command builds the translation prompt
- **THEN** the prompt SHALL request translation from the detected or unspecified
  source language into the target language

### Requirement: Translation Configuration

The system SHALL expose validated translation configuration for default batch
size and optional default target language without requiring a new external
translation service dependency.

#### Scenario: configured batch size is used

- **GIVEN** translation batch size is configured to `50`
- **WHEN** the user runs `subx translate movie.srt --target-language fr`
- **THEN** the command SHALL split AI translation requests into batches of at
  most 50 cues

#### Scenario: invalid batch size is rejected

- **GIVEN** the user configures translation batch size as `0`
- **WHEN** configuration validation runs
- **THEN** validation SHALL fail with an error describing the invalid batch size

### Requirement: Safe Output Behavior

The translate command SHALL write translated output without modifying the source
file by default and SHALL require explicit overwrite or replacement flags before
changing existing files.

#### Scenario: default output uses target language suffix

- **GIVEN** input file `movie.srt` and target language `zh-TW`
- **WHEN** translation succeeds without an explicit output path
- **THEN** the translated file SHALL be written as `movie.zh-TW.srt`
- **AND** the original `movie.srt` SHALL remain unchanged

#### Scenario: existing output is not overwritten by default

- **GIVEN** `movie.zh-TW.srt` already exists
- **WHEN** the user runs `subx translate movie.srt --target-language zh-TW`
  without an overwrite flag
- **THEN** the command SHALL fail that file with an output-exists error
- **AND** SHALL NOT modify the existing output file

#### Scenario: single-file output path may be a file

- **GIVEN** the user translates `movie.srt` with `--output translated.srt`
- **WHEN** translation succeeds
- **THEN** the command SHALL write the translated subtitle to `translated.srt`

#### Scenario: batch output path must be a directory

- **GIVEN** the user translates multiple subtitle files with `--output out/`
- **WHEN** translation succeeds
- **THEN** each translated file SHALL be written inside `out/` using the
  target-language suffix naming convention

#### Scenario: batch output file path is rejected

- **GIVEN** the user translates multiple subtitle files with `--output out.srt`
- **WHEN** command validation resolves more than one input subtitle
- **THEN** the command SHALL reject the invocation because batch output requires
  a directory

#### Scenario: archive default output uses archive parent

- **GIVEN** `archives/subs.zip` contains `episode.srt`
- **WHEN** the user runs `subx translate archives/subs.zip --target-language en`
  without an explicit output path
- **THEN** the translated file SHALL be written under `archives/`
- **AND** SHALL NOT be written only inside the temporary extraction directory

#### Scenario: replace mode uses backup settings

- **GIVEN** the user enables a replace mode and `general.backup_enabled = true`
- **WHEN** translation succeeds
- **THEN** the command SHALL create a backup of the original subtitle before
  replacing it with translated content

### Requirement: Per-File Error Isolation

The translate command SHALL isolate errors per input file during batch
translation so one failed file does not prevent independent files from being
translated.

#### Scenario: one file fails and one succeeds

- **GIVEN** a batch contains `valid.srt` and `bad.srt`
- **WHEN** `bad.srt` fails parsing or AI response validation
- **THEN** the command SHALL still write the translated output for `valid.srt`
- **AND** SHALL report the `bad.srt` failure to the user

### Requirement: Documentation Coverage

The translation capability SHALL be documented in the command reference,
configuration guide, and README files with examples for single-file and batch
translation.

#### Scenario: command reference documents translate

- **WHEN** the translation command is implemented
- **THEN** `docs/command-reference.md` SHALL include the `translate` command,
  required target language argument, major safety flags, and examples

#### Scenario: README documents translation workflow

- **WHEN** the translation command is implemented
- **THEN** `README.md` and `README.zh-TW.md` SHALL describe translation as part
  of the supported subtitle workflow
