use std::sync::Arc;

use gooey_core::{euclid::Length, styles::Style, Points};
use gooey_renderer::{Renderer, TextMetrics};

use crate::{prepared::PreparedSpan, Text};

#[derive(Debug)]
pub(crate) enum Token {
    EndOfLine(TextMetrics<Points>),
    Characters(PreparedSpan),
    Punctuation(PreparedSpan),
    Whitespace(PreparedSpan),
    NoText(Option<TextMetrics<Points>>),
}

#[derive(Debug)]
pub(crate) enum SpanGroup {
    Spans(Vec<PreparedSpan>),
    Whitespace(Vec<PreparedSpan>),
    EndOfLine(TextMetrics<Points>),
}

impl SpanGroup {
    pub(crate) fn spans(&self) -> Vec<PreparedSpan> {
        match self {
            SpanGroup::Spans(spans) | SpanGroup::Whitespace(spans) => spans.clone(),
            SpanGroup::EndOfLine(_) => Vec::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum TokenizerStatus {
    /// We have wrapped to a new line
    AtSpanStart,
    /// We have received at least one glyph for this word
    InWord,
    /// We have encountered a punctuation mark after a word.
    TrailingPunctuation,
    /// We have encountered a whitespace or punctuation character
    Whitespace,
}

#[derive(Default)]
pub(crate) struct Tokenizer {
    tokens: Vec<Token>,
}

struct TokenizerState {
    style: Arc<Style>,
    text: String,
    lexer_state: TokenizerStatus,
    #[allow(dead_code)]
    caret: Length<f32, Points>,
}

impl TokenizerState {
    pub(crate) fn new(style: Style) -> Self {
        Self {
            style: Arc::new(style),
            lexer_state: TokenizerStatus::AtSpanStart,
            text: String::default(),
            caret: Length::default(),
        }
    }

    fn emit_token_if_needed<R: Renderer>(&mut self, scene: &R) -> Option<Token> {
        if self.text.is_empty() {
            None
        } else {
            let text = self.text.clone();
            self.text.clear();
            let metrics = scene.measure_text(&text, &self.style);
            let span = PreparedSpan::new(self.style.clone(), text, metrics);
            self.caret = Length::default();

            let token = match self.lexer_state {
                TokenizerStatus::AtSpanStart => unreachable!(),
                TokenizerStatus::InWord => Token::Characters(span),
                TokenizerStatus::TrailingPunctuation => Token::Punctuation(span),
                TokenizerStatus::Whitespace => Token::Whitespace(span),
            };
            Some(token)
        }
    }
}

impl Tokenizer {
    // Text (Vec<Span>) -> Vec<Token{ PreparedSpan, TokenKind }>
    pub(crate) fn prepare_spans<R: Renderer>(mut self, text: &Text, scene: &R) -> Vec<Token> {
        let mut last_span_metrics = None;
        for span in &text.spans {
            let vmetrics = scene.measure_text("m", &span.style);
            last_span_metrics = Some(vmetrics);

            let mut state = TokenizerState::new(span.style.clone());

            for c in span.text.chars() {
                if c.is_control() {
                    if c == '\n' {
                        self.tokens.push(Token::EndOfLine(vmetrics));
                    }
                } else {
                    let new_lexer_state = if c.is_whitespace() {
                        TokenizerStatus::Whitespace
                    } else if c.is_ascii_punctuation() {
                        TokenizerStatus::TrailingPunctuation
                    } else {
                        TokenizerStatus::InWord
                    };

                    if new_lexer_state != state.lexer_state {
                        if let Some(token) = state.emit_token_if_needed(scene) {
                            self.tokens.push(token);
                        }
                    }

                    state.lexer_state = new_lexer_state;

                    state.text.push(c);
                }
            }

            if let Some(token) = state.emit_token_if_needed(scene) {
                self.tokens.push(token);
            }
        }

        if self.tokens.is_empty() {
            self.tokens.push(Token::NoText(last_span_metrics));
        }

        self.tokens
    }
}
