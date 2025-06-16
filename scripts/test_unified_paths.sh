#!/usr/bin/env bash
# scripts/test_unified_paths.sh
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
# Test script to verify the unified path handling feature
# This will test the new functionality that allows combining -i and path parameters
# 
# ⚠️  WARNING: This script tests the actual match feature which may incur costs!
# ⚠️  Do NOT run this script unless explicitly instructed by the user.
# ⚠️  The match command may use external services that charge for API calls.
# 
# Requirements tested:
# 1. When -i is a directory, all files under it are included
# 2. When -i is a file, that file is included
# 3. Both -i and path can be used together or separately

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test result tracking
TOTAL_TESTS=0
PASSED_TESTS=0

# Helper function to display command output
assert_contains() {
    local output="$1"
    local expected="$2"
    local test_name="$3"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if echo "$output" | grep -q "$expected"; then
        echo -e "${GREEN}✅ PASS${NC}: $test_name"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}❌ FAIL${NC}: $test_name"
        echo -e "${YELLOW}Expected to find:${NC} $expected"
        echo -e "${YELLOW}Actual output:${NC}"
        echo "$output" | head -10
        return 1
    fi
}

# Helper function to assert command exit code
assert_success() {
    local exit_code="$1"
    local test_name="$2"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if [ "$exit_code" -eq 0 ]; then
        echo -e "${GREEN}✅ PASS${NC}: $test_name (exit code: $exit_code)"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}❌ FAIL${NC}: $test_name (exit code: $exit_code)"
        return 1
    fi
}

echo "🔍 Testing unified path handling functionality..."

# Create test workspace
TEST_DIR=$(mktemp -d)
echo "Created test directory: $TEST_DIR"

# Create test files and directories
mkdir -p "$TEST_DIR/dir1"
mkdir -p "$TEST_DIR/dir2"
mkdir -p "$TEST_DIR/dir3"

# Create test video and subtitle files
echo "fake video content" > "$TEST_DIR/dir1/movie1.mp4"
printf "1\n00:00:01,000 --> 00:00:02,000\nSubtitle 1\n" > "$TEST_DIR/dir1/movie1.srt"

echo "fake video content" > "$TEST_DIR/dir2/movie2.mp4"
printf "1\n00:00:01,000 --> 00:00:02,000\nSubtitle 2\n" > "$TEST_DIR/dir2/movie2.srt"

echo "fake video content" > "$TEST_DIR/dir3/movie3.mp4" 
printf "1\n00:00:01,000 --> 00:00:02,000\nSubtitle 3\n" > "$TEST_DIR/dir3/movie3.srt"

echo "✅ Test files created"

# Build the project
cargo build --release > /dev/null 2>&1

echo "✅ Project built successfully"

# Test 1: Using only -i with directory
echo -e "\n${YELLOW}🧪 Test 1: Using -i with directory${NC}"
echo -e "${GREEN}Command:${NC} ./target/release/subx-cli match -i \"$TEST_DIR/dir1\" --dry-run"
output1=$(./target/release/subx-cli match -i "$TEST_DIR/dir1" --dry-run 2>&1)
echo "$output1"
assert_contains "$output1" "movie1" "Test 1: Directory input should include movie1"
assert_contains "$output1" "dir1" "Test 1: Directory input should include dir1 path"

# Test 2: Using only path parameter
echo -e "\n${YELLOW}🧪 Test 2: Using path parameter${NC}"
echo -e "${GREEN}Command:${NC} ./target/release/subx-cli match \"$TEST_DIR/dir2\" --dry-run"
output2=$(./target/release/subx-cli match "$TEST_DIR/dir2" --dry-run 2>&1)
echo "$output2"
assert_contains "$output2" "movie2" "Test 2: Path parameter should include movie2"
assert_contains "$output2" "dir2" "Test 2: Path parameter should include dir2 path"

# Test 3: Using both -i and path together
echo -e "\n${YELLOW}🧪 Test 3: Using both -i and path together${NC}"
echo -e "${GREEN}Command:${NC} ./target/release/subx-cli match -i \"$TEST_DIR/dir1\" \"$TEST_DIR/dir3\" --dry-run"
output3=$(./target/release/subx-cli match -i "$TEST_DIR/dir1" "$TEST_DIR/dir3" --dry-run 2>&1)
echo "$output3"
assert_contains "$output3" "movie1" "Test 3: Combined input should include movie1 from -i"
assert_contains "$output3" "movie3" "Test 3: Combined input should include movie3 from path"

# Test 4: Using -i with only subtitle file (should fail with no matching pairs)
echo -e "\n${YELLOW}🧪 Test 4: Using -i with only subtitle file${NC}"
echo -e "${GREEN}Command:${NC} ./target/release/subx-cli match -i \"$TEST_DIR/dir2/movie2.srt\" --dry-run"
output4=$(./target/release/subx-cli match -i "$TEST_DIR/dir2/movie2.srt" --dry-run 2>&1)
echo "$output4"
assert_contains "$output4" "No matching file pairs found" "Test 4: Only subtitle file should result in no matching pairs"

# Test 5: Using multiple -i flags with individual files (video and subtitle pairs)
echo -e "\n${YELLOW}🧪 Test 5: Using multiple -i flags with individual files${NC}"
echo -e "${GREEN}Command:${NC} ./target/release/subx-cli match -i \"$TEST_DIR/dir1/movie1.mp4\" -i \"$TEST_DIR/dir1/movie1.srt\" -i \"$TEST_DIR/dir2/movie2.mp4\" -i \"$TEST_DIR/dir2/movie2.srt\" --dry-run"
output5=$(./target/release/subx-cli match -i "$TEST_DIR/dir1/movie1.mp4" -i "$TEST_DIR/dir1/movie1.srt" -i "$TEST_DIR/dir2/movie2.mp4" -i "$TEST_DIR/dir2/movie2.srt" --dry-run 2>&1)
echo "$output5"
assert_contains "$output5" "movie1" "Test 5: Multiple -i should match movie1 pair"
assert_contains "$output5" "movie2" "Test 5: Multiple -i should match movie2 pair"

# Test 6: Check exit codes for successful execution
echo -e "\n${YELLOW}🧪 Test 6: Exit code validation${NC}"
echo -e "${GREEN}Command:${NC} ./target/release/subx-cli match -i \"$TEST_DIR/dir1\" --dry-run"
./target/release/subx-cli match -i "$TEST_DIR/dir1" --dry-run > /dev/null 2>&1
assert_success $? "Test 6: Directory input should succeed"

echo -e "${GREEN}Command:${NC} ./target/release/subx-cli match \"$TEST_DIR/dir2\" --dry-run"
./target/release/subx-cli match "$TEST_DIR/dir2" --dry-run > /dev/null 2>&1
assert_success $? "Test 6: Path parameter should succeed"

# Summary
echo -e "\n${YELLOW}� Test Summary:${NC}"
echo -e "Total tests: ${TOTAL_TESTS}"
echo -e "Passed: ${GREEN}${PASSED_TESTS}${NC}"
echo -e "Failed: ${RED}$((TOTAL_TESTS - PASSED_TESTS))${NC}"

if [ $PASSED_TESTS -eq $TOTAL_TESTS ]; then
    echo -e "\n${GREEN}✅ All tests passed! Unified path handling is working correctly.${NC}"
    exit_code=0
else
    echo -e "\n${RED}❌ Some tests failed. Please check the output above.${NC}"
    exit_code=1
fi

# Clean up
rm -rf "$TEST_DIR"
echo -e "\n🧹 Test directory cleaned up"

exit $exit_code
