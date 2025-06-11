# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/SubX-Project/SubX/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/SubX-Project/SubX/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/SubX-Project/SubX/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/SubX-Project/SubX/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/SubX-Project/SubX/releases/tag/v0.1.0
