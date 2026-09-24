#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only

catalog_has_messages() {
    grep -Eq '^[a-z0-9-]+[[:space:]]*=' "$1"
}

locale_capture_changed() {
    local locale="$1" differs="$2"
    if ((differs == 0)); then
        printf 'warning: %s: every shot is identical to English; the locale did not reach visible UI\n' \
            "$locale" >&2
        return 1
    fi
}
