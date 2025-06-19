# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/jim60105/subx-cli/compare/v1.2.0...HEAD  
[1.2.0]: https://github.com/jim60105/subx-cli/compare/v1.1.0...v1.2.0  
[1.1.0]: https://github.com/jim60105/subx-cli/compare/v1.0.0...v1.1.0  
[1.0.0]: https://github.com/jim60105/subx-cli/releases/tag/v1.0.0
