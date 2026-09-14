// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    geometry::{
        CompassGeometry, CompassLayout, DEGREE_LABEL_STEP, MINOR_TICK_LENGTH_UNITS,
        TICK_STEP_DEGREES, attribution_geometry, cardinal_label_radius, crosshair_geometry,
        degree_label_radius, dial_stroke_widths, heading_readout_positions, lean_cross_geometry,
        position_marker_geometry, readout_geometry_in_bounds, typography, warning_text_layout,
    },
    heading::heading_readout_parts,
    sensor::TiltReading,
};
use cosmic::{
    Renderer, Theme,
    iced::{Color, Point, Rectangle, alignment::Vertical, core::text::Alignment, mouse},
    widget::canvas,
};

#[must_use]
pub fn north_marker_color(theme: &Theme) -> Color {
    Color::from(theme.cosmic().destructive_color())
}

#[derive(Clone, Debug)]
pub struct CompassRose {
    pub heading: Option<f32>,
    pub tilt: Option<TiltReading>,
    pub compass_enabled: bool,
    pub heading_warning: Option<String>,
    pub status: String,
    pub location_lines: Vec<String>,
    pub attribution: Option<String>,
}

impl<Message> canvas::Program<Message, Theme, Renderer> for CompassRose {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let layout = CompassGeometry::for_viewport(bounds.width, bounds.height);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let cosmic = theme.cosmic();
        let foreground = Color::from(cosmic.on_bg_color());
        let muted = Color {
            a: 0.55,
            ..foreground
        };
        let dial_foreground = if self.compass_enabled {
            foreground
        } else {
            Color {
                a: 0.25,
                ..foreground
            }
        };
        let dial_muted = if self.compass_enabled {
            muted
        } else {
            Color {
                a: 0.14,
                ..foreground
            }
        };
        let north = if self.compass_enabled {
            north_marker_color(theme)
        } else {
            dial_foreground
        };
        let heading = self.heading.unwrap_or(0.0);
        let center = Point::new(layout.center_x, layout.center_y);
        let scale = layout.scale;
        let typography = typography(scale);
        let strokes = dial_stroke_widths(scale);
        let reading_width = match layout.layout {
            CompassLayout::CompactWide => bounds.width / 2.0,
            CompassLayout::Stacked => bounds.width,
        };
        let reading_center_y = match layout.layout {
            CompassLayout::CompactWide => bounds.height / 2.0,
            CompassLayout::Stacked => bounds.height * 0.75,
        };
        let reading_top = match layout.layout {
            CompassLayout::CompactWide => 0.0,
            CompassLayout::Stacked => bounds.height / 2.0,
        };
        let attribution_layout = self
            .attribution
            .as_ref()
            .map(|_| attribution_geometry(bounds.width, bounds.height, scale));
        let reading_bottom = attribution_layout.map_or(bounds.height, |notice| {
            notice.align_y - notice.text_size - 4.0
        });
        let readout = readout_geometry_in_bounds(
            reading_center_y,
            scale,
            reading_width,
            usize::from(!self.status.is_empty()) + self.location_lines.len(),
            reading_top,
            reading_bottom,
        );

