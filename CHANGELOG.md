# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]

## [1.8.0] - 2026-07-24

### Changed
- AI-powered subtitle matching now produces better filenames out of the box. The model can now tag matches with their language (e.g. `movie.tc.srt` for Traditional Chinese, `movie.en.srt` for English), and SubX-CLI guarantees no two subtitles in the same batch get renamed to the same file — duplicates are automatically suffixed (`movie.srt`, `movie.2.srt`, `movie.3.srt`).

### Migration
- The match cache format has been updated. Your existing match cache will be rebuilt automatically on the next `subx-cli match` run; no action required.

## [1.7.4] - 2026-04-28

### Fixed
- Linux ARM64 (`subx-linux-aarch64`) release builds now succeed again. The previous toolchain was too old to link the ONNX Runtime libraries we ship; the release pipeline now uses an up-to-date toolchain so the aarch64 artifact is published reliably.

## [1.7.3] - 2026-04-28

### Fixed
- Linux ARM64 (`subx-linux-aarch64`) release artifact is now published reliably. The v1.7.2 release run failed to upload this asset because the cross-compilation container (Ubuntu 20.04) ships a curl version older than the one expected by our pre-build script. The release pipeline now uses curl flags supported by that container, so the aarch64 artifact is back in the release bundle.

## [1.7.2] - 2026-04-28

### Fixed
- Linux ARM64 (`subx-linux-aarch64`) release builds now succeed and the artifact is published reliably again. The cross-compilation toolchain now pre-stages the official ONNX Runtime archive, and the release smoke test runs the aarch64 binary under QEMU with a matching sysroot so the dynamic loader resolves correctly.

### Removed
- Linux musl release artifacts (`subx-linux-x86_64-musl`, `subx-linux-aarch64-musl`) are no longer published. Upstream ONNX Runtime ships no musl prebuilts, so SubX-CLI cannot produce a working musl binary in CI. Users on Alpine and other musl-based distros can still build SubX-CLI from source against a locally provided ONNX Runtime; the install script (`scripts/install.sh`) now rejects `--musl` and `SUBX_LIBC=musl` with exit code 2 instead of silently downloading a non-existent asset.

## [1.7.1] - 2026-04-28

### Added
- Windows CI coverage and quality checks: introduced `scripts/check_coverage.ps1` and `scripts/quality_check.ps1`, PowerShell 7 ports of the matching Bash scripts, and updated the GitHub Actions workflow so the `test` job's quality check runs the `.ps1` on Windows and the `coverage` job is matrixized over `[ubuntu-latest, windows-latest]`. The PowerShell coverage script uses `ConvertFrom-Json` and native PowerShell math instead of `jq`/`bc`. Both ports mirror their Bash counterparts' CLI surface, environment variables, and exit codes. Codecov uploads now carry per-OS flags (`coverage-${{ matrix.os }}`) so the two runs don't collide.

### Changed
- CI: migrated all Codecov test-results uploads from the deprecated `codecov/test-results-action@v1` to `codecov/codecov-action@v6` with `report_type: test_results`, following the upstream deprecation notice. Coverage uploads continue to use the same action with the default report type.

### Fixed
- Release pipeline: install OpenSSL development headers in the `cross` Docker images via a new `Cross.toml` so the three cross-compiled Linux artifacts (`subx-linux-aarch64`, `subx-linux-x86_64-musl`, `subx-linux-aarch64-musl`) build successfully. The `ort-sys` build dependency reaches `openssl-sys` through `ureq` → `native-tls`, and the default cross images don't ship `libssl-dev`/`pkg-config`, which broke the v1.7.0 release run.
- Documentation build: replaced four broken intra-doc links in `src/services/vad/resample.rs` (residue from the `rubato` 0.16 → 2.0 upgrade — `audioadapter`, `SequentialSlice`, `SequentialSliceOfVecs`, `Resampler::process_all_into_buffer`) with plain code spans so `cargo doc --all-features --no-deps --document-private-items` no longer fails under `-D rustdoc::broken-intra-doc-links`.
- Removed an unused `args` arrange block in `cli::sync_args::tests::test_validate_auto_vad_sensitivity_with_manual_method_err` that produced an `unused_variables` warning during unit-test compilation; the `args2` block carrying the actual assertion is unchanged.
- macOS test isolation: the `config_set_repair_invalid` integration tests now pin the user config file via `SUBX_CONFIG_PATH` instead of `XDG_CONFIG_HOME`. The `dirs::config_dir()` resolver ignores `XDG_CONFIG_HOME` on macOS and Windows, which caused all ten tests in that file to panic with `NotFound` on macOS-latest in CI.

## [1.7.0] - 2026-04-27

