## Context

SubX-CLI is a Rust CLI tool that processes untrusted subtitle and media files from the local filesystem, calls external AI APIs with user-provided API keys, and writes output files. A comprehensive security audit identified 20+ findings across six domains. The codebase has zero `unsafe` blocks and no command-injection surface, but has meaningful gaps in secrets handling, file operation atomicity, input size validation, parser robustness, and async correctness.

The trust model: CLI arguments come from the invoking user (trusted), but the files being processed and directories being scanned are semi-trusted — an attacker who can plant files or symlinks in a scanned directory can influence behavior. The AI service endpoint is a remote trust boundary. Error messages and CLI output may be captured in logs or CI pipelines.

Current state of each affected area:

- **Secrets:** `config set ai.api_key` echoes the key to stdout. `config list` serializes the full key. Config files are written with default `0644` permissions. `#[derive(Debug)]` on AI client structs includes `api_key`. Plaintext `http://` endpoints are accepted.
- **File operations:** Filename-conflict resolution uses `exists()` + `rename()`/`create()` (classic TOCTOU). `File::create` follows symlinks on destination. Recursive directory scans follow symlinks via `Path::is_file()`. The file-removal rollback is a stub.
- **Input sizes:** No file-size checks before reading subtitle files, audio files, or AI response bodies into memory. The parallel scheduler multiplies this risk.
- **Parsers:** The ASS parser panics on missing `Format:` fields via `.unwrap()`. ASS timestamp parsing overflows `u64` on adversarial input. SRT parser aborts on first bad block.
- **Async runtime:** The parallel task executor calls blocking `std::fs` inside `async fn`, starving the tokio runtime. The scheduler leaks `active_tasks` entries on overflow paths.
- **Supply chain:** The `md5` crate is unmaintained. `symphonia` and `tokio` use `features=["all"]`/`features=["full"]`. No `cargo audit` in CI.

## Goals / Non-Goals

**Goals:**

- Eliminate all high-severity findings: API key exposure in output, TOCTOU file races, ASS parser panics, and unbounded audio memory allocation.
- Reduce medium-severity findings: Debug-impl key leakage, HTTP endpoint validation, input-size guards, scheduler correctness.
- Introduce defense-in-depth patterns that prevent regressions: `SecretString` wrapper, atomic file operations, size-checked reads.
- Add CI supply-chain gates (`cargo audit`).
- Keep all changes backward-compatible — no CLI syntax changes, no breaking config changes.

**Non-Goals:**

- OS keyring integration for API key storage (too platform-specific for this change; can be a follow-up).
- Full sandboxing / `--root` jail mode for input paths (the user is the trust boundary for CLI args).
- `zeroize`-on-drop for all in-memory secrets (low impact for a short-lived CLI; `secrecy::SecretString` provides partial mitigation).
- Rewriting the parallel scheduler from scratch (fix the specific bugs, not the architecture).
- Streaming audio processing to avoid full-decode (would require major VAD pipeline changes; a max-size guard is sufficient).

## Decisions

### D1: Use `secrecy::SecretString` for API keys

Wrap `api_key` fields in `secrecy::SecretString` instead of plain `String`. This type implements `Debug` as `"***"`, implements `Zeroize` on drop, and makes accidental exposure a compile-time friction point (you must call `.expose_secret()` explicitly).

**Alternative considered:** Manual `Debug` impl without `secrecy` crate. Rejected because it only fixes Debug; it doesn't prevent accidental `format!("{}", key)` or logging, and doesn't zeroize.

**Alternative considered:** A custom `MaskedString` newtype. Rejected because `secrecy` is a well-maintained, widely-used crate that solves this exact problem with zero runtime cost.

### D2: Mask sensitive values in config output by key-name pattern

In `config_command.rs`, detect keys matching `api_key`, `token`, `secret` (case-insensitive substring) and replace the displayed value with `****<last4>`. Apply this to `config set` confirmation, `config list` serialization, and `config get` output.

**Alternative considered:** Never store `api_key` in config files at all (env-var only). Rejected because the current `config set ai.api_key` workflow is documented and used; removing it is a breaking change.

### D3: Atomic file operations split by operation type

The TOCTOU fix must be split by operation type because copy/create and rename/move have different semantics:

**For copy and backup operations:** Open the destination file handle with `OpenOptions::new().write(true).create_new(true).open()`. Write content through the **same opened handle** — never close and reopen. On `AlreadyExists`, retry with an incremented numeric suffix. This eliminates the TOCTOU window entirely because the file handle that reserved the path is the same one used for writing.

**For rename/move operations:** Use a copy-to-new-file-then-delete-source pattern: (1) open destination atomically with `create_new(true)`, (2) copy source content through that handle, (3) `fsync` the handle, (4) close, (5) delete source. This changes semantics from a single `rename()` call to a copy+delete, but is necessary because `rename()` on Unix silently overwrites the destination and there is no portable `RENAME_NOREPLACE`.

