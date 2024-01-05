//! Widgets for displaying progress indicators.

use std::ops::RangeInclusive;
use std::time::Duration;

use figures::units::Px;
use figures::{Angle, Point, Ranged, ScreenScale, Size, Zero};
use kludgine::shapes::{Path, StrokeOptions};
use kludgine::Color;

use crate::animation::easings::{EaseInQuadradic, EaseOutQuadradic};
use crate::animation::{
    AnimationHandle, AnimationTarget, IntoAnimate, PercentBetween, Spawn, ZeroToOne,
};
use crate::value::{Destination, Dynamic, IntoReadOnly, IntoReader, MapEach, ReadOnly, Source};
use crate::widget::{MakeWidget, MakeWidgetWithTag, Widget, WidgetInstance};
use crate::widgets::slider::{InactiveTrackColor, Slidable, TrackColor, TrackSize};
use crate::widgets::Data;

/// A bar-shaped progress indicator.
#[derive(Debug)]
pub struct ProgressBar {
    progress: ReadOnly<Progress>,
    spinner: bool,
}

impl ProgressBar {
    /// Returns an indeterminant progress bar.
    #[must_use]
    pub const fn indeterminant() -> Self {
        Self {
            progress: ReadOnly::Constant(Progress::Indeterminant),
            spinner: false,
        }
    }

    /// Returns a new progress bar that displays `progress`.
    #[must_use]
    pub fn new(progress: impl IntoReadOnly<Progress>) -> Self {
        Self {
            progress: progress.into_read_only(),
            spinner: false,
        }
    }

    /// Returns a new progress bar that displays `progress`.
    #[must_use]
    pub fn spinner(mut self) -> Self {
        self.spinner = true;
        self
    }
}

/// A measurement of progress for an indicator widget like [`ProgressBar`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum Progress<T = ZeroToOne> {
    /// The task has an indeterminant length.
    #[default]
    Indeterminant,
    /// The task is a specified amount complete.
    Percent(T),
}

impl MakeWidgetWithTag for ProgressBar {
    fn make_with_tag(self, id: crate::widget::WidgetTag) -> WidgetInstance {
        let start = Dynamic::new(ZeroToOne::ZERO);
        let end = Dynamic::new(ZeroToOne::ZERO);
        let value = (&start, &end).map_each(|(start, end)| *start..=*end);

        let mut indeterminant_animation = None;

        let (slider, degree_offset) = if self.spinner {
            let degree_offset = Dynamic::new(Angle::degrees(270));
            (
                Spinner {
                    start: start.clone(),
                    end: end.clone(),
                    degree_offset: degree_offset.clone(),
                }
                .make_with_tag(id),
                Some(degree_offset),
            )
        } else {
            (
                value
                    .slider()
                    .knobless()
                    .non_interactive()
                    .make_with_tag(id),
                None,
            )
        };

        update_progress_bar(
            self.progress.get(),
            &mut indeterminant_animation,
            &start,
            &end,
            degree_offset.as_ref(),
        );

        match self.progress {
            ReadOnly::Reader(progress) => {
                let callback = progress.for_each(move |progress| {
                    update_progress_bar(
                        *progress,
                        &mut indeterminant_animation,
                        &start,
                        &end,
                        degree_offset.as_ref(),
                    );
                });
                Data::new_wrapping((callback, progress), slider).make_widget()
            }
            ReadOnly::Constant(_) => {
                Data::new_wrapping(indeterminant_animation, slider).make_widget()
            }
        }
    }
}

#[derive(Debug)]
struct IndeterminantAnimations {
    _primary: AnimationHandle,
    _degree_offset: Option<AnimationHandle>,
}

