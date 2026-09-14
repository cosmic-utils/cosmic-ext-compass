#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Refresh the resolved RPM environment lock intentionally.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

ENGINE="${CONTAINER_ENGINE:-}"
if [[ -z "$ENGINE" ]]; then
    if command -v podman >/dev/null; then
        ENGINE=podman
    elif command -v docker >/dev/null; then
        ENGINE=docker
    else
        echo "error: neither podman nor docker is installed" >&2
        exit 1
    fi
fi

IMAGE="${PREVIEW_IMAGE:-compass-previews}"
UNLOCKED_IMAGE="$IMAGE-lock-refresh"
cleanup() {
    "$ENGINE" image rm "$UNLOCKED_IMAGE" >/dev/null 2>&1 || true
}
trap cleanup EXIT

printf '==> Resolving the preview environment from the pinned base image\n'
"$ENGINE" build --build-arg VERIFY_LOCKS=0 -t "$UNLOCKED_IMAGE" \
    -f "$SCRIPT_DIR/Containerfile" "$REPO_DIR"

"$ENGINE" run --rm "$UNLOCKED_IMAGE" -c \
    "rpm -qa --qf '%{NAME}-%{EPOCHNUM}:%{VERSION}-%{RELEASE}.%{ARCH}\\n' | LC_ALL=C sort" \
    >"$SCRIPT_DIR/environment.lock"

printf '==> Rebuilding and verifying the refreshed lock\n'
"$ENGINE" build -t "$IMAGE" -f "$SCRIPT_DIR/Containerfile" "$REPO_DIR"
printf '==> Updated preview/environment.lock\n'
