use std::cmp::Ordering;
use std::fmt::Debug;
use std::panic::UnwindSafe;
use std::time::Duration;

use kludgine::app::winit::event::{ElementState, Ime, KeyEvent};
use kludgine::app::winit::keyboard::Key;
use kludgine::cosmic_text::{Action, Attrs, Buffer, Cursor, Edit, Editor, Metrics, Shaping};
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{
    FloatConversion, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size,
};
use kludgine::shapes::Shape;
use kludgine::text::TextOrigin;
use kludgine::{Color, Kludgine};

use crate::context::{EventContext, LayoutContext, WidgetContext};
use crate::styles::components::{HighlightColor, LineHeight, TextColor, TextSize};
use crate::styles::Styles;
use crate::utils::ModifiersExt;
use crate::value::{Generation, IntoValue, Value};
use crate::widget::{Callback, EventHandling, Widget, HANDLED, IGNORED};
use crate::ConstraintLimit;

const CURSOR_BLINK_DURATION: Duration = Duration::from_millis(500);

/// A text input widget.
#[must_use]
pub struct Input {
    /// The value of this widget.
    pub text: Value<String>,
    on_key: Option<Callback<KeyEvent, EventHandling>>,
    editor: Option<LiveEditor>,
    cursor_state: CursorState,
}

impl Input {
    /// Returns an empty widget.
    pub fn empty() -> Self {
        Self::new(String::new())
    }

    /// Returns a new widget containing `initial_text`.
    pub fn new(initial_text: impl IntoValue<String>) -> Self {
        Self {
            text: initial_text.into_value(),
            editor: None,
            cursor_state: CursorState::default(),
            on_key: None,
        }
    }

    /// Sets the `on_key` callback.
    ///
    /// This function is called for every keyboard input event. If [`HANDLED`]
    /// is returned, this widget will ignore the event.
    pub fn on_key<F>(mut self, on_key: F) -> Self
    where
        F: FnMut(KeyEvent) -> EventHandling + Send + UnwindSafe + 'static,
    {
        self.on_key = Some(Callback::new(on_key));
        self
    }

    fn editor_mut(
        &mut self,
        kludgine: &mut Kludgine,
        styles: &Styles,
        context: &WidgetContext<'_, '_>,
    ) -> &mut Editor {
        match (&self.editor, self.text.generation()) {
            (Some(editor), generation) if editor.generation == generation => {}
            (_, generation) => {
                let scale = kludgine.scale();
                let mut buffer = Buffer::new(
                    kludgine.font_system(),
                    Metrics::new(
                        styles.get(&TextSize, context).into_px(scale).into_float(),
                        styles.get(&LineHeight, context).into_px(scale).into_float(),
                    ),
                );
                self.text.map(|text| {
                    buffer.set_text(
                        kludgine.font_system(),
                        text,
                        Attrs::new(),
                        Shaping::Advanced,
                    );
                });
                self.editor = Some(LiveEditor {
                    editor: Editor::new(buffer),
                    generation,
                });
            }
        }

        &mut self.editor.as_mut().expect("just initialized").editor
    }

    fn styles(context: &WidgetContext<'_, '_>) -> Styles {
        context.query_styles(&[&TextColor, &TextSize, &LineHeight])
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Input")
            .field("text", &self.text)
            .finish_non_exhaustive()
    }
}

impl Widget for Input {
    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn accept_focus(&mut self, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: kludgine::app::winit::event::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        context.focus();
        let styles = context.query_styles(&[&TextColor]);
        self.editor_mut(context.kludgine, &styles, &context.widget)
            .action(
                context.kludgine.font_system(),
                Action::Click {
                    x: location.x.0,
                    y: location.y.0,
                },
            );
        context.set_needs_redraw();
        HANDLED
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: kludgine::app::winit::event::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_, '_>,
    ) {
        let styles = context.query_styles(&[&TextColor]);
        self.editor_mut(context.kludgine, &styles, &context.widget)
            .action(
                context.kludgine.font_system(),
                Action::Drag {
                    x: location.x.0,
                    y: location.y.0,
                },
            );
        self.cursor_state.force_on();
        context.set_needs_redraw();
    }

