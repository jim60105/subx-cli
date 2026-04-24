//! Error message sanitization helpers for AI service clients.
//!
//! These helpers prevent leaking sensitive information (overly long error
//! bodies, URL query strings that may contain API keys or tokens) into
//! user-facing error messages.

use std::sync::OnceLock;

use regex::Regex;

/// Maximum number of characters preserved from an upstream error body.
pub const DEFAULT_ERROR_BODY_MAX_LEN: usize = 500;

/// Truncate an error response body to at most `max_len` characters.
///
/// If the body exceeds `max_len`, the returned string is truncated at the
/// nearest UTF-8 character boundary not exceeding `max_len` and suffixed
/// with `"... (truncated)"`.
pub fn truncate_error_body(body: &str, max_len: usize) -> String {
    if body.len() <= max_len {
        return body.to_string();
    }
    let mut cut = max_len;
    while cut > 0 && !body.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}... (truncated)", &body[..cut])
}

fn url_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(https?://[^\s?#]+)\?[^\s]*").expect("valid URL regex"))
}

/// Strip query parameters from any URLs appearing in an error message.
///
/// Query strings can contain sensitive material such as API keys or session
/// tokens (e.g. Azure AD access tokens passed via `?api-key=...`). This
/// helper removes everything from the `?` up to the next whitespace
/// character, leaving the URL path intact for debugging purposes.
pub fn sanitize_url_in_error(msg: &str) -> String {
    url_regex().replace_all(msg, "$1").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_shorter_than_limit_is_unchanged() {
        assert_eq!(truncate_error_body("abc", 500), "abc");
    }

    #[test]
    fn truncate_longer_is_cut_and_marked() {
        let body = "a".repeat(600);
        let out = truncate_error_body(&body, 500);
        assert!(out.ends_with("... (truncated)"));
        assert_eq!(out.len(), 500 + "... (truncated)".len());
    }

    #[test]
    fn truncate_respects_utf8_boundaries() {
        // "漢" is 3 bytes in UTF-8; truncating at 2 would split it.
        let body = "漢".repeat(300);
        let out = truncate_error_body(&body, 500);
        assert!(out.ends_with("... (truncated)"));
        // Prefix must still be valid UTF-8 (implicitly true if no panic).
        assert!(out.contains('漢'));
    }

    #[test]
    fn sanitize_strips_query_string() {
        let input = "request failed for https://api.example.com/v1/chat?api-key=sk-test-key-12345";
        let out = sanitize_url_in_error(input);
        assert_eq!(out, "request failed for https://api.example.com/v1/chat");
        assert!(!out.contains("sk-test-key"));
    }

    #[test]
    fn sanitize_leaves_urls_without_query_alone() {
        let input = "timeout on https://api.example.com/v1/chat";
        assert_eq!(sanitize_url_in_error(input), input);
    }

    #[test]
    fn sanitize_preserves_trailing_text_after_whitespace() {
        let input = "error at https://x.test/a?token=secret while retrying";
        let out = sanitize_url_in_error(input);
        assert_eq!(out, "error at https://x.test/a while retrying");
    }
}
