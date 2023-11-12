//! Types for creating animations.
//!
//! Animations in Gooey are performed by transitioning a [`Dynamic`]'s contained
//! value over time. This starts with [`Dynamic::transition_to()`], which
//! returns a [`DynamicTransition`].
//!
//! [`DynamicTransition`] implements [`AnimationTarget`], a trait that describes
//! types that can be updated using [linear interpolation](LinearInterpolate).
//! `AnimationTarget` is also implemented for tuples of `AnimationTarget`
//! implementors, allowing multiple transitions to be an `AnimationTarget`.
//!
//! Next, the [`AnimationTarget`] is turned into an animation by invoking
//! [`AnimationTarget::over()`] with the [`Duration`] the transition should
//! occur over. The animation can further be customized using
//! [`Animation::with_easing()`] to utilize any [`Easing`] implementor.
//!
//! ```rust
//! use std::time::Duration;
//!
//! use gooey::animation::easings::EaseInOutElastic;
//! use gooey::animation::{AnimationTarget, Spawn};
//! use gooey::value::Dynamic;
//!
//! let value = Dynamic::new(0);
//!
//! value
//!     .transition_to(100)
//!     .over(Duration::from_millis(100))
//!     .with_easing(EaseInOutElastic)
//!     .launch();
//!
//! let mut reader = value.into_reader();
//! while reader.block_until_updated() {
//!     println!("{}", reader.get());
//! }
//!
//! assert_eq!(reader.get(), 100);
//! ```

pub mod easings;

use std::fmt::{Debug, Display};
use std::ops::{ControlFlow, Deref, Div, Mul};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::str::FromStr;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use alot::{LotId, Lots};
use intentional::Cast;
use kempt::Set;
use kludgine::figures::Ranged;
use kludgine::Color;

use crate::animation::easings::Linear;
use crate::styles::Component;
use crate::value::Dynamic;

static ANIMATIONS: Mutex<Animating> = Mutex::new(Animating::new());
static NEW_ANIMATIONS: Condvar = Condvar::new();

fn thread_state() -> MutexGuard<'static, Animating> {
    static THREAD: OnceLock<()> = OnceLock::new();
    THREAD.get_or_init(|| {
        thread::spawn(animation_thread);
    });
    ANIMATIONS
        .lock()
        .map_or_else(PoisonError::into_inner, |g| g)
}

fn animation_thread() {
    let mut state = thread_state();
    loop {
        if state.running.is_empty() {
            state.last_updated = None;
            state = NEW_ANIMATIONS
                .wait(state)
                .map_or_else(PoisonError::into_inner, |g| g);
        } else {
            let start = Instant::now();
            let last_tick = state.last_updated.unwrap_or(start);
            let elapsed = start - last_tick;
            state.last_updated = Some(start);

            let mut index = 0;
            while index < state.running.len() {
                let animation_id = *state.running.member(index).expect("index in bounds");
                let animation_state = &mut state.animations[animation_id];
                if animation_state.animation.animate(elapsed).is_break() {
                    if !animation_state.handle_attached {
                        state.animations.remove(animation_id);
                    }
                    state.running.remove_member(index);
                } else {
                    index += 1;
                }
            }

            drop(state);
            let next_tick = last_tick + Duration::from_millis(16);
            std::thread::sleep(
                next_tick
                    .checked_duration_since(Instant::now())
                    .unwrap_or(Duration::from_millis(16)),
            );
            state = thread_state();
        }
    }
}

struct AnimationState {
    animation: Box<dyn Animate>,
    handle_attached: bool,
}

struct Animating {
    animations: Lots<AnimationState>,
    running: Set<LotId>,
    last_updated: Option<Instant>,
}

impl Animating {
    const fn new() -> Self {
        Self {
            animations: Lots::new(),
            running: Set::new(),
            last_updated: None,
        }
    }

    fn spawn(&mut self, animation: Box<dyn Animate>) -> AnimationHandle {
        let id = self.animations.push(AnimationState {
            animation,
            handle_attached: true,
        });

        if self.running.is_empty() {
            NEW_ANIMATIONS.notify_one();
        }

        self.running.insert(id);

        AnimationHandle(Some(id))
    }

    fn remove_animation(&mut self, id: LotId) {
        self.animations.remove(id);
        self.running.remove(&id);
    }

    fn run_unattached(&mut self, id: LotId) {
        if self.running.contains(&id) {
            self.animations[id].handle_attached = false;
        } else {
            self.animations.remove(id);
        }
    }
}

