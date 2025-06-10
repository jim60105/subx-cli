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
# Automatically detects platform and downloads the latest release

set -e

# Detect operating system and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case $ARCH in
    x86_64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) echo "Error: Unsupported architecture: $ARCH"; exit 1 ;;
esac

case $OS in
    linux) PLATFORM="linux" ;;
    darwin) PLATFORM="macos" ;;
    *) echo "Error: Unsupported operating system: $OS"; exit 1 ;;
esac

echo "Detected platform: $PLATFORM ($ARCH)"

# Download latest release
RELEASE_URL="https://api.github.com/repos/jim60105/subx-cli/releases/latest"
BINARY_NAME="subx-${PLATFORM}-${ARCH}"

echo "Downloading SubX latest version..."
DOWNLOAD_URL=$(curl -s $RELEASE_URL | grep "browser_download_url.*$BINARY_NAME" | cut -d '"' -f 4)

if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Could not find download file for $PLATFORM-$ARCH"
    exit 1
fi

echo "Download URL: $DOWNLOAD_URL"
curl -L "$DOWNLOAD_URL" -o subx-cli

if [ ! -f "subx-cli" ]; then
    echo "Error: Download failed"
    exit 1
fi

chmod +x subx-cli

# Install to system path
echo "Installing to system path..."
if [[ "$EUID" -eq 0 ]]; then
    mv subx-cli /usr/local/bin/
    echo "SubX has been installed to /usr/local/bin/subx-cli"
else
    sudo mv subx-cli /usr/local/bin/
    echo "SubX has been installed to /usr/local/bin/subx-cli"
fi

echo "Installation complete! Run 'subx-cli --help' to get started"
