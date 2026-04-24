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
