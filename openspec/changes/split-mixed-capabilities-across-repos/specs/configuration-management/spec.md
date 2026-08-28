## REMOVED Requirements

### Requirement: Unified Configuration Schema

**Reason**: Core half of the split. `Config` and its five sub-structs are defined in `src/config/mod.rs`, which is `subx-core` after B2, and the Tauri GUI consumes `config::Config` directly (SDR §8). Re-added verbatim by `import-split-capability-specs`, at `openspec/specs/configuration-management/spec.md` in that repository. It leaves this repository's half of the capability; it does not leave the project.

### Requirement: Configuration Service Abstraction

**Reason**: Core half of the split. The `ConfigService` trait, `ProductionConfigService` and `TestConfigService` are all in `src/config/`, and SDR D10 makes the test implementations unconditional public API of `subx-core`. Its scenario reaches the trait through a command invocation, which `spec-governance` lists as narrative framing rather than ownership evidence; the CLI-side obligation it describes is already stated in full by the `component-factory` capability's *Commands Consume Services via Dependency Injection*, which stays here — so this requirement is moved whole rather than split, to avoid two requirements in one repository constraining the same three command files. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Value Validation

**Reason**: Core half of the split. The normative object is the field validator in `src/config/field_validator.rs`; the `subx config set` phrasing of both scenarios is narrative framing. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Boolean Value Flexibility

**Reason**: Core half of the split. The boolean alias table is applied in `src/config/field_validator.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: AI Environment Variable Overrides

**Reason**: Core half of the split. The requirement already names "Implemented in `src/config/service.rs:222-251`". Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The line range `src/config/service.rs:222-251` SHALL be re-verified against the destination tree and corrected if B2's move or any earlier change shifted it, per `spec-governance`'s *A line range is re-verified at the boundary*. The `local-llm-provider` capability in `subx-core` references this requirement's precedence rule and is re-qualified from `subx-cli` to unqualified by `import-split-capability-specs`.

### Requirement: Custom Configuration File Path

**Reason**: Core half of the split. `SUBX_CONFIG_PATH` is honoured by `ProductionConfigService` in `src/config/service.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Legacy Sync Configuration Rejected

**Reason**: Core half of the split. The requirement names the `SyncConfig` schema in `src/config/mod.rs`; its test citation `tests/config_migration_tests.rs` is core-bound under B3's ownership test and moves with it. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Config Service Reload

**Reason**: Core half of the split. `ProductionConfigService::reload` is in `src/config/service.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Compatibility Environment Variables For Third-Party Providers

**Reason**: Core half of the split. The requirement already names "Implemented in `src/config/service.rs`", and the local-provider carve-out is applied there. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: AI Provider Identifier Canonicalization

**Reason**: Core half of the split. `normalize_ai_provider` is located in `src/config/field_validator.rs` and the GUI consumes it directly (SDR §8). All five call sites the requirement enumerates are core files after B2: `field_validator.rs`, `service.rs`, `validator.rs` and `src/core/factory.rs`. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: Enumerated call sites 1 and 2 are phrased as `subx config set ai.provider <value>` and `subx config get ai.provider`; both are narrative framing over the field validator and SHALL be carried over with the invocation named as `subx-cli`'s command surface, leaving the field validator as the normative subject — the same treatment C2a applied to `media-discovery`'s `--recursive` sentence. The `local-llm-provider` capability in `subx-core` references this requirement and is re-qualified from `subx-cli` to unqualified by `import-split-capability-specs`.

### Requirement: Local Provider Validation Rules

**Reason**: Core half of the split. `validate_ai_config` and its local arm are in `src/config/validator.rs`, and `field_validator.rs` holds the allow-list. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: This requirement carries C2a's edit and supersedes it, and the distinction matters because C2a recorded the risk that this change would silently revert it. C2a restated the requirement so that its reference reads "the HTTPS-required rule documented for hosted providers in the `ai-provider-integration` capability **in `subx-core`**", and added a scenario *The cross-repository reference resolves* asserting that the qualification reads correctly to someone holding only a `subx-cli` checkout. On arrival: (1) the qualification **in `subx-core`** SHALL be removed, because `ai-provider-integration` is in the same repository as the arriving requirement and `spec-governance` reads an unqualified capability reference as a same-repository reference; (2) C2a's added scenario SHALL be retired with it, because its GIVEN — "a reader of this requirement holding only a `subx-cli` checkout" — can no longer occur. The five original scenarios are carried over verbatim. The arriving text therefore differs from both the pre-C2a and the post-C2a text, which is what distinguishes this from the reversion C2a warned against.

### Requirement: Local Provider Environment Variables

**Reason**: Core half of the split. `LOCAL_LLM_BASE_URL` and `LOCAL_LLM_API_KEY` are applied by `ProductionConfigService` in `src/config/service.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Config file permissions enforcement

**Reason**: Core half of the split. The `0o700` directory mode, the `OpenOptionsExt::mode(0o600)` open and the post-write `set_permissions` are at `src/config/service.rs:31`, `:39` and `:43`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

## MODIFIED Requirements

### Requirement: Repair Path For Strict-Invalid Configuration

The system SHALL allow `subx config set <key> <value>`, `subx config get <key>`, and `subx config list` to operate on a `config.toml` whose contents fail cross-section (strict) validation, so long as the file parses as TOML and each individual field passes field-level validation.

