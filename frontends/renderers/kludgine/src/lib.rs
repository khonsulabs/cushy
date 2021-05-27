use gooey_core::{
    euclid::{Point2D, Rect},
    renderer::{Renderer, TextMetrics},
    styles::{
        BackgroundColor, FontSize, ForegroundColor, LineWidth, Points, Srgba, Style, SystemTheme,
    },
};
pub use kludgine;
use kludgine::{prelude::*, text::prepared::PreparedSpan};

#[derive(Debug)]
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
    fn prepare_text(&self, text: &str, options: &Style<Points>) -> PreparedSpan {
        let system_theme = options.get::<SystemTheme>().cloned().unwrap_or_default();
        Text::prepare(
            text,
            &bundled_fonts::ROBOTO,
            options
                .get::<FontSize<Points>>()
                .cloned()
                .unwrap_or_else(|| FontSize::new(13.))
                .length(),
            Color::from(
                options
                    .get::<ForegroundColor>()
                    .cloned()
                    .unwrap_or_else(|| ForegroundColor(Srgba::new(0., 0., 0., 1.).into()))
                    .0
                    .themed_color(&system_theme),
            ),
            &self.target,
        )
    }

    fn stroke_shape(&self, shape: Shape<Points>, style: &Style<Points>) {
        let system_theme = style.get::<SystemTheme>().cloned().unwrap_or_default();
        shape
            .stroke(
                Stroke::new(Color::from(
                    style
                        .get::<ForegroundColor>()
                        .cloned()
                        .unwrap_or_else(|| ForegroundColor(Srgba::new(0., 0., 0., 1.).into()))
                        .0
                        .themed_color(&system_theme),
                ))
                .line_width(
                    style
                        .get::<LineWidth<Points>>()
                        .cloned()
                        .unwrap_or_else(|| LineWidth::new(1.))
                        .length(),
                ),
            )
            .render_at(Point2D::default(), &self.target)
    }
}

impl Renderer for Kludgine {
    fn size(&self) -> gooey_core::euclid::Size2D<f32, Points> {
        self.target
            .clip
            .map(|c| c.size.to_f32() / self.scale())
            .unwrap_or_else(|| self.target.size())
    }

    fn clip_bounds(&self) -> Rect<f32, Points> {
        Rect::new(
            self.target.offset.unwrap_or_default().to_point() / self.scale(),
            self.size(),
        )
    }

    fn clip_to(&self, bounds: Rect<f32, Points>) -> Self {
        // Kludgine's clipping is scene-relative, but the bounds in this function is
        // relative to the current rendering location.
        let scene_relative_bounds =
            bounds.translate(self.target.offset.unwrap_or_default() / self.scale());

        Kludgine::from(
            self.target
                .clipped_to((scene_relative_bounds * self.scale()).to_u32())
                .offset_by(bounds.origin.to_vector() * self.scale()),
        )
    }

    fn scale(&self) -> Scale<f32, Points, gooey_core::styles::Pixels> {
        self.target.scale_factor()
    }

    fn render_text(
        &self,
        text: &str,
        baseline_origin: Point2D<f32, Points>,
        options: &Style<Points>,
    ) {
        self.prepare_text(text, options)
            .render_baseline_at(&self.target, baseline_origin)
            .unwrap();
    }

    fn measure_text(
        &self,
        text: &str,
        options: &Style<Points>,
    ) -> gooey_core::renderer::TextMetrics<Points> {
        let text = self.prepare_text(text, options);
        let vmetrics = text.metrics();
        TextMetrics {
            width: text.data.width,
            ascent: Pixels::new(vmetrics.ascent),
            descent: Pixels::new(vmetrics.descent),
            line_gap: Pixels::new(vmetrics.line_gap),
        } / self.target.scale_factor()
    }

    fn stroke_rect(&self, rect: &Rect<f32, Points>, style: &Style<Points>) {
        self.stroke_shape(Shape::rect(*rect), style);
    }

    fn fill_rect(&self, rect: &Rect<f32, Points>, style: &Style<Points>) {
        let system_theme = style.get::<SystemTheme>().cloned().unwrap_or_default();
        Shape::rect(*rect)
            .fill(Fill::new(Color::from(
                style
                    .get::<BackgroundColor>()
                    .cloned()
                    .unwrap_or_else(|| BackgroundColor(Srgba::new(1., 1., 1., 1.).into()))
                    .0
                    .themed_color(&system_theme),
            )))
            .render_at(Point2D::default(), &self.target);
    }

    fn stroke_line(
        &self,
        point_a: Point2D<f32, Points>,
        point_b: Point2D<f32, Points>,
        style: &Style<Points>,
    ) {
        self.stroke_shape(Shape::polygon(vec![point_a, point_b]), style);
    }
}
