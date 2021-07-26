use std::sync::Arc;

use gooey_core::{
    euclid::{Length, Point2D, Rect, Size2D, Vector2D},
    styles::{Alignment, ColorPair, FallbackComponent, Style, VerticalAlignment},
    Points,
};
use gooey_renderer::{Renderer, TextMetrics};

/// A [`Text`](crate::Text) that has been measured and is ready to render.
#[derive(Default, Debug, Clone)]
pub struct PreparedText {
    /// The prepared lines of text.
    pub lines: Vec<PreparedLine>,
}

impl PreparedText {
    /// Returns the total size this text will occupy when rendered.
    #[must_use]
    pub fn size(&self) -> Size2D<f32, Points> {
        let (width, height) = self.lines.iter().map(PreparedLine::size).fold(
            (Length::default(), Length::default()),
            |(width, height), line_size| {
                (
                    width.max(Length::new(line_size.width)),
                    height + Length::new(line_size.height),
                )
            },
        );
        Size2D::from_lengths(width, height)
    }

    pub(crate) fn align(&mut self, align_width: Length<f32, Points>) {
        let mut last_alignment = Alignment::Left;
        for line in &mut self.lines {
            if let Some(span) = line.spans.first() {
                if let Some(alignment) = span.style().get() {
                    last_alignment = *alignment;
                }
            }
            match last_alignment {
                Alignment::Left => {
                    line.alignment_offset = Length::default();
                }
                Alignment::Center => {
                    line.alignment_offset = (align_width - Length::new(line.size().width)) / 2.;
                }
                Alignment::Right => {
                    line.alignment_offset = align_width - Length::new(line.size().width);
                }
            }
        }
    }

    /// Renders this text at `location`. If `offset_baseline` is true, the text
    /// will be rendered with an additional offset such that the top-left of the
    /// rendered bounding box will be `location`. Otherwise, the baseline of the
    /// first line will be `location`.
    pub fn render<F: FallbackComponent<Value = ColorPair>, R: Renderer>(
        &self,
        scene: &R,
        location: Point2D<f32, Points>,
        offset_baseline: bool,
    ) -> Length<f32, Points> {
        let mut current_line_baseline = Length::new(0.);

        for (line_index, line) in self.lines.iter().enumerate() {
            if offset_baseline || line_index > 0 {
                current_line_baseline += line.metrics.ascent;
            }
            let cursor_position =
                location + Vector2D::from_lengths(line.alignment_offset, current_line_baseline);
            for span in &line.spans {
                scene.render_text::<F>(
                    &span.data.text,
                    cursor_position + Vector2D::from_lengths(span.location(), Length::default()),
                    &span.data.style,
                );
            }
            current_line_baseline += line.metrics.line_gap - line.metrics.descent;
        }

        current_line_baseline
    }

    /// Renders this text within `bounds` honoring [`VerticalAlignment`] from
    /// `style`. This does not affect the alignment of text, just the vertical
    /// location of the text block rendered within `bounds`.
    pub fn render_within<F: FallbackComponent<Value = ColorPair>, R: Renderer>(
        &self,
        scene: &R,
        bounds: Rect<f32, Points>,
        style: &Style,
    ) -> Length<f32, Points> {
        let text_size = self.size();
        let origin_y = match style.get::<VerticalAlignment>() {
            Some(VerticalAlignment::Bottom) => bounds.size.height - text_size.height,
            Some(VerticalAlignment::Center) => (bounds.size.height - text_size.height) / 2.,
            Some(VerticalAlignment::Top) | None => 0.,
        };

        self.render::<F, R>(scene, bounds.origin + Vector2D::new(0., origin_y), true)
    }
}

/// A single line of prepared text.
#[derive(Debug, Clone)]
pub struct PreparedLine {
    /// The spans that comprise this line.
    pub spans: Vec<PreparedSpan>,
    /// The metrics of the line as a whole.
    pub metrics: TextMetrics<Points>,
    /// The offset of this line for the alignment. When rendering, each span's
    /// location is offset by this amount to account for [`Alignment`].
    pub alignment_offset: Length<f32, Points>,
}

impl PreparedLine {
    /// The size of the bounding box of this line.
    #[must_use]
    pub fn size(&self) -> Size2D<f32, Points> {
        if self.spans.is_empty() {
            Size2D::from_lengths(Length::default(), self.height())
        } else {
            let width = self
                .spans
                .iter()
                .map(|s| s.data.metrics.width)
                .fold(Length::default(), |sum, s| sum + s);

            Size2D::from_lengths(width, self.height())
        }
    }

    /// The height of the line.
    #[must_use]
    pub fn height(&self) -> Length<f32, Points> {
        self.metrics.line_height()
    }
}

/// A prepared [`Span`](crate::Span).
#[derive(Clone, Debug)]
#[must_use]
pub struct PreparedSpan {
    data: Arc<PreparedSpanData>,
}

impl PreparedSpan {
    /// Returns a new span with `style`, `text`, and `metrics`.
    pub(crate) fn new(style: Arc<Style>, text: String, metrics: TextMetrics<Points>) -> Self {
        Self {
            data: Arc::new(PreparedSpanData {
                location: Length::default(),
                style,
                text,
                metrics,
            }),
        }
    }

    pub(crate) fn set_location(&mut self, location: Length<f32, Points>) {
        Arc::make_mut(&mut self.data).location = location;
    }

    /// Returns the offset within the line of this text. Does not include alignment.
    #[must_use]
    pub fn location(&self) -> Length<f32, Points> {
        self.data.location
    }

    /// Returns the metrics of this span.
    pub fn metrics(&self) -> &TextMetrics<Points> {
        &self.data.metrics
    }

    /// Returns the style of this span.
    #[must_use]
    pub fn style(&self) -> &Style {
        &self.data.style
    }

    /// Returns the text of this span.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.data.text
    }
}

#[derive(Clone, Debug)]
struct PreparedSpanData {
    location: Length<f32, Points>,
    style: Arc<Style>,
    text: String,
    metrics: TextMetrics<Points>,
}
