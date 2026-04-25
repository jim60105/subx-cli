## 1. CLI and Configuration

- [x] 1.1 Add `TranslateArgs` with input path options, required
  `--target-language`, optional `--source-language`, `--glossary <path>`,
  `--context <text>`, output, overwrite/replace, recursive, and archive
  extraction flags.
- [x] 1.2 Register the `translate` subcommand in `Commands` and route it through
  `commands::dispatcher`.
- [x] 1.3 Add translation-related configuration fields and validation for batch
  size and default language options where CLI defaults should be configurable.
- [x] 1.4 Update shell completion generation coverage for the new command and
  flags.

## 2. Translation Core

- [x] 2.1 Create translation request/response data structures with stable cue
  UUIDv7 IDs, target language, optional source language, glossary, context
  fields, and structured terminology-map entries.
- [x] 2.2 Implement cue ID assignment using UUIDv7 values generated in cue order,
  intentionally spacing generation by 1ms so each `unix_time_ts` is greater than
  the previous cue ID timestamp.
- [x] 2.3 Implement a translation engine that parses subtitle files, batches cue
  text, calls the configured AI provider, validates cue ID mappings, and
  reapplies translated text to the original subtitle entries.
- [x] 2.4 Add a terminology extraction pass that runs before translation,
  identifies proper nouns such as person and place names, and produces a
  source-to-target term map.
- [x] 2.5 Merge user glossary entries with generated terminology so explicit
  glossary mappings override AI-generated mappings.
- [x] 2.6 Preserve SRT, VTT, SUB, and ASS/SSA timing, cue ordering, cue counts,
  and metadata supported by the current format parser/writer pipeline while
  translating only user-visible text.
- [x] 2.7 Reject malformed terminology maps and malformed, duplicate, or
  unknown cue IDs in AI responses without writing partial translated output.
- [x] 2.8 Retry missing cue IDs once after all initial translation batches, then
  fill still-missing translations with empty strings while continuing output
  generation.
- [x] 2.9 Discard hallucinated batch responses containing unknown cue IDs, retry
  the same batch once, and fail the file if the retry still contains unknown cue
  IDs.

## 3. AI Integration

- [x] 3.1 Add translation prompt builders and JSON response parsing helpers under
  `src/services/ai/`, reusing existing retry, timeout, security, and error
  sanitization patterns.
- [x] 3.2 Add terminology-extraction prompt builders that instruct the provider
  to prefer established conventional translations and, when coining names,
  phonetic transliteration before semantic translation.
- [x] 3.3 Wire translation AI access through `ComponentFactory` and the
  configured provider (`openai`, `openrouter`, or `azure-openai`).
- [x] 3.4 Add mock-provider test helpers or fixtures for successful,
  malformed, partial, duplicate, unknown-cue, and terminology-map responses.

## 4. Command Behavior and File Safety

- [x] 4.1 Implement `translate_command::execute` using `InputPathHandler` for
  direct files, directories, repeated `-i`, recursive traversal, and supported
  archive inputs.
- [x] 4.2 Implement default output naming with target-language suffixes and
  explicit output path handling for single-file, batch directory, and
  archive-origin inputs.
- [x] 4.3 Prevent overwriting existing translated outputs unless an explicit
  overwrite flag is set.
- [x] 4.4 Implement replace mode with existing backup and file-operation safety
  behavior before modifying source files.
- [x] 4.5 Isolate errors per input file so independent files continue after a
  parse, AI, validation, or write failure.
- [x] 4.6 Log translation progress after each accepted AI translation response
  as processed cues over total cues.

## 5. Tests

- [x] 5.1 Add CLI argument tests for required target language, optional source
  language, glossary path, inline context, output, overwrite, replace,
  recursive, and no-extract flags.
- [x] 5.2 Add translation engine unit tests for UUIDv7 cue ID format, 1ms-spaced
  strictly increasing `unix_time_ts` values, terminology extraction, glossary
  override precedence, cue mapping validation, batching, ordering, and
  partial-output prevention.
- [x] 5.3 Add prompt tests for established-name preference and transliteration
  before semantic translation when coining new proper-noun translations.
- [x] 5.4 Add format preservation tests for SRT, VTT, SUB, and ASS/SSA subtitle
  inputs, limited to metadata the current parser/writer pipeline supports.
- [x] 5.5 Add command integration tests with mock AI providers for single-file,
  batch directory, archive input, output-exists, replace-with-backup, and
  per-file error isolation scenarios.
- [x] 5.6 Run targeted tests during development and run
  `scripts/quality_check.sh` before committing the implementation.
- [x] 5.7 Add tests for missing cue ID retry and empty-string fallback when the
  retry response still omits a cue.
- [x] 5.8 Add tests for unknown cue ID batch retry, successful retry recovery,
  and failure when the retry still contains an unknown cue ID.
- [x] 5.9 Validate translation progress logging through targeted translation
  tests or existing quality checks.

## 6. Documentation

- [x] 6.1 Document `subx translate` in `docs/command-reference.md` with required
  arguments, terminology consistency behavior, safety flags, examples, and
  output naming behavior.
- [x] 6.2 Update `docs/configuration-guide.md` for any new translation
  configuration fields.
- [x] 6.3 Update `README.md` and `README.zh-TW.md` to include translation in the
  supported workflow and quick examples.
- [x] 6.4 Update `AGENTS.md` edit-target guidance if the implementation adds new
  recurring translation modules or conventions.