/// A type that can animate.
pub trait Animate: Send + Sync {
    /// Update the animation by progressing the timeline by `elapsed`.
    ///
    /// When the animation is complete, return `ControlFlow::Break` with the
    /// remaining time that was not needed to complete the animation. This is
    /// used in multi-step animation processes to ensure time is accurately
    /// tracked.
    fn animate(&mut self, elapsed: Duration) -> ControlFlow<Duration>;
}

/// A pending transition for a [`Dynamic`] to a new value.
pub struct DynamicTransition<T> {
    /// The dynamic value to change.
    pub dynamic: Dynamic<T>,
    /// The final value to store in the [`Dynamic`].
    pub new_value: T,
}

impl<T> AnimationTarget for DynamicTransition<T>
where
    T: LinearInterpolate + Clone + Send + Sync,
{
    type Running = TransitioningDynamic<T>;

    fn begin(self) -> Self::Running {
        self.into()
    }
}

/// A [`DynamicTransition`] that has begun its transition.
pub struct TransitioningDynamic<T> {
    change: DynamicTransition<T>,
    start: T,
}

impl<T> From<DynamicTransition<T>> for TransitioningDynamic<T>
where
    T: Clone,
{
    fn from(change: DynamicTransition<T>) -> Self {
        Self {
            start: change.dynamic.get(),
            change,
        }
    }
}

impl<T> AnimateTarget for TransitioningDynamic<T>
where
    T: LinearInterpolate + Clone + Send + Sync,
{
    fn update(&self, percent: f32) {
        self.change
            .dynamic
            .update(self.start.lerp(&self.change.new_value, percent));
    }

    fn finish(&self) {
        self.change.dynamic.update(self.change.new_value.clone());
    }
}

/// Describes a change to a new value for a [`Dynamic`] over a specified
/// [`Duration`], using the `Easing` generic parameter to control how the value
/// is interpolated.
#[must_use = "animations are not performed until they are spawned"]
pub struct Animation<Target, Easing = Linear>
where
    Target: AnimationTarget,
{
    value: Target,
    duration: Duration,
    easing: Easing,
}

impl<T> Animation<T, Linear>
where
    T: AnimationTarget,
{
    fn new(value: T, duration: Duration) -> Self {
        Self {
            value,
            duration,
            easing: Linear,
        }
    }

    /// Returns this animation with a different easing function.
    pub fn with_easing<Easing: self::Easing>(self, easing: Easing) -> Animation<T, Easing> {
        Animation {
            value: self.value,
            duration: self.duration,
            easing,
        }
    }
}

impl<T, Easing> IntoAnimate for Animation<T, Easing>
where
    T: AnimationTarget,
    Easing: self::Easing,
{
    type Animate = RunningAnimation<T::Running, Easing>;

    fn into_animate(self) -> Self::Animate {
        RunningAnimation {
            target: self.value.begin(),
            duration: self.duration,
            elapsed: Duration::ZERO,
            easing: self.easing,
        }
    }
}

/// A target for a timed [`Animation`].
pub trait AnimationTarget: Sized + Send + Sync {
    /// The type that can linearly interpolate this target.
    type Running: AnimateTarget;

    /// Record the current value of the target, and return a type that can
    /// linearly interpolate between the current value and the desired value.
    fn begin(self) -> Self::Running;

    /// Returns a pending animation that linearly transitions `self` over
    /// `duration`.
    ///
    /// A different [`Easing`] can be used by calling
    /// [`with_easing`](Animation::with_easing) on the result of this function.
    fn over(self, duration: Duration) -> Animation<Self, Linear> {
        Animation::new(self, duration)
    }
}

/// The target of an [`Animate`] implementor.
pub trait AnimateTarget: Send + Sync {
    /// Updates the target with linear interpolation.
    fn update(&self, percent: f32);
    /// Sets the target to the desired completion state.
    fn finish(&self);
}

/// A type that can convert into `Box<dyn Animate>`.
pub trait BoxAnimate {
    /// Returns the boxed animation.
    fn boxed(self) -> Box<dyn Animate>;
}

/// A type that can be converted into an animation.
pub trait IntoAnimate: Sized + Send + Sync {
    /// The running animation type.
    type Animate: Animate;

    /// Return this change as a running animation.
    fn into_animate(self) -> Self::Animate;

