use std::fmt::Debug;

use kludgine::app::winit::keyboard::Key;
use kludgine::cosmic_text::{Action, Attrs, Buffer, Edit, Editor, FontSystem, Metrics, Shaping};
use kludgine::figures::units::Px;
use kludgine::figures::{FloatConversion, IntoUnsigned, Point};
use kludgine::text::TextOrigin;
use kludgine::{Color, Kludgine};

use crate::styles::{Styles, TextColor};
use crate::utils::ModifiersExt;
use crate::widget::{EventHandling, IntoValue, Value, Widget, HANDLED, UNHANDLED};

#[must_use]
pub struct Input {
    pub text: Value<String>,
    editor: Option<LiveEditor>,
}

impl Input {
    pub fn empty() -> Self {
        Self::new(String::new())
    }

    pub fn new(initial_text: impl IntoValue<String>) -> Self {
        Self {
            text: initial_text.into_value(),
            editor: None,
        }
    }

    fn editor_mut(&mut self, font_system: &mut FontSystem, styles: &Styles) -> &mut Editor {
        match (&self.editor, self.text.generation()) {
            (Some(editor), generation) if editor.generation == generation => {}
            (_, generation) => {
                let mut buffer = Buffer::new(font_system, Metrics::new(12., 18.));
                self.text.map(|text| {
                    buffer.set_text(font_system, text, Attrs::new(), Shaping::Advanced);
                });
                self.editor = Some(LiveEditor {
                    editor: Editor::new(buffer),
                    generation,
                });
            }
        }

        &mut self.editor.as_mut().expect("just initialized").editor
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
    fn hit_test(
        &mut self,
        location: Point<Px>,
        context: &mut crate::context::Context<'_, '_>,
    ) -> bool {
        true
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: kludgine::app::winit::event::DeviceId,
        button: kludgine::app::winit::event::MouseButton,
        context: &mut crate::context::Context<'_, '_>,
    ) -> EventHandling {
        // self.editor_mut(, styles);

        HANDLED
    }

    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: kludgine::app::winit::event::DeviceId,
        button: kludgine::app::winit::event::MouseButton,
        context: &mut crate::context::Context<'_, '_>,
    ) {
        context.focus();
    }

    fn redraw(
        &mut self,
        graphics: &mut crate::graphics::Graphics<'_, '_, '_>,
        context: &mut crate::context::Context<'_, '_>,
    ) {
        let size = graphics.size();
        let styles = context.query_style(&[&TextColor]);
        let editor = self.editor_mut(graphics.font_system(), &styles);
        let buffer = editor.buffer_mut();
        buffer.set_size(
            graphics.font_system(),
            size.width.into_float(),
            size.height.into_float(),
        );
        buffer.shape_until_scroll(graphics.font_system());
        graphics.draw_text_buffer(
            buffer,
            styles.get_or_default(&TextColor),
            TextOrigin::TopLeft,
            Point::<Px>::default(),
            None,
            None,
        );
    }

    fn measure(
        &mut self,
        available_space: kludgine::figures::Size<crate::ConstraintLimit>,
        graphics: &mut crate::graphics::Graphics<'_, '_, '_>,
        context: &mut crate::context::Context<'_, '_>,
    ) -> kludgine::figures::Size<kludgine::figures::units::UPx> {
        let styles = context.query_style(&[&TextColor]);
        let editor = self.editor_mut(graphics.font_system(), &styles);
        let buffer = editor.buffer_mut();
        buffer.set_size(
            graphics.font_system(),
            available_space.width.max().into_float(),
            available_space.height.max().into_float(),
        );
        graphics
            .measure_text_buffer::<Px>(buffer, Color::WHITE)
            .size
            .into_unsigned()
    }

    fn keyboard_input(
        &mut self,
        device_id: kludgine::app::winit::event::DeviceId,
        input: kludgine::app::winit::event::KeyEvent,
        is_synthetic: bool,
        kludgine: &mut Kludgine,
        context: &mut crate::context::Context<'_, '_>,
    ) -> EventHandling {
        if !input.state.is_pressed() {
            return UNHANDLED;
        }

        let styles = context.query_style(&[&TextColor]);
        let editor = &mut self.editor.as_mut().expect("input without editor").editor;

        match (input.logical_key, input.text) {
            (Key::Backspace, _) => {
                editor.action(kludgine.font_system(), Action::Backspace);
                context.set_needs_redraw();
                HANDLED
            }
            (_, Some(text)) if !context.modifiers().state().primary() => {
                editor.insert_string(&text, None);
                context.set_needs_redraw();
                HANDLED
            }
            (_, _) => UNHANDLED,
        }
    }
}

struct LiveEditor {
    editor: Editor,
    generation: Option<usize>,
}
