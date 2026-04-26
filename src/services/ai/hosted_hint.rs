//! Hint-emission helpers shared by the hosted AI provider clients.
//!
//! When a hosted provider (`openai`, `openrouter`, `azure-openai`) fails in a
//! pattern that strongly suggests the user actually meant to call an
//! OpenAI-compatible local / LAN endpoint, the hosted client appends the
//! canonical advisory string from
//! [`crate::services::ai::security::local_provider_hint`] to the resulting
//! [`crate::error::SubXError::AiService`] message.
//!
//! This module centralises the predicates and the appender so every hosted
//! client emits the hint identically.

use url::Url;

use crate::services::ai::error_sanitizer::sanitize_url_in_error;
use crate::services::ai::security::local_provider_hint;

/// Return `true` when `url` points at a host that is unambiguously private
/// (loopback, RFC1918, RFC4193, link-local) or a hostname commonly used to
/// refer to a local / LAN endpoint (`localhost`, `*.local`, `*.lan`,
/// `*.internal`, `*.localdomain`).
///
/// When the URL has no host (e.g. `data:` or `file:` schemes) the function
/// returns `false`. Hostnames that resolve dynamically are NOT looked up;
/// the predicate inspects only the syntactic form of the URL.
pub(crate) fn is_private_host(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    is_private_host_str(host)
}

/// Same predicate as [`is_private_host`] but operating on a raw host string.
/// Exposed so connection-error paths that have only the configured URL (and
/// not a parsed `Url`) can still classify the host.
pub(crate) fn is_private_host_str(host: &str) -> bool {
    // IPv6 literals in URLs are wrapped in brackets; `host_str()` already
    // strips them, but we guard against callers passing the bracketed form.
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');

    // Try IPv4 first.
    if let Ok(v4) = host.parse::<std::net::Ipv4Addr>() {
        return is_private_ipv4(v4);
    }
    // Then IPv6.
    if let Ok(v6) = host.parse::<std::net::Ipv6Addr>() {
        return is_private_ipv6(v6);
    }

    // Hostname syntactic checks.
    let lower = host.to_ascii_lowercase();
    if lower == "localhost" {
        return true;
    }
    // Common conventions for non-public TLDs / suffixes used on LANs.
    for suffix in [".local", ".lan", ".internal", ".localdomain", ".home.arpa"] {
        if lower.ends_with(suffix) {
            return true;
        }
    }
    false
}

fn is_private_ipv4(addr: std::net::Ipv4Addr) -> bool {
    // 127.0.0.0/8 — loopback
    if addr.is_loopback() {
        return true;
    }
    // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16 — RFC1918
    if addr.is_private() {
        return true;
    }
    // 169.254.0.0/16 — link-local
    if addr.is_link_local() {
        return true;
    }
    false
}

