#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Replace committed captures only when their decoded pixels changed materially.

set -euo pipefail
NEW_DIR="${1:?usage: sync-previews.sh <new-dir> <committed-dir>}"
OLD_DIR="${2:?usage: sync-previews.sh <new-dir> <committed-dir>}"
FUZZ="${PREVIEW_FUZZ:-2%}"
THRESHOLD="${PREVIEW_THRESHOLD:-0.005}"
TILE_THRESHOLD="${PREVIEW_TILE_THRESHOLD:-0.25}"
TILES="${PREVIEW_TILES:-16}"
OXIPNG_LEVEL="${PREVIEW_OXIPNG_LEVEL:-4}"

if command -v magick >/dev/null; then
    im_convert() { magick "$@"; }
    im_identify() { magick identify "$@"; }
elif command -v convert >/dev/null; then
    im_convert() { convert "$@"; }
    im_identify() { identify "$@"; }
else
    echo "error: ImageMagick is required" >&2
    exit 1
fi

diff_share() {
    im_convert "$1" "$2" -compose Difference -composite \
        -colorspace Gray -threshold "$FUZZ" -format '%[fx:mean]' info:
}
diff_tile_share() {
    im_convert "$1" "$2" -compose Difference -composite \
        -colorspace Gray -threshold "$FUZZ" -scale "${TILES}x${TILES}!" \
        -format '%[fx:maxima]' info:
}
require_share() {
    [[ "$1" =~ ^[0-9]+(\.[0-9]+)?([eE][-+]?[0-9]+)?$ ]] || {
        echo "error: could not compare $2 (got '$1')" >&2
        exit 1
    }
}
as_pct() { awk "BEGIN { printf \"%.2f\", $1 * 100 }"; }

shopt -s nullglob
shots=( "$NEW_DIR"/preview-*.png "$NEW_DIR"/variants/preview-*.png \
        "$NEW_DIR"/locales/*/preview-*.png )
((${#shots[@]})) || { echo "error: no captured previews in $NEW_DIR" >&2; exit 1; }

if [[ "${PREVIEW_OPTIMIZE:-1}" == "1" ]] && command -v oxipng >/dev/null; then
    echo "==> Optimizing ${#shots[@]} captured screenshots"
    oxipng -o "$OXIPNG_LEVEL" --strip safe --quiet "${shots[@]}" || \
        echo "warning: oxipng failed; continuing with valid unoptimized PNGs" >&2
fi

changed=0
added=0
unchanged=0
removed=0
for new in "${shots[@]}"; do
    name="${new#"$NEW_DIR"/}"
    old="$OLD_DIR/$name"
    mkdir -p "$(dirname "$old")"
    if [[ ! -f "$old" ]]; then
        cp "$new" "$old"
        echo "added    $name"
        added=$((added + 1))
        continue
    fi
    new_size="$(im_identify -format '%wx%h' "$new")"
    old_size="$(im_identify -format '%wx%h' "$old")"
    if [[ "$new_size" != "$old_size" ]]; then
        cp "$new" "$old"
        echo "changed  $name (size $old_size -> $new_size)"
        changed=$((changed + 1))
        continue
    fi
    share="$(diff_share "$old" "$new")"
    tile_share="$(diff_tile_share "$old" "$new")"
    require_share "$share" "$name"
    require_share "$tile_share" "$name"
    if awk "BEGIN { exit !($share > $THRESHOLD || $tile_share > $TILE_THRESHOLD) }"; then
        cp "$new" "$old"
        printf 'changed  %s (%s%% overall, %s%% worst tile)\n' \
            "$name" "$(as_pct "$share")" "$(as_pct "$tile_share")"
        changed=$((changed + 1))
    else
        printf 'unchanged %s (%s%% overall, %s%% worst tile)\n' \
            "$name" "$(as_pct "$share")" "$(as_pct "$tile_share")"
        unchanged=$((unchanged + 1))
    fi
done

# A complete capture is authoritative. Filtered developer runs must leave
# screenshots outside their selected scope untouched.
if [[ -z "${SHOTS:-}" && -z "${VARIANTS_FILTER:-}" && -z "${LOCALES_FILTER:-}" ]]; then
    committed=( "$OLD_DIR"/preview-*.png "$OLD_DIR"/variants/preview-*.png \
                "$OLD_DIR"/locales/*/preview-*.png )
    for old in "${committed[@]}"; do
        name="${old#"$OLD_DIR"/}"
        [[ -f "$NEW_DIR/$name" ]] && continue
        rm -f "$old"
        echo "removed  $name"
        removed=$((removed + 1))
    done
fi

printf '%s changed, %s added, %s removed, %s unchanged\n' \
    "$changed" "$added" "$removed" "$unchanged"
