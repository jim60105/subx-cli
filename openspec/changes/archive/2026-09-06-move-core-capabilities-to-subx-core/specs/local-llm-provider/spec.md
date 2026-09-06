## REMOVED Requirements

### Requirement: Local LLM Provider Identifier

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/local-llm-provider/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: The reference to "the `configuration-management` capability" now crosses a repository boundary and SHALL be qualified as that capability in `subx-cli`. C2b re-qualifies it to `subx-core` when it splits `configuration-management` and sends `normalize_ai_provider`'s half across.

### Requirement: OpenAI-Compatible Local Chat Completions

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/local-llm-provider/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Optional API Key, Required Base URL

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/local-llm-provider/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Privacy and Offline Network Policy

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/local-llm-provider/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

### Requirement: Local Provider Environment Variable Overrides

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/local-llm-provider/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: As above: the `configuration-management` reference is qualified to `subx-cli` here and re-qualified by C2b.

### Requirement: Actionable Local-Endpoint Error Mapping

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/local-llm-provider/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

**Migration**: The reference to the `error-handling` capability's URL-redaction requirement is qualified to `subx-cli` here; C2b re-qualifies it when it splits `error-handling`, whose redaction half is core-owned.

### Requirement: Shared Retry, Prompt, and Response Behavior

**Reason**: The whole capability moves to `subx-core`. This requirement is re-added verbatim there by the `import-core-specs` change, at `openspec/specs/local-llm-provider/spec.md` in that repository. It leaves this repository's tree; it does not leave the project.

