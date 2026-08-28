## Context

`subx-cli` is about to be split into a `subx-core` library (own repository, mounted as a git submodule) and a `subx-cli` binary crate that becomes the workspace root. SDR §4 fixes exactly which crate each dependency lands in, based on a use-site grep of `src/`, `tests/`, and `benches/`. That grep turned up six manifest defects that are not about the split at all — they are pre-existing rot:

- `notify` 8.0 (`Cargo.toml:95`), `once_cell` 1.19 (`:141`), `tokio-util` 0.7 (`:128`), `winapi` 0.3 (`:196`), and `libc` 0.2 (`:199`) have **zero** use sites in any of the three trees. Not a single `use`, path-qualified call, or `extern crate`.
- `hound` 3.5 (`:129`) has zero `src/` hits and is used by exactly five files under `tests/`, so it is a dev-dependency masquerading as a runtime one.

Two of these are actively misleading. `once_cell` sits under the comment `# Once Cell for runtime initialization`, but the only "once cell" in the codebase is `std::sync::OnceLock` in `src/cli/output.rs:78-79` — the standard-library type that made the crate unnecessary in the first place. `tokio-util` sits under a `# VAD` heading next to the real VAD crates, implying the audio pipeline needs a codec; it does not.

Two more defects are cosmetic but touch project rules. The `[features]` table is the **first** thing in the manifest, above `[package]` (`Cargo.toml:1-5`), which is legal TOML but reads as a mistake and is annotated with the Chinese comment `# 測試用 feature flag` in a project whose AGENTS.md mandates English comments everywhere. And `# Configuration management dependencies: path resolution and multi-core detection` appears twice — correctly at `:136` above `dirs`/`num_cpus`, and stale at `:113` where it sits immediately above `# Audio processing` and describes nothing.

Finally, `src/cli/validation.rs` is a 0-byte file that `src/cli/mod.rs:32-43` never declares. It compiles to nothing, is unreachable from the crate root, and ships inside the published `.crate` archive because `exclude` (`Cargo.toml:18-35`) does not filter `src/`.

This is the first change in the twelve-change split series (SDR §12, ID A0). It runs entirely inside the existing single crate — no submodule, no workspace, no file moves between repositories.

## Goals / Non-Goals

**Goals:**

- Delete every `[dependencies]` and `[target.'cfg(…)'.dependencies]` entry with no use site anywhere in `src/`, `tests/`, or `benches/`.
- Put `hound` in the table that reflects where it is actually used.
- Remove `src/cli/validation.rs` so the published crate contains only files reachable from the crate root.
- Make the manifest read top-to-bottom in conventional order with English-only comments.
- Regenerate `Cargo.lock` mechanically and verify the build is unchanged on both `--no-default-features` and `--features archive-rar`.
- Encode "a dependency with no use site is a spec violation" as a normative requirement, so the rot cannot silently reaccumulate — and so that the two manifests produced by the later split inherit the rule.

**Non-Goals:**

- Removing or downgrading any dependency that *is* used, however lightly. This change deletes only zero-hit entries.
- Narrowing feature flags on surviving dependencies. `tokio` and `symphonia` are already narrowed and are governed by the existing "Narrow dependency feature flags" requirement; nothing here revisits them.
- Pruning transitive dependencies. `once_cell`, `tokio-util`, `winapi`, and `libc` all remain in `Cargo.lock` as transitive dependencies of crates that genuinely need them; this change removes only the *direct* declarations.
- Auditing or upgrading crates for advisories. That is the existing `cargo audit` CI job's job, and this change does not touch the workflow file.
- Anything to do with the split itself: no `[workspace]` table, no `subx-core` path dependency, no submodule. Those belong to B1 (`bootstrap-subx-core-submodule`) onwards.
- Touching any `.rs` file's contents. The only source-tree edit is a deletion of an empty file.

## Decisions

### Decision 1: Delete, never comment out

The five dead entries are removed from `Cargo.toml` outright, along with their now-meaningless section comments (`# File change monitoring` at `:94`, `# Once Cell for runtime initialization` at `:140`, `# Cross-platform dependencies` at `:194`). No `#`-prefixed carcass is left behind.

