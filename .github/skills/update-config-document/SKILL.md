---
name: update-config-document
description: Audit SubX-CLI's configuration coverage matrix (`docs/config-usage-analysis.md`) so every configuration key in `subx-core/src/config/` has a row with the correct status. Use when the user asks to audit, verify, or update the configuration usage analysis doc, when configuration items have been added, renamed, or removed in `subx-core/src/config/`, or when the README's configuration/subcommand reference needs to be synced with the configuration documentation.
---

# Update Configuration Documentation

Given a new or changed key in `subx-core/src/config/`, confirm the coverage
matrix in `docs/config-usage-analysis.md` has a row for it and that the row's
status is correct.

> **The call-hierarchy line numbers in this document are a dated historical
> record and are NOT maintained** — the document's own header declares this.
> Do not re-derive or "refresh" them; doing so reverses the document's
> declared demotion. Current behaviour lives in `docs/configuration-guide.md`.

## Target Files

- **Primary document to update:** `docs/config-usage-analysis.md`
- **Secondary document to update (after primary is done):** `README.md`
- **Source of truth for configuration items:** `subx-core/src/config/`
  (structs, defaults, validation — the configuration crate half of the
  two-crate split) and all their consumers across `subx-core/src/` and
  `src/` (the CLI half).

## Workflow

Work **one configuration item at a time**. Do not batch updates across items.

### 1. Read the Current Document

Load `docs/config-usage-analysis.md` first. Treat its existing contents as
**potentially outdated**; every field must be re-verified against the code.

### 2. For Each Configuration Item, Do All of the Following

1. **Verify correctness and relevance.** Confirm the item still exists in
   `subx-core/src/config/` with the same name, type, default, and semantics.

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

### 3. Discover Undocumented Items

Scan `subx-core/src/config/` for configuration fields that are **not** represented in
`docs/config-usage-analysis.md`. For every missing item, append a new entry
following the existing document's format and conventions.

### 4. Sync the README

Once every configuration item has been validated and the document fully
reflects the current code, update `README.md` so the subcommand and
configuration user-facing documentation matches the audited matrix.

## Rules and Reminders

- All documentation updates are written in **English**, matching the project
  convention for docs and code comments.
- Do not mark items as deprecated. If a configuration item is no longer used
  in production, report it to the user for removal instead of silently
  keeping a dead entry.
- Never rewrite the call-tree ("呼叫樹") blocks: their line numbers are the
  dated record described above. A new row records its usage sites by path
  and function, not by line number.
- Proceed methodically and incrementally: verify, update the doc, move on.
