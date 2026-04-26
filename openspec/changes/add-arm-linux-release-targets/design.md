## Context

SubX-CLI publishes pre-built binaries through the GitHub Releases attached
to `v*` tags. The release workflow (`.github/workflows/release.yml`) currently
emits four artifacts: `subx-linux-x86_64`, `subx-windows-x86_64.exe`,
`subx-macos-x86_64`, and `subx-macos-aarch64`. The companion installer
(`scripts/install.sh`) detects host OS via `uname -s` and architecture via
`uname -m`, normalizes architectures to `x86_64` or `aarch64`, and constructs
asset names of the form `subx-<platform>-<arch>`. As a result, ARM64 Linux
hosts (Raspberry Pi, AWS Graviton, Oracle Ampere, ARM containers) request
`subx-linux-aarch64`, which the release never produces.

There is no existing OpenSpec capability covering release-artifact
distribution; `supply-chain-hardening` only governs dependency advisories
and minimal feature sets, not packaging. This change introduces a
`release-distribution` capability so the artifact contract is explicit and
testable.

Stakeholders: end users running the installer on ARM64 Linux, container
image authors using Alpine/musl, package maintainers needing predictable
asset URLs, and release maintainers who run the workflow.

## Goals / Non-Goals

**Goals:**

- Publish `aarch64-unknown-linux-gnu` artifacts for every tagged release so
  `scripts/install.sh` succeeds on ARM64 Linux without manual steps.
- Establish a documented, versioned contract between release asset names
  and installer URL construction.
- Provide musl static artifacts (`x86_64-unknown-linux-musl`,
  `aarch64-unknown-linux-musl`) for users on glibc-incompatible distros
  (Alpine, distroless, BusyBox-based images).
- Smoke-test every Linux artifact in CI before it is uploaded so we never
  publish a broken aarch64 binary.
- Improve installer error messaging when the requested asset is missing.

**Non-Goals:**

- ARM Linux 32-bit (`armv7-unknown-linux-gnueabihf`). Demand is low and
  cross-compilation toolchain risk is high; can be added later.
- Windows on ARM64. Requires separate signing/runner work.
- FreeBSD, NetBSD, illumos, or other non-tier-1 platforms.
- Package-manager distribution (Homebrew formulae, deb/rpm, AUR,
  containers). Out of scope; covered separately.
- Reproducible-build attestation / SLSA provenance. Tracked elsewhere.
- Changes to `crates.io` publishing.

## Decisions

### Decision 1: Use `cross` (or QEMU + native gcc) on `ubuntu-latest`, not native ARM runners

GitHub-hosted Linux ARM runners (`ubuntu-24.04-arm`, `ubuntu-22.04-arm`)
became generally available for public repositories in January 2025 at no
cost. Despite that, cross-compilation on the existing `ubuntu-latest`
(x86_64) runner remains the better choice for this change because:

- All other Linux artifacts (gnu and musl x86_64) already build on
  `ubuntu-latest`; using a single runner family keeps the matrix and the
  toolchain setup uniform.
- Cross-compilation is faster and avoids spinning up a second runner pool;
  matrix wall-clock time stays bounded.
- It matches how most Rust projects ship ARM Linux today and lets us
  validate aarch64 binaries via QEMU on the same job that built them.

