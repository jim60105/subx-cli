# Cache Management

## Purpose

Store and invalidate the subtitle-matching cache that allows `subx match` to skip repeated AI calls and to apply previously computed dry-run results. Implemented in `src/commands/cache_command.rs`, `src/cli/cache_args.rs`, and `src/core/matcher/cache.rs`.

## Requirements

### Requirement: Match Cache Location

The system SHALL persist the match cache as a JSON file at `$CONFIG_DIR/subx/match_cache.json`, where `$CONFIG_DIR` is resolved via the standard platform configuration directory (`dirs::config_dir()`).

#### Scenario: Cache persisted after a match run
- **GIVEN** `subx match` executes successfully and produces at least one match operation
- **WHEN** the run finishes
- **THEN** a file at `$CONFIG_DIR/subx/match_cache.json` SHALL contain the serialized `CacheData` including file snapshot, match operations, and a configuration hash

### Requirement: Cache Clear Subcommand

The system SHALL expose `subx cache clear` as the only cache subcommand (`CacheAction::Clear`); executing it SHALL delete the match cache file if it exists and SHALL report a user-visible confirmation message.

#### Scenario: Clear existing cache
- **GIVEN** `$CONFIG_DIR/subx/match_cache.json` exists
- **WHEN** the user runs `subx cache clear`
- **THEN** the file SHALL be removed and the command SHALL print `Cache file cleared: <path>`

#### Scenario: Clear when no cache exists
- **GIVEN** no match cache file exists
- **WHEN** the user runs `subx cache clear`
- **THEN** the command SHALL print `No cache file found` and exit successfully without creating any file

### Requirement: Configuration-Aware Invalidation

The cache SHALL record the configuration hash and relocation settings under which its entries were generated so that changes in matching-relevant configuration invalidate prior results.

#### Scenario: Config hash stored with cache
- **GIVEN** a cache is written by `save_file_list_cache`
- **WHEN** the JSON file is inspected
- **THEN** it SHALL contain a `config_hash` field, an `original_relocation_mode` field, and an `original_backup_enabled` field reflecting the settings in effect during the producing run
