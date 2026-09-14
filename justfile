# SPDX-License-Identifier: GPL-3.0-only

name := 'compass'
apk-pkgname := 'cosmic-ext-compass'
export APPID := 'io.github.cosmic_utils.compass'
rootdir := ''
prefix := '/usr'
base-dir := absolute_path(clean(rootdir / prefix))
cargo-target-dir := env('CARGO_TARGET_DIR', 'target')
bin-src := cargo-target-dir / 'release' / name
icons-src := 'resources/icons/hicolor'
icons-dst := base-dir / 'share/icons/hicolor'

# Build an optimized binary.
default: build-release
build-debug *args:
    cargo build {{args}}
build-release *args:
    cargo build --release {{args}}

fmt:
    cargo fmt
fmt-check:
    cargo fmt --check
cargo-check *args:
    cargo check --all-features {{args}}
test *args:
    cargo test {{args}}
clippy *args:
    cargo clippy --all-features {{args}} -- -D warnings
check: fmt-check cargo-check test clippy

run *args:
    env RUST_LOG=compass=info cargo run --profile release-fast -- {{args}}
demo:
    env RUST_LOG=compass=info cargo run --profile release-fast -- --demo

generate: generate-metadata generate-icons flatpak-cargo-sources
generate-metadata:
    python3 scripts/gen-metadata.py
generate-icons:
    #!/usr/bin/env bash
    set -euo pipefail
    svg="{{icons-src}}/scalable/apps/{{APPID}}.svg"
    test -f "$svg"
    for size in 16 24 32 48 64 128 256 512 1024; do
        dir="{{icons-src}}/${size}x${size}/apps"
        mkdir -p "$dir"
        magick -background none -density 384 "$svg" -resize "${size}x${size}" -define png:exclude-chunks=date,time "$dir/{{APPID}}.png"
    done
validate-metadata:
    desktop-file-validate resources/{{APPID}}.desktop
    appstreamcli validate --no-net resources/{{APPID}}.metainfo.xml

install:
    install -Dm0755 {{bin-src}} {{base-dir}}/bin/{{name}}
    install -Dm0644 resources/{{APPID}}.desktop {{base-dir}}/share/applications/{{APPID}}.desktop
    install -Dm0644 resources/{{APPID}}.metainfo.xml {{base-dir}}/share/metainfo/{{APPID}}.metainfo.xml
    install -Dm0644 {{icons-src}}/scalable/apps/{{APPID}}.svg {{icons-dst}}/scalable/apps/{{APPID}}.svg
    for size in 16x16 24x24 32x32 48x48 64x64 128x128 256x256 512x512 1024x1024; do install -Dm0644 "{{icons-src}}/$size/apps/{{APPID}}.png" "{{icons-dst}}/$size/apps/{{APPID}}.png"; done
uninstall:
    rm -f {{base-dir}}/bin/{{name}} {{base-dir}}/share/applications/{{APPID}}.desktop {{base-dir}}/share/metainfo/{{APPID}}.metainfo.xml {{icons-dst}}/scalable/apps/{{APPID}}.svg
    for size in 16x16 24x24 32x32 48x48 64x64 128x128 256x256 512x512 1024x1024; do rm -f "{{icons-dst}}/$size/apps/{{APPID}}.png"; done

vendor:
    mkdir -p .cargo
    cargo vendor --sync Cargo.toml > .cargo/config.toml
    tar pcf vendor.tar .cargo vendor
    rm -rf .cargo vendor
build-vendored *args:
    rm -rf vendor
    tar pxf vendor.tar
    cargo build --release --frozen --offline {{args}}

get-version:
    @cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])'

flatpak-cargo-sources:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -f flatpak-cargo-generator.py ]; then
        curl -fLo flatpak-cargo-generator.py https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/de2225a6dee4818c1339b3cdbf29f90c471fcb7e/cargo/flatpak-cargo-generator.py
    fi
    if python3 -c 'import aiohttp,tomlkit' 2>/dev/null; then
        python3 flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json
    elif command -v uv >/dev/null 2>&1; then
        uv run --with aiohttp==3.14.3 --with tomlkit==0.15.1 python3 flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json
    elif [ -x "$HOME/.hermes/bin/uv" ]; then
        "$HOME/.hermes/bin/uv" run --with aiohttp==3.14.3 --with tomlkit==0.15.1 python3 flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json
    else
        python3 -m venv .flatpak-venv
        .flatpak-venv/bin/pip install --quiet aiohttp==3.14.3 tomlkit==0.15.1
        .flatpak-venv/bin/python flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json
    fi

flatpak-build: flatpak-cargo-sources
    just get-version > .flatpak-version
    flatpak-builder --user --install --force-clean build-dir {{APPID}}.yml
    rm -f .flatpak-version
flatpak-bundle arch='x86_64':
    just get-version > .flatpak-version
    flatpak-builder --repo=repo --force-clean --arch={{arch}} build-dir {{APPID}}.yml
    flatpak build-bundle repo {{name}}-{{arch}}.flatpak {{APPID}} --arch={{arch}}
    rm -f .flatpak-version
flatpak-run:
    flatpak run {{APPID}}
flatpak-clean:
    rm -rf build-dir .flatpak-builder repo .flatpak-version {{name}}-*.flatpak

apk-build arch='x86_64':
    #!/usr/bin/env bash
    set -euo pipefail
    case '{{arch}}' in
        x86_64) target=x86_64-unknown-linux-musl; platform=linux/amd64 ;;
        aarch64) target=aarch64-unknown-linux-musl; platform=linux/aarch64 ;;
        *) echo 'supported APK architectures: x86_64, aarch64'; exit 2 ;;
    esac
    cross build --locked --target "$target" --profile release-fast
    mkdir -p apk-out
    podman build --platform "$platform" -t compass-apk -f docker/Dockerfile.apk .
    podman run --rm --platform "$platform" -v "$PWD":/src:ro -v "$PWD/apk-out":/out -v "$PWD/target/$target/release-fast/compass":/prebuilt/compass:ro compass-apk
clean-all:
    cargo clean
    rm -rf vendor .cargo .flatpak-venv apk-out cargo-sources.json
