// SPDX-License-Identifier: GPL-3.0-only

use compass::location::{LocationFix, demo_location, format_coordinates};

#[test]
fn location_fix_rejects_invalid_or_unknown_values() {
    assert!(LocationFix::try_new(f64::NAN, 6.0, 20.0, 100.0, "GPS").is_none());
    assert!(LocationFix::try_new(91.0, 6.0, 20.0, 100.0, "GPS").is_none());
    assert!(LocationFix::try_new(50.0, 181.0, 20.0, 100.0, "GPS").is_none());
    assert!(LocationFix::try_new(50.0, 6.0, -1.0, 100.0, "GPS").is_none());

    let unknown_altitude = LocationFix::try_new(50.0, 6.0, 20.0, f64::MIN, "GPS")
        .expect("valid coordinates should survive an unknown altitude");
    assert_eq!(unknown_altitude.altitude_m, None);
}

#[test]
fn coordinates_use_stable_dms_with_hemispheres() {
    assert_eq!(
        format_coordinates(50.205_055_4, 7.336_597_1),
        "50°12′18″ N 7°20′12″ E"
    );
    assert_eq!(
        format_coordinates(-33.858_611, -151.214_167),
        "33°51′31″ S 151°12′51″ W"
    );
}

#[test]
fn demo_location_is_burg_eltz() {
    let fix = demo_location();

    assert_eq!(fix.latitude, 50.205_055_4);
    assert_eq!(fix.longitude, 7.336_597_1);
    assert_eq!(fix.accuracy_m, 12.0);
    assert_eq!(fix.altitude_m, Some(320.0));
    assert_eq!(
        fix.place_name.as_deref(),
        Some("Burg Eltz, Rhineland-Palatinate")
    );
}

#[test]
fn generic_provider_labels_are_not_presented_as_place_names() {
    let wifi = LocationFix::try_new(50.0, 6.0, 86.0, f64::MIN, "WiFi").unwrap();
    let named = LocationFix::try_new(
        50.205_055_4,
        7.336_597_1,
        12.0,
        320.0,
        "Burg Eltz, Rhineland-Palatinate",
    )
    .unwrap();

    assert_eq!(wifi.place_name, None);
    assert_eq!(
        named.place_name.as_deref(),
        Some("Burg Eltz, Rhineland-Palatinate")
    );
    assert_eq!(named.altitude_m, Some(320.0));
}