### Added
- `subx translate` command: AI-assisted subtitle translation that preserves cue timing, ordering, and format metadata while rewriting cue text into a target language. Works on single files, multiple files, directories (recursive with `--recursive`), and archive inputs.
  - Two-pass terminology consistency: a first pass builds a source-to-target term map for recurring proper nouns (people, places) so names stay consistent across batches. Prefers established conventional translations, then phonetic transliteration, then semantic translation when coining new terms. User-provided glossary entries always override AI-generated terminology.
  - Stable per-request cue IDs (UUIDv7, ≥1 ms spacing). Responses are validated against the requested IDs: duplicate or unknown IDs discard the batch (unknown IDs trigger one batch retry, then the file fails); missing IDs are retried once and then filled with empty strings if still absent.
  - Flags: `--target-language` (required), `--source-language`, `--glossary <PATH>` (UTF-8 file), `--context <TEXT>` (inline), `--output`, `--overwrite`, `--replace`, `--recursive`, `--no-extract`.
  - Safe defaults: writes `<stem>.<target-language>.<ext>` next to the source without modifying the original; existing translated outputs are not overwritten unless `--overwrite` is set; batch mode requires `--output` to be a directory; archive inputs default to writing under the archive's parent (not the temp extraction dir); `--replace` honors `general.backup_enabled`.
  - New `[translation]` configuration section with `batch_size` and `default_target_language`, plus `SUBX_TRANSLATION_*` environment overrides.
  - Per-file error isolation: one failing file does not block the rest of a batch.
- Local / OpenAI-compatible LLM provider (`ai.provider = "local"`, alias `ollama` normalized to `local` at config write time). Works with Ollama, LM Studio, llama.cpp `llama-server`, vLLM, and any OpenAI-compatible HTTP endpoint over loopback, LAN, VPN/tailnet, or the public internet. `ai.api_key` is optional, `ai.base_url` is required, both `http://` and `https://` are accepted. New `LOCAL_LLM_BASE_URL` and `LOCAL_LLM_API_KEY` env vars are honored only when the canonical provider is `local`. Privacy carve-out: when `local` is selected, hosted-provider env vars (`OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENROUTER_API_KEY`, all `AZURE_OPENAI_*`) are ignored so they cannot silently switch the active provider or inject credentials.
- Machine-readable JSON output mode for scripting and automation. New top-level `--output <text|json>` flag (must precede the subcommand) and `SUBX_OUTPUT` env var produce a stable, versioned envelope on stdout: `schema_version: "1.0"` with `command`, `status`, `data`, and `error`. Covered subcommands: `match`, `sync`, `convert`, `detect-encoding`, `cache` (`status`/`clear`/`rollback`/`apply`), `translate`, `config`. Per-file batch commands carry per-item `status`/`error` so partial failures keep the top-level envelope `ok`. `generate-completion` rejects JSON mode with `E_OUTPUT_MODE_UNSUPPORTED`. The legacy `cache status --json` flag is preserved as a byte-identical alias. See [`docs/machine-readable-output.md`](docs/machine-readable-output.md) for the full schema and recipes.
- Top-level `--quiet` flag (positioned before the subcommand) that suppresses status chatter in text mode and additionally silences free-form stderr diagnostics in JSON mode while leaving the stdout envelope intact. With `--quiet --output json`, `RUST_LOG=debug` no longer leaks structured logs to stderr.
- Stable error-envelope mapping with `category()` and `machine_code()` helpers on every `SubXError` variant. Clap argument-parsing failures now also emit a synthetic `E_ARGUMENT_PARSING` envelope so JSON-mode invocations always produce exactly one JSON document on stdout, even before any subcommand runs.
- Linux ARM64 release artifacts (`subx-linux-aarch64`) so `scripts/install.sh` works out-of-the-box on Raspberry Pi 4/5, AWS Graviton, Oracle Ampere, and ARM64 containers.
- Optional musl-static Linux artifacts (`subx-linux-x86_64-musl`, `subx-linux-aarch64-musl`) for Alpine and other glibc-incompatible distros. Opt in with `SUBX_LIBC=musl` or the `--musl` installer flag.
- Installer improvements: libc selection via `SUBX_LIBC`, `--musl` flag, and best-effort `ldd` auto-detection; exact-match URL resolution that no longer confuses `subx-linux-x86_64` with `subx-linux-x86_64-musl`; richer diagnostics that print the detected platform/arch, the searched asset name, the list of available assets, and a link to the releases page when the expected artifact is missing.
- Round-trip golden-file regression harness (`tests/format_roundtrip_tests.rs`) with byte-for-byte fixtures under `tests/fixtures/formats/{srt,ass,vtt,sub}/`, including CRLF and BOM variants. Fixture bytes are pinned via `.gitattributes`.
- Opt-in property-based parser fuzzing under `cargo test --features slow-tests`, using a hand-rolled xorshift64* PRNG and six mutation operators (`flip_byte`, `truncate`, `duplicate_random_line`, `inject_bom`, `oversize_cue`, `random_bytes`) in `src/core/formats/tests_support.rs`. No new runtime or dev dependencies were introduced for fuzzing.

### Changed
- Hosted AI providers (`openai`, `openrouter`, `azure-openai`) now require `https://` for any user-set `ai.base_url`. Misconfigured local endpoints under a hosted provider surface a one-line advisory pointing to `ai.provider = "local"` (or `ollama`). The `local` provider remains endpoint-agnostic.
- Unified all in-process identifiers on UUIDv7 via a shared `crate::core::uuidv7::Uuidv7Generator` that enforces strict ≥1 ms spacing between adjacent IDs. The matcher's `FileDiscovery` now emits `file_<uuidv7>` (41 chars total) instead of the previous deterministic `file_<16 hex chars>`, and the parallel scheduler / worker pool now mint UUIDv7 worker and task IDs (previously v4). The translation module's `CueIdGenerator` / `generate_cue_ids` are preserved as re-exports of the canonical `Uuidv7Generator` / `generate_ids` so the translation API stays source-compatible.
  - **Library API break:** `subx_cli::core::matcher::discovery::generate_file_id` now requires a `&mut Uuidv7Generator` parameter and returns `String` directly. Downstream library consumers must instantiate one generator per scan and thread it through. The CLI binary surface is unaffected.
