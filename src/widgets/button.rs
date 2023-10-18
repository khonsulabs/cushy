use std::panic::UnwindSafe;

use kludgine::app::winit::event::{DeviceId, ElementState, KeyEvent, MouseButton};
use kludgine::app::winit::keyboard::KeyCode;
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{Point, Rect, Size};
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::Color;

use crate::context::Context;
use crate::graphics::Graphics;
use crate::widget::{Callback, EventHandling, IntoValue, Value, Widget, HANDLED, UNHANDLED};

#[derive(Debug)]
pub struct Button {
    pub label: Value<String>,
    pub on_click: Option<Callback<()>>,
    buttons_pressed: usize,
}

impl Button {
    pub fn new(label: impl IntoValue<String>) -> Self {
        Self {
            label: label.into_value(),
            on_click: None,
            buttons_pressed: 0,
        }
    }

    #[must_use]
    pub fn on_click<F>(mut self, callback: F) -> Self
    where
        F: FnMut(()) + Send + UnwindSafe + 'static,
    {
        self.on_click = Some(Callback::new(callback));
        self
    }

    fn invoke_on_click(&mut self) {
        if let Some(on_click) = self.on_click.as_mut() {
            on_click.invoke(());
        }
    }
}

impl Widget for Button {
    fn redraw(&mut self, graphics: &mut Graphics<'_, '_, '_>, context: &mut Context) {
        let center = Point::from(graphics.size()) / 2;
        if let Value::Dynamic(label) = &self.label {
            context.redraw_when_changed(label);
        }

        let visible_rect = Rect::from(graphics.size() - (UPx(1), UPx(1)));

        let background = if context.active() {
            Color::new(30, 30, 30, 255)
        } else if context.hovered() {
            Color::new(40, 40, 40, 255)
        } else {
            Color::new(10, 10, 10, 255)
        };
        let background = Shape::filled_rect(visible_rect, background);
        graphics.draw_shape(&background, Point::default(), None, None);

        if context.focused() {
            let focus_ring =
                Shape::stroked_rect(visible_rect, Color::AQUA, StrokeOptions::default());
            graphics.draw_shape(&focus_ring, Point::default(), None, None);
        }

        let width = graphics.size().width;
        self.label.map(|label| {
            graphics.draw_text(
                label,
                Color::WHITE,
                kludgine::text::TextOrigin::Center,
                center,
                None,
                None,
                Some(width),
            );
        });
    }

    fn hit_test(&mut self, _location: Point<Px>, _context: &mut Context<'_, '_>) -> bool {
        true
    }

    fn mouse_down(
        &mut self,
        _location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        context: &mut Context<'_, '_>,
    ) -> EventHandling {
        self.buttons_pressed += 1;
        context.activate();
        HANDLED
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        context: &mut Context<'_, '_>,
    ) {
        let changed = if Rect::from(
            context
                .last_rendered_at()
                .expect("must have been rendered")
                .size,
        )
        .contains(location)
        {
            context.activate()
        } else {
            context.deactivate()
        };

        if changed {
            context.set_needs_redraw();
        }
    }

    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        _device_id: DeviceId,
        _button: MouseButton,
        context: &mut Context<'_, '_>,
    ) {
        self.buttons_pressed -= 1;
        if self.buttons_pressed == 0 {
            context.deactivate();

            if let Some(location) = location {
                if Rect::from(
                    context
                        .last_rendered_at()
                        .expect("must have been rendered")
                        .size,
                )
                .contains(location)
                {
                    context.focus();

                    self.invoke_on_click();
                }
            }
        }
    }

    fn measure(
        &mut self,
        available_space: Size<crate::ConstraintLimit>,
        graphics: &mut Graphics<'_, '_, '_>,
        _context: &mut Context<'_, '_>,
    ) -> Size<UPx> {
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        self.label.map(|label| {
            graphics
                .measure_text::<Px>(label, Color::RED, Some(width))
                .size
                .try_cast::<UPx>()
                .unwrap_or_default()
        })
    }

    fn keyboard_input(
        &mut self,
        _device_id: DeviceId,
        input: KeyEvent,
        _is_synthetic: bool,
        context: &mut Context<'_, '_>,
    ) -> EventHandling {
        if input.physical_key == KeyCode::Space {
            let changed = match input.state {
                ElementState::Pressed => context.activate(),
                ElementState::Released => {
                    self.invoke_on_click();
                    context.deactivate()
                }
            };
            if changed {
                context.set_needs_redraw();
            }
            HANDLED
        } else {
            UNHANDLED
        }
    }

    fn unhover(&mut self, context: &mut Context<'_, '_>) {
        context.set_needs_redraw();
    }

    fn hover(&mut self, _location: Point<Px>, context: &mut Context<'_, '_>) {
        context.set_needs_redraw();
    }

    fn focus(&mut self, context: &mut Context<'_, '_>) {
        context.set_needs_redraw();
    }

    fn blur(&mut self, context: &mut Context<'_, '_>) {
        context.set_needs_redraw();
    }

    fn activate(&mut self, context: &mut Context<'_, '_>) {
        context.set_needs_redraw();
    }

    fn deactivate(&mut self, context: &mut Context<'_, '_>) {
        context.set_needs_redraw();
    }
}
