#!/bin/bash
# scripts/install.sh
#
# Copyright (C) 2025 陳鈞
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SubX CLI installation script
# Automatically detects platform and downloads the latest release.
#
# Linux musl/Alpine note:
# As of v1.7.2 the `*-unknown-linux-musl` artifacts are NOT published.
# Upstream ONNX Runtime (consumed via `ort`) does not ship musl prebuilts
# and source-building it on every release is out of scope. The legacy
# `--musl` flag and `SUBX_LIBC=musl` selector are still parsed for clear
# error messaging, but they exit non-zero with a pointer at
# `cargo install subx-cli`. Auto-detection on musl hosts also exits with
# the same guidance instead of silently 404'ing.

set -e

RELEASES_PAGE_URL="https://github.com/jim60105/subx-cli/releases"
RELEASE_API_URL="https://api.github.com/repos/jim60105/subx-cli/releases/latest"

# ---------------------------------------------------------------------------
# Pure helper functions (sourced by scripts/test_install.sh for unit tests).
# Keep these free of network calls and global side effects.
# ---------------------------------------------------------------------------

# usage: print the installer usage line.
usage() {
    cat <<'EOF'
Usage: install.sh [--musl] [--help]

Options:
  --musl       (Deprecated, no-op apart from a clear error.) musl
               artifacts are no longer published; install from source
               via `cargo install subx-cli` on Alpine and other musl
               distros.
  --help, -h   Show this help message and exit.

Environment:
  SUBX_LIBC    Either `gnu` (default) or `musl`. `musl` is rejected
               with a pointer at the from-source workflow because no
               musl artifact is published. Ignored on macOS.
EOF
}

# compute_binary_name <platform> <arch> <libc>
#
# Build the release asset name following the documented contract:
#   linux/gnu      -> subx-linux-<arch>
#   linux/musl     -> subx-linux-<arch>-musl
#   macos/<arch>   -> subx-macos-<arch>          (libc is ignored)
#   windows/x86_64 -> subx-windows-x86_64.exe    (libc is ignored)
#
# IMPORTANT: `subx-linux-x86_64` is a substring of `subx-linux-x86_64-musl`.
# Substring/prefix matching against asset names is FORBIDDEN — see
# select_download_url below for the exact-match selection logic.
compute_binary_name() {
    local platform="$1"
    local arch="$2"
    local libc="$3"

    case "$platform" in
        linux)
            if [ "$libc" = "musl" ]; then
                printf 'subx-linux-%s-musl' "$arch"
            else
                printf 'subx-linux-%s' "$arch"
            fi
            ;;
        macos)
            printf 'subx-macos-%s' "$arch"
            ;;
        windows)
            printf 'subx-windows-%s.exe' "$arch"
            ;;
        *)
            return 1
            ;;
    esac
}

# extract_asset_names <release_json>
#
# Print, one per line, the basename of every browser_download_url found in
# the release JSON. Used both for exact-match URL selection and for the
# fallback diagnostics output.
extract_asset_names() {
    local release_json="$1"
    if command -v jq >/dev/null 2>&1; then
        printf '%s' "$release_json" | jq -r '.assets[].name' 2>/dev/null
    else
        printf '%s\n' "$release_json" \
            | grep -o '"browser_download_url":[[:space:]]*"[^"]*"' \
            | sed -E 's/.*"([^"]*)".*/\1/' \
            | while IFS= read -r url; do
                [ -z "$url" ] && continue
                printf '%s\n' "${url##*/}"
            done
    fi
}

