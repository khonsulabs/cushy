//! Cross-platform rendering types.

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    missing_docs,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![allow(clippy::if_not_else)]
#![cfg_attr(doc, warn(rustdoc::all))]

use std::fmt::Debug;

use gooey_core::{
    euclid::{Length, Point2D, Rect, Scale, Size2D},
    palette::Srgba,
    styles::{
        Color, ColorPair, FallbackComponent, FontFamily, FontSize, ForegroundColor, LineWidth,
        Style, SystemTheme,
    },
    Pixels, Points,
};

/// Implements drawing APIs.
pub trait Renderer: Debug + Send + Sync + Sized + 'static {
    /// The size of the area being drawn.
    fn size(&self) -> Size2D<f32, Points>;

    /// Returns the current system theme.
    fn theme(&self) -> SystemTheme;

    /// A [`Rect`] representing the area being drawn. Due to how rendering
    /// works, the origin is always zero.
    fn bounds(&self) -> Rect<f32, Points> {
        Rect::new(Point2D::default(), self.size())
    }

    /// Returns a new renderer instance with the state such that each operation
    /// executes as if the origin is `bounds.origin`. The returned instance's
    /// `size()` should equal `bounds.size`.
    fn clip_to(&self, bounds: Rect<f32, Points>) -> Self;

    /// A [`Rect`] representing the area being drawn. This rect should be offset
    /// relative to the origin of the renderer.
    fn clip_bounds(&self) -> Rect<f32, Points>;

    /// The scale when converting between [`Points`] and [`Pixels`].
    fn scale(&self) -> Scale<f32, Points, Pixels>;

    /// Renders `text` at `baseline_origin` with `options`.
    fn render_text(&self, text: &str, baseline_origin: Point2D<f32, Points>, options: &TextOptions);

    /// Renders `text` at `baseline_origin` with `options`.
    fn render_text_with_style<F: FallbackComponent<Value = ColorPair>>(
        &self,
        text: &str,
        baseline_origin: Point2D<f32, Points>,
        style: &Style,
    ) {
        self.render_text(
            text,
            baseline_origin,
            &TextOptions::from_style::<F>(style, self.theme()),
        )
    }

    /// Measures `text` using `options`.
    fn measure_text(&self, text: &str, options: &TextOptions) -> TextMetrics<Points>;

    /// Measures `text` using `style`.
    fn measure_text_with_style(&self, text: &str, style: &Style) -> TextMetrics<Points> {
        self.measure_text(
            text,
            &TextOptions::from_style::<ForegroundColor>(style, self.theme()),
        )
    }

    /// Fills `rect` using `color`.
    fn fill_rect(&self, rect: &Rect<f32, Points>, color: Color);

    /// Fills `rect` using `style`.
    fn fill_rect_with_style<F: FallbackComponent<Value = ColorPair>>(
        &self,
        rect: &Rect<f32, Points>,
        style: &Style,
    ) {
        self.fill_rect(
            rect,
            style
                .get_with_fallback::<F>()
                .copied()
                .unwrap_or_else(|| Srgba::new(1., 1., 1., 1.).into())
                .themed_color(self.theme()),
        )
    }

    /// Strokes the outline of `rect` using `options`.
    fn stroke_rect(&self, rect: &Rect<f32, Points>, options: &StrokeOptions);

    /// Strokes the outline of `rect` using `style`.
    fn stroke_rect_with_style<F: FallbackComponent<Value = ColorPair>>(
        &self,
        rect: &Rect<f32, Points>,
        style: &Style,
    ) {
        self.stroke_rect(rect, &StrokeOptions::from_style::<F>(style, self.theme()))
    }

    /// Draws a line between `point_a` and `point_b` using `options`.
    fn stroke_line(
        &self,
        point_a: Point2D<f32, Points>,
        point_b: Point2D<f32, Points>,
        options: &StrokeOptions,
    );

    /// Draws a line between `point_a` and `point_b` using `style`.
    fn stroke_line_with_style<F: FallbackComponent<Value = ColorPair>>(
        &self,
        point_a: Point2D<f32, Points>,
        point_b: Point2D<f32, Points>,
        style: &Style,
    ) {
        self.stroke_line(
            point_a,
            point_b,
            &StrokeOptions::from_style::<F>(style, self.theme()),
        )
    }
}

/// Text rendering and measurement options.
#[must_use]
pub struct TextOptions {
    /// The font family to use.
    pub font_family: Option<String>,
    /// The text size, in [`Points`].
    pub text_size: Length<f32, Points>,
    /// The color to render.
    pub color: Color,
}

