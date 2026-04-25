# Archive Extraction

## Purpose

Transparently extract `.zip` and `.rar` archive files directly supplied as `-i` inputs so that downstream commands process the contained files without requiring manual extraction. Implements format detection, secure extraction with path-traversal prevention, temporary directory lifecycle management, and decompression bomb protection. Implemented in `src/core/archive.rs`.

## Requirements

### Requirement: Zip Archive Extraction
The system SHALL extract the contents of `.zip` archive files using the
`zip` crate. Extraction SHALL preserve the archive-internal directory
structure within the temporary extraction directory. All extracted entries
SHALL have their paths validated against path-traversal attacks: any entry
whose resolved path falls outside the extraction root SHALL be skipped and
a warning SHALL be logged.

#### Scenario: Valid zip archive extracted
- **WHEN** `extract_archive` is called with a valid `.zip` file
- **THEN** all non-directory entries SHALL be extracted to the temp
  directory, preserving internal folder structure, and the function SHALL
  return the list of extracted file paths

#### Scenario: Zip entry with path traversal is rejected
- **WHEN** a `.zip` archive contains an entry with path `../../etc/passwd`
- **THEN** that entry SHALL be skipped, a warning SHALL be logged, and
  extraction SHALL continue for remaining entries

#### Scenario: Symlink entry in zip is rejected
- **WHEN** a `.zip` archive contains a symbolic link entry
- **THEN** that entry SHALL be skipped, a warning SHALL be logged, and
  extraction SHALL continue for remaining entries

#### Scenario: Empty zip archive
- **WHEN** `extract_archive` is called with a `.zip` that contains no
  file entries
- **THEN** the function SHALL return an empty path list and no error

### Requirement: RAR Archive Extraction
The system SHALL extract the contents of `.rar` archive files using the
`unrar` crate, which statically compiles the UnRAR C++ library via
`unrar_sys` — no runtime library dependency is required. Extraction SHALL
preserve internal directory structure. Path-traversal validation SHALL
apply identically to zip extraction, including rejection of entries
containing parent-directory components or absolute paths.
RAR support SHALL be gated behind a cargo feature flag `archive-rar`.
The feature is **enabled in release builds** so published binaries include
RAR support out of the box. It remains opt-in for development builds.

#### Scenario: Valid rar archive extracted
- **WHEN** `extract_archive` is called with a valid `.rar` file
- **THEN** all entries SHALL be extracted preserving folder structure and
  the function SHALL return the list of extracted file paths

#### Scenario: RAR feature disabled
- **WHEN** the binary is compiled without the `archive-rar` feature and a
  `.rar` file is supplied as input
- **THEN** the system SHALL log a warning indicating RAR support is not
  compiled in and SHALL skip the file

### Requirement: Format Detection by Extension
The system SHALL detect archive format by file extension (case-insensitive):
`.zip` → Zip format, `.rar` → RAR format. No magic-byte sniffing SHALL be
performed.

#### Scenario: Case-insensitive extension matching
- **WHEN** a file named `Subtitles.ZIP` is supplied as input
- **THEN** the system SHALL recognise it as a zip archive

#### Scenario: Unknown extension is not treated as archive
- **WHEN** a file named `data.tar.gz` is supplied as input
- **THEN** the system SHALL NOT attempt extraction and SHALL pass it
  through to normal file processing

### Requirement: Temporary Directory Lifecycle
The system SHALL create one `TempDir` per extracted archive. The temp
directory SHALL be automatically deleted when the owning
`CollectedFiles` struct is dropped. The temp directory SHALL be created
in the system's default temp location (`std::env::temp_dir()`).

#### Scenario: Temp dir cleaned up after command completes
- **WHEN** a command extracts an archive and then completes (success or
  error)
- **THEN** the temporary extraction directory SHALL no longer exist on
  disk

#### Scenario: Multiple archives each get their own temp dir
- **WHEN** two archive files are supplied as input
- **THEN** each SHALL be extracted to a separate `TempDir`

### Requirement: No Nested Archive Extraction
Archives found inside extracted archives SHALL NOT be extracted. They SHALL
be treated as regular files and subject to the normal extension filter
(which will skip them since `.zip`/`.rar` are not media extensions).

#### Scenario: Archive inside archive is not extracted
- **WHEN** an outer `.zip` contains an inner `subs.zip` which in turn
  contains `movie.srt`
- **THEN** `movie.srt` SHALL NOT appear in the collected files; `subs.zip`
  SHALL be skipped by the extension filter

### Requirement: Direct-Input-Only Extraction
Only archive files directly specified via `-i` (or positional arguments)
SHALL be eligible for extraction. Archives discovered during directory
traversal (when `-i` points to a directory containing archive files) SHALL
NOT be extracted — they SHALL be treated as regular files and filtered by
extension.

#### Scenario: Archive in traversed directory is not extracted
- **GIVEN** the user runs `subx match -i /media/` and `/media/` contains
  `subs.zip` alongside `movie.mp4`
- **WHEN** `collect_files()` runs
- **THEN** `subs.zip` SHALL NOT be extracted; it SHALL be skipped by the
  extension filter since `.zip` is not a media extension

#### Scenario: Directly specified archive is extracted
- **GIVEN** the user runs `subx match -i /media/subs.zip`
- **WHEN** `collect_files()` runs
- **THEN** `subs.zip` SHALL be extracted and its contents SHALL be
  processed

### Requirement: Error Handling for Corrupt or Protected Archives
The system SHALL NOT abort the command when an archive fails to extract.
Instead, it SHALL log a warning identifying the archive file and the
error reason, then continue processing remaining inputs.

#### Scenario: Corrupted archive is skipped
- **WHEN** a corrupted `.zip` file is among the inputs
- **THEN** the system SHALL log a warning for that file and SHALL
  continue processing other inputs

#### Scenario: Password-protected archive is skipped
- **WHEN** a password-protected `.rar` file is among the inputs
- **THEN** the system SHALL log a warning indicating password protection
  and SHALL continue processing other inputs

#### Scenario: Password-protected zip archive is skipped
- **WHEN** a password-protected `.zip` file is among the inputs
- **THEN** the system SHALL log a warning indicating password protection
  and SHALL continue processing other inputs

### Requirement: Decompression Bomb Protection
The system SHALL enforce a maximum total expanded size (default 1 GiB) and
a maximum entry count (default 10,000 files) per archive. If either limit
is exceeded during extraction, the system SHALL abort extraction of that
archive, log a warning, and skip it.

#### Scenario: Archive exceeding size limit
- **WHEN** a `.zip` archive's extracted content would exceed 1 GiB
- **THEN** extraction SHALL stop, the partial temp dir SHALL be cleaned
  up, and the archive SHALL be skipped with a warning

#### Scenario: Archive exceeding entry count limit
- **WHEN** a `.zip` archive contains more than 10,000 entries
- **THEN** extraction SHALL stop and the archive SHALL be skipped with a
  warning
