#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# shellcheck source=preview/locale-policy.sh
source "$SCRIPT_DIR/locale-policy.sh"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

: >"$work/empty.ftl"
printf '# translation pending\n' >"$work/comments.ftl"
printf 'compass = Kompass\n' >"$work/translated.ftl"

if catalog_has_messages "$work/empty.ftl"; then
    echo "empty catalogs must not be captured" >&2
    exit 1
fi
if catalog_has_messages "$work/comments.ftl"; then
    echo "comment-only catalogs must not be captured" >&2
    exit 1
fi
catalog_has_messages "$work/translated.ftl"

if locale_capture_changed de 0 2>/dev/null; then
    echo "pixel-identical localized captures must fail validation" >&2
    exit 1
fi
locale_capture_changed de 1

for key in view location-accuracy location-elevation; do
    if ! grep -Eq "^${key} = .+" "$REPO_DIR/i18n/de/compass.ftl"; then
        echo "German catalog must translate visible preview key: $key" >&2
        exit 1
    fi
done

capture="$SCRIPT_DIR/capture-previews.sh"
grep -Fq 'source "$SCRIPT_DIR/locale-policy.sh"' "$capture"
grep -Fq 'catalog_has_messages "$dir/compass.ftl"' "$capture"
grep -Fq 'images_identical "$OUT_DIR/preview-$num.png" "$out"' "$capture"
grep -Fq 'locale_capture_changed "$locale" "$locale_differs"' "$capture"

workflow="$REPO_DIR/.github/workflows/regenerate-previews.yml"
grep -Fq -- "- 'preview/locale-policy.sh'" "$workflow"

echo "preview localization policy passed"
