## REMOVED Requirements

### Requirement: Match Cache Location

**Reason**: Core half of the split. The cache artefact is produced by `save_file_list_cache` in `src/core/matcher/engine.rs`, and the path is joined at `src/core/matcher/engine.rs:2244`; both are in `subx-core` after B2. Re-added verbatim there by `import-split-capability-specs`, at `openspec/specs/cache-management/spec.md`. It leaves this repository's half of the capability; it does not leave the project.

**Migration**: `subx-cli` resolves the same path independently at `src/commands/cache_command.rs:224` (via `get_config_dir()` at `:218`), and after the split nothing forces the two resolvers to agree. The agreement obligation is added to this capability's CLI half, in the restated *Cache Clear Subcommand* below. The arriving requirement's prose SHALL name that CLI half as the holder of the agreement obligation, without citing a `subx-cli` path.

### Requirement: Configuration-Aware Invalidation

**Reason**: Core half of the split. `CacheData`'s `config_hash`, `original_relocation_mode` and `original_backup_enabled` fields are written by `save_file_list_cache` in `src/core/matcher/engine.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Cache Reuse Preserves Relocation Mode

**Reason**: Core half of the split. The requirement already names "Implemented in `src/core/matcher/engine.rs`", which is `subx-core` after B2. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: Its two test citations are CLI-bound under B3's ownership test — `tests/match_cache_reuse_tests.rs` and `tests/match_cache_target_directory_tests.rs` both import `subx_cli::cli::MatchArgs` and `subx_cli::commands::match_command`, so B3 rule 2 fires and they stay here. Both SHALL be qualified as `subx-cli:tests/match_cache_reuse_tests.rs` and `subx-cli:tests/match_cache_target_directory_tests.rs` on arrival.

### Requirement: Dry-Run Cache Reuse Without AI Calls

**Reason**: Core half of the split. `check_file_list_cache` and `save_file_list_cache` are in `src/core/matcher/engine.rs`. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The citation `tests/match_cache_reuse_tests.rs` is CLI-bound (see above) and SHALL be qualified as `subx-cli:tests/match_cache_reuse_tests.rs` on arrival.

### Requirement: Cache Invalidation On Relocation-Affecting Config Change

**Reason**: Core half of the split. The requirement already names "Implemented in `src/core/matcher/engine.rs::calculate_config_hash`" (`:2248`). Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Cache Scoped To File-List Directory Key

**Reason**: Core half of the split. The `filelist_<hash>` key is computed in `src/core/matcher/engine.rs`. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The citation `tests/match_cache_target_directory_tests.rs` is CLI-bound and SHALL be qualified as `subx-cli:tests/match_cache_target_directory_tests.rs` on arrival.

## MODIFIED Requirements

### Requirement: Cache Clear Subcommand

The system SHALL expose `subx cache clear` as the only cache subcommand (`CacheAction::Clear`); executing it SHALL delete the match cache file if it exists and SHALL report a user-visible confirmation message.

The path the subcommand deletes SHALL be the same path the cache producer writes. `subx-cli` resolves it independently in `src/commands/cache_command.rs` (`cache_path()`, over `get_config_dir()`), and the producer resolves it in `subx-core` under the `cache-management` capability's *Match Cache Location* requirement in that repository. Because the two resolvers are now in different repositories, with no compiler and no shared constant binding them, this requirement SHALL be treated as an agreement obligation on the CLI side: a change to either resolver SHALL be accompanied by a check that the other still produces the identical path, and the `XDG_CONFIG_HOME` override honoured by `get_config_dir()` SHALL be honoured by the producer too. A divergence is silent — `cache clear` reports `No cache file found` while a cache remains on disk — so it SHALL NOT be left to be discovered by a user.

#### Scenario: Clear existing cache
- **GIVEN** `$CONFIG_DIR/subx/match_cache.json` exists
- **WHEN** the user runs `subx cache clear`
- **THEN** the file SHALL be removed and the command SHALL print `Cache file cleared: <path>`

#### Scenario: Clear when no cache exists
- **GIVEN** no match cache file exists
- **WHEN** the user runs `subx cache clear`
- **THEN** the command SHALL print `No cache file found` and exit successfully without creating any file

#### Scenario: The two resolvers agree across the repository boundary
- **GIVEN** a run of `subx match` that wrote a match cache, and any value of `$XDG_CONFIG_HOME` in effect for both invocations
- **WHEN** the user then runs `subx cache clear`
- **THEN** the path `subx-cli` resolves SHALL be byte-identical to the path the producer in `subx-core` wrote, so that the file is found and removed rather than reported absent
