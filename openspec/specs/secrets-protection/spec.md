# Secrets Protection

## Purpose

Hold the display-site half of the secrets-protection agreement: `config set`, `config list`, and `config get` SHALL mask sensitive configuration values in CLI output by calling the core masking helper and SHALL never print raw values. Implemented in `src/commands/config_command.rs` against `subx-core:src/config/masking.rs`. The masking helper itself, API-key `Debug` redaction, the permission check, and the insecure-HTTP warning are specified by the same-named `secrets-protection` capability in `subx-core`.

## Requirements

### Requirement: Mask sensitive config values in CLI output

The `config set`, `config list`, and `config get` subcommands SHALL mask the values of sensitive keys in everything they write to stdout, by passing each key and value through the masking helper before display and never printing the raw value alongside the masked one.

Which keys are sensitive and what the masked form looks like — the `api_key` / `token` / `secret` case-insensitive match, the `****<last 4 chars>` form, and the `****` form for values of four characters or fewer — is specified by the `secrets-protection` capability's *Sensitive Value Masking Helper* requirement in `subx-core`, which owns `mask_sensitive_value` in `subx-core:src/config/masking.rs`. This requirement owns only the obligation that the display sites use it.

This capability and the `configuration-management` capability's *Sensitive value masking in config display* requirement have specified overlapping obligations over these same display sites since both were written. The overlap is preserved unchanged: both stay in `subx-cli`, and deciding which of the two owns config-display masking is left to a later change.

#### Scenario: config set echoes masked value

- **WHEN** user runs `config set ai.api_key "sk-abc123def456"`
- **THEN** stdout shows `****f456` not the full key

#### Scenario: config list masks api_key

- **WHEN** user runs `config list`
- **THEN** the `api_key` field displays `****<last4>` instead of the plaintext value