        // The dense marks define the dial; there is intentionally no heavy bezel.
        for degree in (0..360).step_by(TICK_STEP_DEGREES) {
            let major = degree % DEGREE_LABEL_STEP == 0;
            let medium = degree % 15 == 0;
            let length = if major {
                13.0 * scale
            } else if medium {
                9.0 * scale
            } else {
                MINOR_TICK_LENGTH_UNITS * scale
            };
            let angle = angle(degree as f32, heading);
            let outer = radial_point(center, layout.radius, angle);
            let inner = radial_point(center, layout.radius - length, angle);
            let path = canvas::Path::line(inner, outer);
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_color(if major { dial_foreground } else { dial_muted })
                    .with_width(if major {
                        strokes.major_tick
                    } else {
                        strokes.minor_tick
                    }),
            );
        }

        // Upright degree labels rotate around the outside with the rose.
        for degree in (0..360).step_by(DEGREE_LABEL_STEP) {
            let position = radial_point(
                center,
                degree_label_radius(layout.radius, scale),
                angle(degree as f32, heading),
            );
            let mut text = canvas::Text::from(degree.to_string());
            text.position = position;
            text.color = dial_muted;
            text.size = typography.degree.into();
            text.align_x = Alignment::Center;
            text.align_y = Vertical::Center;
            frame.fill_text(text);
        }

        for (degree, label) in [(0.0, "N"), (90.0, "E"), (180.0, "S"), (270.0, "W")] {
            let position = radial_point(
                center,
                cardinal_label_radius(layout.radius),
                angle(degree, heading),
            );
            let mut text = canvas::Text::from(label);
            text.position = position;
            text.color = dial_foreground;
            text.size = typography.cardinal.into();
            text.align_x = Alignment::Center;
            text.align_y = Vertical::Center;
            frame.fill_text(text);
        }

        // This orientation guide is deliberately screen-aligned while the
        // dial rotates beneath it.
        let crosshair = crosshair_geometry(layout.center_x, layout.center_y, layout.radius);
        let crosshair_color = Color {
            a: 0.38,
            ..dial_foreground
        };
        for (start, end) in [
            (crosshair.horizontal_start, crosshair.horizontal_end),
            (crosshair.vertical_start, crosshair.vertical_end),
        ] {
            frame.stroke(
                &canvas::Path::line(Point::from(start), Point::from(end)),
                canvas::Stroke::default()
                    .with_color(crosshair_color)
                    .with_width(strokes.crosshair),
            );
        }
        if let Some(tilt) = self.tilt {
            let lean = lean_cross_geometry(layout.center_x, layout.center_y, layout.radius, tilt);
            for (start, end) in [
                (lean.horizontal_start, lean.horizontal_end),
                (lean.vertical_start, lean.vertical_end),
            ] {
                frame.stroke(
                    &canvas::Path::line(Point::from(start), Point::from(end)),
                    canvas::Stroke::default()
                        .with_color(dial_foreground)
                        .with_width(strokes.center_mark),
                );
            }
        }

        // The destructive-colored marker belongs to north and rotates with the dial.
        let north_angle = angle(0.0, heading);
        let (sin, cos) = north_angle.sin_cos();
        let radial = Point::new(sin, -cos);
        let tangent = Point::new(cos, sin);
        let tip_distance = layout.radius + 8.0 * scale;
        let base_distance = layout.radius + 1.0 * scale;
        let half_width = 4.5 * scale;
        let mut marker = canvas::path::Builder::new();
        marker.move_to(Point::new(
            center.x + radial.x * tip_distance,
            center.y + radial.y * tip_distance,
        ));
        marker.line_to(Point::new(
            center.x + radial.x * base_distance + tangent.x * half_width,
            center.y + radial.y * base_distance + tangent.y * half_width,
        ));
        marker.line_to(Point::new(
            center.x + radial.x * base_distance - tangent.x * half_width,
            center.y + radial.y * base_distance - tangent.y * half_width,
        ));
        marker.close();
        frame.fill(&marker.build(), north);

        // A fixed, high-contrast index makes the current bearing unambiguous.
        let marker = position_marker_geometry(layout.needle_tip_y, scale);
        let index = canvas::Path::line(
            Point::new(layout.center_x, marker.start_y),
            Point::new(layout.center_x, marker.end_y),
        );
        frame.stroke(
            &index,
            canvas::Stroke::default()
                .with_color(dial_foreground)
                .with_width(3.0 * scale),
        );

        let heading_size = readout.heading_size;
        if let Some(warning) = &self.heading_warning {
            let warning_layout = warning_text_layout(scale, reading_width, warning);
            let mut warning_text = canvas::Text::from(warning.as_str());
            warning_text.position = Point::new(layout.heading_x, readout.heading_baseline);
            warning_text.color = foreground;
            warning_text.size = warning_layout.size.into();
            warning_text.max_width = warning_layout.max_width;
            warning_text.align_x = Alignment::Center;
            warning_text.align_y = Vertical::Center;
            frame.fill_text(warning_text);
        } else {
            let (degrees, direction) = self.heading.map_or_else(
                || ("—".to_owned(), ""),
                |heading| {
                    let parts = heading_readout_parts(heading);
                    (parts.degrees, parts.direction)
                },
            );
            let positions = heading_readout_positions(layout.heading_x, heading_size);
            let side_width = (reading_width / 2.0 - 8.0).max(1.0);

            let mut value = canvas::Text::from(degrees);
            value.position = Point::new(positions.value_end_x, readout.heading_baseline);
            value.color = foreground;
            value.size = heading_size.into();
            value.max_width = side_width;
            value.align_x = Alignment::Right;
            value.align_y = Vertical::Center;
            frame.fill_text(value);

            let mut degree = canvas::Text::from("°");
            degree.position = Point::new(positions.degree_center_x, readout.heading_baseline);
            degree.color = foreground;
            degree.size = heading_size.into();
            degree.align_x = Alignment::Center;
            degree.align_y = Vertical::Center;
            frame.fill_text(degree);

            if !direction.is_empty() {
                let mut direction = canvas::Text::from(direction);
                direction.position =
                    Point::new(positions.direction_start_x, readout.heading_baseline);
                direction.color = foreground;
                direction.size = heading_size.into();
                direction.max_width = side_width;
                direction.align_x = Alignment::Left;
                direction.align_y = Vertical::Center;
                frame.fill_text(direction);
            }
        }

        let location_baseline_offset = if self.status.is_empty() {
            0
        } else {
            let mut status = canvas::Text::from(self.status.as_str());
            status.position = Point::new(layout.heading_x, readout.secondary_baselines[0]);
            status.color = muted;
            status.size = readout.secondary_size.into();
            status.max_width = (reading_width - 16.0).max(1.0);
            status.align_x = Alignment::Center;
            status.align_y = Vertical::Center;
            frame.fill_text(status);
            1
        };

        for (line, baseline) in self.location_lines.iter().zip(
            readout
                .secondary_baselines
                .iter()
                .skip(location_baseline_offset),
        ) {
            let mut location = canvas::Text::from(line.as_str());
            location.position = Point::new(layout.heading_x, *baseline);
            location.color = foreground;
            location.size = readout.secondary_size.into();
            location.max_width = (reading_width - 16.0).max(1.0);
            location.align_x = Alignment::Center;
            location.align_y = Vertical::Center;
            frame.fill_text(location);
        }

        if let Some(attribution) = &self.attribution {
            let geometry = attribution_layout.expect("attribution geometry must be available");
            let mut notice = canvas::Text::from(attribution.as_str());
            notice.position = Point::new(geometry.align_x, geometry.align_y);
            notice.color = muted;
            notice.size = geometry.text_size.into();
            notice.max_width = (bounds.width - 24.0).max(1.0);
            notice.align_x = Alignment::Right;
            notice.align_y = Vertical::Bottom;
            frame.fill_text(notice);
        }

        vec![frame.into_geometry()]
    }
}

fn angle(mark: f32, heading: f32) -> f32 {
    (mark - heading).to_radians()
}

fn radial_point(center: Point, distance: f32, angle: f32) -> Point {
    let (sin, cos) = angle.sin_cos();
    Point::new(center.x + sin * distance, center.y - cos * distance)
}
