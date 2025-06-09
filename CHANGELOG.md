# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/SubX-Project/SubX/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/SubX-Project/SubX/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/SubX-Project/SubX/releases/tag/v0.1.0
