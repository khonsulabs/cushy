use std::{borrow::Cow, sync::Arc};

use gooey_core::{
    figures::{Displayable, Figure, Point, Rect, Rectlike, Round, Size, Vector},
    styles::{Alignment, ColorPair, FallbackComponent, Style, VerticalAlignment},
    Scaled,
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
    pub fn size(&self) -> Size<f32, Scaled> {
        let (width, height) = self.lines.iter().map(PreparedLine::size).fold(
            (Figure::default(), Figure::default()),
            |(width, height), line_size| {
                (
                    width.max(Figure::new(line_size.width)),
                    height + line_size.height,
                )
            },
        );
        Size::from_figures(width, height)
    }

    pub(crate) fn align(&mut self, align_width: Figure<f32, Scaled>) {
        let mut last_alignment = Alignment::Left;
        for line in &mut self.lines {
            if let Some(span) = line.spans.first() {
                if let Some(alignment) = span.style().get() {
                    last_alignment = *alignment;
                }
            }
            match last_alignment {
                Alignment::Left => {
                    line.alignment_offset = Figure::default();
                }
                Alignment::Center => {
                    line.alignment_offset = (align_width - line.size().width) / 2.;
                }
                Alignment::Right => {
                    line.alignment_offset = align_width - line.size().width;
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
        renderer: &R,
        location: Point<f32, Scaled>,
        offset_baseline: bool,
        context_style: Option<&Style>,
    ) -> Figure<f32, Scaled> {
        let mut current_line_baseline = Figure::new(0.);

        for (line_index, line) in self.lines.iter().enumerate() {
            if offset_baseline || line_index > 0 {
                current_line_baseline += line.metrics.ascent;
            }
            let cursor_position =
                location + Vector::from_figures(line.alignment_offset, current_line_baseline);
            for span in &line.spans {
                let style = context_style.map_or_else(
                    || Cow::Borrowed(span.data.style.as_ref()),
                    |style| Cow::Owned(span.data.style.merge_with(style, false)),
                );
                let span_location = (cursor_position
                    + Vector::from_figures(span.location(), Figure::default()))
                .to_pixels(&renderer.scale())
                .round();
                renderer.render_text_with_style::<F, _>(&span.data.text, span_location, &style);
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
        renderer: &R,
        bounds: Rect<f32, Scaled>,
        style: &Style,
    ) -> Figure<f32, Scaled> {
        let bounds = bounds.as_sized();
        let text_size = self.size();
        let origin_y = match style.get::<VerticalAlignment>() {
            Some(VerticalAlignment::Bottom) => bounds.size.height - text_size.height,
            Some(VerticalAlignment::Center) => (bounds.size.height - text_size.height) / 2.,
            Some(VerticalAlignment::Top) | None => 0.,
        };

        self.render::<F, R>(
            renderer,
            bounds.origin + Vector::new(0., origin_y),
            true,
            Some(style),
        )
    }
}

/// A single line of prepared text.
#[derive(Debug, Clone)]
pub struct PreparedLine {
    /// The spans that comprise this line.
    pub spans: Vec<PreparedSpan>,
    /// The metrics of the line as a whole.
    pub metrics: TextMetrics<Scaled>,
    /// The offset of this line for the alignment. When rendering, each span's
    /// location is offset by this amount to account for [`Alignment`].
    pub alignment_offset: Figure<f32, Scaled>,
}

impl PreparedLine {
    /// The size of the bounding box of this line.
    #[must_use]
    pub fn size(&self) -> Size<f32, Scaled> {
        if self.spans.is_empty() {
            Size::from_figures(Figure::default(), self.height())
        } else {
            let width = self
                .spans
                .iter()
                .map(|s| s.data.metrics.width)
                .fold(Figure::default(), |sum, s| sum + s);

            Size::from_figures(width, self.height())
        }
    }

    /// The height of the line.
    #[must_use]
    pub fn height(&self) -> Figure<f32, Scaled> {
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
    pub(crate) fn new(
        style: Arc<Style>,
        text: String,
        offset: usize,
        length: usize,
        metrics: TextMetrics<Scaled>,
    ) -> Self {
        Self {
            data: Arc::new(PreparedSpanData {
                location: Figure::default(),
                offset,
                length,
                style,
                text,
                metrics,
            }),
        }
    }

    pub(crate) fn set_location(&mut self, location: Figure<f32, Scaled>) {
        Arc::make_mut(&mut self.data).location = location;
    }

    /// Returns the offset within the line of this text. Does not include alignment.
    #[must_use]
    pub fn location(&self) -> Figure<f32, Scaled> {
        self.data.location
    }

    /// Returns the metrics of this span.
    pub fn metrics(&self) -> &TextMetrics<Scaled> {
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

    /// Returns the offset, in characters, of this span.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.data.offset
    }

    /// Returns the length, in characters, of this span.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.length
    }

    /// Returns the length, in characters, of this span.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.length == 0
    }
}

#[derive(Clone, Debug)]
struct PreparedSpanData {
    location: Figure<f32, Scaled>,
    offset: usize,
    length: usize,
    style: Arc<Style>,
    text: String,
    metrics: TextMetrics<Scaled>,
}
