#!/bin/bash
# Setup resources for lumi-tester development environment
# Downloads: ADBKeyboard, ADB (platform-tools), FFmpeg, Playwright

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESOURCES_DIR="$SCRIPT_DIR"
APK_DIR="$RESOURCES_DIR/apk"
INSTALL_DIR="$HOME/.lumi-tester"

# Detect OS
OS="unknown"
case "$(uname -s)" in
    Darwin*) OS="macos" ;;
    Linux*)  OS="linux" ;;
esac

echo "🔧 Setting up lumi-tester resources for $OS..."
echo ""

# Create directories
mkdir -p "$APK_DIR"
mkdir -p "$INSTALL_DIR/apk"
mkdir -p "$INSTALL_DIR/platform-tools"
mkdir -p "$INSTALL_DIR/playwright"

#---------------------------------------
# 1. ADBKeyboard APK
#---------------------------------------
echo "📦 [1/5] ADBKeyboard APK..."
ADBKEYBOARD_URL="https://github.com/senzhk/ADBKeyBoard/raw/master/ADBKeyboard.apk"
ADBKEYBOARD_APK="$APK_DIR/ADBKeyboard.apk"

if [ ! -f "$ADBKEYBOARD_APK" ]; then
    curl -L -s -o "$ADBKEYBOARD_APK" "$ADBKEYBOARD_URL"
    echo "   ✓ Downloaded ADBKeyboard.apk"
else
    echo "   ✓ ADBKeyboard.apk already exists"
fi
cp "$ADBKEYBOARD_APK" "$INSTALL_DIR/apk/"

#---------------------------------------
# 2. Android Platform Tools (ADB)
#---------------------------------------
echo "📦 [2/5] Android Platform Tools (ADB)..."
if [ "$OS" = "macos" ]; then
    PLATFORM_TOOLS_URL="https://dl.google.com/android/repository/platform-tools-latest-darwin.zip"
elif [ "$OS" = "linux" ]; then
    PLATFORM_TOOLS_URL="https://dl.google.com/android/repository/platform-tools-latest-linux.zip"
fi

if [ ! -f "$INSTALL_DIR/platform-tools/adb" ]; then
    TEMP_ZIP="/tmp/platform-tools.zip"
    curl -L -s -o "$TEMP_ZIP" "$PLATFORM_TOOLS_URL"
    unzip -q -o "$TEMP_ZIP" -d "$INSTALL_DIR/"
    rm "$TEMP_ZIP"
    chmod +x "$INSTALL_DIR/platform-tools/adb"
    echo "   ✓ Downloaded and extracted platform-tools"
else
    echo "   ✓ ADB already exists at $INSTALL_DIR/platform-tools/adb"
fi

#---------------------------------------
# 3. FFmpeg (via Playwright)
#---------------------------------------
echo "📦 [3/5] FFmpeg..."
FFMPEG_PATH="$INSTALL_DIR/playwright/ffmpeg"

if [ ! -f "$FFMPEG_PATH" ]; then
    # Try to get ffmpeg from Playwright cache or download
    PLAYWRIGHT_CACHE="$HOME/Library/Caches/ms-playwright"
    if [ "$OS" = "linux" ]; then
        PLAYWRIGHT_CACHE="$HOME/.cache/ms-playwright"
    fi
    
    # Find ffmpeg in Playwright cache
    FOUND_FFMPEG=$(find "$PLAYWRIGHT_CACHE" -name "ffmpeg-*" -type d 2>/dev/null | head -1)
    if [ -n "$FOUND_FFMPEG" ] && [ -f "$FOUND_FFMPEG/ffmpeg" ]; then
        cp "$FOUND_FFMPEG/ffmpeg" "$FFMPEG_PATH"
        chmod +x "$FFMPEG_PATH"
        echo "   ✓ Copied ffmpeg from Playwright cache"
    else
        echo "   ⚠ FFmpeg not found. Run 'npx playwright install' first, then re-run this script"
    fi
else
    echo "   ✓ FFmpeg already exists"
fi

#---------------------------------------
# 4. Playwright (Node.js dependency)
#---------------------------------------
echo "📦 [4/5] Playwright browsers..."
if command -v npx &> /dev/null; then
    echo "   Installing Playwright browsers..."
    npx playwright install chromium 2>/dev/null || true
    echo "   ✓ Playwright browsers installed"
else
    echo "   ⚠ npx not found. Install Node.js to use Playwright"
fi

#---------------------------------------
# 5. iOS: idb (macOS only)
#---------------------------------------
echo "📦 [5/5] iOS tools (idb)..."
if [ "$OS" = "macos" ]; then
    if command -v idb &> /dev/null; then
        echo "   ✓ idb already installed"
    else
        if command -v brew &> /dev/null; then
            echo "   Installing idb via Homebrew..."
            brew tap facebook/fb
            brew install idb-companion
            pip3 install fb-idb || pip install fb-idb
            echo "   ✓ idb installed"
        else
            echo "   ⚠ Homebrew not found. Install manually: brew tap facebook/fb && brew install idb-companion"
        fi
    fi
else
    echo "   ⏭ Skipped (iOS tools only available on macOS)"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Resources setup complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Installed locations:"
echo "  📱 ADBKeyboard: $INSTALL_DIR/apk/"
echo "  🤖 ADB:         $INSTALL_DIR/platform-tools/adb"
echo "  🎬 FFmpeg:      $INSTALL_DIR/playwright/ffmpeg"
echo "  🌐 Playwright:  ~/.cache/ms-playwright (browsers)"
if [ "$OS" = "macos" ]; then
    echo "  🍎 idb:         $(which idb 2>/dev/null || echo 'not installed')"
fi
echo ""
