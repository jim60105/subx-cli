## 1. Verify implementation already shipped in v1.7.2

The implementation half of this change shipped in v1.7.2 (commit
`6b35136`, tag `v1.7.2`). The tasks below verify that the shipped
implementation matches the spec delta in this change. They MUST all
pass before the change is archived.

- [x] 1.1 Confirm `Cross.toml` contains no `[target.*-unknown-linux-musl]` section
- [x] 1.2 Confirm `.github/workflows/release.yml` matrix has no `*-unknown-linux-musl` entries
- [x] 1.3 Confirm `.github/workflows/release.yml` smoke-test step exports `QEMU_LD_PREFIX=/usr/aarch64-linux-gnu` and installs `gcc-aarch64-linux-gnu`
- [x] 1.4 Confirm `scripts/install.sh` defines a `musl_unsupported_exit` helper that prints `cargo install subx-cli` guidance and exits with status `2`
- [x] 1.5 Confirm `scripts/install.sh` routes `SUBX_LIBC=musl`, `--musl`, and auto-detected musl hosts (`ldd --version` reports musl) to `musl_unsupported_exit`
- [x] 1.6 Confirm `scripts/install.sh` rejection happens BEFORE any `curl`/`wget`/HTTP call (no network hit on rejection)
- [x] 1.7 Confirm `scripts/test_install.sh` includes harness cases `g.4` (`SUBX_LIBC=musl`) and `g.5` (`--musl`) covering exit code `2`, the `not published` phrasing, and the `cargo install subx-cli` recommendation
- [x] 1.8 Confirm `README.md` install table omits the musl rows and includes a note pointing musl users at `cargo install subx-cli`
- [x] 1.9 Confirm `README.zh-TW.md` install table omits the musl rows and includes the equivalent zh-TW note
- [x] 1.10 Confirm `CHANGELOG.md` has a `[1.7.2]` `### Removed` entry documenting the musl artifact removal and naming `cargo install subx-cli` as the supported migration path

## 2. Validate the openspec change artifacts

- [x] 2.1 Run `openspec validate drop-musl-support --strict` and resolve any reported issues
- [x] 2.2 Run `openspec status --change drop-musl-support` and confirm all four artifacts (`proposal`, `design`, `specs`, `tasks`) report status `done`
- [x] 2.3 Run `openspec show drop-musl-support --type change --deltas-only --json | jq` and review the rendered delta against `openspec/specs/release-distribution/spec.md`; confirm the diff matches the intent in `proposal.md` (musl removed; new `Installer musl-input rejection` requirement; `archive-rar` parity scoped to two artifacts; documentation requirement points musl users at a source-build with locally provisioned ONNX Runtime)

## 3. Honest-migration corrections (post-rubber-duck round-1)

The first rubber-duck pass on this change identified two latent
implementation/spec mismatches in the v1.7.2 hot-fix that this change
must reconcile before archiving:

- [x] 3.1 Tighten installer precedence: `--musl` MUST always reject regardless of `SUBX_LIBC`. (`scripts/install.sh` updated to OR the two musl signals before resolving libc; `scripts/test_install.sh` adds case `g.6` for `SUBX_LIBC=gnu --musl` rejecting with `not published`. 33/33 tests pass.)
- [x] 3.2 Update `README.md` and `README.zh-TW.md` musl notes to drop the unqualified `cargo install subx-cli` recommendation and replace it with the honest source-build path (locally provisioned ONNX Runtime, `ORT_LIB_LOCATION` or equivalent `ort` configuration). Plain `cargo install subx-cli` still hits `ort`'s `download-binaries` default and fails on musl for the same upstream reason as the release pipeline did.
- [x] 3.3 Update `scripts/install.sh`'s `musl_unsupported_exit` message to match — it currently recommends bare `cargo install subx-cli`; reword to point at "build from source with `ORT_LIB_LOCATION` set to a local musl-compatible ONNX Runtime, e.g., from your distro packages or built from source."
- [x] 3.4 Update `CHANGELOG.md`'s `[1.7.2]` `### Removed` entry's "supported migration path" wording to match the README/installer wording (do NOT remove the entry — only refine the wording so it does not contradict the spec contract being added by this change).

## 4. Cross-check spec against shipped behavior

- [x] 4.1 Re-read each scenario in `specs/release-distribution/spec.md` (this change's delta) and confirm a corresponding code path exists in `scripts/install.sh` or `.github/workflows/release.yml` (smoke-test step, matrix, `musl_unsupported_exit` paths) — every scenario MUST be testable against shipped code (cross-checked via 1.1-1.10 plus 4.2 harness; 30 scenarios → matrix in release.yml, smoke step + QEMU sysroot, install.sh musl rejection paths, README/CHANGELOG wording)
- [x] 4.2 Run `bash scripts/test_install.sh` locally and confirm 33/33 cases pass (verifies the rejection contract from this change still holds, including the new precedence case `g.6`)
- [x] 4.3 Confirm the v1.7.2 GitHub Release page (`https://github.com/jim60105/subx-cli/releases/tag/v1.7.2`) contains zero `*-musl` assets (the only release-listing assertion in scope for this change; the *count* of remaining gnu/macOS/Windows assets is governed by the unrelated cross-build pipeline and is out of scope here)