    /// Returns an combined animation that performs `self` and `other` in
    /// sequence.
    fn and_then<Other: IntoAnimate>(self, other: Other) -> Chain<Self, Other> {
        Chain::new(self, other)
    }

    /// Invokes `on_complete` after this animation finishes.
    fn on_complete<F>(self, on_complete: F) -> OnCompleteAnimation<Self>
    where
        F: FnMut() + Send + Sync + 'static,
    {
        OnCompleteAnimation::new(self, on_complete)
    }
}

macro_rules! impl_tuple_animate {
    ($($type:ident $field:tt $var:ident),+) => {
        impl<$($type),+> AnimationTarget for ($($type,)+) where $($type: AnimationTarget),+ {
            type Running = ($(<$type>::Running,)+);

            fn begin(self) -> Self::Running {
                ($(self.$field.begin(),)+)
            }
        }

        impl<$($type),+> AnimateTarget for ($($type,)+) where $($type: AnimateTarget),+ {
            fn update(&self, percent: f32) {
                $(self.$field.update(percent);)+
            }

            fn finish(&self) {
                $(self.$field.finish();)+
            }
        }
    }
}

impl_all_tuples!(impl_tuple_animate);

impl<T> BoxAnimate for T
where
    T: IntoAnimate + 'static,
{
    fn boxed(self) -> Box<dyn Animate> {
        Box::new(self.into_animate())
    }
}

/// A [`Animate`] implementor that has been boxed as a trait object.
pub struct BoxedAnimation(Box<dyn Animate>);

/// An animation that can be spawned.
pub trait Spawn {
    /// Spawns the animation, returning a handle that tracks the animation.
    ///
    /// When the returned handle is dropped, the animation is stopped.
    fn spawn(self) -> AnimationHandle;

    /// Launches this animation, running it to completion in the background.
    fn launch(self)
    where
        Self: Sized,
    {
        self.spawn().detach();
    }
}

impl<T> Spawn for T
where
    T: BoxAnimate,
{
    fn spawn(self) -> AnimationHandle {
        self.boxed().spawn()
    }
}

impl Spawn for Box<dyn Animate> {
    fn spawn(self) -> AnimationHandle {
        thread_state().spawn(self)
    }
}

impl<T, Easing> Animate for RunningAnimation<T, Easing>
where
    T: AnimateTarget,
    Easing: self::Easing,
{
    fn animate(&mut self, elapsed: Duration) -> ControlFlow<Duration> {
        self.elapsed = self.elapsed.checked_add(elapsed).unwrap_or(Duration::MAX);

        if let Some(remaining_elapsed) = self.elapsed.checked_sub(self.duration) {
            self.target.finish();
            ControlFlow::Break(remaining_elapsed)
        } else {
            let progress = self.easing.ease(ZeroToOne::new(
                self.elapsed.as_secs_f32() / self.duration.as_secs_f32(),
            ));
            self.target.update(progress);
            ControlFlow::Continue(())
        }
    }
}

/// A running [`Animation`] that changes a [`Dynamic`] over a specified
/// [`Duration`], using the `Easing` generic parameter to control how the value
/// is interpolated.
///
/// The initial value for interpolation is recorded at the time this type is
/// created: [`IntoAnimate::into_animate`]. [`Easing`] is used to customize how
/// interpolation is performed.
pub struct RunningAnimation<T, Easing> {
    target: T,
    duration: Duration,
    elapsed: Duration,
    easing: Easing,
}

/// A handle to a spawned animation. When dropped, the associated animation will
/// be stopped.
#[derive(Default, Debug)]
#[must_use]
pub struct AnimationHandle(Option<LotId>);

impl AnimationHandle {
    /// Returns an empty handle that references no animation.
    pub const fn new() -> Self {
        Self(None)
    }

    /// Cancels the animation immediately.
    ///
    /// This has the same effect as dropping the handle.
    pub fn clear(&mut self) {
        if let Some(id) = self.0.take() {
            thread_state().remove_animation(id);
        }
    }

    /// Detaches the animation from the [`AnimationHandle`], allowing the
    /// animation to continue running to completion.
    ///
    /// Normally, dropping an [`AnimationHandle`] will cancel the underlying
    /// animation. This API provides a way to continue running an animation
    /// through completion without needing to hold onto the handle.
    pub fn detach(mut self) {
        if let Some(id) = self.0.take() {
            thread_state().run_unattached(id);
        }
    }
}

impl Drop for AnimationHandle {
    fn drop(&mut self) {
        self.clear();
    }
}