    #[allow(clippy::too_many_lines)]
    fn redraw(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_, '_>) {
        self.cursor_state.update(context.elapsed());
        let cursor_state = self.cursor_state;
        let size = context.gfx.size();
        let styles = context.query_styles(&[&TextColor, &HighlightColor]);
        let highlight = styles.get(&HighlightColor, context);
        let editor = self.editor_mut(&mut context.gfx, &styles, &context.widget);
        let cursor = editor.cursor();
        let selection = editor.select_opt();
        let buffer = editor.buffer_mut();
        buffer.set_size(
            context.gfx.font_system(),
            size.width.into_float(),
            size.height.into_float(),
        );
        buffer.shape_until_scroll(context.gfx.font_system());

        if context.focused() {
            context.draw_focus_ring_using(&styles);
            context.set_ime_allowed(true);
            let line_height = Px::from_float(buffer.metrics().line_height);
            if let Some(selection) = selection {
                let (start, end) = if selection < cursor {
                    (selection, cursor)
                } else {
                    (cursor, selection)
                };

                match (cursor_glyph(buffer, &start), cursor_glyph(buffer, &end)) {
                    (Ok((start_position, _)), Ok((end_position, end_width))) => {
                        if start_position.y == end_position.y {
                            // Single line selection
                            let width = end_position.x - start_position.x + end_width;
                            context.gfx.draw_shape(
                                &Shape::filled_rect(
                                    Rect::new(start_position, Size::new(width, line_height)),
                                    highlight,
                                ),
                                Point::default(),
                                None,
                                None,
                            );
                        } else {
                            // Draw from start to end of line,
                            let width = size.width.into_signed() - start_position.x;
                            context.gfx.draw_shape(
                                &Shape::filled_rect(
                                    Rect::new(start_position, Size::new(width, line_height)),
                                    highlight,
                                ),
                                Point::default(),
                                None,
                                None,
                            );
                            // Fill region between
                            let bottom_of_first_line = start_position.y + line_height;
                            let distance_between = end_position.y - bottom_of_first_line;
                            if distance_between > 0 {
                                context.gfx.draw_shape(
                                    &Shape::filled_rect(
                                        Rect::new(
                                            Point::new(Px(0), bottom_of_first_line),
                                            Size::new(size.width.into_signed(), distance_between),
                                        ),
                                        highlight,
                                    ),
                                    Point::default(),
                                    None,
                                    None,
                                );
                            }
                            // Draw from 0 to end + width
                            context.gfx.draw_shape(
                                &Shape::filled_rect(
                                    Rect::new(
                                        Point::new(Px(0), end_position.y),
                                        Size::new(end_position.x + end_width, line_height),
                                    ),
                                    highlight,
                                ),
                                Point::default(),
                                None,
                                None,
                            );
                        }
                    }
                    (Ok((start_position, _)), Err(_)) => {
                        let width = size.width.into_signed() - start_position.x;
                        context.gfx.draw_shape(
                            &Shape::filled_rect(
                                Rect::new(start_position, Size::new(width, line_height)),
                                highlight,
                            ),
                            Point::default(),
                            None,
                            None,
                        );
                    }
                    (Err(_), Ok((end_position, end_width))) => {
                        if end_position.y > 0 {
                            todo!("fill above start");
                        }
                        context.gfx.draw_shape(
                            &Shape::filled_rect(
                                Rect::new(
                                    Point::new(Px(0), end_position.y),
                                    Size::new(end_position.x + end_width, line_height),
                                ),
                                highlight,
                            ),
                            Point::default(),
                            None,
                            None,
                        );
                    }
                    (Err(start_not_visible), Err(end_not_visible))
                        if start_not_visible != end_not_visible =>
                    {
                        todo!("render full selection")
                    }
                    (Err(_), Err(_)) => {}
                }
            } else if let Ok((location, _)) = cursor_glyph(buffer, &cursor) {
                let window_focused = context.window().focused().get();
                if window_focused && cursor_state.visible {
                    context.gfx.draw_shape(
                        &Shape::filled_rect(
                            Rect::new(
                                location,
                                Size::new(Px(1), line_height),
                            ),
                            highlight, // TODO cursor should be a bold color, highlight probably not. This should have its own color.
                        ),
                        Point::default(),
                        None,
                        None,
                    );
                }
                if window_focused {
                    context.redraw_in(cursor_state.remaining_until_blink);
                } else {
                    context.redraw_when_changed(context.window().focused());
                }
            }
        }

        let text_color = styles.get(&TextColor, context);
        context.gfx.draw_text_buffer(
            buffer,
            text_color,
            TextOrigin::TopLeft,
            Point::<Px>::default(),
            None,
            None,
        );
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let styles = context.query_styles(&[&TextColor]);
        let editor = self.editor_mut(&mut context.graphics.gfx, &styles, &context.graphics.widget);
        let buffer = editor.buffer_mut();
        buffer.set_size(
            context.gfx.font_system(),
            available_space.width.max().into_float(),
            available_space.height.max().into_float(),
        );
        context
            .gfx
            .measure_text_buffer::<Px>(buffer, Color::WHITE)
            .size
            .into_unsigned()
    }

