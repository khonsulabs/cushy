//! A widget that allows a user to "slide" between values.
use std::fmt::Debug;
use std::mem;
use std::ops::RangeInclusive;
use std::panic::UnwindSafe;

use intentional::{Assert, Cast as _};
use kludgine::app::winit::event::{DeviceId, MouseButton, MouseScrollDelta, TouchPhase};
use kludgine::app::winit::keyboard::{Key, NamedKey};
use kludgine::figures::units::{Lp, Px, UPx};
use kludgine::figures::{
    FloatConversion, FromComponents, IntoComponents, IntoSigned, Point, Ranged, Rect, Round,
    ScreenScale, Size,
};
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::{Color, DrawableExt, Origin};

use crate::animation::{LinearInterpolate, PercentBetween, ZeroToOne};
use crate::context::{EventContext, GraphicsContext, LayoutContext};
use crate::styles::components::{
    AutoFocusableControls, OpaqueWidgetColor, OutlineColor, WidgetAccentColor,
};
use crate::styles::{Dimension, HorizontalOrder, VerticalOrder, VisualOrder};
use crate::value::{Dynamic, IntoDynamic, IntoValue, Value};
use crate::widget::{EventHandling, Widget, HANDLED, IGNORED};
use crate::ConstraintLimit;

/// A widget that allows sliding between two values.
#[derive(Debug, Clone)]
pub struct Slider<T>
where
    T: SliderValue,
{
    /// The current value.
    pub value: Dynamic<T>,
    /// The minimum value represented by this slider.
    pub minimum: Value<T::Value>,
    /// The maximum value represented by this slider.
    pub maximum: Value<T::Value>,
    /// The percentage to step when advancing the slider using alternative
    /// inputs (e.g, keyboard/mousewheel).
    ///
    /// The widget will use this as a starting value, but will continue to step
    /// by this amount until a new unique value is obtained from linear
    /// interpolation.
    ///
    /// This defaults to `0.05`/5%.
    pub step: Value<ZeroToOne>,
    knob_visible: bool,
    interactive: bool,
    knob_size: UPx,
    horizontal: bool,
    rendered_size: Px,
    focused_knob: Option<Knob>,
    previous_focus: Option<Knob>,
    mouse_buttons_down: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Knob {
    Start,
    End,
}

impl<T> Slider<T>
where
    T: SliderValue,
    T::Value: Ranged,
{
    /// Returns a new slider over `value` using the types full range.
    #[must_use]
    pub fn from_value(value: impl IntoDynamic<T>) -> Self {
        Self::new(value, <T::Value>::MIN, <T::Value>::MAX)
    }
}

impl<T> Slider<T>
where
    T: SliderValue,
{
    /// Returns a new slider using `value` as the slider's value, keeping the
    /// value between `min` and `max`.
    #[must_use]
    pub fn new(
        value: impl IntoDynamic<T>,
        min: impl IntoValue<T::Value>,
        max: impl IntoValue<T::Value>,
    ) -> Self {
        Self {
            value: value.into_dynamic(),
            minimum: min.into_value(),
            maximum: max.into_value(),
            knob_visible: true,
            interactive: true,
            step: Value::Constant(ZeroToOne::new(0.05)),
            knob_size: UPx::ZERO,
            horizontal: true,
            rendered_size: Px::ZERO,
            focused_knob: None,
            mouse_buttons_down: 0,
            previous_focus: None,
        }
    }

    /// Sets the maximum value of this slider to `max` and returns self.
    #[must_use]
    pub fn maximum(mut self, max: impl IntoValue<T::Value>) -> Self {
        self.maximum = max.into_value();
        self
    }

    /// Sets the minimum value of this slider to `min` and returns self.
    #[must_use]
    pub fn minimum(mut self, min: impl IntoValue<T::Value>) -> Self {
        self.minimum = min.into_value();
        self
    }

    /// The percentage to step when advancing the slider using alternative
    /// inputs (e.g, keyboard/mousewheel).
    ///
    /// The widget will use this as a starting value, but will continue to step
    /// by this amount until a new unique value is obtained from linear
    /// interpolation.
    ///
    /// This defaults to `0.05`/5%.
    #[must_use]
    pub fn step_by(mut self, percent: impl IntoValue<ZeroToOne>) -> Self {
        self.step = percent.into_value();
        self
    }

    /// Updates this slider to not show knobs and returns self.
    ///
    /// This also prevents the slider from being focused.
    #[must_use]
    pub fn knobless(mut self) -> Self {
        self.knob_visible = false;
        self
    }

    /// Updates this slider to ignore all user input and returns self.
    #[must_use]
    pub fn non_interactive(mut self) -> Self {
        self.interactive = false;
        self
    }

    fn draw_track(&mut self, spec: &TrackSpec, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        if self.horizontal {
            self.rendered_size = spec.size.width;
        } else {
            self.rendered_size = spec.size.height;
        }
        let half_focus_ring =
            spec.if_knobbed(|| (Lp::points(2).into_px(context.gfx.scale()) / 2).ceil());
        let focus_ring = half_focus_ring * 2;
        let track_length = self.rendered_size - spec.if_knobbed(|| spec.knob_size - focus_ring);
        let (start, end) = if let Some(end) = spec.end {
            (track_length * spec.start, track_length * end)
        } else {
            (Px::ZERO, track_length * spec.start)
        };
        let inset = Point::squared(half_focus_ring);

        let half_track = spec.track_size / 2;
        let start_inset = (spec.half_knob - half_track).max(Px::ZERO);
        // Draw the track
        if start > 0 {
            context.gfx.draw_shape(
                Shape::filled_round_rect(
                    Rect::new(
                        flipped(!self.horizontal, Point::new(start_inset, start_inset)),
                        flipped(!self.horizontal, Size::new(start, spec.track_size)),
                    ),
                    half_track,
                    spec.inactive_track_color,
                )
                .translate_by(inset),
            );
        }
        if end < track_length {
            context.gfx.draw_shape(
                Shape::filled_round_rect(
                    Rect::new(
                        flipped(
                            !self.horizontal,
                            Point::new(end + spec.if_knobbed(|| spec.half_knob), start_inset),
                        ),
                        flipped(
                            !self.horizontal,
                            Size::new(
                                track_length - end + spec.if_knobbed(|| half_track),
                                spec.track_size,
                            ),
                        ),
                    ),
                    half_track,
                    spec.inactive_track_color,
                )
                .translate_by(inset),
            );
        }

        if start != end {
            context.gfx.draw_shape(
                Shape::filled_round_rect(
                    Rect::new(
                        flipped(
                            !self.horizontal,
                            Point::new(
                                start + spec.if_knobbed(|| spec.half_knob - half_track),
                                start_inset,
                            ),
                        ),
                        flipped(
                            !self.horizontal,
                            Size::new(
                                end - start + spec.if_knobbed(|| spec.track_size),
                                spec.track_size,
                            ),
                        ),
                    ),
                    half_track,
                    spec.track_color,
                )
                .translate_by(inset),
            );
        }

        // Draw the knob
        if spec.knob_size > 0 {
            let focus = context.focused().then_some(self.focused_knob).flatten();
            self.draw_knobs(
                flipped(
                    !self.horizontal,
                    Point::new(end + spec.half_knob, spec.half_knob) + inset,
                ),
                spec.end.map(|_| {
                    flipped(
                        !self.horizontal,
                        Point::new(start + spec.half_knob, spec.half_knob) + inset,
                    )
                }),
                focus,
                focus_ring,
                spec,
                context,
            );
            // let this_knob_role = if spec.end.is_some() {
            //     Knob::End
            // } else {
            //     Knob::Start
            // };
            // self.draw_knob(
            //     flipped(
            //         !self.horizontal,
            //         Point::new(end + spec.half_knob, spec.half_knob) + inset,
            //     ),
            //     focused && self.focused_knob == Some(this_knob_role),
            //     focus_ring,
            //     spec,
            //     context,
            // );

            // if spec.end.is_some() {
            //     self.draw_knob(
            //         flipped(
            //             !self.horizontal,
            //             Point::new(start + spec.half_knob, spec.half_knob) + inset,
            //         ),
            //         focused && matches!(self.focused_knob, Some(Knob::Start)),
            //         focus_ring,
            //         spec,
            //         context,
            //     );
            // }
        }
    }

    fn draw_knobs(
        &mut self,
        end_knob: Point<Px>,
        start_knob: Option<Point<Px>>,
        focus: Option<Knob>,
        focus_ring_width: Px,
        spec: &TrackSpec,
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
    ) {
        let (a, a_is_focused, b) = match (start_knob, focus) {
            (Some(start_knob), Some(Knob::Start)) => (end_knob, false, Some((start_knob, true))),
            (Some(start_knob), focus) => (start_knob, false, Some((end_knob, focus.is_some()))),
            (None, focus) => (end_knob, focus.is_some(), None),
        };

        self.draw_knob(a, a_is_focused, focus_ring_width, spec, context);
        if let Some((b, b_is_focused)) = b {
            self.draw_knob(b, b_is_focused, focus_ring_width, spec, context);
        }
    }

    fn draw_knob(
        &mut self,
        knob_center: Point<Px>,
        is_focused: bool,
        focus_ring_width: Px,
        spec: &TrackSpec,
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
    ) {
        context.gfx.draw_shape(
            Shape::filled_circle(spec.half_knob, spec.knob_color, Origin::Center)
                .translate_by(flipped(!self.horizontal, knob_center)),
        );

        if is_focused {
            let focus_color = context.get(&OutlineColor);
            context.gfx.draw_shape(
                Shape::stroked_circle(
                    spec.half_knob,
                    focus_color,
                    Origin::Center,
                    StrokeOptions::px_wide(focus_ring_width),
                )
                .translate_by(knob_center),
            );
        }
    }
}

impl<T> Slider<T>
where
    T: SliderValue,
{
    fn update_from_click(&mut self, position: Point<Px>, previous_focus: Option<Knob>) {
        let knob_size = self.knob_size.into_signed();
        let position = if self.horizontal {
            position.x - knob_size / 2
        } else {
            position.y - knob_size / 2
        };
        let track_width = self.rendered_size - knob_size;
        let position = position.clamp(Px::ZERO, track_width);
        let percent = position.into_float() / track_width.into_float();

        let min = self.minimum.get();
        let max = self.maximum.get();
        let value = min.lerp(&max, percent);
        let (mut start, mut opt_end) = T::into_parts(self.value.get());
        if let Some(end) = &opt_end {
            let knob = if let Some(knob) = self.focused_knob {
                knob
            } else {
                // Check if the click is overlapping either knob
                let start_percent = start.percent_between(&min, &max);
                let end_percent = end.percent_between(&min, &max);
                let knob_width_as_percent =
                    self.knob_size.into_float() / 2. / track_width.into_float();
                let start_delta = percent - *start_percent;
                let end_delta = *end_percent - percent;
                let on_overlapping_knobs =
                    end_delta <= knob_width_as_percent && start_delta <= knob_width_as_percent;
                if let (true, Some(previous)) = (on_overlapping_knobs, previous_focus) {
                    previous
                } else if start_delta < end_delta {
                    Knob::Start
                } else {
                    Knob::End
                }
            };
            match knob {
                Knob::Start => {
                    if &value <= end {
                        start = value;
                    } else {
                        start = end.clone();
                    }
                }
                Knob::End => {
                    if value >= start {
                        opt_end = Some(value);
                    } else {
                        opt_end = Some(start.clone());
                    }
                }
            }
            self.focused_knob = Some(knob);
        } else {
            start = value;
            self.focused_knob = Some(Knob::Start);
        }
        self.value.update(T::from_parts(start, opt_end));
    }

    fn step(&mut self, forwards: bool, factor: f32) {
        let Some(focus) = self
            .focused_knob
            .or_else(|| (!T::RANGED).then_some(Knob::Start))
        else {
            return;
        };
        let (current, other) = match (focus, T::into_parts(self.value.get())) {
            (Knob::Start, (current, other)) => (current, other),
            (Knob::End, (other, Some(current))) => (current, Some(other)),
            (Knob::End, (_, None)) => unreachable!("invalid state"),
        };
        let min = self.minimum.get();
        let max = self.maximum.get();
        let step = self.step.get();
        let mut current_percent = current.percent_between(&min, &max);
        let new_value = loop {
            let next = if forwards {
                *current_percent + *step * factor
            } else {
                *current_percent - *step * factor
            };
            if next < 0. {
                break min.clone();
            } else if next > 1. {
                break max.clone();
            }
            current_percent = ZeroToOne::new(next);
            let generated_value = min.lerp(&max, *current_percent);
            if generated_value != current {
                break generated_value;
            }
        };
        // Check that the new value didn't go past the other marker, or min/max.
        let valid_relative_to_other = match (&other, focus) {
            (Some(end), Knob::Start) => new_value < *end,
            (Some(start), Knob::End) => new_value > *start,
            (None, _) => true,
        };
        if valid_relative_to_other && new_value >= min && new_value <= max {
            let (start, end) = match (focus, other) {
                (_, None) => (new_value, None),
                (Knob::Start, Some(end)) => (new_value, Some(end)),
                (Knob::End, Some(start)) => (start, Some(new_value)),
            };
            self.value.update(T::from_parts(start, end));
        }
    }
}

impl<T> Widget for Slider<T>
where
    T: SliderValue,
{
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let track_color = context.get(&TrackColor);
        let inactive_track_color = context.get(&InactiveTrackColor);
        let knob_color = context.get(&KnobColor);
        let knob_size = self.knob_size.into_signed();
        let mut track_size = context.get(&TrackSize).into_px(context.gfx.scale());
        if knob_size > 0 {
            track_size = track_size.min(knob_size);
        }

        let half_knob = knob_size / 2;

        let (mut start_value, mut end_value) =
            T::into_parts(self.value.get_tracking_refresh(context));
        let min = self.minimum.get_tracked(context);
        let mut max = self.maximum.get_tracked(context);

        if max < min {
            self.maximum.map_mut(|max| *max = min.clone());
            max = min.clone();
        }
        let mut value_clamped = false;
        if start_value < min {
            value_clamped = true;
            start_value = min.clone();
        } else if start_value > max {
            value_clamped = true;
            start_value = max.clone();
        }

        if let Some(end) = &mut end_value {
            if *end < min {
                value_clamped = true;
                *end = min.clone();
            } else if *end < start_value {
                value_clamped = true;
                mem::swap(&mut start_value, end);
            } else if *end > max {
                value_clamped = true;
                *end = max.clone();
            }
        }

        if value_clamped {
            self.value
                .map_mut(|v| *v = T::from_parts(start_value.clone(), end_value.clone()));
        }

        let start_percent = start_value.percent_between(&min, &max);
        let end_percent = end_value.map(|end| *end.percent_between(&min, &max));

        let size = context.gfx.region().size;
        self.horizontal = size.width >= size.height;

        self.draw_track(
            &TrackSpec {
                size,
                start: *start_percent,
                end: end_percent,
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
        self.knob_size = if self.knob_visible {
            context.get(&KnobSize).into_upx(context.gfx.scale())
        } else {
            UPx::ZERO
        };
        let minimum_size = context
            .get(&MinimumSliderSize)
            .into_upx(context.gfx.scale());
        let focus_ring_width = if self.knob_visible {
            (Lp::points(2).into_upx(context.gfx.scale()) / 2).ceil() * 2
        } else {
            UPx::ZERO
        };
        let static_side = if self.knob_visible {
            self.knob_size + focus_ring_width
        } else {
            context.get(&TrackSize).into_upx(context.gfx.scale())
        };

        match (available_space.width, available_space.height) {
            (ConstraintLimit::Fill(width), ConstraintLimit::Fill(height)) => {
                // This comparison is done such that if width == height, we end
                // up with a horizontal slider.
                if width < height {
                    // Vertical slider
                    Size::new(static_side, height.max(minimum_size))
                } else {
                    // Horizontal slider
                    Size::new(width.max(minimum_size), static_side)
                }
            }
            (ConstraintLimit::Fill(width), ConstraintLimit::SizeToFit(_)) => {
                Size::new(width.max(minimum_size), static_side)
            }
            (ConstraintLimit::SizeToFit(_), ConstraintLimit::Fill(height)) => {
                Size::new(static_side, height.max(minimum_size))
            }
            (ConstraintLimit::SizeToFit(width), ConstraintLimit::SizeToFit(_)) => {
                // When we have no limit on our, we still want to be draggable.
                // Since we have no limit in both directions, we have to make a
                // choice: horizontal or vertical. It seems to @ecton at the
                // time of writing this that when there is no intent from the
                // user of the slider, a horizontal slider is expected. So, we
                // set the minimum measurement based on a horizontal
                // orientation.
                Size::new(width.min(minimum_size), static_side)
            }
        }
    }

    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_, '_>) -> bool {
        self.interactive
    }

    fn accept_focus(&mut self, context: &mut EventContext<'_, '_>) -> bool {
        self.interactive && self.knob_visible && context.get(&AutoFocusableControls).is_all()
    }

    fn focus(&mut self, context: &mut EventContext<'_, '_>) {
        if self.mouse_buttons_down == 0 {
            self.focused_knob = Some(if T::RANGED && !context.focus_is_advancing() {
                Knob::End
            } else {
                Knob::Start
            });
            context.set_needs_redraw();
        }
    }

    fn advance_focus(
        &mut self,
        direction: VisualOrder,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        let (true, Some(focused)) = (T::RANGED, self.focused_knob) else {
            return IGNORED;
        };

        let new_knob = if self.horizontal {
            match (direction.horizontal, focused) {
                (HorizontalOrder::LeftToRight, Knob::Start) => Knob::End,
                (HorizontalOrder::RightToLeft, Knob::End) => Knob::Start,
                _ => return IGNORED,
            }
        } else {
            match (direction.vertical, focused) {
                (VerticalOrder::TopToBottom, Knob::Start) => Knob::End,
                (VerticalOrder::BottomToTop, Knob::End) => Knob::Start,
                _ => return IGNORED,
            }
        };
        self.focused_knob = Some(new_knob);
        context.set_needs_redraw();
        HANDLED
    }

    fn blur(&mut self, context: &mut EventContext<'_, '_>) {
        self.previous_focus = self.focused_knob.take();
        context.set_needs_redraw();
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        let true = self.interactive else {
            return IGNORED;
        };

        let previous_focus = match (self.previous_focus.take(), self.focused_knob.take()) {
            (None | Some(_), Some(focus)) | (Some(focus), None) => Some(focus),
            (None, None) => None,
        };
        self.update_from_click(location, previous_focus);
        self.mouse_buttons_down += 1;
        context.focus();
        HANDLED
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        _context: &mut EventContext<'_, '_>,
    ) {
        self.update_from_click(location, None);
    }

    fn mouse_up(
        &mut self,
        _location: Option<Point<Px>>,
        _device_id: DeviceId,
        _button: MouseButton,
        _context: &mut EventContext<'_, '_>,
    ) {
        self.mouse_buttons_down -= 1;
    }

    fn keyboard_input(
        &mut self,
        _device_id: DeviceId,
        input: kludgine::app::winit::event::KeyEvent,
        _is_synthetic: bool,
        _context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        let true = self.interactive else {
            return IGNORED;
        };

        let forwards = match input.logical_key {
            Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp) => false,
            Key::Named(NamedKey::ArrowRight | NamedKey::ArrowDown) => true,
            _ => return IGNORED,
        };
        if !input.state.is_pressed() {
            return HANDLED;
        }

        self.step(forwards, 1.);

        HANDLED
    }

    fn mouse_wheel(
        &mut self,
        _device_id: DeviceId,
        delta: MouseScrollDelta,
        _phase: TouchPhase,
        _context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        let true = self.interactive else {
            return IGNORED;
        };

        let factor: f32 = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(pt) => pt.y.cast(),
        };

        let (forwards, factor) = if factor.is_sign_negative() {
            (false, -factor)
        } else {
            (true, factor)
        };

        self.step(forwards, factor);

        // @ecton: Unlike scroll alreas cascasing, I feel like scrolling while
        // using a mouse wheel as an input is annoying.
        HANDLED
    }
}

