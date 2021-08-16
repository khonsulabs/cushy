use std::{cmp::Ordering, fmt::Display, ops::Range, sync::Arc};

use gooey_core::styles::Style;
use gooey_renderer::Renderer;
use parking_lot::Mutex;

use crate::{prepared::PreparedText, wrap::TextWrap, Text};

/// A multi-paragraph rich text data type.
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct RichText {
    data: Arc<Mutex<RichTextData>>,
}

#[derive(Debug)]
struct RichTextData {
    paragraphs: Vec<Text>,
}

impl Default for RichTextData {
    fn default() -> Self {
        Self {
            paragraphs: vec![Text::default()],
        }
    }
}

/// Indicates whether a paragraph should be kept or removed.
pub enum ParagraphRemoval {
    /// The paragraph should be removed.
    Remove,
    /// The paragraph should be kept.
    Keep,
}

impl RichText {
    /// Creates a new instance with `paragraphs`.
    pub fn new(paragraphs: Vec<Text>) -> Self {
        Self {
            data: Arc::new(Mutex::new(RichTextData { paragraphs })),
        }
    }

    /// Removes a range of characters.
    ///
    /// # Panics
    ///
    /// Panics if `range.end` is less than `range.start`.
    pub fn remove_range(&self, range: Range<RichTextPosition>) {
        assert!(range.start < range.end);

        if range.start != range.end {
            self.for_each_in_range_mut(range.clone(), |text, text_range, paragraph_index| {
                text.remove_range(text_range);
                if paragraph_index != range.start.paragraph
                    && paragraph_index != range.end.paragraph
                {
                    ParagraphRemoval::Remove
                } else {
                    ParagraphRemoval::Keep
                }
            });

            // If the range spanned paragraphs, the inner paragraphs will be removed but we need to
            // merge the first and last paragraphs
            if range.start.paragraph != range.end.paragraph {
                let mut data = self.data.lock();
                let mut paragraph_to_merge = data.paragraphs.remove(range.start.paragraph + 1);
                data.paragraphs[range.start.paragraph]
                    .spans
                    .append(&mut paragraph_to_merge.spans);
            }
        }
    }

    /// Inserts `value` at `location`, preserving the style at the location.
    pub fn insert_str(&self, location: RichTextPosition, value: &str) {
        self.for_each_in_range_mut(location..location, |text, text_range, _| {
            text.insert_str(text_range.start, value);

            ParagraphRemoval::Keep
        });
    }

    /// Interates over each paragraph calling `callback` with the paragraph and
    /// the local character range.
    pub fn for_each_in_range<F: FnMut(&Text, Range<usize>)>(
        &self,
        range: Range<RichTextPosition>,
        mut callback: F,
    ) {
        let data = self.data.lock();
        for paragraph_index in range.start.paragraph..=range.end.paragraph {
            if let Some(paragraph) = data.paragraphs.get(paragraph_index) {
                let start = if range.start.paragraph == paragraph_index {
                    range.start.offset
                } else {
                    0
                };
                let end = if range.end.paragraph == paragraph_index {
                    paragraph.len().min(range.end.offset)
                } else {
                    paragraph.len()
                };

                callback(paragraph, start..end);
            }
        }
    }

    /// Interates over each paragraph calling `callback` with a mutable
    /// reference to the paragraph and the local character range.
    pub fn for_each_in_range_mut<F: Fn(&mut Text, Range<usize>, usize) -> ParagraphRemoval>(
        &self,
        range: Range<RichTextPosition>,
        callback: F,
    ) {
        let mut data = self.data.lock();
        let mut paragraphs_to_remove = Vec::new();

        for paragraph_index in range.start.paragraph..=range.end.paragraph {
            if let Some(paragraph) = data.paragraphs.get_mut(paragraph_index) {
                let start = if range.start.paragraph == paragraph_index {
                    range.start.offset
                } else {
                    0
                };
                let end = if range.end.paragraph == paragraph_index {
                    paragraph.len().min(range.end.offset)
                } else {
                    paragraph.len()
                };

                if matches!(
                    callback(paragraph, start..end, paragraph_index),
                    ParagraphRemoval::Remove
                ) {
                    paragraphs_to_remove.push(paragraph_index);
                }
            }
        }

        // Remove in reverse order to ensure that indexes don't change while removing
        paragraphs_to_remove.reverse();
        for paragraph_index in paragraphs_to_remove {
            drop(data.paragraphs.remove(paragraph_index));
        }
    }

    /// Prepares the rich text for rendering.
    pub fn prepare<R: Renderer>(&self, renderer: &R, wrapping: TextWrap) -> Vec<PreparedText> {
        let data = self.data.lock();
        let mut prepared = Vec::new();
        for paragraph in &data.paragraphs {
            prepared.push(paragraph.wrap(renderer, wrapping, None));
        }
        prepared
    }

    /// Returns the next [`RichTextPosition`] after `position`. Returns the
    /// passed in location if it's at the end of this text.
    pub fn position_after(&self, mut position: RichTextPosition) -> RichTextPosition {
        let data = self.data.lock();
        let next_offset = position.offset + 1;
        if next_offset > data.paragraphs[position.paragraph].len() {
            let next_paragraph = position.paragraph + 1;
            if next_paragraph < data.paragraphs.len() {
                position.paragraph = next_paragraph;
                position.offset = 0;
            }
        } else {
            position.offset = next_offset;
        }
        position
    }

