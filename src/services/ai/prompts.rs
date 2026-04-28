use crate::Result;
use crate::error::SubXError;
use crate::services::ai::{AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest};
use serde_json;

/// Prompt builder trait for AI providers.
pub trait PromptBuilder {
    /// Build analysis prompt.
    fn build_analysis_prompt(&self, request: &AnalysisRequest) -> String {
        build_analysis_prompt_base(request)
    }

    /// Build verification prompt.
    fn build_verification_prompt(&self, request: &VerificationRequest) -> String {
        build_verification_prompt_base(request)
    }

    /// System message for analysis prompt.
    fn get_analysis_system_message() -> &'static str {
        "You are an expert subtitle-matching assistant."
    }

    /// System message for verification prompt.
    fn get_verification_system_message() -> &'static str {
        "You are an expert subtitle-matching verifier."
    }
}

/// Response parsing trait for AI providers.
pub trait ResponseParser {
    /// Parse match result.
    fn parse_match_result(&self, response: &str) -> Result<MatchResult> {
        parse_match_result_base(response)
    }

    /// Parse confidence score.
    fn parse_confidence_score(&self, response: &str) -> Result<ConfidenceScore> {
        parse_confidence_score_base(response)
    }
}

/// Escape a string for inclusion in an XML attribute value.
fn xml_escape_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Wrap `content` in CDATA, neutralizing any literal `]]>` substring by
/// splitting it across two CDATA sections so the surrounding XML
/// structure remains parseable.
fn cdata_wrap(content: &str) -> String {
    let safe = content.replace("]]>", "]]]]><![CDATA[>");
    format!("<![CDATA[{}]]>", safe)
}

/// Tolerant parser for the legacy `"ID:… | Name:… | Path:…"` strings that
/// callers pass into [`AnalysisRequest::video_files`] and
/// [`AnalysisRequest::subtitle_files`]. Returns `(id, name, path)` or
/// `None` if the input does not look like that shape; the caller falls
/// back to embedding the raw string verbatim inside a `<file>` element.
fn parse_id_name_path(entry: &str) -> Option<(String, String, String)> {
    let mut id = None;
    let mut name = None;
    let mut path = None;
    for part in entry.split(" | ") {
        if let Some(rest) = part.strip_prefix("ID:") {
            id = Some(rest.trim().to_string());
        } else if let Some(rest) = part.strip_prefix("Name:") {
            name = Some(rest.trim().to_string());
        } else if let Some(rest) = part.strip_prefix("Path:") {
            path = Some(rest.trim().to_string());
        }
    }
    match (id, name, path) {
        (Some(i), Some(n), Some(p)) => Some((i, n, p)),
        _ => None,
    }
}

/// Render a single file-inventory entry as `<file id="…" name="…" path="…"/>`.
fn render_file_element(entry: &str) -> String {
    if let Some((id, name, path)) = parse_id_name_path(entry) {
        format!(
            "  <file id=\"{}\" name=\"{}\" path=\"{}\"/>\n",
            xml_escape_attr(&id),
            xml_escape_attr(&name),
            xml_escape_attr(&path)
        )
    } else {
        // Fallback: embed the raw string verbatim so we never lose data.
        format!(
            "  <file><![CDATA[{}]]></file>\n",
            entry.replace("]]>", "]]]]><![CDATA[>")
        )
    }
}

