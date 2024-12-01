//! A labeled widget with a circular indicator representing a value.
use std::fmt::Debug;

use figures::units::Px;
use figures::{Point, Rect, Round, ScreenScale, Size};
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::{Color, DrawableExt};

use super::button::{ButtonActiveBackground, ButtonDisabledBackground, ButtonHoverBackground};
use super::indicator::{Indicator, IndicatorBehavior, IndicatorState};
use crate::animation::{LinearInterpolate, ZeroToOne};
use crate::context::{GraphicsContext, LayoutContext, Trackable, WidgetContext};
use crate::styles::components::{
    FocusColor, LineHeight, OutlineColor, OutlineWidth, WidgetAccentColor, WidgetBackground,
};
use crate::styles::{ColorExt, Dimension};
use crate::value::{Destination, Dynamic, DynamicReader, IntoDynamic, IntoValue, Source, Value};
use crate::widget::{
    Baseline, MakeWidget, MakeWidgetWithTag, Widget, WidgetInstance, WidgetLayout,
};
use crate::widgets::button::ButtonKind;
use crate::ConstraintLimit;

/// A labeled widget with a circular indicator representing a value.
pub struct Radio<T> {
    /// The value this button represents.
    pub value: T,
    /// The state (value) of the radio.
    pub state: Dynamic<T>,
    /// The button kind to use as the basis for this radio. Radios default to
    /// [`ButtonKind::Transparent`].
    pub kind: Option<Value<ButtonKind>>,
    focusable: bool,
    label: Option<WidgetInstance>,
}

impl<T> Radio<T> {
    /// Returns a new radio that sets `state` to `value` when pressed.
    pub fn new(value: T, state: impl IntoDynamic<T>) -> Self {
        Self {
            value,
            state: state.into_dynamic(),
            kind: None,
            label: None,
            focusable: true,
        }
    }

    /// Draws `label` next to the radio indicator and is also clickable to
    /// select the radio.
    #[must_use]
    pub fn labelled_by(mut self, label: impl MakeWidget) -> Self {
        self.label = Some(label.make_widget());
        self
    }

    /// Updates the button kind to use as the basis for this radio, and
    /// returns self.
    #[must_use]
    pub fn kind(mut self, kind: impl IntoValue<ButtonKind>) -> Self {
        self.kind = Some(kind.into_value());
        self
    }
}

impl<T> MakeWidgetWithTag for Radio<T>
where
    T: Clone + Debug + PartialEq + Send + 'static,
{
    fn make_with_tag(self, id: crate::widget::WidgetTag) -> WidgetInstance {
        if let Some(kind) = self.kind {
            let adornment = RadioOrnament {
                value: self.value.clone(),
                state: self.state.create_reader(),
            };
            let button_label = if let Some(label) = self.label {
                adornment.and(label).into_columns().make_widget()
            } else {
                adornment.make_widget()
            };

            let mut button = button_label
                .into_button()
                .on_click(move |_| {
                    self.state.set(self.value.clone());
                })
                .kind(kind);

            if !self.focusable {
                button = button.prevent_focus();
            }
            button.make_with_tag(id)
        } else {
            let mut indicator = Indicator::new(RadioIndicator {
                state: self.state,
                value: self.value,
            })
            .focusable(self.focusable);
            if let Some(label) = self.label {
                indicator = indicator.labelled_by(label);
            }
            indicator.make_with_tag(id)
        }
    }
}

#[derive(LinearInterpolate, Debug, Eq, PartialEq, Clone, Copy)]
struct RadioColors {
    fill: Color,
    outline: Color,
}

impl RadioColors {
    fn for_state(indicator: IndicatorState, context: &mut WidgetContext<'_>) -> Self {
        let fill = if indicator.hovered {
            if indicator.active {
                context.get(&ButtonActiveBackground)
            } else {
                context.get(&ButtonHoverBackground)
            }
        } else {
            context.get(&WidgetBackground)
        };

        let outline = if indicator.focused {
            let focus_color = context.get(&FocusColor);
            if indicator.hovered {
                focus_color.darken_by(ZeroToOne::new(0.8))
            } else {
                focus_color
            }
        } else if !context.enabled() {
            context.get(&ButtonDisabledBackground)
        } else if indicator.active {
            context.get(&OutlineColor).darken_by(ZeroToOne::new(0.7))
        } else if indicator.hovered {
            context.get(&OutlineColor).darken_by(ZeroToOne::new(0.8))
        } else {
            context.get(&OutlineColor)
        };
        Self { fill, outline }
    }
}

