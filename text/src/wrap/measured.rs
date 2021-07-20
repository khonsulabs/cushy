use gooey_core::Points;
use gooey_renderer::{Renderer, TextMetrics};

use super::{ParserStatus, SpanGroup, Token, Tokenizer};
use crate::{prepared::PreparedSpan, Text};

#[derive(Debug)]
pub struct MeasuredText {
    pub(crate) info: MeasuredTextInfo,
}

#[derive(Debug)]
pub(crate) enum MeasuredTextInfo {
    Groups(Vec<SpanGroup>),
    NoText(TextMetrics<Points>),
}

struct TextMeasureState {
    current_group: Option<SpanGroup>,
    status: ParserStatus,
    groups: Vec<SpanGroup>,
    no_text_metrics: Option<TextMetrics<Points>>,
}

impl TextMeasureState {
    fn push_token(&mut self, token: Token) {
        match token {
            Token::EndOfLine(vmetrics) => {
                self.commit_current_group();
                self.groups.push(SpanGroup::EndOfLine(vmetrics));
                self.status = ParserStatus::LineStart;
            }
            Token::NoText(vmetrics) => {
                self.no_text_metrics = vmetrics;
            }
            Token::Characters(span) => {
                match self.status {
                    ParserStatus::LineStart | ParserStatus::InWord => {
                        self.push_visual_span(span);
                        self.status = ParserStatus::InWord;
                    }

                    ParserStatus::Whitespace | ParserStatus::TrailingPunctuation => {
                        self.commit_current_group();
                        self.push_visual_span(span);
                        self.status = ParserStatus::TrailingPunctuation;
                    }
                };
            }
            Token::Punctuation(span) => match self.status {
                ParserStatus::TrailingPunctuation => {
                    self.push_visual_span(span);
                }
                ParserStatus::LineStart | ParserStatus::InWord => {
                    self.push_visual_span(span);
                    self.status = ParserStatus::TrailingPunctuation;
                }
                ParserStatus::Whitespace => {
                    self.commit_current_group();
                    self.push_visual_span(span);
                    self.status = ParserStatus::TrailingPunctuation;
                }
            },
            Token::Whitespace(span) => match self.status {
                ParserStatus::Whitespace => {
                    self.push_whitespace_span(span);
                }
                _ => {
                    self.commit_current_group();
                    self.push_whitespace_span(span);
                    self.status = ParserStatus::Whitespace;
                }
            },
        }
    }

    fn push_visual_span(&mut self, span: PreparedSpan) {
        if let Some(SpanGroup::Spans(group)) = &mut self.current_group {
            group.push(span);
        } else {
            self.commit_current_group();
            self.current_group = Some(SpanGroup::Spans(vec![span]));
        }
    }

    fn push_whitespace_span(&mut self, span: PreparedSpan) {
        if let Some(SpanGroup::Whitespace(group)) = &mut self.current_group {
            group.push(span);
        } else {
            self.commit_current_group();
            self.current_group = Some(SpanGroup::Whitespace(vec![span]));
        }
    }

    fn commit_current_group(&mut self) {
        if let Some(group) = self.current_group.take() {
            self.groups.push(group);
        }
    }

    fn finish(mut self) -> Vec<SpanGroup> {
        self.commit_current_group();
        self.groups
    }
}

impl MeasuredText {
    pub fn new<R: Renderer>(text: &Text, scene: &R) -> Self {
        let mut state = TextMeasureState {
            no_text_metrics: None,
            current_group: None,
            status: ParserStatus::LineStart,
            groups: Vec::new(),
        };

        // Tokens -> "Words" (groups of characters, and where the breaks would happen)
        for token in Tokenizer::default().prepare_spans(text, scene) {
            state.push_token(token);
        }

        let info = if let Some(no_text_metrics) = state.no_text_metrics {
            MeasuredTextInfo::NoText(no_text_metrics)
        } else {
            MeasuredTextInfo::Groups(state.finish())
        };

        MeasuredText { info }
    }
}
