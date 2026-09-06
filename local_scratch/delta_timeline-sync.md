## REMOVED Requirements

### Requirement: Sync Method Selection

**Reason**: Core half of the split. The `sync.default_method` fallback and `SyncEngine::new`'s unconditional VAD requirement are in `src/core/sync/engine.rs` and `src/services/vad/`, both `subx-core` after B2, and the GUI constructs `SyncEngine` directly (SDR §8). Re-added in `subx-core` by `import-split-capability-specs`, at `openspec/specs/timeline-sync/spec.md` in that repository. It leaves this repository's half of the capability; it does not leave the project.

**Migration**: The `--method` flag and the *Manual mode requires an explicit offset* scenario — whose message `Manual method requires --offset parameter.` is produced by `SyncArgs::validate` in `src/cli/sync_args.rs` — are `subx-cli` obligations and are lifted into bullet 1 of *Sync Argument Struct Is a Thin Adapter Over Core Pairing*, added below. The arriving requirement SHALL state the two methods as values the caller selects, with the `--method` surface named as `subx-cli`'s, and SHALL keep the *VAD detector is required unconditionally* scenario.

### Requirement: Offset Clamping Against Maximum

**Reason**: Core half of the split. `apply_manual_offset` and `vad_detect_sync_offset`, together with the `SyncResult.warnings` and `additional_info` population, are in `src/core/sync/engine.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Subtitle Timing Application

**Reason**: Core half of the split. The per-entry shift and the clamp to `Duration::ZERO` are in `src/core/sync/engine.rs`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: VAD Audio Processing

**Reason**: Core half of the split. The requirement already names "Implemented in `src/services/vad/`", and both test citations — `tests/vad_audio_processor_tests.rs` and `tests/vad_integration_tests.rs` — are core-bound under B3's ownership test and move with it. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: B3 preserves both basenames when it flattens them into `subx-core/tests/`, and neither is among the three files B4 renames, so both citations are correct verbatim and SHALL NOT be rewritten. The `hound` dev-dependency those tests need moves to `subx-core`'s `[dev-dependencies]` with them, per A0 and B3.

### Requirement: VAD Detector Behavior

**Reason**: Core half of the split. `LocalVadDetector` and `VadSyncDetector::detect_sync_offset` are in `src/services/vad/`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: First-Sentence Offset Annotation

**Reason**: Core half of the split. The `SyncResult.additional_info` population is in `src/core/sync/engine.rs`, and `tests/sync_first_sentence_offset_integration_tests.rs` is core-bound under B3's ownership test. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: The scenario's "real audio+subtitle asset pair" is the `assets/SubX - The Subtitle Revolution.{srt,mp3}` pair, which B3 moves to `subx-core/assets/` and re-resolves from `CARGO_MANIFEST_DIR`. The citation is a test file, not an asset path, so it is carried over verbatim; the asset relocation is recorded here only so that a reader who follows the test does not conclude the spec is stale.

### Requirement: VAD Padding Chunks Configuration

**Reason**: Core half of the split, and the second requirement in this change most likely to be misfiled. It reads as a user-facing knob and sits between two requirements about CLI overrides, but the requirement already names "Implemented in `src/services/vad/detector.rs` (the `vad.label(..., padding_chunks, ...)` call) and defined in `src/config/mod.rs::VadConfig`" — two core files. Adjacency in a file is not ownership evidence. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Core-Owned Sync Pairing Resolution

**Reason**: Core half of the split, and the requirement A2 wrote precisely to make this move possible: it defines `SyncMode`, `BatchRequest`, `SyncPairingRequest`, `resolve_sync_pairing`, `SYNC_VIDEO_EXTENSIONS` and `SYNC_SUBTITLE_EXTENSIONS` in `src/core/sync/mod.rs`, and the six-step resolution algorithm with them. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: Its penultimate paragraph is entirely a `subx-cli` obligation — that `SyncArgs::get_sync_mode` becomes a thin adapter translating `Option<Option<PathBuf>>` into `BatchRequest` and `is_manual_mode()` into `manual`, containing no filesystem access and no pairing logic, and that `crate::cli::SyncMode` remains a legacy re-export without `#[deprecated]`. It is lifted into bullets 2 and 4 of *Sync Argument Struct Is a Thin Adapter Over Core Pairing*, together with the *The CLI adapter adds no behaviour* scenario. The final paragraph — that the batch pairing performed afterwards inside `src/commands/sync_command.rs` is a separate command-level concern — SHALL be carried over with the path qualified as `subx-cli:src/commands/sync_command.rs` and the three requirements that specify it (*Batch Prefix-Match Pairing*, *Batch Skip Directories Without Videos*, *Batch Single-Pair Override*) named as being in `subx-cli`. The other seven scenarios stay with the core half.

