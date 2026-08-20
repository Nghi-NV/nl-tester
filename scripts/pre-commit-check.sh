#!/usr/bin/env bash
# ==============================================================================
# 🛡️ Lumi Tester - Pre-Commit & Pre-Release Verification Pipeline
# ==============================================================================
# Script này kiểm tra toàn diện tất cả các điều kiện CI/CD trước khi commit hoặc release:
# 1. Cú pháp toàn bộ shell scripts (bash -n)
# 2. Đồng bộ hoá AI reference CSVs & schema JSON (check-ai-reference.py)
# 3. Tính toàn vẹn của package manager manifests (check-package-manifests.sh)
# 4. Unit tests của lumi-tester-mcp (Node test runner)
# 5. Unit tests của lumi-tester-vscode (VS Code Extension test suite)
# 6. Unit tests của lumi-tester core engine (cargo test)
# ==============================================================================

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

echo "================================================================="
echo "🔍 1. Validating Shell Script Syntaxes (bash -n)..."
echo "================================================================="
for script in \
  install.sh \
  lumi-tester/scripts/install.sh \
  lumi-tester/scripts/install-ai.sh \
  lumi-tester/scripts/generate-package-manifests.sh \
  lumi-tester/scripts/publish-package-manager-manifests.sh \
  lumi-tester/scripts/check-package-manifests.sh \
  lumi-tester/scripts/pack-mcp-bundle.sh \
  lumi-tester/scripts/camera-profile.sh; do
  if [[ -f "$script" ]]; then
    bash -n "$script"
    echo "  ✓ $script syntax ok"
  fi
done

echo ""
echo "================================================================="
echo "🤖 2. Checking AI References & Schema Sync..."
echo "================================================================="
python3 lumi-tester/scripts/check-ai-reference.py
echo "  ✓ AI references & schema in sync"

echo ""
echo "================================================================="
echo "📦 3. Checking Package Manager Manifests..."
echo "================================================================="
bash lumi-tester/scripts/check-package-manifests.sh
echo "  ✓ Package manifests check passed"

echo ""
echo "================================================================="
echo "🔌 4. Running lumi-tester-mcp Unit Tests..."
echo "================================================================="
(
  cd lumi-tester-mcp
  npm test
)
echo "  ✓ MCP unit tests passed"

echo ""
echo "================================================================="
echo "🎨 5. Running lumi-tester-vscode Extension Tests..."
echo "================================================================="
(
  cd lumi-tester-vscode
  npm test
)
echo "  ✓ VS Code extension tests passed"

echo ""
echo "================================================================="
echo "🦀 6. Running lumi-tester Rust Unit Tests..."
echo "================================================================="
cargo test --manifest-path lumi-tester/Cargo.toml
echo "  ✓ Rust core unit tests passed"

echo ""
echo "================================================================="
echo "✅ All Pre-Commit / Release checks passed successfully!"
echo "================================================================="