The tolerant load these three subcommands use, its exclusion of environment-variable overlays, the post-mutation strict validation performed before the file is written, the cache invariant, and the unchanged strict load path used by every other entry point are specified by the `configuration-management` capability's *Tolerant Configuration Load Path* requirement in `subx-core`. This requirement governs only what the three subcommands do with the result.

For `subx config set`, when the post-mutation configuration fails cross-section validation the command SHALL fail, SHALL NOT print a success confirmation, and the on-disk file SHALL remain unchanged. When it is strict-valid the command SHALL report success.

For `subx config get` and `subx config list`, the command SHALL produce its normal output and SHALL additionally surface a non-fatal warning when the on-disk configuration is strict-invalid. In text mode the warning SHALL be a single line on stderr beginning with `warning: configuration is currently invalid:` followed by the validator's message. When the global `--output json` flag is in effect, the warning SHALL be appended to the existing `Envelope::warnings` array (a `Vec<String>` per the machine-readable-output capability); when the configuration is strict-valid the `warnings` field SHALL remain absent from the JSON document, matching today's shape. The exit code SHALL remain `0` for successful reads even when the warning is emitted.

The advisory SHALL track on-disk state, not the environment-merged effective view, which follows from the tolerant load's exclusion of environment overlays.

#### Scenario: Repair via provider switch from a strict-invalid pairing

- **GIVEN** `~/.config/subx/config.toml` contains `ai.provider = "openai"` and `ai.base_url = "http://localhost:1234/v1"`, which fails cross-section validation because hosted providers require `https://`
- **WHEN** the user runs `subx config set ai.provider local`
- **THEN** the command SHALL exit with status `0`, the on-disk file SHALL contain `ai.provider = "local"`, the existing `ai.base_url` and `ai.api_key` values SHALL be preserved verbatim, and the resulting file SHALL pass strict cross-section validation

#### Scenario: Repair via base_url switch from a strict-invalid pairing

- **GIVEN** `~/.config/subx/config.toml` contains `ai.provider = "openai"` and `ai.base_url = "http://localhost:1234/v1"`
- **WHEN** the user runs `subx config set ai.base_url https://api.openai.com/v1`
- **THEN** the command SHALL exit with status `0`, the on-disk file SHALL contain the new `https://` URL, the existing `ai.provider` value SHALL be preserved, and the resulting file SHALL pass strict cross-section validation

#### Scenario: Non-repair edit on a strict-invalid file is rejected

- **GIVEN** `~/.config/subx/config.toml` contains `ai.provider = "openai"` and `ai.base_url = "http://localhost:1234/v1"`
- **WHEN** the user runs `subx config set general.backup_enabled true` (an unrelated key whose mutation does not heal the cross-section error)
- **THEN** the command SHALL fail with the standard cross-section error naming the offending `ai.base_url` scheme, the on-disk file SHALL remain byte-identical to its prior contents, and the in-memory cache SHALL NOT be populated

#### Scenario: Field-level invalid new value is still rejected on a strict-invalid file

- **GIVEN** `~/.config/subx/config.toml` is strict-invalid for any reason
- **WHEN** the user runs `subx config set sync.max_offset_seconds -5` (a value the field validator rejects)
- **THEN** the command SHALL fail with the field-level error explaining the acceptable range, and the on-disk file SHALL remain unchanged

#### Scenario: `config get` on a strict-invalid file emits the value plus an advisory

- **GIVEN** `~/.config/subx/config.toml` is strict-invalid because of the `provider=openai + http://` pairing
- **WHEN** the user runs `subx config get ai.base_url`
- **THEN** the command SHALL exit with status `0`, stdout SHALL contain `http://localhost:1234/v1`, and stderr SHALL contain a single-line advisory beginning with `warning: configuration is currently invalid:` followed by the validator's message

#### Scenario: `config list` JSON output on a strict-invalid file populates `warnings`

- **GIVEN** `~/.config/subx/config.toml` is strict-invalid
- **WHEN** the user runs `subx-cli --output json config list`
- **THEN** the command SHALL exit with status `0`, stdout SHALL be valid JSON, and the JSON SHALL include a top-level `warnings` array containing at least one non-empty string reproducing the validator's error

#### Scenario: `config list` JSON output on a strict-valid file omits `warnings`

- **GIVEN** `~/.config/subx/config.toml` is strict-valid
- **WHEN** the user runs `subx-cli --output json config list`
- **THEN** the JSON output SHALL NOT include a `warnings` field (or SHALL include `warnings: null`, matching today's `Option::is_none` serialization), and the document SHALL otherwise be byte-equivalent to the pre-change output for the same file

#### Scenario: Advisory reflects file state, not env-merged state

- **GIVEN** `~/.config/subx/config.toml` is strict-valid (e.g. `ai.provider = "local"` with `ai.base_url = "http://localhost:1234/v1"`) but environment variables would create a strict-invalid effective view (e.g. `SUBX_AI_PROVIDER=openai` is exported)
- **WHEN** the user runs `subx config get ai.base_url`
- **THEN** the command SHALL exit with status `0`, stdout SHALL contain the file's `ai.base_url`, and stderr SHALL NOT contain an "configuration is currently invalid" advisory (because the file itself is valid; advisories track on-disk state)
