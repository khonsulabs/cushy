//! A [`Renderer`](gooey_renderer::Renderer) for `Gooey` that uses
//! [`Kludgine`](https://github.com/kludgine/kludgine/) to draw. Under the hood,
//! `Kludgine` uses `wgpu`, and in the future we [aim to support embedding
//! `Gooey` into other `wgpu`
//! applications](https://github.com/khonsulabs/kludgine/issues/51).
//!
//! ## User interface scaling (Points)
//!
//! Kludgine uses [`winit`](kludgine::core::winit) to determine the scaling ratio to use. For more information on the approaches taken, see the [`winit::dpi`](kludgine::core::winit::dpi) module.

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    // TODO missing_docs,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![allow(
    clippy::if_not_else,
    clippy::module_name_repetitions,
    clippy::multiple_crate_versions, // this is a mess due to winit dependencies and wgpu dependencies not lining up
    clippy::missing_errors_doc, // TODO clippy::missing_errors_doc
    clippy::missing_panics_doc, // TODO clippy::missing_panics_doc
)]
#![cfg_attr(doc, warn(rustdoc::all))]

use gooey_core::{
    euclid::{Point2D, Rect},
    styles::SystemTheme,
    Pixels, Points,
};
use gooey_renderer::{Renderer, StrokeOptions, TextMetrics, TextOptions};
pub use kludgine;
use kludgine::{core::winit::window::Theme, prelude::*};

#[derive(Debug, Clone)]
pub struct Kludgine {
    target: Target,
}

impl From<Target> for Kludgine {
    fn from(target: Target) -> Self {
        Self { target }
    }
}

impl<'a> From<&'a Target> for Kludgine {
    fn from(target: &'a Target) -> Self {
        Self {
            target: target.clone(),
        }
    }
}

impl Kludgine {
    fn prepare_text(&self, text: &str, options: &TextOptions) -> PreparedSpan {
        Text::prepare(
            text,
            &bundled_fonts::ROBOTO,
            options.text_size.cast_unit(),
            Color::new(
                options.color.red,
                options.color.green,
                options.color.blue,
                options.color.alpha,
            ),
            &self.target,
        )
    }

    fn stroke_shape(&self, shape: Shape<Points>, options: &StrokeOptions) {
        shape
            .cast_unit()
            .stroke(
                Stroke::new(Color::new(
                    options.color.red,
                    options.color.green,
                    options.color.blue,
                    options.color.alpha,
                ))
                .line_width(options.line_width.cast_unit()),
            )
            .render_at(Point2D::default(), &self.target);
    }
}

impl Renderer for Kludgine {
    fn theme(&self) -> SystemTheme {
        match self.target.system_theme() {
            Theme::Light => SystemTheme::Light,
            Theme::Dark => SystemTheme::Dark,
        }
    }

    fn size(&self) -> gooey_core::euclid::Size2D<f32, Points> {
        self.target.clip.map_or_else(
            || self.target.size().cast_unit::<Points>(),
            |c| c.size.to_f32().cast_unit::<Pixels>() / self.scale(),
        )
    }

    fn clip_bounds(&self) -> Rect<f32, Points> {
        Rect::new(
            self.target
                .offset
                .unwrap_or_default()
                .to_point()
                .cast_unit::<Pixels>()
                / self.scale(),
            self.size(),
        )
    }

    fn clip_to(&self, bounds: Rect<f32, Points>) -> Self {
        // Kludgine's clipping is scene-relative, but the bounds in this function is
        // relative to the current rendering location.
        let mut scene_relative_bounds = bounds
            .translate(self.target.offset.unwrap_or_default().cast_unit::<Pixels>() / self.scale());
        if scene_relative_bounds.origin.x < 0. {
            scene_relative_bounds.size.width += scene_relative_bounds.origin.x;
            scene_relative_bounds.origin.x = 0.;
        }
        if scene_relative_bounds.origin.y < 0. {
            scene_relative_bounds.size.height += scene_relative_bounds.origin.y;
            scene_relative_bounds.origin.y = 0.;
        }

        if scene_relative_bounds.size.height < 0. {
            scene_relative_bounds.size.height = 0.;
        }

        if scene_relative_bounds.size.width < 0. {
            scene_relative_bounds.size.width = 0.;
        }

        Self::from(
            self.target
                .clipped_to(
                    (scene_relative_bounds * self.scale())
                        .round_out()
                        .to_u32()
                        .cast_unit(),
                )
                .offset_by((bounds.origin.to_vector() * self.scale()).cast_unit()),
        )
    }

    fn scale(&self) -> Scale<f32, Points, gooey_core::Pixels> {
        Scale::new(self.target.scale_factor().get())
    }

    fn render_text(
        &self,
        text: &str,
        baseline_origin: Point2D<f32, Points>,
        options: &TextOptions,
    ) {
        self.prepare_text(text, options)
            .render_baseline_at(&self.target, baseline_origin.cast_unit())
            .unwrap();
    }

    fn measure_text(&self, text: &str, options: &TextOptions) -> TextMetrics<Points> {
        let text = self.prepare_text(text, options);
        TextMetrics {
            width: text.width.cast_unit::<Pixels>(),
            ascent: Length::new(text.metrics.ascent),
            descent: Length::new(text.metrics.descent),
            line_gap: Length::new(text.metrics.line_gap),
        } / self.scale()
    }

    fn stroke_rect(&self, rect: &Rect<f32, Points>, options: &StrokeOptions) {
        self.stroke_shape(Shape::rect(*rect), options);
    }

    fn fill_rect(&self, rect: &Rect<f32, Points>, color: gooey_core::styles::Color) {
        Shape::rect(rect.cast_unit())
            .fill(Fill::new(Color::new(
                color.red,
                color.green,
                color.blue,
                color.alpha,
            )))
            .render_at(Point2D::default(), &self.target);
    }

    fn stroke_line(
        &self,
        point_a: Point2D<f32, Points>,
        point_b: Point2D<f32, Points>,
        options: &StrokeOptions,
    ) {
        self.stroke_shape(Shape::polygon(vec![point_a, point_b]), options);
    }
}
