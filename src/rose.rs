// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    geometry::{
        CompassGeometry, CompassLayout, DEGREE_LABEL_STEP, TICK_STEP_DEGREES,
        heading_readout_positions, heading_size_for_region, typography,
    },
    heading::heading_readout_parts,
};
use cosmic::{
    Renderer, Theme,
    iced::{Color, Point, Rectangle, alignment::Vertical, core::text::Alignment, mouse},
    widget::canvas,
};

#[derive(Clone, Debug)]
pub struct CompassRose {
    pub heading: Option<f32>,
    pub status: String,
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
        let north = Color::from_rgb(0.93, 0.20, 0.24);
        let heading = self.heading.unwrap_or(0.0);
        let center = Point::new(layout.center_x, layout.center_y);
        let scale = layout.scale;
        let typography = typography(scale);
        let reading_width = match layout.layout {
            CompassLayout::CompactWide => bounds.width / 2.0,
            CompassLayout::Stacked => bounds.width,
        };

        // The dense marks define the dial; there is intentionally no heavy bezel.
        for degree in (0..360).step_by(TICK_STEP_DEGREES) {
            let major = degree % DEGREE_LABEL_STEP == 0;
            let medium = degree % 15 == 0;
            let length = if major {
                13.0 * scale
            } else if medium {
                9.0 * scale
            } else {
                5.5 * scale
            };
            let angle = angle(degree as f32, heading);
            let outer = radial_point(center, layout.radius, angle);
            let inner = radial_point(center, layout.radius - length, angle);
            let path = canvas::Path::line(inner, outer);
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_color(if major { foreground } else { muted })
                    .with_width(if major { 1.8 * scale } else { 1.0 }),
            );
        }

        // Upright degree labels rotate around the outside with the rose.
        for degree in (0..360).step_by(DEGREE_LABEL_STEP) {
            let position = radial_point(
                center,
                layout.radius + 11.0 * scale,
                angle(degree as f32, heading),
            );
            let mut text = canvas::Text::from(degree.to_string());
            text.position = position;
            text.color = muted;
            text.size = typography.degree.into();
            text.align_x = Alignment::Center;
            text.align_y = Vertical::Center;
            frame.fill_text(text);
        }

        for (degree, label) in [(0.0, "N"), (90.0, "E"), (180.0, "S"), (270.0, "W")] {
            let position = radial_point(center, layout.radius * 0.64, angle(degree, heading));
            let mut text = canvas::Text::from(label);
            text.position = position;
            text.color = foreground;
            text.size = typography.cardinal.into();
            text.align_x = Alignment::Center;
            text.align_y = Vertical::Center;
            frame.fill_text(text);
        }

        // A red marker belongs to north and rotates with the dial.
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
        let index = canvas::Path::line(
            Point::new(layout.center_x, layout.needle_tip_y - 13.0 * scale),
            Point::new(layout.center_x, layout.needle_tip_y + 11.0 * scale),
        );
        frame.stroke(
            &index,
            canvas::Stroke::default()
                .with_color(foreground)
                .with_width(3.0 * scale),
        );

        let (degrees, direction) = self.heading.map_or_else(
            || ("—".to_owned(), ""),
            |heading| {
                let parts = heading_readout_parts(heading);
                (parts.degrees, parts.direction)
            },
        );
        let heading_size = heading_size_for_region(scale, reading_width);
        let positions = heading_readout_positions(layout.heading_x, heading_size);
        let side_width = (reading_width / 2.0 - 8.0).max(1.0);

        let mut value = canvas::Text::from(degrees);
        value.position = Point::new(positions.value_end_x, layout.heading_baseline);
        value.color = foreground;
        value.size = heading_size.into();
        value.max_width = side_width;
        value.align_x = Alignment::Right;
        value.align_y = Vertical::Center;
        frame.fill_text(value);

        let mut degree = canvas::Text::from("°");
        degree.position = Point::new(positions.degree_center_x, layout.heading_baseline);
        degree.color = foreground;
        degree.size = heading_size.into();
        degree.align_x = Alignment::Center;
        degree.align_y = Vertical::Center;
        frame.fill_text(degree);

        if !direction.is_empty() {
            let mut direction = canvas::Text::from(direction);
            direction.position = Point::new(positions.direction_start_x, layout.heading_baseline);
            direction.color = foreground;
            direction.size = heading_size.into();
            direction.max_width = side_width;
            direction.align_x = Alignment::Left;
            direction.align_y = Vertical::Center;
            frame.fill_text(direction);
        }

        let mut status = canvas::Text::from(self.status.as_str());
        status.position = Point::new(layout.heading_x, layout.status_baseline);
        status.color = muted;
        status.size = typography.status.into();
        status.max_width = (reading_width - 16.0).max(1.0);
        status.align_x = Alignment::Center;
        status.align_y = Vertical::Center;
        frame.fill_text(status);

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
