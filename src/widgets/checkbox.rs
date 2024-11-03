//! A tri-state, labelable checkbox widget.
use std::error::Error;
use std::fmt::Display;
use std::ops::Not;
use std::time::Duration;

use figures::units::{Lp, Px, UPx};
use figures::{IntoSigned, IntoUnsigned, Point, Rect, Round, ScreenScale, Size, Zero};
use kludgine::app::winit::window::CursorIcon;
use kludgine::shapes::{CornerRadii, PathBuilder, Shape, StrokeOptions};
use kludgine::Color;

use super::button::{
    ButtonActiveBackground, ButtonActiveForeground, ButtonBackground, ButtonColors,
    ButtonDisabledBackground, ButtonHoverBackground, ButtonHoverForeground, VisualState,
};
use super::Button;
use crate::animation::{AnimationHandle, AnimationTarget, Spawn, ZeroToOne};
use crate::context::{EventContext, GraphicsContext, LayoutContext, WidgetContext};
use crate::styles::components::{
    AutoFocusableControls, CornerRadius, Easing, FocusColor, IntrinsicPadding, LineHeight,
    OutlineColor, OutlineWidth, TextColor, WidgetAccentColor, WidgetBackground,
};
use crate::styles::{Dimension, Hsla};
use crate::value::{Destination, Dynamic, DynamicReader, IntoDynamic, IntoValue, Source, Value};
use crate::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetInstance, WidgetRef, HANDLED,
    IGNORED,
};
use crate::widgets::button::ButtonKind;
use crate::window::WindowLocal;
use crate::ConstraintLimit;

/// A labeled-widget that supports three states: Checked, Unchecked, and
/// Indeterminant
pub struct Checkbox {
    /// The state (value) of the checkbox.
    pub state: Dynamic<CheckboxState>,
    /// The button kind to use as the basis for this checkbox. If `None`, the
    /// checkbox indicator will be a standalone, focusable widget.
    pub kind: Option<Value<ButtonKind>>,
    label: Option<WidgetInstance>,
    focusable: bool,
}

impl Checkbox {
    /// Returns a new checkbox that updates `state` when clicked.
    ///
    /// `state` can also be a `Dynamic<bool>` if there is no need to represent
    /// an indeterminant state.
    pub fn new(state: impl IntoDynamic<CheckboxState>) -> Self {
        Self {
            state: state.into_dynamic(),
            kind: None,
            label: None,
            focusable: true,
        }
    }

    /// Returns a new checkbox that updates `state` when clicked. `label` is
    /// drawn next to the checkbox and is also clickable to toggle the checkbox.
    ///
    /// `state` can also be a `Dynamic<bool>` if there is no need to represent
    /// an indeterminant state.
    #[must_use]
    pub fn labelled_by(mut self, label: impl MakeWidget) -> Self {
        self.label = Some(label.make_widget());
        self
    }

    /// Updates the button kind to use as the basis for this checkbox, and
    /// returns self.
    ///
    /// This causes the checkbox to become a regular [`Button`], which may be
    /// desireable for layout and/or visual consistency purposes.
    #[must_use]
    pub fn kind(mut self, kind: impl IntoValue<ButtonKind>) -> Self {
        self.kind = Some(kind.into_value());
        self
    }
}

impl MakeWidgetWithTag for Checkbox {
    fn make_with_tag(self, id: crate::widget::WidgetTag) -> WidgetInstance {
        if let Some(kind) = self.kind {
            let adornment = CheckboxOrnament {
                value: self.state.create_reader(),
            };
            let button_label = if let Some(label) = self.label {
                adornment.and(label).into_columns().make_widget()
            } else {
                adornment.make_widget()
            };

            let mut button = button_label
                .into_button()
                .on_click(move |_| {
                    let mut value = self.state.lock();
                    *value = !*value;
                })
                .kind(kind);

            if !self.focusable {
                button = button.prevent_focus();
            }
            button.make_with_tag(id)
        } else {
            InteractiveCheckbox {
                state: self.state,
                label: self.label.map(WidgetRef::from),
                focusable: self.focusable,
                per_window: WindowLocal::default(),
            }
            .make_with_tag(id)
        }
    }
}

