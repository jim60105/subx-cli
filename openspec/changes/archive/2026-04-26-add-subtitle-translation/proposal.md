## Why

SubX-CLI already uses AI for subtitle matching, but users still need a
separate tool to translate matched, synced, or converted subtitles. Adding
translation turns SubX into a more complete subtitle workflow while reusing
the existing AI provider, format parsing, batching, retry, configuration, and backup
infrastructure.

## What Changes

- Add a `subx translate` command that rewrites subtitle cue text into a target
  language while preserving cue timing, ordering, and format metadata supported
  by the existing parser/writer pipeline.
- Support single-file, multi-file, directory, recursive, and archive-expanded
  subtitle inputs using the same input path conventions as existing commands.
- Use the configured AI provider (`openai`, `openrouter`, or `azure-openai`)
  with structured prompts and parseable responses for deterministic per-cue
  translation.
- Assign request cue IDs with UUIDv7 values generated in cue order, spacing
  generation by 1ms so each UUIDv7 `unix_time_ts` is greater than the previous
  cue ID timestamp.
- Add a two-pass terminology consistency workflow: first extract proper nouns
  such as people and places into a translation map, then translate subtitles
  with that map so recurring names use consistent translations.
- Instruct terminology extraction to prefer established conventional
  translations when they exist, and when coining a new translation, prefer
  phonetic transliteration before semantic translation.
- Add translation controls for target language, optional source language,
  optional glossary file, optional inline context guidance, batching,
  overwrite/output behavior, and per-file error isolation.
- Define single-file, batch-directory, and archive-origin output rules so
  generated translations are predictable across all input modes.
- Preserve source files by default unless the user explicitly requests
  replacement, and apply the existing backup/file-operation safety rules when
  overwriting.
- Document the command, configuration, and examples in the command reference,
  configuration guide, and README files.
- No breaking changes.

## Capabilities

### New Capabilities

- `subtitle-translation`: AI-assisted subtitle text translation, including the
  CLI contract, input/output behavior, provider usage, two-pass terminology
  consistency, batching, preservation rules, error handling, and documentation
  requirements.

### Modified Capabilities

- None.

## Impact

- Affected CLI and command code: `src/cli/`, `src/commands/dispatcher.rs`, and
  a new translation command module.
- Affected core/services code: subtitle format parsing/writing in
  `src/core/formats/`, AI provider prompt/response helpers in
  `src/services/ai/`, and component construction in `src/core/factory.rs`.
- Affected configuration/docs/tests: `src/config/`, `docs/command-reference.md`,
  `docs/configuration-guide.md`, `README.md`, `README.zh-TW.md`, and integration
  tests using mock AI providers.
- No new mandatory external dependencies are expected; the feature should use
  existing HTTP AI providers and Rust subtitle-format infrastructure. It may
  require enabling UUIDv7 support on the existing `uuid` dependency.
