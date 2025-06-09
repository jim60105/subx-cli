#!/bin/bash
# scripts/check_coverage.sh
#
# Test coverage threshold checking script
# Use cargo llvm-cov to generate coverage report and verify minimum requirements

set -euo pipefail # Strict mode: exit immediately on error

# Default configuration
DEFAULT_THRESHOLD=75.0
COVERAGE_THRESHOLD=${COVERAGE_THRESHOLD:-$DEFAULT_THRESHOLD}

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
    echo "  -v, --verbose            Show verbose output"
    echo "  -h, --help               Show this help"
    echo ""
    echo "Environment variables:"
    echo "  COVERAGE_THRESHOLD       Coverage threshold (default: ${DEFAULT_THRESHOLD}%)"
    echo ""
    echo "Examples:"
    echo "  $0                       Check coverage with default threshold"
    echo "  $0 -t 80                 Set threshold to 80%"
    echo "  COVERAGE_THRESHOLD=70 $0  Set threshold to 70% via environment variable"
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

    if [ ${#missing_deps[@]} -ne 0 ]; then
        echo -e "${RED}❌ Missing required dependencies:${NC}" >&2
        for dep in "${missing_deps[@]}"; do
            echo -e "   - ${dep}" >&2
        done
        echo -e "\n${YELLOW}Installation commands:${NC}" >&2
        echo -e "   cargo install cargo-llvm-cov" >&2
        echo -e "   # Ubuntu/Debian: sudo apt install jq bc" >&2
        echo -e "   # macOS: brew install jq bc" >&2
        exit 1
    fi
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
        -t | --threshold)
            COVERAGE_THRESHOLD="$2"
            shift 2
            ;;
        -v | --verbose)
            VERBOSE=true
            shift
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

# Main coverage check function
check_coverage() {
    echo -e "${BLUE}🔍 Checking test coverage...${NC}"

    # Generate coverage report
    local coverage_json
    local coverage_cmd_output
    if ! coverage_cmd_output=$(cargo llvm-cov --all-features --workspace --json --summary-only -q 2>&1); then
        echo -e "${RED}❌ Unable to generate coverage report${NC}" >&2
        echo -e "${YELLOW}Error message: ${coverage_cmd_output}${NC}" >&2
        echo -e "${YELLOW}Please ensure the project contains tests and can be compiled${NC}" >&2
        exit 1
    fi

    # Extract JSON part (filter out test output)
    coverage_json=$(echo "$coverage_cmd_output" | grep -E '^\{.*\}$' | tail -1)

    if [[ -z "$coverage_json" ]]; then
        echo -e "${RED}❌ Unable to extract JSON data from output${NC}" >&2
        echo -e "${YELLOW}Raw output:${NC}" >&2
        echo "$coverage_cmd_output" >&2
        exit 1
    fi

    # Parse coverage data
    local current_coverage
    if ! current_coverage=$(echo "$coverage_json" | jq -r '.data[0].totals.lines.percent' 2>/dev/null); then
        echo -e "${RED}❌ Unable to parse coverage data${NC}" >&2
        echo -e "${YELLOW}JSON format may have changed, please check cargo llvm-cov output${NC}" >&2
        if [[ "${VERBOSE:-false}" == "true" ]]; then
            echo -e "${YELLOW}JSON content:${NC}" >&2
            echo "$coverage_json" >&2
        fi
        exit 1
    fi

    # Validate data validity
    if [[ "$current_coverage" == "null" ]] || [[ -z "$current_coverage" ]]; then
        echo -e "${RED}❌ Unable to get valid coverage data${NC}" >&2
        exit 1
    fi

    # Display results
    echo -e "Current coverage: ${BLUE}${current_coverage}%${NC}"
    echo -e "Required threshold: ${BLUE}${COVERAGE_THRESHOLD}%${NC}"

    # Show detailed information (if verbose mode is enabled)
    if [[ "${VERBOSE:-false}" == "true" ]]; then
        echo -e "\n${YELLOW}Detailed coverage information:${NC}"
        echo "$coverage_json" | jq -r '
            .data[0].totals | 
            "  Function coverage: \(.functions.percent)% (\(.functions.covered)/\(.functions.count))",
            "  Line coverage:     \(.lines.percent)% (\(.lines.covered)/\(.lines.count))",
            "  Region coverage:   \(.regions.percent)% (\(.regions.covered)/\(.regions.count))"
        '
    fi

    # Compare coverage with threshold
    if (($(echo "${current_coverage} >= ${COVERAGE_THRESHOLD}" | bc -l))); then
        echo -e "\n${GREEN}✅ Coverage meets requirements${NC}"
        return 0
    else
        local deficit
        deficit=$(echo "${COVERAGE_THRESHOLD} - ${current_coverage}" | bc -l)
        echo -e "\n${RED}❌ Coverage below threshold (deficit: ${deficit}%)${NC}"
        return 1
    fi
}

# Main program
main() {
    parse_args "$@"
    check_dependencies
    check_coverage
}

# Execute main program
main "$@"