/// An animation combinator that runs animation `A`, then animation `B`.
pub struct Chain<A: IntoAnimate, B: IntoAnimate>(A, B);

/// A [`Chain`] that is currently animating.
pub struct RunningChain<A: IntoAnimate, B: IntoAnimate>(Option<ChainState<A, B>>);

enum ChainState<A: IntoAnimate, B: IntoAnimate> {
    AnimatingFirst(A::Animate, B),
    AnimatingSecond(B::Animate),
}

impl<A, B> Chain<A, B>
where
    A: IntoAnimate,
    B: IntoAnimate,
{
    /// Returns a new instance with `first` and `second`.
    pub const fn new(first: A, second: B) -> Self {
        Self(first, second)
    }
}

impl<A, B> IntoAnimate for Chain<A, B>
where
    A: IntoAnimate,
    B: IntoAnimate,
{
    type Animate = RunningChain<A, B>;

    fn into_animate(self) -> Self::Animate {
        let a = self.0.into_animate();
        RunningChain(Some(ChainState::AnimatingFirst(a, self.1)))
    }
}

impl<A, B> Animate for RunningChain<A, B>
where
    A: IntoAnimate,
    B: IntoAnimate,
{
    fn animate(&mut self, elapsed: Duration) -> ControlFlow<Duration> {
        match self.0.as_mut().expect("invalid state") {
            ChainState::AnimatingFirst(a, _) => match a.animate(elapsed) {
                ControlFlow::Continue(()) => ControlFlow::Continue(()),
                ControlFlow::Break(remaining) => {
                    let Some(ChainState::AnimatingFirst(_, b)) = self.0.take() else {
                        unreachable!("invalid state")
                    };
                    self.0 = Some(ChainState::AnimatingSecond(b.into_animate()));
                    self.animate(remaining)
                }
            },
            ChainState::AnimatingSecond(b) => b.animate(elapsed),
        }
    }
}

/// An animation wrapper that invokes a callback upon the animation completing.
///
/// This type guarantees the callback will only be invoked once per animation
/// completion. If the animation is restarted after completing, the callback
/// will be invoked again.
pub struct OnCompleteAnimation<A> {
    animation: A,
    callback: Box<dyn FnMut() + Send + Sync + 'static>,
    completed: bool,
}

impl<A> OnCompleteAnimation<A> {
    /// Returns a pending animation that performs `animation` then invokes
    /// `on_complete`.
    pub fn new<F>(animation: A, on_complete: F) -> Self
    where
        F: FnMut() + Send + Sync + 'static,
    {
        Self {
            animation,
            callback: Box::new(on_complete),
            completed: false,
        }
    }
}

impl<A> IntoAnimate for OnCompleteAnimation<A>
where
    A: IntoAnimate,
{
    type Animate = OnCompleteAnimation<A::Animate>;

    fn into_animate(self) -> Self::Animate {
        OnCompleteAnimation {
            animation: self.animation.into_animate(),
            callback: self.callback,
            completed: false,
        }
    }
}

impl<A> Animate for OnCompleteAnimation<A>
where
    A: Animate,
{
    fn animate(&mut self, elapsed: Duration) -> ControlFlow<Duration> {
        if self.completed {
            ControlFlow::Break(elapsed)
        } else {
            match self.animation.animate(elapsed) {
                ControlFlow::Break(remaining) => {
                    self.completed = true;
                    (self.callback)();
                    ControlFlow::Break(remaining)
                }
                ControlFlow::Continue(()) => ControlFlow::Continue(()),
            }
        }
    }
}

impl IntoAnimate for Duration {
    type Animate = Self;

    fn into_animate(self) -> Self::Animate {
        self
    }
}

impl Animate for Duration {
    fn animate(&mut self, elapsed: Duration) -> ControlFlow<Duration> {
        if let Some(remaining) = self.checked_sub(elapsed) {
            *self = remaining;
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(elapsed - *self)
        }
    }
}

/// Performs a linear interpolation between two values.
pub trait LinearInterpolate: PartialEq {
    /// Interpolate linearly between `self` and `target` using `percent`.
    #[must_use]
    fn lerp(&self, target: &Self, percent: f32) -> Self;
}

macro_rules! impl_lerp_for_int {
    ($type:ident, $unsigned:ident, $float:ident) => {
        impl LinearInterpolate for $type {
            fn lerp(&self, target: &Self, percent: f32) -> Self {
                let percent = $float::from(percent);
                let delta = target.abs_diff(*self);
                let delta = (delta as $float * percent).round() as $unsigned;
                if target > self {
                    self.checked_add_unsigned(delta).expect("direction checked")
                } else {
                    self.checked_sub_unsigned(delta).expect("direction checked")
                }
            }
        }
    };
}

