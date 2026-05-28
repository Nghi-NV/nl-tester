#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?usage: publish-package-manager-manifests.sh <version> <dist-dir>}"
DIST_DIR="${2:?usage: publish-package-manager-manifests.sh <version> <dist-dir>}"
HOMEBREW_TAP_REPO="${HOMEBREW_TAP_REPO:-Nghi-NV/homebrew-tap}"
SCOOP_BUCKET_REPO="${SCOOP_BUCKET_REPO:-Nghi-NV/scoop-bucket}"
DRY_RUN="${PUBLISH_PACKAGE_MANAGERS_DRY_RUN:-0}"

need_file() {
  local file="$1"
  [ -f "$file" ] || {
    echo "Missing required file: $file" >&2
    exit 1
  }
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "Missing required command: $1" >&2
    exit 1
  }
}

copy_if_changed() {
  local source="$1"
  local dest="$2"
  mkdir -p "$(dirname "$dest")"
  cp "$source" "$dest"
}

ensure_repo() {
  local repo="$1"
  local description="$2"

  if gh repo view "$repo" >/dev/null 2>&1; then
    return
  fi

  echo "Creating repository: $repo"
  gh repo create "$repo" --public --description "$description"
}

publish_repo_file() {
  local repo="$1"
  local description="$2"
  local source="$3"
  local dest_rel="$4"
  local tmp_dir repo_dir

  tmp_dir="$(mktemp -d)"
  repo_dir="$tmp_dir/repo"

  ensure_repo "$repo" "$description"
  gh repo clone "$repo" "$repo_dir" -- --quiet

  copy_if_changed "$source" "$repo_dir/$dest_rel"

  (
    cd "$repo_dir"
    git config user.name "github-actions[bot]"
    git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
    git add "$dest_rel"
    if git diff --cached --quiet; then
      echo "$repo already up to date: $dest_rel"
      exit 0
    fi
    git commit -m "Update lumi-tester ${VERSION}"
    if [ "$DRY_RUN" = "1" ]; then
      echo "Dry run enabled; not pushing $repo"
    else
      git push
    fi
  )

  rm -rf "$tmp_dir"
}

main() {
  need_cmd gh
  need_cmd git
  need_file "$DIST_DIR/homebrew-lumi-tester.rb"
  need_file "$DIST_DIR/scoop-lumi-tester.json"

  publish_repo_file \
    "$HOMEBREW_TAP_REPO" \
    "Homebrew tap for Lumi Tester" \
    "$DIST_DIR/homebrew-lumi-tester.rb" \
    "Formula/lumi-tester.rb"

  publish_repo_file \
    "$SCOOP_BUCKET_REPO" \
    "Scoop bucket for Lumi Tester" \
    "$DIST_DIR/scoop-lumi-tester.json" \
    "bucket/lumi-tester.json"

  local tap_owner tap_repo tap_name scoop_owner
  tap_owner="${HOMEBREW_TAP_REPO%%/*}"
  tap_repo="${HOMEBREW_TAP_REPO#*/}"
  tap_name="${tap_repo#homebrew-}"
  scoop_owner="${SCOOP_BUCKET_REPO%%/*}"

  echo "Package manager manifests published for ${VERSION}."
  echo "Homebrew: brew install ${tap_owner}/${tap_name}/lumi-tester"
  echo "Scoop: scoop bucket add ${scoop_owner} https://github.com/${SCOOP_BUCKET_REPO}.git && scoop install lumi-tester"
}

main "$@"
