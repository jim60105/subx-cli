## REMOVED Requirements

### Requirement: Non-blocking file I/O in all async functions

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/async-runtime-safety/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Scheduler active-task cleanup on all code paths

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/async-runtime-safety/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Scheduler loop restart on submission

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/async-runtime-safety/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Library Engine Types Are `Send` and `Sync`

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/async-runtime-safety/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: This requirement was added by D1 (`make-core-engines-thread-safe`) after this proposal's census, and its eight citations are self-qualified as `subx-core/src/...`. Inside `subx-core` a path naming one's own repository is root-relative and unqualified, so on arrival the `subx-core/` prefix is stripped from all eight instances (`src/core/factory.rs`, `src/core/file_manager.rs`, `src/core/formats/converter.rs`, `src/core/formats/manager.rs`, `src/core/matcher/engine.rs`, `src/core/mod.rs`, `src/core/sync/engine.rs`, `src/core/translation/engine.rs`). No other text changes.