macro_rules! impl_lerp_for_uint {
    ($type:ident, $float:ident) => {
        impl LinearInterpolate for $type {
            fn lerp(&self, target: &Self, percent: f32) -> Self {
                let percent = $float::from(percent);
                if let Some(delta) = target.checked_sub(*self) {
                    *self + (delta as $float * percent).round() as $type
                } else {
                    *self - ((*self - *target) as $float * percent).round() as $type
                }
            }
        }
    };
}

impl_lerp_for_uint!(u8, f32);
impl_lerp_for_uint!(u16, f32);
impl_lerp_for_uint!(u32, f32);
impl_lerp_for_uint!(u64, f32);
impl_lerp_for_uint!(u128, f64);
impl_lerp_for_uint!(usize, f64);
impl_lerp_for_int!(i8, u8, f32);
impl_lerp_for_int!(i16, u16, f32);
impl_lerp_for_int!(i32, u32, f32);
impl_lerp_for_int!(i64, u64, f32);
impl_lerp_for_int!(i128, u128, f64);
impl_lerp_for_int!(isize, usize, f64);

impl LinearInterpolate for f32 {
    fn lerp(&self, target: &Self, percent: f32) -> Self {
        let delta = *target - *self;
        *self + delta * percent
    }
}

impl LinearInterpolate for f64 {
    fn lerp(&self, target: &Self, percent: f32) -> Self {
        let delta = *target - *self;
        *self + delta * f64::from(percent)
    }
}

impl LinearInterpolate for bool {
    fn lerp(&self, target: &Self, percent: f32) -> Self {
        if percent >= 0.5 {
            *target
        } else {
            *self
        }
    }
}

impl PercentBetween for bool {
    fn percent_between(&self, min: &Self, max: &Self) -> ZeroToOne {
        if *min == *max || *self == *min {
            ZeroToOne::ZERO
        } else {
            ZeroToOne::ONE
        }
    }
}

#[test]
fn integer_lerps() {
    #[track_caller]
    fn test_lerps<T: LinearInterpolate + Debug + Eq>(a: &T, b: &T, mid: &T) {
        assert_eq!(&a.lerp(b, 1.), b);
        assert_eq!(&a.lerp(b, 0.), a);
        assert_eq!(&a.lerp(b, 0.5), mid);
    }

    test_lerps(&u8::MIN, &u8::MAX, &128);
    test_lerps(&u16::MIN, &u16::MAX, &32_768);
    test_lerps(&u32::MIN, &u32::MAX, &2_147_483_648);
    test_lerps(&i8::MIN, &i8::MAX, &0);
    test_lerps(&i16::MIN, &i16::MAX, &0);
    test_lerps(&i32::MIN, &i32::MAX, &0);
    test_lerps(&i64::MIN, &i64::MAX, &0);
    test_lerps(&i128::MIN, &i128::MAX, &0);
    test_lerps(&isize::MIN, &isize::MAX, &0);
}

impl LinearInterpolate for Color {
    fn lerp(&self, target: &Self, percent: f32) -> Self {
        Color::new(
            self.red().lerp(&target.red(), percent),
            self.green().lerp(&target.green(), percent),
            self.blue().lerp(&target.blue(), percent),
            self.alpha().lerp(&target.alpha(), percent),
        )
    }
}

/// Calculates the ratio of one value against a minimum and maximum.
pub trait PercentBetween {
    /// Return the percentage that `self` is between `min` and `max`.
    fn percent_between(&self, min: &Self, max: &Self) -> ZeroToOne;
}

macro_rules! impl_percent_between {
    ($type:ident, $float:ident) => {
        impl PercentBetween for $type {
            fn percent_between(&self, min: &Self, max: &Self) -> ZeroToOne {
                let range = *max - *min;
                ZeroToOne::from(*self as $float / range as $float)
            }
        }
    };
}

impl_percent_between!(u8, f32);
impl_percent_between!(u16, f32);
impl_percent_between!(u32, f32);
impl_percent_between!(u64, f32);
impl_percent_between!(u128, f64);
impl_percent_between!(usize, f64);
impl_percent_between!(i8, f32);
impl_percent_between!(i16, f32);
impl_percent_between!(i32, f32);
impl_percent_between!(i64, f32);
impl_percent_between!(i128, f64);
impl_percent_between!(isize, f64);
impl_percent_between!(f32, f32);
impl_percent_between!(f64, f64);

