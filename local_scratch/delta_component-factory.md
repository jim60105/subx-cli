## REMOVED Requirements

### Requirement: ConfigService-Driven Construction

**Reason**: Core half of the split. `ComponentFactory::new` and the cached `Config` live in `src/core/factory.rs`, which is `subx-core` after B2, and the Tauri GUI constructs `ComponentFactory` directly (SDR §8). Re-added verbatim by `import-split-capability-specs`, at `openspec/specs/component-factory/spec.md` in that repository. It leaves this repository's half of the capability; it does not leave the project.

### Requirement: AI Provider Creation

**Reason**: Core half of the split. `create_ai_provider` dispatches over the providers under `src/services/ai/`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Pre-Construction Configuration Validation

**Reason**: Core half of the split. The requirement already names `validate_ai_config` in `src/core/factory.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Match Engine Creation

**Reason**: Core half of the split. `create_match_engine`, `MatchConfig` and `MatchEngine::new` are all under `src/core/`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: VAD and Audio Component Creation

**Reason**: Core half of the split. `create_vad_sync_detector`, `create_vad_detector` and `create_audio_processor` construct services under `src/services/vad/`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Tests Use TestConfigService via TestConfigBuilder

**Reason**: Core half of the split, contrary to how it reads. Its unit-test scenario cites a test inside `src/core/factory.rs`, and all three integration tests it names are core-bound under B3's ownership test: `tests/openrouter_integration_tests.rs` and `tests/azure_openai_api_integration_tests.rs` import only `config`, `core` and `services`, and `tests/dependency_injection_integration_tests.rs` is listed core-side by B3 (tasks 5.5, design line 137). Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: One citation crosses the line. `docs/testing-guidelines.md` is a `subx-cli` file and SHALL be qualified as `subx-cli:docs/testing-guidelines.md` on arrival. The three `tests/` citations SHALL be carried over unqualified, because B3 preserves their basenames when it flattens them into `subx-core/tests/`. If B3 or B4 re-files `tests/dependency_injection_integration_tests.rs` to `subx-cli` — its `subx_cli::App` import makes B3's own rule 3 fire only loosely — that one citation becomes `subx-cli:tests/dependency_injection_integration_tests.rs` and nothing else changes.
