#!/bin/bash
# scripts/check_coverage.sh
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
# Test coverage threshold checking script
# Use cargo llvm-cov to generate coverage report and verify minimum requirements

set -euo pipefail # Strict mode: exit immediately on error

# Default configuration
DEFAULT_THRESHOLD=75.0
COVERAGE_THRESHOLD=${COVERAGE_THRESHOLD:-$DEFAULT_THRESHOLD}
DEFAULT_THRESHOLD_CORE=90.0
COVERAGE_THRESHOLD_CORE=${COVERAGE_THRESHOLD_CORE:-$DEFAULT_THRESHOLD_CORE}
DEFAULT_THRESHOLD_CLI=82.0
COVERAGE_THRESHOLD_CLI=${COVERAGE_THRESHOLD_CLI:-$DEFAULT_THRESHOLD_CLI}
DEFAULT_NEXTEST_PROFILE="default"
NEXTEST_PROFILE=${NEXTEST_PROFILE:-$DEFAULT_NEXTEST_PROFILE}

# Reporting exclusion set (normative value fixed by the
# cross-crate-testing spec). Every `cargo llvm-cov report` invocation in this
# script MUST receive it via --ignore-filename-regex — the lcov file uploaded
# to Codecov and the JSON the thresholds are computed from MUST describe the
# same codebase, or the gated number and the displayed number are different
# measurements.
LLVM_COV_IGNORE='(^|/)(tests|benches)/|(^|/)src/main\.rs$|(^|/)src/test_support/'

# Full tests mode
FULL_TESTS=false

# Track if profile was explicitly set
PROFILE_SET=false

# Output LCOV file
LCOV_OUTPUT_PATH=""

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Usage information
usage() {
    echo "Usage: $0 [options]"
    echo "Options:"
    echo "  -t, --threshold PERCENT   Set coverage threshold (default: ${DEFAULT_THRESHOLD}%)"
    echo "  --threshold-core PERCENT  Set subx-core crate floor (default: ${DEFAULT_THRESHOLD_CORE}%)"
    echo "  --threshold-cli PERCENT   Set subx-cli crate floor (default: ${DEFAULT_THRESHOLD_CLI}%)"
    echo "  -p, --profile PROFILE     Set nextest profile (default: ${DEFAULT_NEXTEST_PROFILE})"
    echo "  --full                   Run full tests including slow tests (uses full profile unless -p is set)"
    echo "  -T, --table              Show coverage table for all files"
    echo "  -f, --file FILENAME      Show coverage for specific file (supports partial matching)"
    echo "  -v, --verbose            Show verbose output"
    echo "  --lcov PATH              Also generate LCOV output file at specified path"
    echo "  -h, --help               Show this help"
    echo ""
    echo "Environment variables:"
    echo "  COVERAGE_THRESHOLD       Coverage threshold (default: ${DEFAULT_THRESHOLD}%)"
    echo "  COVERAGE_THRESHOLD_CORE  subx-core crate floor (default: ${DEFAULT_THRESHOLD_CORE}%)"
    echo "  COVERAGE_THRESHOLD_CLI   subx-cli crate floor (default: ${DEFAULT_THRESHOLD_CLI}%)"
    echo "  NEXTEST_PROFILE          Nextest profile (default: ${DEFAULT_NEXTEST_PROFILE})"
    echo ""
    echo "Available nextest profiles: default, ci, quick, full"
    echo ""
    echo "Examples:"
    echo "  $0                       Check coverage with default threshold and profile (excludes slow tests)"
    echo "  $0 -t 80                 Set threshold to 80%"
    echo "  $0 -p ci                 Use CI profile"
    echo "  $0 --full                Run full tests including slow tests (~90 vs ~143 seconds)"
    echo "  $0 --table               Show coverage table for all files"
    echo "  $0 -f manager.rs         Show coverage for files matching 'manager.rs'"
    echo "  $0 --lcov lcov.info      Generate both JSON and LCOV coverage reports"
    echo "  COVERAGE_THRESHOLD=70 $0  Set threshold to 70% via environment variable"
    echo "  NEXTEST_PROFILE=ci $0     Set profile to ci via environment variable"
}