**Alternative considered:** `renameat2` with `RENAME_NOREPLACE` (Linux-specific). Rejected because it's not portable and requires `libc` FFI. The copy+delete approach works on all platforms.

### D4: Symlink detection via `symlink_metadata` / `DirEntry::file_type` with parent-chain validation

In recursive directory scanning (`input_handler.rs`, `discovery.rs`), use `entry.file_type()` (which does not follow symlinks) instead of `Path::is_file()` / `Path::is_dir()` (which do follow symlinks). Skip symlinked entries with a debug log.

For write targets, validate the **entire parent directory chain** — not just the leaf path. Canonicalize the parent directory of the target path and verify it is under the expected output directory. This prevents an attacker from placing a symlinked parent directory that redirects writes. As defense-in-depth on Unix, also apply `O_NOFOLLOW` via `OpenOptionsExt` where available.

**Alternative considered:** `O_NOFOLLOW` flag alone. Rejected as the sole approach because it's Unix-only and doesn't protect against symlinked parent directories in the path — the parent-chain validation is the primary defense, `O_NOFOLLOW` is belt-and-suspenders.

### D5: File-size guards as configurable limits

Add `general.max_subtitle_bytes` (default: 50 MiB) and `general.max_audio_bytes` (default: 2 GiB) config keys. Check `fs::metadata(path)?.len()` before any `read`/`read_to_string`/Symphonia probe. For AI response bodies, check `Response::content_length()` and cap at 10 MiB before calling `.text()`.

**Alternative considered:** Streaming reads with bounded buffers. Rejected for subtitle files (they need full content for parsing) and AI responses (small payloads); appropriate for audio but deferred as non-goal (too invasive for the VAD pipeline).

### D6: Replace `.unwrap()` with error propagation in ASS parser

Replace all three `.unwrap()` calls in ASS `Format:` field lookup with `.ok_or_else(|| SubXError::subtitle_format("ASS", "missing required field"))`. Use `checked_mul`/`checked_add` in `parse_ass_time` and return `SubXError` on overflow.

### D7: `tokio::task::spawn_blocking` for all blocking filesystem operations in async functions

Wrap blocking `std::fs` calls in `tokio::task::spawn_blocking()` in **all** async functions that perform file I/O — not just the parallel task executor in `task.rs`. This includes:
- `task.rs`: `execute_copy_operation`, `execute_move_operation`, `execute_create_backup_operation`, `execute_rename_file_operation`
- `engine.rs`: `read_to_string` in match processing, cache load/save, `create_dir_all`/`write` in result output
- Any other `async fn` calling `std::fs::*` directly

**Alternative considered:** `tokio::fs::*` async equivalents. Rejected because `tokio::fs` is itself just `spawn_blocking` under the hood; using `spawn_blocking` explicitly is clearer about the blocking nature and avoids changing every call site's signature.

### D8: RAII guard for scheduler `active_tasks`

Introduce a guard struct that holds the `task_id` and a reference to `active_tasks`, removing the entry on `Drop`. This ensures cleanup happens on all code paths (normal return, early error return, overflow reject/drop).

### D9: Replace `md5` with `md-5`, narrow feature flags

Replace `md5 = "0.7"` with `md-5 = "0.7"` (RustCrypto maintained version, API-compatible). Narrow `tokio` features from `"full"` to the specific features used. Narrow `symphonia` features from `"all"` to the codecs actually used (mp4, mkv, wav, ogg, aac, mp3).

### D10: Warn on plaintext HTTP endpoint

When constructing AI clients, if the base URL scheme is `http://` and an API key is configured, emit a warning via `log::warn!`. Do not reject outright (users may have local proxies or test servers on localhost).

## Risks / Trade-offs

- **[`secrecy` dependency]** → Adds one new crate. It's widely used (100M+ downloads), pure Rust, no transitive deps. Minimal supply-chain risk.
- **[Atomic file creation changes behavior on conflict]** → Previously, parallel workers could silently overwrite each other's output. Now they'll each get unique suffixed names. This is more correct but changes output filenames in the conflict case. → Mitigated by the existing conflict-resolution logic already producing suffixed names; the atomic approach just makes it race-free.
- **[File-size guards reject large files]** → A user with a legitimately large subtitle file (>50 MiB) would get an error. → Mitigated by making the limit configurable and setting a generous default.
- **[`spawn_blocking` pool exhaustion]** → Heavy batch operations could exhaust the tokio blocking thread pool. → Mitigated by tokio's default 512-thread blocking pool, which is far more than the typical `max_workers` (default 4-8).
- **[Narrowing symphonia features]** → May break processing of less common audio formats. → Mitigated by testing with all formats currently mentioned in docs/tests and keeping the common ones.
- **[ASS parser now returns errors instead of panicking]** → Callers that relied on panics being caught (unlikely in this codebase) would see different behavior. → This is strictly an improvement; panics in release mode cause `abort`.
