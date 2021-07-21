use std::sync::Arc;

use gooey_core::{
    euclid::{Length, Point2D, Size2D, Vector2D},
    styles::{Alignment, ForegroundColor, Style},
    Points,
};
use gooey_renderer::{Renderer, TextMetrics};

/// A [`Text`] that has been measured and is ready to render.
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

    #[allow(clippy::needless_collect)] // false positive, needed to get rid of borrow.
    pub(crate) fn align(&mut self, alignment: Alignment, width: Length<f32, Points>) {
        let line_sizes = self
            .lines
            .iter()
            .map(PreparedLine::size)
            .collect::<Vec<_>>();
        for (i, size) in line_sizes.into_iter().enumerate() {
            match alignment {
                Alignment::Left => {
                    self.lines[i].alignment_offset = Length::default();
                }
                Alignment::Center => {
                    self.lines[i].alignment_offset = (width - Length::new(size.width)) / 2.;
                }
                Alignment::Right => {
                    self.lines[i].alignment_offset = width - Length::new(size.width);
                }
            }
        }
    }

    /// Renders this text at `location`. If `offset_baseline` is true, the text
    /// will be rendered with an additional offset such that the top-left of the
    /// rendered bounding box will be `location`. Otherwise, the baseline of the
    /// first line will be `location`.
    pub fn render<R: Renderer>(
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
            let metrics = line.metrics;
            let cursor_position =
                location + Vector2D::from_lengths(line.alignment_offset, current_line_baseline);
            for span in &line.spans {
                scene.render_text::<ForegroundColor>(
                    &span.data.text,
                    cursor_position + Vector2D::from_lengths(span.location(), Length::default()),
                    &span.data.style,
                );
            }
            current_line_baseline += metrics.line_gap - metrics.descent;
        }

        current_line_baseline
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

/// A prepared [`Span`].
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
