## 1. Release workflow: cross-compilation setup

- [x] 1.1 Add `aarch64-unknown-linux-gnu` entry to the build matrix in `.github/workflows/release.yml` (os: `ubuntu-latest`, asset_name: `subx-linux-aarch64`)
- [x] 1.2 Add `x86_64-unknown-linux-musl` matrix entry (asset_name: `subx-linux-x86_64-musl`)
- [x] 1.3 Add `aarch64-unknown-linux-musl` matrix entry (asset_name: `subx-linux-aarch64-musl`)
- [x] 1.4 Choose builder per target — gnu/musl ARM uses `cross`; musl x86_64 uses `cross` or native musl toolchain. Pin the `cross` version in the workflow.
- [x] 1.5 Add a fallback step that installs `gcc-aarch64-linux-gnu` and sets `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER` for native cross-compile when `cross` cannot be used
- [x] 1.6 Confirm `--features archive-rar` continues to compile for all four Linux targets (gnu and musl, x86_64 and aarch64). Choose `cross` images / toolchains that include the C toolchain required by `unrar`. Treat a musl build failure as a blocker for this change rather than silently dropping the feature; if dropping `archive-rar` for musl is unavoidable, update the `archive-rar feature parity across Linux artifacts` requirement and the changelog before merging.
- [x] 1.7 Ensure the matrix `include:` block carries a per-row `builder` (or `use_cross: true/false`) field so the build step can branch cleanly

## 2. Release workflow: smoke testing

- [x] 2.1 Add a "Smoke test" workflow step after `Build` and before `Upload Release Asset` for every Linux matrix row
- [x] 2.2 For native x86_64 Linux: run `./<asset_name> --version` directly with a 30s timeout
- [x] 2.3 For aarch64 Linux: install `qemu-user-static` (or use `uraimo/run-on-arch-action`) and run `./<asset_name> --version` under emulation with a 30s timeout
- [x] 2.4 Make smoke-test failure abort the workflow before `Upload Release Asset` runs
- [x] 2.5 Verify macOS and Windows matrix rows are unaffected by the new step (gate it on `runs-on: ubuntu-latest` only)

## 3. Installer changes (`scripts/install.sh`)

- [x] 3.1 Parse `--musl` (and `--help`) command-line flags without breaking unknown-arg behavior
- [x] 3.2 Read `SUBX_LIBC` env var; valid values `gnu` (default) and `musl`
- [x] 3.3 Implement best-effort musl auto-detection on Linux via `ldd --version 2>&1 | grep -qi musl`; explicit env var or flag overrides it
- [x] 3.4 Build the asset name as `subx-linux-<arch>` for gnu and `subx-linux-<arch>-musl` for musl; preserve macOS/Windows naming
- [x] 3.4a Select the download URL using exact asset-name matching against the GitHub Releases JSON. Prefer `jq -r '.assets[] | select(.name == "'"$BINARY_NAME"'") | .browser_download_url'`; if `jq` is unavailable, fall back to iterating `browser_download_url` values and comparing `basename` to `$BINARY_NAME` with a string-equality test. Do **not** use `grep "$BINARY_NAME"` or any substring/prefix match — `subx-linux-x86_64` is a substring of `subx-linux-x86_64-musl`.
- [x] 3.4b Add installer unit/integration tests (e.g., `bats`, `shellcheck`-friendly shell harness, or a small Rust test that shells out) that feed a fixture release JSON containing both `subx-linux-x86_64` and `subx-linux-x86_64-musl` and assert: (a) gnu host selects the gnu URL exactly, (b) `SUBX_LIBC=musl` selects the musl URL exactly, (c) aarch64 variants behave the same, (d) a missing asset triggers the fallback diagnostics path.
- [x] 3.5 When the asset URL is not found in the API response, print: detected platform/arch, searched asset name, list of available asset names from the JSON, link to releases page, then exit non-zero
- [x] 3.6 When the GitHub API request fails or returns empty, print a clear network/availability error and exit non-zero
- [x] 3.7 Verify x86_64 Linux, x86_64 macOS, and aarch64 macOS code paths still produce identical asset names to the pre-change installer
- [x] 3.8 Run `shellcheck scripts/install.sh` and resolve any findings introduced by the new code