- Tightened JSON-mode stderr discipline. Free-form `eprintln!` / `println!` chatter is now unconditionally suppressed under `--output json`, including the matcher's `🔍 AI Analysis Results:` block, `Total matches:` / `Preview:` summaries, per-candidate `- file_<id>` listings, the "Cannot find AI-suggested file pair" / "Skipping relocation" / "Conflict resolution prompt not implemented" warnings, the file-manager "Cannot restore removed file" warning, the translation engine's unknown-cue-ID warning and progress chatter, and the parallel worker shutdown notice. Sync command `[DEBUG]` chatter has migrated from `eprintln!` to `log::debug!` so it now respects user log filters in every mode. Structured `tracing` / `log` records (gated by `RUST_LOG`) are still emitted unless `--quiet` is also set.
- Refactored `src/core/formats/` from monolithic per-format files into directory modules (`srt/`, `ass/`, `vtt/`, `sub/`), each split into `mod.rs`, `parser.rs`, `serializer.rs`, `time.rs`, and `tests.rs`. Public API and module paths are preserved. Shared subtitle types and the `SubtitleFormat` trait moved into a private `formats/types.rs` re-exported from `formats::*`; the `formats/mod.rs` facade is now ~150 lines.
- Hardened all subtitle parsers with consistent defensive behavior: empty input is rejected with a typed error, BOMs are stripped at the parser layer, per-cue payloads are capped at 1 MiB on raw pre-normalization bytes, out-of-order cues are preserved (SRT/VTT), and negative timestamps are skipped without aborting the parse. VTT validates the `WEBVTT` header, ASS validates the `[Events]` section and rejects `Dialogue:` rows with fewer columns than the `Format:` line declares, and SUB rejects non-numeric frame ranges. A per-format hardening disposition matrix is documented in the rustdoc of each `SubtitleFormat` implementation.
- Upgraded direct dependencies: `rubato` 0.16 → 2.0 (new `audioadapter`-based API), `zip` 4 → 8, `dialoguer` 0.11 → 0.12. Added `audioadapter-buffers = "3"` as the buffer-adapter companion now required by `rubato` 2.0. Transitive bumps via `cargo update` include `reqwest` 0.13.2 → 0.13.3, `rustls-platform-verifier` 0.6 → 0.7, `pbkdf2` 0.12 → 0.13, and `sha1` 0.10 → 0.11.
- Rewrote `services::vad::resample` against the rubato 2.0 API: replaces the manual `FftFixedIn` chunk loop and the previous double-init resampler cache with a single `Fft::<f32>::new(..., FixedSync::Input)` call plus `Resampler::process_all_into_buffer`, which trims the resampler delay and processes the whole input in one shot. Added direct unit tests covering identity, empty-input, downsample (44.1 → 16 kHz), and upsample (16 → 48 kHz) paths.

### Fixed
- `subx-cli config set/get/list/reset` no longer abort when the persisted `config.toml` fails strict cross-section validation (e.g. `ai.provider = "openai"` paired with an `http://` `ai.base_url`). Configuration commands now load the file through a tolerant path that performs only TOML parsing and provider-name canonicalization, so users can repair an invalid configuration in place via a follow-up `config set`. Strict validation still runs after every mutation; writes that leave the file invalid are rejected and the original bytes are preserved. `config get` and `config list` surface the underlying validation error as an advisory (a `warning:` line on stderr in text mode; a populated top-level `warnings: ["..."]` field in the JSON envelope). Non-`config` subcommands continue to refuse to run against an invalid configuration. The repair path no longer reads environment-only secrets (e.g. `OPENAI_API_KEY`) into the file, so `config set` cannot accidentally bake an env-only credential into the persisted config.
- SRT and VTT parsers now correctly handle CRLF (`\r\n`), bare-CR (`\r`), and mixed line-ending inputs. Previously, the literal `"\n\n"` block splitter caused CRLF SRT files to collapse into a single cue (subsequent cues' text was absorbed into the first entry's payload) and CRLF VTT files to parse to zero cue entries because the cue-marker line's trailing `\r` defeated the timing regex. Both parsers now normalize line endings and split on any blank-line separator regardless of `\r\n`/`\n`/`\r` convention. ASS and SUB parsers were already line-based and remain unaffected.

### Removed
- Unused direct dependencies `md-5` and `dialoguer` were removed from `Cargo.toml` (they had no call sites anywhere in the workspace). The `uuid/v4` Cargo feature has also been removed; the crate enables only `uuid/v7` now that all in-process IDs are UUIDv7.

### Documentation
- Documented the `translate` command, terminology consistency behavior, output naming rules, and safety flags in `docs/command-reference.md`.
- Added the `[translation]` configuration section and `SUBX_TRANSLATION_*` environment overrides to `docs/configuration-guide.md`.
- Added `docs/machine-readable-output.md` covering the JSON envelope schema, error categories, per-command payload schemas, and scripting recipes.
- Updated `README.md` and `README.zh-TW.md` to include translation in the supported workflow.

