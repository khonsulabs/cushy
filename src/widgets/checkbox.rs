//! A tri-state, labelable checkbox widget.
use std::error::Error;
use std::fmt::Display;
use std::ops::Not;

use kludgine::figures::units::{Lp, Px};
use kludgine::figures::{IntoUnsigned, Point, Rect, ScreenScale, Size};
use kludgine::shapes::{PathBuilder, Shape, StrokeOptions};

use crate::context::{GraphicsContext, LayoutContext};
use crate::styles::components::{
    IntrinsicPadding, LineHeight, OutlineColor, TextColor, WidgetAccentColor,
};
use crate::value::{Dynamic, DynamicReader, IntoDynamic, IntoValue, Value};
use crate::widget::{MakeWidget, WidgetInstance, WidgetRef, WrappedLayout, WrapperWidget};
use crate::widgets::button::ButtonKind;
use crate::ConstraintLimit;

/// A labeled-widget that supports three states: Checked, Unchecked, and
/// Indeterminant
pub struct Checkbox {
    /// The state (value) of the checkbox.
    pub state: Dynamic<CheckboxState>,
    /// The button kind to use as the basis for this checkbox. Checkboxes
    /// default to [`ButtonKind::Transparent`].
    pub kind: Value<ButtonKind>,
    label: WidgetInstance,
}

impl Checkbox {
    /// Returns a new checkbox that updates `state` when clicked. `label` is
    /// drawn next to the checkbox and is also clickable to toggle the checkbox.
    ///
    /// `state` can also be a `Dynamic<bool>` if there is no need to represent
    /// an indeterminant state.
    pub fn new(state: impl IntoDynamic<CheckboxState>, label: impl MakeWidget) -> Self {
        Self {
            state: state.into_dynamic(),
            kind: Value::Constant(ButtonKind::Transparent),
            label: label.make_widget(),
        }
    }

    /// Updates the button kind to use as the basis for this checkbox, and
    /// returns self.
    ///
    /// Checkboxes default to [`ButtonKind::Transparent`].
    #[must_use]
    pub fn kind(mut self, kind: impl IntoValue<ButtonKind>) -> Self {
        self.kind = kind.into_value();
        self
    }
}

impl MakeWidget for Checkbox {
    fn make_widget(self) -> WidgetInstance {
        CheckboxLabel {
            value: self.state.create_reader(),
            label: WidgetRef::new(self.label),
        }
        .into_button()
        .on_click(move |()| {
            let mut value = self.state.lock();
            *value = !*value;
        })
        .kind(self.kind)
        .make_widget()
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
struct CheckboxLabel {
    value: DynamicReader<CheckboxState>,
    label: WidgetRef,
}

impl WrapperWidget for CheckboxLabel {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.label
    }

    fn position_child(
        &mut self,
        size: Size<Px>,
        _available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> WrappedLayout {
        let checkbox_size = context.get(&LineHeight).into_px(context.gfx.scale()); // TODO create a component?
        let padding = context.get(&IntrinsicPadding).into_px(context.gfx.scale());
        let label_inset = checkbox_size + padding * 2;
        let effective_height = size.height.max(label_inset);
        let size_with_checkbox =
            Size::new(size.width + label_inset + padding, effective_height).into_unsigned();
        WrappedLayout {
            child: Rect::new(
                Point::new(label_inset, Px::ZERO),
                Size::new(size.width, effective_height),
            ),
            size: size_with_checkbox,
        }
    }

    fn redraw_background(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let checkbox_size = context.get(&LineHeight).into_px(context.gfx.scale());
        let padding = context.get(&IntrinsicPadding).into_px(context.gfx.scale());
        let checkbox_rect = Rect::new(Point::squared(padding), Size::squared(checkbox_size));
        let stroke_options = StrokeOptions::lp_wide(Lp::points(2)).into_px(context.gfx.scale());
        match self.value.get_tracking_refresh(context) {
            state @ (CheckboxState::Checked | CheckboxState::Indeterminant) => {
                let color = context.get(&WidgetAccentColor);
                context
                    .gfx
                    .draw_shape(&Shape::filled_rect(checkbox_rect, color));
                let icon_area = checkbox_rect.inset(Lp::points(3).into_px(context.gfx.scale()));
                let text_color = context.get(&TextColor);
                let center = icon_area.origin + icon_area.size / 2;
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
                            .stroke(stroke_options.colored(text_color)),
                    );
                } else {
                    context.gfx.draw_shape(
                        &PathBuilder::new(Point::new(icon_area.origin.x, center.y))
                            .line_to(Point::new(
                                icon_area.origin.x + icon_area.size.width,
                                center.y,
                            ))
                            .build()
                            .stroke(stroke_options.colored(text_color)),
                    );
                }
            }
            CheckboxState::Unchecked => {
                let color = context.get(&OutlineColor);
                context.gfx.draw_shape(&Shape::stroked_rect(
                    checkbox_rect,
                    stroke_options.colored(color),
                ));
            }
        }
    }
}

/// A value that can be used as a checkbox.
pub trait Checkable: IntoDynamic<CheckboxState> + Sized {
    /// Returns a new checkbox using `self` as the value and `label`.
    fn into_checkbox(self, label: impl MakeWidget) -> Checkbox {
        Checkbox::new(self.into_dynamic(), label)
    }
}

impl<T> Checkable for T where T: IntoDynamic<CheckboxState> {}