fn update_progress_bar(
    progress: Progress,
    indeterminant_animation: &mut Option<IndeterminantAnimations>,
    start: &Dynamic<ZeroToOne>,
    end: &Dynamic<ZeroToOne>,
    degree_offset: Option<&Dynamic<Angle>>,
) {
    match progress {
        Progress::Indeterminant => {
            if indeterminant_animation.is_none() {
                *indeterminant_animation = Some(IndeterminantAnimations {
                    _primary: (
                        start
                            .transition_to(ZeroToOne::ZERO)
                            .immediately()
                            .and_then(Duration::from_millis(250))
                            .and_then(
                                start
                                    .transition_to(ZeroToOne::new(0.33))
                                    .over(Duration::from_millis(500))
                                    .with_easing(EaseInQuadradic),
                            )
                            .and_then(
                                start
                                    .transition_to(ZeroToOne::new(1.0))
                                    .over(Duration::from_millis(500))
                                    .with_easing(EaseOutQuadradic),
                            ),
                        end.transition_to(ZeroToOne::ZERO)
                            .immediately()
                            .and_then(
                                end.transition_to(ZeroToOne::new(0.75))
                                    .over(Duration::from_millis(500))
                                    .with_easing(EaseInQuadradic),
                            )
                            .and_then(
                                end.transition_to(ZeroToOne::ONE)
                                    .over(Duration::from_millis(250))
                                    .with_easing(EaseOutQuadradic),
                            ),
                    )
                        .cycle()
                        .spawn(),
                    _degree_offset: degree_offset.map(|degree_offset| {
                        degree_offset
                            .transition_to(Angle::MIN)
                            .immediately()
                            .and_then(
                                degree_offset
                                    .transition_to(Angle::MAX)
                                    .over(Duration::from_secs_f32(1.66)),
                            )
                            .cycle()
                            .spawn()
                    }),
                });
            }
        }
        Progress::Percent(value) => {
            let _stopped_animation = indeterminant_animation.take();
            if let Some(degree_offset) = degree_offset {
                degree_offset.set(Angle::degrees(270));
            }
            start.set(ZeroToOne::ZERO);
            end.set(value);
        }
    }
}

/// A value that can be used in a progress indicator.
pub trait Progressable<T>: IntoReader<T> + Sized
where
    T: ProgressValue + Send,
{
    /// Returns a new progress bar that displays progress from `T::MIN` to
    /// `T::MAX`.
    fn progress_bar(self) -> ProgressBar {
        ProgressBar::new(self.into_reader().map_each(|value| value.to_progress(None)))
    }

    /// Returns a new progress bar that displays progress from `T::MIN` to
    /// `max`. The maximum value can be either a `T` or an `Option<T>`. If
    /// `None` is the maximum value, an indeterminant progress bar will be
    /// displayed.
    fn progress_bar_to(self, max: impl IntoReadOnly<T::Value>) -> ProgressBar
    where
        T::Value: PartialEq + Ranged + Send + Clone,
    {
        let max = max.into_read_only();
        match max {
            ReadOnly::Constant(max) => self.progress_bar_between(<T::Value>::MIN..=max),
            ReadOnly::Reader(max) => {
                self.progress_bar_between(max.map_each(|max| <T::Value>::MIN..=max.clone()))
            }
        }
    }

    /// Returns a new progress bar that displays progress over the specified
    /// `range` of `T`. The range can be either a `T..=T` or an `Option<T>`. If
    /// `None` is specified as the range, an indeterminant progress bar will be
    /// displayed.
    fn progress_bar_between<Range>(self, range: Range) -> ProgressBar
    where
        T::Value: Send,
        Range: IntoReadOnly<RangeInclusive<T::Value>>,
    {
        let value = self.into_reader();
        let range = range.into_read_only();
        ProgressBar::new(match range {
            ReadOnly::Constant(range) => value
                .map_each(move |value| value.to_progress(Some(range.start()..=range.end())))
                .into_reader(),
            ReadOnly::Reader(range) => (&range, &value)
                .map_each(|(range, value)| value.to_progress(Some(range.start()..=range.end())))
                .into_reader(),
        })
    }
}

impl<T, U> Progressable<U> for T
where
    T: IntoReader<U> + Send,
    U: ProgressValue + Send,
{
}