# Check dependencies
check_dependencies() {
    local missing_deps=()

    if ! command -v cargo &>/dev/null; then
        missing_deps+=("cargo")
    fi

    if ! command -v jq &>/dev/null; then
        missing_deps+=("jq")
    fi

    if ! command -v bc &>/dev/null; then
        missing_deps+=("bc")
    fi

    if ! cargo llvm-cov --version &>/dev/null; then
        missing_deps+=("cargo-llvm-cov")
    fi

    if ! cargo nextest --version &>/dev/null; then
        missing_deps+=("cargo-nextest")
    fi

    if [ ${#missing_deps[@]} -ne 0 ]; then
        echo -e "${RED}❌ Missing required dependencies:${NC}" >&2
        for dep in "${missing_deps[@]}"; do
            echo -e "   - ${dep}" >&2
        done
        echo -e "\n${YELLOW}Installation commands:${NC}" >&2
        echo -e "   cargo install cargo-llvm-cov" >&2
        echo -e "   cargo install cargo-nextest --locked" >&2
        echo -e "   # Ubuntu/Debian: sudo apt install jq bc" >&2
        echo -e "   # macOS: brew install jq bc" >&2
        exit 1
    fi
}

# Format percentage to 2 decimal places
format_percentage() {
    local field="$1"
    echo "(($field * 100 | round) / 100)"
}

# Generate jq filter for percentage formatting
get_percentage_filter() {
    echo 'def format_pct(x): ((x * 100 | round) / 100);'
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
        -t | --threshold)
            COVERAGE_THRESHOLD="$2"
            shift 2
            ;;
        --threshold-core)
            COVERAGE_THRESHOLD_CORE="$2"
            shift 2
            ;;
        --threshold-cli)
            COVERAGE_THRESHOLD_CLI="$2"
            shift 2
            ;;
        -p | --profile)
            if [[ -z "$2" ]]; then
                echo -e "${RED}Error: --profile requires a value${NC}" >&2
                usage >&2
                exit 1
            fi
            # Validate profile value
            case "$2" in
                default|ci|quick|full)
                    NEXTEST_PROFILE="$2"
                    PROFILE_SET=true
                    ;;
                *)
                    echo -e "${RED}Error: Invalid profile '$2'. Available profiles: default, ci, quick, full${NC}" >&2
                    exit 1
                    ;;
            esac
            shift 2
            ;;
        --full)
            FULL_TESTS=true
            # Only override profile if not explicitly set with -p
            if [[ "$PROFILE_SET" != "true" ]]; then
                NEXTEST_PROFILE="full"
            fi
            shift
            ;;
        -T | --table)
            SHOW_TABLE=true
            shift
            ;;
        -f | --file)
            SEARCH_FILE="$2"
            shift 2
            ;;
        -v | --verbose)
            VERBOSE=true
            shift
            ;;
        --lcov)
            LCOV_OUTPUT_PATH="$2"
            shift 2
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo -e "${RED}Error: Unknown option $1${NC}" >&2
            usage >&2
            exit 1
            ;;
        esac
    done
}

