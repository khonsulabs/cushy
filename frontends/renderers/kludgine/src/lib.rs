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
    figures::{DisplayScale, Displayable, Point, Rect, Rectlike, Vectorlike},
    styles::SystemTheme,
    Pixels, Scaled,
};
use gooey_rasterizer::ImageExt;
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
            options.text_size,
            Color::new(
                options.color.red,
                options.color.green,
                options.color.blue,
                options.color.alpha,
            ),
            &self.target,
        )
    }

    fn stroke_shape(&self, shape: Shape<Scaled>, options: &StrokeOptions) {
        shape
            .cast_unit()
            .stroke(
                Stroke::new(Color::new(
                    options.color.red,
                    options.color.green,
                    options.color.blue,
                    options.color.alpha,
                ))
                .line_width(options.line_width),
            )
            .render_at(Point::default(), &self.target);
    }
}

impl Renderer for Kludgine {
    fn theme(&self) -> SystemTheme {
        match self.target.system_theme() {
            Theme::Light => SystemTheme::Light,
            Theme::Dark => SystemTheme::Dark,
        }
    }

    fn size(&self) -> gooey_core::figures::Size<f32, Scaled> {
        self.target.clip.map_or_else(
            || self.target.size(),
            |c| c.size.cast::<f32>().to_scaled(&self.scale()),
        )
    }

    fn clip_bounds(&self) -> Rect<f32, Scaled> {
        Rect::sized(
            self.target
                .offset
                .unwrap_or_default()
                .to_point()
                .cast_unit::<Pixels>()
                .to_scaled(&self.scale()),
            self.size(),
        )
    }

    fn clip_to(&self, bounds: Rect<f32, Scaled>) -> Self {
        // Kludgine's clipping is scene-relative, but the bounds in this function is
        // relative to the current rendering location.
        let bounds = bounds.as_sized();
        let mut scene_relative_bounds = bounds;
        if let Some(offset) = self.target.offset {
            scene_relative_bounds = scene_relative_bounds
                .translate(offset.cast_unit::<Pixels>().to_scaled(&self.scale()));
        }

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

        let scene_relative_bounds = (scene_relative_bounds.to_pixels(&self.scale()))
            .round_out()
            .cast::<u32>();

        Self::from(
            self.target
                .clipped_to(scene_relative_bounds)
                .offset_by(bounds.origin.to_vector().to_pixels(&self.scale())),
        )
    }

    fn scale(&self) -> DisplayScale<f32> {
        *self.target.scale()
    }

    fn render_text(&self, text: &str, baseline_origin: Point<f32, Scaled>, options: &TextOptions) {
        self.prepare_text(text, options)
            .render_baseline_at(&self.target, baseline_origin.cast_unit())
            .unwrap();
    }

    fn measure_text(&self, text: &str, options: &TextOptions) -> TextMetrics<Scaled> {
        let text = self.prepare_text(text, options);
        TextMetrics {
            width: text.width.cast_unit::<Pixels>(),
            ascent: Figure::new(text.metrics.ascent),
            descent: Figure::new(text.metrics.descent),
            line_gap: Figure::new(text.metrics.line_gap),
        }
        .to_scaled(&self.scale())
    }

    fn stroke_rect(&self, rect: &Rect<f32, Scaled>, options: &StrokeOptions) {
        self.stroke_shape(Shape::rect(rect.as_sized()), options);
    }

    fn fill_rect(&self, rect: &Rect<f32, Scaled>, color: gooey_core::styles::Color) {
        Shape::rect(rect.as_sized())
            .fill(Fill::new(Color::new(
                color.red,
                color.green,
                color.blue,
                color.alpha,
            )))
            .render_at(Point::default(), &self.target);
    }

    fn stroke_line(
        &self,
        point_a: Point<f32, Scaled>,
        point_b: Point<f32, Scaled>,
        options: &StrokeOptions,
    ) {
        self.stroke_shape(Shape::polygon(vec![point_a, point_b]), options);
    }

    fn draw_image(&self, image: &gooey_core::assets::Image, location: Point<f32, Scaled>) {
        if let Some(image) = image.as_rgba_image() {
            let texture = Texture::new(image);
            let sprite = SpriteSource::entire_texture(texture);
            sprite.render_at(&self.target, location, SpriteRotation::default());
        }
    }
}
