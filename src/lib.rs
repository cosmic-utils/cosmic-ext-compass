// SPDX-License-Identifier: GPL-3.0-only

pub mod app;
pub mod geocode;
pub mod i18n;
pub mod location;
pub mod rose;
pub mod sensor;

pub mod geometry {
    use crate::sensor::TiltReading;

    pub const TICK_STEP_DEGREES: usize = 3;
    pub const DEGREE_LABEL_STEP: usize = 30;
    pub const MINOR_TICK_LENGTH_UNITS: f32 = 5.5;
    pub const HEADING_TEXT_UNITS: f32 = 50.0;
    pub const STATUS_TEXT_UNITS: f32 = 14.0;
    const READOUT_GAP_UNITS: f32 = 8.0;

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct CompassTypography {
        pub degree: f32,
        pub cardinal: f32,
        pub heading: f32,
        pub status: f32,
    }

    #[must_use]
    pub fn typography(scale: f32) -> CompassTypography {
        CompassTypography {
            degree: (11.0 * scale).max(8.0),
            cardinal: (18.0 * scale).max(16.0),
            heading: (HEADING_TEXT_UNITS * scale).max(28.0),
            status: (STATUS_TEXT_UNITS * scale).max(11.0),
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct DialStrokeWidths {
        pub major_tick: f32,
        pub minor_tick: f32,
        pub crosshair: f32,
        pub center_mark: f32,
    }

    #[must_use]
    pub fn dial_stroke_widths(scale: f32) -> DialStrokeWidths {
        DialStrokeWidths {
            major_tick: (1.8 * scale).max(1.0),
            minor_tick: scale.max(1.0),
            crosshair: scale.max(1.0),
            center_mark: (1.5 * scale).max(1.0),
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct AttributionGeometry {
        pub align_x: f32,
        pub align_y: f32,
        pub text_size: f32,
    }

    #[must_use]
    pub fn attribution_geometry(width: f32, height: f32, scale: f32) -> AttributionGeometry {
        const MARGIN: f32 = 12.0;
        AttributionGeometry {
            align_x: (width - MARGIN).max(0.0),
            align_y: (height - MARGIN).max(0.0),
            text_size: (8.0 + scale).clamp(9.0, 14.0),
        }
    }

    #[must_use]
    pub fn heading_size_for_region(scale: f32, region_width: f32) -> f32 {
        let available_width = (region_width - 16.0).max(1.0);
        typography(scale)
            .heading
            .min((available_width / 4.2).max(28.0))
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct WarningTextLayout {
        pub size: f32,
        pub max_width: f32,
        pub max_height: f32,
    }

    #[must_use]
    pub fn warning_text_layout(scale: f32, region_width: f32, warning: &str) -> WarningTextLayout {
        let available_width = (region_width - 24.0).max(1.0);
        let estimated_units = warning.chars().count().max(1) as f32 * 0.55;
        let size = (typography(scale).cardinal * 1.1)
            .min((available_width * 1.9) / estimated_units)
            .max(8.0);
        WarningTextLayout {
            size,
            max_width: available_width,
            max_height: size * 2.4,
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct HeadingReadoutPositions {
        pub value_end_x: f32,
        pub degree_center_x: f32,
        pub direction_start_x: f32,
    }

    /// Anchors the degree glyph at the center of the readings region so the
    /// readout remains visually stable as the digit count and direction vary.
    #[must_use]
    pub fn heading_readout_positions(center_x: f32, text_size: f32) -> HeadingReadoutPositions {
        HeadingReadoutPositions {
            value_end_x: center_x - text_size * 0.20,
            degree_center_x: center_x,
            direction_start_x: center_x + text_size * 0.35,
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct CrosshairGeometry {
        pub horizontal_start: (f32, f32),
        pub horizontal_end: (f32, f32),
        pub vertical_start: (f32, f32),
        pub vertical_end: (f32, f32),
        pub center: (f32, f32),
    }

    /// Fixed screen-aligned orientation guide drawn independently of heading.
    #[must_use]
    pub fn crosshair_geometry(center_x: f32, center_y: f32, radius: f32) -> CrosshairGeometry {
        let half_extent = radius * 0.60;
        CrosshairGeometry {
            horizontal_start: (center_x - half_extent, center_y),
            horizontal_end: (center_x + half_extent, center_y),
            vertical_start: (center_x, center_y - half_extent),
            vertical_end: (center_x, center_y + half_extent),
            center: (center_x, center_y),
        }
    }

    /// Small accelerometer-driven level mark; the large crosshair remains fixed.
    #[must_use]
    pub fn lean_cross_geometry(
        center_x: f32,
        center_y: f32,
        radius: f32,
        reading: TiltReading,
    ) -> CrosshairGeometry {
        let (offset_x, offset_y) = reading.offset_factor();
        let lean_x = center_x + offset_x * radius;
        let lean_y = center_y + offset_y * radius;
        let half_extent = radius * 0.06;
        CrosshairGeometry {
            horizontal_start: (lean_x - half_extent, lean_y),
            horizontal_end: (lean_x + half_extent, lean_y),
            vertical_start: (lean_x, lean_y - half_extent),
            vertical_end: (lean_x, lean_y + half_extent),
            center: (lean_x, lean_y),
        }
    }

    #[must_use]
    pub fn cardinal_label_radius(radius: f32) -> f32 {
        radius * 0.72
    }

    #[must_use]
    pub fn degree_label_radius(radius: f32, scale: f32) -> f32 {
        radius + 15.0 * scale
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct PositionMarkerGeometry {
        pub start_y: f32,
        pub end_y: f32,
    }

    #[must_use]
    pub fn position_marker_geometry(ring_y: f32, scale: f32) -> PositionMarkerGeometry {
        let half_length = 12.0 * scale;
        let marker_center = ring_y + MINOR_TICK_LENGTH_UNITS * scale / 2.0;
        PositionMarkerGeometry {
            start_y: marker_center - half_length,
            end_y: marker_center + half_length,
        }
    }

    /// High-level composition chosen for the available canvas.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum CompassLayout {
        Stacked,
        CompactWide,
    }

    /// Pure layout shared by the canvas and compact-window tests.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct CompassGeometry {
        pub layout: CompassLayout,
        pub center_x: f32,
        pub center_y: f32,
        pub radius: f32,
        pub dial_extent: f32,
        pub scale: f32,
        pub needle_tip_y: f32,
        pub heading_x: f32,
        pub heading_baseline: f32,
        pub status_baseline: f32,
    }

    impl CompassGeometry {
        #[must_use]
        pub fn for_viewport(width: f32, height: f32) -> Self {
            let width = finite_positive_or(width, 240.0);
            let height = finite_positive_or(height, 320.0);
            let compact_wide = width > height * 1.2;
            let layout = if compact_wide {
                CompassLayout::CompactWide
            } else {
                CompassLayout::Stacked
            };

            if compact_wide {
                let center_x = width * 0.25;
                let heading_x = width * 0.75;
                let (radius, scale, dial_extent) = dial_metrics(width / 2.0, height);
                let center_y = height / 2.0;
                let (heading_baseline, status_baseline) =
                    reading_baselines(center_y, scale, width / 2.0);

                Self {
                    layout,
                    center_x,
                    center_y,
                    radius,
                    dial_extent,
                    scale,
                    needle_tip_y: center_y - radius,
                    heading_x,
                    heading_baseline,
                    status_baseline,
                }
            } else {
                let (radius, scale, dial_extent) = dial_metrics(width, height / 2.0);
                let center_x = width / 2.0;
                let center_y = height * 0.25;
                let (heading_baseline, status_baseline) =
                    reading_baselines(height * 0.75, scale, width);

                Self {
                    layout,
                    center_x,
                    center_y,
                    radius,
                    dial_extent,
                    scale,
                    needle_tip_y: center_y - radius,
                    heading_x: center_x,
                    heading_baseline,
                    status_baseline,
                }
            }
        }
    }

    fn finite_positive_or(value: f32, fallback: f32) -> f32 {
        if value.is_finite() && value > 0.0 {
            value
        } else {
            fallback
        }
    }

    fn dial_metrics(region_width: f32, region_height: f32) -> (f32, f32, f32) {
        let short_side = region_width.min(region_height);
        let padding = (short_side * 0.02).clamp(2.0, 12.0);
        let dial_extent = (short_side / 2.0 - padding).max(1.0);
        let scale = (dial_extent / 120.0).max(0.75);
        let radius = (dial_extent - 18.0 * scale).max(1.0);
        (radius, scale, dial_extent)
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct ReadoutGeometry {
        pub heading_baseline: f32,
        pub heading_size: f32,
        pub secondary_baselines: Vec<f32>,
        pub secondary_size: f32,
    }

    /// Centers the heading and all secondary readings as one visual group.
    #[must_use]
    pub fn readout_geometry(
        center_y: f32,
        scale: f32,
        region_width: f32,
        secondary_count: usize,
    ) -> ReadoutGeometry {
        let sizes = typography(scale);
        let heading_size = heading_size_for_region(scale, region_width);
        readout_geometry_with_sizes(
            center_y,
            heading_size,
            sizes.status,
            (READOUT_GAP_UNITS * scale).max(6.0),
            secondary_count,
        )
    }

    fn readout_geometry_with_sizes(
        center_y: f32,
        heading_size: f32,
        secondary_size: f32,
        gap: f32,
        secondary_count: usize,
    ) -> ReadoutGeometry {
        if secondary_count == 0 {
            return ReadoutGeometry {
                heading_baseline: center_y,
                heading_size,
                secondary_baselines: Vec::new(),
                secondary_size,
            };
        }

        let secondary_step = secondary_size * 1.45;
        let secondary_height =
            secondary_size + secondary_step * secondary_count.saturating_sub(1) as f32;
        let total_height = heading_size + gap + secondary_height;
        let top = center_y - total_height / 2.0;
        let heading_baseline = top + heading_size / 2.0;
        let first_secondary = top + heading_size + gap + secondary_size / 2.0;
        let secondary_baselines = (0..secondary_count)
            .map(|index| first_secondary + secondary_step * index as f32)
            .collect();

        ReadoutGeometry {
            heading_baseline,
            heading_size,
            secondary_baselines,
            secondary_size,
        }
    }

    /// Fits a complete readout group inside the available vertical region.
    #[must_use]
    pub fn readout_geometry_in_bounds(
        center_y: f32,
        scale: f32,
        region_width: f32,
        secondary_count: usize,
        minimum_top: f32,
        maximum_bottom: f32,
    ) -> ReadoutGeometry {
        let sizes = typography(scale);
        let heading_size = heading_size_for_region(scale, region_width);
        let gap = (READOUT_GAP_UNITS * scale).max(6.0);
        let secondary_height = if secondary_count == 0 {
            0.0
        } else {
            sizes.status + sizes.status * 1.45 * secondary_count.saturating_sub(1) as f32
        };
        let total_height =
            heading_size + if secondary_count == 0 { 0.0 } else { gap } + secondary_height;
        let available_height = (maximum_bottom - minimum_top).max(1.0);
        let compression = (available_height / total_height.max(1.0)).min(1.0);
        let fitted_height = total_height * compression;
        let fitted_center = center_y.clamp(
            minimum_top + fitted_height / 2.0,
            maximum_bottom - fitted_height / 2.0,
        );

        readout_geometry_with_sizes(
            fitted_center,
            heading_size * compression,
            sizes.status * compression,
            gap * compression,
            secondary_count,
        )
    }

    fn reading_baselines(center_y: f32, scale: f32, region_width: f32) -> (f32, f32) {
        let geometry = readout_geometry(center_y, scale, region_width, 1);
        (geometry.heading_baseline, geometry.secondary_baselines[0])
    }
}

pub mod heading {
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct HeadingReadoutParts {
        pub degrees: String,
        pub direction: &'static str,
    }

    /// Separates the changing portions of the primary readout so its degree
    /// glyph can be rendered at a fixed horizontal anchor.
    #[must_use]
    pub fn heading_readout_parts(heading: f32) -> HeadingReadoutParts {
        let rounded = heading.round().rem_euclid(360.0);
        HeadingReadoutParts {
            degrees: format!("{rounded:.0}"),
            direction: cardinal_direction(heading),
        }
    }

    /// Formats the primary readout with unpadded degrees and its nearest direction.
    #[must_use]
    pub fn heading_readout(heading: f32) -> String {
        let parts = heading_readout_parts(heading);
        format!("{}° {}", parts.degrees, parts.direction)
    }

    /// Returns the nearest of the eight familiar compass directions.
    #[must_use]
    pub fn cardinal_direction(heading: f32) -> &'static str {
        const DIRECTIONS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
        if !heading.is_finite() {
            return "N";
        }
        let sector = ((heading.rem_euclid(360.0) + 22.5) / 45.0).floor() as usize % 8;
        DIRECTIONS[sector]
    }

    /// Normalizes a SensorProxy heading to [0, 360). The sentinel -1 and
    /// non-finite values represent an unavailable reading.
    #[must_use]
    pub fn normalize_heading(value: f64) -> Option<f32> {
        if !value.is_finite() || value < 0.0 {
            None
        } else {
            Some(value.rem_euclid(360.0) as f32)
        }
    }

    /// Signed angular distance in [-180, 180), suitable for interpolation.
    #[must_use]
    pub fn shortest_delta(from: f32, to: f32) -> f32 {
        (to - from + 180.0).rem_euclid(360.0) - 180.0
    }

    /// Interpolates headings without taking the long route across north.
    #[must_use]
    pub fn smooth_heading(current: f32, target: f32, factor: f32) -> f32 {
        let factor = if factor.is_finite() {
            factor.clamp(0.0, 1.0)
        } else {
            0.0
        };
        (current + shortest_delta(current, target) * factor).rem_euclid(360.0)
    }
}