    fn keyboard_input(
        &mut self,
        _device_id: kludgine::app::winit::event::DeviceId,
        input: kludgine::app::winit::event::KeyEvent,
        _is_synthetic: bool,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        if let Some(on_key) = &mut self.on_key {
            on_key.invoke(input.clone())?;
        }

        let styles = context.query_styles(&[&TextColor]);
        let editor = self.editor_mut(context.kludgine, &styles, &context.widget);

        // println!(
        //     "Keyboard input: {:?}. {:?}, {:?}",
        //     input.logical_key, input.text, input.physical_key
        // );
        let (text_changed, handled) = match (input.state, input.logical_key, input.text) {
            (ElementState::Pressed,  key @ (Key::Backspace | Key::Delete), _) => {
                editor.action(
                    context.kludgine.font_system(),
                    match key {
                        Key::Backspace => Action::Backspace,
                        Key::Delete => Action::Delete,
                        _ => unreachable!("previously matched"),
                    },
                );
                (true, HANDLED)
            }
            (ElementState::Pressed, key @ (Key::ArrowLeft | Key::ArrowDown | Key::ArrowUp | Key::ArrowRight), _) => {
                let modifiers = context.modifiers();
                match (editor.select_opt(), modifiers.state().shift_key()) {
                    (None, true) => {
                        editor.set_select_opt(Some(editor.cursor()));
                    }
                    (Some(_), false) => {
                        editor.set_select_opt(None);
                    }
                    _ => {}
                };

                editor.action(
                    context.kludgine.font_system(),
                    match key {
                        Key::ArrowLeft if modifiers.word_select() => Action::PreviousWord,
                        Key::ArrowLeft => Action::Left,
                        Key::ArrowDown => Action::Down,
                        Key::ArrowUp => Action::Up,
                        Key::ArrowRight if modifiers.word_select() => Action::NextWord,
                        Key::ArrowRight => Action::Right,
                        _ => unreachable!("previously matched"),
                    },
                );
                (false, HANDLED)
            }
            (state, _, Some(text))
                if !context.modifiers().primary()
                    && text != "\t" // tab
                    && text != "\r" // enter/return
                    && text != "\u{1b}" // escape
                    =>
            {
                if state.is_pressed() {
                    editor.insert_string(&text, None);
                }
                (state.is_pressed(), HANDLED)
            }
            (_, _, _) => (false, IGNORED),
        };

        if handled.is_break() {
            context.set_needs_redraw();
            self.cursor_state.force_on();
            if text_changed {
                if let Value::Dynamic(value) = &self.text {
                    let state = self.editor.as_mut().expect("just_used");
                    value.map_mut(|text| {
                        text.clear();
                        for line in &state.editor.buffer().lines {
                            text.push_str(line.text());
                        }
                    });
                    state.generation = Some(value.generation());
                }
            }
        }

        handled
    }

