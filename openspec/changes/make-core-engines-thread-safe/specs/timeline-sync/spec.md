## ADDED Requirements

### Requirement: VAD-Independent Manual Offset Application

The manual-offset timing transform SHALL be reachable without constructing a `SyncEngine`, so that a caller who never performs detection is not subject to the engine's VAD precondition.

- `subx-core` SHALL expose `shift_subtitle_timing(subtitle: &mut Subtitle, offset_seconds: f32) -> Result<SyncResult>` as a free function in `subx_core::core::sync` (`subx-core/src/core/sync/mod.rs`), alongside `resolve_sync_pairing` and `create_default_output_path`.
- The function SHALL apply the offset to every entry with the semantics defined by *Subtitle Timing Application* and SHALL return the `SyncResult` defined there.
- The function SHALL NOT read `sync.max_offset_seconds` and SHALL NOT require, construct, or consult a VAD detector, an audio processor, a `SyncConfig`, or a `ConfigService`. Its only inputs are the subtitle and the offset.
- `SyncEngine::apply_manual_offset` SHALL delegate to this function after performing its own guard, so that exactly one implementation of the transform exists. The two entry points SHALL produce identical `SyncResult` field values and identical entry timings for any input the guard admits.
- The function's rustdoc SHALL state that `sync.max_offset_seconds` is not enforced and SHALL name `SyncEngine::apply_manual_offset` as the entry point that enforces it, so a caller choosing between them is told which obligation it is taking on.

#### Scenario: A caller with no VAD available applies a manual offset
- **GIVEN** a host with `sync.vad.enabled = false`, for which `SyncEngine::new` returns a configuration error
- **WHEN** the host calls `shift_subtitle_timing` with a parsed subtitle and an offset
- **THEN** the offset SHALL be applied and a `SyncResult` with `method_used = SyncMethod::Manual` SHALL be returned, with no engine constructed and no VAD detector involved

#### Scenario: The free function does not enforce the configured maximum
- **GIVEN** `sync.max_offset_seconds = 60` in configuration and a call to `shift_subtitle_timing` with an offset of 120 seconds
- **WHEN** the function runs
- **THEN** it SHALL apply the 120-second offset and SHALL NOT return a configuration error, because the guard belongs to `SyncEngine::apply_manual_offset` and the function is documented as unguarded

#### Scenario: The two entry points cannot drift
- **GIVEN** a subtitle and an offset within `sync.max_offset_seconds`
- **WHEN** the same input is passed to `SyncEngine::apply_manual_offset` and to `shift_subtitle_timing`
- **THEN** the resulting entry timings SHALL be identical and the returned `SyncResult`'s `offset_seconds`, `confidence`, `method_used`, `correlation_peak`, `additional_info` and `warnings` SHALL be identical

## MODIFIED Requirements

### Requirement: Sync Method Selection

The system SHALL support two sync methods selected via `--method`: `vad` (local Voice Activity Detection) and `manual` (user-supplied offset). When `--method` is omitted the engine SHALL fall back to the method declared by `sync.default_method` in configuration.

`SyncEngine`'s VAD precondition SHALL apply to engine construction only, and SHALL NOT be the gate on the manual-offset transform:

- `SyncEngine::new` SHALL retain its current signature and its current behaviour, including the unconditional VAD requirement. Relaxing it is a separate change with the command surface on the other side of it.
- A caller that needs only the manual-offset transform SHALL use `shift_subtitle_timing` (see *VAD-Independent Manual Offset Application*) rather than constructing an engine, and SHALL NOT reimplement the transform.
- `SyncEngine::new`'s rustdoc SHALL state that the VAD requirement is unconditional and SHALL name the free function as the entry point for manual-offset-only callers.

#### Scenario: Manual mode requires an explicit offset
- **GIVEN** the user passes `--method manual` without `--offset`
- **WHEN** argument validation runs
- **THEN** validation SHALL fail with the message `Manual method requires --offset parameter.`

#### Scenario: VAD detector is required unconditionally
- **GIVEN** VAD is disabled in configuration or the VAD detector fails to initialize
- **WHEN** `SyncEngine::new` is called
- **THEN** engine construction SHALL unconditionally return a configuration error stating that the VAD detector is required but unavailable, regardless of which sync method the user ultimately selects