impl TextOptions {
    /// Returns a default `TextOptionsBuilder`.
    pub fn build() -> TextOptionsBuilder {
        TextOptionsBuilder::default()
    }

    /// Returns options initialized from `style`, using the generic `TextColor`
    /// and `theme` to resolve the color.
    pub fn from_style<TextColor: FallbackComponent<Value = ColorPair>>(
        style: &Style,
        theme: SystemTheme,
    ) -> Self {
        TextOptions::build()
            .font_family(
                style
                    .get::<FontFamily>()
                    .cloned()
                    .unwrap_or_else(|| FontFamily::from("Roboto"))
                    .0,
            )
            .text_size(
                style
                    .get::<FontSize<Points>>()
                    .copied()
                    .unwrap_or_else(|| FontSize::new(13.))
                    .get(),
            )
            .color(
                style
                    .get_with_fallback::<TextColor>()
                    .copied()
                    .unwrap_or_else(|| Srgba::new(0., 0., 0., 1.).into())
                    .themed_color(theme),
            )
            .finish()
    }
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            font_family: None,
            text_size: Length::new(13.),
            color: Color::default(),
        }
    }
}

/// Builds [`TextOptions`]
#[derive(Default)]
#[must_use]
pub struct TextOptionsBuilder {
    options: TextOptions,
}

impl TextOptionsBuilder {
    /// Sets the font family to `family`.
    pub fn font_family(mut self, family: impl Into<String>) -> Self {
        self.options.font_family = Some(family.into());
        self
    }

    /// Sets the text size to `size_in_points`.
    pub fn text_size(mut self, size_in_points: f32) -> Self {
        self.options.text_size = Length::new(size_in_points);
        self
    }

    /// Sets the color to `color`.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.options.color = color.into();
        self
    }

    /// Returns the built options.
    pub fn finish(self) -> TextOptions {
        self.options
    }
}

/// Shape outline drawing options.
#[must_use]
pub struct StrokeOptions {
    /// The color to stroke.
    pub color: Color,
    /// The width of each line segment.
    pub line_width: Length<f32, Points>,
}

impl Default for StrokeOptions {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            line_width: Length::new(1.),
        }
    }
}

impl StrokeOptions {
    /// Returns a default `StrokeOptionsBuilder`.
    pub fn build() -> StrokeOptionsBuilder {
        StrokeOptionsBuilder::default()
    }

    /// Returns options initialized from `style` using `F` and `theme` to
    /// resolve the color of the stroke.
    pub fn from_style<F: FallbackComponent<Value = ColorPair>>(
        style: &Style,
        theme: SystemTheme,
    ) -> Self {
        Self {
            color: style
                .get_with_fallback::<F>()
                .copied()
                .unwrap_or_else(|| Srgba::new(0., 0., 0., 1.).into())
                .themed_color(theme),
            line_width: style
                .get::<LineWidth<Points>>()
                .copied()
                .unwrap_or_else(|| LineWidth::new(1.))
                .length(),
        }
    }
}

/// Builds [`StrokeOptions`]
#[derive(Default)]
#[must_use]
pub struct StrokeOptionsBuilder {
    options: StrokeOptions,
}

impl StrokeOptionsBuilder {
    /// Sets the width of the line stroked to `width_in_points`.
    pub fn line_width(mut self, width_in_points: f32) -> Self {
        self.options.line_width = Length::new(width_in_points);
        self
    }

    /// Sets the color to `color`.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.options.color = color.into();
        self
    }

    /// Returns the built options.
    pub fn finish(self) -> StrokeOptions {
        self.options
    }
}

/// A measurement of text.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[must_use]
pub struct TextMetrics<U> {
    /// The width of the text.
    pub width: Length<f32, U>,
    /// The height above the baseline.
    pub ascent: Length<f32, U>,
    /// The height below the baseline. Typically a negative number.
    pub descent: Length<f32, U>,
    /// The spacing expected between lines of text.
    pub line_gap: Length<f32, U>,
}

impl<U> TextMetrics<U> {
    /// The height of the rendered text. This is computed by combining
    /// [`ascent`](TextMetrics::ascent) and [`descent`](TextMetrics::descent).
    #[must_use]
    pub fn height(&self) -> Length<f32, U> {
        self.ascent - self.descent
    }

    /// The height of a line of text. This is computed by combining
    /// [`height()`](TextMetrics::height) and
    /// [`line_gap`](TextMetrics::line_gap)
    #[must_use]
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
