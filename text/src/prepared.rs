use std::sync::Arc;

use gooey_core::{
    euclid::{Length, Point2D, Size2D, Vector2D},
    styles::{Alignment, ForegroundColor, Style},
    Points,
};
use gooey_renderer::{Renderer, TextMetrics};

#[derive(Default, Debug, Clone)]
pub struct PreparedText {
    pub lines: Vec<PreparedLine>,
}

impl PreparedText {
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
                    cursor_position + Vector2D::from_lengths(span.location, Length::default()),
                    &span.data.style,
                );
            }
            current_line_baseline += metrics.line_gap - metrics.descent;
        }

        current_line_baseline
    }
}

#[derive(Debug, Clone)]
pub struct PreparedLine {
    pub spans: Vec<PreparedSpan>,
    pub metrics: TextMetrics<Points>,
    pub alignment_offset: Length<f32, Points>,
}

impl PreparedLine {
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

    #[must_use]
    pub fn height(&self) -> Length<f32, Points> {
        self.metrics.line_height()
    }
}

#[derive(Clone, Debug)]
#[must_use]
pub struct PreparedSpan {
    pub location: Length<f32, Points>,
    pub data: Arc<PreparedSpanData>,
}

impl PreparedSpan {
    pub fn new(style: Arc<Style>, text: String, metrics: TextMetrics<Points>) -> Self {
        Self {
            location: Length::default(),
            data: Arc::new(PreparedSpanData {
                style,
                text,
                metrics,
            }),
        }
    }

    pub fn translate(&self, location: Length<f32, Points>) -> Self {
        Self {
            // TODO: We want to ensure that we are pixel-aligned when rendering a span's start.
            location,
            data: self.data.clone(),
        }
    }

    pub(crate) fn metrics(&self) -> TextMetrics<Points> {
        self.data.metrics
    }
}

#[derive(Debug)]
pub struct PreparedSpanData {
    pub style: Arc<Style>,
    pub text: String,
    pub metrics: TextMetrics<Points>,
}