#[derive(Debug)]
struct InteractiveCheckbox {
    state: Dynamic<CheckboxState>,
    label: Option<WidgetRef>,
    focusable: bool,
    per_window: WindowLocal<WindowLocalState>,
}

#[derive(Debug, Default)]
struct WindowLocalState {
    active_colors: Option<Dynamic<ButtonColors>>,
    target_colors: Option<ButtonColors>,
    color_animation: AnimationHandle,
    checkbox_region: Rect<Px>,
    label_region: Rect<Px>,
    focused: bool,
    hovered: bool,
    mouse_buttons_pressed: usize,
    checkbox_size: Px,
}

impl WindowLocalState {
    fn update_colors(&mut self, context: &mut WidgetContext<'_>, immediate: bool, is_empty: bool) {
        let (background, foreground) = if self.hovered {
            if self.mouse_buttons_pressed > 0 {
                (
                    context.get(&ButtonActiveBackground),
                    context.get(&ButtonActiveForeground),
                )
            } else {
                (
                    context.get(&ButtonHoverBackground),
                    context.get(&ButtonHoverForeground),
                )
            }
        } else {
            (context.get(&WidgetBackground), context.get(&TextColor))
        };
        let outline = if self.focused {
            if is_empty {
                let focus_color = context.get(&FocusColor);
                if self.hovered {
                    let mut focus_hsla = Hsla::from(focus_color);
                    focus_hsla.hsl.lightness *= ZeroToOne::new(0.8);
                    Color::from(focus_hsla)
                } else {
                    focus_color
                }
            } else {
                context.get(&OutlineColor)
            }
        } else if !context.enabled() {
            context.get(&ButtonDisabledBackground)
        } else if self.mouse_buttons_pressed > 0 && self.hovered {
            context.get(&ButtonActiveBackground)
        } else if self.hovered {
            context.get(&ButtonHoverBackground)
        } else {
            context.get(&ButtonBackground)
        };
        let desired_colors = ButtonColors {
            background,
            foreground,
            outline,
        };

        if let Some(active_colors) = &self.active_colors {
            if self.target_colors.as_ref() != Some(&desired_colors) {
                if immediate {
                    active_colors.set(desired_colors);
                    self.color_animation.clear();
                } else {
                    self.color_animation = active_colors
                        .transition_to(desired_colors)
                        .over(Duration::from_millis(150))
                        .with_easing(context.get(&Easing))
                        .spawn();
                }
            }
        } else {
            self.active_colors = Some(Dynamic::new(desired_colors));
        }
        self.target_colors = Some(desired_colors);
    }

    fn hit_test(&self, location: Point<Px>) -> bool {
        self.checkbox_region.contains(location)
            || self.label_region.contains(location)
            || (location.x > self.checkbox_region.size.width
                && location.x < self.label_region.origin.x
                && location.y >= self.checkbox_region.origin.y
                && location.y <= self.checkbox_region.origin.y + self.checkbox_region.size.height)
    }
}

impl InteractiveCheckbox {
    fn update_colors(&mut self, context: &mut WidgetContext<'_>, immediate: bool, is_empty: bool) {
        let window_local = self.per_window.entry(context).or_default();
        window_local.update_colors(context, immediate, is_empty);
    }

    fn clicked(&mut self, context: &WidgetContext<'_>) {
        if context.enabled() {
            self.state.toggle();
        }
    }
}

impl Widget for InteractiveCheckbox {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        let window_local = self.per_window.entry(context).or_default();
        let state = self.state.get_tracking_redraw(context);
        let state = if window_local.mouse_buttons_pressed > 0 && window_local.hovered {
            !state
        } else {
            state
        };
        window_local.update_colors(context, false, state == CheckboxState::Unchecked);
        let colors = window_local
            .active_colors
            .as_ref()
            .expect("always present after update_colors")
            .get_tracking_redraw(context);
        let mut fill_color = context.get(&WidgetAccentColor);
        if window_local.mouse_buttons_pressed > 0 {
            let mut color_hsla = Hsla::from(fill_color);
            color_hsla.hsl.lightness *= ZeroToOne::new(0.8);
            fill_color = Color::from(color_hsla);
        }

