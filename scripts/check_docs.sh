#!/bin/bash
# Documentation Quality Check Script for SubX
# This script performs comprehensive documentation quality checks

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "🔍 SubX 文件品質檢查開始..."
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to check command success
check_result() {
    local exit_code=$1
    local test_name=$2
    
    if [ $exit_code -eq 0 ]; then
        print_status "$GREEN" "✅ $test_name: 通過"
        return 0
    else
        print_status "$RED" "❌ $test_name: 失敗"
        return 1
    fi
}

# Initialize counters
total_checks=0
passed_checks=0
failed_checks=0

run_check() {
    local check_name=$1
    local command=$2
    
    total_checks=$((total_checks + 1))
    print_status "$BLUE" "\n🔍 執行檢查: $check_name"
    
    if eval "$command"; then
        check_result 0 "$check_name"
        passed_checks=$((passed_checks + 1))
        return 0
    else
        check_result $? "$check_name"
        failed_checks=$((failed_checks + 1))
        return 1
    fi
}

# 1. Code compilation check
run_check "程式碼編譯檢查" "cargo check --all-features"

# 2. Code formatting check
run_check "程式碼格式化檢查" "cargo fmt -- --check"

# 3. Clippy linting check
run_check "Clippy 程式碼品質檢查" "cargo clippy --all-features -- -D warnings"

# 4. Documentation generation check
print_status "$BLUE" "\n🔍 執行檢查: 文件生成檢查"
total_checks=$((total_checks + 1))

cargo doc --all-features --no-deps --document-private-items 2>&1 | tee doc_output.log

# Check for critical errors (excluding known lint warnings)
if grep -E "(error)" doc_output.log | grep -v "warning\[E0602\]: unknown lint"; then
    print_status "$RED" "❌ 文件生成檢查: 發現嚴重錯誤"
    failed_checks=$((failed_checks + 1))
else
    # Count warnings (excluding known lint warnings)
    warning_count=$(grep -E "(warning)" doc_output.log | grep -v "warning\[E0602\]: unknown lint" | wc -l || echo "0")
    if [ "$warning_count" -gt 0 ]; then
        print_status "$YELLOW" "⚠️  文件生成檢查: 通過 (包含 $warning_count 個警告)"
    else
        print_status "$GREEN" "✅ 文件生成檢查: 通過"
    fi
    passed_checks=$((passed_checks + 1))
fi

# 5. Documentation examples test
run_check "文件範例測試" "cargo test --doc --verbose --all-features"

# 6. Documentation coverage check  
print_status "$BLUE" "\n🔍 執行檢查: 文件覆蓋率檢查"
total_checks=$((total_checks + 1))

# Check for missing documentation (allow warnings, don't fail build)
missing_docs_output=$(cargo clippy --all-features -- -W missing_docs 2>&1 | grep -v "warning\[E0602\]" | grep "missing documentation" || true)

if [ -n "$missing_docs_output" ]; then
    missing_count=$(echo "$missing_docs_output" | wc -l || echo "0")
    print_status "$YELLOW" "⚠️  文件覆蓋率檢查: 發現 $missing_count 個缺少文件的項目"
    
    # Only show first 5 items to avoid overwhelming output
    echo "$missing_docs_output" | head -5
    if [ "$missing_count" -gt 5 ]; then
        echo "... (顯示前 5 個，共 $missing_count 個)"
    fi
    print_status "$BLUE" "ℹ️  這些是建議改善項目，不會影響建置成功"
else
    print_status "$GREEN" "✅ 文件覆蓋率檢查: 所有公開 API 都有文件"
fi
passed_checks=$((passed_checks + 1))

# 7. Unit tests
run_check "單元測試" "cargo test --verbose"

# 8. Integration tests  
run_check "整合測試" "cargo test --test '*' --verbose"

# Cleanup
rm -f doc_output.log

# Summary
echo ""
echo "======================================"
print_status "$BLUE" "📊 文件品質檢查總結"
echo "======================================"
print_status "$GREEN" "✅ 通過檢查: $passed_checks"
print_status "$RED" "❌ 失敗檢查: $failed_checks"  
print_status "$BLUE" "📋 總計檢查: $total_checks"

if [ $failed_checks -eq 0 ]; then
    print_status "$GREEN" "\n🎉 所有文件品質檢查通過！"
    exit 0
else
    print_status "$RED" "\n⚠️  部分檢查失敗，請檢查上述錯誤訊息"
    exit 1
fi