impl PercentBetween for Color {
    fn percent_between(&self, min: &Self, max: &Self) -> ZeroToOne {
        fn channel_percent(
            value: Color,
            min: Color,
            max: Color,
            func: impl Fn(Color) -> u8,
        ) -> ZeroToOne {
            func(value).percent_between(&func(min), &func(max))
        }

        channel_percent(*self, *min, *max, Color::red)
            * channel_percent(*self, *min, *max, Color::green)
            * channel_percent(*self, *min, *max, Color::blue)
            * channel_percent(*self, *min, *max, Color::alpha)
    }
}

/// An `f32` that is clamped between 0.0 and 1.0 and cannot be NaN or Infinity.
///
/// Because of these restrictions, this type implements `Ord` and `Eq`.
#[derive(Clone, Copy, Debug)]
pub struct ZeroToOne(f32);

impl ZeroToOne {
    /// The maximum value this type can contain.
    pub const ONE: Self = Self(1.);
    /// The minimum type this type can contain.
    pub const ZERO: Self = Self(0.);

    /// Returns a new instance after clamping `value` between +0.0 and 1.0.
    ///
    /// # Panics
    ///
    /// This function panics if `value` is not a number.
    #[must_use]
    pub fn new(value: f32) -> Self {
        assert!(!value.is_nan());

        Self(value.clamp(0., 1.))
    }

    /// Returns the difference between `self` and `other` as a positive number.
    #[must_use]
    pub fn difference_between(self, other: Self) -> Self {
        Self((self.0 - other.0).abs())
    }

    /// Returns the contained floating point value.
    #[must_use]
    pub fn into_f32(self) -> f32 {
        self.0
    }
}

impl Display for ZeroToOne {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl FromStr for ZeroToOne {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl From<f32> for ZeroToOne {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl From<f64> for ZeroToOne {
    fn from(value: f64) -> Self {
        Self::new(value.cast())
    }
}

impl Default for ZeroToOne {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Deref for ZeroToOne {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Eq for ZeroToOne {}

impl PartialEq for ZeroToOne {
    fn eq(&self, other: &Self) -> bool {
        *self == other.0
    }
}

impl PartialEq<f32> for ZeroToOne {
    fn eq(&self, other: &f32) -> bool {
        (self.0 - *other).abs() < f32::EPSILON
    }
}

impl Ord for ZeroToOne {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for ZeroToOne {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd<f32> for ZeroToOne {
    fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
        Some(self.0.total_cmp(other))
    }
}

impl LinearInterpolate for ZeroToOne {
    fn lerp(&self, target: &Self, percent: f32) -> Self {
        let delta = **target - **self;
        ZeroToOne::new(**self + delta * percent)
    }
}

impl PercentBetween for ZeroToOne {
    fn percent_between(&self, min: &Self, max: &Self) -> ZeroToOne {
        self.0.percent_between(&min.0, &max.0)
    }
}

impl Mul for ZeroToOne {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Div for ZeroToOne {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

impl Ranged for ZeroToOne {
    const MAX: Self = Self::ONE;
    const MIN: Self = Self::ZERO;
}

/// An easing function for customizing animations.
#[derive(Debug, Clone)]
pub enum EasingFunction {
    /// A function pointer to use as an easing function.
    Fn(fn(ZeroToOne) -> f32),
    /// A custom easing implementation.
    Custom(Arc<dyn Easing>),
}

impl Easing for EasingFunction {
    fn ease(&self, progress: ZeroToOne) -> f32 {
        match self {
            EasingFunction::Fn(func) => func(progress),
            EasingFunction::Custom(func) => func.ease(progress),
        }
    }
}

impl From<EasingFunction> for Component {
    fn from(value: EasingFunction) -> Self {
        Component::Easing(value)
    }
}

impl TryFrom<Component> for EasingFunction {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::Easing(easing) => Ok(easing),
            other => Err(other),
        }
    }
}

/// Performs easing for value interpolation.
pub trait Easing: Debug + Send + Sync + RefUnwindSafe + UnwindSafe + 'static {
    /// Eases a value ranging between zero and one. The resulting value does not
    /// need to be bounded between zero and one.
    fn ease(&self, progress: ZeroToOne) -> f32;
}
