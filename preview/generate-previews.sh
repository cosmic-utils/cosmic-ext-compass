#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Regenerate deterministic Compass screenshots in the pinned container.

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
CACHE_DIR="$REPO_DIR/.preview-cache"
mkdir -p "$CACHE_DIR/cargo" "$CACHE_DIR/target"

if [[ "${NO_BUILD_IMAGE:-0}" != "1" ]]; then
    echo "==> Building $IMAGE with $ENGINE"
    "$ENGINE" build -t "$IMAGE" -f "$SCRIPT_DIR/Containerfile" "$REPO_DIR"
fi

run_args=(
    --rm
    --security-opt label=disable
    -v "$REPO_DIR:/src"
    -v "$CACHE_DIR/cargo:/cargo"
    -v "$CACHE_DIR/target:/target"
    -e CARGO_HOME=/cargo
    -e CARGO_TARGET_DIR=/target
)
for var in SHOTS VARIANTS_FILTER LOCALES_FILTER KEEP_GOING KEEP_LOGS \
           PREVIEW_FUZZ PREVIEW_THRESHOLD PREVIEW_TILE_THRESHOLD PREVIEW_TILES \
           PREVIEW_OPTIMIZE PREVIEW_OXIPNG_LEVEL; do
    if [[ -n "${!var:-}" ]]; then
        run_args+=( -e "$var=${!var}" )
    fi
done
if [[ "$ENGINE" == "docker" ]]; then
    run_args+=( --user "$(id -u):$(id -g)" -e HOME=/tmp/home )
fi

"$ENGINE" run "${run_args[@]}" "$IMAGE" \
    -c 'preview/capture-previews.sh /tmp/shots && preview/sync-previews.sh /tmp/shots preview'

echo "==> Done. Review with: git diff --stat preview/"
