## Context

SubX-CLI currently provides an AI-assisted subtitle workflow for matching
subtitle files to video files, converting formats, detecting encodings, and
synchronizing subtitle timing. Translation is a natural extension of this
workflow, but today users must export subtitles to a separate tool after SubX
has already parsed, matched, synced, and converted them.

The codebase already has the main primitives needed for a translation command:
subtitle parsing/writing in `src/core/formats/`, provider-neutral AI clients in
`src/services/ai/`, retry and timeout handling, testable configuration
services, input path handling, archive extraction, and command dispatch. The
implementation should reuse those primitives rather than introducing a new AI
SDK or subtitle data model.

## Goals / Non-Goals

**Goals:**

- Add a `subx translate` command that translates subtitle cue text into a
  target language while preserving timing, order, and metadata supported by the
  existing parser/writer pipeline.
- Reuse the configured AI provider and existing retry, timeout, error
  sanitization, and test-mocking patterns.
- Keep recurring proper nouns consistent by extracting a terminology map before
  translating cue batches.
- Support the same input styles users expect from mutating subtitle commands:
  direct files, directories, recursion, repeated `-i`, and archive-expanded
  inputs.
- Provide safe output behavior with non-destructive defaults, explicit
  overwrite/replacement, backups, and per-file error isolation.
- Make translation behavior testable with deterministic mock provider
  responses and structured response parsing.

**Non-Goals:**

- Offline machine translation or bundling translation models.
- Speech-to-text transcription from video audio.
- Changing matching, sync, or conversion semantics.
- Guaranteeing perfect translation quality or domain-specific localization.
- Supporting binary/image subtitle OCR as part of this change.

## Decisions

### Use a translation engine above existing subtitle formats

Create a translation component that accepts parsed `Subtitle` data, constructs
AI batches from cue text, applies translated text back to entries, and writes
through the existing format manager/converter path.

Alternatives considered:

- Translating raw file text: simpler initially, but risks corrupting timestamps,
  indices, ASS tags, and format headers.
- Adding translation logic inside each format parser: duplicates AI batching and
  makes behavior inconsistent between formats.

### Preserve only supported formatting metadata and translate visible text

For SRT/VTT/SUB, translate cue text while preserving timing and cue order. For
ASS/SSA, preserve style metadata and override tags to the extent supported by
the current parser/writer model, translating only visible dialogue text. The
initial feature should not promise full format round-trip fidelity beyond what
the existing format layer can represent.

Alternatives considered:

- Plain-text-only output for every input: easier, but loses useful metadata for
  formats that the existing pipeline can preserve.
- Full ASS semantic transformation: higher fidelity, but much larger scope and
  not required for an initial translation capability.

### Use structured AI responses with batch IDs

Translation prompts should include stable cue IDs and require a JSON response
mapping each cue ID to translated text. The parser should reject malformed or
duplicate IDs. If a response contains unknown IDs that were not requested for
that batch, the engine should treat the response as hallucinated, discard the
entire batch response, retry that same batch once, and fail the file if the
retry still contains unknown IDs. If a response omits otherwise valid requested
IDs, the engine should collect the missing cue IDs, finish all initial
translation batches, resend only the missing cue content once, and then fill any
still-missing translations with empty strings so output generation can continue
without silently reordering or misapplying text. Cue IDs should be request-local
identifiers and should not be written into the translated subtitle output.

Cue IDs should be UUIDv7 values generated in subtitle cue order. The generation
process should intentionally wait at least 1ms between adjacent UUIDv7 values so
each ID's `unix_time_ts` is strictly greater than the previous cue ID timestamp.
This avoids same-millisecond ambiguity and makes request ordering auditable from
the IDs themselves.

Alternatives considered:

- Asking the model to return a full translated subtitle file: prone to timing
  and formatting corruption.
- Accepting free-form translated lines: hard to validate when line wrapping or
  cue counts change.
- Sequential integer IDs: easy to read, but less robust if request fragments are
  logged or merged across batches.
- UUIDv4 IDs: unique, but they do not encode generation order.

### Run a terminology extraction pass before translation

Before translating cue batches, the engine should send subtitle text to the AI
provider in a terminology-extraction mode. The response should be a structured
term map for recurring proper nouns, especially people and places. Translation
batches should include this map and explicitly instruct the model to use it.

The extraction prompt should encode the naming policy:

- If a target-language conventional translation already exists, use it.
- If no conventional translation exists and a new translation must be coined,
  prefer phonetic transliteration.
- Use semantic translation only when transliteration is unsuitable or would be
  misleading.

User-provided glossary entries should take precedence over generated terms
because they represent explicit user intent. Generated terms can then fill gaps
not covered by the glossary.

Alternatives considered:

- Relying on one-shot translation prompts: simpler and cheaper, but recurring
  names can drift across batches.
- Asking the model to self-enforce consistency without a structured map: less
  reliable and harder to test.
- Requiring users to provide every term manually: precise, but poor default UX
  for long subtitles.

### Keep provider integration generic

The first implementation can use the existing chat-completion provider
abstraction and shared prompt/response helpers. A dedicated translation method
may be added if it improves type safety, but translation should still route
through `ComponentFactory` and the configured provider.

Alternatives considered:

- Provider-specific translation endpoints: can improve quality for some
  vendors but would fragment behavior and add configuration complexity.
- A new third-party translation dependency: unnecessary for an AI-first
  feature and increases supply-chain surface.

### Default to non-destructive output

By default, translated files should be written beside the source using a target
language suffix, such as `movie.zh-TW.srt`, without deleting or modifying the
source. Replacement/overwrite behavior should require explicit CLI flags and
reuse existing backup settings.

Alternatives considered:

- Replace input files by default, matching some conversion behavior: risky for
  AI-generated text and hard to undo if quality is poor.
- Always require `--output`: safe but unnecessarily verbose for common use.

### Treat glossary and context as separate prompt inputs

`--glossary` should reference a UTF-8 text file containing terminology guidance.
`--context` should accept short inline project/domain guidance. Keeping these
inputs separate avoids ambiguity and makes CLI validation, documentation, and
tests straightforward.

Alternatives considered:

- One overloaded option accepting either a path or inline text: convenient, but
  ambiguous and hard to validate reliably.
- Only glossary files: misses simple one-off context such as "use formal tone"
  or "anime fansub terminology".

## Risks / Trade-offs

- AI providers may return malformed JSON, hallucinated cue IDs, or omit cue IDs
  -> reject malformed and duplicate IDs; discard and retry batches with unknown
  IDs once before failing; retry omitted IDs once after the initial batch pass;
  and use empty-string fallback for IDs still omitted by the retry.
- UUIDv7 generation with 1ms spacing adds latency for large files -> generate
  IDs once before AI requests, document the trade-off, and keep batching
  independent from ID generation.
- Terminology extraction may miss or over-classify names -> validate the term
  map schema, allow empty maps, and let explicit glossary entries override
  generated terms.
- Long subtitle files may exceed token limits -> batch cues with configurable
  batch size and preserve ordering across batches.
- Translation quality may vary by provider/model -> allow glossary/context
  guidance and document that quality depends on the configured provider.
- ASS/SSA and VTT styling may contain constructs the current format layer cannot
  round-trip perfectly -> document preservation as limited to supported
  metadata and add tests around the supported behavior.
- Translation can be expensive -> show progress, batch requests, and consider
  cache integration in a future proposal after the initial command contract is
  stable.
- Overwrite mode can destroy source text -> require explicit flags and use the
  existing backup/file-operation safety mechanisms.