/// A value that can be used in a progress indicator.
pub trait ProgressValue: 'static {
    /// The type that progress is ranged over.
    type Value;

    /// Converts this value to a progress using the range given, if provided. If
    /// no range is provided, the full range of the type should be considered.
    fn to_progress(&self, range: Option<RangeInclusive<&Self::Value>>) -> Progress;
}

impl<T> ProgressValue for T
where
    T: Ranged + PercentBetween + 'static,
{
    type Value = T;

    fn to_progress(&self, range: Option<RangeInclusive<&Self::Value>>) -> Progress {
        if let Some(range) = range {
            Progress::Percent(self.percent_between(range.start(), range.end()))
        } else {
            Progress::Percent(self.percent_between(&T::MIN, &T::MAX))
        }
    }
}

impl<T> ProgressValue for Option<T>
where
    T: Ranged + PercentBetween + 'static,
{
    type Value = T;

    fn to_progress(&self, range: Option<RangeInclusive<&Self::Value>>) -> Progress {
        self.as_ref()
            .map_or(Progress::Indeterminant, |value| value.to_progress(range))
    }
}

impl<T> ProgressValue for Progress<T>
where
    T: Ranged + PercentBetween + 'static,
{
    type Value = T;

    fn to_progress(&self, range: Option<RangeInclusive<&Self::Value>>) -> Progress {
        match self {
            Progress::Indeterminant => Progress::Indeterminant,
            Progress::Percent(value) => value.to_progress(range),
        }
    }
}

/// A circular progress widget.
#[derive(Debug)]
pub struct Spinner {
    start: Dynamic<ZeroToOne>,
    end: Dynamic<ZeroToOne>,
    degree_offset: Dynamic<Angle>,
}

impl Spinner {
    fn draw_arc(
        track_size: Px,
        radius: Px,
        degree_offset: Angle,
        start: ZeroToOne,
        sweep: ZeroToOne,
        color: Color,
        context: &mut crate::context::GraphicsContext<'_, '_, '_, '_>,
    ) {
        if sweep > 0. {
            context.gfx.draw_shape(
                &Path::arc(
                    Point::squared(radius + track_size / 2),
                    Size::squared(radius),
                    Angle::degrees_f(*start * 360.) + degree_offset,
                    Angle::degrees_f(*sweep * 360.),
                )
                .stroke(StrokeOptions::px_wide(track_size).colored(color)),
            );
        }
    }
}

impl Widget for Spinner {
    fn redraw(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_>) {
        let track_size = context.get(&TrackSize).into_px(context.gfx.scale());
        let start = self.start.get_tracking_redraw(context);
        let end = self.end.get_tracking_redraw(context);
        let size = context.gfx.region().size;
        let render_size = size.width.min(size.height);
        let radius = render_size / 2 - track_size;
        let degree_offset = self.degree_offset.get();

        if start > ZeroToOne::ZERO {
            Self::draw_arc(
                track_size,
                radius,
                degree_offset,
                ZeroToOne::ZERO,
                start,
                context.get(&InactiveTrackColor),
                context,
            );
        }

        if start != end {
            Self::draw_arc(
                track_size,
                radius,
                degree_offset,
                start,
                ZeroToOne::new(*end - *start),
                context.get(&TrackColor),
                context,
            );
        }

        if end < ZeroToOne::ONE {
            Self::draw_arc(
                track_size,
                radius,
                degree_offset,
                end,
                end.one_minus(),
                context.get(&InactiveTrackColor),
                context,
            );
        }
    }

    fn layout(
        &mut self,
        available_space: figures::Size<crate::ConstraintLimit>,
        context: &mut crate::context::LayoutContext<'_, '_, '_, '_>,
    ) -> figures::Size<figures::units::UPx> {
        let track_size = context.get(&TrackSize).into_px(context.gfx.scale());
        let minimum_size = track_size * 4;

        available_space.map(|constraint| constraint.fit_measured(minimum_size, context.gfx.scale()))
    }
}