struct TrackSpec {
    size: Size<Px>,
    start: f32,
    end: Option<f32>,
    half_knob: Px,
    knob_size: Px,
    track_size: Px,
    knob_color: Color,
    track_color: Color,
    inactive_track_color: Color,
}

impl TrackSpec {
    fn if_knobbed<R>(&self, knobbed: impl FnOnce() -> R) -> R
    where
        R: Default,
    {
        if self.knob_size > 0 {
            knobbed()
        } else {
            R::default()
        }
    }
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

define_components! {
    Slider {
        /// The size of the track that the knob of a [`Slider`] traversesq.
        TrackSize(Dimension, "track_size", Dimension::Lp(Lp::points(5)))
        /// The width and height of the draggable portion of a [`Slider`].
        KnobSize(Dimension, "knob_size", Dimension::Lp(Lp::points(14)))
        /// The minimum length of the slidable dimension.
        MinimumSliderSize(Dimension, "minimum_size", |context| context.get(&KnobSize) * 2)
        /// The color of the draggable portion of the knob.
        KnobColor(Color, "knob_color", @WidgetAccentColor)
        /// The color of the track that the knob rests on.
        TrackColor(Color,"track_color", |context| context.get(&KnobColor))
        /// The color of the track that the knob rests on.
        InactiveTrackColor(Color, "inactive_track_color", |context| context.get(&OpaqueWidgetColor))
    }
}

/// A value that can be used in a [`Slider`] widget.
pub trait SliderValue: Clone + PartialEq + UnwindSafe + Send + Debug + 'static {
    /// The component value for the slider.
    type Value: Clone
        + Debug
        + PartialOrd
        + LinearInterpolate
        + PercentBetween
        + UnwindSafe
        + Send
        + 'static;
    /// When true, this type is expected to represent two values: start and an
    /// end.
    const RANGED: bool;

