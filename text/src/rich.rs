use gooey_renderer::Renderer;

use crate::{prepared::PreparedText, wrap::TextWrap, Text};
use std::{
    cmp::Ordering,
    ops::Range,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Default)]
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

pub enum ParagraphRemoval {
    Remove,
    Keep,
}

impl RichText {
    pub fn new(paragraphs: Vec<Text>) -> Self {
        Self {
            data: Arc::new(Mutex::new(RichTextData { paragraphs })),
        }
    }

    pub async fn remove_range(&self, range: Range<RichTextPosition>) {
        assert!(range.start <= range.end);

        self.for_each_in_range_mut(range.clone(), |text, text_range, paragraph_index| {
            text.remove_range(text_range);
            if paragraph_index != range.start.paragraph && paragraph_index != range.end.paragraph {
                ParagraphRemoval::Remove
            } else {
                ParagraphRemoval::Keep
            }
        })
        .await;

        // If the range spanned paragraphs, the inner paragraphs will be removed but we need to
        // merge the first and last paragraphs
        if range.start.paragraph != range.end.paragraph {
            let mut data = self.data.write().await;
            let mut paragraph_to_merge = data.paragraphs.remove(range.start.paragraph + 1);
            data.paragraphs[range.start.paragraph]
                .spans
                .append(&mut paragraph_to_merge.spans);
        }
    }

    pub async fn insert_str(&self, location: RichTextPosition, value: &str) {
        self.for_each_in_range_mut(location..location, |text, text_range, _| {
            text.insert_str(text_range.start, value);

            ParagraphRemoval::Keep
        })
        .await;
    }

    pub async fn for_each_in_range<F: FnMut(&Text, Range<usize>)>(
        &self,
        range: Range<RichTextPosition>,
        mut callback: F,
    ) {
        let data = self.data.read().await;
        for paragraph_index in range.start.paragraph..(range.end.paragraph + 1) {
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

                callback(paragraph, start..end)
            }
        }
    }

    pub async fn for_each_in_range_mut<
        F: Fn(&mut Text, Range<usize>, usize) -> ParagraphRemoval,
    >(
        &self,
        range: Range<RichTextPosition>,
        callback: F,
    ) {
        let mut data = self.data.write().await;
        let mut paragraphs_to_remove = Vec::new();

        for paragraph_index in range.start.paragraph..(range.end.paragraph + 1) {
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
            data.paragraphs.remove(paragraph_index);
        }
    }

    pub async fn prepare<R: Renderer>(&self, context: &R, wrapping: TextWrap) -> Vec<PreparedText> {
        let data = self.data.read().await;
        let mut prepared = Vec::new();
        for paragraph in data.paragraphs.iter() {
            prepared.push(paragraph.wrap(context.scene(), wrapping.clone()).await?);
        }
        Ok(prepared)
    }

    pub async fn position_after(&self, mut position: RichTextPosition) -> RichTextPosition {
        let data = self.data.read().await;
        let next_offset = position.offset + 1;
        if next_offset > data.paragraphs[position.paragraph].len() {
            if data.paragraphs.len() > position.paragraph + 1 {
                todo!("Need to support multiple paragraphs")
            }
        } else {
            position.offset = next_offset;
        }
        position
    }

    pub async fn position_before(&self, mut position: RichTextPosition) -> RichTextPosition {
        if position.offset == 0 {
            if position.paragraph > 0 {
                todo!("Need to support multiple paragraphs")
            }
        } else {
            position.offset -= 1;
        }

        position
    }

    pub async fn end(&self) -> RichTextPosition {
        let data = self.data.read().await;
        RichTextPosition {
            paragraph: data.paragraphs.len() - 1,
            offset: data.paragraphs.last().unwrap().len(),
        }
    }

    pub async fn to_string(&self) -> String {
        let data = self.data.read().await;
        let mut paragraphs = Vec::with_capacity(data.paragraphs.len());
        for paragraph in data.paragraphs.iter() {
            paragraphs.push(paragraph.to_string());
        }

        paragraphs.join("\n")
    }

    pub async fn paragraphs(&self) -> Vec<Text> {
        let data = self.data.read().await;
        data.paragraphs.clone()
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct RichTextPosition {
    pub paragraph: usize,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_range_one_paragraph_start() {
        let text = RichText::new(vec![Text::span("a123", Default::default())]);
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
        let text = RichText::new(vec![Text::span("123a", Default::default())]);
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
        let text = RichText::new(vec![Text::span("1a23", Default::default())]);
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
            Text::span("123a", Default::default()),
            Text::span("b456", Default::default()),
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
            Text::span("123a", Default::default()),
            Text::span("bc", Default::default()),
            Text::span("d456", Default::default()),
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
