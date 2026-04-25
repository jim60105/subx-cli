## MODIFIED Requirements

### Requirement: Archive Extensions Excluded from Media Classification

`FileDiscovery` SHALL NOT include `.zip`, `.rar`, `.7z`, `.tar.gz`, or
`.tgz` in its recognized video or subtitle extension sets. Archive files
SHALL be intercepted by `InputPathHandler` before reaching `FileDiscovery`.
If an archive file somehow reaches `scan_file_list` or `scan_directory`,
it SHALL be silently skipped like any other unrecognized extension.

#### Scenario: Zip file passed to scan_file_list is skipped
- **WHEN** `scan_file_list` receives a path list containing `subs.zip`
- **THEN** the returned `Vec<MediaFile>` SHALL NOT contain an entry for
  `subs.zip`

#### Scenario: 7z file passed to scan_file_list is skipped
- **WHEN** `scan_file_list` receives a path list containing `subs.7z`
- **THEN** the returned `Vec<MediaFile>` SHALL NOT contain an entry for
  `subs.7z`

#### Scenario: Tar.gz file passed to scan_file_list is skipped
- **WHEN** `scan_file_list` receives a path list containing `subs.tar.gz`
- **THEN** the returned `Vec<MediaFile>` SHALL NOT contain an entry for
  `subs.tar.gz`