## 4. Documentation

- [x] 4.1 Update the installation section in `README.md` to list every `(platform, arch, libc)` combination served by the installer
- [x] 4.2 Document how to opt into musl artifacts (`SUBX_LIBC=musl` env var or `--musl` flag) in `README.md`
- [x] 4.3 Mirror the same updates in `README.zh-TW.md`
- [x] 4.4 Update `docs/command-reference.md` (or add a brief release-targets section) listing supported binary downloads
- [x] 4.5 Add an `## [Unreleased]` `### Added` entry to `CHANGELOG.md` describing the new aarch64 Linux and musl artifacts and the installer libc selection
- [x] 4.6 Update `AGENTS.md` — its "CI/CD Pipeline" section currently says the release workflow "cross-compiles for 4 targets (Linux/Windows/macOS x86_64, macOS ARM64)"; revise the count and the target list to include the new aarch64 Linux gnu/musl and x86_64 Linux musl artifacts

## 5. Validation and release rollout

- [x] 5.0 Gate the `publish-crates` job in `.github/workflows/release.yml` so it runs **only** on stable semver tags (e.g., add a job-level `if` such as `if: ${{ !contains(github.ref_name, '-') }}` to skip pre-release tags like `vX.Y.Z-rc.1`, `-beta.N`, `-alpha.N`). Without this gate, pushing an RC tag would attempt to publish a pre-release version to crates.io, which is undesirable. Acceptable alternatives if the gate is rejected:
  - (a) replace `cargo publish` with `cargo publish --dry-run` and require a manual follow-up workflow_dispatch for the real publish, or
  - (b) skip the RC tag entirely and instead validate the matrix in a fork or via a manually-triggered `workflow_dispatch` of `release.yml` that omits the `publish-crates` job.
  Pick exactly one strategy and document it in `design.md` before pushing any pre-release tag.
- [ ] 5.1 With strategy 5.0 in place, push a draft / RC tag (e.g., `vX.Y.Z-rc.1`) **only after confirming `publish-crates` will be skipped**, and verify the workflow produces every expected asset (`subx-linux-x86_64`, `subx-linux-aarch64`, `subx-linux-x86_64-musl`, `subx-linux-aarch64-musl`, `subx-macos-x86_64`, `subx-macos-aarch64`, `subx-windows-x86_64.exe`). If strategy 5.0 (b) was chosen, validate via fork / `workflow_dispatch` instead of a real tag.
- [ ] 5.2 Download `subx-linux-aarch64` from the RC release on a real ARM64 host (or via Docker `--platform=linux/arm64`) and run `./subx-cli --version`
- [ ] 5.3 Run `bash scripts/install.sh` on an Alpine-based container with `SUBX_LIBC=musl` and confirm the musl asset installs and `subx-cli --version` works
- [x] 5.4 Run `bash scripts/install.sh` on x86_64 Ubuntu and confirm behavior is unchanged from the previous release
- [x] 5.5 Trigger an installer failure on purpose (e.g., temporarily set `BINARY_NAME=subx-bogus`) and confirm the new diagnostics output appears
- [x] 5.6 Run `scripts/quality_check.sh` on the main branch before tagging the official release (no Rust source changes are expected from this proposal, but verify nothing regressed)
- [ ] 5.7 Tag the official release once the RC validation passes; verify all Linux artifacts pass smoke tests in the workflow logs

## 6. Post-release follow-up

- [ ] 6.1 Monitor the next 1–2 releases for cross-compile or QEMU smoke-test flakiness; pin `cross` to a newer version if upstream regressions appear
- [ ] 6.2 Decide whether to add SHA256 checksums (`SHA256SUMS` file) as a follow-up change — out of scope here
- [ ] 6.3 Decide whether to add `armv7-unknown-linux-gnueabihf` (32-bit ARM) as a follow-up change — out of scope here
