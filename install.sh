#!/bin/bash
set -e

# Repository configuration
REPO_OWNER="nghi-nv"
REPO_NAME="nl-tester"

# Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)     OS_NAME="linux";;
    Darwin*)    OS_NAME="darwin";;
    *)          echo "Unsupported OS: ${OS}"; exit 1;;
esac

# Detect Architecture
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64)    ARCH_NAME="amd64";;
    aarch64)   ARCH_NAME="arm64";;
    arm64)     ARCH_NAME="arm64";;
    *)         echo "Unsupported Architecture: ${ARCH}"; exit 1;;
esac

# Construct Asset Name
ASSET_NAME="lumi-tester-${OS_NAME}-${ARCH_NAME}"
if [ "${OS_NAME}" = "linux" ] && [ "${ARCH_NAME}" = "arm64" ]; then
    echo "Warning: arm64 linux build might not be available yet. Checking..."
fi

# Get Latest Release URL (GitHub API)
LATEST_RELEASE_URL="https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases/latest"
echo "Fetching latest release from $LATEST_RELEASE_URL..."

# Simple download using curl/wget to find the browser_download_url for the asset
DOWNLOAD_URL=$(curl -s $LATEST_RELEASE_URL | grep "browser_download_url" | grep "$ASSET_NAME" | cut -d '"' -f 4)

if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Could not find download URL for asset: $ASSET_NAME"
    echo "Available assets might not match your system."
    exit 1
fi

echo "Downloading $ASSET_NAME from $DOWNLOAD_URL..."

# Install Directory
INSTALL_DIR="/usr/local/bin"
if [ ! -w "$INSTALL_DIR" ]; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
    echo "Installing to $INSTALL_DIR (add this to your PATH if not present)"
fi

# Download
curl -L -o "$INSTALL_DIR/lumi-tester" "$DOWNLOAD_URL"
chmod +x "$INSTALL_DIR/lumi-tester"

echo "Successfully installed lumi-tester to $INSTALL_DIR/lumi-tester"
echo "Run 'lumi-tester --version' to verify."
