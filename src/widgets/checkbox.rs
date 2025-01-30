//! A tri-state, labelable checkbox widget.
use std::error::Error;
use std::fmt::{Debug, Display};
use std::ops::Not;

use figures::units::{Lp, Px, UPx};
use figures::{Point, Rect, Round, ScreenScale, Size, Zero};
use kludgine::shapes::{CornerRadii, PathBuilder, Shape, StrokeOptions};
use kludgine::Color;

use super::button::{
    ButtonActiveBackground, ButtonActiveForeground, ButtonBackground, ButtonDisabledBackground,
    ButtonHoverBackground, ButtonHoverForeground,
};
use super::indicator::{Indicator, IndicatorBehavior, IndicatorState};
use crate::animation::{LinearInterpolate, ZeroToOne};
use crate::context::{GraphicsContext, LayoutContext, WidgetContext};
use crate::reactive::value::{
    Destination, Dynamic, DynamicReader, IntoDynamic, IntoValue, Source, Value,
};
use crate::styles::components::{
    CornerRadius, FocusColor, LineHeight, OutlineColor, OutlineWidth, TextColor, VerticalAlignment,
    WidgetAccentColor, WidgetBackground,
};
use crate::styles::{ColorExt, Dimension, VerticalAlign};
use crate::widget::{MakeWidget, MakeWidgetWithTag, Widget, WidgetInstance};
use crate::widgets::button::ButtonKind;
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
    /// This causes the checkbox to become a regular
    /// [`Button`](crate::widgets::Button), which may be desireable for layout
    /// and/or visual consistency purposes.
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
                // TODO Set this to Baseline.
                adornment
                    .and(label)
                    .into_columns()
                    .with(&VerticalAlignment, VerticalAlign::Center)
                    .make_widget()
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
            let mut indicator =
                Indicator::new(CheckboxIndicator { state: self.state }).focusable(self.focusable);
            if let Some(label) = self.label {
                indicator = indicator.labelled_by(label);
            }
            indicator
                .make_with_tag(id)
                // TODO Set this to Baseline.
                .with(&VerticalAlignment, VerticalAlign::Center)
                .make_widget()
        }
    }
}

#[derive(Debug)]
struct CheckboxIndicator {
    state: Dynamic<CheckboxState>,
}

#[derive(LinearInterpolate, Debug, Eq, PartialEq, Clone, Copy)]
struct CheckboxColors {
    foreground: Color,
    fill: Color,
    outline: Color,
}

impl CheckboxColors {
    fn for_state(
        state: CheckboxState,
        indicator: IndicatorState,
        context: &mut WidgetContext<'_>,
    ) -> Self {
        let is_empty = CheckboxState::Unchecked == if indicator.active { !state } else { state };
        let (fill, foreground) = if indicator.hovered {
            if indicator.active {
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

        let outline = if indicator.focused {
            if is_empty {
                let focus_color = context.get(&FocusColor);
                if indicator.hovered {
                    focus_color.darken_by(ZeroToOne::new(0.8))
                } else {
                    focus_color
                }
            } else {
                let outline_color = context.get(&OutlineColor);
                if indicator.hovered {
                    outline_color.darken_by(ZeroToOne::new(0.8))
                } else {
                    outline_color
                }
            }
        } else if !context.enabled() {
            context.get(&ButtonDisabledBackground)
        } else if indicator.active {
            context.get(&ButtonActiveBackground)
        } else if indicator.hovered {
            context
                .get(&WidgetAccentColor)
                .darken_by(ZeroToOne::new(0.8))
        } else if is_empty {
            context.get(&ButtonBackground)
        } else {
            context.get(&WidgetAccentColor)
        };
        Self {
            foreground,
            fill,
            outline,
        }
    }
}

impl IndicatorBehavior for CheckboxIndicator {
    type Colors = CheckboxColors;

    fn size(&self, context: &mut GraphicsContext<'_, '_, '_, '_>) -> Size<UPx> {
        Size::squared(
            context
                .get(&CheckboxSize)
                .into_upx(context.gfx.scale())
                .ceil(),
        )
    }

    fn desired_colors(
        &mut self,
        context: &mut WidgetContext<'_>,
        indicator: IndicatorState,
    ) -> Self::Colors {
        let state = self.state.get_tracking_redraw(context);
        CheckboxColors::for_state(state, indicator, context)
    }

    fn activate(&mut self) {
        self.state.toggle();
    }

    fn empty(&self) -> bool {
        self.state.get() == CheckboxState::Unchecked
    }

    fn will_be_empty_if_activated(&self) -> bool {
        !self.state.get() == CheckboxState::Unchecked
    }

    fn render(
        &mut self,
        is_active: bool,
        colors: &Self::Colors,
        selected_color: Color,
        region: Rect<Px>,
        context: &mut GraphicsContext<'_, '_, '_, '_>,
    ) {
        let state = self.state.get_tracking_redraw(context);
        let state = if is_active { !state } else { state };
        draw_checkbox(state, colors, selected_color, region, context);
    }
}

fn draw_checkbox(
    state: CheckboxState,
    colors: &CheckboxColors,
    selected_color: Color,
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

    match state {
        state @ (CheckboxState::Checked | CheckboxState::Indeterminant) => {
            if corners.is_zero() {
                context
                    .gfx
                    .draw_shape(&Shape::filled_rect(checkbox_rect, selected_color));
                if selected_color != colors.outline {
                    context.gfx.draw_shape(&Shape::stroked_rect(
                        checkbox_rect,
                        stroke_options.colored(colors.outline),
                    ));
                }
            } else {
                context.gfx.draw_shape(&Shape::filled_round_rect(
                    checkbox_rect,
                    corners,
                    selected_color,
                ));
                if selected_color != colors.outline {
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
                    .draw_shape(&Shape::filled_rect(checkbox_rect, colors.fill));
                context.gfx.draw_shape(&Shape::stroked_rect(
                    checkbox_rect,
                    stroke_options.colored(colors.outline),
                ));
            } else {
                context.gfx.draw_shape(&Shape::filled_round_rect(
                    checkbox_rect,
                    corners,
                    colors.fill,
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

impl Widget for CheckboxOrnament {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        let state = self.value.get_tracking_redraw(context);
        let colors = CheckboxColors::for_state(
            state,
            IndicatorState {
                hovered: false,
                active: false,
                focused: false,
                enabled: context.enabled(),
            },
            context,
        );
        draw_checkbox(
            state,
            &colors,
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
