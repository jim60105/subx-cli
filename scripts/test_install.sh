#!/bin/bash
# scripts/test_install.sh
#
# Copyright (C) 2025 陳鈞
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Unit-test harness for `scripts/install.sh`.
#
# This harness sources the pure helper functions from `install.sh`
# (`compute_binary_name`, `select_download_url`,
# `print_missing_asset_diagnostics`) and runs them against an inline
# fixture release JSON. It does NOT make any network calls.
#
# Usage: bash scripts/test_install.sh
# Exit code: 0 on success, non-zero on any failed assertion.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_SH="$SCRIPT_DIR/install.sh"

if [ ! -f "$INSTALL_SH" ]; then
    echo "Error: cannot find $INSTALL_SH" >&2
    exit 1
fi

# shellcheck source=install.sh disable=SC1091
source "$INSTALL_SH"

# Sandbox temp dir for any incidental file output.
TMPDIR_TEST="$(mktemp -d -p "$SCRIPT_DIR/..")"
trap 'rm -rf "$TMPDIR_TEST"' EXIT

PASSED=0
FAILED=0
FAILURES=()

# assert_eq <name> <expected> <actual>
assert_eq() {
    local name="$1" expected="$2" actual="$3"
    if [ "$expected" = "$actual" ]; then
        PASSED=$((PASSED + 1))
        echo "  ok  - $name"
    else
        FAILED=$((FAILED + 1))
        FAILURES+=("$name")
        echo "  FAIL- $name" >&2
        echo "    expected: $expected" >&2
        echo "    actual  : $actual" >&2
    fi
}

# assert_contains <name> <haystack> <needle>
assert_contains() {
    local name="$1" haystack="$2" needle="$3"
    case "$haystack" in
        *"$needle"*)
            PASSED=$((PASSED + 1))
            echo "  ok  - $name"
            ;;
        *)
            FAILED=$((FAILED + 1))
            FAILURES+=("$name")
            echo "  FAIL- $name" >&2
            echo "    needle not found: $needle" >&2
            echo "    in haystack:" >&2
            printf '%s\n' "$haystack" | sed 's/^/      /' >&2
            ;;
    esac
}

# Fixture: a release JSON containing all seven expected assets.
FIXTURE_JSON_FULL=$(cat <<'JSON'
{
  "tag_name": "v1.2.3",
  "assets": [
    {
      "name": "subx-linux-x86_64",
      "browser_download_url": "https://example.com/r/v1.2.3/subx-linux-x86_64"
    },
    {
      "name": "subx-linux-x86_64-musl",
      "browser_download_url": "https://example.com/r/v1.2.3/subx-linux-x86_64-musl"
    },
    {
      "name": "subx-linux-aarch64",
      "browser_download_url": "https://example.com/r/v1.2.3/subx-linux-aarch64"
    },
    {
      "name": "subx-linux-aarch64-musl",
      "browser_download_url": "https://example.com/r/v1.2.3/subx-linux-aarch64-musl"
    },
    {
      "name": "subx-macos-x86_64",
      "browser_download_url": "https://example.com/r/v1.2.3/subx-macos-x86_64"
    },
    {
      "name": "subx-macos-aarch64",
      "browser_download_url": "https://example.com/r/v1.2.3/subx-macos-aarch64"
    },
    {
      "name": "subx-windows-x86_64.exe",
      "browser_download_url": "https://example.com/r/v1.2.3/subx-windows-x86_64.exe"
    }
  ]
}
JSON
)

# Fixture: missing the requested asset (no aarch64 variants), so the
# diagnostics path should fire when asking for `subx-linux-aarch64`.
FIXTURE_JSON_MISSING=$(cat <<'JSON'
{
  "tag_name": "v1.2.3",
  "assets": [
    {
      "name": "subx-linux-x86_64",
      "browser_download_url": "https://example.com/r/v1.2.3/subx-linux-x86_64"
    },
    {
      "name": "subx-macos-x86_64",
      "browser_download_url": "https://example.com/r/v1.2.3/subx-macos-x86_64"
    }
  ]
}
JSON
)

run_case_with_jq() {
    local label="$1"
    echo
    echo "==> $label"
}

