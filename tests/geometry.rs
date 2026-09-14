// SPDX-License-Identifier: GPL-3.0-only

use compass::geometry::{
    CompassGeometry, CompassLayout, DEGREE_LABEL_STEP, TICK_STEP_DEGREES, heading_size_for_region,
    typography,
};

fn assert_fits(width: f32, height: f32) {
    let geometry = CompassGeometry::for_viewport(width, height);
    assert!(geometry.radius >= 40.0);
    assert!(geometry.center_x - geometry.dial_extent >= 0.0);
    assert!(geometry.center_x + geometry.dial_extent <= width);
    assert!(geometry.center_y - geometry.dial_extent >= 0.0);
    assert!(geometry.center_y + geometry.dial_extent <= height);
    assert!(geometry.heading_baseline <= height);
    assert!(geometry.needle_tip_y >= 0.0);
}

#[test]
fn dial_uses_dense_ticks_and_sparse_degree_labels() {
    assert_eq!(TICK_STEP_DEGREES, 3);
    assert_eq!(DEGREE_LABEL_STEP, 30);
    assert_eq!(360 / TICK_STEP_DEGREES, 120);
}

#[test]
fn aspect_ratio_alone_selects_the_two_region_orientation() {
    let wide = CompassGeometry::for_viewport(300.0, 299.0);
    assert_eq!(wide.layout, CompassLayout::CompactWide);
    assert_eq!((wide.center_x, wide.center_y), (75.0, 149.5));
    assert_eq!(wide.heading_x, 225.0);

    let tall = CompassGeometry::for_viewport(299.0, 300.0);
    assert_eq!(tall.layout, CompassLayout::Stacked);
    assert_eq!((tall.center_x, tall.center_y), (149.5, 75.0));
    assert_eq!(tall.heading_x, 149.5);
}

#[test]
fn tall_portrait_separates_the_dial_from_the_primary_readout() {
    let geometry = CompassGeometry::for_viewport(360.0, 640.0);

    assert_eq!(geometry.layout, CompassLayout::Stacked);
    assert!(geometry.center_y < 280.0);
    assert!(geometry.heading_baseline - (geometry.center_y + geometry.dial_extent) >= 60.0);
    assert!(geometry.status_baseline < 640.0);
}

#[test]
fn phone_portrait_keeps_the_dial_at_a_readable_scale() {
    let geometry = CompassGeometry::for_viewport(240.0, 320.0);

    assert_eq!(geometry.layout, CompassLayout::Stacked);
    assert!((60.0..=65.0).contains(&geometry.radius));
}

#[test]
fn geometry_fits_phone_portrait_content_below_chrome() {
    assert_fits(240.0, 224.0);
}

#[test]
fn geometry_fits_short_landscape_content_below_chrome() {
    assert_fits(320.0, 144.0);
}

#[test]
fn geometry_fits_minimum_square_window_content_below_chrome() {
    assert_fits(240.0, 144.0);
}

#[test]
fn spacious_desktop_fills_the_visualization_region() {
    let geometry = CompassGeometry::for_viewport(1600.0, 900.0);

    assert_eq!(geometry.layout, CompassLayout::CompactWide);
    assert!((320.0..=340.0).contains(&geometry.radius));
    assert!(geometry.center_x < 800.0);
    assert!(geometry.heading_x > geometry.center_x + geometry.radius);
}

#[test]
fn compass_fills_the_visualization_region_without_crossing_it() {
    for (width, height) in [(1600.0, 900.0), (900.0, 1600.0)] {
        let geometry = CompassGeometry::for_viewport(width, height);
        let region_short_side = if width > height {
            (width / 2.0).min(height)
        } else {
            width.min(height / 2.0)
        };
        let available_radius = region_short_side / 2.0;

        assert!(geometry.dial_extent >= available_radius * 0.90);
        assert!(geometry.dial_extent <= available_radius);
    }
}

#[test]
fn dial_typography_scales_up_and_has_a_readable_minimum() {
    let large = CompassGeometry::for_viewport(1600.0, 900.0);
    assert!(large.scale >= 3.0);

    let phone = CompassGeometry::for_viewport(240.0, 320.0);
    assert!(phone.scale >= 0.75);
}

