#!/bin/bash

set -e

# 檢測作業系統和架構
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case $ARCH in
    x86_64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) echo "不支援的架構: $ARCH"; exit 1 ;;
esac

case $OS in
    linux) PLATFORM="linux" ;;
    darwin) PLATFORM="macos" ;;
    *) echo "不支援的作業系統: $OS"; exit 1 ;;
esac

# 下載最新版本
RELEASE_URL="https://api.github.com/repos/yourusername/subx/releases/latest"
BINARY_NAME="subx-${PLATFORM}-${ARCH}"

echo "正在下載 SubX 最新版本..."
curl -L "$(curl -s $RELEASE_URL | grep "browser_download_url.*$BINARY_NAME" | cut -d '"' -f 4)" -o subx

chmod +x subx

# 安裝到系統路徑
if [[ "$EUID" -eq 0 ]]; then
    mv subx /usr/local/bin/
    echo "SubX 已安裝到 /usr/local/bin/subx"
else
    sudo mv subx /usr/local/bin/
    echo "SubX 已安裝到 /usr/local/bin/subx"
fi

echo "安裝完成! 執行 'subx --help' 開始使用"