        draw_checkbox(
            &colors,
            state,
            fill_color,
            window_local.checkbox_region,
            context,
        );

        if let Some(label) = &mut self.label {
            let label = label.mounted(context);
            context.for_other(&label).redraw();
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let window_local = self.per_window.entry(context).or_default();
        window_local.checkbox_size = context
            .get(&CheckboxSize)
            .into_px(context.gfx.scale())
            .ceil();
        window_local.checkbox_region.size = Size::squared(window_local.checkbox_size);

        let full_size = if let Some(label) = &mut self.label {
            let padding = context
                .get(&IntrinsicPadding)
                .into_px(context.gfx.scale())
                .ceil();
            let x_offset = window_local.checkbox_size + padding;
            let remaining_space = Size::new(
                available_space.width - x_offset.into_unsigned(),
                available_space.height,
            );
            let mounted = label.mounted(context);
            let label_size = context
                .for_other(&mounted)
                .layout(remaining_space)
                .into_signed();
            let height = available_space
                .height
                .fit_measured(label_size.height.into_unsigned())
                .into_signed()
                .max(window_local.checkbox_size);

            window_local.label_region = Rect::new(
                Point::new(x_offset, (height - label_size.height) / 2),
                label_size,
            );
            context.set_child_layout(&mounted, window_local.label_region);

            Size::new(label_size.width + x_offset, height).into_unsigned()
        } else {
            Size::squared(window_local.checkbox_size).into_unsigned()
        };

        window_local.checkbox_region.origin.y =
            (full_size.height.into_signed() - window_local.checkbox_size) / 2;

        full_size
    }

    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> bool {
        let window_local = self.per_window.entry(context).or_default();
        window_local.hit_test(location)
    }

    fn mouse_down(
        &mut self,
        _location: Point<Px>,
        _device_id: crate::window::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        if context.enabled() {
            let window_local = self.per_window.entry(context).or_default();
            window_local.mouse_buttons_pressed += 1;

            context.set_needs_redraw();

            HANDLED
        } else {
            IGNORED
        }
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: crate::window::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_>,
    ) {
        let window_local = self.per_window.entry(context).or_default();
        let hovered = window_local.hit_test(location);
        if hovered != window_local.hovered {
            window_local.hovered = hovered;
            context.set_needs_redraw();
        }
    }

    fn mouse_up(
        &mut self,
        _location: Option<Point<Px>>,
        _device_id: crate::window::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_>,
    ) {
        let window_local = self.per_window.entry(context).or_default();
        window_local.mouse_buttons_pressed -= 1;
        let hovered = window_local.hovered;
        if window_local.mouse_buttons_pressed == 0 {
            self.clicked(context);
        }
        if self.focusable && hovered {
            context.focus();
        }
        context.set_needs_redraw();
    }

    fn accept_focus(&mut self, context: &mut EventContext<'_>) -> bool {
        self.focusable && context.enabled() && context.get(&AutoFocusableControls).is_all()
    }

    fn focus(&mut self, context: &mut EventContext<'_>) {
        let window_local = self.per_window.entry(context).or_default();
        window_local.focused = true;
        context.set_needs_redraw();
    }

    fn blur(&mut self, context: &mut EventContext<'_>) {
        let window_local = self.per_window.entry(context).or_default();
        window_local.focused = false;
        context.set_needs_redraw();
    }

    fn hover(
        &mut self,
        _location: Point<Px>,
        context: &mut EventContext<'_>,
    ) -> Option<CursorIcon> {
        if context.enabled() {
            let window_local = self.per_window.entry(context).or_default();
            window_local.hovered = true;
            context.set_needs_redraw();
            Some(CursorIcon::Pointer)
        } else {
            Some(CursorIcon::NotAllowed)
        }
    }

    fn unhover(&mut self, context: &mut EventContext<'_>) {
        let window_local = self.per_window.entry(context).or_default();
        window_local.hovered = false;
        context.set_needs_redraw();
    }