#### Scenario: The construction precondition does not reach the manual transform
- **GIVEN** the same configuration on which `SyncEngine::new` returns the configuration error above
- **WHEN** a manual offset is applied through `shift_subtitle_timing`
- **THEN** the transform SHALL succeed, and no caller SHALL be required to duplicate the transform in order to reach it

### Requirement: Offset Clamping Against Maximum

The system SHALL enforce `sync.max_offset_seconds`: manual offsets exceeding this absolute value SHALL be rejected with an error, and VAD-detected offsets exceeding it SHALL be clamped (preserving sign) and accompanied by a warning in the sync result.

The guard's location is normative, because the transform it guards is reachable independently:

- The manual-offset guard SHALL live on `SyncEngine::apply_manual_offset`, which reads `self.config.max_offset_seconds` and returns the configuration error before any entry is modified.
- The guard SHALL NOT be moved into `shift_subtitle_timing`. That function is specified as unguarded (*VAD-Independent Manual Offset Application*) so that a host which validates the offset against its own presentation of `sync.max_offset_seconds` — for example in whole milliseconds, to keep its own pre-check and the library's check on the same side of a rounding boundary — is not subjected to a second, differently-rounded check.
- A caller reaching the transform directly SHALL be responsible for its own bound. This division SHALL be stated in both items' rustdoc.

#### Scenario: Manual offset exceeds maximum
- **GIVEN** `sync.max_offset_seconds = 60` and the user supplies `--offset 120`
- **WHEN** `apply_manual_offset` runs
- **THEN** the call SHALL return a configuration error referencing `sync.max_offset_seconds` and the subtitle entries SHALL remain unchanged

#### Scenario: VAD offset clamping
- **GIVEN** `sync.max_offset_seconds = 30` and VAD detects an offset of 45s
- **WHEN** `vad_detect_sync_offset` returns
- **THEN** the resulting `SyncResult.offset_seconds` SHALL equal 30 (sign preserved), `SyncResult.warnings` SHALL contain a message explaining the clamping, and `additional_info` SHALL record the original and clamped values

#### Scenario: The guard is not duplicated into the free function
- **GIVEN** a proposal to add the `sync.max_offset_seconds` check to `shift_subtitle_timing`
- **WHEN** it is evaluated against this requirement
- **THEN** it SHALL be rejected, because the free function has no `SyncConfig` and adding one would restore the construction-time configuration dependency the function exists to avoid

### Requirement: Subtitle Timing Application

The system SHALL shift every subtitle entry's start and end time by the applied offset, clamping negative results to zero rather than producing negative timestamps.

The transform SHALL have exactly one implementation:

- The shift SHALL be implemented once, in `shift_subtitle_timing` (`subx-core/src/core/sync/mod.rs`). A positive offset SHALL use a checked addition and SHALL return an audio-processing error if any entry's time would overflow; a negative offset SHALL saturate at `Duration::ZERO` rather than erroring.
- The returned `SyncResult` SHALL carry `offset_seconds` as supplied, `confidence = 1.0`, `method_used = SyncMethod::Manual`, `correlation_peak = 1.0`, an `additional_info` object recording the applied offset and the number of entries modified, and the measured processing duration.
- `SyncEngine::apply_manual_offset` SHALL obtain both the shift and the `SyncResult` from that function and SHALL NOT construct either itself.

#### Scenario: Positive offset delays subtitles
- **GIVEN** a subtitle entry with `start_time = 10s` and the engine applies a +2.5s offset
- **WHEN** `apply_manual_offset` runs
- **THEN** the entry's new `start_time` SHALL be 12.5s

#### Scenario: Negative offset clamps to zero
- **GIVEN** a subtitle entry with `start_time = 1s` and the engine applies a -5s offset within the maximum
- **WHEN** `apply_manual_offset` runs
- **THEN** the entry's new `start_time` SHALL be `Duration::ZERO` rather than a negative value

#### Scenario: Positive offset beyond the representable range is rejected
- **GIVEN** a subtitle entry whose `end_time` is `Duration::MAX` and a positive offset within `sync.max_offset_seconds`
- **WHEN** the shift is applied through either entry point
- **THEN** an audio-processing error SHALL be returned rather than a wrapped or truncated timestamp
