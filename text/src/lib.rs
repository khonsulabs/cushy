//! Text rendering and wrapping.

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    missing_docs,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![allow(clippy::if_not_else, clippy::module_name_repetitions)]
#![cfg_attr(doc, warn(rustdoc::all))]

use std::{fmt::Display, ops::Range};

use gooey_core::{
    euclid::Point2D,
    styles::{ColorPair, FallbackComponent, Style},
    Points,
};
use gooey_renderer::Renderer;
use prepared::PreparedText;
use wrap::{TextWrap, TextWrapper};

/// Measured and laid out text types ready to render.
pub mod prepared;
// pub mod rich;
/// Text wrapping functionality.
pub mod wrap;

/// A styled String.
#[derive(Debug, Clone, Default)]
pub struct Span {
    /// The text to draw.
    pub text: String,
    /// The style to use when drawing.
    pub style: Style,
}

impl Span {
    /// Returns a new span with `text` and `style`.
    pub fn new<S: Into<String>>(text: S, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

/// A sequence of [`Spans`][Span].
#[derive(Debug, Clone)]
#[must_use]
pub struct Text {
    spans: Vec<Span>,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            spans: vec![Span::default()],
        }
    }
}

impl Text {
    /// Returns a new `Text` with a single span created from `text` and `style`.
    pub fn span<S: Into<String>>(text: S, style: Style) -> Self {
        Self::from(vec![Span::new(text, style)])
    }

    /// Calculates how to render this text and returns the results.
    pub fn wrap<R: Renderer>(&self, renderer: &R, options: TextWrap) -> PreparedText {
        TextWrapper::wrap(self, renderer, options)
    }

    /// Renders this text at `location` in `renderer`. The top-left of the bounding box of the text will be at `location`.
    pub fn render_at<F: FallbackComponent<Value = ColorPair>, R: Renderer>(
        &self,
        renderer: &R,
        location: Point2D<f32, Points>,
        wrapping: TextWrap,
    ) {
        self.render_core::<F, R>(renderer, location, true, wrapping);
    }

    /// Renders this text at `location` in `renderer`. The baseline of the first line will start at `location`.
    pub fn render_baseline_at<F: FallbackComponent<Value = ColorPair>, R: Renderer>(
        &self,
        scene: &R,
        location: Point2D<f32, Points>,
        wrapping: TextWrap,
    ) {
        self.render_core::<F, R>(scene, location, false, wrapping);
    }

    fn render_core<F: FallbackComponent<Value = ColorPair>, R: Renderer>(
        &self,
        scene: &R,
        location: Point2D<f32, Points>,
        offset_baseline: bool,
        wrapping: TextWrap,
    ) {
        let prepared_text = self.wrap(scene, wrapping);
        prepared_text.render::<F, R>(scene, location, offset_baseline);
    }

    /// Removes text in `range`. Empty spans will be removed.
    pub fn remove_range(&mut self, range: Range<usize>) {
        self.for_each_in_range_mut(range, |span, relative_range| {
            span.text.replace_range(relative_range, "");
        });
    }

    /// Inserts `value` at `offset`. Inserts into an existing span.
    #[allow(clippy::range_plus_one)]
    pub fn insert_str(&mut self, offset: usize, value: &str) {
        self.for_each_in_range_mut(offset..offset + 1, |span, relative_range| {
            span.text.insert_str(relative_range.start, value);
        });
    }