## [1.6.0] - 2026-04-25

### Added
- Archive input support: transparent extraction of `.zip` and `.rar` archives supplied as direct `-i` inputs, with `--no-extract` opt-out flag across all commands.
- 7-Zip (`.7z`) and tar-gzip (`.tar.gz`, `.tgz`) archive extraction via pure-Rust `sevenz-rust`, `tar`, and `flate2` crates — always available, no feature flag required.
- Cache `apply`, `rollback`, and `clear` commands with operation journal, file locking, and snapshot validation.
- Comprehensive security hardening: secrets redaction in Debug output and CLI, secure config file permissions (0600/0700), atomic file creation, TOCTOU-safe conflict resolution, symlink skipping, parent-chain validation, and configurable input size guards (50 MiB subtitle / 2 GiB audio).
- Decompression bomb protection for all archive formats: 1 GiB total size limit, 10,000 entry limit, path-traversal prevention, and symlink/hardlink rejection.
- `archive-rar` feature flag for optional RAR support via native `libunrar`.
- OpenSpec specification framework for managing capability specs and changes.

### Changed
- Refactored `archive.rs` into SRP module directory (`src/core/archive/`) with per-format modules: `common.rs`, `zip.rs`, `rar.rs`, `sevenz.rs`, `targz.rs`, and `mod.rs`.
- Rewrote README.md and README.zh-TW.md with updated project overview and feature documentation.
- Extracted command reference into `docs/command-reference.md`.
- Consolidated agent instructions into AGENTS.md.
- Bumped all dependencies to latest versions, including `indicatif` 0.18 for cargo audit compliance.
- Bumped GitHub Actions to latest major versions; added `actions-rust-lang/audit` v1.2.7.

### Fixed
- Cross-platform file locking: replaced Unix-only `libc::flock` with `std::fs::File::try_lock`/`unlock` (Rust 1.86+) for Windows compatibility.
- `detect-encoding` command now handles nonexistent positional file paths gracefully instead of failing with a validation error.
- Coverage script no longer fails on test failures — `--full` flag respects explicit profile and diagnostic output is preserved.
- Archive extraction on macOS: use original destination path instead of canonicalized path, preventing `/var` → `/private/var` symlink mismatch in archive origin lookups.
- Removed rustdoc warnings for private archive sub-module links.
- Production config integration tests no longer assert hardcoded model defaults, fixing failures when user config differs from defaults.
- Moved feature-gated test imports behind `#[cfg(feature = "slow-tests")]` to eliminate unused-import warnings.

### Security
- HTTP scheme warning for non-loopback AI endpoints using plain HTTP.
- Copy + fsync + delete pattern for cross-device file moves to prevent data loss.

## [1.5.2] - 2025-08-23
### Changed
- Updated voice_activity_detector dependency to official crate v0.2.1, replacing the silero-specific fork for improved maintainability and upstream support.
- Enhanced configuration guide documentation for accuracy and consistency, synchronizing with current config.toml and removing deprecated configuration items.

## [1.5.1] - 2025-07-08
### Changed
- Centralized AI prompt parsing and retry logic by introducing generic PromptBuilder and ResponseParser traits for all AI providers (AzureOpenAI, OpenAI, OpenRouter).
- Unified HTTP retry handling with new HttpRetryClient trait, replacing per-client custom retry implementations.
- Improved AI service architecture by removing duplicate prompt building and response parsing methods across all client modules.
- Enhanced test coverage for AI service traits with comprehensive edge case testing and retry scenario validation.

### Fixed
- Updated development container configuration to use correct repository name (jim60105/subx-cli).

## [1.5.0] - 2025-07-07
### Added
- Azure OpenAI and OpenRouter AI provider support, including CLI, config, test suite, and environment variable override (AZURE_OPENAI_DEPLOYMENT_ID).
- JUnit XML test results and Codecov integration in CI; support for #[cfg(feature = 'slow-tests')] to enable slow test selection.
- AI provider integration guide documentation.

### Changed
- Azure OpenAI provider now uses model name as deployment identifier (deployment_id removed).
- Unified coverage generation and checks in CI; streamlined workflow.
- Updated config usage analysis and README for accuracy; removed invalid options and fixed defaults.
- Refactored Azure OpenAI test structure for maintainability and coverage.
- Adjusted slow timeout and devcontainer settings for improved development and testing experience.
- Updated clippy lints configuration for better linting control.

### Fixed
- OpenRouterClient now retries on server error status codes.
- Fixed model name extraction logic in AI client and improved test readability.
- Fixed clippy lifetime and format-args warnings.

## [1.4.0] - 2025-07-06
### Added
- Added: Workspace override support via SUBX_WORKSPACE environment variable or config, enabling flexible project root selection.
- Added: Comprehensive integration tests for sync command, covering all documented parameter combinations and edge cases.

### Changed
- Changed: Migrated VAD backend from git-based dependency to official voice_activity_detector_silero_v5 crate for improved maintainability.
- Changed: Unified configuration flow and updated documentation to reflect new config loading and validation logic.
- Changed: Enhanced batch sync logic with smart video-subtitle pairing, improved skip messaging, and robust edge case handling.
- Changed: Refined sync argument validation and batch mode detection for more intuitive CLI usage.
- Changed: Updated VAD comment and dependency revision for clarity and accuracy.

