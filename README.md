# Compass

A clean, responsive magnetic compass for Linux desktops and phones, built with [libcosmic](https://github.com/pop-os/libcosmic).

<p>
  <img src="resources/icons/hicolor/scalable/apps/org.cosmic_utils.compass.svg" alt="Compass app icon" width="80">
</p>

![Compass preview](preview/preview-001.png)

[View more screenshots](preview/README.md)

## Features

- GPU-rendered 2D compass rose with degree ticks and cardinal/intercardinal labels
- Fixed screen-aligned crosshair, optional accelerometer-driven lean indicator, and top index with shortest-path heading smoothing
- Responsive layout tested at 240×320 and in short landscape windows
- SensorProxy compass integration, GeoClue coordinates/elevation, and localized OpenStreetMap place names
- A greyed-out compass with a fitted warning when the magnetometer is unavailable, independent GPS/location warnings, and continued location acquisition
- Fixed 42° (`--demo --still`), configurable fixed-heading, and naturally animated demonstration modes using Burg Eltz reference coordinates

## Run

```bash
just demo
cargo run -- --demo-heading 42
cargo run
```

`just demo` continuously simulates smooth, changing magnetometer headings without requiring sensor hardware. Both demonstration modes use the public Burg Eltz reference location (`50°12′18″ N 7°20′12″ E`, 320 m) rather than requesting the device location.

Without `--demo` or `--demo-heading`, the app uses the system services `net.hadess.SensorProxy` and `org.freedesktop.GeoClue2`. `ClaimCompass` may require launching from an active local desktop session so Polkit can identify the user. A remote or inactive session can be denied even when the service is running. `HasCompass=false` is authoritative; a cached numeric heading is ignored in that state.

GeoClue is requested independently of magnetometer availability. It may supply a GNSS fix or a less precise fallback such as Wi-Fi location. Coordinates, accuracy, and elevation are always derived from the broker. An incomplete GNSS driver therefore remains at “Waiting for a location reading…” or reports that no GPS or location source is available instead of displaying invented values.

For a live fix, Compass sends the displayed one-arcsecond coordinate to an OpenStreetMap Nominatim service to obtain a place name in the system locale. Lookups start automatically for new displayed coordinates; up to 512 coordinate-and-locale results are retained in a private per-user cache, and network lookups are serialized across concurrent Compass processes and limited to one per minute. `COMPASS_NOMINATIM_URL` switches to another compatible endpoint without rebuilding the app. The public endpoint is suitable only for low-volume use; distributors with more than a small user base must set that variable to a proxy or alternative provider. OpenStreetMap attribution is shown as a small bottom-right notice with each result and in About.

## Build and install

Requirements: Rust 1.98+, `just`, pkg-config, Wayland and XKB development libraries.

```bash
just check
just build-release
sudo just install
```

The Flatpak manifest grants Wayland, fallback X11, DRI, network access for Nominatim, and narrowly scoped system-bus access to SensorProxy and GeoClue.

## Packaging

The repository includes Flatpak, Alpine APK, cross-compilation, CI, and release blueprints. Run `just generate` after dependency or translation changes to refresh derived metadata and Flatpak Cargo sources.

## License

Licensed under GPL-3.0-only. Contributions intentionally submitted for inclusion are licensed under the same terms. Source files should carry `SPDX-License-Identifier: GPL-3.0-only`.