/// Build analysis prompt for AI providers.
///
/// Emits an XML-tagged prompt body following the Claude prompt-engineering
/// best-practices guide: a `<role>` description, a `<instructions>` block
/// with Markdown-headed sub-sections, separate `<video_files>` and
/// `<subtitle_files>` inventories, a `<content_samples>` block keyed by
/// stable `subtitle_file_id`, an `<output_schema>` block describing the
/// expected JSON response, and a single `<example>` for the model to
/// imitate.
pub fn build_analysis_prompt_base(request: &AnalysisRequest) -> String {
    let mut p = String::new();
    p.push_str("<role>You are an expert subtitle-matching assistant. Pair each subtitle file with the video it belongs to using filename evidence and the subtitle preview text.</role>\n");
    p.push_str("<instructions>\n");
    p.push_str("## Task\n");
    p.push_str("Analyze the inventories below and return JSON-only output that pairs every confidently matched subtitle with its video. Use the file IDs verbatim — never invent new IDs and never echo filenames as IDs.\n\n");
    p.push_str("## Language Inference Rules\n");
    p.push_str("Use BOTH the filename and the `<content_samples>` text to infer each subtitle's language. The content sample is the primary signal: when filename and content disagree, trust the content. Output a short language code (e.g. `tc`, `sc`, `en`, `ja`, `ko`, `fr`, `de`, `es`, `pt`, `ru`). Use `und` only when the language genuinely cannot be determined.\n\n");
    p.push_str("## Naming and Uniqueness Rules\n");
    p.push_str("Across the entire `matches[]` array — not just within a single video — no two entries MAY produce the same final filename in the same target directory. When two subtitles for the same video would otherwise collide, supply distinct `language` codes. If both share the same language, supply a numeric `target_filename_suffix` (`2`, `3`, …) starting at `2`. The suffix MUST be one short token of `[A-Za-z0-9_-]` and at most 16 characters; it is spliced between the video base name and the subtitle extension.\n\n");
    p.push_str("## Output Schema\n");
    p.push_str("Respond with a single JSON object — no prose, no Markdown fences. Keys are stable: `matches[]` (array of objects with `video_file_id`, `subtitle_file_id`, `confidence`, `match_factors`, optional `language`, optional `target_filename_suffix`), `confidence` (overall score), and `reasoning` (short English explanation).\n");
    p.push_str("</instructions>\n");

    p.push_str("<video_files>\n");
    for entry in &request.video_files {
        p.push_str(&render_file_element(entry));
    }
    p.push_str("</video_files>\n");

    p.push_str("<subtitle_files>\n");
    for entry in &request.subtitle_files {
        p.push_str(&render_file_element(entry));
    }
    p.push_str("</subtitle_files>\n");

    if !request.content_samples.is_empty() {
        // Each `<sample>` is keyed by `subtitle_file_id` so the model
        // links samples back to entries in `<subtitle_files>` by ID
        // alone — no filename leak that could be echoed as an ID.
        p.push_str("<content_samples_instructions>Each `<sample>` element's `subtitle_file_id` attribute matches the `id` attribute on the corresponding `<file>` element in `<subtitle_files>`. Resolve sample-to-subtitle relationships through this ID, never by filename.</content_samples_instructions>\n");
        p.push_str("<content_samples>\n");
        for sample in &request.content_samples {
            p.push_str(&format!(
                "  <sample subtitle_file_id=\"{}\">{}</sample>\n",
                xml_escape_attr(&sample.subtitle_file_id),
                cdata_wrap(&sample.content_preview)
            ));
        }
        p.push_str("</content_samples>\n");
    }

    p.push_str("<output_schema>\n");
    p.push_str("{\n");
    p.push_str("  \"matches\": [\n");
    p.push_str("    {\n");
    p.push_str("      \"video_file_id\": \"file_<uuid>\",\n");
    p.push_str("      \"subtitle_file_id\": \"file_<uuid>\",\n");
    p.push_str("      \"confidence\": 0.95,\n");
    p.push_str("      \"match_factors\": [\"filename_similarity\", \"content_correlation\"],\n");
    p.push_str("      \"language\": \"tc\",\n");
    p.push_str("      \"target_filename_suffix\": \"tc\"\n");
    p.push_str("    }\n");
    p.push_str("  ],\n");
    p.push_str("  \"confidence\": 0.9,\n");
    p.push_str("  \"reasoning\": \"Short English explanation\"\n");
    p.push_str("}\n");
    p.push_str("</output_schema>\n");

    p.push_str("<example>\n");
    p.push_str("  <input_summary>One video plus two same-named subtitles whose content samples reveal Traditional vs Simplified Chinese.</input_summary>\n");
    p.push_str("  <output>{\"matches\":[{\"video_file_id\":\"file_v1\",\"subtitle_file_id\":\"file_s1\",\"confidence\":0.95,\"match_factors\":[\"content_correlation\"],\"language\":\"tc\",\"target_filename_suffix\":\"tc\"},{\"video_file_id\":\"file_v1\",\"subtitle_file_id\":\"file_s2\",\"confidence\":0.95,\"match_factors\":[\"content_correlation\"],\"language\":\"sc\",\"target_filename_suffix\":\"sc\"}],\"confidence\":0.95,\"reasoning\":\"Distinguished by script\"}</output>\n");
    p.push_str("</example>\n");

    p
}