### Removed
- Removed: Example audio processing code and obsolete tests for relative/absolute path handling.

### Fixed
- Fixed: Batch processing now correctly handles video-subtitle pairing and displays accurate skip messages for unmatched files.

### Documentation
- Documentation: Clarified sync integration test data policy to use only assets/ files and updated all related documentation.

## [1.3.0] - 2025-06-19
### Added
- Added: Positional argument support for sync command enabling direct usage like 'subx sync movie.mkv' and 'subx sync subtitle.srt'.
- Added: Enhanced auto-pairing logic with single file mode that automatically discovers matching video/subtitle pairs by filename stem.
- Added: Flexible batch mode support with syntax variations including '-b directory', '-b -i path', and mixed input combinations.
- Added: Manual mode flexibility where sync operations require only subtitle file and support positional arguments.

### Changed
- Changed: SyncArgs.batch field modified from bool to Option<Option<PathBuf>> for optional directory specification flexibility.
- Changed: Enhanced validate() method to support flexible parameter combinations and improve user experience.
- Changed: Improved get_sync_mode() with robust auto-pairing logic and enhanced batch processing capabilities.

### Fixed
- Fixed: Manual mode video file requirement issue - sync command no longer incorrectly requires both video and subtitle files in manual offset mode.
- Fixed: Positional argument handling and auto-pairing functionality in sync command for more intuitive usage patterns.

### Documentation
- Documentation: Added comprehensive implementation report documenting complete resolution of sync command argument flexibility issues.
- Documentation: Updated usage examples and documentation to reflect new argument flexibility capabilities.

## [1.2.0] - 2025-06-19
### Added
- Added: Unified audio processing pipeline with FFT-based Rubato resampler and upgraded Silero VAD to V5 for improved accuracy and performance.
- Added: Example for MP3 decoding and WAV resampling using Symphonia and Rubato.
- Added: Increased test timeouts in nextest configuration for more robust CI runs.

### Changed
- Changed: Audio loader now records last packet timestamp, computes duration from time_base, and infers channels if missing.
- Changed: VAD API refactored to accept preloaded and optionally resampled audio data, enforcing strict chunk sizes per Silero VAD V5 model.
- Changed: ProcessedAudioData is now cloneable and preserves original duration in VadAudioProcessor.
- Changed: Sync detector now loads and optionally crops audio before detection, using the new data-based VAD method.

### Fixed
- Fixed: Graceful handling of empty audio files—returns default ProcessedAudioData with empty samples and metadata instead of erroring.
- Fixed: Accurate sample count handling for stereo audio in tests.

### Documentation
- Documentation: Translated and clarified in-code comments to English for config sensitivity, language code mappings, error conversion, audio loading, resampling, and VAD processing logic.

### Test
- Test: Sensitivity tests now use Tokio multi-threaded runtime for improved concurrency and reliability.

