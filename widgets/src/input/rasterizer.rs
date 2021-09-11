use std::{
    fmt::Debug,
    marker::PhantomData,
    time::{Duration, Instant},
};

#[cfg(not(target_arch = "wasm32"))]
use arboard::Clipboard;
use gooey_core::{
    figures::{Displayable, Figure, Point, Rect, Rectlike, Size, SizedRect},
    styles::{Color, HighlightColor, Style, TextColor},
    Pixels, Scaled, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{
    winit::event::{ElementState, MouseButton, ScanCode, VirtualKeyCode},
    ContentArea, EventStatus, ModifiersStateExt, Rasterizer, Renderer, TransmogrifierContextExt,
    WidgetRasterizer,
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

    fn initialize(
        &self,
        widget: &mut Self::Widget,
        _reference: &gooey_core::WidgetRef<Self::Widget>,
        _frontend: &Rasterizer<R>,
    ) -> Self::State {
        InputState {
            text: RichText::from(widget.value.as_str()),
            ..InputState::default()
        }
    }

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
            Command::PasswordModeSet => {}
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
        if context.ui_state.focused && context.state.cursor.end.is_none() {
            // Update the blinking cursor.
            if let Some(duration) = context.state.cursor.blink_state.update() {
                context.frontend.schedule_redraw_in(duration);
            }
        }

        if let Some(renderer) = context.frontend.renderer() {
            let scale = renderer.scale();
            let bounds = content_area.content_bounds().as_sized();

            let mut y = Figure::<f32, Scaled>::default();
            let prepared = context.state.prepared_text(
                renderer,
                content_area.size.content,
                context.widget.password_mode(),
            );
            for paragraph in &prepared {
                y += paragraph.render::<TextColor, _>(
                    renderer,
                    Point::new(bounds.origin.x, bounds.origin.y + y.get()),
                    true,
                    Some(context.style()),
                );
            }
            context.state.prepared = Some(prepared);

            let selection_color = context
                .style
                .get_with_fallback::<HighlightColor>()
                .map(|w| w.themed_color(renderer.theme()))
                .unwrap_or_default();

            if let Some(end) = context.state.cursor.end {
                let selection_start = context.state.cursor.start.min(end);
                let selection_end = context.state.cursor.start.max(end);
                if let Some(start_position) = context
                    .state
                    .character_rect_for_position(renderer, selection_start)
                    .map(|r| r.as_extents())
                {
                    if let Some(end_position) = context
                        .state
                        .character_rect_for_position(renderer, selection_end)
                        .map(|r| r.as_extents())
                    {
                        let transparent_selection = selection_color.with_alpha(0.3);
                        if start_position.extent.y <= end_position.origin.y {
                            // Multi-line
                            // First line is start_position -> end of bounds
                            let mut area = start_position.translate(bounds.origin).as_sized();
                            area.size.width = bounds.size.width - start_position.origin.x;
                            // TODO change to a SelectionColor component.
                            renderer.fill_rect(&area.as_rect(), Color::new(1., 0., 0., 0.3));
                            if start_position.extent.y < end_position.origin.y {
                                // Draw a solid block for all the inner lines
                                renderer.fill_rect(
                                    &Rect::sized(
                                        Point::from_y(start_position.extent.y),
                                        Size::from_figures(
                                            bounds.size.width(),
                                            end_position.origin.y() - start_position.extent.y(),
                                        ),
                                    )
                                    .translate(bounds.origin),
                                    transparent_selection,
                                );
                            }
                            // Last line is start of line -> start of end position
                            renderer.fill_rect(
                                &Rect::sized(
                                    Point::from_y(end_position.origin.y),
                                    Size::from_figures(
                                        end_position.origin.x(),
                                        end_position.height(),
                                    ),
                                )
                                .translate(bounds.origin),
                                transparent_selection,
                            );
                        } else {
                            // Single-line
                            let mut area = start_position.as_sized();
                            area.size.width = end_position.origin.x - start_position.origin.x;
                            renderer.fill_rect(
                                &area.translate(bounds.origin).as_rect(),
                                transparent_selection,
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
                        &Rect::sized(
                            bounds.origin + cursor_location.origin,
                            Size::from_figures(
                                Figure::<f32, Pixels>::new(1.).to_scaled(&scale),
                                Figure::new(cursor_location.size.height),
                            ),
                        ),
                        selection_color,
                    );
                }
            }
        }
    }

    fn measure_content(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size<Option<f32>, Scaled>,
    ) -> Size<f32, Scaled> {
        context
            .frontend
            .renderer()
            .map_or_else(Size::default, |renderer| {
                let wrapped = wrap_text(
                    &context.widget.value,
                    context.style(),
                    renderer,
                    Figure::new(constraints.width.unwrap_or_else(|| renderer.size().width)),
                );
                wrapped.size()
            })
    }

    fn mouse_down(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        button: MouseButton,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) -> EventStatus {
        if button == MouseButton::Left {
            context.focus();
            context.state.cursor.blink_state.force_on();

            let bounds = area.content_bounds();

            if let Some(location) =
                InputState::position_for_location(context, location - bounds.origin())
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
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) {
        if button == MouseButton::Left {
            context.state.cursor.blink_state.force_on();
            let bounds = area.content_bounds();
            if let Some(location) =
                InputState::position_for_location(context, location - bounds.origin())
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

    fn receive_character(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        character: char,
    ) -> EventStatus {
        match character {
            '\x08' => {
                if context.state.cursor.end.is_none() && context.state.cursor.start.offset > 0 {
                    // Select the previous character
                    context.state.cursor.end = Some(context.state.cursor.start);
                    context.state.cursor.start = context
                        .state
                        .text
                        .position_before(context.state.cursor.start);
                }
                InputState::replace_selection("", context);
                EventStatus::Processed
            }
            character => {
                if !character.is_control() {
                    InputState::replace_selection(&character.to_string(), context);
                    EventStatus::Processed
                } else {
                    EventStatus::Ignored
                }
            }
        }
    }

    fn keyboard(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        _scancode: ScanCode,
        keycode: Option<VirtualKeyCode>,
        state: ElementState,
    ) -> EventStatus {
        if let Some(key) = keycode {
            if matches!(state, ElementState::Pressed) {
                // TODO handle modifiers
                let handled = match key {
                    VirtualKeyCode::Left => {
                        InputState::set_selection(
                            context,
                            context
                                .state
                                .text
                                .position_before(context.state.cursor.selection_start()),
                            None,
                        );
                        true
                    }
                    VirtualKeyCode::Right => {
                        InputState::set_selection(
                            context,
                            context
                                .state
                                .text
                                .position_after(context.state.cursor.selection_start()),
                            None,
                        );
                        true
                    }
                    VirtualKeyCode::A => {
                        if context.frontend.keyboard_modifiers().primary() {
                            InputState::set_selection(
                                context,
                                RichTextPosition::default(),
                                Some(context.state.text.end()),
                            );
                        }
                        true
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    VirtualKeyCode::V => {
                        if context.frontend.keyboard_modifiers().primary() {
                            InputState::paste(context);
                            true
                        } else {
                            false
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    VirtualKeyCode::X | VirtualKeyCode::C => {
                        if context.frontend.keyboard_modifiers().primary()
                            && InputState::copy(context)
                            && key == VirtualKeyCode::X
                        {
                            InputState::replace_selection("", context);
                            true
                        } else {
                            false
                        }
                    }
                    // VirtualKeyCode::Up | VirtualKeyCode::Down |
                    _ => false,
                };
                if handled {
                    context.state.cursor.blink_state.force_on();
                    context.frontend.set_needs_redraw();
                    return EventStatus::Processed;
                }
            }
        }
        EventStatus::Ignored
    }
}

fn wrap_text<R: Renderer>(
    label: &str,
    style: &Style,
    renderer: &R,
    width: Figure<f32, Scaled>,
) -> PreparedText {
    Text::span(label, style.clone()).wrap(
        renderer,
        TextWrap::MultiLine {
            size: Size::from_figures(width, Figure::new(renderer.size().height)),
        },
        None,
    )
}

pub struct InputState<R> {
    text: RichText,
    cursor: Cursor,
    prepared: Option<Vec<PreparedText>>,
    #[cfg(not(target_arch = "wasm32"))]
    clipboard: Option<Clipboard>,
    _renderer: PhantomData<R>,
}

impl<R: Renderer> Debug for InputState<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputState")
            .field("text", &self.text)
            .field("cursor", &self.cursor)
            .field("prepared", &self.prepared)
            .finish_non_exhaustive()
    }
}

impl<R: Renderer> Default for InputState<R> {
    fn default() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            clipboard: Clipboard::new().ok(),
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
    #[cfg(not(target_arch = "wasm32"))]
    fn paste(context: &mut InputTransmogrifierContext<'_, R>) {
        if let Some(clipboard) = &mut context.state.clipboard {
            // Convert Result to Option to get rid of the Box<dyn Error> before the await
            let pasted = clipboard.get_text().ok();
            if let Some(pasted) = pasted {
                Self::replace_selection(&pasted, context);
            }
        }
    }

    /// copies the selected text to the clipboard.
    ///
    /// Returns whether text was successfully written to the clipboard. If
    /// feature `clipboard` isn't enabled, this function will always return
    /// Ok(false)
    #[cfg(not(target_arch = "wasm32"))]
    fn copy(context: &mut InputTransmogrifierContext<'_, R>) -> bool {
        let selected = context.state.selected_string();
        context
            .state
            .clipboard
            .as_mut()
            .map_or(false, |clipboard| clipboard.set_text(selected).is_ok())
    }

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
                    span_strings.push(
                        span.text()
                            .chars()
                            .skip(relative_range.start)
                            .take(relative_range.end - relative_range.start)
                            .collect::<String>(),
                    );
                });
                copied_paragraphs.push(span_strings.join(""));
            },
        );
        copied_paragraphs.join("\n")
    }

    fn prepared_text(
        &self,
        renderer: &R,
        constraints: Size<f32, Scaled>,
        password_mode: bool,
    ) -> Vec<PreparedText> {
        if password_mode {
            // In password mode we render an obscuring character instead of the expected character.
            let value = self.text.to_string();
            let number_of_chars = value.chars().count();
            let obscured = "\u{2022}".repeat(number_of_chars);
            vec![Text::span(obscured, Style::default()).wrap(
                renderer,
                TextWrap::SingleLine {
                    width: Figure::new(constraints.width),
                },
                None,
            )]
        } else {
            self.text.prepare(
                renderer,
                TextWrap::SingleLine {
                    width: Figure::new(constraints.width),
                },
            )
        }
    }

    fn position_for_location(
        context: &mut InputTransmogrifierContext<'_, R>,
        location: Point<f32, Scaled>,
    ) -> Option<RichTextPosition> {
        if let (Some(prepared), Some(renderer)) =
            (&context.state.prepared, context.frontend.renderer())
        {
            let mut y = Figure::<f32, Scaled>::default();
            for (paragraph_index, paragraph) in prepared.iter().enumerate() {
                for line in &paragraph.lines {
                    let line_bottom = y + line.size().height();
                    if location.y < line_bottom.get() {
                        // Click location was within this line
                        for span in &line.spans {
                            let span_end = span.location() + span.metrics().width;
                            if !span.text().is_empty() && location.x < span_end.get() {
                                // Click was within this span
                                let relative_pixels = location.x() - span.location();
                                let mut partial = String::with_capacity(span.text().len());
                                let mut span_position = Figure::<f32, Scaled>::default();
                                for (index, ch) in span.text().chars().enumerate() {
                                    partial.push(ch);
                                    let width = renderer
                                        .measure_text_with_style(&partial, span.style())
                                        .width;
                                    if relative_pixels <= span_position + width {
                                        return Some(RichTextPosition {
                                            paragraph: paragraph_index,
                                            offset: span.offset() + index,
                                        });
                                    }
                                }
                                span_position += span.metrics().width;

                                return Some(RichTextPosition {
                                    paragraph: paragraph_index,
                                    offset: span.offset() + span.len(),
                                });
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
    ) -> Option<SizedRect<f32, Scaled>> {
        let mut last_location = None;
        if let Some(prepared) = &self.prepared {
            let prepared = prepared.get(position.paragraph)?;
            let mut line_top = Figure::<f32, Scaled>::default();
            for line in &prepared.lines {
                let line_height = line.height();
                for span in &line.spans {
                    if !span.text().is_empty() {
                        let mut last_width = Figure::default();
                        if position.offset >= span.offset()
                            && position.offset < span.offset() + span.len()
                        {
                            let mut measured = String::with_capacity(span.text().len());
                            for (offset, ch) in span.text().chars().enumerate() {
                                measured.push(ch);
                                let new_measurement =
                                    renderer.measure_text_with_style(&measured, span.style());
                                if position.offset <= span.offset() + offset {
                                    return Some(SizedRect::new(
                                        Point::from_figures(last_width + span.location(), line_top),
                                        Size::from_figures(
                                            new_measurement.width - last_width,
                                            line_height,
                                        ),
                                    ));
                                }
                                last_width = new_measurement.width;
                            }
                            unreachable!();
                        }
                        // Set the last location a 0-width rect at the end of the span.
                        last_location = Some(SizedRect::new(
                            Point::from_figures(span.location() + span.metrics().width, line_top),
                            Size::from_height(line_height),
                        ));
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