/// Parse matching results from AI response.
pub fn parse_match_result_base(response: &str) -> Result<MatchResult> {
    let json_start = response.find('{').unwrap_or(0);
    let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
    let json_str = &response[json_start..json_end];
    serde_json::from_str(json_str)
        .map_err(|e| SubXError::AiService(format!("AI response parsing failed: {}", e)))
}

/// Build verification prompt for AI providers.
pub fn build_verification_prompt_base(request: &VerificationRequest) -> String {
    let mut p = String::new();
    p.push_str("<role>You are an expert subtitle-matching verifier.</role>\n");
    p.push_str("<instructions>\n");
    p.push_str("## Task\n");
    p.push_str("Score the supplied video/subtitle pairing on a 0.0–1.0 confidence scale and return strictly JSON.\n\n");
    p.push_str("## Output Schema\n");
    p.push_str("Respond with a single JSON object: `{\"score\": <0..1>, \"factors\": [\"...\"]}`. No prose, no Markdown.\n");
    p.push_str("</instructions>\n");

    p.push_str("<match>\n");
    p.push_str(&format!(
        "  <video path=\"{}\"/>\n",
        xml_escape_attr(&request.video_file)
    ));
    p.push_str(&format!(
        "  <subtitle path=\"{}\"/>\n",
        xml_escape_attr(&request.subtitle_file)
    ));
    p.push_str("  <factors>\n");
    if request.match_factors.is_empty() {
        // Preserve a recognizable "Matching factors:" anchor for tests
        // and operators reading raw prompts.
        p.push_str("    <!-- Matching factors: (none) -->\n");
    } else {
        for factor in &request.match_factors {
            p.push_str(&format!(
                "    <factor>{}</factor>\n",
                xml_escape_attr(factor)
            ));
        }
        p.push_str("    <!-- Matching factors: list above -->\n");
    }
    p.push_str("  </factors>\n");
    p.push_str("</match>\n");

    p.push_str("<output_schema>{\"score\": 0.9, \"factors\": [\"...\"]}</output_schema>\n");
    // Keep a literal "JSON format" anchor for legacy assertions.
    p.push_str("<!-- Respond strictly in JSON format. -->\n");
    p
}

/// Parse confidence score from AI response.
pub fn parse_confidence_score_base(response: &str) -> Result<ConfidenceScore> {
    let json_start = response.find('{').unwrap_or(0);
    let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
    let json_str = &response[json_start..json_end];
    serde_json::from_str(json_str)
        .map_err(|e| SubXError::AiService(format!("AI confidence parsing failed: {}", e)))
}

#[cfg(test)]
mod tests {

    use crate::services::ai::prompts::{
        PromptBuilder, ResponseParser, build_analysis_prompt_base, parse_match_result_base,
    };
    use crate::services::ai::{AnalysisRequest, ContentSample, OpenAIClient};

    #[test]
    fn test_ai_prompt_with_file_ids_english() {
        let client = OpenAIClient::new("test_key".into(), "gpt-4.1".into(), 0.1, 1000, 0, 0);
        let video_id = "file_019dcc51-f7da-74e3-9e0d-f75d40fc569c";
        let subtitle_id = "file_019dcc51-f7d5-7640-8bb1-d2bbbc127a23";
        let request = AnalysisRequest {
            video_files: vec![format!("ID:{video_id} | Name:movie.mkv | Path:movie.mkv")],
            subtitle_files: vec![format!(
                "ID:{subtitle_id} | Name:movie.srt | Path:movie.srt"
            )],
            content_samples: vec![],
        };

        let prompt = client.build_analysis_prompt(&request);

        assert!(prompt.contains(&format!("id=\"{video_id}\"")));
        assert!(prompt.contains(&format!("id=\"{subtitle_id}\"")));
        assert!(prompt.contains("<role>"));
        assert!(prompt.contains("<instructions>"));
        assert!(prompt.contains("<video_files>"));
        assert!(prompt.contains("<subtitle_files>"));
        assert!(prompt.contains("<output_schema>"));
        assert!(prompt.contains("Language Inference Rules"));
        assert!(prompt.contains("Naming and Uniqueness Rules"));
        assert!(prompt.contains("video_file_id"));
        assert!(prompt.contains("subtitle_file_id"));
        assert!(!prompt.contains("請分析"));
    }

