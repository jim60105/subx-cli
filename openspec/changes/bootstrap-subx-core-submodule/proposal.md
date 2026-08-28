## Why

SDR §0 fixes the end state: `subx-core` is a library in **its own git repository** (`https://github.com/jim60105/subx-core`), consumed by `subx-cli` as a git submodule mounted at `subx-core/`, with `subx-cli` acting as the Cargo workspace root. Today none of that physical structure exists. There is one repository, one `Cargo.toml`, one crate, and no workspace table anywhere.

A0–A2 have by now made the *logical* split possible — the thirteen `core`/`services` → `cli` edges are gone (A1), the four misplaced modules sit on the correct side of the line (A2), and five phantom dependencies no longer have to be transcribed into two manifests (A0). What is still missing is somewhere to put the code. B2 (`move-core-sources-into-subx-core`) is scheduled as a near-pure `git mv` of `src/core/**`, `src/services/**`, `src/config/**` and `src/error.rs`; that is only true if the destination directory, the second manifest, the workspace, and the submodule pointer already exist and are known to work.

Doing the skeleton and the move in one change would fuse two unrelated failure modes. A workspace that does not resolve, a submodule that is not checked out in CI, a manifest that accidentally declares a nested `[workspace]`, or a `path`-only dependency that cannot be published are all infrastructure faults with infrastructure symptoms. Compile errors from `broken_intra_doc_links = "deny"` crossing a fresh crate boundary are code faults. Diagnosing the second while the first is still uncertain is exactly the situation SDR §12 arranges the series to avoid.

Three specific hazards make the skeleton worth isolating and proving:

