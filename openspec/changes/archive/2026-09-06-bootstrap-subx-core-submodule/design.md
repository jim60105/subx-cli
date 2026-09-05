## Context

`subx-cli` is one crate: one `Cargo.toml`, one `src/`, one `tests/`, one repository. SDR §0 fixes the target shape — a `subx-core` library in its own repository at `https://github.com/jim60105/subx-core`, mounted into `subx-cli` as a git submodule at `subx-core/`, with `subx-cli` as the Cargo workspace root and both crates published independently to crates.io.

Two consumers pull in opposite directions and jointly force that shape. The **CLI** wants a single buildable tree with tuned release profiles, an `assert_cmd` integration suite, and a five-target release matrix. The **Tauri GUI** at `../subx` wants a library and nothing else: SDR §8 verified across all sixteen `.rs` files of `../subx/src-tauri/src/` that it references `subx_cli::commands` zero times, any clap `*Args` type zero times, and `cli::{ui, output, table}` zero times. Its end state is a `Cargo.toml` naming `subx-core` and no mention of `subx-cli` at all. That is only expressible if the core is a separately addressable crate in a separately clonable repository.

By the time this change runs, A0–A2 have removed the logical obstacles. The thirteen `core`/`services` → `crate::cli` edges are gone behind A1's `Reporter` seam; `input_handler`, the sync pairing logic and the `exit_code`/`user_friendly_message` presentation methods are on their correct sides after A2; five zero-use-site dependencies are gone after A0. What remains is purely physical: there is nowhere to move the code to.

This change builds that destination and proves it works, without moving one line of application code. The success criterion is deliberately narrow and mechanical: after B1, `git clone --recurse-submodules` followed by `cargo build` is green, `scripts/quality_check.sh` is green, the produced binary behaves identically, and `subx_core::VERSION` is referenced from `subx-cli`'s own crate root so that a broken submodule checkout cannot be silently tolerated.

## Goals / Non-Goals

**Goals:**

- Create `https://github.com/jim60105/subx-core` (GPL-3.0-or-later) with an initial commit that carries the crate skeleton plus every per-repository configuration file that a submodule cannot inherit (SDR §5).
- Mount it as a submodule at `subx-core/` with `.gitmodules` recording `branch = main`.
- Turn `subx-cli/Cargo.toml` into the workspace root with `members = [".", "subx-core"]` and an explicit `resolver`, keeping `[profile.*]` at the root and only at the root.
- Guarantee that `subx-core` builds, formats, lints and tests correctly **both** as a workspace member and as a standalone clone — which is the constraint that rules out workspace inheritance.
- Declare the dependency in the dual `{ path, version }` form so both crates remain publishable.
- Prove the wiring end to end with a compile-time reference plus a version-agreement test, so an unresolved submodule fails at B1 rather than at B2.
- Make every CI job check the submodule out, with the smallest possible workflow diff.
- Leave a written record of the contributor-facing clone/update commands.

**Non-Goals:**

- Moving any source file, test, bench, asset or fixture into `subx-core`. That is B2 and B3. `subx-core/src/lib.rs` holds one `const` and no logic.
- Splitting `[dependencies]` or `[dev-dependencies]` across the two manifests. SDR §4's allocation is transcribed in B2; `subx-core` declares no dependencies at all here.
- Re-allocating the `archive-rar` and `slow-tests` features. They stay exactly as A0 left them in `subx-cli`; SDR §4's pass-through arrangement lands in B2 when `unrar` actually moves.
- Reworking the release pipeline. `cargo publish --workspace` (SDR D5), the pinned-pointer version assertion, per-crate coverage thresholds, `hashFiles('**/Cargo.lock')` cache keys, and the core repository's own CI are C1's, itemised in SDR §7.
- Bumping `subx-cli` to 2.0.0. That is C1 (SDR D6); this change leaves it at `1.9.1`.
- Adding `openspec/` to the `subx-core` repository. SDR D12 assigns `openspec init` and the `import-core-specs` change to C2a.
- Introducing `cargo-release` or `release-please`. SDR §7 records that the first refuses submodule workspace members and the second's rust strategy does not understand submodules; neither is in use today and neither is introduced.
- Changing `subx-cli --version` output, any CLI flag, any configuration key, or any JSON envelope field.