fn is_private_ipv6(addr: std::net::Ipv6Addr) -> bool {
    // ::1 — loopback
    if addr.is_loopback() {
        return true;
    }
    let segments = addr.segments();
    // fc00::/7 — unique local addresses (RFC4193). The high 7 bits of the
    // first segment must be 1111110, i.e. the segment lies in [0xfc00, 0xfdff].
    if (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    // fe80::/10 — link-local. High 10 bits 1111111010, segment in
    // [0xfe80, 0xfebf].
    if (segments[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    false
}

/// Predicate for the *connection refused / DNS failure to a private host*
/// branch of the *Hosted Provider Errors Hint Toward Local Provider*
/// requirement.
///
/// Returns `true` when:
///
/// 1. `err` is a connect-time failure (`reqwest::Error::is_connect()`) **or**
///    a request error against an unresolved hostname, and
/// 2. the configured base URL parses and points at a private host
///    (per [`is_private_host`]).
///
/// The function deliberately favours false-negatives over false-positives:
/// when in doubt (unparseable URL, public host, or non-transport error) it
/// returns `false` so the hint is not emitted spuriously for genuine
/// upstream failures.
pub(crate) fn should_hint_for_transport(err: &reqwest::Error, configured_url: &str) -> bool {
    // Only transport-layer failures qualify. Status-coded responses (4xx /
    // 5xx) come back as `Ok(Response)` — they never reach this predicate.
    if !(err.is_connect() || err.is_request() || err.is_timeout()) {
        return false;
    }
    let Ok(url) = Url::parse(configured_url) else {
        return false;
    };
    is_private_host(&url)
}

/// Predicate for the *HTTP 200 with non-OpenAI-canonical body* branch.
///
/// Returns `true` when the response body parsed as JSON (`body_was_json`)
/// but the canonical `choices[0].message.content` field is missing — i.e.
/// the upstream is reachable and speaking JSON but does not implement the
/// OpenAI chat-completions schema. In that situation the user almost
/// certainly pointed a hosted provider at a non-OpenAI endpoint.
///
/// `parse_error_msg` is accepted for symmetry with future predicates (e.g.
/// distinguishing different `serde_json` error kinds) but is not currently
/// inspected.
pub(crate) fn should_hint_for_parse(body_was_json: bool, _parse_error_msg: &str) -> bool {
    body_was_json
}

/// Append the canonical local-provider hint to `message`, first running the
/// message through [`sanitize_url_in_error`] so any URLs embedded by
/// `reqwest` or upstream bodies have their query strings stripped before
/// the hint is concatenated.
pub(crate) fn append_local_hint(message: &str) -> String {
    let sanitized = sanitize_url_in_error(message);
    format!("{}\n{}", sanitized, local_provider_hint())
}

/// Wrap an existing [`crate::error::SubXError`] with the local-provider hint
/// **iff** the configured base URL points at a private host (per
/// [`is_private_host_str`]) and the error is an [`crate::error::SubXError::AiService`]
/// variant.
///
/// This is the single decision point used by hosted-provider clients
/// (`OpenAIClient`, `OpenRouterClient`, `AzureOpenAIClient`) when wrapping a
/// failure surfaced by the shared retry machinery (which converts
/// `reqwest::Error` into [`crate::error::SubXError`] before the caller can
/// inspect the original transport-layer kind).
///
/// The predicate intentionally classifies on the **configured URL** rather
/// than the post-conversion error string: when a hosted provider is pointed
/// at a private host, *every* failure mode (connect refused, DNS failure,
/// timeout, even an unexpected HTTP status from a non-OpenAI server
/// listening on that port) implies the same misconfiguration. Conversely
/// when the configured host is public, no transport failure should be
/// attributed to a "did you mean local?" mistake — preserving the negative
/// scenario in the *Hosted Provider Errors Hint Toward Local Provider*
/// requirement (e.g. HTTP 401 from `https://api.openai.com/v1`).
pub(crate) fn maybe_attach_local_hint(
    err: crate::error::SubXError,
    configured_url: &str,
) -> crate::error::SubXError {
    use crate::error::SubXError;
    let Ok(url) = Url::parse(configured_url) else {
        return err;
    };
    if !is_private_host(&url) {
        return err;
    }
    match err {
        SubXError::AiService(msg) => SubXError::AiService(append_local_hint(&msg)),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn is_private_host_loopback_v4() {
        assert!(is_private_host(&url("http://127.0.0.1:8080/v1")));
        assert!(is_private_host(&url("http://127.255.255.254/v1")));
    }

    #[test]
    fn is_private_host_loopback_v6() {
        assert!(is_private_host(&url("http://[::1]:8080/v1")));
    }

    #[test]
    fn is_private_host_rfc1918() {
        assert!(is_private_host(&url("http://10.0.0.5:11434/v1")));
        assert!(is_private_host(&url("http://172.16.0.1/v1")));
        assert!(is_private_host(&url("http://172.31.255.255/v1")));
        assert!(is_private_host(&url("http://192.168.0.1/v1")));
        assert!(is_private_host(&url("http://192.168.255.255/v1")));
    }

    #[test]
    fn is_private_host_link_local() {
        assert!(is_private_host(&url("http://169.254.1.1/v1")));
        assert!(is_private_host(&url("http://[fe80::1]/v1")));
        assert!(is_private_host(&url("http://[febf::1]/v1")));
    }

    #[test]
    fn is_private_host_rfc4193() {
        assert!(is_private_host(&url("http://[fc00::1]/v1")));
        assert!(is_private_host(&url("http://[fdff::1]/v1")));
    }

    #[test]
    fn is_private_host_hostname_aliases() {
        assert!(is_private_host(&url("http://localhost:11434/v1")));
        assert!(is_private_host(&url("http://my-box.local/v1")));
        assert!(is_private_host(&url("http://server.lan/v1")));
        assert!(is_private_host(&url("http://gpu.internal/v1")));
        assert!(is_private_host(&url("http://x.localdomain/v1")));
    }

    #[test]
    fn is_private_host_public_addresses_negative() {
        assert!(!is_private_host(&url("https://api.openai.com/v1")));
        assert!(!is_private_host(&url("https://1.1.1.1/v1")));
        assert!(!is_private_host(&url("https://8.8.8.8/v1")));
        assert!(!is_private_host(&url("https://172.32.0.1/v1"))); // outside /12
        assert!(!is_private_host(&url("https://192.169.0.1/v1"))); // outside /16
        assert!(!is_private_host(&url("https://[2001:4860:4860::8888]/v1")));
    }

    #[test]
    fn is_private_host_str_handles_bracketed_v6() {
        assert!(is_private_host_str("[::1]"));
        assert!(is_private_host_str("::1"));
    }

    #[test]
    fn should_hint_for_parse_only_when_body_was_json() {
        assert!(should_hint_for_parse(true, "missing field"));
        assert!(!should_hint_for_parse(false, "expected value"));
    }

    #[test]
    fn append_local_hint_appends_full_advisory_and_strips_query() {
        let appended = append_local_hint("oops at https://x.test/a?token=secret");
        // The sanitizer drops the query string.
        assert!(!appended.contains("token=secret"));
        // Original message preserved up to the URL boundary.
        assert!(appended.contains("oops at https://x.test/a"));
        // Canonical hint appears after a newline.
        assert!(
            appended.contains("ai.provider"),
            "missing canonical hint: {appended}"
        );
        assert!(appended.contains("ollama"));
        assert!(appended.contains('\n'));
    }

    #[test]
    fn append_local_hint_uses_canonical_helper() {
        // Sanity: the appended tail must equal the canonical helper output.
        let appended = append_local_hint("x");
        assert!(appended.ends_with(local_provider_hint()));
    }

    // Note: `should_hint_for_transport` is exercised indirectly through the
    // hosted-client integration tests in `openai.rs`, `openrouter.rs`, and
    // `azure_openai.rs`, where genuine `reqwest::Error` values are produced
    // by attempting connections to ports without a listener. Constructing a
    // `reqwest::Error` directly requires private constructors, so the
    // predicate is best validated end-to-end.

    #[test]
    fn maybe_attach_local_hint_appends_when_url_private() {
        use crate::error::SubXError;
        let err = SubXError::AiService("connection refused".to_string());
        let wrapped = maybe_attach_local_hint(err, "http://127.0.0.1:11434/v1");
        let msg = wrapped.to_string();
        assert!(msg.contains("connection refused"));
        assert!(msg.contains("ollama"), "missing canonical hint: {msg}");
        assert!(msg.contains("ai.provider"));
    }

    #[test]
    fn maybe_attach_local_hint_skips_when_url_public() {
        use crate::error::SubXError;
        let err = SubXError::AiService("HTTP 401".to_string());
        let wrapped = maybe_attach_local_hint(err, "https://api.openai.com/v1");
        let msg = wrapped.to_string();
        assert!(msg.contains("HTTP 401"));
        assert!(
            !msg.contains("ollama"),
            "hint must not be emitted for public hosts: {msg}"
        );
    }

    #[test]
    fn maybe_attach_local_hint_skips_when_url_unparseable() {
        use crate::error::SubXError;
        let err = SubXError::AiService("boom".to_string());
        let wrapped = maybe_attach_local_hint(err, "not a url");
        assert!(!wrapped.to_string().contains("ollama"));
    }

    #[test]
    fn maybe_attach_local_hint_passthrough_for_non_ai_service_errors() {
        use crate::error::SubXError;
        let err = SubXError::config("bad");
        let wrapped = maybe_attach_local_hint(err, "http://127.0.0.1/v1");
        // Non-AiService variants are returned untouched.
        assert!(matches!(wrapped, SubXError::Config { .. }));
    }
}
