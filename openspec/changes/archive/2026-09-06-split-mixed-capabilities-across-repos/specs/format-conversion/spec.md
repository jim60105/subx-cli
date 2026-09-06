## REMOVED Requirements

### Requirement: File size check before parsing

**Reason**: Core half of the split. The `general.max_subtitle_bytes` check is enforced under `src/core/formats/` before a subtitle file is read (see the note at `src/core/formats/srt/parser.rs:19`), which is `subx-core` after B2. Re-added verbatim by `import-split-capability-specs`, at `openspec/specs/format-conversion/spec.md` in that repository. It leaves this repository's half of the capability; it does not leave the project.

### Requirement: Parser robustness on malformed input

**Reason**: Core half of the split. All four parsers are under `src/core/formats/`. Re-added verbatim in `subx-core` by `import-split-capability-specs`.

### Requirement: Parser/serializer round-trip stability

**Reason**: Core half of the split. The parsers and serializers are under `src/core/formats/`, and B3 moves `tests/fixtures/formats/**` into `subx-core` at the identical relative path with its `.gitattributes` `-text` rule, so the byte-exact fixtures survive. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: Every `tests/fixtures/formats/<format>/` citation is correct verbatim inside `subx-core` under `spec-governance`'s identity rule and SHALL NOT be rewritten. The requirement's phrase "across this refactor" refers to the format-module reorganization that predates the crate split and SHALL be carried over unchanged; it is not a reference to this change.

### Requirement: Public format API stability across module reorganization

**Reason**: Core half of the split. Every public path the requirement enumerates is under `crate::core::formats`, which after B2 is `subx_core::core::formats`. Re-added in `subx-core` by `import-split-capability-specs`.

**Migration**: The sentence "Downstream crates and other modules in `subx-cli` MUST continue to compile without import path changes" is the one clause that does not survive verbatim. Inside `subx-core` the paths are `crate::core::formats::<Item>` and are correct as written; the `subx-cli` guarantee is now provided by the D11 re-exports in `subx-cli`'s `lib.rs`. On arrival the sentence SHALL be re-phrased so that the obligation is on `subx-core`'s own public paths, with the `subx-cli` compatibility named as a consequence of the re-export surface specified by the `crate-topology` capability in `subx-cli`. The scenario's `cargo build` / `cargo clippy` / `cargo test --doc --all-features` commands SHALL be read as `subx-core`'s own, per C1 Decision 4.

## MODIFIED Requirements

### Requirement: Supported Output Formats

The system SHALL accept `--format` values `srt`, `ass`, `vtt`, and `sub`, defined by the `OutputSubtitleFormat` enum in `src/cli/convert_args.rs`, and SHALL write output files with the file extension matching the selected value. When `--format` is omitted the command SHALL resolve the target format from `formats.default_output` in configuration.

`OutputSubtitleFormat` is a clap-derived enum and stays in `subx-cli` permanently under SDR D8, so the accepted value set and the extension mapping are CLI-owned. What each target format's output must look like — that an SRT-to-VTT conversion produces a `WEBVTT` header and dot timecodes, and equivalently for the other targets — is specified by the `format-conversion` capability's *Target Format Conversion Semantics* requirement in `subx-core`.

#### Scenario: Default output format from configuration
- **GIVEN** the user omits `--format` and `formats.default_output` is `srt` in configuration
- **WHEN** the command runs
- **THEN** every input file SHALL be converted to SRT
