use gooey_core::{
    euclid::{Point2D, Rect},
    palette::Srgba,
    renderer::{Renderer, TextMetrics},
    styles::{
        ColorPair, FallbackComponent, FontSize, ForegroundColor, LineWidth, Style, SystemTheme,
        TextColor,
    },
    Pixels, Points,
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
    fn prepare_text(&self, text: &str, options: &Style) -> PreparedSpan {
        let system_theme = options.get::<SystemTheme>().cloned().unwrap_or_default();
        Text::prepare(
            text,
            &bundled_fonts::ROBOTO,
            options
                .get::<FontSize<Points>>()
                .cloned()
                .unwrap_or_else(|| FontSize::new(13.))
                .length()
                .cast_unit(),
            Color::from(
                options
                    .get_with_fallback::<TextColor>()
                    .cloned()
                    .unwrap_or_else(|| Srgba::new(0., 0., 0., 1.).into())
                    .themed_color(&system_theme),
            ),
            &self.target,
        )
    }

    fn stroke_shape(&self, shape: Shape<Points>, style: &Style) {
        let system_theme = style.get::<SystemTheme>().cloned().unwrap_or_default();
        shape
            .cast_unit()
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
                        .length()
                        .cast_unit(),
                ),
            )
            .render_at(Point2D::default(), &self.target)
    }
}

impl Renderer for Kludgine {
    fn size(&self) -> gooey_core::euclid::Size2D<f32, Points> {
        self.target
            .clip
            .map(|c| c.size.to_f32().cast_unit::<Pixels>() / self.scale())
            .unwrap_or_else(|| self.target.size().cast_unit::<Points>())
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
        let scene_relative_bounds = bounds
            .translate(self.target.offset.unwrap_or_default().cast_unit::<Pixels>() / self.scale());

        Kludgine::from(
            self.target
                .clipped_to((scene_relative_bounds * self.scale()).to_u32().cast_unit())
                .offset_by((bounds.origin.to_vector() * self.scale()).cast_unit()),
        )
    }

    fn scale(&self) -> Scale<f32, Points, gooey_core::Pixels> {
        Scale::new(self.target.scale_factor().get())
    }

    fn render_text(&self, text: &str, baseline_origin: Point2D<f32, Points>, options: &Style) {
        self.prepare_text(text, options)
            .render_baseline_at(&self.target, baseline_origin.cast_unit())
            .unwrap();
    }

    fn measure_text(&self, text: &str, options: &Style) -> TextMetrics<Points> {
        let text = self.prepare_text(text, options);
        let vmetrics = text.metrics();
        TextMetrics {
            width: text.data.width.cast_unit::<Pixels>(),
            ascent: Length::new(vmetrics.ascent),
            descent: Length::new(vmetrics.descent),
            line_gap: Length::new(vmetrics.line_gap),
        } / self.scale()
    }

    fn stroke_rect(&self, rect: &Rect<f32, Points>, style: &Style) {
        self.stroke_shape(Shape::rect(*rect), style);
    }

    fn fill_rect<F: FallbackComponent<Value = ColorPair>>(
        &self,
        rect: &Rect<f32, Points>,
        style: &Style,
    ) {
        let system_theme = style.get::<SystemTheme>().cloned().unwrap_or_default();
        Shape::rect(rect.cast_unit())
            .fill(Fill::new(Color::from(
                style
                    .get_with_fallback::<F>()
                    .cloned()
                    .unwrap_or_else(|| Srgba::new(1., 1., 1., 1.).into())
                    .themed_color(&system_theme),
            )))
            .render_at(Point2D::default(), &self.target);
    }

    fn stroke_line(
        &self,
        point_a: Point2D<f32, Points>,
        point_b: Point2D<f32, Points>,
        style: &Style,
    ) {
        self.stroke_shape(Shape::polygon(vec![point_a, point_b]), style);
    }
}
