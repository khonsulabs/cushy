//! A clickable, labeled button
use std::borrow::Cow;
use std::panic::UnwindSafe;
use std::time::Duration;

use kludgine::app::winit::event::{DeviceId, ElementState, KeyEvent, MouseButton};
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{IntoUnsigned, Point, Rect, ScreenScale, Size};
use kludgine::shapes::Shape;
use kludgine::text::Text;
use kludgine::Color;

use crate::animation::{AnimationHandle, AnimationTarget, Spawn};
use crate::context::{EventContext, GraphicsContext, LayoutContext, WidgetContext};
use crate::names::Name;
use crate::styles::components::{
    Easing, HighlightColor, IntrinsicPadding, PrimaryColor, TextColor,
};
use crate::styles::{ComponentDefinition, ComponentGroup, ComponentName, NamedComponent};
use crate::utils::ModifiersExt;
use crate::value::{Dynamic, IntoValue, Value};
use crate::widget::{Callback, EventHandling, Widget, HANDLED, IGNORED};

/// A clickable button.
#[derive(Debug)]
pub struct Button {
    /// The label to display on the button.
    pub label: Value<String>,
    /// The callback that is invoked when the button is clicked.
    pub on_click: Option<Callback<()>>,
    buttons_pressed: usize,
    background_color: Option<Dynamic<Color>>,
    background_color_animation: AnimationHandle,
}

impl Button {
    /// Returns a new button with the provided label.
    pub fn new(label: impl IntoValue<String>) -> Self {
        Self {
            label: label.into_value(),
            on_click: None,
            buttons_pressed: 0,
            background_color: None,
            background_color_animation: AnimationHandle::default(),
        }
    }

    /// Sets the `on_click` callback and returns self.
    ///
    /// This callback will be invoked each time the button is clicked.
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

    fn update_background_color(&mut self, context: &WidgetContext<'_, '_>, immediate: bool) {
        let styles = context.query_styles(&[
            &ButtonActiveBackground,
            &ButtonBackground,
            &ButtonHoverBackground,
            &PrimaryColor,
            &Easing,
        ]);
        let background_color = if context.active() {
            styles.get_or_default(&ButtonActiveBackground)
        } else if context.hovered() {
            styles.get_or_default(&ButtonHoverBackground)
        } else if context.is_default() {
            styles.get_or_default(&PrimaryColor)
        } else {
            styles.get_or_default(&ButtonBackground)
        };

        match (immediate, &self.background_color) {
            (false, Some(dynamic)) => {
                self.background_color_animation = dynamic
                    .transition_to(background_color)
                    .over(Duration::from_millis(150))
                    .with_easing(styles.get_or_default(&Easing))
                    .spawn();
            }
            (true, Some(dynamic)) => {
                dynamic.set(background_color);
                self.background_color_animation.clear();
            }
            (_, None) => {
                let dynamic = Dynamic::new(background_color);
                self.background_color = Some(dynamic);
            }
        }
    }

    fn current_background_color(&mut self, context: &WidgetContext<'_, '_>) -> Color {
        if self.background_color.is_none() {
            self.update_background_color(context, false);
        }

        let background_color = self.background_color.as_ref().expect("always initialized");
        context.redraw_when_changed(background_color);
        background_color.get()
    }
}

impl Widget for Button {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let size = context.graphics.region().size;
        let center = Point::from(size) / 2;
        self.label.redraw_when_changed(context);

        let styles = context.query_styles(&[
            &TextColor,
            &HighlightColor,
            &ButtonActiveBackground,
            &ButtonBackground,
            &ButtonHoverBackground,
        ]);

        let visible_rect = Rect::from(size - (Px(1), Px(1)));

        let background = self.current_background_color(context);
        let background = Shape::filled_rect(visible_rect, background);
        context
            .graphics
            .draw_shape(&background, Point::default(), None, None);

        if context.focused() {
            context.draw_focus_ring_using(&styles);
        }

        self.label.map(|label| {
            context.graphics.draw_text(
                Text::new(label, styles.get_or_default(&TextColor))
                    .origin(kludgine::text::TextOrigin::Center)
                    .wrap_at(size.width),
                center,
                None,
                None,
            );
        });
    }

    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn accept_focus(&mut self, _context: &mut EventContext<'_, '_>) -> bool {
        // TODO this should be driven by a "focus_all_widgets" setting that hopefully can be queried from the OS.
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
        let changed = if Rect::from(context.last_layout().expect("must have been rendered").size)
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
                if Rect::from(context.last_layout().expect("must have been rendered").size)
                    .contains(location)
                {
                    context.focus();

                    self.invoke_on_click();
                }
            }
        }
    }

    fn layout(
        &mut self,
        available_space: Size<crate::ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let padding = context
            .query_style(&IntrinsicPadding)
            .into_px(context.graphics.scale())
            .into_unsigned();
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        self.label.map(|label| {
            let measured = context
                .graphics
                .measure_text::<Px>(Text::from(label).wrap_at(width));

            let mut size = measured.size.into_unsigned();
            size.width += padding * 2;
            size.height = size.height.max(measured.line_height.into_unsigned()) + padding * 2;
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
        // TODO should this be handled at the window level?
        if input.text.as_deref() == Some(" ") && !context.modifiers().possible_shortcut() {
            let changed = match input.state {
                ElementState::Pressed => {
                    let changed = context.activate();
                    if !changed {
                        // The widget was already active. This is now a repeated keypress
                        self.invoke_on_click();
                    }
                    changed
                }
                ElementState::Released => context.deactivate(),
            };
            if changed {
                context.set_needs_redraw();
            }
            HANDLED
        } else {
            IGNORED
        }
    }

    fn unhover(&mut self, context: &mut EventContext<'_, '_>) {
        self.update_background_color(context, false);
    }

    fn hover(&mut self, _location: Point<Px>, context: &mut EventContext<'_, '_>) {
        self.update_background_color(context, false);
    }

    fn focus(&mut self, context: &mut EventContext<'_, '_>) {
        context.set_needs_redraw();
    }

    fn blur(&mut self, context: &mut EventContext<'_, '_>) {
        context.set_needs_redraw();
    }

    fn activate(&mut self, context: &mut EventContext<'_, '_>) {
        // If we have no buttons pressed, the event should fire on activate not
        // on deactivate.
        if self.buttons_pressed == 0 {
            self.invoke_on_click();
        }
        self.update_background_color(context, true);
    }

    fn deactivate(&mut self, context: &mut EventContext<'_, '_>) {
        self.update_background_color(context, false);
    }
}

impl ComponentGroup for Button {
    fn name() -> Name {
        Name::new("button")
    }
}

/// The background color of the button.
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

/// The background color of the button when it is active (depressed).
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

/// The background color of the button when the mouse cursor is hovering over
/// it.
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