**Why:** A commented-out dependency line is indistinguishable from a dependency that was temporarily disabled and never restored — it invites a future contributor to uncomment it rather than re-justify it. Git history is the record of what was removed and why; the manifest is the record of what the crate needs today.

**Alternatives considered:**

- *Comment them out with a "removed in A0" note.* Rejected — see above. The proposal, the CHANGELOG entry, and the commit message all carry the justification.
- *Move them behind an off-by-default feature.* Rejected — a feature that gates unused code is still unused code, and it would keep the crates in the resolved graph whenever anyone builds with `--all-features` (including `docs.rs`, which sets `all-features = true` at `Cargo.toml:37-39`).

### Decision 2: Remove the emptied target tables entirely

`winapi` is the only entry under `[target.'cfg(windows)'.dependencies]` (`Cargo.toml:195-196`) and `libc` the only entry under `[target.'cfg(unix)'.dependencies]` (`:198-199`). Both tables are deleted along with their contents rather than left as empty headers.

**Why:** An empty dependency table is inert to Cargo but signals to a reader that the crate has platform-specific needs it does not have. It also creates a tempting, unreviewed place for a future platform hack to land. If a genuine platform dependency appears later, re-adding the table is one line.

### Decision 3: `hound` moves to `[dev-dependencies]`, not behind a feature

`hound` is used by five integration tests to synthesize WAV fixtures for the VAD and sync pipelines:

- `tests/vad_integration_tests.rs:85-92`, `:130-137`
- `tests/vad_performance_tests.rs:40-47`
- `tests/vad_audio_processor_tests.rs:1`
- `tests/sync_engine_integration_tests.rs:1`
- `tests/sync_engine_performance_tests.rs:1`

It moves verbatim (`hound = "3.5"`) into `[dev-dependencies]`, placed next to the other fixture-generation crates.

**Why not a feature?** Integration tests and benches automatically resolve `[dev-dependencies]`; no feature gate, no `cfg`, and no test-source edit is needed. A feature would additionally have to be enabled by every CI invocation and would leak into the published crate's feature list.

**Consequence worth stating:** this is the correct pre-split placement. SDR §4 puts `hound` in `subx-core`'s `[dev-dependencies]`, and all five consumers are `subx_cli::`-library tests that move to `subx-core` in B3 (`split-test-suite-across-crates`). Doing the table move now means B3 transcribes the line instead of re-deciding it.

**Consequence for the release binary:** `hound` currently links into the release binary as a normal dependency even though no `src/` code calls it. Dead-code elimination probably strips it, but "probably" is not a property worth relying on; after this change the question does not arise.

### Decision 4: Manifest section order and comment language

The manifest is reordered to the conventional Cargo layout:

```toml
[package]
…
exclude = […]

[package.metadata.docs.rs]
…

[features]
# Test-only feature flags: `slow-tests` gates long-running tests;
# `archive-rar` enables optional RAR archive extraction.
default = []
slow-tests = []
archive-rar = ["dep:unrar"]

[lints.rustdoc]
…
```

`# 測試用 feature flag` is rewritten in English. The stale duplicate `# Configuration management dependencies: path resolution and multi-core detection` at `:113` is deleted, leaving `# Audio processing` (`:114`) to head the `symphonia` block it actually describes.

**Why `[features]` after `[package.metadata.docs.rs]` rather than immediately after `[package]`?** `[package.metadata.docs.rs]` is a sub-table of `[package]`; splitting the two with an unrelated table is the arrangement most Cargo manifests avoid. Placing `[features]` after the whole `[package]` group and before `[lints.*]` keeps every `[package]`-scoped key contiguous.

**Why this belongs in a dependency-pruning change at all:** it is the same file, the same review, and the same five minutes. Deferring it means a second commit touching `Cargo.toml` in a series where every other change also wants to touch `Cargo.toml` — which is exactly the merge conflict this isolated A0 change exists to avoid.

### Decision 5: `Cargo.lock` is regenerated by Cargo, never hand-edited

After the manifest edits, the lockfile is refreshed by running `cargo build` and then `cargo build --features archive-rar` (so the optional `unrar` subtree is re-resolved too). The resulting diff is reviewed but not authored.