| Hazard | Why it bites later if not proven now |
|---|---|
| Nested workspace roots | A `[workspace]` table inside `subx-core/Cargo.toml` is a hard Cargo error (rust-lang/cargo#14862), not a warning. Discovering it during B2 means debugging manifest structure while 46k LOC are mid-flight. |
| Workspace inheritance | `version.workspace = true`, `[workspace.dependencies]` and `[workspace.lints]` all resolve against the *parent* workspace. Every one of them compiles fine inside `subx-cli` and breaks a standalone `git clone` of `subx-core` — which is the only way the Tauri GUI and crates.io consumers will ever see the crate. |
| Empty submodule directories | Seven `actions/checkout@v6` steps currently lack `submodules: recursive` (SDR §7). Without it the submodule directory is empty, the workspace member fails to load, and every CI job fails at manifest-parse time with an error that says nothing about submodules. |

This change therefore creates the two-crate skeleton and **moves no application code at all**. When it lands, `src/` is byte-identical to what A2 left behind, `cargo build` produces the same binary, `scripts/quality_check.sh` is green, and the CLI's observable behaviour is unchanged — but the second repository exists, the submodule is pinned, the workspace resolves, and `subx-cli` provably compiles against `subx-core`.

## What Changes

**1. A new repository `https://github.com/jim60105/subx-core` (GPL-3.0-or-later).**

Its initial commit contains no application logic — only the crate skeleton and the per-repository configuration that a submodule cannot inherit from its parent (SDR §5):

| File | Content |
|---|---|
| `LICENSE` | Verbatim copy of `subx-cli/LICENSE` (GPL-3.0, 674 lines). |
| `README.md` | What the crate is, how it relates to `subx-cli` and to the Tauri GUI, and that it is consumed as a submodule. |
| `.gitignore` | Copy of `subx-cli/.gitignore`. Required, not optional: a submodule has its own index and worktree, so the parent's ignore rules never apply inside it — and the parent's `/target/` is root-anchored anyway. |
| `.gitattributes` | The `tests/fixtures/formats/** -text` rule, shipped **now** even though the fixtures do not arrive until B3. |
| `rustfmt.toml` | `edition = "2024"`, `max_width = 100`. |
| `.config/nextest.toml` | The four profiles (`default` / `ci` / `quick` / `full`), with the Chinese comments rewritten in English per AGENTS.md. |
| `.llvm-cov.toml` | Same shape as the parent's, minus the `src/main.rs` exclusion, which is meaningless for a library. |
| `.codegraph/.gitignore` | The three-line self-ignoring file, so `subx-core` can be CodeGraph-indexed independently. |
| `AGENTS.md` | Derived from `subx-cli/AGENTS.md`: coding, testing, documentation and configuration conventions kept; CLI execution flow, release matrix and shell-completion sections dropped; a new section on the submodule contract added. |
| `Cargo.toml` | `[package] name = "subx-core", version = "1.0.0", edition = "2024"` plus full crates.io metadata, `[package.metadata.docs.rs]`, `[features] default = []`, and duplicated `[lints.rustdoc]` / `[lints.clippy]` / `[lints.rust]` blocks. **No `[workspace]` table and no workspace inheritance.** No `[profile.*]`. |
| `src/lib.rs` | A documented placeholder exposing exactly one item: `pub const VERSION: &str = env!("CARGO_PKG_VERSION");`. |

**2. The submodule.** `git submodule add -b main https://github.com/jim60105/subx-core subx-core` creates `.gitmodules` with `path`, `url` and `branch = main`, and commits the gitlink at `subx-core/`.

**3. `subx-cli/Cargo.toml` becomes the workspace root.** A `[workspace]` table is added as the manifest's first table:

```toml
[workspace]
members = [".", "subx-core"]
resolver = "3"
```

`[profile.release]` and `[profile.dev]` stay exactly where they are and **only** where they are — Cargo ignores `[profile.*]` in a non-root member and warns on every build if one is declared there. `default-members` is deliberately not set.

**4. The dependency.** `subx-cli` gains `subx-core = { path = "subx-core", version = "1.0" }` under `[dependencies]`. The dual form is mandatory (SDR D4): `path` drives local and workspace builds, and `cargo publish` strips it and ships only the `version` requirement. A crate whose dependency is declared with `path` alone cannot be published to crates.io at all.

**5. The wiring is proven, not merely declared.** `src/lib.rs` gains one new public item next to the existing `VERSION` (`src/lib.rs:114`):

```rust
/// Version of the `subx-core` library this build is linked against.
pub const CORE_VERSION: &str = subx_core::VERSION;
```

plus a unit test asserting that `CORE_VERSION`'s major component is `1`, matching the caret requirement in the dependency line. A missing or stale submodule checkout then fails at `cargo build`, immediately and with a comprehensible message, instead of surfacing as a mystery in B2.

**6. The minimum CI touch.** All seven `actions/checkout@v6` steps gain `submodules: recursive`; two of them additionally gain `fetch-depth: 0`:

| Workflow | Job | Step | Added |
|---|---|---|---|
| `.github/workflows/build-test-audit-coverage.yml` | `test` | `:26` | `submodules: recursive` |
| | `archive-rar` | `:99` | `submodules: recursive` |
| | `security` | `:145` | `submodules: recursive` |
| | `coverage` | `:157` | `submodules: recursive` |
| `.github/workflows/release.yml` | `create-release` | `:18` | `submodules: recursive`, `fetch-depth: 0` |
| | `build` | `:95` | `submodules: recursive` |
| | `publish-crates` | `:192` | `submodules: recursive`, `fetch-depth: 0` |

Nothing else in the workflows changes. The two-stage `cargo publish --workspace` flow (SDR D5), the pinned-pointer version assertion, the per-crate coverage thresholds, the `hashFiles('**/Cargo.lock')` cache keys, the `subx-core` repository's own CI, and the `subx-cli` 2.0.0 bump are **all C1's** (`update-ci-and-release-for-two-crates`) and are not duplicated here.

**7. The contributor-facing consequence is documented.** A fresh clone is `git clone --recurse-submodules https://github.com/jim60105/subx-cli`; an existing clone needs `git submodule update --init --recursive`; and `git config submodule.recurse true` makes `git pull` and `git checkout` carry the pointer forward automatically thereafter.

No source file under `src/core/`, `src/services/`, `src/config/`, `src/cli/` or `src/commands/` is touched. No CLI flag, configuration key, JSON envelope field, error variant or public API is added, removed or changed apart from `subx_cli::CORE_VERSION`. `subx-cli`'s own version stays at `1.9.1`.

## Capabilities

### New Capabilities

- `crate-topology`: The repository and crate structure contract for the two-crate split. Fixes which repository owns which crate; the submodule mount path and `.gitmodules` contents; the workspace shape and the root-only ownership of `[profile.*]`; the absolute prohibition on a nested `[workspace]` table or workspace inheritance inside `subx-core`, together with the standalone-clone guarantee that prohibition exists to protect; the dual `{ path, version }` dependency form and why a `path`-only declaration is unpublishable; the independent version lines of the two crates and the caret relationship between them; the exhaustive list of configuration files that must be duplicated into the core repository rather than inherited; the submodule checkout requirements for both contributors and every CI job; and the compile-time reference that proves the wiring. This capability describes *structure*; per-module ownership (which source file belongs to which crate) is deliberately left unspecified here and is added by B2.

### Modified Capabilities

_None._

## Impact

- **Code:** `Cargo.toml` — a new `[workspace]` table at the top of the manifest and one new `[dependencies]` entry; `[profile.release]` and `[profile.dev]` unchanged and unmoved. `src/lib.rs` — one new `pub const CORE_VERSION` beside the existing `VERSION` (`:114`), its rustdoc, and its module-header mention. `.gitmodules` — new file. `subx-core/` — new submodule gitlink. Nothing under `src/core/`, `src/services/`, `src/config/`, `src/cli/`, `src/commands/`, `src/main.rs` or `src/error.rs` changes.
- **Tests:** One new unit test in `src/lib.rs`'s existing `#[cfg(test)]` module, asserting `CORE_VERSION` is non-empty and that its major component is `1`. No existing test is modified. The 136 test files and 23,006 LOC of `tests/` are untouched — they move in B3. `scripts/check_coverage.sh:369` already runs `cargo llvm-cov nextest --workspace`, so `subx-core` enters the coverage denominator at this change; a `pub const` contributes no executable coverage regions, so the measured percentage is expected to be unchanged, and the 75% threshold stays at 75% for the combined workspace. Per-crate thresholds are C1's.
- **APIs:** *Added:* `subx_core::VERSION` (the entirety of the new crate's public surface) and `subx_cli::CORE_VERSION`. *Unchanged:* everything else. `subx_cli::VERSION` keeps its meaning — the CLI's own version — and is not aliased to the core's. The `--version` output of the binary is not changed.
- **Dependencies:** `subx-cli` gains exactly one direct dependency, `subx-core = { path = "subx-core", version = "1.0" }`. `subx-core` declares **no** dependencies and **no** dev-dependencies at this change; SDR §4's allocation is transcribed in B2. `Cargo.lock` gains one path-source `[[package]]` entry for `subx-core` and no registry packages. `subx-core` commits its own `Cargo.lock` for standalone reproducibility; inside the workspace that file is inert, because the workspace root's lockfile governs.
- **Repositories and CI:** A second git repository comes into existence and becomes a hard prerequisite for building `subx-cli`. Seven `actions/checkout@v6` steps gain `submodules: recursive`; two gain `fetch-depth: 0`. `submodules: recursive` needs no credentials because `subx-core` is public over HTTPS. `scripts/quality_check.sh` and `scripts/check_coverage.sh` are **not** modified — `cargo fmt` already spans workspace members, `cargo clippy -- -D warnings` and `cargo nextest run` still cover only the root package (`default-members` is unset), and that is acceptable while `subx-core` holds one const and no tests. Widening them is B3/C1 work.
- **Documentation:** `README.md` and `README.zh-TW.md` gain a build-from-source note about `--recurse-submodules`. `AGENTS.md` gains a repository-layout section covering the submodule, the workspace, and the nested-workspace prohibition (its stale "7 targets" release claim is left for C3). `docs/tech-architecture.md` gains a short two-crate topology note ahead of C3's full rewrite. `CHANGELOG.md` gains `[Unreleased]` → `### Added` and `### Changed` entries. The new repository ships its own `README.md` and `AGENTS.md`.