# Display coverage table for all files
show_coverage_table() {
    local coverage_json="$1"

    echo -e "${BLUE}📊 File Coverage Report${NC}"
    echo ""

    # Table header
    printf "%-60s %8s %8s %8s %8s\n" "File" "Lines" "Funcs" "Regions" "Instants"
    printf "%-60s %8s %8s %8s %8s\n" "$(printf '%*s' 60 '' | tr ' ' '-')" "--------" "--------" "--------" "--------"

    # Extract and display file coverage data
    echo "$coverage_json" | jq -r "$(get_percentage_filter)"'
        .data[0].files[] |
        [
            .filename,
            (format_pct(.summary.lines.percent) | tostring + "%"),
            (format_pct(.summary.functions.percent) | tostring + "%"),
            (format_pct(.summary.regions.percent) | tostring + "%"),
            (format_pct(.summary.instantiations.percent) | tostring + "%")
        ] | @tsv
    ' | while IFS=$'\t' read -r filename lines_pct funcs_pct regions_pct instants_pct; do
        # Truncate long filenames for better display
        local display_name
        if [[ ${#filename} -gt 57 ]]; then
            display_name="...${filename: -54}"
        else
            display_name="$filename"
        fi

        # Color coding based on line coverage
        local line_coverage
        line_coverage=$(echo "$lines_pct" | sed 's/%//')
        local color=""
        if (( $(echo "$line_coverage >= 80" | bc -l) )); then
            color="$GREEN"
        elif (( $(echo "$line_coverage >= 60" | bc -l) )); then
            color="$YELLOW"
        else
            color="$RED"
        fi

        printf "%-60s ${color}%8s${NC} %8s %8s %8s\n" \
            "$display_name" "$lines_pct" "$funcs_pct" "$regions_pct" "$instants_pct"
    done

    echo ""
    echo -e "${BLUE}Legend:${NC} ${GREEN}>=80%${NC} ${YELLOW}60-79%${NC} ${RED}<60%${NC} (based on line coverage)"
    
    # Display overall coverage summary
    show_overall_coverage "$coverage_json"
}

# Partition the JSON file list into the two crates and derive one line
# coverage percentage per crate.
#
# Rules:
#   - filenames are normalised backslash -> slash before any path test, so a
#     Windows-resident workspace (C:\\...\\subx-core\\src\\...) is attributed
#     to the same crate as a POSIX one;
#   - a file is `subx-core` iff its path contains `/subx-core/src/`;
#   - otherwise it is `subx-cli` iff it starts with "<workspace root>/src/";
#   - anything else is `unexpected` — it is reported and makes the run fail,
#     because silently crediting a file outside both source roots to the CLI
#     group would let one crate's floor gate the other's lines.
# Percentages are derived from summed covered/count lines, NEVER as a mean of
# per-file percentages (that would weight a 3-line module like a 900-line
# engine). The group line counts MUST sum to the report total (task 4.10):
# a mis-filed or dropped file makes the sums disagree.
show_per_crate_coverage() {
    local coverage_json="$1"
    local root_prefix="${PWD}/src/"

    # One jq pass producing five numbers:
    #   core_covered core_count cli_covered cli_count unexpected_count
    local sums
    if ! sums=$(echo "$coverage_json" | jq -r --arg root "$root_prefix" '
        def norm: gsub("\\\\"; "/");
        def nroot: $root | gsub("\\\\"; "/");
        def iscore: .filename | norm | contains("/subx-core/src/");
        def iscli: (.filename | norm | startswith(nroot)) and (iscore | not);
        (.data[0].files // []) as $files
        | [ $files, ($files | map(select(iscore))) ] as [$all, $core]
        | ($all | map(select(iscli))) as $cli
        | [
            ($core | map(.summary.lines.covered) | add // 0),
            ($core | map(.summary.lines.count) | add // 0),
            ($cli | map(.summary.lines.covered) | add // 0),
            ($cli | map(.summary.lines.count) | add // 0),
            ($all | map(select((iscore or iscli) | not)) | length)
          ]
        | @tsv
    ' 2>/dev/null); then
        echo -e "${RED}❌ Unable to partition coverage data per crate${NC}" >&2
        return 1
    fi

    IFS=$'\t' read -r core_covered core_count cli_covered cli_count unexpected_count <<<"$sums"

    local total_lines_count
    total_lines_count=$(echo "$coverage_json" | jq -r '.data[0].totals.lines.count // 0' 2>/dev/null)

    if [[ "$unexpected_count" -gt 0 ]]; then
        echo -e "${RED}❌ Coverage partition found $unexpected_count file(s) outside both source roots:${NC}" >&2
        echo "$coverage_json" | jq -r --arg root "$root_prefix" '
            def norm: gsub("\\\\"; "/");
            def nroot: $root | gsub("\\\\"; "/");
            (.data[0].files // [])[]
            | select(((.filename | norm | contains("/subx-core/src/")) or (.filename | norm | startswith(nroot))) | not)
            | "   - " + .filename
        ' | head -10 >&2
        return 1
    fi

    # Partition-completeness assertion: every reported line belongs to exactly
    # one crate group.
    if [[ $((core_count + cli_count)) -ne "$total_lines_count" ]]; then
        echo -e "${RED}❌ Coverage partition incomplete: core($core_count) + cli($cli_count) != total($total_lines_count)${NC}" >&2
        return 1
    fi

    local core_pct cli_pct
    core_pct=$(jq -rn --argjson c "$core_covered" --argjson t "$core_count" 'if $t == 0 then 0 else (($c * 10000 / $t | round) / 100) end')
    cli_pct=$(jq -rn --argjson c "$cli_covered" --argjson t "$cli_count" 'if $t == 0 then 0 else (($c * 10000 / $t | round) / 100) end')

    echo -e "\n${BLUE}📦 Per-crate line coverage (workspace-attributed):${NC}"
    echo -e "  subx-core: ${BLUE}${core_pct}%${NC} (${core_covered}/${core_count}) — floor: ${BLUE}${COVERAGE_THRESHOLD_CORE}%${NC}"
    echo -e "  subx-cli:  ${BLUE}${cli_pct}%${NC} (${cli_covered}/${cli_count}) — floor: ${BLUE}${COVERAGE_THRESHOLD_CLI}%${NC}"

    local rc=0
    if (($(echo "${core_pct} >= ${COVERAGE_THRESHOLD_CORE}" | bc -l))); then
        echo -e "  ${GREEN}✅ subx-core meets its floor${NC}"
    else
        local core_deficit
        core_deficit=$(echo "${COVERAGE_THRESHOLD_CORE} - ${core_pct}" | bc -l)
        echo -e "  ${RED}❌ subx-core below its floor (deficit: ${core_deficit}%)${NC}"
        rc=1
    fi
    if (($(echo "${cli_pct} >= ${COVERAGE_THRESHOLD_CLI}" | bc -l))); then
        echo -e "  ${GREEN}✅ subx-cli meets its floor${NC}"
    else
        local cli_deficit
        cli_deficit=$(echo "${COVERAGE_THRESHOLD_CLI} - ${cli_pct}" | bc -l)
        echo -e "  ${RED}❌ subx-cli below its floor (deficit: ${cli_deficit}%)${NC}"
        rc=1
    fi
    return $rc
}

# Display overall coverage summary and check all three floors
show_overall_coverage() {
    local coverage_json="$1"
    local show_header="${2:-true}"

    if [[ "$show_header" == "true" ]]; then
        echo ""
        echo -e "${BLUE}📈 Overall Coverage Summary${NC}"
        echo ""
    fi

    # Parse overall coverage data
    local current_coverage
    if ! current_coverage=$(echo "$coverage_json" | jq -r "$(get_percentage_filter)"'format_pct(.data[0].totals.lines.percent)' 2>/dev/null); then
        echo -e "${RED}❌ Unable to parse overall coverage data${NC}" >&2
        return 1
    fi

    # Validate data validity
    if [[ "$current_coverage" == "null" ]] || [[ -z "$current_coverage" ]]; then
        echo -e "${RED}❌ Unable to get valid overall coverage data${NC}" >&2
        return 1
    fi

    # Display overall results
    echo -e "Current coverage: ${BLUE}${current_coverage}%${NC}"
    echo -e "Required threshold: ${BLUE}${COVERAGE_THRESHOLD}%${NC}"

    # Show detailed information (if verbose mode is enabled or in table mode)
    if [[ "${VERBOSE:-false}" == "true" ]] || [[ "${SHOW_TABLE:-false}" == "true" ]]; then
        echo -e "\n${YELLOW}Detailed coverage information:${NC}"
        echo "$coverage_json" | jq -r "$(get_percentage_filter)"'
            .data[0].totals |
            "  Function coverage: " + (format_pct(.functions.percent) | tostring) + "% (\(.functions.covered)/\(.functions.count))",
            "  Line coverage:     " + (format_pct(.lines.percent) | tostring) + "% (\(.lines.covered)/\(.lines.count))",
            "  Region coverage:   " + (format_pct(.regions.percent) | tostring) + "% (\(.regions.covered)/\(.regions.count))"
        '
    fi

    local rc=0

    # Compare combined coverage with the workspace threshold
    if (($(echo "${current_coverage} >= ${COVERAGE_THRESHOLD}" | bc -l))); then
        echo -e "\n${GREEN}✅ Combined coverage meets requirements${NC}"
    else
        local deficit
        deficit=$(echo "${COVERAGE_THRESHOLD} - ${current_coverage}" | bc -l)
        echo -e "\n${RED}❌ Combined coverage below threshold (deficit: ${deficit}%)${NC}"
        rc=1
    fi

    # Gate the two per-crate floors. All three numbers are printed
    # regardless of outcome; any one below its floor fails the run.
    if ! show_per_crate_coverage "$coverage_json"; then
        rc=1
    fi

    return $rc
}

# Search and display coverage for specific file
search_file_coverage() {
    local coverage_json="$1"
    local search_pattern="$2"

    echo -e "${BLUE}🔍 Searching for files matching '${search_pattern}'...${NC}"
    echo ""

    # Search for matching files (case-insensitive)
    local matching_files
    matching_files=$(echo "$coverage_json" | jq -r --arg pattern "$search_pattern" "$(get_percentage_filter)"'
        .data[0].files[] |
        select(.filename | ascii_downcase | contains($pattern | ascii_downcase)) |
        [
            .filename,
            format_pct(.summary.lines.percent),
            .summary.lines.covered,
            .summary.lines.count,
            format_pct(.summary.functions.percent),
            .summary.functions.covered,
            .summary.functions.count,
            format_pct(.summary.regions.percent),
            .summary.regions.covered,
            .summary.regions.count,
            format_pct(.summary.instantiations.percent),
            .summary.instantiations.covered,
            .summary.instantiations.count
        ] | @tsv
    ')

    if [[ -z "$matching_files" ]]; then
        echo -e "${RED}❌ No files found matching '${search_pattern}'${NC}"
        return 1
    fi
    
    echo "$matching_files" | while IFS=$'\t' read -r filename lines_pct lines_covered lines_total \
        funcs_pct funcs_covered funcs_total regions_pct regions_covered regions_total \
        instants_pct instants_covered instants_total; do

        echo -e "${GREEN}📄 File: ${NC}${filename}"
        echo -e "   ${BLUE}Lines:${NC}         ${lines_pct}% (${lines_covered}/${lines_total})"
        echo -e "   ${BLUE}Functions:${NC}     ${funcs_pct}% (${funcs_covered}/${funcs_total})"
        echo -e "   ${BLUE}Regions:${NC}       ${regions_pct}% (${regions_covered}/${regions_total})"
        echo -e "   ${BLUE}Instantiations:${NC} ${instants_pct}% (${instants_covered}/${instants_total})"
        echo ""
    done
}

# Main coverage check function
check_coverage() {
    echo -e "${BLUE}🔍 Checking test coverage...${NC}"
    echo -e "${BLUE}🔧 Using nextest profile: ${NEXTEST_PROFILE}${NC}"
    if [[ "${FULL_TESTS}" == "true" ]]; then
        echo -e "${BLUE}⚡ Full tests mode: Including slow tests (~143s vs ~90s)${NC}"
    else
        echo -e "${BLUE}🚀 Fast tests mode: Excluding slow tests (~90s vs ~143s)${NC}"
        echo -e "${BLUE}   Use --full to include slow tests${NC}"
    fi

    # Generate coverage report
    local coverage_json
    local coverage_cmd_output
    local coverage_features=""
    if [[ "${FULL_TESTS}" == "true" ]]; then
        coverage_features="--features slow-tests"
    fi
    
    echo -e "${BLUE}📄 Running tests to generate coverage data...${NC}"
    
    # Run tests to generate coverage data.
    # Coverage profraw files are collected even when some tests fail,
    # so we continue to generate the report regardless of test exit code.
    local test_log
    test_log=$(mktemp)
    local test_exit_code=0
    cargo llvm-cov nextest --profile "${NEXTEST_PROFILE}" --workspace ${coverage_features} --no-report >"$test_log" 2>&1 || test_exit_code=$?
    
    if [[ $test_exit_code -ne 0 ]]; then
        echo -e "${YELLOW}⚠️  Test runner exited with code ${test_exit_code}${NC}"
        if [[ "${VERBOSE:-false}" == "true" ]]; then
            cat "$test_log"
        else
            # Show summary line (last few lines) for quick diagnostics
            echo -e "${YELLOW}   $(tail -3 "$test_log" | head -1)${NC}"
            echo -e "${YELLOW}   Use -v for full test output${NC}"
        fi
    fi
    rm -f "$test_log"
    
    if [[ $test_exit_code -eq 0 ]]; then
        echo -e "${GREEN}✅ Tests completed successfully${NC}"
    else
        echo -e "${YELLOW}⚠️  Continuing with coverage report generation...${NC}"
    fi
    
    # Generate LCOV output if requested, using report to avoid re-running
    # tests. BOTH report invocations below MUST carry the identical
    # --ignore-filename-regex "${LLVM_COV_IGNORE}": the lcov.info uploaded to
    # Codecov and the JSON the thresholds are computed from MUST describe the
    # same codebase, or the number CI enforces and the number Codecov
    # displays are different measurements.
    if [[ -n "${LCOV_OUTPUT_PATH}" ]]; then
        echo -e "${BLUE}📄 Generating LCOV output at: ${LCOV_OUTPUT_PATH}${NC}"
        if ! cargo llvm-cov report --lcov --output-path "${LCOV_OUTPUT_PATH}" --ignore-filename-regex "${LLVM_COV_IGNORE}" >/dev/null 2>&1; then
            echo -e "${RED}❌ Unable to generate LCOV coverage report${NC}" >&2
            exit 1
        fi
        echo -e "${GREEN}✅ LCOV coverage report generated successfully${NC}"
    fi
    
    # Generate JSON output for analysis, using report to avoid re-running tests
    if ! coverage_cmd_output=$(cargo llvm-cov report --json --summary-only --ignore-filename-regex "${LLVM_COV_IGNORE}" 2>&1); then
        echo -e "${RED}❌ Unable to extract JSON coverage data${NC}" >&2
        echo -e "${YELLOW}Error message: ${coverage_cmd_output}${NC}" >&2
        exit 1
    fi
    
    echo -e "${GREEN}✅ JSON coverage data extracted successfully${NC}"

    # Extract JSON part (filter out test output)
    coverage_json=$(echo "$coverage_cmd_output" | grep -E '^\{.*\}$' | tail -1)

    if [[ -z "$coverage_json" ]]; then
        echo -e "${RED}❌ Unable to extract JSON data from output${NC}" >&2
        echo -e "${YELLOW}Raw output:${NC}" >&2
        echo "$coverage_cmd_output" >&2
        exit 1
    fi

    # Handle table display option
    if [[ "${SHOW_TABLE:-false}" == "true" ]]; then
        show_coverage_table "$coverage_json"
        return $?
    fi

    # Handle file search option
    if [[ -n "${SEARCH_FILE:-}" ]]; then
        search_file_coverage "$coverage_json" "$SEARCH_FILE"
        return 0
    fi

    # Default mode - show overall coverage
    show_overall_coverage "$coverage_json" "false"
}

# Main program
main() {
    parse_args "$@"
    check_dependencies
    check_coverage
}

# Execute main program
main "$@"