    #[test]
    fn test_parse_match_result_with_ids() {
        let client = OpenAIClient::new("test_key".into(), "gpt-4.1".into(), 0.1, 1000, 0, 0);
        let video_id = "file_019dcc51-f7da-74e3-9e0d-f75d40fc569c";
        let subtitle_id = "file_019dcc51-f7d5-7640-8bb1-d2bbbc127a23";
        let json_resp = format!(
            r#"{{
            "matches": [{{
                "video_file_id": "{video_id}",
                "subtitle_file_id": "{subtitle_id}",
                "confidence": 0.95,
                "match_factors": ["filename_similarity"]
            }}],
            "confidence": 0.9,
            "reasoning": "Strong match based on filename patterns"
        }}"#
        );

        let result = client.parse_match_result(&json_resp).unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].video_file_id, video_id);
        assert_eq!(result.matches[0].subtitle_file_id, subtitle_id);
        assert_eq!(result.matches[0].confidence, 0.95);
        assert_eq!(result.matches[0].match_factors[0], "filename_similarity");
        assert!(result.matches[0].language.is_none());
        assert!(result.matches[0].target_filename_suffix.is_none());
    }

    #[test]
    fn test_ai_prompt_structure_consistency() {
        let client = OpenAIClient::new("test_key".into(), "gpt-4.1".into(), 0.1, 1000, 0, 0);
        let request = AnalysisRequest {
            video_files: vec![
                "ID:file_video1 | Name:video1.mkv | Path:season1/video1.mkv".into(),
                "ID:file_video2 | Name:video2.mkv | Path:season1/video2.mkv".into(),
            ],
            subtitle_files: vec![
                "ID:file_sub1 | Name:sub1.srt | Path:season1/sub1.srt".into(),
                "ID:file_sub2 | Name:sub2.srt | Path:season1/sub2.srt".into(),
            ],
            content_samples: vec![],
        };

        let prompt = client.build_analysis_prompt(&request);

        assert!(prompt.contains("id=\"file_video1\""));
        assert!(prompt.contains("id=\"file_video2\""));
        assert!(prompt.contains("id=\"file_sub1\""));
        assert!(prompt.contains("id=\"file_sub2\""));
        assert!(prompt.contains("<video_files>"));
        assert!(prompt.contains("<subtitle_files>"));
        assert!(prompt.contains("<output_schema>"));
    }

    #[test]
    fn test_parse_confidence_score() {
        let client = OpenAIClient::new("test_key".into(), "gpt-4.1".into(), 0.1, 1000, 0, 0);
        let json_resp = r#"{
            "score": 0.88,
            "factors": ["filename_similarity", "content_correlation"]
        }"#;

        let result = client.parse_confidence_score(json_resp).unwrap();
        assert_eq!(result.score, 0.88);
        assert_eq!(
            result.factors,
            vec![
                "filename_similarity".to_string(),
                "content_correlation".to_string()
            ]
        );
    }

    #[test]
    fn test_xml_escapes_filename_with_ampersand() {
        let request = AnalysisRequest {
            video_files: vec!["ID:vid1 | Name:m & m.mkv | Path:/x/m & m.mkv".into()],
            subtitle_files: vec!["ID:sub1 | Name:m & m.srt | Path:/x/m & m.srt".into()],
            content_samples: vec![],
        };
        let prompt = build_analysis_prompt_base(&request);
        assert!(
            prompt.contains("name=\"m &amp; m.mkv\""),
            "ampersand must be XML-escaped"
        );
        assert!(prompt.contains("path=\"/x/m &amp; m.mkv\""));
        assert!(!prompt.contains("name=\"m & m.mkv\""));
    }

    #[test]
    fn test_xml_escapes_all_five_entities() {
        // `xml_escape_attr` covers `&`, `<`, `>`, `"`, and `'`. A
        // realistic filename can contain every one at once; verify they
        // all materialize as their entity references and that the raw
        // characters disappear from the rendered attribute.
        let raw_name = "weird & <stuff> \"x\" 'y'.mkv";
        let raw_path = "/x/weird & <stuff> \"x\" 'y'.mkv";
        let request = AnalysisRequest {
            video_files: vec![format!("ID:vid1 | Name:{} | Path:{}", raw_name, raw_path)],
            subtitle_files: vec!["ID:sub1 | Name:s.srt | Path:/x/s.srt".into()],
            content_samples: vec![],
        };
        let prompt = build_analysis_prompt_base(&request);
        let expected_attr = "weird &amp; &lt;stuff&gt; &quot;x&quot; &#39;y&#39;.mkv";
        assert!(
            prompt.contains(&format!("name=\"{}\"", expected_attr)),
            "all five XML entities must be escaped: prompt was\n{prompt}"
        );
        // Path attribute uses the same escaper.
        let expected_path = "/x/weird &amp; &lt;stuff&gt; &quot;x&quot; &#39;y&#39;.mkv";
        assert!(prompt.contains(&format!("path=\"{}\"", expected_path)));
        // The raw, unescaped form must be absent from any attribute.
        assert!(!prompt.contains(&format!("name=\"{}\"", raw_name)));
    }

    #[test]
    fn test_sample_uses_id_only_no_filename_attribute() {
        // The `<sample>` element must key off `subtitle_file_id` alone;
        // exposing `filename=` would weaken the ID-based contract and
        // tempt the model to echo filenames as IDs. Also assert the
        // dedicated instruction sentence describing the ID linkage is
        // present.
        let request = AnalysisRequest {
            video_files: vec!["ID:vid1 | Name:movie.mkv | Path:/x/movie.mkv".into()],
            subtitle_files: vec!["ID:sub1 | Name:movie.srt | Path:/x/movie.srt".into()],
            content_samples: vec![ContentSample {
                filename: "movie.srt".into(),
                subtitle_file_id: "sub1".into(),
                content_preview: "hi".into(),
                file_size: 2,
            }],
        };
        let prompt = build_analysis_prompt_base(&request);
        assert!(
            prompt.contains("<sample subtitle_file_id=\"sub1\">"),
            "sample must use only subtitle_file_id; got\n{prompt}"
        );
        assert!(
            !prompt.contains("filename=\""),
            "sample element must not carry a filename attribute"
        );
        assert!(
            prompt.contains("<content_samples_instructions>"),
            "ID-linkage instruction sentence must be emitted"
        );
        assert!(prompt.contains("subtitle_file_id"));
    }

    #[test]
    fn test_cdata_preserves_italic_markup() {
        let request = AnalysisRequest {
            video_files: vec!["ID:vid1 | Name:movie.mkv | Path:/x/movie.mkv".into()],
            subtitle_files: vec!["ID:sub1 | Name:movie.srt | Path:/x/movie.srt".into()],
            content_samples: vec![ContentSample {
                filename: "movie.srt".into(),
                subtitle_file_id: "sub1".into(),
                content_preview: "Hello <i>world</i>".into(),
                file_size: 10,
            }],
        };
        let prompt = build_analysis_prompt_base(&request);
        assert!(prompt.contains("subtitle_file_id=\"sub1\""));
        assert!(prompt.contains("<![CDATA[Hello <i>world</i>]]>"));
    }

    #[test]
    fn test_cdata_neutralizes_terminator() {
        let request = AnalysisRequest {
            video_files: vec!["ID:vid1 | Name:movie.mkv | Path:/x/movie.mkv".into()],
            subtitle_files: vec!["ID:sub1 | Name:movie.srt | Path:/x/movie.srt".into()],
            content_samples: vec![ContentSample {
                filename: "movie.srt".into(),
                subtitle_file_id: "sub1".into(),
                content_preview: "weird ]]> sequence".into(),
                file_size: 10,
            }],
        };
        let prompt = build_analysis_prompt_base(&request);
        // The literal "]]>" must not appear inside the original content
        // CDATA without being split.
        let cdata_start = prompt.find("<![CDATA[weird ").unwrap();
        let split_marker = "]]]]><![CDATA[>";
        assert!(
            prompt[cdata_start..].contains(split_marker),
            "raw ]]> must be neutralized"
        );
    }

    #[test]
    fn test_round_trip_with_optional_fields() {
        let json_resp = r#"{
            "matches": [{
                "video_file_id": "vid1",
                "subtitle_file_id": "sub1",
                "confidence": 0.9,
                "match_factors": ["content_correlation"],
                "language": "tc",
                "target_filename_suffix": "tc"
            }],
            "confidence": 0.9,
            "reasoning": "ok"
        }"#;
        let result = parse_match_result_base(json_resp).unwrap();
        assert_eq!(result.matches[0].language.as_deref(), Some("tc"));
        assert_eq!(
            result.matches[0].target_filename_suffix.as_deref(),
            Some("tc")
        );
    }
}
