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
    assert_eq!(window.header_title, "Compass");
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
