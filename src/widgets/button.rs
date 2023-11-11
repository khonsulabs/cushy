//! A clickable, labeled button
use std::borrow::Cow;
use std::panic::UnwindSafe;
use std::time::Duration;

use kludgine::app::winit::event::{DeviceId, ElementState, KeyEvent, MouseButton};
use kludgine::figures::units::{Lp, Px, UPx};
use kludgine::figures::{IntoUnsigned, Point, Rect, ScreenScale, Size};
use kludgine::shapes::StrokeOptions;
use kludgine::text::Text;
use kludgine::Color;

use crate::animation::{AnimationHandle, AnimationTarget, LinearInterpolate, Spawn};
use crate::context::{EventContext, GraphicsContext, LayoutContext, WidgetContext};
use crate::names::Name;
use crate::styles::components::{
    AutoFocusableControls, DisabledOutlineColor, Easing, IntrinsicPadding, OutlineColor,
    SurfaceColor, TextColor,
};
use crate::styles::{ColorExt, ComponentDefinition, ComponentGroup, ComponentName, NamedComponent};
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
    /// The enabled state of the button.
    pub enabled: Value<bool>,
    currently_enabled: bool,
    buttons_pressed: usize,
    colors: Option<Dynamic<Colors>>,

    color_animation: AnimationHandle,
}

impl Button {
    /// Returns a new button with the provided label.
    pub fn new(label: impl IntoValue<String>) -> Self {
        Self {
            label: label.into_value(),
            on_click: None,
            enabled: Value::Constant(true),
            currently_enabled: true,
            buttons_pressed: 0,
            colors: None,
            color_animation: AnimationHandle::default(),
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

    /// Sets the value to use for the button's enabled status.
    #[must_use]
    pub fn enabled(mut self, enabled: impl IntoValue<bool>) -> Self {
        self.enabled = enabled.into_value();
        self.currently_enabled = self.enabled.get();
        self
    }

    fn invoke_on_click(&mut self) {
        if self.enabled.get() {
            if let Some(on_click) = self.on_click.as_mut() {
                on_click.invoke(());
            }
        }
    }

    fn update_colors(&mut self, context: &WidgetContext<'_, '_>, immediate: bool) {
        let styles = context.query_styles(&[
            &ButtonActiveBackground,
            &ButtonBackground,
            &ButtonHoverBackground,
            &ButtonDisabledBackground,
            &Easing,
            &TextColor,
            &SurfaceColor,
            &OutlineColor,
            &DisabledOutlineColor,
        ]);
        let text_color = styles.get(&TextColor, context);
        let surface_color = styles.get(&SurfaceColor, context);
        let outline_color = styles.get(&OutlineColor, context);
        let (background, outline, text_color, surface_color) = if !self.enabled.get() {
            (
                styles.get(&ButtonDisabledBackground, context),
                styles.get(&DisabledOutlineColor, context),
                text_color,
                surface_color,
            )
        } else if context.is_default() {
            // TODO this probably should be de-prioritized if ButtonBackground is explicitly set.
            (
                context.theme().primary.color,
                context.theme().primary.color,
                context.theme().primary.on_color,
                context.theme().primary.color,
            )
        } else if context.active() {
            (
                styles.get(&ButtonActiveBackground, context),
                outline_color,
                text_color,
                surface_color,
            )
        } else if context.hovered() {
            (
                styles.get(&ButtonHoverBackground, context),
                outline_color,
                text_color,
                surface_color,
            )
        } else {
            (
                styles.get(&ButtonBackground, context),
                outline_color,
                text_color,
                surface_color,
            )
        };

        let text = background.most_contrasting(&[text_color, surface_color]);

        let new_colors = Colors {
            background,
            text,
            outline,
        };

        match (immediate, &self.colors) {
            (false, Some(colors)) => {
                self.color_animation = colors
                    .transition_to(new_colors)
                    .over(Duration::from_millis(150))
                    .with_easing(styles.get(&Easing, context))
                    .spawn();
            }
            (true, Some(colors)) => {
                colors.update(new_colors);
                self.color_animation.clear();
            }
            _ => {
                self.colors = Some(Dynamic::new(new_colors));
            }
        }
    }

    fn current_colors(&mut self, context: &WidgetContext<'_, '_>) -> Colors {
        if self.colors.is_none() {
            self.update_colors(context, false);
        }

        let colors = self.colors.as_ref().expect("always initialized");
        context.redraw_when_changed(colors);
        colors.get()
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
struct Colors {
    background: Color,
    text: Color,
    outline: Color,
}

impl LinearInterpolate for Colors {
    fn lerp(&self, target: &Self, percent: f32) -> Self {
        Self {
            background: self.background.lerp(&target.background, percent),
            text: self.text.lerp(&target.text, percent),
            outline: self.outline.lerp(&target.outline, percent),
        }
    }
}

impl Widget for Button {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let enabled = self.enabled.get();
        // TODO This seems ugly. It needs context, so it can't be moved into the
        // dynamic system.
        if self.currently_enabled != enabled {
            self.update_colors(context, false);
            self.currently_enabled = enabled;
        }

        let size = context.gfx.region().size;
        let center = Point::from(size) / 2;
        self.label.redraw_when_changed(context);
        self.enabled.redraw_when_changed(context);

        let colors = self.current_colors(context);
        context.gfx.fill(colors.background);

        if context.focused() {
            context.draw_focus_ring();
        } else {
            context.stroke_outline::<Lp>(colors.outline, StrokeOptions::default());
        }

        self.label.map(|label| {
            context.gfx.draw_text(
                Text::new(label, colors.text)
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

    fn accept_focus(&mut self, context: &mut EventContext<'_, '_>) -> bool {
        self.enabled.get() && context.query_style(&AutoFocusableControls).is_all()
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
            .into_px(context.gfx.scale())
            .into_unsigned();
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        self.label.map(|label| {
            let measured = context
                .gfx
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
        self.update_colors(context, false);
    }

    fn hover(&mut self, _location: Point<Px>, context: &mut EventContext<'_, '_>) {
        self.update_colors(context, false);
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
        self.update_colors(context, true);
    }

    fn deactivate(&mut self, context: &mut EventContext<'_, '_>) {
        self.update_colors(context, false);
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

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Color {
        context.theme().surface.color
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

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Color {
        context.theme().surface.dim_color
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

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Color {
        context.theme().surface.bright_color
    }
}

/// The background color of the button when the mouse cursor is hovering over
/// it.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ButtonDisabledBackground;

impl NamedComponent for ButtonDisabledBackground {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Button>("disabled_background_color"))
    }
}

impl ComponentDefinition for ButtonDisabledBackground {
    type ComponentType = Color;

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Color {
        context.theme().surface.dim_color
    }
}