# -----------------------------------------------------------------------
# (a) gnu host: linux/x86_64 with no SUBX_LIBC -> exact `subx-linux-x86_64`
# -----------------------------------------------------------------------
run_case_with_jq "(a) Linux x86_64 gnu selects exact gnu URL"
name=$(compute_binary_name linux x86_64 gnu)
assert_eq "(a.1) compute_binary_name linux/x86_64/gnu" \
    "subx-linux-x86_64" "$name"
url=$(select_download_url "$FIXTURE_JSON_FULL" "$name")
assert_eq "(a.2) select_download_url -> exact gnu URL" \
    "https://example.com/r/v1.2.3/subx-linux-x86_64" "$url"
# Make sure we did NOT pick the musl variant.
case "$url" in
    *-musl) FAILED=$((FAILED + 1)); FAILURES+=("(a.3) gnu host must not match -musl"); echo "  FAIL- (a.3) gnu host must not match -musl" >&2 ;;
    *) PASSED=$((PASSED + 1)); echo "  ok  - (a.3) gnu host did not match -musl" ;;
esac

# -----------------------------------------------------------------------
# (b) SUBX_LIBC=musl on x86_64 -> exact `subx-linux-x86_64-musl`
# -----------------------------------------------------------------------
run_case_with_jq "(b) Linux x86_64 musl selects exact musl URL"
name=$(compute_binary_name linux x86_64 musl)
assert_eq "(b.1) compute_binary_name linux/x86_64/musl" \
    "subx-linux-x86_64-musl" "$name"
url=$(select_download_url "$FIXTURE_JSON_FULL" "$name")
assert_eq "(b.2) select_download_url -> exact musl URL" \
    "https://example.com/r/v1.2.3/subx-linux-x86_64-musl" "$url"

# -----------------------------------------------------------------------
# (c) aarch64 gnu -> exact `subx-linux-aarch64`
# -----------------------------------------------------------------------
run_case_with_jq "(c) Linux aarch64 gnu selects exact gnu URL"
name=$(compute_binary_name linux aarch64 gnu)
assert_eq "(c.1) compute_binary_name linux/aarch64/gnu" \
    "subx-linux-aarch64" "$name"
url=$(select_download_url "$FIXTURE_JSON_FULL" "$name")
assert_eq "(c.2) select_download_url -> exact aarch64 gnu URL" \
    "https://example.com/r/v1.2.3/subx-linux-aarch64" "$url"
case "$url" in
    *-musl) FAILED=$((FAILED + 1)); FAILURES+=("(c.3) aarch64 gnu must not match -musl"); echo "  FAIL- (c.3) aarch64 gnu must not match -musl" >&2 ;;
    *) PASSED=$((PASSED + 1)); echo "  ok  - (c.3) aarch64 gnu did not match -musl" ;;
esac

# -----------------------------------------------------------------------
# (d) aarch64 musl -> exact `subx-linux-aarch64-musl`
# -----------------------------------------------------------------------
run_case_with_jq "(d) Linux aarch64 musl selects exact musl URL"
name=$(compute_binary_name linux aarch64 musl)
assert_eq "(d.1) compute_binary_name linux/aarch64/musl" \
    "subx-linux-aarch64-musl" "$name"
url=$(select_download_url "$FIXTURE_JSON_FULL" "$name")
assert_eq "(d.2) select_download_url -> exact aarch64 musl URL" \
    "https://example.com/r/v1.2.3/subx-linux-aarch64-musl" "$url"

# -----------------------------------------------------------------------
# (e) Missing asset -> diagnostics function returns non-zero AND its
# output mentions the searched name + at least one available asset name +
# the releases page URL.
# -----------------------------------------------------------------------
run_case_with_jq "(e) Missing asset triggers fallback diagnostics"
missing_name="subx-linux-aarch64"
diag_output=$(print_missing_asset_diagnostics linux aarch64 "$missing_name" "$FIXTURE_JSON_MISSING" 2>&1 || true)
# The function must return non-zero.
if print_missing_asset_diagnostics linux aarch64 "$missing_name" "$FIXTURE_JSON_MISSING" >/dev/null 2>&1; then
    FAILED=$((FAILED + 1))
    FAILURES+=("(e.1) diagnostics should exit non-zero")
    echo "  FAIL- (e.1) diagnostics should exit non-zero" >&2
