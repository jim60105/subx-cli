## REMOVED Requirements

### Requirement: Language Code Normalization Table

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/language-detection/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: The citation `src/core/language.rs:58-75` is repository-root-relative and is already correct inside `subx-core`, because B2 relocated `src/core/**` at an identical relative path. The line range SHALL be re-verified against the destination tree at import time and corrected if A1 or A2 shifted it.

### Requirement: Directory-Name Detection

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/language-detection/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Filename-Pattern Detection

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/language-detection/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: As above for `src/core/language.rs:77-81`: the path is correct verbatim; the line range is re-verified at import time.

### Requirement: Directory Evidence Outranks Filename Evidence

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/language-detection/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Integration as Match-Engine Metadata

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/language-detection/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: `src/core/matcher/mod.rs:343-344` and `src/core/matcher/engine.rs:766-782` are correct verbatim in `subx-core`, but `engine.rs` was edited by A1 (five `println!` guard sites and `operation_error_from`). Both ranges SHALL be re-verified against the destination tree and corrected before the import is committed.

