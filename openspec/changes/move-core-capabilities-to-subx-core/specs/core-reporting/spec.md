## REMOVED Requirements

### Requirement: Transport-Agnostic Reporter Seam

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/core-reporting/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: `crate::core::report` SHALL be read as `subx_core::core::report`. B2 Decision 2a declines a `pub use core::report;` alias, so that is the public path, and `src/core/report/mod.rs` is already the correct relative path inside `subx-core`.

### Requirement: Reporter Is Send and Sync

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/core-reporting/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Core-Owned AI Usage Payload

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/core-reporting/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Reporter Attachment Preserves Constructor Signatures

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/core-reporting/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: No Core or Service Module References the CLI Layer

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/core-reporting/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: This requirement and B2's `crate-topology` requirement "`subx-core` Never References `subx-cli`" state overlapping invariants. `crate-topology` stays in `subx-cli` (it governs the relationship between the repositories), so the moved copy SHALL note that the enforced, post-split form of the boundary lives there, and SHALL NOT be deleted here — it is the reason the `Reporter` seam exists and it is the invariant `subx-core`'s own guard test enforces.

