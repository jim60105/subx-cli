## Why

A comprehensive security audit of the SubX-CLI codebase identified 20+ findings across six domains: secrets management, file operation safety, input validation, subtitle parser robustness, async runtime correctness, and supply chain hygiene. Several high-severity issues exist — API keys are printed in plaintext by `config set`/`config list`, config files are written world-readable, file write operations are vulnerable to symlink-based redirection and TOCTOU races, the ASS subtitle parser panics on malformed input, and full audio files are decoded into RAM without any size guard. While SubX is a local CLI (the user is the trust boundary for CLI arguments), the tool processes untrusted files from the filesystem and communicates with external AI services, making these issues actionable.

## What Changes

- Mask sensitive configuration values (`ai.api_key` and similar) in all CLI output — `config set`, `config list`, `config get`, and Debug impls on AI client structs.
- Set file permissions to `0600` on the config file and `0700` on the config directory after every write (Unix).
- Replace TOCTOU `exists()` + `rename()`/`create()` patterns with atomic file operations (`OpenOptions::create_new(true)`) in filename-conflict resolution and all file write paths.
- Add symlink detection to recursive directory scanning and file write targets to prevent symlink-following attacks.
- Add configurable file-size guards before reading subtitle files, audio files, and AI API responses into memory.
- Fix ASS subtitle parser: replace `.unwrap()` calls on Format field lookup with proper error returns, and use checked arithmetic for timestamp parsing to prevent integer overflow.
- Replace blocking `std::fs` calls inside `async fn` with `spawn_blocking` across all async functions (not just the parallel task executor).
- Fix scheduler `active_tasks` leak on task overflow/rejection, and guard against scheduler loop premature termination.
- Warn (but not reject) when a plaintext `http://` AI endpoint URL is used with an API key configured, since users may run self-hosted API proxies without SSL.
- Replace the unmaintained `md5` crate with `md-5` (or `blake3`), narrow `symphonia` and `tokio` feature flags, and verify the existing `cargo audit` CI gate is enforcing failures.

## Capabilities

### New Capabilities
- `secrets-protection`: Mask sensitive values in CLI output, redact API keys in Debug impls, enforce restrictive file permissions on config files, and warn on insecure transport.
- `file-operation-safety`: Atomic file creation to eliminate TOCTOU races, symlink detection on both scan inputs and write targets with full parent-chain validation.
- `input-size-guards`: Configurable maximum file-size checks before reading subtitle files, audio files, and AI API response bodies into memory.
- `subtitle-parser-hardening`: Eliminate panics on malformed ASS/SRT/VTT/SUB input, use checked arithmetic for timestamp parsing, and improve error recovery in all subtitle format parsers.
- `async-runtime-safety`: Replace blocking filesystem calls in all async functions with non-blocking alternatives, fix scheduler active-task accounting on overflow, and prevent premature scheduler loop termination.
- `supply-chain-hardening`: Replace unmaintained crates, narrow feature flags to reduce attack surface, and verify the existing `cargo audit` CI gate enforces failures.

### Modified Capabilities
- `configuration-management`: Config file writes now enforce `0600` permissions; `config set`/`config list`/`config get` now mask sensitive values.
- `file-organization`: File rename/copy/move operations use atomic creation instead of check-then-act; symlinks are detected and handled.
- `parallel-processing`: Task executor uses non-blocking I/O; all async functions with blocking fs ops are wrapped in spawn_blocking; scheduler correctly tracks active tasks across all code paths.
- `format-conversion`: Subtitle parsers return errors instead of panicking on malformed input; file-size checks applied before parsing.
- `error-handling`: Error messages no longer echo raw upstream API response bodies; sensitive data redacted from error chains.

## Impact

- **Source files affected:** ~20 files across `src/config/`, `src/commands/`, `src/services/ai/`, `src/core/formats/`, `src/core/parallel/`, `src/core/matcher/`, `src/core/fs_util.rs`, `src/core/file_manager.rs`
- **New dependencies:** Potentially `secrecy` crate for `SecretString` wrapper; `md-5` or `blake3` replacing `md5`
- **Removed dependencies:** `md5` (unmaintained)
- **Config schema:** New optional keys `general.max_subtitle_bytes` and `general.max_audio_bytes` for file-size guards
- **CI/CD:** Verify existing `cargo audit` workflow enforces failure on advisories
- **Behavioral changes:** `config list`/`config set` output changes (masked values); files that previously triggered panics in ASS parser now produce error messages; `http://` AI endpoints produce warnings
- **No breaking CLI changes** — all fixes are internal hardening; existing command syntax and flags are unchanged
