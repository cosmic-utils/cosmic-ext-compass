// SPDX-License-Identifier: GPL-3.0-only

use compass::{
    geometry::{crosshair_geometry, lean_cross_geometry},
    sensor::TiltReading,
};

#[test]
fn proxy_orientation_and_tilt_move_only_the_small_cross() {
    let reading = TiltReading::from_proxy_values("left-up", "vertical")
        .expect("supported accelerometer values should produce a lean reading");
    let fixed = crosshair_geometry(100.0, 100.0, 80.0);
    let leaning = lean_cross_geometry(100.0, 100.0, 80.0, reading);

    assert_eq!(fixed.center, (100.0, 100.0));
    assert_eq!(leaning.center, (52.0, 100.0));
    assert_eq!(leaning.horizontal_start, (47.2, 100.0));
    assert_eq!(leaning.horizontal_end, (56.8, 100.0));
}

#[test]
fn undefined_accelerometer_values_do_not_fabricate_lean() {
    assert!(TiltReading::from_proxy_values("undefined", "vertical").is_none());
    assert!(TiltReading::from_proxy_values("normal", "undefined").is_none());
    assert!(TiltReading::from_proxy_values("diagonal", "vertical").is_none());
}

#[test]
fn face_up_device_keeps_the_small_cross_centered() {
    let reading = TiltReading::from_proxy_values("right-up", "face-up").unwrap();
    let leaning = lean_cross_geometry(40.0, 50.0, 100.0, reading);

    assert_eq!(leaning.center, (40.0, 50.0));
}
