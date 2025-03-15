//! A clickable, labeled button
use std::time::{Duration, Instant};

use figures::units::Px;
use figures::{IntoSigned, Point, Rect, Round, ScreenScale, Size, Zero};
use kludgine::app::winit::event::{Modifiers, MouseButton};
use kludgine::app::winit::window::CursorIcon;
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::Color;

use crate::animation::{
    AnimationHandle, AnimationTarget, IntoAnimate, LinearInterpolate, Spawn, ZeroToOne,
};
use crate::context::{AsEventContext, EventContext, GraphicsContext, LayoutContext, WidgetContext};
use crate::reactive::value::{Destination, Dynamic, IntoValue, Source, Value};
use crate::styles::components::{
    AutoFocusableControls, CornerRadius, DefaultActiveBackgroundColor,
    DefaultActiveForegroundColor, DefaultBackgroundColor, DefaultDisabledBackgroundColor,
    DefaultDisabledForegroundColor, DefaultForegroundColor, DefaultHoveredBackgroundColor,
    DefaultHoveredForegroundColor, Easing, HighlightColor, IntrinsicPadding, OpaqueWidgetColor,
    OutlineColor, OutlineWidth, SurfaceColor, TextColor,
};
use crate::styles::{ColorExt, Styles};
use crate::widget::{
    EventHandling, MakeWidget, Notify, SharedCallback, Widget, WidgetLayout, WidgetRef, HANDLED,
};
use crate::window::{DeviceId, WindowLocal};
use crate::FitMeasuredSize;

/// A clickable button.
#[derive(Debug)]
pub struct Button {
    /// The label to display on the button.
    pub content: WidgetRef,
    /// The callback that is invoked when the button is clicked.
    pub on_click: Option<Notify<Option<ButtonClick>>>,
    /// The kind of button to draw.
    pub kind: Value<ButtonKind>,
    focusable: bool,
    per_window: WindowLocal<PerWindow>,
}

#[derive(Debug, Default)]
struct PerWindow {
    buttons_pressed: usize,
    modifiers: Modifiers,
    cached_state: CacheState,
    active_colors: Option<Dynamic<ButtonColors>>,
    color_animation: AnimationHandle,
}