else
    PASSED=$((PASSED + 1))
    echo "  ok  - (e.1) diagnostics returned non-zero"
fi
assert_contains "(e.2) diagnostics mentions searched asset" \
    "$diag_output" "$missing_name"
assert_contains "(e.3) diagnostics lists an available asset" \
    "$diag_output" "subx-linux-x86_64"
assert_contains "(e.4) diagnostics points to releases page" \
    "$diag_output" "https://github.com/jim60105/subx-cli/releases"

# Confirm select_download_url itself returns non-zero on a miss.
if select_download_url "$FIXTURE_JSON_MISSING" "$missing_name" >/dev/null 2>&1; then
    FAILED=$((FAILED + 1))
    FAILURES+=("(e.5) select_download_url should fail on miss")
    echo "  FAIL- (e.5) select_download_url should fail on miss" >&2
else
    PASSED=$((PASSED + 1))
    echo "  ok  - (e.5) select_download_url returned non-zero on miss"
fi

# -----------------------------------------------------------------------
# (f) Backward compatibility: macOS and Windows asset names unchanged.
# -----------------------------------------------------------------------
run_case_with_jq "(f) macOS/Windows asset names unchanged"
assert_eq "(f.1) macOS x86_64" "subx-macos-x86_64" \
    "$(compute_binary_name macos x86_64 gnu)"
assert_eq "(f.2) macOS aarch64" "subx-macos-aarch64" \
    "$(compute_binary_name macos aarch64 gnu)"
assert_eq "(f.3) Windows x86_64" "subx-windows-x86_64.exe" \
    "$(compute_binary_name windows x86_64 gnu)"
url=$(select_download_url "$FIXTURE_JSON_FULL" "subx-macos-aarch64")
assert_eq "(f.4) macOS aarch64 URL" \
    "https://example.com/r/v1.2.3/subx-macos-aarch64" "$url"

# -----------------------------------------------------------------------
# (g) main() flag/env handling — exit codes for argument & env validation.
#
# These cases exit before any network call (`curl`) or filesystem write,
# so they are safe to drive end-to-end via a fresh `bash $INSTALL_SH`
# subprocess. We only assert exit code and that the error message is
# printed on the right stream.
# -----------------------------------------------------------------------
run_case_with_jq "(g) main() argument & env validation"

# (g.1) --help exits 0 and prints the usage banner to stdout.
help_out=$(bash "$INSTALL_SH" --help 2>/dev/null) && help_rc=$? || help_rc=$?
assert_eq "(g.1) --help exit code" "0" "$help_rc"
assert_contains "(g.1) --help prints SUBX_LIBC env doc" "$help_out" "SUBX_LIBC"

# (g.2) Unknown flag exits non-zero (specifically 2) with diagnostic on stderr.
bogus_err=$(bash "$INSTALL_SH" --no-such-flag 2>&1 >/dev/null) && bogus_rc=$? || bogus_rc=$?
assert_eq "(g.2) unknown flag exit code" "2" "$bogus_rc"
assert_contains "(g.2) unknown flag mentions the offending arg" "$bogus_err" "--no-such-flag"

# (g.3) Invalid SUBX_LIBC exits non-zero (specifically 2) before any network call.
invalid_err=$(SUBX_LIBC=bogus bash "$INSTALL_SH" 2>&1 >/dev/null) && invalid_rc=$? || invalid_rc=$?
assert_eq "(g.3) SUBX_LIBC=bogus exit code" "2" "$invalid_rc"
assert_contains "(g.3) SUBX_LIBC=bogus error mentions the env var" "$invalid_err" "SUBX_LIBC"

# -----------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------
TOTAL=$((PASSED + FAILED))
echo
echo "Test harness: $PASSED/$TOTAL passed"
if [ "$FAILED" -ne 0 ]; then
    echo "Failures:" >&2
    for f in "${FAILURES[@]}"; do
        echo "  - $f" >&2
    done
    exit 1
fi
exit 0
