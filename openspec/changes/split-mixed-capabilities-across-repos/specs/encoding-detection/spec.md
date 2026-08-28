## REMOVED Requirements

### Requirement: Low-Confidence Fallback To Default Encoding

**Reason**: Core half of the split. The requirement already names "Implemented in `src/core/formats/encoding/detector.rs::select_best_encoding`", which is `subx-core` after B2, and it is the only place the detector's numeric contract — the `0.5` and `0.1` fallback confidences and the two exact sample-text prefixes — is written down. Re-added verbatim by `import-split-capability-specs`, at `openspec/specs/encoding-detection/spec.md` in that repository. It leaves this repository's half of the capability; it does not leave the project.

## MODIFIED Requirements

### Requirement: Robust Handling of Empty and Binary Files

The `detect-encoding` command SHALL complete for each supplied file without terminating the whole batch when the file is empty or contains binary (non-text) bytes; it SHALL either emit a normal detection report for the file or surface a per-file error while still processing subsequent inputs, and it SHALL exit successfully when at least one input was processed.

The detector's own obligation not to panic on empty or binary input is specified by the `encoding-detection` capability's *Detector Tolerates Empty and Binary Input* requirement in `subx-core`. This requirement governs only the batch loop's resilience and the process exit status; the two are separable because a detector that returns an error rather than panicking still leaves the command free to abort the batch, which is what this requirement forbids.

#### Scenario: Empty file
- **GIVEN** a zero-byte subtitle file supplied to `subx detect-encoding`
- **WHEN** the command runs
- **THEN** the command SHALL not panic and SHALL exit successfully after recording a per-file outcome

#### Scenario: Binary file
- **GIVEN** a file containing binary (non-text) bytes supplied to `subx detect-encoding`
- **WHEN** the command runs
- **THEN** the command SHALL not panic and SHALL exit successfully, emitting either a best-effort detection result or a per-file error message without aborting subsequent inputs