The expected diff is precisely seven package entries removed:

`notify`, `notify-types`, `inotify`, `inotify-sys`, `fsevent-sys`, `kqueue`, `kqueue-sys`

plus the removal of `notify`, `once_cell`, `tokio-util`, `winapi`, and `libc` from the `subx-cli` package's own `dependencies` list, and the relocation of `hound` within it.

**Explicitly not expected:** `once_cell`, `tokio-util`, `winapi`, and `libc` disappearing from the lockfile. They stay, because other crates in the graph depend on them:

| Crate | Remaining reverse dependencies in `Cargo.lock` |
|---|---|
| `once_cell` | `rustls`, `rustls-platform-verifier`, `tempfile`, `wiremock`, `tracing-core`, `js-sys`, `wasm-bindgen`, `quinn-udp`, `ron`, `const-random-macro` |
| `tokio-util` | `reqwest`, `h2` |
| `winapi` | `socks`, `unrar_sys` |
| `libc` | 35 crates including `tokio`, `mio`, `rustix`, `getrandom`, `ring`, `tar`, `num_cpus`, `dirs-sys` |

**Why state this so loudly:** a reviewer who greps `Cargo.lock` after the change and still finds `libc` could reasonably conclude the change did not work. It did — the direct declaration is what was wrong, and the direct declaration is what is gone.

**Why the second build with `archive-rar`:** the `unrar` subtree pulls `unrar_sys`, which is the other consumer of `winapi`. Resolving only the default feature set would leave that half of the graph unverified.

### Decision 6: `src/cli/validation.rs` is deleted, not populated

The file is 0 bytes and is not declared anywhere in `src/cli/mod.rs` (`:32-43` lists `cache_args`, `config_args`, `convert_args`, `detect_encoding_args`, `generate_completion_args`, `input_handler`, `match_args`, `output`, `sync_args`, `table`, `translate_args`, `ui` — no `validation`). Grepping `src/cli/` for `validation` finds only four unrelated doc-comment occurrences of the English word in `config_args.rs`, `cache_args.rs`, and `translate_args.rs`.

**Why delete rather than wire it up:** there is nothing to wire up. It has no contents and no design intent recorded anywhere — no proposal, no spec, no issue. Argument validation in this crate is done by clap derive attributes on the individual `*Args` structs plus `src/config/field_validator.rs`; a separate `cli::validation` module would duplicate one of those.

**Why it matters here:** `Cargo.toml`'s `exclude` list (`:18-35`) filters `tests/`, `benches/`, `assets/`, and media files out of the published archive, but not anything under `src/`. The orphan therefore ships to crates.io in every release. SDR §2.2 already flags it for deletion; doing it in A0 means B2 (`move-core-sources-into-subx-core`) moves only real files.

### Decision 7: The spec rule is "has a use site", not "is used at runtime"

The new requirement is phrased against **use sites in the source trees**, deliberately, rather than against runtime reachability or linker output.

**Why:** "is this crate reachable at runtime" is undecidable in general and needs tooling (`cargo-udeps`, which requires nightly, or `cargo-machete`, which is heuristic) that this project does not run in CI. "Does any file under `src/`, `tests/`, or `benches/` mention this crate" is a grep, verifiable by any contributor in one command, and it catches 100% of the defects actually present. A crate that is declared, mentioned once in a dead `cfg` branch, and never executed is a different and much rarer problem, out of scope here.

**Corollary encoded in the requirement:** the check must consider *all* `cfg` branches, not just the ones that compile on the reviewer's platform. `winapi` and `libc` are exactly the case where a Linux-only grep of compiled code would have been wrong — a text grep of the source tree is not.

### Decision 8: The audit gate is defined over the resolved graph

The existing "CI cargo audit gate" requirement is restated to say that the audited surface is the graph `cargo audit` resolves from `Cargo.lock`, and that this graph SHALL NOT contain packages reachable only through a declaration with no use site.