## Decisions

### Decision 1: A git submodule, over a monorepo workspace, a subtree, or a crates.io-only dependency

The requirement that decides this is not technical convenience — it is SDR §0/D1: **the core must live in its own git repository.** The GUI at `../subx` is a separate repository with its own release cadence; it must be able to clone, fork, file issues against and eventually vendor the core without dragging the CLI's release matrix, `assert_cmd` suite and clap surface along. Given that constraint, the question is only how `subx-cli` consumes it.

**Rejected — a monorepo workspace (`crates/subx-core`, `crates/subx-cli`).** This is the option that is technically nicest in every respect except the one that matters: it does not produce a second repository. `cargo publish --workspace`, shared `Cargo.lock`, atomic cross-crate refactors, one CI run, no pointer to go stale. If SDR D1 were negotiable this would win. It is not.

**Rejected — `git subtree`.** It does give `subx-cli` a real directory of real files with no pointer and no `--recurse-submodules` for contributors. But every contribution to core made from inside `subx-cli` has to be pushed back with `git subtree push`, history is duplicated in both repositories, and the two copies drift silently rather than loudly. A submodule's failure mode — "the pointer is behind" — is visible in `git status` and `git submodule status`. A subtree's failure mode is a divergence nobody notices for weeks.

**Rejected — crates.io-only, with `[patch.crates-io]` for local work.** `subx-cli` would depend on a published `subx-core = "1"` and developers would add a `[patch]` section pointing at a sibling checkout. This decouples the repositories completely, but it makes the *default* developer experience "you cannot build a change to core and the CLI together without editing a manifest", and `[patch]` sections are exactly the kind of thing that gets accidentally committed. It also means every core fix requires a crates.io release before the CLI can consume it, which is precisely wrong during a twelve-change split where core changes daily.

**Chosen — submodule.** It satisfies D1, keeps one checkout that builds everything, keeps the pointer explicit and reviewable in `git diff`, and composes with the Cargo workspace so that `path` resolution just works.

**What it costs, stated plainly rather than glossed:**

- **The pointer is maintained by hand.** A core commit is invisible to `subx-cli` until someone commits the moved gitlink. There is no automatic propagation. Decision 8 lists the mitigations; none of them removes the manual step.
- **Contributors will forget `--recurse-submodules`.** The failure is a Cargo manifest-load error naming `subx-core/Cargo.toml`, which does not mention submodules. This is why the clone instructions go into three READMEs and `AGENTS.md`, and why `git config submodule.recurse true` is recommended in the same breath.
- **`cargo-release` refuses submodule workspace members**, and `release-please`'s rust strategy does not model submodules (SDR §7). Neither is in use — the flow is a manual `v*` tag — so this costs nothing today, but it does foreclose adopting either later without extra work.
- **`cargo package` and `cargo publish` recursively check submodules for uncommitted changes** (Cargo ≥ 1.39) and treat a moved pointer as dirty. The pointer must be committed before publishing, and `--allow-dirty` must never be used to paper over it: it stamps `"dirty": true` into `.cargo_vcs_info.json` inside the published `.crate`.
- **Two CI systems.** Core changes are untested until a pointer bump unless `subx-core` gets its own workflow. That workflow is C1's (SDR §7), which means there is a window between B1 and C1 in which core-side commits are only validated through `subx-cli`. Acceptable, because during that window core holds one `const`.

### Decision 2: `subx-cli/Cargo.toml` is the workspace root, and `[profile.*]` lives there and nowhere else

```toml
[workspace]
members = [".", "subx-core"]
resolver = "3"
```

placed as the **first table** in the manifest, above `[package]`. A0's manifest normalisation asserted that `[package]` be the first table (so that `[features]` stops being); that assertion is about `[features]`, and a workspace root conventionally announces the workspace before the package. The reader should learn the shape of the tree before the shape of one crate in it.

`"."` is listed explicitly even though the root package of a non-virtual workspace is a member automatically. SDR D3 spells the list that way, and an explicit list is self-documenting when a third member eventually appears.

