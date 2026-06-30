#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/camera-profile.sh calibrate <profile_name> [port]
  scripts/camera-profile.sh detect <profile_name>
  scripts/camera-profile.sh observe <profile_name> [port]

Examples:
  scripts/camera-profile.sh calibrate lab_switch4_camera
  scripts/camera-profile.sh detect lab_switch4_camera
  scripts/camera-profile.sh observe lab_switch4_camera 9445
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_DIR="$(cd "$ROOT_DIR/.." && pwd)"
ENV_FILE="$REPO_DIR/.env"

read_env_value() {
  local key="$1"
  if [[ ! -f "$ENV_FILE" ]]; then
    return 1
  fi
  awk -v key="$key" '
    BEGIN { FS = "=" }
    $0 ~ "^[[:space:]]*" key "[[:space:]]*=" {
      sub(/^[^=]*=/, "", $0)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0)
      gsub(/^"|"$/, "", $0)
      gsub(/^'\''|'\''$/, "", $0)
      print $0
      exit
    }
  ' "$ENV_FILE"
}

cmd="${1:-}"
profile_name="${2:-}"
port="${3:-9444}"

if [[ -z "$cmd" || -z "$profile_name" ]]; then
  usage
  exit 2
fi

rtsp="${CAMERA_RTSP:-$(read_env_value CAMERA_RTSP || true)}"
if [[ -z "$rtsp" ]]; then
  echo "Missing CAMERA_RTSP. Add CAMERA_RTSP=... to $ENV_FILE or export it first." >&2
  exit 1
fi

profile_path="e2e/workspaces/lumi_life/profiles/${profile_name%.json}.json"

cd "$ROOT_DIR"
case "$cmd" in
  calibrate)
    exec cargo run -- camera calibrate \
      --rtsp "$rtsp" \
      --profile "$profile_path" \
      --port "$port" \
      --transport tcp
    ;;
  observe)
    exec cargo run -- camera calibrate \
      --rtsp "$rtsp" \
      --profile "$profile_path" \
      --port "$port" \
      --transport tcp \
      --observe
    ;;
  detect)
    exec cargo run -- camera detect \
      --rtsp "$rtsp" \
      --profile "$profile_path" \
      --transport tcp
    ;;
  *)
    usage
    exit 2
    ;;
esac