**Why:** the requirement as written today ("SHALL fail the build pipeline on any direct dependency with a known vulnerability advisory") is silent on what makes a dependency *legitimately* present. Before this change, an advisory on `inotify` would have failed CI over a filesystem watcher the binary never instantiates — a real failure with a nonsense cause, and the kind of thing that trains a team to add advisory ignores. Tying the gate to the resolved graph, and requiring that graph to contain only justified roots, makes every audit failure actionable by construction.

## Risks / Trade-offs

- **Risk: a "dead" crate is actually used through a macro or a re-export the grep missed.** → Mitigation: the grep pattern matches `use <crate>`, `extern crate <crate>`, and any path-qualified `<crate>::` occurrence across all three trees and all `cfg` branches; the result was zero hits for all five, versus 12 hits for `hound` as a positive control. Task 1 re-runs it and records the counts before any edit. The compile in task 5 is the backstop — a missed use site is a hard build error, not a silent regression.
- **Risk: a platform-only use site exists that a Linux grep cannot see.** → Mitigation: the grep is over source text, so `#[cfg(windows)]` blocks are included in the search regardless of host. Additionally the change is verified with `cargo check --target x86_64-pc-windows-msvc` if the target is installed locally, and unconditionally by the existing Windows leg of the CI coverage job.
- **Risk: `hound` in `[dev-dependencies]` breaks a bench.** → Mitigation: benches resolve `[dev-dependencies]` exactly like integration tests do, and in any case both benches (`retry_performance`, `file_id_generation_bench`) have zero `hound` hits. `cargo bench --no-run` in task 5 confirms.
- **Risk: `cargo audit` output does not shrink, making the change look ineffective.** → Mitigation: Decision 5's table documents exactly which crates stay and why. The measurable win is the seven-package `notify` subtree leaving the graph, plus five roots that can no longer be the *cause* of an advisory failure.
- **Risk: the `Cargo.lock` churn invalidates CI caches.** → Mitigation: accepted and one-time. All cache keys in `.github/workflows/build-test-audit-coverage.yml` are `hashFiles('**/Cargo.lock')`, so the next run after merge rebuilds cold and every run after that is warm again. The rebuild is cheaper than before, since seven fewer packages compile.
- **Risk: reordering `[features]` conflicts with a parallel change that also edits `Cargo.toml`.** → Mitigation: A0 is scoped to run first and independently precisely for this reason (SDR §12: A0–A2 happen in the existing single crate). A1 and A2 touch no manifest at all; B1 adds a `[workspace]` table at the end of the file. If a conflict does arise it is a two-line textual one with no semantic ambiguity.
- **Risk: deleting `src/cli/validation.rs` surprises an out-of-tree consumer.** → Mitigation: impossible. The module is not declared in `src/cli/mod.rs`, so it is not part of the crate and no `subx_cli::cli::validation` path has ever resolved. The GUI's consumption surface (SDR §8) does not mention it.
- **Trade-off: the new "no source file unreachable from the crate root" requirement is broader than the one file it removes today.** → Accepted. It is cheap to satisfy (the compiler already knows the module tree) and it is the rule that prevents a second orphan from surviving into the two-crate layout, where "which crate does this file belong to" becomes a much harder question to answer for a file nothing references.

## Migration Plan

1. Re-run the use-site grep for all six crates and record the hit counts as the evidence for the edit (task 1).
2. Delete the five dead entries and their orphaned comments; delete the two emptied `[target.'cfg(…)'.dependencies]` tables (task 2).
3. Move `hound` into `[dev-dependencies]` (task 2).
4. Relocate `[features]` below the `[package]` group and rewrite its comment in English; delete the stale duplicated comment at `:113` (task 3).
5. Delete `src/cli/validation.rs` (task 4).
6. Run `cargo build`, `cargo build --features archive-rar`, and `cargo bench --no-run`; review the `Cargo.lock` diff against the seven-package expectation in Decision 5 (task 5).
7. Re-sync the `docs/tech-architecture.md` dependency block and add the `[Unreleased]` CHANGELOG entries (task 6).
8. Run the quality gate (task 7).

Rollback is a single `git revert`: the change adds no code, no API, and no data format, so nothing depends on it having happened.

## Open Questions

_None._ Every claim in this document was verified against the working tree at `fedc484`.
