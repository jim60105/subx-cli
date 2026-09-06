## REMOVED Requirements

### Requirement: Redact API keys in Debug output

**Reason**: Core half of the split. `OpenAIClient`, `OpenRouterClient` and `AzureOpenAIClient` are under `src/services/ai/` and `AIConfig` is in `src/config/mod.rs`; all four are `subx-core` after B2. Re-added verbatim by `import-split-capability-specs`, at `openspec/specs/secrets-protection/spec.md` in that repository. It leaves this repository's half of the capability; it does not leave the project.

### Requirement: Restrict config file permissions

**Reason**: Core half of the split. The `0o600` file mode and `0o700` directory mode are applied at `src/config/service.rs:31`, `:39` and `:43`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

**Migration**: The `configuration-management` capability's *Config file permissions enforcement* requirement states a stricter form of the same obligation and also migrates in this change, so the pre-existing overlap between the two moves intact rather than being split across the boundary. Both arrive in `subx-core`; de-duplicating them is not this change's decision to make.

### Requirement: Warn on insecure HTTP endpoint

**Reason**: Core half of the split. The plaintext-transmission warning is emitted by the AI client constructors under `src/services/ai/`, and after A1 it reaches the user through the `Reporter` seam rather than a direct write. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

## MODIFIED Requirements

### Requirement: Mask sensitive config values in CLI output

The `config set`, `config list`, and `config get` subcommands SHALL mask the values of sensitive keys in everything they write to stdout, by passing each key and value through the masking helper before display and never printing the raw value alongside the masked one.

Which keys are sensitive and what the masked form looks like — the `api_key` / `token` / `secret` case-insensitive match, the `****<last 4 chars>` form, and the `****` form for values of four characters or fewer — is specified by the `secrets-protection` capability's *Sensitive Value Masking Helper* requirement in `subx-core`, which owns `mask_sensitive_value` in `src/config/masking.rs`. This requirement owns only the obligation that the display sites use it.

This capability and the `configuration-management` capability's *Sensitive value masking in config display* requirement have specified overlapping obligations over these same display sites since both were written. The overlap is preserved unchanged: both stay in `subx-cli`, and deciding which of the two owns config-display masking is left to a later change.

#### Scenario: config set echoes masked value

- **WHEN** user runs `config set ai.api_key "sk-abc123def456"`
- **THEN** stdout shows `****f456` not the full key

#### Scenario: config list masks api_key

- **WHEN** user runs `config list`
- **THEN** the `api_key` field displays `****<last4>` instead of the plaintext value