#[derive(Debug)]
struct RadioIndicator<T> {
    value: T,
    state: Dynamic<T>,
}

impl<T> RadioIndicator<T>
where
    T: Clone + PartialEq + Debug + Send + 'static,
{
    fn is_selected(&self) -> bool {
        self.state.map_ref(|state| state == &self.value)
    }
}

impl<T> IndicatorBehavior for RadioIndicator<T>
where
    T: Clone + PartialEq + Debug + Send + 'static,
{
    type Colors = RadioColors;

    fn size(&self, context: &mut GraphicsContext<'_, '_, '_, '_>) -> WidgetLayout {
        let size = Size::squared(context.get(&RadioSize).into_upx(context.gfx.scale()).ceil());
        let outline_width = context
            .get(&OutlineWidth)
            .into_upx(context.gfx.scale())
            .ceil();

        WidgetLayout {
            size,
            baseline: Baseline::from(size.height - outline_width * 2),
        }
    }

    fn desired_colors(
        &mut self,
        context: &mut WidgetContext<'_>,
        indicator: IndicatorState,
    ) -> Self::Colors {
        RadioColors::for_state(indicator, context)
    }

    fn activate(&mut self) {
        self.state.set(self.value.clone());
    }

    fn empty(&self) -> bool {
        !self.is_selected()
    }

    fn will_be_empty_if_activated(&self) -> bool {
        false
    }

    fn render(
        &mut self,
        is_active: bool,
        colors: &Self::Colors,
        selected_color: Color,
        region: Rect<Px>,
        context: &mut GraphicsContext<'_, '_, '_, '_>,
    ) {
        self.state.redraw_when_changed(context);
        let state = self.is_selected();
        let state = is_active || state;
        draw_radio(state, *colors, selected_color, region, context);
    }
}

fn draw_radio(
    selected: bool,
    colors: RadioColors,
    selected_color: Color,
    region: Rect<Px>,
    context: &mut GraphicsContext<'_, '_, '_, '_>,
) {
    let radio_size = region.size.width.min(region.size.height);
    let vertical_center = region.size.height / 2 + region.origin.y;

    let stroke_options = StrokeOptions::px_wide(
        context
            .get(&OutlineWidth)
            .into_px(context.gfx.scale())
            .ceil(),
    );
    let radius = radio_size / 2;
    context.gfx.draw_shape(
        Shape::stroked_circle(
            radius - stroke_options.line_width / 2,
            kludgine::Origin::Center,
            stroke_options.colored(colors.outline),
        )
        .translate_by(Point::new(radius + region.origin.x, vertical_center)),
    );
    if selected {
        context.gfx.draw_shape(
            Shape::filled_circle(
                radius - stroke_options.line_width * 2,
                selected_color,
                kludgine::Origin::Center,
            )
            .translate_by(Point::new(radius + region.origin.x, vertical_center)),
        );
    }
}

#[derive(Debug)]
struct RadioOrnament<T> {
    value: T,
    state: DynamicReader<T>,
}

impl<T> Widget for RadioOrnament<T>
where
    T: Debug + PartialEq + Send + 'static,
{
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        context.redraw_when_changed(&self.state);
        let selected = self.state.map_ref(|state| state == &self.value);
        let colors = RadioColors::for_state(
            IndicatorState {
                hovered: false,
                active: false,
                focused: false,
                enabled: context.enabled(),
            },
            context,
        );
        draw_radio(
            selected,
            colors,
            context.get(&WidgetAccentColor),
            Rect::from(context.gfx.region().size),
            context,
        );
    }

    fn layout(
        &mut self,
        _available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WidgetLayout {
        let radio_size = context.get(&RadioSize).into_upx(context.gfx.scale());
        Size::squared(radio_size).into()
    }
}

define_components! {
    Radio {
        /// The size to render a [`Radio`] indicator.
        RadioSize(Dimension, "size", @LineHeight)
    }
}