`resolver = "3"` is declared explicitly even though it would be inferred from the root package's `edition = "2024"`. Inference is a property of the *root package's* edition, so the moment a virtual manifest or a differently-editioned root is contemplated the inference silently changes. Writing it down costs one line and removes the question.

**`[profile.release]` and `[profile.dev]` stay in `subx-cli/Cargo.toml` and are not copied into `subx-core/Cargo.toml`.** This is not a stylistic preference. Cargo ignores `[profile.*]` in a non-root workspace member and emits a warning on *every* build that one is present. In a project whose CI runs `cargo clippy -- -D warnings` and whose quality gate is a single green script, a permanent per-build warning is not acceptable. The consequence is that a standalone `cargo build --release` inside a lone `subx-core` clone gets Cargo's stock release profile rather than `opt-level = 3, lto = true, codegen-units = 1, panic = "abort", strip = true`. That is harmless: a library is never shipped as a standalone release artifact, and in every configuration that matters — the workspace build, and a crates.io consumer's build — the *consumer's* profile governs the compilation of `subx-core` anyway.

**`default-members` is deliberately not set.** For a non-virtual workspace, the default member set is the root package, so `cargo build`, `cargo clippy` and `cargo nextest run` still act on `subx-cli` alone (and pull `subx-core` in as a dependency). Three consequences follow, all acceptable at B1 and all owned later:

- `cargo fmt -- --check` **does** span members — `cargo-fmt` enumerates workspace targets — so `subx-core/src/lib.rs` is format-checked from the parent, using `subx-core/rustfmt.toml`.
- `cargo clippy -- -D warnings` does **not** lint `subx-core` unless `--workspace` is passed. At B1 that leaves one `const` unlinted; the tasks verify `cargo clippy --workspace -- -D warnings` by hand without changing `scripts/quality_check.sh`.
- `scripts/check_coverage.sh:369` already runs `cargo llvm-cov nextest --profile … --workspace`, so `subx-core` enters the coverage denominator immediately. A `pub const` emits no executable coverage regions, so the measured percentage should not move. Per-crate thresholds are C1's (SDR §6).

### Decision 3: `subx-core/Cargo.toml` carries no `[workspace]` table — and therefore no workspace inheritance either

These are two halves of one decision, and the second half is the one that gets forgotten.

