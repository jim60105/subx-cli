## MODIFIED Requirements

### Requirement: Public format API stability across module reorganization

The system SHALL preserve every existing public path under
`crate::core::formats` (including `Subtitle`, `SubtitleEntry`,
`SubtitleMetadata`, `StylingInfo`, `SubtitleFormatType`,
`SubtitleFormat`, `SrtFormat`, `AssFormat`, `VttFormat`, `SubFormat`,
`FormatManager`, and `FormatConverter`) while internal modules are
reorganized. The full method signatures of the `SubtitleFormat` trait
(`parse`, `serialize`, `detect`, `format_name`, `file_extensions`,
`supports_styling`, `uses_frame_timing`) SHALL remain unchanged in
arity, parameter types, return types, and default-method semantics.
Downstream crates and other modules in `subx-cli` MUST continue to
compile without import path changes.

The trait's **supertrait list** is part of this frozen surface:

- `SubtitleFormat` SHALL declare `Send + Sync` as supertraits, so that
  `Box<dyn SubtitleFormat>`, `&dyn SubtitleFormat` and any container of
  either carry the auto traits without naming them at the storage site.
- Every implementor SHALL therefore be `Send + Sync`. The four
  registered implementors — `AssFormat`, `VttFormat`, `SrtFormat`,
  `SubFormat` — are field-less structs and satisfy this without
  synchronisation.
- An implementor SHALL NOT hold `Rc`, `RefCell`, `Cell`, a raw pointer,
  or any other non-thread-safe state in order to satisfy the trait. A
  format handler is a parser: it is stateless, or it holds
  configuration.
- Removing either supertrait SHALL be treated as a breaking change
  requiring a major version, on the same footing as changing a method
  signature. Adding a supertrait to an already-published trait is
  likewise a major change, and SHALL be sequenced accordingly.
- The obligation SHALL be stated in the trait's own rustdoc under
  `# Implementation Notes`, so an implementor learns it from the item
  rather than from this specification.

#### Scenario: Existing import paths still resolve

- **GIVEN** any pre-existing `use crate::core::formats::<Item>;`
  statement in the codebase or in published rustdoc examples
- **WHEN** the format module is reorganized into per-format submodules
- **THEN** the import SHALL still resolve and `cargo build`,
  `cargo clippy -- -D warnings`, and `cargo test --doc --all-features`
  SHALL pass

#### Scenario: FormatManager registration remains complete

- **GIVEN** a default `FormatManager::new()` instance after the refactor
- **WHEN** `detect_format` is called on a path with extension `srt`,
  `ass`, `ssa`, `vtt`, or `sub`
- **THEN** the manager SHALL return the corresponding
  `SubtitleFormatType` exactly as before the refactor

#### Scenario: The trait object is thread-safe without a storage-site bound

- **GIVEN** `FormatManager`'s registry field written as
  `Vec<Box<dyn SubtitleFormat>>`, with no `+ Send + Sync` at the
  storage site
- **WHEN** the enclosing type is required to be `Send + Sync`
- **THEN** the bound SHALL be satisfied by the trait's supertraits, and
  `get_format` and `get_format_by_extension` SHALL keep returning
  `Option<&dyn SubtitleFormat>` with no cast at the call site

#### Scenario: A non-thread-safe format handler is rejected

- **GIVEN** a proposed fifth `SubtitleFormat` implementor holding a
  `Rc<RefCell<_>>` parser cache
- **WHEN** it is registered in `FormatManager::new`
- **THEN** compilation SHALL fail at the registration site with the
  unsatisfied `Send`/`Sync` bound, and the resolution SHALL be to make
  the handler's state thread-safe or stateless — not to relax the
  trait
