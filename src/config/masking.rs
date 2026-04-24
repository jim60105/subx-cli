//! Utilities for masking sensitive configuration values for display.
//!
//! Configuration values such as API keys, tokens, and secrets must never be
//! printed in their raw form to terminal output or log files. This module
//! provides a single helper, [`mask_sensitive_value`], that inspects the
//! configuration key and, when it refers to a sensitive item, replaces the
//! value with a safe masked representation.

/// Return a display-safe representation of a configuration value.
///
/// If `key` (case-insensitive) contains any of the sensitive markers
/// (`api_key`, `token`, `secret`), the value is replaced with a masked form:
///
/// * Values of 4 characters or fewer are fully masked as `****`.
/// * Longer values keep their last 4 characters prefixed by `****` to help
///   users identify which key is configured without leaking it.
///
/// Non-sensitive keys return the value unchanged.
pub fn mask_sensitive_value(key: &str, value: &str) -> String {
    let key_lower = key.to_lowercase();
    let is_sensitive = key_lower.contains("api_key")
        || key_lower.contains("token")
        || key_lower.contains("secret");

    if !is_sensitive {
        return value.to_string();
    }

    if value.is_empty() {
        return String::new();
    }

    if value.chars().count() <= 4 {
        "****".to_string()
    } else {
        let tail: String = value
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("****{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_sensitive_key_returns_value_unchanged() {
        assert_eq!(mask_sensitive_value("ai.model", "gpt-4"), "gpt-4");
        assert_eq!(mask_sensitive_value("formats.default_output", "srt"), "srt");
    }

    #[test]
    fn short_sensitive_value_is_fully_masked() {
        assert_eq!(mask_sensitive_value("ai.api_key", "abcd"), "****");
        assert_eq!(mask_sensitive_value("ai.api_key", "a"), "****");
    }

    #[test]
    fn long_sensitive_value_preserves_last_four() {
        assert_eq!(
            mask_sensitive_value("ai.api_key", "sk-1234567890abcdef"),
            "****cdef"
        );
    }

    #[test]
    fn empty_sensitive_value_stays_empty() {
        assert_eq!(mask_sensitive_value("ai.api_key", ""), "");
    }

    #[test]
    fn matching_is_case_insensitive_and_substring() {
        assert_eq!(mask_sensitive_value("AI.API_KEY", "abcdefgh"), "****efgh");
        assert_eq!(
            mask_sensitive_value("auth.access_token", "tokenvalue12345"),
            "****2345"
        );
        assert_eq!(
            mask_sensitive_value("client.secret", "supersecretvalue"),
            "****alue"
        );
    }
}
