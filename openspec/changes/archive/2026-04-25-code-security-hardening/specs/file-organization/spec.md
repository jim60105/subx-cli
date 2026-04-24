# File Organization (Delta)

## MODIFIED Requirements

### Requirement: Filename conflict resolution uses atomic creation

The filename conflict resolution logic SHALL use atomic file creation (`OpenOptions::create_new(true)`) to test and claim a filename in a single operation, eliminating the TOCTOU race between `exists()` and file creation. On `AlreadyExists` error, the system SHALL retry with an incremented numeric suffix up to 999 attempts. If all 999 suffix slots are taken, the system SHALL return an error rather than silently overwriting or looping indefinitely.

#### Scenario: atomic conflict resolution
- **WHEN** two parallel operations target the same output path
- **THEN** each SHALL atomically claim a unique filename and neither SHALL silently overwrite the other

#### Scenario: sequential conflict resolution
- **WHEN** a target file already exists and the system retries with suffixes
- **THEN** each suffix attempt SHALL use atomic creation to prevent races

#### Scenario: suffix exhaustion
- **WHEN** all 999 suffix slots are taken
- **THEN** the system SHALL return an error

## ADDED Requirements

### Requirement: Rename operation conflict check

The rename file operation SHALL check for filename conflicts before renaming, using the same atomic resolution logic as copy and move operations. Silent overwrite on rename SHALL NOT occur.

#### Scenario: rename to existing file gets suffix
- **WHEN** renaming a file and the target already exists
- **THEN** the system SHALL use atomic conflict resolution to generate a unique name
