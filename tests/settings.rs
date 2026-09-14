// SPDX-License-Identifier: GPL-3.0-only

use compass::app::{AppTheme, CompassApp, Flags, Message};
use cosmic::{Application, Core};

fn initialized_app() -> CompassApp {
    compass::i18n::init(&[]);
    let (app, _) = <CompassApp as Application>::init(
        Core::default(),
        Flags {
            demo_heading: Some(0.0),
            demo_motion: false,
        },
    );
    app
}

#[test]
fn settings_drawer_changes_theme_and_reset_restores_system_default() {
    let mut app = initialized_app();

    assert_eq!(app.app_theme(), AppTheme::System);
    assert!(app.context_drawer().is_none());

    let _ = app.update(Message::ToggleSettings);
    assert!(app.core().window.show_context);
    assert!(app.context_drawer().is_some());

    let _ = app.update(Message::SetAppTheme(1));
    assert_eq!(app.app_theme(), AppTheme::Dark);

    let _ = app.update(Message::ResetAllSettings);
    assert_eq!(app.app_theme(), AppTheme::System);
}
