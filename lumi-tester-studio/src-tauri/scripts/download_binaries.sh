#!/bin/bash
# Script để tải và đóng gói các binaries cần thiết

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESOURCES_DIR="$SCRIPT_DIR/../resources/binaries"
PLATFORM="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

echo "📦 Downloading binaries for $PLATFORM-$ARCH..."

mkdir -p "$RESOURCES_DIR"

# Detect platform
if [[ "$PLATFORM" == "darwin" ]]; then
    PLATFORM_NAME="macos"
elif [[ "$PLATFORM" == "linux" ]]; then
    PLATFORM_NAME="linux"
else
    echo "❌ Unsupported platform: $PLATFORM"
    exit 1
fi

# Detect architecture
if [[ "$ARCH" == "arm64" || "$ARCH" == "aarch64" ]]; then
    ARCH_NAME="arm64"
elif [[ "$ARCH" == "x86_64" ]]; then
    ARCH_NAME="x64"
else
    echo "❌ Unsupported architecture: $ARCH"
    exit 1
fi

# Download ADB (Android Debug Bridge)
echo "⬇️  Downloading ADB..."
ADB_DIR="$RESOURCES_DIR/platform-tools"
mkdir -p "$ADB_DIR"

if [[ "$PLATFORM_NAME" == "macos" ]]; then
    ADB_URL="https://dl.google.com/android/repository/platform-tools-latest-darwin.zip"
elif [[ "$PLATFORM_NAME" == "linux" ]]; then
    ADB_URL="https://dl.google.com/android/repository/platform-tools-latest-linux.zip"
fi

if [ ! -f "$ADB_DIR/adb" ] && [ ! -f "$ADB_DIR/adb.exe" ]; then
    TEMP_ZIP="$RESOURCES_DIR/platform-tools.zip"
    curl -L -o "$TEMP_ZIP" "$ADB_URL"
    
    if [[ "$PLATFORM_NAME" == "macos" ]]; then
        unzip -q "$TEMP_ZIP" -d "$RESOURCES_DIR"
    else
        unzip -q "$TEMP_ZIP" -d "$RESOURCES_DIR"
    fi
    
    rm "$TEMP_ZIP"
    
    # Make executable
    chmod +x "$ADB_DIR/adb" 2>/dev/null || true
    
    echo "✅ ADB downloaded successfully"
else
    echo "✅ ADB already exists"
fi

# Create symlink for adb in binaries directory
if [ -f "$ADB_DIR/adb" ]; then
    cp "$ADB_DIR/adb" "$RESOURCES_DIR/adb"
    chmod +x "$RESOURCES_DIR/adb"
fi

# Note: IDB và FFmpeg cần được cài đặt thủ công hoặc tải từ nguồn khác
# IDB: https://github.com/facebook/idb
# FFmpeg: https://ffmpeg.org/download.html

echo ""
echo "✅ Binaries download complete!"
echo "📁 Binaries location: $RESOURCES_DIR"
echo ""
echo "⚠️  Note: IDB và FFmpeg cần được cài đặt thủ công:"
echo "   - IDB: brew install idb-companion (macOS) hoặc pip install fb-idb"
echo "   - FFmpeg: brew install ffmpeg (macOS) hoặc apt install ffmpeg (Linux)"
