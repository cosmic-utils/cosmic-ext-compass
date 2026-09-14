# Compass

A clean, responsive magnetic compass for Linux desktops and phones, built with [libcosmic](https://github.com/pop-os/libcosmic).

<p>
  <img src="resources/icons/hicolor/scalable/apps/io.github.cosmic_utils.compass.svg" alt="Compass app icon" width="80">
</p>

## Features

- GPU-rendered 2D compass rose with degree ticks and cardinal/intercardinal labels
- Fixed top index with shortest-path heading smoothing
- Responsive layout tested at 240×320 and in short landscape windows
- SensorProxy integration through its dedicated Compass child object
- Centered unavailable and access-denied messages that replace the compass
- Fixed-heading and naturally animated demonstration modes

## Run

```bash
just demo
cargo run -- --demo-heading 42
cargo run
```

`just demo` continuously simulates smooth, changing magnetometer headings without requiring sensor hardware.

Without `--demo` or `--demo-heading`, the app uses the system service `net.hadess.SensorProxy`. `ClaimCompass` may require launching from an active local desktop session so Polkit can identify the user. A remote or inactive session can be denied even when the service is running. `HasCompass=false` is authoritative; a cached numeric heading is ignored in that state.

## Build and install

Requirements: Rust 1.98+, `just`, pkg-config, Wayland and XKB development libraries.

```bash
just check
just build-release
sudo just install
```

The Flatpak manifest grants only Wayland, fallback X11, DRI, and system-bus access to SensorProxy.

## Packaging

The repository includes Flatpak, Alpine APK, cross-compilation, CI, and release blueprints. Run `just generate` after dependency or translation changes to refresh derived metadata and Flatpak Cargo sources.

## License

Licensed under GPL-3.0-only. Contributions intentionally submitted for inclusion are licensed under the same terms. Source files should carry `SPDX-License-Identifier: GPL-3.0-only`.
