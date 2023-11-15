//! A clickable, labeled button
use std::panic::UnwindSafe;
use std::time::Duration;

use kludgine::app::winit::event::{DeviceId, ElementState, KeyEvent, MouseButton};
use kludgine::figures::units::{Lp, Px, UPx};
use kludgine::figures::{IntoSigned, Point, Rect, ScreenScale, Size};
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::Color;

use crate::animation::{AnimationHandle, AnimationTarget, LinearInterpolate, Spawn};
use crate::context::{
    AsEventContext, EventContext, GraphicsContext, LayoutContext, WidgetCacheKey, WidgetContext,
};
use crate::styles::components::{
    AutoFocusableControls, Easing, HighlightColor, IntrinsicPadding, OpaqueWidgetColor,
    OutlineColor, SurfaceColor, TextColor,
};
use crate::styles::{ColorExt, Styles};
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
    /// The kind of button to draw.
    pub kind: Value<ButtonKind>,
    buttons_pressed: usize,
    cached_state: CacheState,
    active_colors: Option<Dynamic<ButtonColors>>,
    color_animation: AnimationHandle,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
struct CacheState {
    key: WidgetCacheKey,
    kind: ButtonKind,
}

/// The type of a [`Button`] or similar clickable widget.
#[derive(Debug, Default, Eq, PartialEq, Clone, Copy)]
pub enum ButtonKind {
    /// A solid button.
    #[default]
    Solid,
    /// An outline button, which uses the same colors as [`ButtonKind::Solid`]
    /// but swaps the outline and background colors.
    Outline,
    /// A transparent button, which is transparent until it is hovered.
    Transparent,
}

impl ButtonKind {
    /// Returns the [`ButtonColors`] to apply for a
    /// [default](MakeWidget::into_default) button.
    #[must_use]
    pub fn colors_for_default(
        self,
        visual_state: VisualState,
        context: &WidgetContext<'_, '_>,
    ) -> ButtonColors {
        match self {
            ButtonKind::Solid => match visual_state {
                VisualState::Normal => ButtonColors {
                    background: context.theme().primary.color,
                    foreground: context.theme().primary.on_color,
                    outline: context.get(&ButtonOutline),
                },
                VisualState::Hovered => ButtonColors {
                    background: context.theme().primary.color_bright,
                    foreground: context.theme().primary.on_color,
                    outline: context.get(&ButtonHoverOutline),
                },
                VisualState::Active => ButtonColors {
                    background: context.theme().primary.color_dim,
                    foreground: context.theme().primary.on_color,
                    outline: context.get(&ButtonActiveOutline),
                },
                VisualState::Disabled => ButtonColors {
                    background: context.theme().primary.color_dim,
                    foreground: context.theme().primary.on_color,
                    outline: context.get(&ButtonDisabledOutline),
                },
            },
            ButtonKind::Outline | ButtonKind::Transparent => match visual_state {
                VisualState::Normal => ButtonColors {
                    background: context.get(&ButtonOutline),
                    foreground: context.theme().primary.color,
                    outline: context.theme().primary.color,
                },
                VisualState::Hovered => ButtonColors {
                    background: context.get(&ButtonHoverOutline),
                    foreground: context.theme().primary.color,
                    outline: context.theme().primary.color_bright,
                },
                VisualState::Active => ButtonColors {
                    background: context.get(&ButtonActiveOutline),
                    foreground: context.theme().primary.color,
                    outline: context.theme().surface.color,
                },
                VisualState::Disabled => ButtonColors {
                    background: context.get(&ButtonDisabledOutline),
                    foreground: context.theme().primary.on_color,
                    outline: context.theme().primary.color_dim,
                },
            },
        }
    }
}

/// The coloring to apply to a [`Button`] or button-like widget.
#[derive(Debug, PartialEq, Eq, Clone, Copy, LinearInterpolate)]
pub struct ButtonColors {
    /// The background color of the button.
    pub background: Color,
    /// The foreground (text) color of the button.
    pub foreground: Color,
    /// A color to use to surround the button.
    pub outline: Color,
}

impl Button {
    /// Returns a new button with the provided label.
    pub fn new(content: impl MakeWidget) -> Self {
        Self {
            content: content.widget_ref(),
            on_click: None,
            cached_state: CacheState {
                key: WidgetCacheKey::default(),
                kind: ButtonKind::default(),
            },
            buttons_pressed: 0,
            active_colors: None,
            kind: Value::Constant(ButtonKind::default()),
            color_animation: AnimationHandle::default(),
        }
    }

