---
mode: agent
description: "This agent is designed to automate the process of version bumping in a Rust project, following best practices for semantic versioning and changelog management. It will extract the current version, gather commit messages, format them into a changelog, increment the version number appropriately, and create a new Git commit with a tag for the release."
---
# Bump Version Agent

1. Extract the current version number from the `Cargo.toml` manifest file to establish a baseline for the upcoming versioning operation.

2. Retrieve all commit messages with `--pretty=format:'%H%n%s%n%b%n----END----'` from the Git history that have occurred since the last git tag, denoted by the pattern `v<current_version_number>`, up to the present `HEAD` state.

3. Aggregate and format the collected commit messages into a structured changelog entry, adhering strictly to the [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) specification to ensure consistency, readability, and long-term maintainability in English. NEVER REMOVE OLD CHANGELOG ENTRIES. NEVER REMOVE OLD CHANGELOG ENTRIES. NEVER REMOVE OLD CHANGELOG ENTRIES.

4. Programmatically increment the version number following [Semantic Versioning](https://semver.org/) conventions. The decision to increment the MAJOR, MINOR, or PATCH component should be based on the nature of the changes:

  * **MAJOR** for incompatible API changes,
  * **MINOR** for backward-compatible functionality additions,
  * **PATCH** for backward-compatible bug fixes.

5. Modify the `Cargo.toml` manifest to reflect the new version number by explicitly updating the `version` field, thereby establishing the canonical source of truth for the crate's version metadata.

6. Compile the project using the Cargo build system to propagate the updated version number into all associated lockfiles (`Cargo.lock`) and build artifacts, ensuring consistency across the dependency resolution graph and facilitating reproducible builds.

7. Git add `-A` and create a new Git commit in English encapsulating the version bump and changelog modifications, ensuring atomicity and traceability of the release change. Use here document syntax for the command so that the commit message can be multi-line and formatted properly.

8. Annotate the newly created commit with a Git tag in the format `v<new_version_number>` to denote the release point in version control history.

**Do not execute `git push` at this stage**, as the final verification and remote publishing will be performed manually to allow for pre-release inspection.

# Reference: changelog best practices

## 📝 1. Structure Your Changelog Properly

* **Use `CHANGELOG.md`**, with entries in **reverse-chronological** order (newest at the top).
* Begin with an **`Unreleased`** section to record ongoing development, then move its contents under a version heading during releases.

---

## 2. Version Headings & Dates

* Each release should start with a standard header:

  ```
  ## [1.2.3] - 2025-06-09
  ```

  * Follow the semantic versioning scheme.
  * Date in **ISO‑8601 format** (`YYYY‑MM‑DD`) for clarity and consistency.

---

## 3. Categorize Changes

Group your bullet points under these standard categories:

| Category       | Purpose                                             |
| -------------- | --------------------------------------------------- |
| **Added**      | Introduce new features                              |
| **Changed**    | Describe modifications to existing functionality    |
| **Deprecated** | Note features slated for removal in future releases |
| **Removed**    | Announce removed or cleaned‑up features             |
| **Fixed**      | Specify bug fixes                                   |
| **Security**   | Highlight security-related updates                  |

Use categories **only if applicable**—omit empty sections to reduce clutter.

---

## 4. Use Human‑Readable, Concise Bullets

* Write from the **user's perspective**, not a commit log.
* Focus on **“what changed and why it matters”**.
* Example for a feature addition:

  * ✅ `- Added: Dark-mode toggle in the settings panel (requested by many users).`

Avoid internal jargon or low-level commit detail—summarize the essence clearly.

---

## 5. Linkable References

* Place **Markdown reference-style links** at the bottom for versions and "Unreleased."
  Example:

  ```
  [Unreleased]: https://github.com/org/repo/compare/v1.2.3...HEAD
  [1.2.3]: https://github.com/org/repo/compare/v1.2.2...v1.2.3
  ```
* GitHub auto-converts headings like `## [1.2.3] – YYYY-MM-DD` into comparison links.

---

## 6. Follow Best Practices & Avoid Pitfalls

* **Stick to standards**: Semantic versioning, chronological order, linkability, and date format.
* **Changelogs are for humans**: Avoid committing dump or raw logs.
* **Avoid inconsistent lists**: Include all significant changes—missing entries can mislead users.
* Consider designating **“YANKED”** for pulled-back releases:

  ```md
  ## [0.4.0] - 2023-12-31 [YANKED]
  ```

  This flags unsafe versions clearly.

---

## 7. Maintenance Tips

* Always bump the **Unreleased** section ahead of each release.
* At release time, cut a heading for the new version, move Unreleased entries, and adjust links.
* **Update retroactively** if you missed noting a significant change.
* **Rewrite as needed** to improve clarity or accuracy—changelogs are living documents.

---

### ✅ Changelog Example

```md
# Changelog

All notable changes to this project are documented below.

## [Unreleased]
### Added
- Added: Dark-mode toggle in the settings panel.
### Fixed
- Fixed: Crash when loading large files on Windows.

## [1.2.3] - 2025-06-09
### Added
- Added: CSV export option for reports.
### Changed
- Changed: Improved performance of image renderer by ~30%.
### Fixed
- Fixed: Memory leak in the authentication module.
### Security
- Security: Updated dependency `openssl` to v3.1.0 to fix CVE-2025-1234.

[Unreleased]: https://github.com/org/repo/compare/v1.2.3...HEAD
[1.2.3]: https://github.com/org/repo/compare/v1.2.2...v1.2.3
```

---

## 🚀 Summary

1. Use a **standard template**: heading, date, categories.
2. Write clear, grouped bullet points—**focus on value**.
3. Keep it **human-readable** and consistently formatted.
4. Maintain **linkable sections** for easy browsing.
5. Update **Unreleased** regularly and tidy unused categories.

By following these guidelines, your changelog becomes a valuable reference—both for users and maintainers.

===============================================

Let's do this step by step.
