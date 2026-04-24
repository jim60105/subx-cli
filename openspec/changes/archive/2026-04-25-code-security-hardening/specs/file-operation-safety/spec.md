## ADDED Requirements

### Requirement: Atomic file creation in conflict resolution

The system SHALL use atomic file creation (`OpenOptions::create_new(true)`) instead of `exists()` + `create()` when resolving filename conflicts. For copy and backup operations, the system SHALL write content through the **same opened file handle** that reserved the path — never close and reopen. For rename/move operations, the system SHALL use a copy-to-new-file-then-delete-source pattern: open destination atomically, copy content through that handle, fsync, close, then delete source. If `AlreadyExists` is returned, the system SHALL retry with an incremented numeric suffix up to 999 attempts.

#### Scenario: concurrent writes get unique names

- **WHEN** two parallel tasks attempt to write to the same target path simultaneously
- **THEN** each task creates a uniquely-suffixed file and neither overwrites the other

#### Scenario: normal non-conflicting write succeeds

- **WHEN** the target path does not exist
- **THEN** the file is created atomically on the first attempt

#### Scenario: copy writes through reserved handle

- **WHEN** a copy operation atomically creates the destination file
- **THEN** content is written through the same file handle without closing and reopening

#### Scenario: rename uses copy-then-delete

- **WHEN** a rename/move operation is performed
- **THEN** the system atomically creates the destination, copies content, fsyncs, and deletes the source

### Requirement: Symlink detection in directory scanning

The recursive directory scanner SHALL use `DirEntry::file_type()` (which does not follow symlinks) instead of `Path::is_file()`/`Path::is_dir()` to classify entries. Symlinked entries SHALL be skipped with a debug-level log message.

#### Scenario: symlink to external directory is skipped

- **WHEN** a scanned directory contains a symlink pointing to `/etc`
- **THEN** the symlink is not followed and its target contents are not processed

#### Scenario: regular files are processed normally

- **WHEN** a scanned directory contains regular files
- **THEN** they are processed as before

### Requirement: Symlink protection on write targets with parent-chain validation

Before writing any output file, the system SHALL validate the **entire parent directory chain** of the target path — not just the leaf. The system SHALL canonicalize the parent directory and verify it is under the expected output directory. If any component in the parent chain is a symlink that escapes the expected directory, the operation SHALL return an error. As defense-in-depth on Unix, `O_NOFOLLOW` SHALL be applied via `OpenOptionsExt` where available.

#### Scenario: write to symlink target is rejected

- **WHEN** the output path is a symlink
- **THEN** the system returns an error instead of following the symlink

#### Scenario: write through symlinked parent is rejected

- **WHEN** a parent directory in the output path is a symlink pointing outside the expected output directory
- **THEN** the system returns an error

#### Scenario: write to regular path succeeds

- **WHEN** the output path and all parent directories are regular (not symlinks)
- **THEN** the write proceeds normally

### Requirement: Safe rename operations

The `execute_rename_file_operation` function SHALL check for filename conflicts before renaming, using the same atomic resolution logic as copy and move operations. Silent overwrite on rename SHALL NOT occur.

#### Scenario: rename with existing target gets suffix

- **WHEN** renaming and the target file already exists
- **THEN** the system uses atomic conflict resolution to generate a unique name
