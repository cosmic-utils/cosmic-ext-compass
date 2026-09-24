#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Capture Compass in fixed demo states under an isolated headless compositor.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SHOTS_CONF="$SCRIPT_DIR/shots.conf"
OUT_DIR="${1:-$SCRIPT_DIR}"
WINDOW_TIMEOUT=60

# shellcheck source=preview/locale-policy.sh
source "$SCRIPT_DIR/locale-policy.sh"

log() { printf '\033[1;34m==>\033[0m %s\n' "$*" >&2; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

for tool in sway swaymsg grim wtype jq identify magick dbus-daemon; do
    command -v "$tool" >/dev/null || die "$tool is not installed"
done
[[ -f "$SHOTS_CONF" ]] || die "shot list not found: $SHOTS_CONF"

COMPASS="${COMPASS_BIN:-}"
if [[ -z "$COMPASS" ]]; then
    log "Building Compass (release-fast)"
    (cd "$REPO_DIR" && cargo build --locked --profile release-fast)
    COMPASS="${CARGO_TARGET_DIR:-$REPO_DIR/target}/release-fast/compass"
fi
[[ -x "$COMPASS" ]] || die "Compass binary is not executable: $COMPASS"
mkdir -p "$OUT_DIR/variants"

SESSION_DIR="$(mktemp -d)"
export HOME="$SESSION_DIR/home"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_CACHE_HOME="$HOME/.cache"
export XDG_RUNTIME_DIR="$SESSION_DIR/runtime"
mkdir -p "$HOME" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_CACHE_HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"

seed_theme() {
    local source seeded=0
    mkdir -p "$XDG_CONFIG_HOME/cosmic"
    for source in /usr/share/cosmic/com.system76.CosmicTheme.*; do
        [[ -d "$source" ]] || continue
        rm -rf "$XDG_CONFIG_HOME/cosmic/$(basename "$source")"
        cp -r "$source" "$XDG_CONFIG_HOME/cosmic/"
        seeded=1
    done
    ((seeded == 1)) || die "COSMIC theme defaults are missing"
}
seed_theme

SWAY_PID=""
DBUS_PID=""
APP_PID=""
KEYBOARD_PID=""
cleanup() {
    local status=$?
    [[ -n "$APP_PID" ]] && kill "$APP_PID" 2>/dev/null || true
    [[ -n "$KEYBOARD_PID" ]] && kill "$KEYBOARD_PID" 2>/dev/null || true
    [[ -n "$SWAY_PID" ]] && kill "$SWAY_PID" 2>/dev/null || true
    [[ -n "$DBUS_PID" ]] && kill "$DBUS_PID" 2>/dev/null || true
    rm -rf "$SESSION_DIR"
    exit "$status"
}
trap cleanup EXIT INT TERM

if [[ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ]]; then
    dbus_out="$(dbus-daemon --session --print-address=1 --print-pid=1 --fork)"
    DBUS_SESSION_BUS_ADDRESS="$(sed -n '1p' <<<"$dbus_out")"
    DBUS_PID="$(sed -n '2p' <<<"$dbus_out")"
    export DBUS_SESSION_BUS_ADDRESS
fi

SWAY_CONFIG="$SESSION_DIR/sway.conf"
printf '%s\n' \
    'output HEADLESS-1 resolution 1920x1080 position 0 0 scale 1' \
    'default_border none' \
    'default_floating_border none' \
    'gaps inner 0' \
    'gaps outer 0' \
    'focus_follows_mouse no' \
    'for_window [title=".*"] floating enable' \
    'for_window [app_id=".*"] floating enable' >"$SWAY_CONFIG"
sway --config "$SWAY_CONFIG" >"$SESSION_DIR/sway.log" 2>&1 &
SWAY_PID=$!
for _ in $(seq 50); do
    SWAYSOCK="$(compgen -G "$XDG_RUNTIME_DIR/sway-ipc.*.sock" | head -n1 || true)"
    [[ -n "$SWAYSOCK" ]] && break
    sleep 0.2
done
[[ -n "${SWAYSOCK:-}" ]] || { cat "$SESSION_DIR/sway.log" >&2; die "sway did not start"; }
export SWAYSOCK
WAYLAND_DISPLAY=""
for _ in $(seq 50); do
    while IFS= read -r socket; do
        [[ "$socket" == *.lock ]] && continue
        WAYLAND_DISPLAY="$(basename "$socket")"
        break
    done < <(compgen -G "$XDG_RUNTIME_DIR/wayland-*" || true)
    [[ -n "$WAYLAND_DISPLAY" ]] && break
    sleep 0.2
done
[[ -n "$WAYLAND_DISPLAY" ]] || die "sway did not create a Wayland socket"
export WAYLAND_DISPLAY

# Keep a virtual keyboard attached so winit gives the window active focus.
KEYBOARD_FIFO="$SESSION_DIR/keyboard.fifo"
mkfifo "$KEYBOARD_FIFO"
wtype - <"$KEYBOARD_FIFO" &
KEYBOARD_PID=$!
exec 9>"$KEYBOARD_FIFO"
for _ in $(seq 20); do
    [[ "$(swaymsg -t get_seats | jq -r 'map(.capabilities) | add')" != "0" ]] && break
    sleep 0.1
done
[[ "$(swaymsg -t get_seats | jq -r 'map(.capabilities) | add')" != "0" ]] || \
    die "virtual keyboard did not attach"

window_geometry() {
    swaymsg -t get_tree | jq -r --argjson pid "$1" '
        .. | objects | select(.pid? == $pid and .type? == "floating_con")
        | "\(.rect.x + .window_rect.x),\(.rect.y + .window_rect.y) \(.window_rect.width)x\(.window_rect.height)"
    ' | head -n1
}
kill_app() {
    [[ -n "$APP_PID" ]] || return 0
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
    APP_PID=""
}
keep_log() {
    [[ "${KEEP_LOGS:-0}" == "1" ]] || return 0
    cp "$SESSION_DIR/compass-$1.log" "$OUT_DIR/compass-$1.log" 2>/dev/null || true
}

# Compare decoded pixels rather than PNG bytes, which may differ only in their
# encoding metadata.
images_identical() {
    local share
    share="$(magick "$1" "$2" -compose Difference -composite \
        -colorspace Gray -threshold 2% -format '%[fx:mean]' info: 2>/dev/null || echo "")"
    [[ -n "$share" ]] || return 1
    awk -v s="$share" 'BEGIN { exit !(s == 0) }'
}

capture_shot() {
    local num="$1" window="$2" heading="$3" settle_ms="$4"
    local description="$5" theme="$6" out="$7"
    local width="${window%x*}" height="${window#*x}"
    local tag="${out#"$OUT_DIR"/}"
    tag="${tag%.png}"
    tag="${tag//\//-}"
    [[ "$settle_ms" =~ ^[0-9]+$ ]] || { warn "$tag: invalid settle time"; return 1; }

    local args=(
        --demo-heading "$heading"
        --preview-window "$window"
        --preview-theme "$theme"
    )

    log "$tag: $description ($window, $theme)"
    "$COMPASS" "${args[@]}" >"$SESSION_DIR/compass-$tag.log" 2>&1 &
    APP_PID=$!
    local geometry="" waited=0
    while ((waited < WINDOW_TIMEOUT * 5)); do
        if ! kill -0 "$APP_PID" 2>/dev/null; then
            cat "$SESSION_DIR/compass-$tag.log" >&2
            APP_PID=""
            warn "$tag: app exited before its window appeared"
            return 1
        fi
        geometry="$(window_geometry "$APP_PID")"
        [[ -n "$geometry" ]] && break
        sleep 0.2
        waited=$((waited + 1))
    done
    [[ -n "$geometry" ]] || { warn "$tag: window never appeared"; kill_app; return 1; }
    swaymsg "[pid=$APP_PID] resize set width ${width}px height ${height}px" >/dev/null || {
        warn "$tag: could not resize window"; kill_app; return 1;
    }
    swaymsg "[pid=$APP_PID] focus" >/dev/null || true
    sleep "$(awk -v ms="$settle_ms" 'BEGIN { print ms / 1000 }')"
    geometry="$(window_geometry "$APP_PID")"
    [[ -n "$geometry" ]] || { warn "$tag: window disappeared"; kill_app; return 1; }
    grim -g "$geometry" "$out" || { warn "$tag: grim failed"; kill_app; return 1; }
    kill_app
    keep_log "$tag"
    local actual
    actual="$(identify -format '%wx%h' "$out" 2>/dev/null || true)"
    [[ "$actual" == "$window" ]] || { warn "$tag: captured $actual, expected $window"; return 1; }
}

VARIANTS=("dark|dark" "light|light")
PUBLISHED_VARIANT=dark
BASE_LOCALE=en
selected="${SHOTS:-}"
selected_variants="${VARIANTS_FILTER:-}"
failed=0
LOCALES=("$BASE_LOCALE")
if [[ "${LOCALES_FILTER:-}" != "none" ]]; then
    for dir in "$REPO_DIR"/i18n/*/; do
        locale="$(basename "$dir")"
        [[ "$locale" == "$BASE_LOCALE" || ! -f "$dir/compass.ftl" ]] && continue
        catalog_has_messages "$dir/compass.ftl" || continue
        [[ -n "${LOCALES_FILTER:-}" && ",${LOCALES_FILTER}," != *",$locale,"* ]] && continue
        LOCALES+=("$locale")
    done
fi
log "Languages: ${LOCALES[*]}"

for locale in "${LOCALES[@]}"; do
    export LANGUAGE="$locale" LANG="$locale.UTF-8"
    if [[ "$locale" == "$BASE_LOCALE" ]]; then
        locale_variants=("${VARIANTS[@]}")
        locale_dir=""
    else
        locale_variants=("dark|dark")
        locale_dir="$OUT_DIR/locales/$locale"
        mkdir -p "$locale_dir"
        locale_differs=0
        locale_captures=0
    fi
    while IFS='|' read -r num window heading settle_ms description; do
        [[ -z "${num// }" || "${num:0:1}" == "#" ]] && continue
        [[ -n "$selected" && ",$selected," != *",$num,"* ]] && continue
        for variant in "${locale_variants[@]}"; do
            IFS='|' read -r theme name <<<"$variant"
            [[ -n "$selected_variants" && ",$selected_variants," != *",$name,"* ]] && continue
            if [[ -n "$locale_dir" ]]; then
                out="$locale_dir/preview-$num.png"
            elif [[ "$name" == "$PUBLISHED_VARIANT" ]]; then
                out="$OUT_DIR/preview-$num.png"
            else
                out="$OUT_DIR/variants/preview-$num-$name.png"
            fi
            if ! capture_shot "$num" "$window" "$heading" "$settle_ms" \
                    "$description" "$theme" "$out"; then
                failed=$((failed + 1))
                [[ "${KEEP_GOING:-0}" == "1" ]] || die "preview-$num ($name, $locale) failed"
                continue
            fi
            if [[ -n "$locale_dir" && -f "$OUT_DIR/preview-$num.png" ]]; then
                locale_captures=$((locale_captures + 1))
                if ! images_identical "$OUT_DIR/preview-$num.png" "$out"; then
                    locale_differs=1
                fi
            fi
        done
    done <"$SHOTS_CONF"
    if [[ -n "$locale_dir" ]] && ((locale_captures > 0)) && \
            ! locale_capture_changed "$locale" "$locale_differs"; then
        failed=$((failed + 1))
        [[ "${KEEP_GOING:-0}" == "1" ]] || die "$locale did not apply to visible UI"
    fi
done
((failed == 0)) || die "$failed screenshot(s) failed"
log "All previews captured into $OUT_DIR"
