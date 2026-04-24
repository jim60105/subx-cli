## 1. Secrets Protection

- [x] 1.1 Skipped `secrecy` crate — used simpler Debug + masking approach per design decision D1
- [x] 1.2 Removed `Debug` derive from `AIConfig`; added manual `Debug` impl that redacts `api_key` as `Some("[REDACTED]")` or `None`
- [x] 1.3 Implement manual `Debug` on `OpenAIClient`, `OpenRouterClient`, `AzureOpenAIClient` to redact `api_key` as `[REDACTED]`
- [x] 1.4 Create a `mask_sensitive_value(key: &str, value: &str) -> String` utility that returns `****<last4>` for keys matching `api_key`/`token`/`secret` (case-insensitive)
- [x] 1.5 Apply `mask_sensitive_value` in `config_command.rs`: `config set` confirmation, `config list` output, and `config get` output
- [x] 1.6 Create secure config file write helper using `OpenOptionsExt::mode(0o600)` on Unix to create files with restricted permissions from the start (no race window); create config directory with `0o700` before writing; no-op on non-Unix platforms
- [x] 1.7 Replace all `std::fs::write` calls for the config file in `service.rs` (`save_config`, `save_config_to_file`, `set_config_value`) with the secure write helper
- [x] 1.8 Add HTTP scheme check in AI client constructors (`openai.rs`, `openrouter.rs`, `azure_openai.rs`): `log::warn!` when `http://` + non-loopback + non-empty API key
- [x] 1.9 Write tests for masking utility, permissions helper, and HTTP warning logic

## 2. File Operation Safety

- [x] 2.1 Create `atomic_create_file(path: &Path) -> io::Result<File>` in `src/core/fs_util.rs` using `OpenOptions::create_new(true)`; on Unix also apply `mode(0o644)`
- [x] 2.2 Rewrite `resolve_filename_conflict` in `src/core/parallel/task.rs` to use `atomic_create_file` with retry-on-`AlreadyExists` loop; returns `(PathBuf, File)` tuple
- [x] 2.3 Rewrite `resolve_filename_conflict` in `src/core/matcher/engine.rs` AutoRename branch to use `atomic_create_file` retry loop
- [x] 2.4 Update `execute_copy_operation` and `execute_create_backup_operation` to write through the file handle from atomic conflict resolution
- [x] 2.5 Update `execute_move_operation` and `execute_rename_file_operation` to use copy+fsync+delete with rename fast-path
- [x] 2.6 Replace `Path::is_file()`/`Path::is_dir()` in `input_handler.rs` with `entry.file_type()` and skip symlinks with `debug!` log
- [x] 2.7 Replace `Path::is_file()` in `discovery.rs` `scan_directory` with `entry.file_type()` and skip symlinks
- [x] 2.8 Add `validate_write_target` parent-chain validation applied in copy/move/backup/rename operations
- [x] 2.9 Tests: atomic create, conflict resolution, symlink skipping, parent-chain validation, copy/move operations

## 3. Input Size Guards

- [x] 3.1 Add `max_subtitle_bytes` (default 50 MiB) and `max_audio_bytes` (default 2 GiB) fields to `GeneralConfig`
- [x] 3.2 Add config validation rules for the new size-limit keys in `field_validator.rs` (1 KiB–1 GiB / 1 KiB–10 GiB)
- [x] 3.3 Wire up `config set`/`config get` plumbing for the new keys in `service.rs` and `test_service.rs`
- [x] 3.4 Create `check_file_size(path, max_bytes, label)` utility in `src/core/fs_util.rs`
- [x] 3.5 Insert `check_file_size` in subtitle read paths: manager.rs, converter.rs, encoding/converter.rs, encoding/detector.rs, engine.rs
- [x] 3.6 Insert `check_file_size` before audio decode in `audio_loader.rs`
- [x] 3.7 Add AI response size guard: check `content_length()` before `.text()` in openai.rs, azure_openai.rs, openrouter.rs; cap at 10 MiB
- [x] 3.8 Tests: size-guard acceptance/rejection, config round-trip, subtitle manager oversize rejection

## 4. Subtitle Parser Hardening

