use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

use gooey_core::{
    euclid::{Length, Point2D, Rect, Size2D},
    styles::{Color, Style, TextColor},
    Context, Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{
    winit::event::MouseButton, ContentArea, EventStatus, Rasterizer, Renderer,
    TransmogrifierContextExt, WidgetRasterizer,
};
use gooey_text::{
    prepared::PreparedText,
    rich::{RichText, RichTextPosition},
    wrap::TextWrap,
    Text,
};

use crate::input::{Command, Input, InputTransmogrifier};

// TODO implement this as an actual lookup on platforms that support it.
const CURSOR_BLINK_MS: u64 = 500;

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for InputTransmogrifier {
    type State = InputState<R>;
    type Widget = Input;

    fn receive_command(
        &self,
        command: Command,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
    ) {
        match command {
            Command::ValueSet => {
                context.state.text = RichText::from(context.widget.value.as_str());
                context.state.prepared = None;
            }
            Command::SelectionSet => {
                context.state.cursor.start = context
                    .state
                    .text
                    .position_of_character(context.widget.selection_start);
                context.state.cursor.end = context
                    .widget
                    .selection_end
                    .map(|offset| context.state.text.position_of_character(offset));
            }
        }
        context.frontend.set_needs_redraw();
    }
}

impl<R: Renderer> WidgetRasterizer<R> for InputTransmogrifier {
    fn render(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        content_area: &ContentArea,
    ) {
        if let Some(renderer) = context.frontend.renderer() {
            let scale = renderer.scale();
            let bounds = content_area.content_bounds();

            let mut y = Length::default();
            let prepared = context
                .state
                .prepared_text(renderer, content_area.size.content);
            for paragraph in &prepared {
                y += paragraph.render::<TextColor, _>(
                    renderer,
                    Point2D::new(bounds.origin.x, bounds.origin.y + y.get()),
                    true,
                );
            }
            context.state.prepared = Some(prepared);

            if let Some(end) = context.state.cursor.end {
                let selection_start = context.state.cursor.start.min(end);
                let selection_end = context.state.cursor.start.max(end);
                if let Some(start_position) = context
                    .state
                    .character_rect_for_position(renderer, selection_start)
                {
                    if let Some(end_position) = context
                        .state
                        .character_rect_for_position(renderer, selection_end)
                    {
                        if start_position.max_y() <= end_position.min_y() {
                            // Multi-line
                            // First line is start_position -> end of bounds
                            let mut area = start_position.translate(bounds.origin.to_vector());
                            area.size.width = bounds.size.width - start_position.origin.x;
                            // TODO change to a SelectionColor component.
                            renderer.fill_rect(&area, Color::new(1., 0., 0., 0.3));
                            if start_position.max_y() < end_position.min_y() {
                                // Draw a solid block for all the inner lines
                                renderer.fill_rect(
                                    &Rect::new(
                                        Point2D::new(0., start_position.max_y()),
                                        Size2D::new(
                                            bounds.size.width,
                                            end_position.min_y() - start_position.max_y(),
                                        ),
                                    )
                                    .translate(bounds.origin.to_vector()),
                                    Color::new(1., 0., 0., 0.3),
                                );
                            }
                            // Last line is start of line -> start of end position
                            renderer.fill_rect(
                                &Rect::new(
                                    Point2D::new(0., end_position.min_y()),
                                    Size2D::new(end_position.origin.x, end_position.size.height),
                                )
                                .translate(bounds.origin.to_vector()),
                                Color::new(1., 0., 0., 0.3),
                            );
                        } else {
                            // Single-line
                            let mut area = start_position;
                            area.size.width = end_position.origin.x - start_position.origin.x;
                            renderer.fill_rect(
                                &area.translate(bounds.origin.to_vector()),
                                Color::new(1., 0., 0., 0.3),
                            );
                        }
                    }
                }
            } else if context.is_focused() && context.state.cursor.blink_state.visible {
                if let Some(cursor_location) = context
                    .state
                    .character_rect_for_position(renderer, context.state.cursor.start)
                {
                    // No selection, draw a caret
                    renderer.fill_rect(
                        &Rect::new(
                            bounds.origin,
                            Size2D::from_lengths(
                                Length::new(1.) / scale,
                                Length::new(cursor_location.size.height),
                            ),
                        ),
                        Color::RED,
                    );
                }
            }
        }
    }

    fn measure_content(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        context
            .frontend
            .renderer()
            .map_or_else(Size2D::default, |renderer| {
                let wrapped = wrap_text(
                    &context.widget.value,
                    context.style,
                    renderer,
                    Length::new(constraints.width.unwrap_or_else(|| renderer.size().width)),
                );
                wrapped.size()
            })
    }

    fn mouse_down(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        button: MouseButton,
        location: Point2D<f32, Points>,
        area: &ContentArea,
    ) -> EventStatus {
        if button == MouseButton::Left {
            context.focus();
            context.state.cursor.blink_state.force_on();

            let bounds = area.content_bounds();

            if let Some(location) =
                InputState::position_for_location(context, location - bounds.origin.to_vector())
            {
                context.state.cursor.start = location;
                context.state.cursor.end = None;
            }

            context.frontend.set_needs_redraw();

            EventStatus::Processed
        } else {
            EventStatus::Ignored
        }
    }

    fn mouse_drag(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        button: MouseButton,
        location: Point2D<f32, Points>,
        area: &ContentArea,
    ) {
        if button == MouseButton::Left {
            context.state.cursor.blink_state.force_on();
            let bounds = area.content_bounds();
            if let Some(location) =
                InputState::position_for_location(context, location - bounds.origin.to_vector())
            {
                if location == context.state.cursor.start {
                    if context.state.cursor.end != None {
                        InputState::set_selection(context, context.state.cursor.start, None);
                        context.frontend.set_needs_redraw();
                    }
                } else if context.state.cursor.end != Some(location) {
                    context.state.cursor.end = Some(location);
                    InputState::set_selection(context, context.state.cursor.start, Some(location));
                    context.frontend.set_needs_redraw();
                }
            }
        }
    }
}

fn wrap_text<R: Renderer>(
    label: &str,
    style: &Style,
    renderer: &R,
    width: Length<f32, Points>,
) -> PreparedText {
    Text::span(label, style.clone()).wrap(
        renderer,
        TextWrap::MultiLine {
            size: Size2D::from_lengths(width, Length::new(renderer.size().height)),
        },
        None,
    )
}

#[derive(Debug)]
pub struct InputState<R> {
    text: RichText,
    cursor: Cursor,
    prepared: Option<Vec<PreparedText>>,
    _renderer: PhantomData<R>,
}

impl<R: Renderer> Default for InputState<R> {
    fn default() -> Self {
        Self {
            text: RichText::default(),
            cursor: Cursor::default(),
            prepared: None,
            _renderer: PhantomData::default(),
        }
    }
}

type InputTransmogrifierContext<'a, R> =
    TransmogrifierContext<'a, InputTransmogrifier, Rasterizer<R>>;

impl<R: Renderer> InputState<R> {
    // /// pastes text from the clipboard into the field
    // ///
    // /// If feature `clipboard` isn't enabled, this function will return Ok(()).
    // #[allow(unused_variables)] // when clipboard is disabled, `context` is unused
    // async fn paste(&mut self, context: &mut Context) -> KludgineResult<()> {
    //     #[cfg(feature = "clipboard")]
    //     {
    //         let mut clipboard = arboard::Clipboard::new()?;

    //         // Convert Result to Option to get rid of the Box<dyn Error> before the await
    //         let pasted = clipboard.get_text().ok();
    //         if let Some(pasted) = pasted {
    //             self.replace_selection(&pasted, context);
    //         }
    //     }

    //     Ok(())
    // }

    // /// copies the selected text to the clipboard.
    // ///
    // /// Returns whether text was successfully written to the clipboard. If
    // /// feature `clipboard` isn't enabled, this function will always return
    // /// Ok(false)
    // async fn copy(&mut self) -> KludgineResult<bool> {
    //     #[cfg(feature = "clipboard")]
    //     {
    //         let mut clipboard = arboard::Clipboard::new()?;

    //         let selected = self.selected_string();

    //         clipboard.set_text(selected)?;
    //     }

    //     return Ok(true);
    // }

    pub fn replace_selection(replacement: &str, context: &mut InputTransmogrifierContext<'_, R>) {
        if context.state.cursor.end.is_some() {
            let selection_start = context.state.cursor.selection_start();
            let selection_end = context.state.cursor.selection_end();
            context
                .state
                .text
                .remove_range(selection_start..selection_end);
            context.state.cursor.end = None;
            context.state.cursor.start = selection_start;
        }

        context
            .state
            .text
            .insert_str(context.state.cursor.start, replacement);
        context.state.cursor.start.offset += replacement.len();
        context.state.cursor.blink_state.force_on();

        Self::notify_changed(context);
        Self::notify_selection_changed(context);
        context.frontend.set_needs_redraw();
    }

    pub fn selected_string(&self) -> String {
        let mut copied_paragraphs = Vec::new();
        self.text.for_each_in_range(
            self.cursor.selection_start()..self.cursor.selection_end(),
            |paragraph, relative_range| {
                let mut span_strings = Vec::new();
                paragraph.for_each_in_range(relative_range, |span, relative_range| {
                    span_strings.push(span.text()[relative_range].to_string());
                });
                copied_paragraphs.push(span_strings.join(""));
            },
        );
        copied_paragraphs.join("\n")
    }

    fn prepared_text(&self, renderer: &R, constraints: Size2D<f32, Points>) -> Vec<PreparedText> {
        self.text.prepare(
            renderer,
            TextWrap::SingleLine {
                width: Length::new(constraints.width),
            },
        )
    }

    fn position_for_location(
        context: &mut InputTransmogrifierContext<'_, R>,
        location: Point2D<f32, Points>,
    ) -> Option<RichTextPosition> {
        if let Some(prepared) = &context.state.prepared {
            let mut y = Length::<f32, Points>::default();
            let scale = context
                .frontend
                .renderer()
                .map(|r| r.scale())
                .unwrap_or_default();
            for (paragraph_index, paragraph) in prepared.iter().enumerate() {
                for line in &paragraph.lines {
                    let line_bottom = y + Length::new(line.size().height) / scale;
                    if location.y < line_bottom.get() {
                        // Click location was within this line
                        for span in &line.spans {
                            let span_end = span.location() + span.metrics().width;
                            if !span.text().is_empty() && location.x < span_end.get() {
                                // Click was within this span
                                todo!()
                                // let relative_pixels = (location.x() - x) * scale;
                                // for info in span.data.glyphs.iter() {
                                //     if relative_pixels <= info.location().x() + info.width() {
                                //         return Some(RichTextPosition {
                                //             paragraph: paragraph_index,
                                //             offset: info.source_offset,
                                //         });
                                //     }
                                // }

                                // return Some(RichTextPosition {
                                //     paragraph: paragraph_index,
                                //     offset: span.data.glyphs.last().unwrap().source_offset,
                                // });
                            }
                        }
                        // Didn't match a span, return the last character of the line
                        if let Some(span) = line.spans.last() {
                            // Didn't match within the span, put it at the end of the span
                            return Some(RichTextPosition {
                                paragraph: paragraph_index,
                                offset: span.offset() + span.text().chars().count(),
                            });
                        }
                    }

                    y = line_bottom;
                }
            }
        }

        None
    }

    fn character_rect_for_position(
        &self,
        renderer: &R,
        position: RichTextPosition,
    ) -> Option<Rect<f32, Points>> {
        let mut last_location = None;
        if let Some(prepared) = &self.prepared {
            let prepared = prepared.get(position.paragraph)?;
            let mut line_top = Length::default();
            for line in &prepared.lines {
                let line_height = line.height();
                for span in &line.spans {
                    if !span.text().is_empty() {
                        // TODO measure this subset of text.
                        // let last_glyph = span.data.glyphs.last().unwrap();
                        // if position.offset <= last_glyph.source_offset {
                        //     // Return a box of the width of the last character with the start of the character at the origin
                        //     for info in span.data.glyphs.iter() {
                        //         if info.source_offset >= position.offset {
                        //             return Some(Rect::new(
                        //                 Point::from_lengths(
                        //                     (span.location.x() + info.location().x()) / scale,
                        //                     line_top + span.location.y() / scale,
                        //                 ),
                        //                 Size::from_lengths(info.width() / scale, line_height),
                        //             ));
                        //         }
                        //     }
                        // }
                        // last_location = Some(Rect::new(
                        //     Point2D::from_lengths(
                        //         (span.location()
                        //             + last_glyph.location().x()
                        //             + last_glyph.width())
                        //             / scale,
                        //         line_top + span.location.y() / scale,
                        //     ),
                        //     Size::from_lengths(Default::default(), line_height),
                        // ));
                    }
                }
                line_top += line_height;
            }
        }
        last_location
    }

    fn notify_changed(context: &mut InputTransmogrifierContext<'_, R>) {
        context.widget.value = context.state.text.to_string();
        context.widget.changed.invoke(());
    }

    fn notify_selection_changed(context: &mut InputTransmogrifierContext<'_, R>) {
        let selection_start = context
            .state
            .text
            .character_offset_of(context.state.cursor.selection_start());
        context.widget.selection_start = selection_start;
        let selection_end = context
            .state
            .text
            .character_offset_of(context.state.cursor.selection_end());
        if selection_start == selection_end {
            context.widget.selection_end = None;
        } else {
            context.widget.selection_end = Some(selection_end);
        }
        context.widget.selection_changed.invoke(());
    }

    pub fn set_selection(
        context: &mut InputTransmogrifierContext<'_, R>,
        selection_start: RichTextPosition,
        end: Option<RichTextPosition>,
    ) {
        context.state.cursor.start = selection_start;
        context.state.cursor.end = end;
        Self::notify_selection_changed(context);
    }
}

#[derive(Debug, Default)]
pub struct Cursor {
    pub blink_state: BlinkState,
    pub start: RichTextPosition,
    pub end: Option<RichTextPosition>,
}

impl Cursor {
    pub fn selection_start(&self) -> RichTextPosition {
        self.end.map_or(self.start, |end| self.start.min(end))
    }

    pub fn selection_end(&self) -> RichTextPosition {
        self.end.map_or(self.start, |end| self.start.max(end))
    }
}

#[derive(Debug, Clone)]
pub struct BlinkState {
    pub visible: bool,
    pub change_at: Instant,
}

impl Default for BlinkState {
    fn default() -> Self {
        Self {
            visible: true,
            change_at: Self::next_blink(),
        }
    }
}

impl BlinkState {
    pub fn next_blink() -> Instant {
        let now = Instant::now();
        now.checked_add(Duration::from_millis(CURSOR_BLINK_MS))
            .unwrap_or(now)
    }

    pub fn force_on(&mut self) {
        self.visible = true;
        self.change_at = Self::next_blink();
    }

    pub fn update(&mut self) -> Option<Duration> {
        let now = Instant::now();
        if self.change_at < now {
            self.visible = !self.visible;
            self.change_at = Self::next_blink();
        }

        self.change_at.checked_duration_since(now)
    }
}
