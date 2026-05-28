#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?usage: generate-package-manifests.sh <version> <dist-dir>}"
DIST_DIR="${2:?usage: generate-package-manifests.sh <version> <dist-dir>}"
REPO="${LUMI_TESTER_REPO:-Nghi-NV/nl-tester}"
VERSION_NO_V="${VERSION#v}"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
SUMS="${DIST_DIR}/SHA256SUMS"

sha_for() {
  local asset="$1"
  awk -v asset="$asset" '$2 == asset { print $1 }' "$SUMS"
}

asset_url() {
  echo "${BASE_URL}/$1"
}

LINUX_X64="lumi-tester-x86_64-unknown-linux-gnu"
LINUX_ARM64="lumi-tester-aarch64-unknown-linux-gnu"
MAC_X64="lumi-tester-x86_64-apple-darwin"
MAC_ARM64="lumi-tester-aarch64-apple-darwin"
WIN_X64="lumi-tester-x86_64-pc-windows-msvc.exe"
WIN_ARM64="lumi-tester-aarch64-pc-windows-msvc.exe"

cat > "${DIST_DIR}/homebrew-lumi-tester.rb" <<EOF
class LumiTester < Formula
  desc "Multi-platform automation testing CLI"
  homepage "https://github.com/${REPO}"
  version "${VERSION_NO_V}"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "$(asset_url "$MAC_ARM64")", using: :nounzip
      sha256 "$(sha_for "$MAC_ARM64")"
    else
      url "$(asset_url "$MAC_X64")", using: :nounzip
      sha256 "$(sha_for "$MAC_X64")"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "$(asset_url "$LINUX_ARM64")", using: :nounzip
      sha256 "$(sha_for "$LINUX_ARM64")"
    else
      url "$(asset_url "$LINUX_X64")", using: :nounzip
      sha256 "$(sha_for "$LINUX_X64")"
    end
  end

  def install
    chmod 0755, cached_download
    bin.install cached_download => "lumi-tester"
  end

  def caveats
    <<~EOS
      Run 'lumi-tester system install --all' to install ADB and browser dependencies.
      Run 'lumi-tester ai install' to install the Codex skill and MCP server for AI-assisted test authoring/debugging.
    EOS
  end

  test do
    system "#{bin}/lumi-tester", "--version"
  end
end
EOF

cat > "${DIST_DIR}/scoop-lumi-tester.json" <<EOF
{
  "version": "${VERSION_NO_V}",
  "description": "Multi-platform automation testing CLI",
  "homepage": "https://github.com/${REPO}",
  "license": "MIT",
  "notes": "Run 'lumi-tester system install --all' for local drivers, then 'lumi-tester ai install' to install the Codex skill and MCP server.",
  "architecture": {
    "64bit": {
      "url": "$(asset_url "$WIN_X64")",
      "hash": "$(sha_for "$WIN_X64")",
      "bin": [
        [
          "${WIN_X64}",
          "lumi-tester.exe"
        ]
      ]
    },
    "arm64": {
      "url": "$(asset_url "$WIN_ARM64")",
      "hash": "$(sha_for "$WIN_ARM64")",
      "bin": [
        [
          "${WIN_ARM64}",
          "lumi-tester.exe"
        ]
      ]
    }
  },
  "checkver": "github",
  "autoupdate": {
    "architecture": {
      "64bit": {
        "url": "https://github.com/${REPO}/releases/download/v\$version/lumi-tester-x86_64-pc-windows-msvc.exe"
      },
      "arm64": {
        "url": "https://github.com/${REPO}/releases/download/v\$version/lumi-tester-aarch64-pc-windows-msvc.exe"
      }
    }
  }
}
EOF

cat > "${DIST_DIR}/winget-NghiNV.LumiTester.yaml" <<EOF
PackageIdentifier: NghiNV.LumiTester
PackageVersion: ${VERSION_NO_V}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.6.0
EOF

cat > "${DIST_DIR}/winget-NghiNV.LumiTester.locale.en-US.yaml" <<EOF
PackageIdentifier: NghiNV.LumiTester
PackageVersion: ${VERSION_NO_V}
PackageLocale: en-US
Publisher: Nghi NV
PackageName: Lumi Tester
License: MIT
ShortDescription: Multi-platform automation testing CLI
Description: Multi-platform automation testing CLI. Run 'lumi-tester ai install' after installation to install the Codex skill and MCP server for AI-assisted test authoring/debugging.
PackageUrl: https://github.com/${REPO}
ManifestType: defaultLocale
ManifestVersion: 1.6.0
EOF

cat > "${DIST_DIR}/winget-NghiNV.LumiTester.installer.yaml" <<EOF
PackageIdentifier: NghiNV.LumiTester
PackageVersion: ${VERSION_NO_V}
InstallerType: portable
Commands:
  - lumi-tester
Installers:
  - Architecture: x64
    InstallerUrl: $(asset_url "$WIN_X64")
    InstallerSha256: $(sha_for "$WIN_X64")
  - Architecture: arm64
    InstallerUrl: $(asset_url "$WIN_ARM64")
    InstallerSha256: $(sha_for "$WIN_ARM64")
ManifestType: installer
ManifestVersion: 1.6.0
EOF

echo "Generated package manager manifests in ${DIST_DIR}"
