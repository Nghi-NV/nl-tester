#!/bin/bash
set -e

# Detect OS
OS="$(uname -s)"
ARCH="$(uname -m)"

echo "Detected OS: $OS"
echo "Detected Arch: $ARCH"

GITHUB_REPO="Nghi-NV/nl-tester"
LATEST_RELEASE_URL="https://api.github.com/repos/$GITHUB_REPO/releases/latest"

# Determine asset name
if [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "arm64" ]; then
        ASSET_NAME="lumi-tester-aarch64-apple-darwin"
    else
        ASSET_NAME="lumi-tester-x86_64-apple-darwin"
    fi
elif [ "$OS" = "Linux" ]; then
    echo "Linux support is experimental. Assuming x86_64."
    ASSET_NAME="lumi-tester-x86_64-unknown-linux-gnu" # Matches hypothetical linux build
else
    echo "Unsupported OS: $OS"
    exit 1
fi

INSTALL_DIR="/usr/local/bin"
# Fallback to ~/.local/bin if cannot write to /usr/local/bin
if [ ! -w "$INSTALL_DIR" ]; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
    # Check if in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo "Warning: $INSTALL_DIR is not in your PATH."
        echo "Add it with: export PATH=\"\$PATH:$INSTALL_DIR\""
    fi
fi

echo "Installing to $INSTALL_DIR..."

DOWNLOAD_URL="https://github.com/$GITHUB_REPO/releases/latest/download/$ASSET_NAME"
INSTALL_PATH="$INSTALL_DIR/lumi-tester"

# Try downloading with gh if available (best for private repos)
if command -v gh &> /dev/null && gh auth status &> /dev/null; then
    echo "Detected GitHub CLI. Using 'gh release download' for secure access..."
    if ! gh release download -R "$GITHUB_REPO" --pattern "$ASSET_NAME" --dir "/tmp" --clobber; then
        echo "Error: 'gh release download' failed. Ensure you have access to the repository."
        exit 1
    fi
    mv "/tmp/$ASSET_NAME" "$INSTALL_PATH"
else
    echo "Downloading $ASSET_NAME from $DOWNLOAD_URL..."
    # Use -f to fail on HTTP errors
    if ! curl -L -f -o "$INSTALL_PATH" "$DOWNLOAD_URL"; then
        echo "Error: Download failed. Check your internet connection or repository access."
        if [ "$GITHUB_REPO" = "Nghi-NV/nl-tester" ]; then
             echo "If this is a private repo, please ensure you have repository access."
        fi
        exit 1
    fi
fi

# Verify the file size (binary should be large, error messages are small)
FILE_SIZE=$(wc -c < "$INSTALL_PATH")
if [ "$FILE_SIZE" -lt 10000 ]; then
    if grep -q "<!DOCTYPE html>" "$INSTALL_PATH" 2>/dev/null || grep -q "Not Found" "$INSTALL_PATH" 2>/dev/null; then
        echo "Error: Downloaded file appears to be an error page or 'Not Found' message."
        echo "This usually happens with private repositories when using curl."
        echo "Recommendation: Install GitHub CLI ('gh'), run 'gh auth login', and try again."
        rm "$INSTALL_PATH"
        exit 1
    fi
fi

chmod +x "$INSTALL_PATH"

echo "lumi-tester installed successfully!"

echo "Initializing system components (ADB, Playwright)..."
"$INSTALL_PATH" system install --all

echo "Done! You can now use 'lumi-tester' command."