    fn activate(&mut self, context: &mut EventContext<'_>) {
        let window_local = self.per_window.entry(context).or_default();
        // If we have no buttons pressed, the event should fire on activate not
        // on deactivate.
        if window_local.mouse_buttons_pressed == 0 {
            self.clicked(context);
        }
        self.update_colors(context, true, self.state.get() == CheckboxState::Unchecked);
    }
}

/// The state/value of a [`Checkbox`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckboxState {
    /// The checkbox should display showing that it is neither checked or
    /// unchecked.
    ///
    /// This state is used to represent concepts such as:
    ///
    /// - States that are neither true/false, or on/off.
    /// - States that are partially true or partially on.
    Indeterminant,
    /// The checkbox should display in an unchecked/off/false state.
    Unchecked,
    /// The checkbox should display in an checked/on/true state.
    Checked,
}

impl From<bool> for CheckboxState {
    fn from(value: bool) -> Self {
        if value {
            Self::Checked
        } else {
            Self::Unchecked
        }
    }
}

impl From<CheckboxState> for Option<bool> {
    fn from(value: CheckboxState) -> Self {
        match value {
            CheckboxState::Indeterminant => None,
            CheckboxState::Unchecked => Some(false),
            CheckboxState::Checked => Some(true),
        }
    }
}

impl From<Option<bool>> for CheckboxState {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => CheckboxState::Checked,
            Some(false) => CheckboxState::Unchecked,
            None => CheckboxState::Indeterminant,
        }
    }
}

impl TryFrom<CheckboxState> for bool {
    type Error = CheckboxToBoolError;

    fn try_from(value: CheckboxState) -> Result<Self, Self::Error> {
        match value {
            CheckboxState::Checked => Ok(true),
            CheckboxState::Unchecked => Ok(false),
            CheckboxState::Indeterminant => Err(CheckboxToBoolError),
        }
    }
}

impl Not for CheckboxState {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Indeterminant | Self::Unchecked => Self::Checked,
            Self::Checked => Self::Unchecked,
        }
    }
}

impl IntoDynamic<CheckboxState> for Dynamic<bool> {
    fn into_dynamic(self) -> Dynamic<CheckboxState> {
        self.linked(
            |bool| CheckboxState::from(*bool),
            |tri_state: &CheckboxState| bool::try_from(*tri_state).ok(),
        )
    }
}

impl IntoDynamic<CheckboxState> for Dynamic<Option<bool>> {
    fn into_dynamic(self) -> Dynamic<CheckboxState> {
        self.linked(
            |bool| CheckboxState::from(*bool),
            |tri_state: &CheckboxState| bool::try_from(*tri_state).ok(),
        )
    }
}

/// An [`CheckboxState::Indeterminant`] was encountered when converting to a
/// `bool`.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CheckboxToBoolError;

impl Display for CheckboxToBoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CheckboxState was Indeterminant")
    }
}

impl Error for CheckboxToBoolError {}

#[derive(Debug)]
struct CheckboxOrnament {
    value: DynamicReader<CheckboxState>,
}

