// SPDX-License-Identifier: GPL-3.0-only

use compass::app::{CompassApp, Flags, Message};
use cosmic::{Application, Core};

fn initialized_app() -> CompassApp {
    compass::i18n::init(&[]);
    let mut core = Core::default();
    core.window.show_headerbar = false;
    core.window.content_container = false;
    core.window.use_template = false;
    core.window.header_title.clear();
    let (app, _) = <CompassApp as Application>::init(
        core,
        Flags {
            demo_heading: Some(0.0),
            demo_motion: false,
            initial_theme: None,
        },
    );
    app
}

#[test]
fn regular_window_uses_the_standard_cosmic_title_bar() {
    let app = initialized_app();
    let window = &app.core().window;

    assert!(window.show_headerbar);
    assert!(window.content_container);
    assert!(window.use_template);
    assert!(window.header_title.is_empty());
}

#[test]
fn application_identity_uses_the_cosmic_utils_domain() {
    let app = std::fs::read_to_string("src/app.rs").expect("app source should be readable");
    let manifest = std::fs::read_to_string("org.cosmic_utils.compass.yml")
        .expect("renamed Flatpak manifest should be readable");
    let desktop = std::fs::read_to_string("resources/org.cosmic_utils.compass.desktop")
        .expect("renamed desktop entry should be readable");
    let metainfo = std::fs::read_to_string("resources/org.cosmic_utils.compass.metainfo.xml")
        .expect("renamed AppStream metadata should be readable");

    for content in [&app, &manifest, &desktop, &metainfo] {
        assert!(content.contains("org.cosmic_utils.compass"));
        assert!(!content.contains("io.github.cosmic_utils.compass"));
    }
    assert!(
        std::path::Path::new("resources/icons/hicolor/scalable/apps/org.cosmic_utils.compass.svg")
            .is_file()
    );
}

#[test]
fn reverse_geocoding_remains_but_has_no_view_menu_action() {
    let manifest = std::fs::read_to_string("org.cosmic_utils.compass.yml")
        .expect("Flatpak manifest should be readable");
    let source = std::fs::read_to_string("src/app.rs").expect("app source should be readable");

    assert!(manifest.contains("- --share=network"));
    assert!(!source.contains("LookupPlace"));
    assert!(source.contains("geocode::subscription"));
    assert!(source.contains("openstreetmap.org/copyright"));
}

#[test]
fn location_permissions_are_declared_for_native_and_flatpak_builds() {
    let desktop = std::fs::read_to_string("resources/org.cosmic_utils.compass.desktop")
        .expect("desktop entry should be readable");
    assert!(desktop.contains("X-Geoclue-Reason="));

    let flatpak = std::fs::read_to_string("org.cosmic_utils.compass.yml")
        .expect("Flatpak manifest should be readable");
    assert!(flatpak.contains("--system-talk-name=org.freedesktop.GeoClue2"));
}

#[test]
fn flatpak_can_read_and_watch_the_cosmic_desktop_theme() {
    let flatpak = std::fs::read_to_string("org.cosmic_utils.compass.yml")
        .expect("Flatpak manifest should be readable");
    let cargo = std::fs::read_to_string("Cargo.toml").expect("Cargo manifest should be readable");

    assert!(flatpak.contains("--filesystem=xdg-config/cosmic:ro"));
    assert!(flatpak.contains("--talk-name=com.system76.CosmicSettingsDaemon"));
    assert!(flatpak.contains("--talk-name=com.system76.CosmicSettingsDaemon.*"));
    assert!(cargo.contains("\"dbus-config\""));
}

#[test]
fn title_bar_exposes_view_menu_and_about_drawer() {
    let mut app = initialized_app();

    assert_eq!(app.header_start().len(), 1);
    assert!(app.context_drawer().is_none());

    let _ = app.update(Message::ToggleAbout);

    assert!(app.core().window.show_context);
    assert!(app.context_drawer().is_some());

    let _ = app.update(Message::ToggleAbout);

    assert!(!app.core().window.show_context);
    assert!(app.context_drawer().is_none());
}
