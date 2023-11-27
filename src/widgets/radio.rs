//! A tri-state, labelable checkbox widget.
use std::fmt::Debug;
use std::panic::UnwindSafe;

use kludgine::figures::units::Lp;
use kludgine::figures::{Point, ScreenScale, Size};
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::DrawableExt;

use crate::context::{GraphicsContext, LayoutContext};
use crate::styles::components::{LineHeight, OutlineColor, WidgetAccentColor};
use crate::value::{Dynamic, DynamicReader, IntoDynamic, IntoValue, Value};
use crate::widget::{MakeWidget, Widget, WidgetInstance};
use crate::widgets::button::ButtonKind;
use crate::ConstraintLimit;

/// A labeled-widget that supports three states: Checked, Unchecked, and
/// Indeterminant
pub struct Radio<T> {
    /// The value this button represents.
    pub value: T,
    /// The state (value) of the checkbox.
    pub state: Dynamic<T>,
    /// The button kind to use as the basis for this radio. Radios default to
    /// [`ButtonKind::Transparent`].
    pub kind: Value<ButtonKind>,
    label: WidgetInstance,
}

impl<T> Radio<T> {
    /// Returns a new radio that sets `state` to `value` when pressed. `label`
    /// is drawn next to the checkbox and is also clickable to select the radio.
    pub fn new(value: T, state: impl IntoDynamic<T>, label: impl MakeWidget) -> Self {
        Self {
            value,
            state: state.into_dynamic(),
            kind: Value::Constant(ButtonKind::Transparent),
            label: label.make_widget(),
        }
    }

    /// Updates the button kind to use as the basis for this radio, and
    /// returns self.
    ///
    /// Radios default to [`ButtonKind::Transparent`].
    #[must_use]
    pub fn kind(mut self, kind: impl IntoValue<ButtonKind>) -> Self {
        self.kind = kind.into_value();
        self
    }
}

impl<T> MakeWidget for Radio<T>
where
    T: Clone + Debug + Eq + UnwindSafe + Send + 'static,
{
    fn make_widget(self) -> WidgetInstance {
        RadioOrnament {
            value: self.value.clone(),
            state: self.state.create_reader(),
        }
        .and(self.label)
        .into_columns()
        .into_button()
        .on_click(move |()| {
            self.state.set(self.value.clone());
        })
        .kind(self.kind)
        .make_widget()
    }
}

#[derive(Debug)]
struct RadioOrnament<T> {
    value: T,
    state: DynamicReader<T>,
}

impl<T> Widget for RadioOrnament<T>
where
    T: Debug + Eq + UnwindSafe + Send + 'static,
{
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let radio_size = context
            .gfx
            .region()
            .size
            .width
            .min(context.gfx.region().size.height);
        let vertical_center = context.gfx.region().size.height / 2;

        let stroke_options = StrokeOptions::lp_wide(Lp::points(2)).into_px(context.gfx.scale());
        context.redraw_when_changed(&self.state);
        let selected = self.state.map_ref(|state| state == &self.value);
        let color = context.get(&OutlineColor);
        let radius = radio_size / 2;
        context.gfx.draw_shape(
            Shape::stroked_circle(
                radius - stroke_options.line_width / 2,
                color,
                kludgine::Origin::Center,
                stroke_options.colored(color),
            )
            .translate_by(Point::new(radius, vertical_center)),
        );
        if selected {
            let color = context.get(&WidgetAccentColor);
            context.gfx.draw_shape(
                Shape::filled_circle(
                    radius - stroke_options.line_width * 2,
                    color,
                    kludgine::Origin::Center,
                )
                .translate_by(Point::new(radius, vertical_center)),
            );
        }
    }

    fn layout(
        &mut self,
        _available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<kludgine::figures::units::UPx> {
        let radio_size = context.get(&LineHeight).into_upx(context.gfx.scale()); // TODO create a component? Same as checkbox
        Size::squared(radio_size)
    }
}