fn draw_checkbox(
    colors: &ButtonColors,
    value: CheckboxState,
    filled_color: Color,
    region: Rect<Px>,
    context: &mut GraphicsContext<'_, '_, '_, '_>,
) {
    let corners = context
        .get(&CheckboxCornerRadius)
        .into_px(context.gfx.scale())
        .ceil();
    let checkbox_size = region.size.width.min(region.size.height);

    let stroke_options = StrokeOptions::px_wide(
        context
            .get(&OutlineWidth)
            .into_px(context.gfx.scale())
            .ceil(),
    );

    let half_line = stroke_options.line_width / 2;

    let checkbox_rect = Rect::new(
        region.origin
            + Point::new(
                half_line,
                (region.size.height - checkbox_size) / 2 + half_line,
            ),
        Size::squared(checkbox_size - stroke_options.line_width),
    );

    match value {
        state @ (CheckboxState::Checked | CheckboxState::Indeterminant) => {
            if corners.is_zero() {
                context
                    .gfx
                    .draw_shape(&Shape::filled_rect(checkbox_rect, filled_color));
                if filled_color != colors.outline {
                    context.gfx.draw_shape(&Shape::stroked_rect(
                        checkbox_rect,
                        stroke_options.colored(colors.outline),
                    ));
                }
            } else {
                context.gfx.draw_shape(&Shape::filled_round_rect(
                    checkbox_rect,
                    corners,
                    filled_color,
                ));
                if filled_color != colors.outline {
                    context.gfx.draw_shape(&Shape::stroked_round_rect(
                        checkbox_rect,
                        corners,
                        stroke_options.colored(colors.outline),
                    ));
                }
            }
            let icon_area = checkbox_rect.inset(Lp::points(3).into_px(context.gfx.scale()));

            let center = icon_area.origin + icon_area.size / 2;
            let mut double_stroke = stroke_options;
            double_stroke.line_width *= 2;
            if matches!(state, CheckboxState::Checked) {
                context.gfx.draw_shape(
                    &PathBuilder::new(Point::new(icon_area.origin.x, center.y))
                        .line_to(Point::new(
                            icon_area.origin.x + icon_area.size.width / 4,
                            icon_area.origin.y + icon_area.size.height * 3 / 4,
                        ))
                        .line_to(Point::new(
                            icon_area.origin.x + icon_area.size.width,
                            icon_area.origin.y,
                        ))
                        .build()
                        .stroke(double_stroke.colored(colors.foreground)),
                );
            } else {
                context.gfx.draw_shape(
                    &PathBuilder::new(Point::new(icon_area.origin.x, center.y))
                        .line_to(Point::new(
                            icon_area.origin.x + icon_area.size.width,
                            center.y,
                        ))
                        .build()
                        .stroke(double_stroke.colored(colors.foreground)),
                );
            }
        }
        CheckboxState::Unchecked => {
            if corners.is_zero() {
                context
                    .gfx
                    .draw_shape(&Shape::filled_rect(checkbox_rect, colors.background));
                context.gfx.draw_shape(&Shape::stroked_rect(
                    checkbox_rect,
                    stroke_options.colored(colors.outline),
                ));
            } else {
                context.gfx.draw_shape(&Shape::filled_round_rect(
                    checkbox_rect,
                    corners,
                    colors.background,
                ));
                context.gfx.draw_shape(&Shape::stroked_round_rect(
                    checkbox_rect,
                    corners,
                    stroke_options.colored(colors.outline),
                ));
            }
        }
    }
}

impl Widget for CheckboxOrnament {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        let state = if context.enabled() {
            VisualState::Normal
        } else {
            VisualState::Disabled
        };
        let colors = Button::colors_for_transparent(state, context);
        draw_checkbox(
            &colors,
            self.value.get_tracking_redraw(context),
            context.get(&WidgetAccentColor),
            Rect::from(context.gfx.region().size),
            context,
        );
    }

    fn layout(
        &mut self,
        _available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let checkbox_size = context
            .get(&CheckboxSize)
            .into_upx(context.gfx.scale())
            .ceil();
        Size::squared(checkbox_size)
    }
}

/// A value that can be used as a checkbox.
pub trait Checkable: IntoDynamic<CheckboxState> + Sized {
    /// Returns a new checkbox using `self` as the value.
    fn into_checkbox(self) -> Checkbox {
        Checkbox::new(self.into_dynamic())
    }

    /// Returns a new checkbox using `self` as the value.
    fn to_checkbox(&self) -> Checkbox
    where
        Self: Clone,
    {
        self.clone().into_checkbox()
    }
}

impl<T> Checkable for T where T: IntoDynamic<CheckboxState> {}

define_components! {
    Checkbox {
        /// The size to render a [`Checkbox`] indicator.
        CheckboxSize(Dimension, "size", @LineHeight)
        /// The radius of the rounded corners to display on checkbox widgets.
        CheckboxCornerRadius(CornerRadii<Dimension>, "corner_radius", |context| {
            context.get(&CornerRadius).map(|r| r/2)
        })
    }
}
