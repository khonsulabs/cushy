use std::borrow::Cow;
use std::fmt::Debug;
use std::panic::UnwindSafe;

use kludgine::app::winit::event::{DeviceId, MouseButton};
use kludgine::figures::units::{Lp, Px, UPx};
use kludgine::figures::{
    FloatConversion, FromComponents, IntoComponents, IntoSigned, IntoUnsigned, Point, Ranged, Rect,
    ScreenScale, Size,
};
use kludgine::shapes::Shape;
use kludgine::{Color, Origin};

use crate::animation::{LinearInterpolate, PercentBetween};
use crate::context::{EventContext, GraphicsContext, LayoutContext, WidgetContext};
use crate::styles::{ComponentDefinition, ComponentName, Dimension, Group, NamedComponent};
use crate::value::{Dynamic, IntoDynamic, IntoValue, Value};
use crate::widget::{EventHandling, Widget, HANDLED};
use crate::ConstraintLimit;

/// A widget that allows sliding between two values.
#[derive(Debug, Clone)]
pub struct Slider<T> {
    /// The current value.
    pub value: Dynamic<T>,
    /// The minimum value represented by this slider.
    pub minimum: Value<T>,
    /// The maximum value represented by this slider.
    pub maximum: Value<T>,
    knob_size: UPx,
    horizontal: bool,
    rendered_size: Px,
}

impl<T> Slider<T>
where
    T: Ranged,
{
    /// Returns a new slider over `value` using the types full range.
    #[must_use]
    pub fn from_value(value: impl IntoDynamic<T>) -> Self {
        Self::new(value, T::MIN, T::MAX)
    }
}

impl<T> Slider<T> {
    /// Returns a new slider using `value` as the slider's value, keeping the
    /// value between `min` and `max`.
    #[must_use]
    pub fn new(value: impl IntoDynamic<T>, min: impl IntoValue<T>, max: impl IntoValue<T>) -> Self {
        Self {
            value: value.into_dynamic(),
            minimum: min.into_value(),
            maximum: max.into_value(),
            knob_size: UPx(0),
            horizontal: true,
            rendered_size: Px(0),
        }
    }

    /// Sets the maximum value of this slider to `max` and returns self.
    #[must_use]
    pub fn maximum(mut self, max: impl IntoValue<T>) -> Self {
        self.maximum = max.into_value();
        self
    }

    /// Sets the minimum value of this slider to `min` and returns self.
    #[must_use]
    pub fn minimum(mut self, min: impl IntoValue<T>) -> Self {
        self.minimum = min.into_value();
        self
    }

    fn draw_track(&mut self, spec: &TrackSpec, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        if self.horizontal {
            self.rendered_size = spec.size.width;
        } else {
            self.rendered_size = spec.size.height;
        }
        let track_length = self.rendered_size - spec.knob_size;
        let value_location = (track_length) * spec.percent + spec.half_knob;

        let half_track = spec.track_size / 2;
        // Draw the track
        if value_location > spec.half_knob {
            context.gfx.draw_shape(
                &Shape::filled_rect(
                    Rect::new(
                        flipped(
                            !self.horizontal,
                            Point::new(spec.half_knob, spec.half_knob - half_track),
                        ),
                        flipped(!self.horizontal, Size::new(value_location, spec.track_size)),
                    ),
                    spec.track_color,
                ),
                Point::default(),
                None,
                None,
            );
        }

        if value_location < track_length {
            context.gfx.draw_shape(
                &Shape::filled_rect(
                    Rect::new(
                        flipped(
                            !self.horizontal,
                            Point::new(value_location, spec.half_knob - half_track),
                        ),
                        flipped(
                            !self.horizontal,
                            Size::new(
                                track_length - value_location + spec.half_knob,
                                spec.track_size,
                            ),
                        ),
                    ),
                    spec.inactive_track_color,
                ),
                Point::default(),
                None,
                None,
            );
        }

        // Draw the knob
        context.gfx.draw_shape(
            &Shape::filled_circle(spec.half_knob, spec.knob_color, Origin::Center),
            flipped(!self.horizontal, Point::new(value_location, spec.half_knob)),
            None,
            None,
        );
    }
}

impl<T> Slider<T>
where
    T: LinearInterpolate + Clone,
{
    fn update_from_click(&mut self, position: Point<Px>) {
        let position = if self.horizontal {
            position.x
        } else {
            position.y
        };
        let position = position.clamp(Px(0), self.rendered_size);
        let percent = position.into_float() / self.rendered_size.into_float();
        let min = self.minimum.get();
        let max = self.maximum.get();
        self.value.update(min.lerp(&max, percent));
    }
}