- [x] 4.1 Replace `.unwrap()` calls in `ass.rs:111-124` (Format field lookup for Start, End, Text) with `.ok_or_else(|| SubXError::subtitle_format(...))`
- [x] 4.2 Rewrite `parse_ass_time` in `ass.rs:218-232` to use `checked_mul`/`checked_add` and return `SubXError` on overflow
- [x] 4.3 Change SRT parser in `srt.rs:35` to `continue` on bad block index parse instead of aborting the entire parse
- [x] 4.4 Add SUB parser duration validation: skip entries whose computed duration exceeds 24 hours with a `debug!` log (consistent with SRT skip-and-continue behavior)
- [x] 4.5 Write tests for each parser with malformed input: missing ASS fields, overflow ASS timestamps, bad SRT blocks, huge SUB frame numbers

## 5. Async Runtime Safety

- [x] 5.1 Wrap blocking `std::fs` calls in `task.rs` async functions with `tokio::task::spawn_blocking`; converted `resolve_filename_conflict` to sync free function
- [x] 5.2 Wrap blocking `std::fs` calls in `engine.rs` async functions (`extract_content_samples`, `save_file_list_cache`) with `spawn_blocking`
- [x] 5.3 Audit remaining `async fn`s: wrapped blocking calls in `audio_processor.rs` and `cache_command.rs`; others already sync
- [x] 5.4 Create RAII `ActiveTaskGuard` struct in `scheduler.rs` that removes task from `active_tasks` on drop
- [x] 5.5 Use `ActiveTaskGuard` in `submit_task_with_priority` for automatic cleanup on all exit paths
- [x] 5.6 Send `TaskResult::Failed("dropped due to queue overflow")` to evicted task's channel on `DropOldest`
- [x] 5.7 Add scheduler loop restart logic: check `is_finished()` on task submission and restart if needed
- [x] 5.8 Tests: ActiveTaskGuard cleanup, DropOldest sends Failed, scheduler restart after idle timeout

## 6. Error Handling Hardening

- [x] 6.1 Truncate AI error response bodies to 500 chars in `openai.rs:323-329`, `azure_openai.rs:189-195`, and `openrouter.rs` before embedding in `SubXError`
- [x] 6.2 Strip query parameters from URLs in error messages (use `url.set_query(None)` or manual stripping)
- [x] 6.3 Audit all `SubXError` variants for API key exposure and add a test that formats each variant and asserts no `sk-` prefix appears
- [x] 6.4 Replace `.unwrap()` in `retry.rs:88` (`try_clone`) with proper error conversion to `SubXError::AiService`
- [x] 6.5 Guard `retry.rs:59` (`last_error.unwrap()`) against `max_attempts = 0` with an explicit check

## 7. Supply Chain Hardening

- [x] 7.1 Replace `md5 = "0.7"` with `md-5 = "0.10"` in `Cargo.toml` (md5 crate was unused in codebase)
- [x] 7.2 Narrow `tokio` features from `"full"` to `["rt-multi-thread", "macros", "time", "sync", "fs"]`
- [x] 7.3 Narrow `symphonia` features from `"all"` to specific codecs used (`isomp4`, `mkv`, `ogg`, `wav`, `aac`, `flac`, `mp3`, `pcm`, `vorbis`)
- [x] 7.4 Verify existing `cargo audit` CI step in `.github/workflows/build-test-audit-coverage.yml` enforces build failure on advisories; tighten if needed
- [x] 7.5 Run `cargo audit` locally and address any findings from the dependency update

## 8. Documentation Updates

- [x] 8.1 Add "Security Considerations" section to `docs/configuration-guide.md`: API key storage, file permissions, shell history, size limits
- [x] 8.2 Update `docs/ai-provider-integration-guide.md` with HTTP vs HTTPS advisory note
- [x] 8.3 Replaced realistic-looking placeholder API keys with `<YOUR_API_KEY>` across docs and READMEs

## 9. Validation

- [x] 9.1 Run `cargo fmt` and `cargo clippy -- -D warnings` — fixed 13 clippy issues, zero warnings
- [x] 9.2 Run `timeout 240 scripts/quality_check.sh` — all checks pass except 2 pre-existing env-dependent integration test failures
- [x] 9.3 Run `cargo nextest run || true` — 1472/1474 passed, 2 pre-existing failures (config reads user's real config file)
- [x] 9.4 Run `timeout 240 scripts/check_coverage.sh -T` — new security modules at ~100% coverage, no regression
