#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

cat > "${TMP_DIR}/SHA256SUMS" <<'EOF'
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  lumi-tester-x86_64-unknown-linux-gnu
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  lumi-tester-aarch64-unknown-linux-gnu
cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc  lumi-tester-x86_64-apple-darwin
dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd  lumi-tester-aarch64-apple-darwin
eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee  lumi-tester-x86_64-pc-windows-msvc.exe
ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff  lumi-tester-aarch64-pc-windows-msvc.exe
EOF

bash "${ROOT}/lumi-tester/scripts/generate-package-manifests.sh" v9.9.9 "$TMP_DIR" >/dev/null

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "missing generated manifest: $path" >&2
    exit 1
  fi
}

require_text() {
  local path="$1"
  local text="$2"
  if ! grep -Fq "$text" "$path"; then
    echo "missing '$text' in $path" >&2
    exit 1
  fi
}

HOMEBREW="${TMP_DIR}/homebrew-lumi-tester.rb"
SCOOP="${TMP_DIR}/scoop-lumi-tester.json"
WINGET_VERSION="${TMP_DIR}/winget-NghiNV.LumiTester.yaml"
WINGET_LOCALE="${TMP_DIR}/winget-NghiNV.LumiTester.locale.en-US.yaml"
WINGET_INSTALLER="${TMP_DIR}/winget-NghiNV.LumiTester.installer.yaml"

require_file "$HOMEBREW"
require_file "$SCOOP"
require_file "$WINGET_VERSION"
require_file "$WINGET_LOCALE"
require_file "$WINGET_INSTALLER"

require_text "$HOMEBREW" "lumi-tester ai install"
require_text "$HOMEBREW" "Codex skill"
require_text "$HOMEBREW" "MCP server"
require_text "$SCOOP" '"notes"'
require_text "$SCOOP" "lumi-tester ai install"
require_text "$SCOOP" "Codex skill"
require_text "$SCOOP" "MCP server"
require_text "$WINGET_LOCALE" "Description:"
require_text "$WINGET_LOCALE" "lumi-tester ai install"
require_text "$WINGET_LOCALE" "Codex skill"
require_text "$WINGET_LOCALE" "MCP server"

echo "Package manager manifest smoke test passed"
