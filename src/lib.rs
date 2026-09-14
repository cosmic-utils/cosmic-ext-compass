// SPDX-License-Identifier: GPL-3.0-only

pub mod app;
pub mod i18n;
pub mod rose;
pub mod sensor;

pub mod geometry {
    pub const TICK_STEP_DEGREES: usize = 3;
    pub const DEGREE_LABEL_STEP: usize = 30;
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
            degree: (11.0 * scale).clamp(8.0, 36.0),
            cardinal: (18.0 * scale).clamp(16.0, 64.0),
            heading: (HEADING_TEXT_UNITS * scale).clamp(28.0, 160.0),
            status: (STATUS_TEXT_UNITS * scale).clamp(11.0, 32.0),
        }
    }

    #[must_use]
    pub fn heading_size_for_region(scale: f32, region_width: f32) -> f32 {
        let available_width = (region_width - 16.0).max(1.0);
        typography(scale)
            .heading
            .min((available_width / 4.2).max(28.0))
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
            let compact_wide = width > height;
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
        let padding = (short_side * 0.025).clamp(4.0, 12.0);
        let dial_extent = (short_side / 2.0 - padding).max(1.0);
        let scale = (dial_extent / 120.0).clamp(0.75, 3.5);
        let radius = (dial_extent - 18.0 * scale).max(1.0);
        (radius, scale, dial_extent)
    }

    fn reading_baselines(center_y: f32, scale: f32, region_width: f32) -> (f32, f32) {
        let sizes = typography(scale);
        let heading_size = heading_size_for_region(scale, region_width);
        let gap = (READOUT_GAP_UNITS * scale).clamp(6.0, 24.0);
        let heading = center_y - (gap + sizes.status) / 2.0;
        let status = center_y + (gap + heading_size) / 2.0;
        (heading, status)
    }
}

pub mod heading {
    /// Formats the primary readout with unpadded degrees and its nearest direction.
    #[must_use]
    pub fn heading_readout(heading: f32) -> String {
        let rounded = heading.round().rem_euclid(360.0);
        format!("{rounded:.0}° {}", cardinal_direction(heading))
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
