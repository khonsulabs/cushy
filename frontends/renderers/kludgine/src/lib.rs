use gooey_core::{
    euclid::{Point2D, Rect},
    renderer::{Renderer, StrokeOptions, TextMetrics, TextOptions},
    stylecs::Points,
};
pub use kludgine;
use kludgine::{prelude::*, text::prepared::VMetrics};

pub struct Kludgine {
    target: Target,
}

impl<'a> From<&'a Target> for Kludgine {
    fn from(target: &'a Target) -> Self {
        Self {
            target: target.clone(),
        }
    }
}

impl Renderer for Kludgine {
    fn size(&self) -> gooey_core::euclid::Size2D<f32, Points> {
        self.target.size()
    }

    fn scale(&self) -> Scale<f32, Points, gooey_core::stylecs::Pixels> {
        self.target.scale_factor()
    }

    fn render_text(
        &self,
        text: &str,
        baseline_origin: Point2D<f32, Points>,
        options: &TextOptions,
    ) {
        Text::span(text, Style::default())
            .render_baseline_at(&self.target, baseline_origin, TextWrap::NoWrap)
            .unwrap();
    }

    fn measure_text(
        &self,
        text: &str,
        options: &TextOptions,
    ) -> gooey_core::renderer::TextMetrics<Points> {
        let text = Text::span(text, Style::default())
            .wrap(&self.target, TextWrap::NoWrap)
            .unwrap();
        let vmetrics = text
            .lines
            .first()
            .map(|line| line.metrics)
            .unwrap_or(VMetrics {
                ascent: Length::default(),
                descent: Length::default(),
                line_gap: Length::default(),
            });
        TextMetrics {
            width: text.size().width(),
            ascent: vmetrics.ascent,
            descent: vmetrics.descent,
            line_gap: vmetrics.line_gap,
        } / self.target.scale_factor()
    }

    fn stroke_rect(&self, rect: &Rect<f32, Points>, options: StrokeOptions) {
        Shape::rect(*rect)
            .stroke(Stroke::new(Color::from(options.color)))
            .render_at(Point2D::default(), &self.target);
    }

    fn fill_rect(&self, rect: &Rect<f32, Points>, color: gooey_core::stylecs::palette::Srgba) {
        Shape::rect(*rect)
            .fill(Fill::new(Color::from(color)))
            .render_at(Point2D::default(), &self.target);
    }

    fn stroke_line(
        &self,
        point_a: Point2D<f32, Points>,
        point_b: Point2D<f32, Points>,
        options: StrokeOptions,
    ) {
        todo!()
    }
}
