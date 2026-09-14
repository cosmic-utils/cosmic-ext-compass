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

#[test]
fn dial_has_every_three_degree_tick_without_labels() {
    let svg = std::fs::read_to_string(
        "resources/icons/hicolor/scalable/apps/org.cosmic_utils.compass.svg",
    )
    .expect("canonical icon should be readable");

    assert_eq!(svg.matches("<use href=\"#tick-").count(), 120);
    for degree in (0..360).step_by(3) {
        assert!(
            svg.contains(&format!("rotate({degree} 128 128)")),
            "missing dial tick at {degree} degrees"
        );
    }
    assert!(!svg.contains("<text"));
}
