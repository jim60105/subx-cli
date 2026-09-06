## REMOVED Requirements

### Requirement: Relocation Modes

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/file-organization/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Conflict Resolution

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/file-organization/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Filename conflict resolution uses atomic creation

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/file-organization/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Rename operation conflict check

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/file-organization/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Rollback-Safe File Operations

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/file-organization/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Backup Before Move

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/file-organization/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: AutoRename Sequential Suffixes

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/file-organization/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: `tests/match_duplicate_rename_conflict_tests.rs` imports `subx_cli::cli::MatchArgs` and `subx_cli::commands::match_command`, so B3 classifies it CLI-bound and it stays in `subx-cli`. The citation SHALL be rewritten to the repository-qualified form `subx-cli:tests/match_duplicate_rename_conflict_tests.rs` rather than left to resolve against the wrong root.