## [1.1.0] - 2025-06-19
### Added
- Added: Enhanced logging for audio loading lifecycle, including debug, trace, and warning logs for file handling, probing, track selection, decoding, and error conditions.
- Added: Nextest profile support for coverage runs and CI, allowing selection of test profiles and improved reporting.
- Added: Optimized VAD (Voice Activity Detection) audio processing for improved performance and maintainability (Backlog #43).

### Changed
- Changed: Refactored VAD configuration by removing `speech_merge_gap_ms` and `probability` fields, simplifying segment logic and updating chunk size calculation for clarity and correctness.
- Changed: Updated all affected tests, documentation, and config handling to reflect VAD config changes.
- Changed: Enhanced sync engine with detailed debug logs, improved traceability, and robust error/warning reporting for VAD and offset detection.
- Changed: CI workflow and scripts to support configurable test profiles and improved output clarity.

### Removed
- Removed: Unused WAV loader and converter, including the entire `load_wav_file` method and related sample-format conversion logic.
- Removed: Deprecated `OldSyncConfig` type and its re-export in the sync module.
- Removed: Redundant `target_channels` and related design from VadAudioProcessor.

### Fixed
- Fixed: Negative offset application in sync engine and added tests for first sentence sync (Bug #23).
- Fixed: Test failures after VAD config restructure and improved test isolation.

### Documentation
- Documentation: Updated test instructions to capture nextest logs and clarified workaround for output limitations.
- Documentation: Improved bug plans for sync command argument flexibility and clarified asset usage for first sentence alignment tests.
- Documentation: Refined README.zh-TW to clarify direct audio decoding and VAD-based synchronization.
- Documentation: Old config files with removed fields are still accepted; extra fields are safely ignored for backward compatibility.

## [1.0.0] - 2025-06-18
### Added
- Added: Dedicated App struct as a programmatic API for library usage, enabling direct embedding and automation.
- Added: Centralized command dispatcher to eliminate duplication between CLI and library interfaces.
- Added: AI provider factory pattern (ComponentFactory) with OpenAI support and flexible configuration.
- Added: Direct audio loading for VAD synchronization, supporting multiple audio formats via Symphonia.
- Added: Comprehensive test coverage for dispatcher, App API, VAD, cache, CLI argument parsing, and shell completion.
- Added: Nextest-based test runner integration for faster, parallelized test execution.

### Changed
- Changed: Unified all service and factory logic under ComponentFactory, removing legacy ServiceContainer, AIClientFactory, and SyncServiceFactory.
- Changed: Refactored configuration validation into layered modules for maintainability and clarity.
- Changed: Improved error messages and validation feedback throughout the CLI and sync engine.
- Changed: Streamlined CI/CD scripts, increased test and coverage timeouts, and updated documentation to recommend nextest.
- Changed: All documentation comments and public APIs are now in English, following project guidelines.

### Removed
- Removed: AudioTranscoder module and all related transcoding logic (now handled directly by VAD audio loader).
- Removed: ServiceContainer, AIClientFactory, and SyncServiceFactory (replaced by unified ComponentFactory).
- Removed: Deprecated and unused code paths, including debug symlink creation in tests.

### Fixed
- Fixed: Cache target directory error in match command copy/move modes (Bug 21).
- Fixed: CIFS filesystem copy permission error by switching to safe file copy logic.
- Fixed: Broken intra-doc links in config validation modules.
- Fixed: Configuration validation errors and improved test isolation for all config-related tests.

### Documentation
- Documentation: Added and refined rustdoc for all public modules, including VAD and dispatcher.
- Documentation: Updated technical architecture and removed outdated ServiceContainer references.
- Documentation: All documentation comments are now in English; Chinese text retained only in test case strings.

## [0.10.0] - 2025-06-16
### Added
- Added: Configurable HTTP request timeout for AI services with range validation (10-600 seconds).
- Added: Comprehensive tests for duplicate rename conflict resolution scenarios.
- Added: Bug planning documentation for match command cache copy mode issues.

### Changed
- Changed: Simplified CLI help messages and removed excessive documentation for better readability.
- Changed: Streamlined README examples and clarified batch mode processing descriptions.
- Changed: Aligned Chinese README with English version for consistency.
- Changed: Migrated all Rust documentation comments in tests directory to English.

### Fixed
- Fixed: Critical Bug 21 - match cache copy mode incorrect target directory issue.
- Fixed: Missing request_timeout_seconds support in TestConfigService for consistent test behavior.

## [0.9.0] - 2025-06-16
### Added
- Added: Comprehensive configuration analysis and validation for max_offset_seconds usage throughout the codebase.
- Added: Full VAD (Voice Activity Detection) CLI support with get/set operations for all VAD-related configuration parameters.
- Added: AI sync and VAD settings exposure including max_sample_length, max_tokens, retry_attempts, and retry_delay_ms.
- Added: Cargo package size optimization by excluding unnecessary files (assets/, .github/, tests/) reducing package size by ~15MB.
- Added: Enhanced sync engine with max_offset_seconds enforcement for both manual and VAD-based operations.
- Added: Comprehensive integration tests for offset validation and VAD configuration consistency.
- Added: Support for manual-only sync mode (subtitle without video file).

### Changed
- Changed: Translated all Chinese documentation comments and error messages to English for consistency.
- Changed: Updated configuration documentation with accurate line numbers and call trees for all 30 configuration items.
- Changed: Improved error messages in sync command with detailed validation feedback.
- Changed: Enhanced config get/set command consistency supporting identical key sets.

### Fixed
- Fixed: Critical configuration inconsistency where 16 configuration items could be set but not retrieved via CLI.
- Fixed: VAD configurations completely missing from config get/set commands.
- Fixed: Config value consistency tests for proper float formatting.
- Fixed: Sync command validation to respect configured maximum offset limits.

## [0.8.0] - 2025-06-16
### Added
- Added: max_tokens configuration support for OpenAI responses with configurable range (1-100000).
- Added: Support for ai.max_tokens in config get/set operations with proper validation.

### Changed
- Changed: OpenAIClient now uses configurable max_tokens instead of hardcoded 1000 value.
- Changed: All Chinese comments translated to English in CLI modules for documentation consistency.

### Fixed
- Fixed: Unit test environment isolation issues ensuring all 244+ tests pass without external dependencies.
- Fixed: ProductionConfigService now properly respects injected SUBX_CONFIG_PATH environment variable.
- Fixed: Environment variable configuration tests no longer interfere with existing user config files.
- Fixed: Test macros now automatically set isolated config paths during testing.

## [0.7.0] - 2025-06-16
### Added
- Added: Unified input path parameter (-i/--input) support across all CLI commands (match, convert, sync, detect-encoding).
- Added: Recursive directory scanning functionality with --recursive flag for batch operations.
- Added: Batch mode support for sync and convert commands enabling efficient processing of multiple files.
- Added: File-centric match architecture for processing user-specified files directly instead of parent directories.
- Added: Enhanced audio transcoding support with Symphonia library for multiple audio formats (MP4, MKV/WebM, OGG).
- Added: Local Voice Activity Detection (VAD) for subtitle synchronization without external API dependencies.
- Added: Comprehensive test infrastructure improvements with over 300 new integration tests.
- Added: SubX theme audio/video assets for enhanced testing and demonstration.

### Changed
- Changed: Complete documentation migration from Chinese to English across all source code and comments.
- Changed: Unified file handling architecture across all commands for consistent behavior.
- Changed: Enhanced error handling in audio decoder with proper Symphonia API error classification.
- Changed: Improved CLI argument structure with better validation and user experience.
- Changed: Upgraded audio processing pipeline to support multiple container and codec formats.
- Changed: Enhanced match command to respect user-specified file inputs exactly as provided.

### Fixed
- Fixed: "No matching file pairs found" issue in match command when AI correctly identified matches.
- Fixed: Audio decoder error handling to properly manage recoverable vs fatal Symphonia decode errors.
- Fixed: Sync command video file requirement when using manual --offset parameter.
- Fixed: Audio processing panic when aus crate returns empty samples array.
- Fixed: File ID generation consistency using absolute paths for reliable caching.
- Fixed: CLI parameter conflicts in sync command by removing ambiguous short options.

### Removed
- Removed: aus audio processing library dependency in favor of Symphonia-based transcoding.
- Removed: Legacy audio analyzer and dialogue detector components.
- Removed: Deprecated sync configuration items and related error handling.

## [0.6.0] - 2025-06-13
### Added
- Added: Configuration set command with `config set` functionality and comprehensive validation system.
- Added: Default encoding configuration integration in EncodingDetector with configurable fallback behavior.
- Added: TestConfigService support for isolated testing environments.
- Added: Comprehensive unit and integration tests for configuration set command (Backlog #30).
- Added: Configuration value validation module with type checking and range validation.

### Changed
- Changed: Improved documentation readability for configuration guide with better structure and Traditional Chinese translation.
- Changed: Aligned configuration examples between English and Traditional Chinese README files.
- Changed: Renamed check_docs.sh to quality_check.sh to better reflect comprehensive quality assurance functionality.
- Changed: Enhanced CI workflow by consolidating test steps and accelerating coverage uploads.

### Fixed
- Fixed: AI model name now properly retrieved from configuration service instead of hardcoded values.
- Fixed: Removed unused imports and unnecessary mut variables to eliminate compiler warnings.
- Fixed: Missing rustdoc documentation and replaced Chinese comments with English equivalents.

### Removed
- Removed: Unused configuration items (temp_dir, log_level, cache_dir, chunk_size, enable_work_stealing) from configuration system.
- Removed: Placeholder performance tests that were not implemented.

## [0.5.0] - 2025-06-12
### Added
- Added: Wiremock integration testing framework for comprehensive mock testing without external API dependencies.
- Added: Success check mark (✓) display after successful file operations (rename, copy, move).
- Added: Cross mark (✗) display for failed file operations with meaningful error messages.
- Added: Copy/move-to-video-folder functionality for match command with --copy/-c and --move/-m parameters.
- Added: Comprehensive ConfigService integration tests for file I/O and configuration management.
- Added: Performance and stability testing with high load scenarios and memory stability validation.
- Added: MockOpenAITestHelper with delayed response support and error simulation capabilities.

### Changed
- Changed: Default AI model upgraded from "gpt-4o-mini" to "gpt-4.1-mini" across all configurations and examples.
- Changed: Replaced legacy config system with unified ConfigService architecture using dependency injection.
- Changed: Project description refined to "AI subtitle processing CLI tool, which automatically matches, renames, and converts subtitle files."
- Changed: All Chinese comments and documentation converted to English for international consistency.
- Changed: Configuration error hints updated to reference correct CLI command (`subx-cli config --help`).
- Changed: Increased documentation check timeout from 20 to 30 seconds to prevent CI failures.

### Fixed
- Fixed: Critical logic error in copy mode where original files were incorrectly renamed, now properly preserves source files.
- Fixed: Cache reuse mechanism in match command to support copy/move parameters correctly.
- Fixed: Clippy warnings for await_holding_lock by replacing std::sync::Mutex with tokio::sync::Mutex.
- Fixed: Wiremock integration test cache reuse issues with proper test isolation using static mutex.
- Fixed: File ID mismatch issues between mock expectations and actual IDs in cache validation.

### Removed
- Removed: "Early development" notice from README files as project has matured.
- Removed: Legacy config_legacy.rs module completely in favor of unified ConfigService.

## [0.4.1] - 2025-06-11
### Fixed
- Fixed: Documentation test examples to match current struct field names in cli::table module.
- Fixed: Compilation errors in doc tests by updating MatchDisplayRow examples from 'status'/'filename' to 'file_type'/'file_path'.

## [0.4.0] - 2025-06-11
### Added
- Added: Redesigned match table layout with status symbols and tree structure for improved visual organization.
- Added: Vertical multiline format display replacing the horizontal 4-column layout.
- Added: Status symbols (✓ for successful matches, 🔍 for dry-run preview) for better visual feedback.
- Added: Tree structure with ├ and └ symbols for visual grouping of related files.
- Added: Complete file path display without truncation for better readability.

### Changed
- Changed: Translated all Chinese documentation to English for consistency with project standards.
- Changed: Improved match table layout from 4-column horizontal to 2-column vertical design.
- Changed: Enhanced readability especially for long file paths through better visual hierarchy.

### Fixed
- Fixed: Subtitle filename issue where video file extensions were incorrectly retained in renamed files.
- Fixed: Added comprehensive unit tests for extension removal and edge cases.

## [0.3.0] - 2025-06-11
### Added
- Added: Comprehensive test infrastructure optimization with CLITestHelper, TestFileManager, OutputValidator, AudioMockGenerator, and SubtitleGenerator utilities.
- Added: Configuration service system with dependency injection pattern, ConfigService trait, and production/test implementations.
- Added: Complete test system refactoring enabling parallel test execution by removing all #[serial] annotations.
- Added: TestConfigBuilder API for fluent configuration creation and isolated test environments.
- Added: Coverage checking script with configurable thresholds and verbose reporting.
- Added: Comprehensive English documentation for all public APIs following rustdoc standards.
- Added: GPLv3 license with proper headers and copyright information.
- Added: Enhanced CI/CD pipeline with documentation quality checks and coverage enforcement.
- Added: Performance benchmarks for AI retry mechanisms and file ID generation.
- Added: Animated SVG logo with gradient effects and feature visualization.

### Changed
- Changed: Migrated all Chinese comments and documentation to English for international accessibility.
- Changed: Refactored configuration system to eliminate unsafe global state and enable dependency injection.
- Changed: Improved test architecture with isolated configurations for parallel safety.
- Changed: Enhanced documentation structure with comprehensive rustdoc guidelines and quality validation.
- Changed: Upgraded test execution to support true parallel processing without race conditions.
- Changed: Standardized all scripts with GPLv3 headers and improved error handling.
- Changed: Moved unit tests from tests/ directory to implementation files following Rust conventions.

### Fixed
- Fixed: Test race conditions in configuration integration tests with proper state isolation.
- Fixed: Documentation compilation issues and broken intra-doc links.
- Fixed: AI retry mechanism performance benchmarks with correct error handling and threading.
- Fixed: CLI help display tests to check for English strings instead of Chinese.
- Fixed: All clippy warnings and rustdoc validation errors.

### Security
- Security: Eliminated unsafe code in configuration management system.
- Security: Improved dependency management with updated security guidelines.

## [0.2.0] - 2025-06-09
### Added
- Added: Parallel processing system for batch operations, including TaskScheduler, WorkerPool, and priority/FIFO queue support.
- Added: AI configuration integration, OpenAI support, and related tests.
- Added: Automatic file encoding detection and CLI command.
- Added: Dialogue detection module and configuration integration.
- Added: Dynamic configuration support with file watching (hot-reload).
- Added: Validators and cache for unified config management.
- Added: Unit and integration tests for config, audio, and parallel modules.
- Added: Comprehensive stress testing for parallel processing.
- Added: Extensive debug logging for configuration management and tests.
- Added: Support for custom OpenAI base_url configuration.
- Added: Table-based display for file mapping results in match command.
- Added: Language code detection in subtitle filenames.

### Changed
- Changed: Migrated audio processing pipeline to aus crate (v2), removed legacy migration code and configuration.
- Changed: Upgraded Rust edition to 2024 and updated all dependencies to latest compatible versions.
- Changed: Improved configuration documentation, usage analysis, and README examples.
- Changed: Enhanced configuration system with unified management and hot-reload.
- Changed: Improved error messages and install script for internationalization.
- Changed: Standardized changelog, commit, and release processes.

### Removed
- Removed: Unused resampling components and configuration fields.
- Removed: Legacy audio analyzer and migration modules.
- Removed: Unused parallel config items (cpu_intensive_limit, io_intensive_limit).
- Removed: Dead code and redundant configuration items.

### Fixed
- Fixed: Windows path separator handling in FileInfo and tests.
- Fixed: Test reliability on Windows CI for config integration.
- Fixed: File removal logic in convert command and related error handling.
- Fixed: Progress bar, task timeout, and idle loop exit in scheduler.
- Fixed: Various bug fixes in parallel, sync, and config modules.

### Security
- Security: Improved dependency management and CI workflows.

## [0.1.0] - 2025-06-08

### Added
- Initial release of SubX CLI tool
- Rust-based intelligent subtitle processing

[Unreleased]: https://github.com/jim60105/subx-cli/compare/v1.8.0...HEAD
[1.8.0]: https://github.com/jim60105/subx-cli/compare/v1.7.4...v1.8.0
[1.7.4]: https://github.com/jim60105/subx-cli/compare/v1.7.3...v1.7.4
[1.7.3]: https://github.com/jim60105/subx-cli/compare/v1.7.2...v1.7.3
[1.7.2]: https://github.com/jim60105/subx-cli/compare/v1.7.1...v1.7.2
[1.7.1]: https://github.com/jim60105/subx-cli/compare/v1.7.0...v1.7.1
[1.7.0]: https://github.com/jim60105/subx-cli/compare/v1.6.0...v1.7.0
[1.6.0]: https://github.com/jim60105/subx-cli/compare/v1.5.2...v1.6.0
[1.5.2]: https://github.com/jim60105/subx-cli/compare/v1.5.1...v1.5.2  
[1.5.1]: https://github.com/jim60105/subx-cli/compare/v1.5.0...v1.5.1  
[1.5.0]: https://github.com/jim60105/subx-cli/compare/v1.4.0...v1.5.0  
[1.4.0]: https://github.com/jim60105/subx-cli/compare/v1.3.0...v1.4.0  
[1.3.0]: https://github.com/jim60105/subx-cli/compare/v1.2.0...v1.3.0  
[1.2.0]: https://github.com/jim60105/subx-cli/compare/v1.1.0...v1.2.0  
[1.1.0]: https://github.com/jim60105/subx-cli/compare/v1.0.0...v1.1.0  
[1.0.0]: https://github.com/jim60105/subx-cli/releases/tag/v1.0.0