    /// Returns the previous [`RichTextPosition`] before position. Returns the
    /// first position if it's at the beginning of this text.
    pub fn position_before(&self, mut position: RichTextPosition) -> RichTextPosition {
        if position.offset == 0 {
            if position.paragraph > 0 {
                let data = self.data.lock();
                position.paragraph -= 1;
                position.offset = data.paragraphs[position.paragraph].len();
            }
        } else {
            position.offset -= 1;
        }

        position
    }

    /// Returns the last valid location in this text.
    #[allow(clippy::missing_panics_doc)]
    pub fn end(&self) -> RichTextPosition {
        let data = self.data.lock();
        RichTextPosition {
            paragraph: data.paragraphs.len() - 1,
            // This data structure guarantees at least one paragraph.
            offset: data.paragraphs.last().unwrap().len(),
        }
    }

    /// Returns a clone of the paragraphs contained in this text. To access the
    /// data without a copy, use [`for_each_in_range`] or
    /// [`for_each_in_range_mut`].
    #[must_use]
    pub fn paragraphs(&self) -> Vec<Text> {
        let data = self.data.lock();
        data.paragraphs.clone()
    }

    /// Returns the character offset of `position` within this text. If the
    /// position is beyond the bounds of the text, the last character position
    /// is returned.
    #[must_use]
    pub fn character_offset_of(&self, position: RichTextPosition) -> usize {
        let data = self.data.lock();
        let mut offset = 0;
        for (index, paragraph) in data.paragraphs.iter().enumerate() {
            if index == position.paragraph {
                offset += position.offset;
                break;
            }

            // Add 1 for the end of line.
            offset += paragraph.len() + 1;
        }

        offset
    }

    /// Returns the position of the character `offset` into this text. If the
    /// character offset is beyond the bounds of this text, the last position is
    /// returned.
    pub fn position_of_character(&self, mut offset: usize) -> RichTextPosition {
        let data = self.data.lock();
        let mut last_paragraph_len = 0;
        for (index, paragraph) in data.paragraphs.iter().enumerate() {
            let paragraph_len = paragraph.len();
            if paragraph_len >= offset {
                return RichTextPosition {
                    paragraph: index,
                    offset,
                };
            }

            // Add 1 for the end of line.
            offset -= paragraph_len + 1;
            last_paragraph_len = paragraph_len;
        }

        RichTextPosition {
            paragraph: data.paragraphs.len() - 1,
            offset: last_paragraph_len,
        }
    }
}

impl Display for RichText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.data.lock();
        for paragraph in &data.paragraphs {
            <Text as Display>::fmt(paragraph, f)?;
        }

        Ok(())
    }
}

/// A location within a [`RichText`] object.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
#[must_use]
pub struct RichTextPosition {
    /// The paragraph index.
    pub paragraph: usize,
    /// The character offset within the paragraph.
    pub offset: usize,
}

impl PartialOrd for RichTextPosition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RichTextPosition {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.paragraph.cmp(&other.paragraph) {
            Ordering::Equal => self.offset.cmp(&other.offset),
            not_equal => not_equal,
        }
    }
}

impl<'a> From<&'a str> for RichText {
    fn from(text: &str) -> Self {
        // TODO handle \r and \r\n. This is doable with Pattern but it's not straightforward because there's no impl of pattern for a callback with a str.
        Self::new(
            text.split('\n')
                .map(|s| Text::span(s, Style::default()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use gooey_core::styles::Style;

    use super::*;

    #[test]
    fn remove_range_one_paragraph_start() {
        let text = RichText::new(vec![Text::span("a123", Style::default())]);
        text.remove_range(
            RichTextPosition {
                offset: 0,
                paragraph: 0,
            }..RichTextPosition {
                offset: 1,
                paragraph: 0,
            },
        );
        assert_eq!(text.to_string(), "123");
    }

    #[test]
    fn remove_range_one_paragraph_end() {
        let text = RichText::new(vec![Text::span("123a", Style::default())]);
        text.remove_range(
            RichTextPosition {
                offset: 3,
                paragraph: 0,
            }..RichTextPosition {
                offset: 4,
                paragraph: 0,
            },
        );
        assert_eq!(text.to_string(), "123");
    }

    #[test]
    fn remove_range_one_paragraph_inner() {
        let text = RichText::new(vec![Text::span("1a23", Style::default())]);
        text.remove_range(
            RichTextPosition {
                offset: 1,
                paragraph: 0,
            }..RichTextPosition {
                offset: 2,
                paragraph: 0,
            },
        );
        assert_eq!(text.to_string(), "123");
    }

    #[test]
    fn remove_range_multi_paragraph_cross_boundaries() {
        let text = RichText::new(vec![
            Text::span("123a", Style::default()),
            Text::span("b456", Style::default()),
        ]);
        text.remove_range(
            RichTextPosition {
                offset: 3,
                paragraph: 0,
            }..RichTextPosition {
                offset: 1,
                paragraph: 1,
            },
        );
        assert_eq!(text.to_string(), "123456");
    }

    #[test]
    fn remove_range_multi_paragraph_cross_multiple_boundaries() {
        let text = RichText::new(vec![
            Text::span("123a", Style::default()),
            Text::span("bc", Style::default()),
            Text::span("d456", Style::default()),
        ]);
        text.remove_range(
            RichTextPosition {
                offset: 3,
                paragraph: 0,
            }..RichTextPosition {
                offset: 1,
                paragraph: 2,
            },
        );
        assert_eq!(text.to_string(), "123456");
    }
}