#[test]
fn rendered_typography_scales_smoothly_between_readable_limits() {
    let small = typography(0.75);
    assert!(small.degree >= 8.0);
    assert!(small.cardinal >= 16.0);
    assert!(small.heading >= 28.0);
    assert!(small.status >= 11.0);

    let large = typography(3.5);
    assert!(large.degree > small.degree * 2.0);
    assert!(large.cardinal > small.cardinal * 2.0);
    assert!(large.heading > small.heading * 2.0);

    let oversized = typography(100.0);
    assert!(oversized.degree <= 36.0);
    assert!(oversized.cardinal <= 64.0);
    assert!(oversized.heading <= 160.0);
    assert!(oversized.status <= 32.0);
}

#[test]
fn primary_reading_scales_down_to_stay_inside_a_narrow_region() {
    let size = heading_size_for_region(0.75, 150.0);
    assert!((28.0..=32.0).contains(&size));
}

#[test]
fn readings_are_centered_in_the_second_region() {
    for (width, height) in [(1600.0, 900.0), (900.0, 1600.0), (300.0, 299.0)] {
        let geometry = CompassGeometry::for_viewport(width, height);
        let expected_x = if width > height {
            width * 0.75
        } else {
            width * 0.5
        };
        let expected_y = if width > height {
            height * 0.5
        } else {
            height * 0.75
        };
        let sizes = typography(geometry.scale);
        let region_width = if width > height { width / 2.0 } else { width };
        let heading_size = heading_size_for_region(geometry.scale, region_width);
        let readings_top = geometry.heading_baseline - heading_size / 2.0;
        let readings_bottom = geometry.status_baseline + sizes.status / 2.0;
        let readings_midpoint = (readings_top + readings_bottom) / 2.0;

        assert!((geometry.heading_x - expected_x).abs() < f32::EPSILON);
        assert!((readings_midpoint - expected_y).abs() < f32::EPSILON);
    }
}

#[test]
fn spacious_near_square_window_does_not_shrink_the_dial() {
    let geometry = CompassGeometry::for_viewport(900.0, 1000.0);

    assert_eq!(geometry.layout, CompassLayout::Stacked);
    assert!((195.0..=210.0).contains(&geometry.radius));
}

#[test]
fn portrait_stacks_readout_below_a_centered_dial() {
    let geometry = CompassGeometry::for_viewport(224.0, 240.0);

    assert_eq!(geometry.layout, CompassLayout::Stacked);
    assert_eq!(geometry.center_x, 112.0);
    assert_eq!(geometry.heading_x, geometry.center_x);
    assert!(geometry.center_y + geometry.dial_extent < geometry.heading_baseline);
    assert!(geometry.status_baseline - geometry.heading_baseline >= 16.0);
    assert!(geometry.status_baseline <= 240.0);
}

#[test]
fn short_landscape_switches_to_horizontal_composition() {
    let geometry = CompassGeometry::for_viewport(320.0, 240.0);

    assert_eq!(geometry.layout, CompassLayout::CompactWide);
    assert!(geometry.center_x + geometry.dial_extent < geometry.heading_x);
}

#[test]
fn compact_landscape_places_dial_and_readout_side_by_side() {
    for (width, height) in [(320.0, 144.0), (240.0, 144.0)] {
        let geometry = CompassGeometry::for_viewport(width, height);

        assert_eq!(geometry.layout, CompassLayout::CompactWide);
        assert!(geometry.radius >= 40.0);
        assert!(geometry.center_x + geometry.dial_extent < geometry.heading_x);
        assert!(geometry.center_x - geometry.dial_extent >= 0.0);
        assert!(geometry.center_y - geometry.dial_extent >= 0.0);
        assert!(geometry.center_y + geometry.dial_extent <= height);
        assert!(geometry.status_baseline <= height);
    }
}

#[test]
fn geometry_sanitizes_invalid_viewports() {
    let geometry = CompassGeometry::for_viewport(f32::NAN, -20.0);
    assert!(geometry.center_x.is_finite());
    assert!(geometry.center_y.is_finite());
    assert!(geometry.radius.is_finite());
}