    fn ime(&mut self, ime: Ime, context: &mut EventContext<'_, '_>) -> EventHandling {
        match ime {
            Ime::Enabled | Ime::Disabled => {}
            Ime::Preedit(text, cursor) => {
                tracing::warn!("TODO: preview IME input {text}, cursor: {cursor:?}");
            }
            Ime::Commit(text) => {
                self.editor_mut(
                    context.kludgine,
                    &Self::styles(&context.widget),
                    &context.widget,
                )
                .insert_string(&text, None);
                context.set_needs_redraw();
            }
        }

        HANDLED
    }

    fn focus(&mut self, context: &mut EventContext<'_, '_>) {
        context.set_ime_allowed(true);
        context.set_needs_redraw();
    }

    fn blur(&mut self, context: &mut EventContext<'_, '_>) {
        context.set_ime_allowed(false);
        context.set_needs_redraw();
    }
}

struct LiveEditor {
    editor: Editor,
    generation: Option<Generation>,
}

fn cursor_glyph(buffer: &Buffer, cursor: &Cursor) -> Result<(Point<Px>, Px), NotVisible> {
    // let cursor = buffer.layout_cursor(cursor);

    let mut layout_cursor = buffer.layout_cursor(cursor);
    // TODO this is because of a TODO inside of layout_cursor. It currently
    // falls back to 0,0 on the current line, rather than picking the last one.
    if layout_cursor.glyph == 0 && layout_cursor.layout == 0 && cursor.index > 0 {
        layout_cursor.glyph = usize::MAX;
    }
    let mut return_after_character = false;
    let searching_for = match buffer
        .lines
        .get(layout_cursor.line)
        .and_then(|line| {
            line.layout_opt()
                .as_ref()
                .expect("line layout missing")
                .get(layout_cursor.layout)
        })
        .and_then(|layout| layout.glyphs.get(layout_cursor.glyph))
        // TODO these should progressively fail rather than a single or_else.
        .or_else(|| {
            return_after_character = true;
            buffer
                .lines
                .last()
                .and_then(|line| {
                    line.layout_opt()
                        .as_ref()
                        .expect("line layout missing")
                        .last()
                })
                .and_then(|layout| layout.glyphs.last())
        }) {
        Some(glyph) => glyph,
        None => return Err(NotVisible::Before),
    }
    .start;

    for (index, run) in buffer.layout_runs().enumerate() {
        match run.line_i.cmp(&cursor.line) {
            Ordering::Less => continue,
            Ordering::Equal => {}
            Ordering::Greater => {
                if index > 0 {
                    return Err(NotVisible::After);
                }
                return Err(NotVisible::Before);
            }
        }
        if let Some(glyph) = run.glyphs.iter().find(|g| g.start == searching_for) {
            let physical = glyph.physical((0., run.line_y), 1.);
            let position = Point::new(Px(physical.x), Px::from_float(run.line_top));
            let width = Px::from_float(glyph.w);
            return Ok(if return_after_character {
                (Point::new(position.x + width, position.y), Px(0))
            } else {
                (position, width)
            });
        }
    }

    Err(NotVisible::After)
}

#[derive(Debug, Eq, PartialEq)]
enum NotVisible {
    Before,
    After,
}

#[derive(Clone, Copy)]
struct CursorState {
    visible: bool,
    remaining_until_blink: Duration,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            visible: true,
            remaining_until_blink: CURSOR_BLINK_DURATION,
        }
    }
}

impl CursorState {
    pub fn update(&mut self, elapsed: Duration) {
        let total_cycles = elapsed.as_nanos() / CURSOR_BLINK_DURATION.as_nanos();
        let remaining = Duration::from_nanos(
            u64::try_from(elapsed.as_nanos() % CURSOR_BLINK_DURATION.as_nanos())
                .expect("remainder fits in u64"),
        );
        // If we have an odd number of totaal cycles, flip the visibility.
        if total_cycles & 1 == 1 {
            self.visible = !self.visible;
        }

        if let Some(remaining) = self.remaining_until_blink.checked_sub(remaining) {
            self.remaining_until_blink = remaining;
        } else {
            self.visible = !self.visible;
            self.remaining_until_blink =
                CURSOR_BLINK_DURATION - (remaining - self.remaining_until_blink);
        }
    }

    pub fn force_on(&mut self) {
        self.visible = true;
        self.remaining_until_blink = CURSOR_BLINK_DURATION;
    }
}
