---
name: update-config-document
description: Audit and update SubX-CLI's configuration documentation (`docs/config-usage-analysis.md`) so every configuration item is accurately described, linked to its real production usage site, and reflects the current codebase. Use when the user asks to update, refresh, audit, verify, or regenerate the configuration usage analysis doc, when configuration items have been added, renamed, or removed in `src/config/`, when call-hierarchy line numbers in the doc are stale, or when the README's configuration/subcommand reference needs to be synced with updated configuration documentation.
---

# Update Configuration Documentation

Perform a full audit of SubX-CLI's configuration-item documentation and keep
it synchronized with the real codebase.

## Target Files

- **Primary document to update:** `docs/config-usage-analysis.md`
- **Secondary document to update (after primary is done):** `README.md`
- **Source of truth for configuration items:** `src/config/` (structs,
  defaults, validation) and all their consumers across the codebase.

## Workflow

Work **one configuration item at a time**. Do not batch updates across items.

### 1. Read the Current Document

Load `docs/config-usage-analysis.md` first. Treat its existing contents as
**potentially outdated**; every field must be re-verified against the code.

### 2. For Each Configuration Item, Do All of the Following

1. **Verify correctness and relevance.** Confirm the item still exists in
   `src/config/` with the same name, type, default, and semantics.

2. **Identify the real "Actual Usage Location".**
   - Search for every production call site that reads the value.
   - Follow the chain: when a config value is assigned to a struct field,
     continue searching for uses of that struct field. A field that is
     written but never read is effectively **unused** — flag it.
   - **Ignore all references that appear only in unit tests, integration
     tests, test helpers, `#[cfg(test)]` blocks, or the `tests/` directory.**
     Only production-level usage counts.

3. **Exclude items that are obviously set only through the `subx-cli config`
   subcommand.** These CLI-plumbing entries do not need their own doc row.

4. **Update the document immediately** after verifying this item. Do **not**
   accumulate edits across multiple items — commit each correction to the
   file as soon as it is established. This keeps progress auditable and
   recoverable if the session is interrupted.

### 3. Refresh the Call Hierarchy ("呼叫樹")

Cross-check every line-number reference inside each call-tree block against
the current source files and update any that have drifted.

### 4. Discover Undocumented Items

Scan `src/config/` for configuration fields that are **not** represented in
`docs/config-usage-analysis.md`. For every missing item, append a new entry
following the existing document's format and conventions.

### 5. Sync the README

Once every configuration item has been validated and the document fully
reflects the current code, update `README.md` so the subcommand and
configuration user-facing documentation matches the refreshed analysis.

## Rules and Reminders

- All documentation updates are written in **English**, matching the project
  convention for docs and code comments.
- Do not mark items as deprecated. If a configuration item is no longer used
  in production, report it to the user for removal instead of silently
  keeping a dead entry.
- Do not rely on memory for line numbers — always re-read the current file
  before recording a line reference.
- Proceed methodically and incrementally: verify, update the doc, move on.
