use std::borrow::Cow;
use std::panic::UnwindSafe;

use kludgine::app::winit::event::{DeviceId, ElementState, KeyEvent, MouseButton};
use kludgine::app::winit::keyboard::KeyCode;
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{IntoUnsigned, Point, Rect, Size};
use kludgine::shapes::Shape;
use kludgine::Color;

use crate::context::{EventContext, GraphicsContext};
use crate::names::Name;
use crate::styles::{
    ComponentDefinition, ComponentGroup, ComponentName, HighlightColor, NamedComponent, TextColor,
};
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
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let center = Point::from(context.graphics.size()) / 2;
        self.label.redraw_when_changed(context);

        let styles = context.query_style(&[
            &TextColor,
            &HighlightColor,
            &ButtonActiveBackground,
            &ButtonBackground,
            &ButtonHoverBackground,
        ]);

        let visible_rect = Rect::from(context.graphics.size() - (UPx(1), UPx(1)));

        let background = if context.active() {
            styles.get_or_default(&ButtonActiveBackground)
        } else if context.hovered() {
            styles.get_or_default(&ButtonHoverBackground)
        } else {
            styles.get_or_default(&ButtonBackground)
        };
        let background = Shape::filled_rect(visible_rect, background);
        context
            .graphics
            .draw_shape(&background, Point::default(), None, None);

        if context.focused() {
            context.draw_focus_ring(&styles);
        }

        let width = context.graphics.size().width;
        self.label.map(|label| {
            context.graphics.draw_text(
                label,
                styles.get_or_default(&TextColor),
                kludgine::text::TextOrigin::Center,
                center,
                None,
                None,
                Some(width),
            );
        });
    }

    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn mouse_down(
        &mut self,
        _location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        context: &mut EventContext<'_, '_>,
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
        context: &mut EventContext<'_, '_>,
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
        context: &mut EventContext<'_, '_>,
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
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        self.label.map(|label| {
            let measured = context
                .graphics
                .measure_text::<Px>(label, Color::WHITE, Some(width));

            let mut size = measured.size.into_unsigned();
            size.height = size.height.max(measured.line_height.into_unsigned());
            size
        })
    }

    fn keyboard_input(
        &mut self,
        _device_id: DeviceId,
        input: KeyEvent,
        _is_synthetic: bool,
        context: &mut EventContext<'_, '_>,
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

    fn unhover(&mut self, context: &mut EventContext<'_, '_>) {
        context.set_needs_redraw();
    }

    fn hover(&mut self, _location: Point<Px>, context: &mut EventContext<'_, '_>) {
        context.set_needs_redraw();
    }

    fn focus(&mut self, context: &mut EventContext<'_, '_>) {
        context.set_needs_redraw();
    }

    fn blur(&mut self, context: &mut EventContext<'_, '_>) {
        context.set_needs_redraw();
    }

    fn activate(&mut self, context: &mut EventContext<'_, '_>) {
        context.set_needs_redraw();
    }

    fn deactivate(&mut self, context: &mut EventContext<'_, '_>) {
        context.set_needs_redraw();
    }
}

impl ComponentGroup for Button {
    fn name() -> Name {
        Name::new("button")
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ButtonBackground;

impl NamedComponent for ButtonBackground {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Button>("background_color"))
    }
}

impl ComponentDefinition for ButtonBackground {
    type ComponentType = Color;

    fn default_value(&self) -> Color {
        Color::new(10, 10, 10, 255)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ButtonActiveBackground;

impl NamedComponent for ButtonActiveBackground {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Button>("active_background_color"))
    }
}

impl ComponentDefinition for ButtonActiveBackground {
    type ComponentType = Color;

    fn default_value(&self) -> Color {
        Color::new(30, 30, 30, 255)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ButtonHoverBackground;

impl NamedComponent for ButtonHoverBackground {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Button>("hover_background_color"))
    }
}

impl ComponentDefinition for ButtonHoverBackground {
    type ComponentType = Color;

    fn default_value(&self) -> Color {
        Color::new(40, 40, 40, 255)
    }
}
