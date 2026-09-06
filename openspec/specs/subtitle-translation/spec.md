# Subtitle Translation

## Purpose

Hold the CLI half of the translation agreement: the `translate` command interface, subtitle input collection, guidance-file and flag plumbing for prompt guidance, safe output behavior, per-file error isolation, and documentation coverage. Implemented in `src/commands/translate_command.rs` and `src/cli/translate_args.rs`. The translation pipeline — structure preservation, AI-provider translation with two-pass terminology consistency, cue mapping, and prompt-configuration handling — is specified by the same-named `subtitle-translation` capability in `subx-core`.

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

### Requirement: Translation Guidance Options

The `translate` command SHALL expose optional `--source-language`, `--glossary <FILE>` and `--context <TEXT>` options, SHALL read the glossary file, and SHALL pass the resulting values to the translation engine without changing subtitle timing or file discovery behavior.

- `--glossary <FILE>` SHALL be read as a UTF-8 text file at `src/commands/translate_command.rs` before any AI request is made. A missing or unreadable file SHALL be reported as an invalid path or input error and SHALL NOT result in an AI translation request.
- The file's contents SHALL be turned into glossary entries through `parse_glossary_text`, and both the raw text and the parsed entries SHALL be handed to the engine.
- `--context <TEXT>` SHALL be passed through verbatim and SHALL NOT be interpreted as a filesystem path.
- What the engine then does with these three inputs — that they appear in the translation prompt as terminology and tone guidance, that an explicit glossary outranks the generated terminology map, and that an omitted source language becomes a detected-or-unspecified source — is specified by the `subtitle-translation` capability's *Translation Prompt Guidance Inputs* requirement in `subx-core`.

#### Scenario: glossary file is included in prompt

- **GIVEN** the user provides `--glossary glossary.txt`
- **WHEN** the command builds the translation request
- **THEN** the command SHALL read `glossary.txt` as a UTF-8 text file
- **AND** the file content SHALL be handed to the engine as terminology guidance
- **AND** the command SHALL still require the AI response to use the structured cue ID mapping

#### Scenario: missing glossary file is rejected

- **GIVEN** the user provides `--glossary missing.txt`
- **WHEN** the command validates translation inputs
- **THEN** the command SHALL return an invalid path or input error
- **AND** SHALL NOT send an AI translation request
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