    /// Returns this value split into its start and end components.
    fn into_parts(self) -> (Self::Value, Option<Self::Value>);
    /// Constructs a value from its start and end components.
    fn from_parts(min_or_value: Self::Value, max: Option<Self::Value>) -> Self;
}

impl<T> SliderValue for T
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
    type Value = T;

    const RANGED: bool = false;

    fn into_parts(self) -> (Self::Value, Option<Self::Value>) {
        (self, None)
    }

    fn from_parts(min_or_value: Self::Value, _max: Option<Self::Value>) -> Self {
        min_or_value
    }
}

impl<T> SliderValue for RangeInclusive<T>
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
    type Value = T;

    const RANGED: bool = true;

    fn into_parts(self) -> (Self::Value, Option<Self::Value>) {
        let (start, end) = self.into_inner();
        (start, Some(end))
    }

    fn from_parts(min_or_value: Self::Value, max: Option<Self::Value>) -> Self {
        min_or_value..=max.assert("always provided")
    }
}

impl<T> SliderValue for (T, T)
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
    type Value = T;

    const RANGED: bool = true;

    fn into_parts(self) -> (Self::Value, Option<Self::Value>) {
        (self.0, Some(self.1))
    }

    fn from_parts(min_or_value: Self::Value, max: Option<Self::Value>) -> Self {
        (min_or_value, max.assert("always provided"))
    }
}

/// A value that can be used in a [`Slider`] widget.
pub trait Slidable<T>: IntoDynamic<T> + Sized
where
    T: SliderValue,
{
    /// Returns a new slider over the full [range](Ranged) of the type.
    fn slider(self) -> Slider<T>
    where
        T::Value: Ranged,
    {
        Slider::from_value(self.into_dynamic())
    }

    /// Returns a new slider using the value of `self`. The slider will be
    /// limited to values between `min` and `max`.
    fn slider_between(
        self,
        min: impl IntoValue<T::Value>,
        max: impl IntoValue<T::Value>,
    ) -> Slider<T> {
        Slider::new(self.into_dynamic(), min, max)
    }
}

impl<U, T> Slidable<U> for T
where
    T: IntoDynamic<U>,
    U: SliderValue,
{
}
