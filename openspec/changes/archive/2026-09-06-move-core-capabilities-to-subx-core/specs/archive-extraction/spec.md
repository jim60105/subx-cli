## REMOVED Requirements

### Requirement: Zip Archive Extraction

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/archive-extraction/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: RAR Archive Extraction

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/archive-extraction/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: "gated behind a cargo feature flag `archive-rar`" SHALL be read as the real gate in `subx-core` (`archive-rar = ["dep:unrar"]`); `subx-cli`'s same-named feature is the pass-through that the release workflow enables, per B2's "Feature Flags Are Gated in Core and Forwarded by the CLI".

### Requirement: 7z Archive Extraction

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/archive-extraction/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Tar-Gzip Archive Extraction

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/archive-extraction/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Format Detection by Extension

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/archive-extraction/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Temporary Directory Lifecycle

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/archive-extraction/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: No Nested Archive Extraction

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/archive-extraction/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Direct-Input-Only Extraction

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/archive-extraction/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: `-i` is a `subx-cli` flag, but the normative object is `collect_files()`, which A2 moved to `subx_core::core::input::InputPathHandler`. The requirement moves; the two scenarios SHALL name the flag as `subx-cli`'s and keep `collect_files()` as the subject.

### Requirement: Error Handling for Corrupt or Protected Archives

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/archive-extraction/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Decompression Bomb Protection

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/archive-extraction/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