    /// Sets the button's `kind` and returns self.
    #[must_use]
    pub fn kind(mut self, kind: impl IntoValue<ButtonKind>) -> Self {
        self.kind = kind.into_value();
        self
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

    fn invoke_on_click(&mut self, context: &WidgetContext<'_, '_>) {
        if context.enabled() {
            if let Some(on_click) = self.on_click.as_mut() {
                on_click.invoke(());
            }
        }
    }

    fn visual_style(context: &WidgetContext<'_, '_>) -> VisualState {
        if !context.enabled() {
            VisualState::Disabled
        } else if context.active() {
            VisualState::Active
        } else if context.hovered() {
            VisualState::Hovered
        } else {
            VisualState::Normal
        }
    }

    /// Returns the coloring to apply to a [`ButtonKind::Transparent`] button.
    #[must_use]
    pub fn colors_for_transparent(
        visual_state: VisualState,
        context: &WidgetContext<'_, '_>,
    ) -> ButtonColors {
        match visual_state {
            VisualState::Normal => ButtonColors {
                background: Color::CLEAR_BLACK,
                foreground: context.get(&TextColor),
                outline: context.get(&ButtonOutline),
            },
            VisualState::Hovered => ButtonColors {
                background: context.get(&OpaqueWidgetColor),
                foreground: context.get(&TextColor),
                outline: context.get(&ButtonHoverOutline),
            },
            VisualState::Active => ButtonColors {
                background: context.get(&ButtonActiveBackground),
                foreground: context.get(&ButtonActiveForeground),
                outline: context.get(&ButtonActiveOutline),
            },
            VisualState::Disabled => ButtonColors {
                background: Color::CLEAR_BLACK,
                foreground: context.theme().surface.on_color_variant,
                outline: context.get(&ButtonDisabledOutline),
            },
        }
    }

    fn determine_stateful_colors(&mut self, context: &mut WidgetContext<'_, '_>) -> ButtonColors {
        let kind = self.kind.get_tracked(context);
        let visual_state = Self::visual_style(context);

        self.cached_state = CacheState {
            key: context.cache_key(),
            kind,
        };

        // TODO this should be genericized to happen automatically.
        if !context.enabled() {
            context.blur();
        }

        if context.is_default() {
            kind.colors_for_default(visual_state, context)
        } else {
            match kind {
                ButtonKind::Transparent => Self::colors_for_transparent(visual_state, context),
                ButtonKind::Solid => visual_state.solid_colors(context),
                ButtonKind::Outline => visual_state.outline_colors(context),
            }
        }
    }

    fn update_colors(&mut self, context: &mut WidgetContext<'_, '_>, immediate: bool) {
        let new_style = self.determine_stateful_colors(context);

        match (immediate, &self.active_colors) {
            (false, Some(style)) => {
                self.color_animation = (style.transition_to(new_style))
                    .over(Duration::from_millis(150))
                    .with_easing(context.get(&Easing))
                    .spawn();
            }
            (true, Some(style)) => {
                style.update(new_style);
                self.color_animation.clear();
            }
            _ => {
                let new_style = Dynamic::new(new_style);
                let foreground = new_style.map_each(|s| s.foreground);
                self.active_colors = Some(new_style);
                context.attach_styles(Styles::new().with(&TextColor, foreground));
            }
        }
    }

    fn current_style(&mut self, context: &mut WidgetContext<'_, '_>) -> ButtonColors {
        if self.active_colors.is_none() {
            self.update_colors(context, false);
        }

        let style = self.active_colors.as_ref().expect("always initialized");
        context.redraw_when_changed(style);
        style.get()
    }
}

/// The effective visual state of an element.
///
/// While an element may be multiple states (e.g., active and hovered), when
/// rendering a widget sometimes a single visual style must take priority. This
/// enum represents the various states a widget may be in for such a decision.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VisualState {
    /// The widget should render in its normal state.
    Normal,
    /// The widget should render in reaction to the mouse cursor being above the
    /// widget.
    Hovered,
    /// The widget should render in reaction to the widget being clicked on or
    /// activated by the user.
    Active,
    /// The widget should render in a way to convey to the user it is not
    /// enabled for interaction.
    Disabled,
}

impl VisualState {
    /// Returns the colors to apply to a [`ButtonKind::Solid`] [`Button`] or
    /// button-like widget.
    #[must_use]
    pub fn solid_colors(self, context: &WidgetContext<'_, '_>) -> ButtonColors {
        match self {
            VisualState::Normal => ButtonColors {
                background: context.get(&ButtonBackground),
                foreground: context.get(&ButtonForeground),
                outline: context.get(&ButtonOutline),
            },
            VisualState::Hovered => ButtonColors {
                background: context.get(&ButtonHoverBackground),
                foreground: context.get(&ButtonHoverForeground),
                outline: context.get(&ButtonHoverOutline),
            },
            VisualState::Active => ButtonColors {
                background: context.get(&ButtonActiveBackground),
                foreground: context.get(&ButtonActiveForeground),
                outline: context.get(&ButtonActiveOutline),
            },
            VisualState::Disabled => ButtonColors {
                background: context.get(&ButtonDisabledBackground),
                foreground: context.get(&ButtonDisabledForeground),
                outline: context.get(&ButtonDisabledOutline),
            },
        }
    }

