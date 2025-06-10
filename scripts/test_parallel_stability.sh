#!/bin/bash
# scripts/test_parallel_stability.sh
# 測試並行測試穩定性驗證腳本

set -e

echo "開始並行測試穩定性驗證..."

# 高並行度測試
echo "執行高並行度測試..."
if ! cargo test --workspace --all-features -- --test-threads=16; then
    echo "❌ 高並行度測試失敗"
    exit 1
fi

# 中等並行度重複測試
echo "重複執行中等並行度測試確認穩定性..."
FAILED_COUNT=0
for i in {1..10}; do
    echo "測試執行 $i/10"
    if ! cargo test --workspace --all-features -- --test-threads=8 > /dev/null 2>&1; then
        echo "❌ 測試在第 $i 次執行時失敗"
        FAILED_COUNT=$((FAILED_COUNT + 1))
    fi
done

if [ $FAILED_COUNT -gt 0 ]; then
    echo "❌ $FAILED_COUNT/10 次測試執行失敗"
    exit 1
fi

# 預設並行度測試
echo "執行預設並行度測試..."
if ! cargo test --workspace --all-features; then
    echo "❌ 預設並行度測試失敗"
    exit 1
fi

echo "✅ 所有並行測試通過！"
echo "✅ 測試系統成功實現真正的並行隔離"
