## MODIFIED Requirements

### Requirement: Shared Prompt and Response Schema

The system SHALL build analysis and verification prompts in English via the shared `build_analysis_prompt_base` / `build_verification_prompt_base` functions, structuring them with XML tags (`<role>`, `<instructions>`, `<video_files>`, `<subtitle_files>`, `<content_samples>`, `<output_schema>`, `<example>`) and Markdown headings inside `<instructions>`.

The analysis prompt SHALL:

1. Render every video and subtitle file as a `<file id="…" name="…" path="…"/>` element with all attribute values XML-attribute-escaped.
2. Render every content sample as `<sample subtitle_file_id="…"><![CDATA[…preview…]]></sample>`, where `subtitle_file_id` matches a `<file id="…"/>` in `<subtitle_files>` exactly. The literal substring `]]>` inside any preview SHALL be neutralized by splitting it as `]]]]><![CDATA[>` so CDATA cannot be terminated prematurely.
3. Contain a `## Language Inference Rules` section telling the model to use both filename and content text and to prefer content evidence when they disagree.
4. Contain a `## Naming and Uniqueness Rules` section explicitly forbidding two `matches[]` entries from producing the same final filename in the same target directory **across the entire response** (not only within the same video), and instructing the model to disambiguate via distinct `language` codes or, as a last resort, a numeric `target_filename_suffix`.

The system SHALL require providers to respond with JSON that deserializes into `MatchResult` / `ConfidenceScore`. `FileMatch` SHALL accept two optional, additive fields: `language` (a short language code such as `tc`, `sc`, `en`, `ja`, or `und`) and `target_filename_suffix` (a disambiguating tag such as `tc`, `en`, `2`). Both fields SHALL be deserialized with `#[serde(default)]` so older cached responses remain valid.

`ContentSample` SHALL include a non-optional `subtitle_file_id: String` field populated from the same UUIDv7 the engine assigned to the originating subtitle, so previews can be unambiguously linked to subtitles even when filenames are not unique.

#### Scenario: Prompt contains stable contract
- **GIVEN** an `AnalysisRequest` with video file IDs and subtitle file IDs
- **WHEN** `build_analysis_prompt_base` runs
- **THEN** the generated prompt SHALL instruct the model to respond with a JSON object containing `matches[].video_file_id`, `matches[].subtitle_file_id`, `matches[].confidence`, `confidence`, and `reasoning`, and the prompt SHALL describe `matches[].language` and `matches[].target_filename_suffix` as optional fields the model MAY supply

#### Scenario: Prompt is XML-structured per Claude best-practices
- **GIVEN** any non-empty `AnalysisRequest`
- **WHEN** `build_analysis_prompt_base` runs
- **THEN** the prompt SHALL contain top-level XML elements named `<role>`, `<instructions>`, `<video_files>`, `<subtitle_files>`, and `<output_schema>`, and each video / subtitle file SHALL be rendered as a `<file id="…" name="…" path="…"/>` element

#### Scenario: Content samples reference subtitles by ID
- **GIVEN** an `AnalysisRequest` with two subtitles whose `name` is identical (`subs.srt`) but whose `subtitle_file_id` differs and a `content_samples` entry exists for each
- **WHEN** `build_analysis_prompt_base` runs
- **THEN** each `<sample>` element SHALL carry a `subtitle_file_id` attribute that matches exactly one `<file id="…"/>` in `<subtitle_files>` and SHALL NOT rely on filename to associate the preview with the subtitle

#### Scenario: Subtitle preview markup is preserved without breaking XML
- **GIVEN** a `ContentSample` whose `content_preview` contains the substring `<i>italic</i>` and the substring `]]>`
- **WHEN** `build_analysis_prompt_base` runs
- **THEN** the preview SHALL appear inside a CDATA section, the `<i>` markup SHALL be preserved verbatim inside the CDATA, and the literal `]]>` SHALL be neutralized by splitting it across two CDATA sections so the surrounding XML structure remains parseable

#### Scenario: Prompt instructs language inference from content and filename
- **GIVEN** an `AnalysisRequest` with non-empty `content_samples`
- **WHEN** `build_analysis_prompt_base` runs
- **THEN** the prompt SHALL contain a `## Language Inference Rules` section that tells the model to use both the filename and the `<content_samples>` text, and to prefer the content text when the two disagree

#### Scenario: Prompt forbids duplicate final target paths across the response
- **GIVEN** an `AnalysisRequest`
- **WHEN** `build_analysis_prompt_base` runs
- **THEN** the prompt SHALL contain a `## Naming and Uniqueness Rules` section that explicitly forbids two `matches[]` entries — for the same video or for different videos — from producing the same final filename in the same target directory, and instructs the model to disambiguate via distinct `language` codes or, as a last resort, a numeric `target_filename_suffix`

#### Scenario: Optional fields deserialize from legacy responses
- **GIVEN** an AI response JSON whose `matches[]` entries contain only the legacy fields (no `language`, no `target_filename_suffix`)
- **WHEN** `parse_match_result_base` runs
- **THEN** parsing SHALL succeed and the resulting `FileMatch.language` and `FileMatch.target_filename_suffix` SHALL both be `None`

#### Scenario: Optional fields deserialize when present
- **GIVEN** an AI response JSON whose `matches[]` entries include `"language": "tc"` and `"target_filename_suffix": "tc"`
- **WHEN** `parse_match_result_base` runs
- **THEN** parsing SHALL succeed and the resulting `FileMatch.language` SHALL be `Some("tc")` and `FileMatch.target_filename_suffix` SHALL be `Some("tc")`

#### Scenario: Unparseable response yields a typed error
- **GIVEN** a provider returns text that cannot be parsed as the expected JSON schema
- **WHEN** `parse_match_result_base` runs
- **THEN** it SHALL return `SubXError::AiService` with a message indicating `AI response parsing failed`
