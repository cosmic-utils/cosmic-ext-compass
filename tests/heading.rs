// SPDX-License-Identifier: GPL-3.0-only

use compass::{
    geometry::heading_readout_positions,
    heading::{
        cardinal_direction, heading_readout, heading_readout_parts, normalize_heading,
        shortest_delta, smooth_heading,
    },
};

#[test]
fn heading_readout_combines_unpadded_degrees_and_direction() {
    assert_eq!(heading_readout(7.0), "7° N");
    assert_eq!(heading_readout(165.0), "165° S");
    assert_eq!(heading_readout(359.6), "0° N");
}

#[test]
fn degree_glyph_stays_at_the_reading_center_as_digits_change() {
    let positions = heading_readout_positions(180.0, 64.0);

    for heading in [7.0, 42.0, 165.0, 359.6] {
        let parts = heading_readout_parts(heading);
        assert_eq!(positions.degree_center_x, 180.0);
        assert!(!parts.degrees.contains('°'));
        assert!(!parts.direction.contains('°'));
    }

    assert!(positions.value_end_x < positions.degree_center_x);
    assert!(positions.direction_start_x > positions.degree_center_x);
}

#[test]
fn normalization_wraps_finite_degrees_and_rejects_unknown_values() {
    assert_eq!(normalize_heading(0.0), Some(0.0));
    assert_eq!(normalize_heading(360.0), Some(0.0));
    assert_eq!(normalize_heading(725.0), Some(5.0));
    assert_eq!(normalize_heading(-30.0), None);
    assert_eq!(normalize_heading(-1.0), None);
    assert_eq!(normalize_heading(-2.0), None);
    assert_eq!(normalize_heading(f64::NAN), None);
    assert_eq!(normalize_heading(f64::INFINITY), None);
}

#[test]
fn direction_names_follow_eight_equal_compass_sectors() {
    assert_eq!(cardinal_direction(0.0), "N");
    assert_eq!(cardinal_direction(22.4), "N");
    assert_eq!(cardinal_direction(22.5), "NE");
    assert_eq!(cardinal_direction(67.5), "E");
    assert_eq!(cardinal_direction(157.5), "S");
    assert_eq!(cardinal_direction(247.5), "W");
    assert_eq!(cardinal_direction(337.5), "N");
    assert_eq!(cardinal_direction(360.0), "N");
}

#[test]
fn smoothing_follows_the_shortest_path_across_north() {
    assert!((shortest_delta(350.0, 10.0) - 20.0).abs() < f32::EPSILON);
    assert!((shortest_delta(10.0, 350.0) + 20.0).abs() < f32::EPSILON);
    assert!((smooth_heading(350.0, 10.0, 0.25) - 355.0).abs() < f32::EPSILON);
    assert!((smooth_heading(10.0, 350.0, 0.5) - 0.0).abs() < f32::EPSILON);
}

#[test]
fn smoothing_clamps_the_factor_and_normalizes_results() {
    assert_eq!(smooth_heading(90.0, 180.0, -2.0), 90.0);
    assert_eq!(smooth_heading(350.0, 10.0, 2.0), 10.0);
}
