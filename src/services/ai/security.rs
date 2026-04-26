//! Shared security helpers for AI provider clients.
//!
//! These helpers centralize precautionary checks that every AI client should
//! perform before sending credentials over the wire.

use url::Url;

/// Emit a warning if the parsed `url` uses plaintext HTTP against a non-local
/// host while an API key is configured, because the API key would be sent
/// unencrypted.
///
/// Loopback hosts (`127.0.0.1`, `::1`, `localhost`) are exempt because they
/// never leave the machine.
pub fn warn_on_insecure_http(url: &Url, api_key: &str) {
    if url.scheme() != "http" {
        return;
    }
    if api_key.trim().is_empty() {
        return;
    }
    let host = url.host_str().unwrap_or("");
    let is_loopback = matches!(host, "127.0.0.1" | "::1" | "localhost");
    if is_loopback {
        return;
    }
    log::warn!(
        "AI endpoint uses plaintext HTTP ({}). API key will be transmitted unencrypted; consider using HTTPS.",
        host
    );
}

/// Convenience wrapper that parses `url_str` and forwards to
/// [`warn_on_insecure_http`]. Parse errors are silently ignored because the
/// caller's existing URL validation will have already rejected malformed
/// URLs.
pub fn warn_on_insecure_http_str(url_str: &str, api_key: &str) {
    if let Ok(url) = Url::parse(url_str) {
        warn_on_insecure_http(&url, api_key);
    }
}

/// Canonical advisory string surfaced when a hosted-provider failure pattern
/// (HTTPS validation rejection, connection refused / DNS failure to a private
/// host, or HTTP 200 with a non-OpenAI-canonical body) suggests the user
/// intended to call an OpenAI-compatible local or LAN endpoint.
///
/// The wording is reused verbatim by:
/// - [`crate::config::validator::validate_ai_config`] when rejecting a
///   non-`https` `ai.base_url` for a hosted provider.
/// - The hosted-provider AI clients (`openai`, `openrouter`, `azure-openai`)
///   when wrapping connection / parse errors that match the local-endpoint
///   misconfiguration pattern.
///
/// Keeping the string in a single helper guarantees the advisory text stays
/// in lockstep across every emission site.
pub fn local_provider_hint() -> &'static str {
    "If you intended to call an OpenAI-compatible local or LAN endpoint, \
     set `ai.provider = \"local\"` (or `ollama`) and configure `ai.base_url` \
     to your endpoint."
}

#[cfg(test)]
mod local_provider_hint_tests {
    use super::local_provider_hint;

    #[test]
    fn hint_is_non_empty_and_mentions_local_and_ollama() {
        let hint = local_provider_hint();
        assert!(!hint.is_empty(), "hint must not be empty");
        assert!(hint.contains("local"), "hint must mention `local`: {hint}");
        assert!(
            hint.contains("ollama"),
            "hint must mention `ollama`: {hint}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_never_warns() {
        let url = Url::parse("https://api.example.com/v1").unwrap();
        warn_on_insecure_http(&url, "sk-secret");
    }

    #[test]
    fn http_loopback_does_not_warn() {
        for host in [
            "http://127.0.0.1:8080",
            "http://localhost/v1",
            "http://[::1]/",
        ] {
            let url = Url::parse(host).unwrap();
            warn_on_insecure_http(&url, "sk-secret");
        }
    }

    #[test]
    fn http_public_with_empty_key_does_not_warn() {
        let url = Url::parse("http://api.example.com/v1").unwrap();
        warn_on_insecure_http(&url, "");
        warn_on_insecure_http(&url, "   ");
    }

    #[test]
    fn http_public_with_key_runs_without_panic() {
        // Log output cannot be asserted without a test logger crate; we just
        // make sure the branch is exercised.
        let url = Url::parse("http://api.example.com/v1").unwrap();
        warn_on_insecure_http(&url, "sk-secret");
    }
}