    /// Returns the colors to apply to a [`ButtonKind::Outline`] [`Button`] or
    /// button-like widget.
    #[must_use]
    pub fn outline_colors(self, context: &WidgetContext<'_, '_>) -> ButtonColors {
        let solid = self.solid_colors(context);
        ButtonColors {
            background: solid.outline,
            foreground: solid.foreground,
            outline: solid.background,
        }
    }
}

impl Widget for Button {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        #![allow(clippy::similar_names)]

        let current_style = self.kind.get_tracked(context);
        if self.cached_state.key != context.cache_key() || self.cached_state.kind != current_style {
            self.update_colors(context, false);
        }

        let style = self.current_style(context);
        context.gfx.fill(style.background);

        let two_lp_stroke = StrokeOptions::lp_wide(Lp::points(2));
        context.stroke_outline(style.outline, two_lp_stroke);

        if context.focused() {
            if current_style == ButtonKind::Transparent {
                let two_lp_stroke = two_lp_stroke.into_px(context.gfx.scale());
                let focus_color = context.get(&HighlightColor);
                // Some states of a transparent button have solid background
                // colors. most_contrasting from a 0-alpha color is not a
                // meaningful measurement, so we only start measuring contrast
                // once we reach 50% opacity. If we ever add solid background
                // tracking (<https://github.com/khonsulabs/gooey/issues/73>),
                // we should use that color for most_contrasting always.
                let color = if style.background.alpha() > 128 {
                    style
                        .background
                        .most_contrasting(&[focus_color, context.get(&TextColor)])
                } else {
                    focus_color
                };

                let inset = context.get(&IntrinsicPadding).into_px(context.gfx.scale());

                let focus_ring = Shape::stroked_rect(
                    Rect::new(
                        Point::new(inset, inset),
                        context.gfx.region().size - inset * 2,
                    ),
                    color,
                    two_lp_stroke,
                );
                context
                    .gfx
                    .draw_shape(&focus_ring, Point::default(), None, None);
            } else if context.is_default() {
                context.stroke_outline(context.get(&OutlineColor), two_lp_stroke);
            } else {
                context.draw_focus_ring();
            }
        }

        let content = self.content.mounted(&mut context.as_event_context());
        context.for_other(&content).redraw();
    }

    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn accept_focus(&mut self, context: &mut EventContext<'_, '_>) -> bool {
        context.get(&AutoFocusableControls).is_all()
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

                    self.invoke_on_click(context);
                }
            }
        }
    }

    fn layout(
        &mut self,
        available_space: Size<crate::ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let padding = context.get(&IntrinsicPadding).into_upx(context.gfx.scale());
        let double_padding = padding * 2;
        let mounted = self.content.mounted(&mut context.as_event_context());
        let available_space = Size::new(
            available_space.width - double_padding,
            available_space.height - double_padding,
        );
        let size = context.for_other(&mounted).layout(available_space);
        let size = Size::new(
            available_space
                .width
                .fit_measured(size.width, context.gfx.scale()),
            available_space
                .height
                .fit_measured(size.height, context.gfx.scale()),
        );
        context.set_child_layout(
            &mounted,
            Rect::new(Point::new(padding, padding), size).into_signed(),
        );
        size + double_padding
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
                        self.invoke_on_click(context);
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
            self.invoke_on_click(context);
        }
        self.update_colors(context, true);
    }

    fn deactivate(&mut self, context: &mut EventContext<'_, '_>) {
        self.update_colors(context, false);
    }
}

define_components! {
    Button {
        /// The background color of the button.
        ButtonBackground(Color, "background_color", @OpaqueWidgetColor)
        /// The background color of the button when it is active (depressed).
        ButtonActiveBackground(Color, "active_background_color", .surface.color)
        /// The background color of the button when the mouse cursor is hovering over
        /// it.
        ButtonHoverBackground(Color, "hover_background_color", .surface.lowest_container)
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
        /// The outline color of the button.
        ButtonOutline(Color, "outline_color", Color::CLEAR_BLACK)
        /// The outline color of the button when it is active (depressed).
        ButtonActiveOutline(Color, "active_outline_color", Color::CLEAR_BLACK)
        /// The outline color of the button when the mouse cursor is hovering over
        /// it.
        ButtonHoverOutline(Color, "hover_outline_color", Color::CLEAR_BLACK)
        /// The outline color of the button when the mouse cursor is hovering over
        /// it.
        ButtonDisabledOutline(Color, "disabled_outline_color", Color::CLEAR_BLACK)
    }
}
