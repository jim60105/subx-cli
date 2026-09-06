## MODIFIED Requirements

### Requirement: Replace unmaintained md5 crate

No manifest this project publishes SHALL declare the unmaintained `md5` crate. Where a maintained hash implementation is needed, `md-5` (the RustCrypto-maintained crate) or an equivalent maintained crate SHALL be used, and a hash computed from standard-library facilities SHALL be treated as satisfying this requirement as well.

This rule applies **per manifest**, in the same sense as the "Every Declared Dependency Has a Use Site" requirement: once the project spans more than one crate, each crate's manifest SHALL satisfy it independently. Neither the superproject's manifest nor a submodule member's is exempt, and a re-introduction in either is a violation.

At the time of writing no manifest declares `md5` or `md-5`. The cache-key hashing this requirement was originally written about is computed with `std::collections::hash_map::DefaultHasher` at `subx-core:src/services/ai/cache.rs:177` and in `subx-core:src/core/matcher/engine.rs`, both of which are in the library crate. The scenario below is retained so that the rule has a referent if a hash crate is ever added back.

#### Scenario: cache hashing uses maintained crate

- **WHEN** a cache key hash is computed in `subx-core`
- **THEN** it SHALL use `std`'s hashing facilities or the `md-5` crate (or an equivalent maintained crate), and SHALL NOT use `md5`

#### Scenario: neither manifest re-introduces the unmaintained crate

- **WHEN** the manifests of every crate the project publishes are inspected
- **THEN** none SHALL declare `md5` in `[dependencies]`, `[dev-dependencies]`, or any `[target.'cfg(…)'.dependencies]` table

### Requirement: Narrow dependency feature flags

Every dependency declaration SHALL enable only the features its declaring crate actually uses. Aggregate feature values that pull in a whole crate — `tokio`'s `"full"`, `symphonia`'s `"all"` — SHALL NOT be used.

This rule applies **per manifest**, and after the crate split the two manifests are constrained separately rather than jointly:

- `tokio` is declared in both. Each declaration SHALL list only the features its own crate's use sites require, and the two lists SHALL be allowed to differ — the library needs the runtime, synchronisation, timer, filesystem and macro features; the binary needs the multi-threaded runtime, the macros, and the timer. A feature enabled in one manifest SHALL NOT be treated as justification for enabling it in the other, because Cargo's feature unification means an over-broad declaration in either widens the graph for both.
- `symphonia` is declared only by the library crate, which is the only one with audio-decoding use sites. Its declaration SHALL list specific codec features. The superproject's manifest SHALL NOT declare it at all.

#### Scenario: tokio features are minimal in every manifest

- **WHEN** each crate's `Cargo.toml` is inspected
- **THEN** every `tokio` declaration SHALL list specific features and SHALL NOT list `"full"`, and each listed feature SHALL have a use site in that crate's own source trees

#### Scenario: symphonia features are minimal and declared once

- **WHEN** the manifests are inspected
- **THEN** `symphonia` SHALL appear in exactly one of them, SHALL list specific codec features, and SHALL NOT list `"all"`