**Choice**: Use [`cross`](https://github.com/cross-rs/cross) when its Docker
images cover the target, and fall back to a native `cargo build` with
`gcc-aarch64-linux-gnu` plus `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=
aarch64-linux-gnu-gcc` when `cross` is unavailable.

**Alternative considered**: Use native ARM runners
(`ubuntu-24.04-arm` / `ubuntu-22.04-arm`). Rejected for now because the
benefit (no QEMU smoke test) does not outweigh the cost of maintaining a
second runner OS family in the matrix. Can be revisited if `cross`
upstream regresses or if QEMU smoke tests prove flaky in practice.

**Alternative considered**: Use Docker `buildx` with `--platform=linux/arm64`
plus a Rust container. Rejected because it adds container build time per
release and duplicates what `cross` already provides.

### Decision 2: Smoke-test cross-compiled artifacts via QEMU user-mode

Building does not prove the binary runs. We add a post-build step that
runs `./<asset> --version` on every Linux artifact:

- For natively-built x86_64 artifacts: run directly on the runner.
- For cross-compiled aarch64 artifacts: install `qemu-user-static`
  (or use [`uraimo/run-on-arch-action`](https://github.com/uraimo/run-on-arch-action))
  and execute through QEMU.

Failure of the smoke test fails the workflow before the artifact is
uploaded — the release will simply not include a broken asset.

**Alternative considered**: Skip the smoke test and rely on user reports.
Rejected: this is exactly the failure mode that motivated the change.

### Decision 3: musl artifacts are produced but installer defaults to gnu

Most users want the gnu binary (smaller, dynamically linked, faster
startup). musl is essential for Alpine and a small set of container
images. The installer defaults to `linux-<arch>` (gnu) and only switches
to the musl asset when the user opts in via:

- `SUBX_LIBC=musl` environment variable, or
- a positional/`--musl` flag passed to the installer, or
- automatic detection: when `ldd --version 2>&1 | grep -qi musl` succeeds,
  prefer musl. Detection is best-effort; an explicit env var always wins.

**Alternative considered**: Always default to musl. Rejected: musl static
binaries are larger and have known performance trade-offs (memory
allocator, DNS resolver) that surprise glibc users.

**Alternative considered**: Only ship musl. Rejected: would change behavior
for every existing Linux user.

### Decision 4: Asset naming preserves all current names

`subx-linux-x86_64`, `subx-macos-x86_64`, `subx-macos-aarch64`, and
`subx-windows-x86_64.exe` keep their existing names. New names use the
same `subx-<platform>-<arch>[-<libc>]` pattern:

- `subx-linux-aarch64` (gnu, default)
- `subx-linux-x86_64-musl` (optional musl)
- `subx-linux-aarch64-musl` (optional musl)

The `-musl` suffix is appended only when libc differs from the default
(gnu). This rule is documented in the spec so the installer URL builder
can rely on it.

### Decision 5: Installer error messaging lists available assets, exact-match URL selection

When the constructed `BINARY_NAME` is not found in the latest release JSON,
the installer SHALL print:

1. The platform/arch it detected.
2. The asset name it looked for.
3. The list of available asset names (parsed from the same JSON
   response).
4. A pointer to `https://github.com/jim60105/subx-cli/releases` for
   manual download.

This converts a silent dead-end into actionable output.

URL selection MUST use exact asset-name matching (e.g., `jq` with
`select(.name == "<expected>")`, or basename string-equality on
`browser_download_url`). Substring/prefix matching such as
`grep "$BINARY_NAME"` is forbidden because `subx-linux-x86_64` is a
substring of `subx-linux-x86_64-musl` and would silently install the
wrong libc variant on a gnu host. Tests covering the gnu/musl pair on
both architectures are required (see tasks 3.4a/3.4b).

### Decision 6: Release workflow build matrix

The matrix gains entries for the new targets. Final matrix (Linux subset
shown):

| os | target | asset_name | builder |
|---|---|---|---|
| ubuntu-latest | x86_64-unknown-linux-gnu | subx-linux-x86_64 | cargo (native) |
| ubuntu-latest | aarch64-unknown-linux-gnu | subx-linux-aarch64 | cross |
| ubuntu-latest | x86_64-unknown-linux-musl | subx-linux-x86_64-musl | cross |
| ubuntu-latest | aarch64-unknown-linux-musl | subx-linux-aarch64-musl | cross |

macOS and Windows entries are unchanged.

`archive-rar` feature parity is required across all four Linux
artifacts (gnu and musl, x86_64 and aarch64). `unrar` is pure C and is
expected to cross-compile cleanly under both gnu and musl when the
chosen `cross` image (or native cross toolchain) ships the right
C toolchain. If a future build environment makes that infeasible for a
specific target, the divergence MUST be documented by updating the
`archive-rar feature parity across Linux artifacts` requirement and
the changelog rather than silently dropping the feature.

## Risks / Trade-offs

- [Risk] `cross` Docker image lag behind Rust stable → Mitigation: pin to a
  known-good `cross` version in the workflow, and fall back to native
  cross-toolchain (`gcc-aarch64-linux-gnu`) if `cross` fails.
- [Risk] QEMU smoke test is slow or flaky → Mitigation: only run
  `--version`, which exercises argument parsing and library load without
  any heavy I/O. Set a short timeout (30s) so a hang fails fast.
- [Risk] musl static binary size or runtime regression surprises users →
  Mitigation: ship musl as opt-in only; document size/performance caveats
  in README.
- [Risk] `unrar` (the C archive lib used by `archive-rar`) fails to build
  under musl → Mitigation: if it does, drop `archive-rar` from musl
  builds with a clearly documented feature-set difference; `archive-rar`
  is already optional.
- [Risk] Older glibc on the runner produces a binary that won't load on
  older user systems → Mitigation: musl artifacts cover that scenario;
  document the minimum glibc shipped with the gnu artifact.
- [Risk] Increased release wall-clock time → Mitigation: matrix jobs run
  in parallel; expected delta is one cross-compile (~5–10 minutes).
- [Risk] Installer libc auto-detection misclassifies a host → Mitigation:
  explicit `SUBX_LIBC` env var and `--musl` flag always override
  auto-detection.

## Migration Plan

1. Land workflow + installer + docs changes on `master` behind no
   feature flag (the changes are additive). As part of this step, gate
   the `publish-crates` job in `.github/workflows/release.yml` so it
   only runs on stable semver tags (skip tags containing `-`, e.g.,
   `-rc.N`, `-beta.N`). Without that gate, the RC validation in
   step 2 would attempt to publish a pre-release version to crates.io,
   which is undesirable. If the gate cannot be added, validate the
   matrix instead via a fork or a manually-triggered `workflow_dispatch`
   run that omits `publish-crates`, or replace `cargo publish` with
   `cargo publish --dry-run` and perform the real publish via a
   separate manual workflow.
2. Cut a release candidate tag (e.g., `vX.Y.Z-rc.1`) — only after
   step 1's `publish-crates` gating is merged — to exercise the new
   matrix end-to-end; verify all assets are present and pass smoke
   tests, and confirm `publish-crates` was skipped for the RC tag.
3. Tag the official release once the RC succeeds.
4. Rollback strategy: if a new artifact is broken, delete just the
   offending asset from the GitHub Release page and re-run only that
   matrix job; existing assets stay intact.

## Open Questions

- Should we also publish SHA256 checksums and a `SHA256SUMS` file
  alongside the binaries? (Recommended but technically separate from
  this change; can be added in tasks if desired.)
- Should `unrar` (the C lib enabling `archive-rar`) be disabled for musl
  builds if it fails to compile? **Decided**: parity is required across
  all four Linux artifacts (see `archive-rar feature parity` requirement
  and Decision 6). If the first cross build proves infeasible, update
  the requirement and changelog rather than silently dropping it.
- Is `uraimo/run-on-arch-action` preferred over plain `qemu-user-static`?
  Both work; pick during implementation based on log clarity.
