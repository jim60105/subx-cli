## ADDED Requirements

### Requirement: Replace unmaintained md5 crate

The project SHALL replace the `md5` crate with `md-5` (RustCrypto maintained version) or an alternative maintained hash crate.

#### Scenario: cache hashing uses maintained crate

- **WHEN** a cache key hash is computed
- **THEN** the `md-5` crate (or equivalent maintained crate) is used instead of `md5`

### Requirement: Narrow dependency feature flags

The `tokio` dependency SHALL specify only the features actually used instead of `"full"`. The `symphonia` dependency SHALL specify only the codec features actually needed instead of `"all"`.

#### Scenario: tokio features are minimal

- **WHEN** Cargo.toml is inspected
- **THEN** `tokio` lists specific features (e.g., `rt-multi-thread`, `macros`, `fs`, `time`, `sync`) not `"full"`

#### Scenario: symphonia features are minimal

- **WHEN** Cargo.toml is inspected
- **THEN** `symphonia` lists specific codec features not `"all"`

### Requirement: CI cargo audit gate

The project's existing `cargo audit` CI step SHALL be verified to fail the build pipeline on any direct dependency with a known vulnerability advisory. If the current configuration allows advisory-only warnings without failing, it SHALL be tightened to enforce failure.

#### Scenario: vulnerable dependency fails CI

- **WHEN** a direct dependency has a RUSTSEC advisory
- **THEN** the CI pipeline fails

#### Scenario: clean dependencies pass CI

- **WHEN** no direct dependencies have advisories
- **THEN** the CI pipeline succeeds