**Half one: no `[workspace]` table.** Two manifests each declaring a `[workspace]`, one nested inside the other, is a hard Cargo error, not a warning (rust-lang/cargo#14862). `subx-core` is a member of the `subx-cli` workspace; a member cannot also be a root. The only escape hatch Cargo offers is `package.workspace` or an `exclude` entry, neither of which is wanted here. So `subx-core/Cargo.toml` contains `[package]`, `[package.metadata.docs.rs]`, `[features]`, the three `[lints.*]` tables, `[dependencies]` and `[dev-dependencies]` — and nothing else.

**Half two: therefore no workspace inheritance.** `version.workspace = true`, `[workspace.dependencies]` + `foo.workspace = true`, `[workspace.lints]` + `[lints] workspace = true`, `authors.workspace = true` — every one of these resolves against *the workspace the package is currently a member of*. Inside `subx-cli` they all resolve fine, which is exactly what makes them dangerous: the manifest looks correct, the workspace build is green, and the breakage is invisible until somebody does the thing the whole split exists to enable —

```
git clone https://github.com/jim60105/subx-core
cd subx-core && cargo build
```

— at which point there is no parent workspace and Cargo fails with "error: failed to parse manifest … `version` was inherited but no `package.version` was found". The GUI's future `subx-core = "1"` dependency, `docs.rs`, `cargo install`, and every crates.io consumer see the crate through that standalone lens, not through `subx-cli`'s.

The practical price is duplication: `[lints.rustdoc]`, `[lints.clippy]` and `[lints.rust]` are written out literally in both manifests, and their contents must be kept in agreement by review rather than by Cargo. That is a real cost and it is the right trade — a lint block that drifts produces warnings someone reads; an inheritance that breaks produces a crate nobody can build.

**Verification is part of the change, not a hope.** The tasks include cloning `subx-core` into a scratch directory *outside* the `subx-cli` tree and running `cargo build` and `cargo fmt --check` there. Doing it inside a subdirectory of `subx-cli` would not test anything, because Cargo would walk up and find the parent workspace.

### Decision 4: The dual `{ path, version }` dependency form

```toml
subx-core = { path = "subx-core", version = "1.0" }
```

The two keys serve two different resolvers and neither is optional.

- **`path` drives every local build.** Inside the workspace, `subx-core` resolves to the submodule directory, so a change to core is visible to `subx-cli` on the next `cargo build` with no publish step and no registry round-trip. This is what makes B2 and B3 tractable at all.
- **`version` is what survives publication.** `cargo package` strips `path` from the manifest it writes into the `.crate` archive and leaves the registry requirement behind. `version = "1.0"` is the caret requirement `^1.0`, so a published `subx-cli` accepts any `subx-core` `>=1.0.0, <2.0.0`.

Omitting `version` is not a smaller version of this decision — it is a different, broken one. A dependency declared with `path` alone is unpublishable: `cargo publish` rejects the crate outright, because the stripped manifest would name a dependency with no source. That failure would surface at the first `v*` tag after the split, in the `publish-crates` job, at the worst possible moment. Declaring it now, three changes before anything is published, costs nothing.

Omitting `path` (registry-only) is the crates.io-only option already rejected in Decision 1.

The pairing carries one standing obligation: the caret requirement and the version in the pinned core commit must stay compatible. Nothing in Cargo checks this — inside the workspace, `path` wins and the `version` key is not even consulted for resolution. Decision 8 covers how that is caught.

### Decision 5: SDR §5's configuration files are duplicated into the core repository, because inheritance either does not exist or does not survive a standalone clone

Per-tool, the actual mechanics differ, and the honest reason to duplicate is not the same reason for each:

| File | Behaviour in the workspace | Behaviour in a standalone `subx-core` clone | Why duplicate |
|---|---|---|---|
| `rustfmt.toml` | rustfmt walks upward from each source file, so `subx-core/src/lib.rs` would find `subx-cli/rustfmt.toml` if core had none | Nothing to walk up to; rustfmt falls back to its defaults, and `max_width` silently becomes 100 → 100 is the default anyway, but `edition` becomes 2015 and the file fails to parse | Required for the standalone case; also pins core's style independent of the parent's |
| `.config/nextest.toml` | nextest reads `<workspace-root>/.config/nextest.toml` — i.e. the **parent's** file. A member's own copy is not consulted | nextest reads `subx-core/.config/nextest.toml`, since that is then the workspace root | Required for core's own CI (C1) to have the `ci` / `quick` / `full` profiles at all |
| `.llvm-cov.toml` | Referenced by no script and no workflow today; `scripts/check_coverage.sh` passes its flags on the command line | Same | Duplicated for parity so C1 inherits a symmetric pair; C1 decides whether to make it load-bearing or delete both |
| `[lints.*]` | Would be inheritable via `[workspace.lints]` — banned by Decision 3 | Must be present or `broken_intra_doc_links = "deny"` is not enforced in core's own CI | Written out literally in both manifests |
| `[package.metadata.docs.rs]` | Per-package by definition; never inherited | Same | Written out in both |
| `.gitignore` | Never applies. A submodule has its own index and worktree; the parent's ignore rules stop at the gitlink. Even if they did apply, the parent's `/target/` is root-anchored and would not match `subx-core/target/` | Governs everything | Required |
| `.gitattributes` | Same as `.gitignore` — a submodule's attributes come from its own tree | Governs everything | Required, and see below |
| `.codegraph/.gitignore` | Per-repository | Per-repository | Required if core is to be indexed independently |
| `LICENSE` | Not inherited by anything | Not inherited by anything | GPL-3.0-or-later; a crates.io crate must carry its own licence text |

Two of these deserve their own note.

**`.gitattributes` ships at B1, three changes before the files it protects arrive.** `tests/fixtures/formats/**` is byte-exact and depends on `-text` to keep its CRLF and BOM bytes intact (SDR §6). B3 moves those fixtures. An attributes rule added *after* the files are committed does not retroactively undo a normalisation that already happened at `git add` time — the bytes are gone and the round-trip tests fail with a diff nobody can read. Shipping the rule in the initial commit costs one line and makes B3's move safe by construction.

**Comments are rewritten in English.** `subx-cli`'s `.config/nextest.toml` and `.llvm-cov.toml` carry Chinese comments, which AGENTS.md's English-only rule forbids. The copies written into `subx-core` are English. This deliberately introduces a comment-level divergence between the two files at B1; reconciling the parent's copies is left to C1, which is the change that owns `.llvm-cov.toml` and the coverage scripts, so that B1's diff stays confined to the workflows' checkout steps.

### Decision 5a: The `.config/nextest.toml` junit path is C1's to fix, and the defect is not quite where SDR §5 places it

SDR §5 flags the `ci` profile's `path = "junit.xml"` as writing somewhere other than the `target/nextest/ci/junit.xml` that CI reads at `build-test-audit-coverage.yml:90` and `:213`. The mechanism is worth stating precisely, because it changes which line is wrong.

nextest resolves a profile's `junit.path` **relative to that profile's store directory**, `target/nextest/<profile-name>/`. So `[profile.ci.junit] path = "junit.xml"` lands on `target/nextest/ci/junit.xml` — exactly what the workflow uploads. The file that actually misresolves is `[profile.default.junit] path = "target/nextest/junit.xml"` (`.config/nextest.toml:55-57`), which becomes `target/nextest/default/target/nextest/junit.xml`. The two entries in `.gitignore:19-20` — a bare `junit.xml` *and* `target/nextest/*/junit.xml` — are the fossil record of that confusion.

**Neither is repaired here.** B1 copies the file into `subx-core` verbatim (modulo English comments) so the two remain diffable, and C1 — which designs per-crate CI, per-crate junit uploads and per-crate coverage thresholds — verifies the resolution empirically and repairs both files together. Fixing it in B1 would mean changing CI artifact paths in a change whose entire CI contract is "add `submodules: recursive` and nothing else", and would leave the two repositories' profiles divergent for the three changes until C1.

### Decision 5b: `subx-core` commits its own `Cargo.lock`

A lone `subx-core` clone with its own CI (C1) needs a reproducible resolution, and modern Cargo guidance is to commit the lockfile for libraries as well as binaries. Inside the workspace the file is inert — the workspace root's `Cargo.lock` is the only one Cargo reads or writes — so it cannot be silently mutated into a dirty submodule, which is the failure this would otherwise risk given Decision 1's `cargo package` dirty check. `Cargo.lock` stays in `subx-core`'s `exclude` list, mirroring `subx-cli`'s manifest, so it is not shipped inside the `.crate`.

The visible consequence is that `hashFiles('**/Cargo.lock')` in the four cache steps of `build-test-audit-coverage.yml` now globs two files instead of one, so cache keys change once and then behave normally. SDR §7 records this; narrowing the glob is C1's.

### Decision 6: `subx-core` starts at 1.0.0; `subx-cli` goes to 2.0.0 — but not in this change

SDR D6 locks both numbers and independent version lines with caret ranges. This is where the numbers first appear in the tree, so the reasoning belongs here.

**Why `subx-core` starts at 1.0.0 rather than 0.1.0.** The crate is not new code looking for its shape; it is ~46k LOC that has been shipping inside `subx-cli` since 1.0 and is already consumed in production by the Tauri GUI against a verified API surface (SDR §8). A `0.x` version would be an honest signal only if the API were expected to churn, and it is not — SDR §3 goes out of its way to preserve `MatchEngine::new`, `SyncEngine::new`, `TranslationEngine::new` and `ComponentFactory::new` signatures precisely so the GUI does not break. `0.x` also has a sharper practical cost: under Cargo's semver rules every `0.x.y` bump is potentially breaking, so `subx-core = "0.1"` would forbid the GUI from picking up `0.2` automatically. 1.0.0 states the truth and gives consumers a caret range that actually ranges.

**Why `subx-cli` goes to 2.0.0.** Its library surface changes shape, not just contents: `subx_cli::{config, core, error, services}` become re-exports of another crate (SDR D11), `SubXError::exit_code` and `user_friendly_message` became trait methods in A2, and the crate stops being the place where the engines live. Consumers who used `subx-cli` as a library — the role `subx-core` now fills — must act. That is a major bump by definition. The *binary's* behaviour is unchanged, which is why the bump reads as surprising until you remember the crate has always had two audiences.

**Why not here.** The bump is a release event: it wants the CHANGELOG heading, the release-notes extraction, the two-stage publish order (core first, then cli — SDR D5), and the tag, all of which are C1's. Bumping the version in B1 would leave `subx-cli` claiming 2.0.0 for three changes while still being 1.9.1 in every observable respect. `subx-cli` therefore stays at `1.9.1` through B1–B3 and moves to 2.0.0 in C1.

**The version relationship is a live constraint from B1 onward.** `subx-core`'s `1.0.0` must satisfy the `version = "1.0"` caret in `subx-cli`'s dependency line, and must keep satisfying it every time the pointer moves. Decision 8 covers enforcement.

### Decision 7: "Proven wired" means three independent layers, and the trivial reference is the load-bearing one

An empty crate that nothing references is not proof of anything. It would compile whether or not the submodule was checked out in CI, whether or not the dependency line was correct, and whether or not the version requirement was satisfiable — because Cargo would have no reason to look. B1 would then be a change that claims to build a skeleton and actually only creates directories, with the first real verification happening in B2 under a 46k-LOC diff.

Three layers, each catching something the others do not:

1. **Workspace member resolution.** `members = [".", "subx-core"]` means Cargo must read `subx-core/Cargo.toml` before it can do anything at all. An uninitialised submodule fails here, at manifest-load time, before compilation starts. This is the layer that catches a CI job whose checkout lacks `submodules: recursive`. It fires even if nothing depends on the crate.
2. **The compile-time reference.** `src/lib.rs` gains

   ```rust
   /// Version of the `subx-core` library this build is linked against.
   pub const CORE_VERSION: &str = subx_core::VERSION;
   ```

   next to the existing `pub const VERSION` (`src/lib.rs:114`). This forces `subx-core` to actually be *built and linked*, not merely parsed as a member. It catches a mistyped or missing `[dependencies]` entry, a crate name/`extern` mismatch, and a `subx-core` that parses but does not compile. A `const` initialised from another crate's `const` is the smallest construct that does this: no runtime cost, no new dependency edge in the public API, nothing to keep in sync.
3. **The version-agreement test.** A unit test in `src/lib.rs`'s existing `#[cfg(test)]` module asserts `CORE_VERSION` is non-empty and that its major component is `1` — the same major the `version = "1.0"` caret admits. Inside a workspace, `path` wins outright and the `version` key is never consulted for resolution, so nothing else in the build will ever notice if the pinned core drifts to `2.0.0` while `subx-cli` still asks for `^1.0`. This test is the only thing standing between that drift and a `publish-crates` failure. It is the local half of the CI assertion C1 adds (Decision 8).

**Why `CORE_VERSION` and not `pub use subx_core::VERSION;`.** `subx_cli::VERSION` already exists and means the *CLI's* version (`src/lib.rs:114`, `env!("CARGO_PKG_VERSION")`, covered by tests at `:569` and `:574`). A re-export would collide with it, and aliasing it would silently change what `subx_cli::VERSION` reports the moment the two version lines diverge — which they are guaranteed to do at C1, when one crate is 1.0.0 and the other 2.0.0. Two distinctly named constants, two distinct meanings.

`CORE_VERSION` is not wired into `subx-cli --version` output. The `--version` string is asserted by the `assert_cmd` suite and by `scripts/install.sh`'s smoke test in the release workflow (`release.yml:164-176`); changing it would be an observable CLI change, and this change makes none.

### Decision 8: Pointer staleness — what B1 installs, and what it deliberately leaves to C1

The submodule pointer is a manually maintained integer that can silently be wrong. Four mitigations exist; B1 installs the two that are structural and cheap, and hands the two that are CI machinery to C1.

**Installed here:**

- **`.gitmodules` records `branch = main`.** Written by `git submodule add -b main`. It does not make anything automatic, but it names the tracking branch so `git submodule update --remote` has a defined meaning and so Dependabot's `gitsubmodules` ecosystem knows what to compare against. Without it, "the pointer is behind" has no referent.
- **`git config submodule.recurse true`, documented for contributors.** Makes `git pull` and `git checkout` update the submodule working tree to whatever the pointer says. It is a per-clone setting, so it can only be recommended, never enforced — and it notably does **not** cover `git clone`, which is why `--recurse-submodules` is documented separately. It protects against the common failure (a contributor whose worktree silently holds yesterday's core) rather than the rare one.

**Deferred to C1, and named here so they are not forgotten:**

- **A Dependabot `package-ecosystem: gitsubmodules` entry.** The repository has no `.github/dependabot.yml` at all today. Creating one is a CI-configuration change, and C1 is the change that owns CI configuration; adding it in B1 would mean opening a new config surface inside a change whose CI contract is "seven checkout steps and nothing else". It also has a prerequisite this change cannot satisfy: it is only useful once core commits happen independently, which starts at B2.
- **A CI assertion that the pinned core commit's version matches the dependency requirement.** The job reads `version` out of `subx-core/Cargo.toml` at the checked-out pointer and checks it against the `version = "1.0"` requirement in `subx-cli`'s dependency line, failing the build on a mismatch. This is the CI counterpart of Decision 7's local unit test. It belongs with C1's publish rework because it is a release-correctness gate: the symptom it prevents is a `cargo publish` that resolves to a registry `subx-core` different from the one the pointer built against.

The residual risk between B1 and C1 is bounded: core holds one `const`, so the only drift possible is a version bump nobody made.

### Decision 9: The CI diff is seven `submodules: recursive` lines and two `fetch-depth: 0` lines — nothing more

Every job in both workflows either builds, tests, audits, measures or publishes the workspace, and all five of those operations load the workspace manifest. Without the submodule the directory is empty, `subx-core/Cargo.toml` does not exist, and the job dies at manifest load. So `submodules: recursive` goes on all seven checkout steps, with no exceptions worth carving out — including `security`, whose `actions-rust-lang/audit` resolves against the workspace lockfile (SDR §7), and `create-release`, which does not build but is the job whose checkout the `build` matrix's assumptions are read against.

`recursive` rather than `true` because the flag costs nothing today (`subx-core` has no submodules of its own) and is correct if it ever gains one. No token is needed: `subx-core` is public over HTTPS, so the default `GITHUB_TOKEN` credential path is not exercised.

`fetch-depth: 0` is added to exactly two jobs, both in `release.yml`:

- **`create-release`** extracts release notes by `awk`-ing `CHANGELOG.md` for the tag's version section (`release.yml:20-34`) and is the job that establishes the tag→release relationship. The file itself is present at depth 1, but full history is what makes the tag's ancestry — and any future `git describe`-style check — meaningful.
- **`publish-crates`** runs `cargo publish`, which since Cargo 1.39 walks submodules recursively for uncommitted changes and stamps VCS metadata into `.cargo_vcs_info.json`. A shallow parent and a shallow submodule make that check unreliable.

The other five jobs stay at the default depth; deepening them would slow every PR for no gain.

Everything else in SDR §7 is C1's and is not touched here: the single-line `cargo publish` at `release.yml:196` stays exactly as it is, the four `hashFiles('**/Cargo.lock')` cache keys stay as they are, `.llvm-cov.toml`'s relative excludes stay as they are, and `subx-core` gets no workflow of its own.

## Risks / Trade-offs

- **Risk: a contributor clones without `--recurse-submodules` and gets a Cargo error that never says "submodule".** → Mitigation: the clone and repair commands (`git clone --recurse-submodules`, `git submodule update --init --recursive`, `git config submodule.recurse true`) go into `README.md`, `README.zh-TW.md` and `AGENTS.md` in this change's documentation phase, and `subx-core`'s own `README.md` states that it is normally consumed through `subx-cli`.
- **Risk: the submodule pointer goes stale and `subx-cli` silently builds against an old core.** → Mitigation: `branch = main` in `.gitmodules` plus documented `submodule.recurse true` here; Dependabot `gitsubmodules` and the CI version assertion in C1 (Decision 8). Between B1 and C1 the exposure is one `const`.
- **Risk: `subx-core` accidentally acquires workspace inheritance during B2's manifest authoring, and nobody notices because the workspace build is green.** → Mitigation: the `crate-topology` spec states the prohibition normatively with both halves of the reasoning, and this change's task list includes a standalone clone-and-build **outside** the `subx-cli` tree as an explicit verification step that B2 can repeat verbatim.
- **Risk: `cargo publish` fails at the first post-split tag because the pointer was dirty or the dependency lacked a `version` key.** → Mitigation: the dual `{ path, version }` form is established three changes early (Decision 4); the dirty-pointer rule and the `--allow-dirty` prohibition are written into the spec; the publish flow itself is designed in C1 with a `--dry-run` gate (SDR §12 batch 6).
- **Risk: `subx-core` is invisible to `cargo clippy` and `cargo nextest run` because `default-members` defaults to the root package, so a lint or test regression in core goes unnoticed.** → Mitigation: at B1 core holds one `const` and no tests, and the task list verifies `cargo clippy --workspace -- -D warnings` by hand. Widening `scripts/quality_check.sh` is B3/C1 work, gated on core actually holding tests.
- **Risk: the two `[lints.*]` blocks drift apart, so a lint that is `deny` in one crate is absent in the other.** → Mitigation: accepted deliberately as the price of Decision 3. The `crate-topology` spec pins the duplication as a requirement, so a drifting block is a spec violation rather than an oversight; C3's documentation sweep re-checks both manifests.
- **Trade-off: cache keys churn once.** `hashFiles('**/Cargo.lock')` starts globbing `subx-core/Cargo.lock` (Decision 5b), invalidating the four caches in `build-test-audit-coverage.yml` on the first run after this change. One slow CI run, then normal.
- **Trade-off: a standalone `subx-core` release build is unoptimised.** No `[profile.*]` in the member manifest (Decision 2). Irrelevant in every configuration that ships anything.

## Migration Plan

The order matters, and each step leaves the tree in a state that either builds or fails loudly.

1. **Create the `subx-core` repository and its initial commit, on GitHub, before touching `subx-cli`.** `git submodule add` needs a reachable remote with at least one commit on `main`. This is the step SDR §5 of the implementation plan lists as the precondition for batch 3.
2. **Add the submodule and commit the gitlink.** At this point `subx-cli` still builds, because nothing references the new directory yet.
3. **Add the `[workspace]` table.** The first moment an uninitialised submodule becomes fatal. Verify by running `cargo metadata` and confirming both members are listed.
4. **Add the dependency and the `CORE_VERSION` reference.** The first moment `subx-core` is actually compiled and linked.
5. **Add `submodules: recursive` to the seven checkout steps and `fetch-depth: 0` to the two release jobs.** Ordered after step 4 so that a CI run at this point exercises a tree that already needs the submodule — a green run is then evidence, not coincidence.
6. **Verify from a clean clone**, both `git clone --recurse-submodules` of `subx-cli` and a bare `git clone` of `subx-core` into a scratch directory outside the tree.

**Rollback** is cheap up to step 3 (`git rm subx-core`, delete `.gitmodules`) and cheap after it too, since steps 3–5 are three small manifest/workflow edits with no code motion behind them. Nothing in this change is one-way except the existence of the GitHub repository itself.

## Open Questions

- **Does `subx-core` want a `CHANGELOG.md` from its first commit, or from B2 when it first contains code?** Deferred to C3, which owns the changelog rewrite across both repositories. B1 ships `README.md` and `AGENTS.md` only.
- **Should the `crate-topology` capability eventually live in `subx-core/openspec/specs/` as well, or stay `subx-cli`-side only?** It describes the relationship between the two repositories, so it plausibly belongs to both. C2a owns the capability allocation and is the right place to settle it; nothing in B1 forecloses either answer.
- **Will `subx-core`'s CI want the `ci` nextest profile's 300s slow-timeout, or a tighter one?** Core's test suite does not exist until B3, and the timeout matters mainly for the bundled-asset VAD test (`build-test-audit-coverage.yml:136-139`). B1 copies the profiles verbatim; C1 tunes them against a real suite.
