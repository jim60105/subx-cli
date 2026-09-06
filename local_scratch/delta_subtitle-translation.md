## REMOVED Requirements

### Requirement: Subtitle Structure Preservation

**Reason**: Core half of the split. Cue timing, ordering, count and format-metadata preservation are properties of `src/core/translation/` operating over `src/core/formats/`, both `subx-core` after B2, and the GUI constructs `TranslationEngine::new` directly (SDR §8). Re-added verbatim by `import-split-capability-specs`, at `openspec/specs/subtitle-translation/spec.md` in that repository. It leaves this repository's half of the capability; it does not leave the project.

### Requirement: AI Provider Translation

**Reason**: Core half of the split. The structured prompts, the request dispatch and the response parsing are in `src/core/translation/` and `src/services/ai/`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: Its *malformed AI response is rejected* scenario says "the command SHALL return a typed AI service error for that file" and "SHALL NOT write a partial translated subtitle for that file". Both are properties of the translation engine's return value, not of the command loop; the loop's obligation to keep going is this capability's *Per-File Error Isolation* requirement, which stays in `subx-cli`. On arrival "the command" SHALL be read as the translation of a single file, so that the arriving requirement stops at the engine boundary.

### Requirement: Stable Cue Mapping

**Reason**: Core half of the split. UUIDv7 cue-ID assignment, response validation against requested IDs, the missing-cue retry and the hallucinated-ID discard are all in `src/core/translation/engine.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Two-Pass Terminology Consistency

**Reason**: Core half of the split. The terminology-extraction pass, the structured map and the prompt assembly that includes it are in `src/core/translation/` and the prompt helpers under `src/services/ai/`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: Its *explicit glossary overrides generated terminology* scenario says "the user-provided glossary" without naming a flag; the glossary's *content* reaches the engine as parsed entries, and the file reading is `subx-cli`'s. On arrival "the user-provided glossary" SHALL be read as the glossary entries supplied by the caller, which is what the engine actually receives.

### Requirement: Batching and Ordering

**Reason**: Core half of the split — and the requirement most likely to be misfiled. Its second scenario requires the `Processed cues: <processed>/<total>` form, which reads like the progress vocabulary SDR §9 names as a CLI seam, but the string is produced at `src/core/translation/engine.rs:405` and after A1 it reaches the user through the `Reporter` seam rather than a direct write. Batch splitting, reassembly in original order, and the no-partial-output guarantee are all in the same file. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: The *translation progress logs processed cue count* scenario SHALL be carried over with "the command SHALL log" read as the engine emitting the message through its `Reporter`, per the `core-reporting` capability in `subx-core`, so that the arriving requirement does not assert that a library writes to a terminal.

### Requirement: Translation Configuration

**Reason**: Core half of the split. The translation batch-size and default-target-language settings are fields under `src/config/` and their validation is in `src/config/validator.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: The *configured batch size is used* scenario is phrased as `subx translate movie.srt --target-language fr`; the invocation is narrative framing over the batching the engine performs and SHALL be named as `subx-cli`'s translate command on arrival.

## MODIFIED Requirements

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