# select_download_url <release_json> <binary_name>
#
# Select the browser_download_url for the asset whose name equals
# <binary_name> EXACTLY. Substring matches such as
# `grep "$BINARY_NAME"` are forbidden because `subx-linux-x86_64` is a
# substring of `subx-linux-x86_64-musl` and a loose match would silently
# install the wrong libc variant on a gnu host.
#
# Prints the URL on stdout and returns 0 if found; prints nothing and
# returns 1 if no asset matches exactly.
select_download_url() {
    local release_json="$1"
    local binary_name="$2"
    local url=""

    if command -v jq >/dev/null 2>&1; then
        url=$(printf '%s' "$release_json" \
            | jq -r --arg name "$binary_name" \
                '.assets[] | select(.name == $name) | .browser_download_url' \
            2>/dev/null)
    else
        # Fallback: parse browser_download_url lines and string-equal
        # compare basenames. NEVER substring-match.
        while IFS= read -r candidate; do
            [ -z "$candidate" ] && continue
            local base="${candidate##*/}"
            if [ "$base" = "$binary_name" ]; then
                url="$candidate"
                break
            fi
        done < <(printf '%s\n' "$release_json" \
            | grep -o '"browser_download_url":[[:space:]]*"[^"]*"' \
            | sed -E 's/.*"([^"]*)".*/\1/')
    fi

    if [ -z "$url" ] || [ "$url" = "null" ]; then
        return 1
    fi
    printf '%s\n' "$url"
    return 0
}

# print_missing_asset_diagnostics <platform> <arch> <binary_name> <release_json>
#
# Emit actionable diagnostics when the requested asset is not present in
# the latest release JSON. Always returns 1 so callers can `|| exit` on it.
print_missing_asset_diagnostics() {
    local platform="$1"
    local arch="$2"
    local binary_name="$3"
    local release_json="$4"

    {
        echo "Error: Could not find a release asset matching the requested name."
        echo "  Detected platform : $platform"
        echo "  Detected arch     : $arch"
        echo "  Searched asset    : $binary_name"
        echo "  Available assets  :"
        local available
        available=$(extract_asset_names "$release_json")
        if [ -z "$available" ]; then
            echo "    (none found in release JSON)"
        else
            printf '%s\n' "$available" | sed 's/^/    - /'
        fi
        echo "  Releases page     : $RELEASES_PAGE_URL"
    } >&2
    return 1
}

# detect_libc_auto: print `musl` if Linux ldd reports musl, else `gnu`.
# Best-effort only; explicit env/flag override always wins. The caller
# (main) treats a `musl` result as a fatal "unsupported" error because
# v1.7.2+ no longer publishes musl artifacts.
detect_libc_auto() {
    if [ "$(uname -s | tr '[:upper:]' '[:lower:]')" != "linux" ]; then
        printf 'gnu\n'
        return 0
    fi
    if ldd --version 2>&1 | grep -qi musl; then
        printf 'musl\n'
    else
        printf 'gnu\n'
    fi
}

# musl_unsupported_exit: print the canonical guidance and exit non-zero.
# Centralised so explicit env/flag overrides and auto-detection report
# the same message when the user is on a musl host.
musl_unsupported_exit() {
    {
        echo "Error: musl Linux artifacts are not published for SubX-CLI."
        echo
        echo "Upstream ONNX Runtime does not ship musl prebuilts, so the"
        echo "release pipeline cannot produce statically-linked musl"
        echo "binaries. To install on Alpine and other musl distros:"
        echo
        echo "    cargo install subx-cli"
        echo
        echo "Or run the gnu artifact inside a glibc-compatible container."
    } >&2
    exit 2
}

# ---------------------------------------------------------------------------
# Main entry point. Skipped when this script is sourced by the test harness.
# ---------------------------------------------------------------------------

