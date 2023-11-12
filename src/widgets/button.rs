//! A clickable, labeled button
use std::borrow::Cow;
use std::panic::UnwindSafe;
use std::time::Duration;

use kludgine::app::winit::event::{DeviceId, ElementState, KeyEvent, MouseButton};
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size};
use kludgine::Color;

use crate::animation::{AnimationHandle, AnimationTarget, Spawn};
use crate::context::{AsEventContext, EventContext, GraphicsContext, LayoutContext, WidgetContext};
use crate::names::Name;
use crate::styles::components::{
    AutoFocusableControls, Easing, IntrinsicPadding, OpaqueWidgetColor, SurfaceColor, TextColor,
};
use crate::styles::{ColorExt, ComponentGroup, Styles};
use crate::utils::ModifiersExt;
use crate::value::{Dynamic, IntoValue, Value};
use crate::widget::{Callback, EventHandling, MakeWidget, Widget, WidgetRef, HANDLED, IGNORED};

/// A clickable button.
#[derive(Debug)]
pub struct Button {
    /// The label to display on the button.
    pub content: WidgetRef,
    /// The callback that is invoked when the button is clicked.
    pub on_click: Option<Callback<()>>,
    /// The enabled state of the button.
    pub enabled: Value<bool>,
    currently_enabled: bool,
    buttons_pressed: usize,
    background_color: Option<Dynamic<Color>>,
    text_color: Option<Dynamic<Color>>,
    color_animation: AnimationHandle,
}

impl Button {
    /// Returns a new button with the provided label.
    pub fn new(content: impl MakeWidget) -> Self {
        Self {
            content: content.widget_ref(),
            on_click: None,
            enabled: Value::Constant(true),
            currently_enabled: true,
            buttons_pressed: 0,
            background_color: None,
            text_color: None,
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
            &ButtonActiveForeground,
            &ButtonForeground,
            &ButtonHoverForeground,
            &ButtonDisabledForeground,
            &Easing,
        ]);

        let (background_color, text_color) = match () {
            () if !self.enabled.get() => (
                styles.get(&ButtonDisabledBackground, context),
                styles.get(&ButtonDisabledForeground, context),
            ),
            // TODO this probably should use actual style.
            () if context.is_default() => (
                context.theme().primary.color,
                context.theme().primary.on_color,
            ),
            () if context.active() => (
                styles.get(&ButtonActiveBackground, context),
                styles.get(&ButtonActiveForeground, context),
            ),
            () if context.hovered() => (
                styles.get(&ButtonHoverBackground, context),
                styles.get(&ButtonHoverForeground, context),
            ),
            () => (
                styles.get(&ButtonBackground, context),
                styles.get(&ButtonForeground, context),
            ),
        };

        match (immediate, &self.background_color, &self.text_color) {
            (false, Some(bg), Some(text)) => {
                self.color_animation = (
                    bg.transition_to(background_color),
                    text.transition_to(text_color),
                )
                    .over(Duration::from_millis(150))
                    .with_easing(styles.get(&Easing, context))
                    .spawn();
            }
            (true, Some(bg), Some(text)) => {
                bg.update(background_color);
                text.update(text_color);
                self.color_animation.clear();
            }
            _ => {
                self.background_color = Some(Dynamic::new(background_color));
                let text_color = Dynamic::new(text_color);
                self.text_color = Some(text_color.clone());
                context.attach_styles(Styles::new().with(&TextColor, text_color));
            }
        }
    }

    fn current_background(&mut self, context: &WidgetContext<'_, '_>) -> Color {
        if self.background_color.is_none() {
            self.update_colors(context, false);
        }

        let background_color = self.background_color.as_ref().expect("always initialized");
        context.redraw_when_changed(background_color);
        background_color.get()
    }
}

impl Widget for Button {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        #![allow(clippy::similar_names)]
        let enabled = self.enabled.get();
        // TODO This seems ugly. It needs context, so it can't be moved into the
        // dynamic system.
        if self.currently_enabled != enabled {
            self.update_colors(context, false);
            self.currently_enabled = enabled;
        }

        self.enabled.redraw_when_changed(context);

        let background_color = self.current_background(context);
        context.gfx.fill(background_color);

        if context.focused() {
            context.draw_focus_ring();
        }

        let content = self.content.mounted(&mut context.as_event_context());
        context.for_other(&content).redraw();
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
        let mounted = self.content.mounted(&mut context.as_event_context());
        let size = context.for_other(&mounted).layout(available_space);
        context.set_child_layout(
            &mounted,
            Rect::new(Point::new(padding, padding), size).into_signed(),
        );
        size + padding * 2
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

define_components! {
    Button {
        /// The background color of the button.
        ButtonBackground(Color, "background_color", |context| context.query_style(&OpaqueWidgetColor))
        /// The background color of the button when it is active (depressed).
        ButtonActiveBackground(Color, "active_background_color", .surface.color)
        /// The background color of the button when the mouse cursor is hovering over
        /// it.
        ButtonHoverBackground(Color, "hover_background_color", .surface.bright_color)
        /// The background color of the button when the mouse cursor is hovering over
        /// it.
        ButtonDisabledBackground(Color, "disabled_background_color", .surface.dim_color)
        /// The foreground color of the button.
        ButtonForeground(Color, "foreground_color", contrasting!(ButtonBackground, TextColor, SurfaceColor))
        /// The foreground color of the button when it is active (depressed).
        ButtonActiveForeground(Color, "active_foreground_color", contrasting!(ButtonActiveBackground, ButtonForeground, TextColor, SurfaceColor))
        /// The foreground color of the button when the mouse cursor is hovering over
        /// it.
        ButtonHoverForeground(Color, "hover_foreground_color", contrasting!(ButtonHoverBackground, ButtonForeground, TextColor, SurfaceColor))
        /// The foreground color of the button when the mouse cursor is hovering over
        /// it.
        ButtonDisabledForeground(Color, "disabled_foreground_color", contrasting!(ButtonDisabledBackground, ButtonForeground, TextColor, SurfaceColor))
    }
}