    /// Returns the total length, in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.spans.iter().map(|s| s.text.len()).sum()
    }

    /// Returns true if there are no characters in this text.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterates over all spans within `range` and invokes `callback` with the
    /// span and the applicable range within the span.
    // TODO refactor to allow `RangeBounds`
    pub fn for_each_in_range<F: FnMut(&Span, Range<usize>)>(
        &self,
        range: Range<usize>,
        mut callback: F,
    ) {
        let mut span_start = 0_usize;
        for span in &self.spans {
            let span_len = span.text.len();
            let span_end = span_start + span_len;

            if span_end >= range.start {
                if span_start >= range.end {
                    return;
                }

                let relative_range =
                    (range.start - span_start).max(0)..(range.end - span_start).min(span_len);
                callback(span, relative_range);
            }

            span_start = span_end;
        }
    }

    /// Iterates over all spans within `range` and invokes `callback` with the
    /// span and the applicable range within the span.
    pub fn for_each_in_range_mut<F: FnMut(&mut Span, Range<usize>)>(
        &mut self,
        range: Range<usize>,
        mut callback: F,
    ) {
        let mut span_start = 0_usize;
        for span in &mut self.spans {
            let span_len = span.text.len();
            let span_end = span_start + span_len;

            if span_end >= range.start {
                if span_start >= range.end {
                    break;
                }

                let relative_range = range.start.checked_sub(span_start).unwrap_or_default()
                    ..(range.end.checked_sub(span_start).unwrap_or_default()).min(span_len);
                callback(span, relative_range);
            }

            span_start = span_end;
        }

        self.cleanup_spans();
    }

    fn cleanup_spans(&mut self) {
        if self.is_empty() {
            // If we have no actual text in this, keep the first span and dump the rest
            // Doing this operation separately allows the other branch to be a simple retain operation
            self.spans.resize_with(1, || unreachable!());
        } else {
            self.spans.retain(|span| !span.text.is_empty());
        }
    }
}

impl From<Vec<Span>> for Text {
    fn from(spans: Vec<Span>) -> Self {
        Self { spans }
    }
}

impl Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for span in &self.spans {
            f.write_str(&span.text)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_remove_one_span_partial() {
        let mut text = Text::span("123456789", Style::default());
        text.remove_range(0..1);
        assert_eq!(text.spans.len(), 1);
        assert_eq!(text.spans[0].text, "23456789");
    }

    #[test]
    fn test_remove_one_span_entire() {
        let mut text = Text::span("1", Style::default());
        text.remove_range(0..1);
        assert_eq!(text.spans.len(), 1);
        assert!(text.spans[0].text.is_empty());
    }

    #[test]
    fn test_remove_multi_span_entire_first() {
        let mut text = Text::from(vec![
            Span::new("1", Style::default()),
            Span::new("2", Style::default()),
            Span::new("3", Style::default()),
        ]);
        text.remove_range(0..1);
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0].text, "2");
        assert_eq!(text.spans[1].text, "3");
    }

    #[test]
    fn test_remove_multi_span_entire_middle() {
        let mut text = Text::from(vec![
            Span::new("1", Style::default()),
            Span::new("2", Style::default()),
            Span::new("3", Style::default()),
        ]);
        text.remove_range(1..2);
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0].text, "1");
        assert_eq!(text.spans[1].text, "3");
    }

    #[test]
    fn test_remove_multi_span_entire_last() {
        let mut text = Text::from(vec![
            Span::new("1", Style::default()),
            Span::new("2", Style::default()),
            Span::new("3", Style::default()),
        ]);
        text.remove_range(2..3);
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0].text, "1");
        assert_eq!(text.spans[1].text, "2");
    }

    #[test]
    fn test_remove_multi_span_multi() {
        let mut text = Text::from(vec![
            Span::new("123a", Style::default()),
            Span::new("b", Style::default()),
            Span::new("c456", Style::default()),
        ]);
        text.remove_range(3..6);
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0].text, "123");
        assert_eq!(text.spans[1].text, "456");
    }

    #[test]
    fn test_insert_start() {
        let mut text = Text::span("2", Style::default());
        text.insert_str(0, "1");
        assert_eq!(text.spans.len(), 1);
        assert_eq!(text.spans[0].text, "12");
    }

    #[test]
    fn test_insert_end() {
        let mut text = Text::span("1", Style::default());
        text.insert_str(1, "2");
        assert_eq!(text.spans.len(), 1);
        assert_eq!(text.spans[0].text, "12");
    }
}
