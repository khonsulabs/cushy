use euclid::{Length, Point2D, Rect, Scale, Size2D};
use stylecs::{palette::Srgba, Pixels, Points};

pub trait Renderer: Send + Sync {
    fn size(&self) -> Size2D<f32, Points>;
    fn scale(&self) -> Scale<f32, Points, Pixels>;

    fn render_text(&self, text: &str, baseline_origin: Point2D<f32, Points>, options: &TextOptions);
    fn measure_text(&self, text: &str, options: &TextOptions) -> TextMetrics<Points>;

    fn stroke_rect(&self, rect: &Rect<f32, Points>, options: StrokeOptions);
    fn fill_rect(&self, rect: &Rect<f32, Points>, color: Srgba);

    fn stroke_line(
        &self,
        point_a: Point2D<f32, Points>,
        point_b: Point2D<f32, Points>,
        options: StrokeOptions,
    );
}

pub struct TextOptions {
    pub font_family: Option<String>,
    pub text_size: Length<f32, Points>,
    pub text_align: stylecs::Alignment,
    pub max_width: Option<Length<f32, Points>>,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            font_family: None,
            text_size: Length::new(13.),
            text_align: stylecs::Alignment::Left,
            max_width: None,
        }
    }
}

pub struct StrokeOptions {
    pub color: Srgba,
    pub line_width: Length<f32, Points>,
}

pub struct TextMetrics<U> {
    pub width: Length<f32, U>,
    pub ascent: Length<f32, U>,
    pub descent: Length<f32, U>,
    pub line_gap: Length<f32, U>,
}

impl<U> TextMetrics<U> {
    pub fn height(&self) -> Length<f32, U> {
        self.ascent - self.descent
    }

    pub fn line_height(&self) -> Length<f32, U> {
        self.height() + self.line_gap
    }
}

impl<U, V> std::ops::Mul<Scale<f32, U, V>> for TextMetrics<U> {
    type Output = TextMetrics<V>;

    fn mul(self, rhs: Scale<f32, U, V>) -> Self::Output {
        TextMetrics {
            width: self.width * rhs,
            ascent: self.ascent * rhs,
            descent: self.descent * rhs,
            line_gap: self.line_gap * rhs,
        }
    }
}

impl<U, V> std::ops::Div<Scale<f32, U, V>> for TextMetrics<V> {
    type Output = TextMetrics<U>;

    fn div(self, rhs: Scale<f32, U, V>) -> Self::Output {
        TextMetrics {
            width: self.width / rhs,
            ascent: self.ascent / rhs,
            descent: self.descent / rhs,
            line_gap: self.line_gap / rhs,
        }
    }
}
