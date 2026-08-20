#!/usr/bin/env bash
set -euo pipefail

TARGET=""
OUTPUT_TAR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET="$2"
      shift 2
      ;;
    --output-tar)
      OUTPUT_TAR="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$TARGET" || -z "$OUTPUT_TAR" ]]; then
  echo "Usage: pack-mcp-bundle.sh --target <TARGET> --output-tar <OUTPUT_TAR>" >&2
  exit 1
fi

mkdir -p "$(dirname "${OUTPUT_TAR}")"
ABS_OUTPUT_TAR="$(cd "$(dirname "${OUTPUT_TAR}")" && pwd)/$(basename "${OUTPUT_TAR}")"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MCP_DIR="${ROOT}/lumi-tester-mcp"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

case "$TARGET" in
  x86_64-unknown-linux-gnu)
    BIN_NAME="lumi-tester"
    PLATFORM_DIR="linux-x64"
    ;;
  aarch64-unknown-linux-gnu)
    BIN_NAME="lumi-tester"
    PLATFORM_DIR="linux-arm64"
    ;;
  x86_64-apple-darwin)
    BIN_NAME="lumi-tester"
    PLATFORM_DIR="darwin-x64"
    ;;
  aarch64-apple-darwin)
    BIN_NAME="lumi-tester"
    PLATFORM_DIR="darwin-arm64"
    ;;
  x86_64-pc-windows-msvc)
    BIN_NAME="lumi-tester.exe"
    PLATFORM_DIR="win32-x64"
    ;;
  aarch64-pc-windows-msvc)
    BIN_NAME="lumi-tester.exe"
    PLATFORM_DIR="win32-arm64"
    ;;
  *)
    echo "Unknown target: $TARGET" >&2
    exit 1
    ;;
esac

BIN_SRC=""
if [[ -f "${ROOT}/lumi-tester/target/${TARGET}/release/${BIN_NAME}" ]]; then
  BIN_SRC="${ROOT}/lumi-tester/target/${TARGET}/release/${BIN_NAME}"
elif [[ -f "${ROOT}/lumi-tester/target/release/${BIN_NAME}" ]]; then
  BIN_SRC="${ROOT}/lumi-tester/target/release/${BIN_NAME}"
elif [[ -f "${ROOT}/dist/lumi-tester-${TARGET}.exe" ]]; then
  BIN_SRC="${ROOT}/dist/lumi-tester-${TARGET}.exe"
elif [[ -f "${ROOT}/dist/lumi-tester-${TARGET}" ]]; then
  BIN_SRC="${ROOT}/dist/lumi-tester-${TARGET}"
elif [[ -f "${ROOT}/dist/${BIN_NAME}" ]]; then
  BIN_SRC="${ROOT}/dist/${BIN_NAME}"
else
  echo "Error: Could not find compiled binary for target ${TARGET}" >&2
  exit 1
fi

STAGE_DIR="${TMP_DIR}/package"
mkdir -p "${STAGE_DIR}/binaries/${PLATFORM_DIR}"
cp -R "${MCP_DIR}/src" "${STAGE_DIR}/"
cp -R "${MCP_DIR}/scripts" "${STAGE_DIR}/"
cp "${MCP_DIR}/package.json" "${STAGE_DIR}/"
cp "${MCP_DIR}/README.md" "${STAGE_DIR}/"

cp "${BIN_SRC}" "${STAGE_DIR}/binaries/${PLATFORM_DIR}/${BIN_NAME}"
if [[ "$BIN_NAME" != *.exe ]]; then
  chmod +x "${STAGE_DIR}/binaries/${PLATFORM_DIR}/${BIN_NAME}"
fi

cd "${STAGE_DIR}"
npm install --omit=dev --no-audit --no-fund

tar -czf "${ABS_OUTPUT_TAR}" -C "${TMP_DIR}" package
echo "Packaged MCP bundle: ${ABS_OUTPUT_TAR}"
