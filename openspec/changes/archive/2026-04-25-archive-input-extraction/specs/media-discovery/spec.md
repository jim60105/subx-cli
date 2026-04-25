## ADDED Requirements

### Requirement: Archive Extensions Excluded from Media Classification

`FileDiscovery` SHALL NOT include `.zip` or `.rar` in its recognized
video or subtitle extension sets. Archive files SHALL be intercepted by
`InputPathHandler` before reaching `FileDiscovery`. If an archive file
somehow reaches `scan_file_list` or `scan_directory`, it SHALL be silently
skipped like any other unrecognized extension.

#### Scenario: Zip file passed to scan_file_list is skipped
- **WHEN** `scan_file_list` receives a path list containing `subs.zip`
- **THEN** the returned `Vec<MediaFile>` SHALL NOT contain an entry for
  `subs.zip`

### Requirement: Temp Directory Paths Accepted

`FileDiscovery::scan_file_list` and `scan_directory` SHALL accept paths
rooted in the system temp directory without special handling. No path
validation SHALL reject files solely because they reside under
`std::env::temp_dir()`.

#### Scenario: Extracted files in temp dir are discovered
- **GIVEN** subtitle files extracted from an archive into `/tmp/subx-XXXX/`
- **WHEN** `scan_file_list` is called with those paths
- **THEN** each file with a recognized extension SHALL be returned as a
  `MediaFile` with correct classification and metadata