#[derive(Default, Debug, Eq, PartialEq, Clone, Copy)]
struct CacheState {
    style: Option<ButtonColors>,
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
        context: &WidgetContext<'_>,
    ) -> ButtonColors {
        match self {
            ButtonKind::Solid => match visual_state {
                VisualState::Normal => ButtonColors {
                    background: context.get(&DefaultBackgroundColor),
                    foreground: context.get(&DefaultForegroundColor),
                    outline: context.get(&ButtonOutline),
                },
                VisualState::Hovered => ButtonColors {
                    background: context.get(&DefaultHoveredBackgroundColor),
                    foreground: context.get(&DefaultHoveredForegroundColor),
                    outline: context.get(&ButtonHoverOutline),
                },
                VisualState::Active => ButtonColors {
                    background: context.get(&DefaultActiveBackgroundColor),
                    foreground: context.get(&DefaultActiveForegroundColor),
                    outline: context.get(&ButtonActiveOutline),
                },
                VisualState::Disabled => ButtonColors {
                    background: context.get(&DefaultDisabledBackgroundColor),
                    foreground: context.get(&DefaultDisabledForegroundColor),
                    outline: context.get(&ButtonDisabledOutline),
                },
            },
            ButtonKind::Outline | ButtonKind::Transparent => match visual_state {
                VisualState::Normal => ButtonColors {
                    background: context.get(&ButtonOutline),
                    foreground: context.get(&DefaultBackgroundColor),
                    outline: context.get(&DefaultBackgroundColor),
                },
                VisualState::Hovered => ButtonColors {
                    background: context.get(&ButtonHoverOutline),
                    foreground: context.get(&DefaultHoveredBackgroundColor),
                    outline: context.get(&DefaultHoveredBackgroundColor),
                },
                VisualState::Active => ButtonColors {
                    background: context.get(&ButtonActiveOutline),
                    foreground: context.get(&DefaultActiveBackgroundColor),
                    outline: context.get(&DefaultActiveBackgroundColor),
                },
                VisualState::Disabled => ButtonColors {
                    background: context.get(&ButtonDisabledOutline),
                    foreground: context.get(&DefaultDisabledBackgroundColor),
                    outline: context.get(&DefaultDisabledBackgroundColor),
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
            content: content.into_ref(),
            on_click: None,
            per_window: WindowLocal::default(),
            kind: Value::Constant(ButtonKind::default()),
            focusable: true,
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
    pub fn on_click<F>(self, callback: F) -> Self
    where
        F: FnMut(Option<ButtonClick>) + Send + 'static,
    {
        self.on_click_notify(callback)
    }

    /// Sets `notify` to receive each click of this button, and returns self.
    #[must_use]
    pub fn on_click_notify(mut self, notify: impl Into<Notify<Option<ButtonClick>>>) -> Self {
        self.on_click = Some(notify.into());
        self
    }

    /// Prevents focus being given to this button.
    #[must_use]
    pub fn prevent_focus(mut self) -> Self {
        self.focusable = false;
        self
    }

    fn invoke_on_click(&mut self, button: Option<ButtonClick>, context: &WidgetContext<'_>) {
        if context.enabled() {
            if let Some(on_click) = self.on_click.as_mut() {
                on_click.notify(button);
            }
        }
    }

    fn visual_style(context: &WidgetContext<'_>) -> VisualState {
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
        context: &WidgetContext<'_>,
    ) -> ButtonColors {
        match visual_state {
            VisualState::Normal => ButtonColors {
                background: context
                    .try_get(&ButtonBackground)
                    .unwrap_or(Color::CLEAR_BLACK),
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
                background: context
                    .try_get(&ButtonDisabledBackground)
                    .unwrap_or(Color::CLEAR_BLACK),
                foreground: context.theme().surface.on_color_variant,
                outline: context.get(&ButtonDisabledOutline),
            },
        }
    }

    fn determine_stateful_colors(&mut self, context: &mut WidgetContext<'_>) -> ButtonColors {
        let kind = self.kind.get_tracking_redraw(context);
        let visual_state = Self::visual_style(context);

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

    fn update_colors(&mut self, context: &mut WidgetContext<'_>, immediate: bool) {
        let new_style = self.determine_stateful_colors(context);
        let window_local = self.per_window.entry(context).or_default();
        if window_local.cached_state.style.as_ref() == Some(&new_style) {
            return;
        }
        window_local.cached_state.style = Some(new_style);

        match (immediate, &window_local.active_colors) {
            (false, Some(style)) => {
                window_local.color_animation = (style.transition_to(new_style))
                    .over(Duration::from_millis(150))
                    .with_easing(context.get(&Easing))
                    .spawn();
            }
            (true, Some(style)) => {
                style.set(new_style);
                window_local.color_animation.clear();
            }
            _ => {
                let new_style = Dynamic::new(new_style);
                let foreground = new_style.map_each(|s| s.foreground);
                window_local.active_colors = Some(new_style);
                context.attach_styles(Styles::new().with(&TextColor, foreground));
            }
        }
    }

    fn current_style(&mut self, context: &mut WidgetContext<'_>) -> ButtonColors {
        if self
            .per_window
            .entry(context)
            .or_default()
            .active_colors
            .is_none()
        {
            self.update_colors(context, false);
        }

        let style = self
            .per_window
            .entry(context)
            .or_default()
            .active_colors
            .as_ref()
            .expect("always initialized");
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
    pub fn solid_colors(self, context: &WidgetContext<'_>) -> ButtonColors {
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
    pub fn outline_colors(self, context: &WidgetContext<'_>) -> ButtonColors {
        let solid = self.solid_colors(context);
        ButtonColors {
            background: solid.outline,
            foreground: solid.foreground,
            outline: solid.background,
        }
    }
}

impl Widget for Button {
    fn summarize(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Button")
            .field("content", &self.content)
            .field("kind", &self.kind)
            .finish()
    }

    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        #![allow(clippy::similar_names)]

        let current_style = self.kind.get_tracking_redraw(context);
        self.update_colors(context, false);

        let style = self.current_style(context);
        context.fill(style.background);

        let outline_options = StrokeOptions::px_wide(
            context
                .get(&OutlineWidth)
                .into_px(context.gfx.scale())
                .ceil(),
        );
        context.stroke_outline(style.outline, outline_options);

        if context.focused(true) {
            if current_style == ButtonKind::Transparent {
                let focus_color = context.get(&HighlightColor);
                // Some states of a transparent button have solid background
                // colors. most_contrasting from a 0-alpha color is not a
                // meaningful measurement, so we only start measuring contrast
                // once we reach 50% opacity. If we ever add solid background
                // tracking (<https://github.com/khonsulabs/cushy/issues/73>),
                // we should use that color for most_contrasting always.
                let color = if style.background.alpha() > 128 {
                    style
                        .background
                        .most_contrasting(&[focus_color, context.get(&TextColor)])
                } else {
                    focus_color
                }
                .with_alpha(128);

                let inset = context
                    .get(&IntrinsicPadding)
                    .into_px(context.gfx.scale())
                    .min(outline_options.line_width)
                    / 2;

                let options = outline_options.colored(color);
                let radii = context.get(&CornerRadius);
                let radii = radii.map(|r| r.into_px(context.gfx.scale()));
                let ring_rect =
                    Rect::new(Point::squared(inset), context.gfx.region().size - inset * 2);

                let focus_ring = if radii.is_zero() {
                    Shape::stroked_rect(ring_rect, options.into_px(context.gfx.scale()))
                } else {
                    Shape::stroked_round_rect(ring_rect, radii, options)
                };
                context.gfx.draw_shape(&focus_ring);
            } else if context.is_default() {
                context.stroke_outline(context.get(&OutlineColor), outline_options);
            } else {
                context.draw_focus_ring();
            }
        }

        let content = self.content.mounted(&mut context.as_event_context());
        context.for_other(&content).redraw();
    }

    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_>) -> bool {
        true
    }

    fn accept_focus(&mut self, context: &mut EventContext<'_>) -> bool {
        self.focusable && context.enabled() && context.get(&AutoFocusableControls).is_all()
    }

    fn mouse_down(
        &mut self,
        _location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        let per_window = self.per_window.entry(context).or_default();
        per_window.buttons_pressed += 1;
        per_window.modifiers = context.modifiers();
        context.activate();
        HANDLED
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        context: &mut EventContext<'_>,
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
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) {
        let window_local = self.per_window.entry(context).or_default();
        window_local.buttons_pressed -= 1;
        if window_local.buttons_pressed == 0 {
            context.deactivate();

            if let (true, Some(location)) = (self.focusable, location) {
                let last_layout = context.last_layout().expect("must have been rendered");
                // let button_relative
                if Rect::from(last_layout.size).contains(location) {
                    context.focus();

                    let modifiers = window_local.modifiers;
                    self.invoke_on_click(
                        Some(ButtonClick {
                            mouse_button: button,
                            location,
                            window_location: location + last_layout.origin,
                            modifiers,
                        }),
                        context,
                    );
                }
            }
        }
    }

    fn layout(
        &mut self,
        available_space: Size<crate::ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WidgetLayout {
        let outline_width = context
            .get(&OutlineWidth)
            .into_upx(context.gfx.scale())
            .ceil();
        let padding = context
            .get(&IntrinsicPadding)
            .into_upx(context.gfx.scale())
            .round()
            .max(outline_width);

        let double_padding = padding * 2;
        let mounted = self.content.mounted(context);
        let available_space = available_space.map(|space| space - double_padding);
        let layout = context.for_other(&mounted).layout(available_space);
        let size = available_space.fit_measured(layout.size);
        context.set_child_layout(
            &mounted,
            Rect::new(Point::squared(padding), size).into_signed(),
        );
        WidgetLayout {
            size: size + double_padding,
            baseline: layout.baseline.map(|baseline| baseline + padding),
        }
    }

    fn unhover(&mut self, context: &mut EventContext<'_>) {
        self.update_colors(context, false);
    }

    fn hover(
        &mut self,
        _location: Point<Px>,
        context: &mut EventContext<'_>,
    ) -> Option<CursorIcon> {
        self.update_colors(context, false);

        if context.enabled() {
            Some(CursorIcon::Pointer)
        } else {
            Some(CursorIcon::NotAllowed)
        }
    }

    fn focus(&mut self, context: &mut EventContext<'_>) {
        context.set_needs_redraw();
    }

    fn blur(&mut self, context: &mut EventContext<'_>) {
        context.set_needs_redraw();
    }

    fn activate(&mut self, context: &mut EventContext<'_>) {
        let window_local = self.per_window.entry(context).or_default();
        // If we have no buttons pressed, the event should fire on activate not
        // on deactivate.
        if window_local.buttons_pressed == 0 {
            self.invoke_on_click(None, context);
        }
        self.update_colors(context, true);
    }

    fn deactivate(&mut self, context: &mut EventContext<'_>) {
        self.update_colors(context, false);
    }

    fn unmounted(&mut self, context: &mut EventContext<'_>) {
        self.content.unmount_in(context);
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
        ButtonHoverBackground(Color, "hover_background_color", |context| context.get(&ButtonBackground).darken_by(ZeroToOne::new(0.8)))
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

/// A mouse click in a [`Button`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ButtonClick {
    /// The mouse button that caused the event.
    pub mouse_button: MouseButton,
    /// The location relative to the button of the click.
    pub location: Point<Px>,
    /// The location relative to the window of the click.
    pub window_location: Point<Px>,

    /// The keyboard modifiers state when this click began.
    pub modifiers: Modifiers,
}

/// A multi-click gesture recognizer.
pub struct ClickCounter {
    threshold: Value<Duration>,
    maximum: usize,
    last_click: Option<Instant>,
    count: usize,
    on_click: SharedCallback<(usize, Option<ButtonClick>)>,
    delay_fire: AnimationHandle,
}

impl ClickCounter {
    /// Returns a new click counter that allows up to `threshold` between each
    /// click to be recognized as a single action. `on_click` will be invoked
    /// after no clicks have been detected for `threshold`.
    ///
    /// `on_click` accepts two parameters:
    ///
    /// - The number of clicks recognized for this action.
    /// - The final [`ButtonClick`], if provided.
    #[must_use]
    pub fn new<F>(threshold: impl IntoValue<Duration>, mut on_click: F) -> Self
    where
        F: FnMut(usize, Option<ButtonClick>) + Send + 'static,
    {
        Self {
            threshold: threshold.into_value(),
            maximum: usize::MAX,
            last_click: None,
            count: 0,
            on_click: SharedCallback::new(move |(count, click)| on_click(count, click)),
            delay_fire: AnimationHandle::new(),
        }
    }

    /// Sets the maximum number of clicks this counter recognizes to `maximum`.
    ///
    /// This causes the counter to immediately invoke the callback when the
    /// maximum clicks have been reached, allowing for slightly more responsive
    /// interfaces when the user is clicking multiple times.
    #[must_use]
    pub fn with_maximum(mut self, maximum: usize) -> Self {
        self.maximum = maximum;
        self
    }

    /// Notes a single click.
    pub fn click(&mut self, click: Option<ButtonClick>) {
        let now = Instant::now();
        let threshold = self.threshold.get();
        if let Some(last_click) = self.last_click {
            let elapsed = now.saturating_duration_since(last_click);
            if elapsed < threshold {
                self.count += 1;
            } else {
                self.count = 1;
            }
        } else {
            self.count = 1;
        }
        self.last_click = Some(now);

        if self.count == self.maximum {
            self.delay_fire.clear();
            self.on_click.invoke((self.count, click));
            self.count = 0;
        } else {
            let on_activation = self.on_click.clone();
            let count = self.count;
            self.delay_fire = threshold
                .on_complete(move || {
                    on_activation.invoke((count, click));
                })
                .spawn();
        }
    }
}
