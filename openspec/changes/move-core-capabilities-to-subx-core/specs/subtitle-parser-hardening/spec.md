## REMOVED Requirements

### Requirement: ASS parser error recovery on missing format fields

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/subtitle-parser-hardening/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: ASS timestamp overflow protection

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/subtitle-parser-hardening/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: SRT parser continues on malformed blocks

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/subtitle-parser-hardening/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: SUB parser safe timestamp conversion

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/subtitle-parser-hardening/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Malformed-input disposition matrix

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/subtitle-parser-hardening/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Property-style parser fuzz coverage gated by feature

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/subtitle-parser-hardening/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: `slow-tests` is declared by `subx-core` as the real gate over the tests that live there (B3), and "`Cargo.lock` resolution is unchanged" SHALL be read against `subx-core`'s own committed lockfile (B1 Decision 5b), not the workspace root's.

### Requirement: Optional cargo-fuzz harness lives outside default workspace

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/subtitle-parser-hardening/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: The harness belongs to the repository holding the parsers, so `fuzz/` SHALL be read as `subx-core/fuzz/` and "excluded from the main `Cargo.toml` workspace members" SHALL be restated as excluded from both workspace roots — `subx-cli/Cargo.toml` and any standalone `subx-core` build. The two `scripts/quality_check.sh` citations SHALL name the owning repository's quality script, which C1 Decision 4 gives `subx-core` in its own right.

### Requirement: SRT and VTT parsers handle CRLF line endings

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/subtitle-parser-hardening/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

