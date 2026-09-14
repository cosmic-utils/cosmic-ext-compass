// SPDX-License-Identifier: GPL-3.0-only

use compass::geometry::{
    CompassGeometry, CompassLayout, DEGREE_LABEL_STEP, TICK_STEP_DEGREES, attribution_geometry,
    cardinal_label_radius, crosshair_geometry, degree_label_radius, dial_stroke_widths,
    heading_size_for_region, position_marker_geometry, readout_geometry,
    readout_geometry_in_bounds, typography, warning_text_layout,
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
fn fixed_crosshair_is_centered_inside_the_dial() {
    let geometry = crosshair_geometry(120.0, 90.0, 60.0);

    assert_eq!(geometry.horizontal_start, (84.0, 90.0));
    assert_eq!(geometry.horizontal_end, (156.0, 90.0));
    assert_eq!(geometry.vertical_start, (120.0, 54.0));
    assert_eq!(geometry.vertical_end, (120.0, 126.0));
    assert_eq!(geometry.center, (120.0, 90.0));
}

#[test]
fn compass_warning_text_fits_narrow_readings_in_at_most_two_lines() {
    for message in [
        "No magnetometer is available",
        "Magnetometer access was denied",
        "Connecting to the magnetometer…",
        "The magnetometer could not be reached",
    ] {
        let layout = warning_text_layout(1.0, 150.0, message);
        let estimated_width = message.chars().count() as f32 * layout.size * 0.55;

        assert!(layout.size >= 11.0);
        assert!(estimated_width <= layout.max_width * 2.0);
        assert!(layout.max_height >= layout.size * 2.0);
    }
}

#[test]
fn cardinal_letters_sit_closer_to_the_compass_ring() {
    assert_eq!(cardinal_label_radius(100.0), 72.0);
}

#[test]
fn fixed_position_marker_is_centered_on_the_smallest_ring_markers() {
    let marker = position_marker_geometry(100.0, 2.0);

    assert_eq!(marker.start_y, 81.5);
    assert_eq!(marker.end_y, 129.5);
    assert_eq!((marker.start_y + marker.end_y) / 2.0, 105.5);
}

#[test]
fn degree_numbers_have_more_space_outside_the_compass_ring() {
    assert_eq!(degree_label_radius(100.0, 2.0), 130.0);
}

#[test]
fn near_square_content_stays_stacked_while_clearly_wide_content_uses_columns() {
    let near_square = CompassGeometry::for_viewport(300.0, 299.0);
    assert_eq!(near_square.layout, CompassLayout::Stacked);
    assert_eq!((near_square.center_x, near_square.center_y), (150.0, 74.75));
    assert_eq!(near_square.heading_x, 150.0);

    let wide = CompassGeometry::for_viewport(360.0, 299.0);
    assert_eq!(wide.layout, CompassLayout::CompactWide);
    assert_eq!((wide.center_x, wide.center_y), (90.0, 149.5));
    assert_eq!(wide.heading_x, 270.0);
}

#[test]
fn portrait_phone_content_below_chrome_remains_stacked() {
    assert_eq!(
        CompassGeometry::for_viewport(240.0, 224.0).layout,
        CompassLayout::Stacked
    );
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

    let regular = typography(2.0);
    let oversized = typography(10.0);
    assert!((oversized.degree / regular.degree - 5.0).abs() < f32::EPSILON);
    assert!((oversized.cardinal / regular.cardinal - 5.0).abs() < f32::EPSILON);
    assert!((oversized.heading / regular.heading - 5.0).abs() < f32::EPSILON);
    assert!((oversized.status / regular.status - 5.0).abs() < f32::EPSILON);
}

#[test]
fn compass_scale_remains_proportional_in_very_large_windows() {
    let regular = CompassGeometry::for_viewport(1600.0, 900.0);
    let oversized = CompassGeometry::for_viewport(3840.0, 2128.0);

    assert!(
        (regular.scale / regular.dial_extent - oversized.scale / oversized.dial_extent).abs()
            < 0.000_01
    );
}

#[test]
fn dial_lines_remain_proportional_to_the_compass_scale() {
    let regular = dial_stroke_widths(2.0);
    let oversized = dial_stroke_widths(10.0);

    assert!((oversized.major_tick / regular.major_tick - 5.0).abs() < f32::EPSILON);
    assert!((oversized.minor_tick / regular.minor_tick - 5.0).abs() < f32::EPSILON);
    assert!((oversized.crosshair / regular.crosshair - 5.0).abs() < f32::EPSILON);
    assert!((oversized.center_mark / regular.center_mark - 5.0).abs() < f32::EPSILON);
}

#[test]
fn openstreetmap_attribution_is_a_small_bottom_right_notice() {
    let notice = attribution_geometry(3840.0, 2128.0, 8.0);

    assert_eq!(notice.align_x, 3828.0);
    assert_eq!(notice.align_y, 2116.0);
    assert!(notice.text_size <= 14.0);
}

#[test]
fn phone_location_readings_fit_entirely_inside_the_second_region() {
    let layout = CompassGeometry::for_viewport(240.0, 224.0);
    let notice = attribution_geometry(240.0, 224.0, layout.scale);
    let readout = readout_geometry_in_bounds(
        168.0,
        layout.scale,
        240.0,
        5,
        112.0,
        notice.align_y - notice.text_size - 4.0,
    );
    let top = readout.heading_baseline - readout.heading_size / 2.0;
    let bottom = readout.secondary_baselines[4] + readout.secondary_size / 2.0;

    assert!(top >= 112.0);
    assert!(bottom <= notice.align_y - notice.text_size - 4.0);
    assert!(readout.secondary_size >= 8.0);
}

#[test]
fn primary_reading_scales_down_to_stay_inside_a_narrow_region() {
    let size = heading_size_for_region(0.75, 150.0);
    assert!((28.0..=32.0).contains(&size));
}

#[test]
fn readings_with_location_remain_centered_in_the_second_region() {
    let geometry = CompassGeometry::for_viewport(360.0, 640.0);
    let readout = readout_geometry(480.0, geometry.scale, 360.0, 4);
    let sizes = typography(geometry.scale);
    let heading_size = heading_size_for_region(geometry.scale, 360.0);
    let top = readout.heading_baseline - heading_size / 2.0;
    let bottom = readout.secondary_baselines[3] + sizes.status / 2.0;

    assert_eq!(readout.secondary_baselines.len(), 4);
    assert!(((top + bottom) / 2.0 - 480.0).abs() < f32::EPSILON);
    assert!(
        readout
            .secondary_baselines
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    );
    assert!(top >= 320.0);
    assert!(bottom <= 640.0);
}

#[test]
fn readings_are_centered_in_the_second_region() {
    for (width, height) in [(1600.0, 900.0), (900.0, 1600.0), (300.0, 299.0)] {
        let geometry = CompassGeometry::for_viewport(width, height);
        let expected_x = match geometry.layout {
            CompassLayout::CompactWide => width * 0.75,
            CompassLayout::Stacked => width * 0.5,
        };
        let expected_y = match geometry.layout {
            CompassLayout::CompactWide => height * 0.5,
            CompassLayout::Stacked => height * 0.75,
        };
        let sizes = typography(geometry.scale);
        let region_width = match geometry.layout {
            CompassLayout::CompactWide => width / 2.0,
            CompassLayout::Stacked => width,
        };
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
