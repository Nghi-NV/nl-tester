#!/usr/bin/env bash
set -euo pipefail

REPO="${LUMI_TESTER_REPO:-Nghi-NV/nl-tester}"
VERSION="${LUMI_TESTER_VERSION:-latest}"
INSTALL_DIR="${LUMI_INSTALL_DIR:-}"
SKIP_SYSTEM_INSTALL="${LUMI_SKIP_SYSTEM_INSTALL:-0}"
TMP_DIR=""

cleanup() {
  if [ -n "$TMP_DIR" ]; then
    rm -rf "$TMP_DIR"
  fi
}

say() {
  printf '%s\n' "$*"
}

fail() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Missing required command: $1"
}

detect_asset() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os:$arch" in
    Darwin:arm64) echo "lumi-tester-aarch64-apple-darwin" ;;
    Darwin:x86_64) echo "lumi-tester-x86_64-apple-darwin" ;;
    Linux:x86_64|Linux:amd64) echo "lumi-tester-x86_64-unknown-linux-gnu" ;;
    Linux:aarch64|Linux:arm64) echo "lumi-tester-aarch64-unknown-linux-gnu" ;;
    *) fail "Unsupported platform: $os $arch" ;;
  esac
}

release_base_url() {
  if [ "$VERSION" = "latest" ]; then
    echo "https://github.com/$REPO/releases/latest/download"
  else
    echo "https://github.com/$REPO/releases/download/$VERSION"
  fi
}

default_install_dir() {
  if [ -n "$INSTALL_DIR" ]; then
    echo "$INSTALL_DIR"
    return
  fi

  if [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
    echo "/usr/local/bin"
  else
    echo "$HOME/.local/bin"
  fi
}

download() {
  local url="$1"
  local output="$2"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$output"
  elif command -v wget >/dev/null 2>&1; then
    wget -q "$url" -O "$output"
  else
    fail "Missing curl or wget"
  fi
}

verify_checksum() {
  local checksums="$1"
  local asset="$2"
  local file="$3"

  [ -s "$checksums" ] || return 0
  command -v shasum >/dev/null 2>&1 || command -v sha256sum >/dev/null 2>&1 || return 0

  local expected
  expected="$(grep -E "[[:space:]]${asset}$" "$checksums" | awk '{print $1}' | head -n1 || true)"
  [ -n "$expected" ] || return 0

  local actual
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$file" | awk '{print $1}')"
  else
    actual="$(shasum -a 256 "$file" | awk '{print $1}')"
  fi

  [ "$expected" = "$actual" ] || fail "Checksum mismatch for $asset"
  say "Checksum verified."
}

main() {
  need_cmd uname
  need_cmd mktemp

  local asset base_url install_dir tmp_asset tmp_checksums install_path
  asset="$(detect_asset)"
  base_url="$(release_base_url)"
  install_dir="$(default_install_dir)"
  TMP_DIR="$(mktemp -d)"
  tmp_asset="$TMP_DIR/$asset"
  tmp_checksums="$TMP_DIR/SHA256SUMS"
  install_path="$install_dir/lumi-tester"

  trap cleanup EXIT

  say "Installing lumi-tester"
  say "  Repository: $REPO"
  say "  Version: $VERSION"
  say "  Asset: $asset"
  say "  Install dir: $install_dir"

  mkdir -p "$install_dir"

  say "Downloading $base_url/$asset"
  download "$base_url/$asset" "$tmp_asset"

  if download "$base_url/SHA256SUMS" "$tmp_checksums" >/dev/null 2>&1; then
    verify_checksum "$tmp_checksums" "$asset" "$tmp_asset"
  else
    say "Checksum file not found; skipping checksum verification."
  fi

  if [ ! -s "$tmp_asset" ]; then
    fail "Downloaded file is empty"
  fi

  install -m 0755 "$tmp_asset" "$install_path"

  if ! command -v lumi-tester >/dev/null 2>&1 && [[ ":$PATH:" != *":$install_dir:"* ]]; then
    say "Warning: $install_dir is not in PATH."
    say "Add it with: export PATH=\"\$PATH:$install_dir\""
  fi

  say "Installed: $install_path"
  "$install_path" --version || true

  if [ "$SKIP_SYSTEM_INSTALL" != "1" ]; then
    say "Initializing drivers and browser dependencies..."
    "$install_path" system install --all
  else
    say "Skipping system install because LUMI_SKIP_SYSTEM_INSTALL=1"
  fi

  say "Done. Run: lumi-tester --help"
}

main "$@"