### Requirement: Core-Owned Default Output Path Derivation

**Reason**: Core half of the split. A2 defines `create_default_output_path` in `src/core/sync/mod.rs`, and SDR §2.1 records that the GUI consumes it. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: Its second paragraph is a `subx-cli` obligation — that `crate::cli::sync_args::create_default_output_path` remains a legacy re-export documented in rustdoc without `#[deprecated]`, and that in-crate callers reference the core path. It is lifted into bullet 3 of *Sync Argument Struct Is a Thin Adapter Over Core Pairing*. All three scenarios stay with the core half. The in-crate caller `src/commands/sync_command.rs` named there is a `subx-cli` file and SHALL NOT appear unqualified in the arriving requirement.

## MODIFIED Requirements

### Requirement: Single-File and Batch Modes

The `sync` command SHALL support a single-pair mode (via `--video` + `--subtitle`, positional paths, or manual mode with only a subtitle) and a batch mode (via `--batch [DIR]` combined with `-i`, positional paths, or an explicit directory) that pairs videos with subtitles inside the same directory.

The decision between the two modes, and the auto-pairing that backs single-pair mode, SHALL be performed by the `timeline-sync` capability's *Core-Owned Sync Pairing Resolution* requirement in `subx-core`, whose `resolve_sync_pairing` is reachable from this crate as `subx_cli::core::sync::resolve_sync_pairing` through the re-export surface the `crate-topology` capability specifies. The `sync` command's clap struct SHALL contribute only the flag definitions and the field-to-request adaptation; it SHALL NOT read the filesystem.

#### Scenario: Batch mode without any input
- **GIVEN** the user passes `--batch` with no directory, no `-i`, no positional path, and no `--video` or `--subtitle`
- **WHEN** argument validation runs
- **THEN** validation SHALL fail with a message explaining that batch mode requires at least one input source

#### Scenario: Mode selection is reproducible outside the CLI
- **GIVEN** a caller that constructs a `SyncPairingRequest` directly, without parsing a command line
- **WHEN** it calls `resolve_sync_pairing`
- **THEN** it SHALL receive the same `SyncMode` the `sync` command would have resolved for the equivalent arguments

## ADDED Requirements

### Requirement: Sync Argument Struct Is a Thin Adapter Over Core Pairing

`SyncArgs` and the `sync` command SHALL own the flag surface and the legacy aliases, and nothing else. Every pairing and path-derivation behaviour is specified by the `timeline-sync` capability in `subx-core`; this requirement states what remains in `src/cli/sync_args.rs`.

1. **Method flag and its validation.** The command SHALL accept `--method` with the values `vad` and `manual`, and SHALL omit the option to select the configured `sync.default_method`. When `--method manual` is supplied without `--offset`, `SyncArgs::validate` SHALL fail with the message `Manual method requires --offset parameter.` This is argument validation, not engine behaviour: no core file is consulted to produce it.
2. **Pairing adapter.** `SyncArgs::get_sync_mode` SHALL populate a `SyncPairingRequest` from its own fields — translating clap's `Option<Option<PathBuf>>` for `--batch` into `BatchRequest`, and `is_manual_mode()` into `manual` — and SHALL return `resolve_sync_pairing`'s result unchanged. It SHALL contain no filesystem access and no pairing logic of its own.
3. **Output-path adapter.** `SyncArgs::get_output_path` SHALL derive its default through the core `create_default_output_path`, and SHALL NOT reimplement the `<file_stem>_synced.<extension>` derivation. In-crate callers, including `src/commands/sync_command.rs`, SHALL reference the core path rather than the legacy alias.
4. **Legacy aliases.** `crate::cli::SyncMode` and `crate::cli::sync_args::create_default_output_path` SHALL remain available as legacy re-exports of their `subx-core` originals, documented as such in rustdoc, without a `#[deprecated]` attribute, so that existing consumers keep compiling.

#### Scenario: Manual mode requires an explicit offset
- **GIVEN** the user passes `--method manual` without `--offset`
- **WHEN** argument validation runs
- **THEN** validation SHALL fail with the message `Manual method requires --offset parameter.`

#### Scenario: The CLI adapter adds no behaviour
- **GIVEN** any `SyncArgs` value
- **WHEN** `SyncArgs::get_sync_mode` is called
- **THEN** the result SHALL equal `resolve_sync_pairing` applied to the `SyncPairingRequest` built from that value's fields

#### Scenario: Legacy sync aliases still resolve
- **GIVEN** a consumer that writes `use subx_cli::cli::SyncMode;` or `use subx_cli::cli::sync_args::create_default_output_path;`
- **WHEN** the crate is compiled
- **THEN** both imports SHALL resolve to their `subx-core` originals and SHALL produce no deprecation warning
