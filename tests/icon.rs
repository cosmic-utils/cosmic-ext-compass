// SPDX-License-Identifier: GPL-3.0-only

#[test]
fn broad_icon_frame_uses_the_cosmic_turquoise_gradient() {
    let svg = std::fs::read_to_string(
        "resources/icons/hicolor/scalable/apps/org.cosmic_utils.compass.svg",
    )
    .expect("canonical icon should be readable");

    assert!(svg.contains("<stop stop-color=\"#229FAD\"/>"));
    assert!(svg.contains("<stop offset=\"1\" stop-color=\"#49BAC8\"/>"));
    assert!(!svg.contains("#A252D2"));
    assert!(!svg.contains("#BE6DEE"));
}
