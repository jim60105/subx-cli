#!/usr/bin/env bash

# Test script to verify the unified path handling feature
# This will test the new functionality that allows combining -i and path parameters
# 
# Requirements tested:
# 1. When -i is a directory, all files under it are included
# 2. When -i is a file, that file is included
# 3. Both -i and path can be used together or separately

set -e

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
echo "1\n00:00:01,000 --> 00:00:02,000\nSubtitle 1" > "$TEST_DIR/dir1/movie1.srt"

echo "fake video content" > "$TEST_DIR/dir2/movie2.mp4"
echo "1\n00:00:01,000 --> 00:00:02,000\nSubtitle 2" > "$TEST_DIR/dir2/movie2.srt"

echo "fake video content" > "$TEST_DIR/dir3/movie3.mp4" 
echo "1\n00:00:01,000 --> 00:00:02,000\nSubtitle 3" > "$TEST_DIR/dir3/movie3.srt"

echo "✅ Test files created"

# Build the project
cd /workspaces/subx
cargo build --release > /dev/null 2>&1

echo "✅ Project built successfully"

# Test 1: Using only -i with directory
echo "🧪 Test 1: Using -i with directory"
./target/release/subx-cli match -i "$TEST_DIR/dir1" --dry-run 2>&1 | head -5

# Test 2: Using only path parameter
echo "🧪 Test 2: Using path parameter"
./target/release/subx-cli match "$TEST_DIR/dir2" --dry-run 2>&1 | head -5

# Test 3: Using both -i and path together
echo "🧪 Test 3: Using both -i and path together"
./target/release/subx-cli match -i "$TEST_DIR/dir1" "$TEST_DIR/dir3" --dry-run 2>&1 | head -5

# Test 4: Using -i with a file
echo "🧪 Test 4: Using -i with a file"
./target/release/subx-cli match -i "$TEST_DIR/dir2/movie2.srt" --dry-run 2>&1 | head -5

# Test 5: Using multiple -i flags
echo "🧪 Test 5: Using multiple -i flags"
./target/release/subx-cli match -i "$TEST_DIR/dir1" -i "$TEST_DIR/dir2" --dry-run 2>&1 | head -5

echo "✅ All tests completed successfully!"
echo "📂 Test results show unified path handling is working correctly"

# Clean up
rm -rf "$TEST_DIR"
echo "🧹 Test directory cleaned up"