impl<T> Widget for Slider<T>
where
    T: Clone
        + Debug
        + PartialOrd
        + LinearInterpolate
        + PercentBetween
        + UnwindSafe
        + Send
        + 'static,
{
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let styles =
            context.query_styles(&[&TrackColor, &InactiveTrackColor, &KnobColor, &TrackSize]);
        let track_color = styles.get(&TrackColor, context);
        let inactive_track_color = styles.get(&InactiveTrackColor, context);
        let knob_color = styles.get(&KnobColor, context);
        let knob_size = self.knob_size.into_signed();
        let track_size = styles
            .get(&TrackSize, context)
            .into_px(context.gfx.scale())
            .min(knob_size);

        let half_knob = knob_size / 2;

        let mut value = self.value.get_tracked(context);
        let min = self.minimum.get_tracked(context);
        let mut max = self.maximum.get_tracked(context);

        if max < min {
            self.maximum.map_mut(|max| *max = min.clone());
            max = min.clone();
        }
        let mut value_clamped = false;
        if value < min {
            value_clamped = true;
            value = min.clone();
        } else if value > max {
            value_clamped = true;
            value = max.clone();
        }

        if value_clamped {
            self.value.map_mut(|v| *v = value.clone());
        }

        let percent = value.percent_between(&min, &max);

        let size = context.gfx.region().size;
        self.horizontal = size.width >= size.height;

        self.draw_track(
            &TrackSpec {
                size,
                percent: *percent,
                half_knob,
                knob_size,
                track_size,
                knob_color,
                track_color,
                inactive_track_color,
            },
            context,
        );
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let styles = context.query_styles(&[&KnobSize, &MinimumSliderSize]);
        self.knob_size = styles
            .get(&KnobSize, context)
            .into_px(context.gfx.scale())
            .into_unsigned();
        let minimum_size = styles
            .get(&MinimumSliderSize, context)
            .into_px(context.gfx.scale())
            .into_unsigned();

        match (available_space.width, available_space.height) {
            (ConstraintLimit::Known(width), ConstraintLimit::Known(height)) => {
                // This comparison is done such that if width == height, we end
                // up with a horizontal slider.
                if width < height {
                    // Vertical slider
                    Size::new(self.knob_size, height.max(minimum_size))
                } else {
                    // Horizontal slider
                    Size::new(width.max(minimum_size), self.knob_size)
                }
            }
            (ConstraintLimit::Known(width), ConstraintLimit::ClippedAfter(_)) => {
                Size::new(width.max(minimum_size), self.knob_size)
            }
            (ConstraintLimit::ClippedAfter(_), ConstraintLimit::Known(height)) => {
                Size::new(self.knob_size, height.max(minimum_size))
            }
            (ConstraintLimit::ClippedAfter(width), ConstraintLimit::ClippedAfter(_)) => {
                // When we have no limit on our, we still want to be draggable.
                // Since we have no limit in both directions, we have to make a
                // choice: horizontal or vertical. It seems to @ecton at the
                // time of writing this that when there is no intent from the
                // user of the slider, a horizontal slider is expected. So, we
                // set the minimum measurement based on a horizontal
                // orientation.
                Size::new(width.min(minimum_size), self.knob_size)
            }
        }
    }

    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        _context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        self.update_from_click(location);
        HANDLED
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        _context: &mut EventContext<'_, '_>,
    ) {
        self.update_from_click(location);
    }
}

struct TrackSpec {
    size: Size<Px>,
    percent: f32,
    half_knob: Px,
    knob_size: Px,
    track_size: Px,
    knob_color: Color,
    track_color: Color,
    inactive_track_color: Color,
}

fn flipped<T, Unit>(flip: bool, value: T) -> T
where
    T: IntoComponents<Unit> + FromComponents<Unit>,
{
    if flip {
        let (a, b) = value.into_components();
        T::from_components((b, a))
    } else {
        value
    }
}

/// The size of the track that the knob of a [`Slider`] traversesq.
pub struct TrackSize;

impl ComponentDefinition for TrackSize {
    type ComponentType = Dimension;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Self::ComponentType {
        Dimension::Lp(Lp::points(5))
    }
}

impl NamedComponent for TrackSize {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::new(Group::new("Slider"), "track_size"))
    }
}

/// The width and height of the draggable portion of a [`Slider`].
pub struct KnobSize;

impl ComponentDefinition for KnobSize {
    type ComponentType = Dimension;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Self::ComponentType {
        Dimension::Lp(Lp::points(14))
    }
}

impl NamedComponent for KnobSize {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::new(Group::new("Slider"), "knob_size"))
    }
}

/// The minimum length of the slidable dimension.
pub struct MinimumSliderSize;

impl ComponentDefinition for MinimumSliderSize {
    type ComponentType = Dimension;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Self::ComponentType {
        Dimension::Lp(Lp::points(14))
    }
}

impl NamedComponent for MinimumSliderSize {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::new(Group::new("Slider"), "minimum_size"))
    }
}

/// The color of the draggable portion of the knob.
pub struct KnobColor;

impl ComponentDefinition for KnobColor {
    type ComponentType = Color;

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Self::ComponentType {
        context.theme().primary.color
    }
}

impl NamedComponent for KnobColor {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::new(Group::new("Slider"), "knob_color"))
    }
}

/// The color of the track that the knob rests on.
pub struct TrackColor;

impl ComponentDefinition for TrackColor {
    type ComponentType = Color;

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Self::ComponentType {
        context.theme().primary.color
    }
}

impl NamedComponent for TrackColor {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::new(Group::new("Slider"), "track_color"))
    }
}

/// The color of the draggable portion of the knob.
pub struct InactiveTrackColor;

impl ComponentDefinition for InactiveTrackColor {
    type ComponentType = Color;

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Self::ComponentType {
        context.theme().surface.highest_container // TODO this is the same as ButtonBackground. This should be abstracted into its own component both can depend on.
    }
}

impl NamedComponent for InactiveTrackColor {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::new(
            Group::new("Slider"),
            "inactive_track_color",
        ))
    }
}
