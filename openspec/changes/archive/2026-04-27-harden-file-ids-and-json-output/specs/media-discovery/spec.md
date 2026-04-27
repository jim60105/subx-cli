## MODIFIED Requirements

### Requirement: UUIDv7-Based File Identifier

The system SHALL generate a unique identifier for every discovered file via `generate_file_id(generator: &mut Uuidv7Generator)`, which SHALL produce a UUIDv7 value through the shared `crate::core::uuidv7::Uuidv7Generator` (defined in `src/core/uuidv7.rs`) and format the result as `file_<uuid-v7-hyphenated>`. The resulting identifier SHALL therefore be exactly 41 characters long, SHALL start with `file_`, SHALL be unique within a single discovery scan, and SHALL embed a `unix_time_ts` (the upper 48 bits of the UUIDv7 payload) that is strictly greater than the `unix_time_ts` of every previously generated identifier from the same generator instance. Strict monotonicity SHALL be enforced by the generator's 1ms-spacing contract: when two consecutive `next_id` calls would land in the same millisecond, the generator SHALL sleep until the next millisecond boundary before returning.

`FileDiscovery::scan_directory` and `FileDiscovery::scan_file_list` SHALL each instantiate a single `Uuidv7Generator` at the start of the scan and SHALL thread a `&mut` reference to that generator through every classification call so that all identifiers produced by a single scan share a strictly increasing `unix_time_ts`.

#### Scenario: Identifier shape and length

- **GIVEN** a freshly constructed `Uuidv7Generator`
- **WHEN** `generate_file_id(&mut generator)` is called once
- **THEN** the returned string SHALL match the pattern `file_xxxxxxxx-xxxx-7xxx-xxxx-xxxxxxxxxxxx` (where `x` is any lowercase hex digit) and SHALL have length exactly 41

#### Scenario: Same scan produces strictly increasing timestamps

- **GIVEN** a `FileDiscovery` instance and a directory containing several recognized media files
- **WHEN** `scan_directory(root, true)` returns the resulting `Vec<MediaFile>`
- **THEN** for every adjacent pair `(file[i], file[i+1])`, the UUIDv7 `unix_time_ts` extracted from `file[i+1].id` SHALL be strictly greater than the `unix_time_ts` extracted from `file[i].id`

#### Scenario: Identifier uniqueness across video and subtitle classifications

- **GIVEN** a directory containing both video and subtitle files
- **WHEN** `scan_directory(root, true)` is invoked
- **THEN** the set of `id` values across the returned `MediaFile` entries SHALL contain no duplicates

## REMOVED Requirements

### Requirement: Deterministic File Identifier

**Reason**: Replaced by the UUIDv7-based identifier requirement. The
deterministic `(canonicalized path, file size) → DefaultHasher` mapping
is removed because it is opaque, collision-prone, not time-sortable, and
not aligned with the UUIDv7 scheme already used by the
`subtitle-translation` capability and now adopted across discovery and
parallel processing.

**Migration**: External scripts that previously matched the
`file_<16 hex chars>` pattern (length 21) in stderr output SHALL update
their pattern to `file_<uuid-v7-hyphenated>` (length 41). The
identifier was never persisted across invocations and is not part of
the JSON envelope, so machine-readable consumers are unaffected.