main() {
    local libc_flag=""

    # Argument parsing. Unknown arguments must NOT be silently ignored.
    while [ $# -gt 0 ]; do
        case "$1" in
            --musl)
                libc_flag="musl"
                shift
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                echo "Error: Unknown argument: $1" >&2
                usage >&2
                exit 2
                ;;
        esac
    done

    # libc resolution: env var > flag > auto-detection > default (gnu).
    # `musl` is rejected up-front because v1.7.2+ no longer publishes
    # musl artifacts; surface a helpful message instead of letting the
    # download step 404 against GitHub Releases.
    local libc=""
    if [ -n "${SUBX_LIBC:-}" ]; then
        case "$SUBX_LIBC" in
            gnu) libc="gnu" ;;
            musl) musl_unsupported_exit ;;
            *)
                echo "Error: SUBX_LIBC must be 'gnu' or 'musl' (got: '$SUBX_LIBC')" >&2
                exit 2
                ;;
        esac
    elif [ "$libc_flag" = "musl" ]; then
        musl_unsupported_exit
    fi

    # Detect operating system and architecture.
    local OS ARCH PLATFORM
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    case "$ARCH" in
        x86_64) ARCH="x86_64" ;;
        arm64|aarch64) ARCH="aarch64" ;;
        *) echo "Error: Unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac

    case "$OS" in
        linux) PLATFORM="linux" ;;
        darwin) PLATFORM="macos" ;;
        *) echo "Error: Unsupported operating system: $OS" >&2; exit 1 ;;
    esac

    # Auto-detect libc on Linux when no explicit selection was made.
    # If the host is musl, exit with the same guidance as the explicit
    # override path — there is no published artifact to download.
    if [ "$PLATFORM" = "linux" ] && [ -z "$libc" ]; then
        local detected
        detected=$(detect_libc_auto)
        if [ "$detected" = "musl" ]; then
            musl_unsupported_exit
        fi
        libc="$detected"
    fi
    # On non-Linux, libc is meaningless; force gnu so compute_binary_name
    # doesn't append a -musl suffix.
    if [ "$PLATFORM" != "linux" ]; then
        libc="gnu"
    fi

    echo "Detected platform: $PLATFORM ($ARCH${libc:+, libc=$libc})"

    local BINARY_NAME
    BINARY_NAME=$(compute_binary_name "$PLATFORM" "$ARCH" "$libc")

    echo "Downloading SubX latest version..."

    # `curl -fsSL` makes non-2xx responses fail (network-error path).
    local RELEASE_JSON
    if ! RELEASE_JSON=$(curl -fsSL "$RELEASE_API_URL"); then
        echo "Error: Failed to fetch release metadata from GitHub." >&2
        echo "  URL: $RELEASE_API_URL" >&2
        echo "  Check your network connection or the GitHub API status." >&2
        exit 1
    fi
    if [ -z "$RELEASE_JSON" ]; then
        echo "Error: GitHub API returned an empty response." >&2
        echo "  URL: $RELEASE_API_URL" >&2
        exit 1
    fi

    local DOWNLOAD_URL
    if ! DOWNLOAD_URL=$(select_download_url "$RELEASE_JSON" "$BINARY_NAME"); then
        print_missing_asset_diagnostics "$PLATFORM" "$ARCH" "$BINARY_NAME" "$RELEASE_JSON" || true
        exit 1
    fi

    echo "Download URL: $DOWNLOAD_URL"
    if ! curl -fsSL "$DOWNLOAD_URL" -o subx-cli; then
        echo "Error: Download failed" >&2
        exit 1
    fi

    if [ ! -f "subx-cli" ]; then
        echo "Error: Download failed" >&2
        exit 1
    fi

    chmod +x subx-cli

    # Install to system path
    echo "Installing to system path..."
    if [ "${EUID:-$(id -u)}" -eq 0 ]; then
        mv subx-cli /usr/local/bin/
    else
        sudo mv subx-cli /usr/local/bin/
    fi
    echo "SubX has been installed to /usr/local/bin/subx-cli"

    echo "Installation complete! Run 'subx-cli --help' to get started"
}

# Only run main() when the script is executed directly. When sourced (e.g.
# by scripts/test_install.sh), the helper functions above are exposed but
# main() does not run, so no network calls happen during tests.
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    main "$@"
fi
