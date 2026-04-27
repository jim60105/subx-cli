## MODIFIED Requirements

### Requirement: Per-Scan Unique UUIDv7 File Identifiers for Matching

The system SHALL assign each discovered media file a per-scan unique identifier of the form `file_<uuid-v7-hyphenated>` (total length 41) generated through the shared `crate::core::uuidv7::Uuidv7Generator` with strict 1ms spacing. Identifiers SHALL be unique within a single discovery scan but SHALL NOT be guaranteed to remain stable across separate invocations of the binary, because UUIDv7 IDs are intrinsically time-based. The match pipeline SHALL reference video and subtitle files by these identifiers (rather than by filename) when sending requests to the AI provider and when correlating the AI response back to disk paths within the same invocation. Cross-invocation correlation (e.g., between the match cache and a later `cache apply`) SHALL use canonical filesystem paths rather than identifiers. Implemented in `src/core/matcher/discovery.rs` (`generate_file_id`, `Uuidv7Generator` integration) and exercised by `tests/match_engine_id_integration_tests.rs`.

#### Scenario: All discovered files receive unique IDs

- **GIVEN** a directory containing several video and subtitle files, including entries with complex non-ASCII filenames
- **WHEN** `FileDiscovery::scan_directory` runs
- **THEN** every returned file SHALL have a non-empty `id` beginning with `file_` and of length 41, the embedded UUIDv7 version nibble SHALL equal `7`, and the full set of IDs SHALL be unique

#### Scenario: AI response is correlated via IDs within the same invocation

- **GIVEN** an AI provider returns `MatchResult.matches` entries referencing `video_file_id` and `subtitle_file_id` shaped as `file_<uuid-v7>`
- **WHEN** `MatchEngine::match_file_list` processes the response
- **THEN** the generated `MatchOperation` set SHALL resolve each ID back to the corresponding `MediaFile` and SHALL produce operations whose `video_file.id` and `subtitle_file.id` match the AI-supplied identifiers

#### Scenario: IDs are not stable across invocations

- **GIVEN** the same directory is scanned in two separate invocations of the binary
- **WHEN** `FileDiscovery::scan_directory` runs in each invocation
- **THEN** the returned `MediaFile.id` values for the same on-disk file MAY differ between the two invocations, AND any cross-invocation correlation SHALL be performed by canonical path rather than by ID

## REMOVED Requirements

### Requirement: Stable File Identifiers for Matching

**Reason**: The previous requirement guaranteed cross-invocation
stable identifiers of the form `file_<16 hex chars>` (length 21).
That guarantee is dropped in favor of per-scan unique UUIDv7
identifiers (length 41). UUIDv7 is intrinsically time-based and not
deterministic across invocations.

**Migration**: The new "Per-Scan Unique UUIDv7 File Identifiers for
Matching" requirement replaces this one. Code or tests that relied on
re-deriving the same `file_<hex>` identifier across invocations
SHALL be migrated to compare media files by canonical filesystem
path. The match cache and `cache apply` flows already key by path,
so no production code change is required outside the matcher itself.

