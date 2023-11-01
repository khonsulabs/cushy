//! Types for creating animations.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{ControlFlow, Deref};
use std::sync::{Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use alot::{LotId, Lots};
use kempt::Set;

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
                if state.animations[animation_id].animate(elapsed).is_break() {
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

struct Animating {
    animations: Lots<Box<dyn Animate>>,
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
        let id = self.animations.push(animation);

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

/// Describes a change to a new value for a [`Dynamic`] over a specified
/// [`Duration`], using the `Easing` generic parameter to control how the value
/// is interpolated.
#[must_use = "animations are not performed until they are spawned"]
pub struct Animation<T, Easing = Linear> {
    value: Dynamic<T>,
    end: T,
    duration: Duration,
    _easing: PhantomData<Easing>,
}

impl<T> Animation<T, Linear>
where
    T: LinearInterpolate + Clone + Send + Sync + 'static,
{
    /// Returns a linearly interpolated animation that transitions `value` to
    /// `end_value` over `duration`.
    pub fn linear(value: Dynamic<T>, end_value: T, duration: Duration) -> Self {
        Self::new(value, end_value, duration)
    }
}

impl<T, Easing> Animation<T, Easing>
where
    T: LinearInterpolate + Clone + Send + Sync + 'static,
    Easing: self::Easing,
{
    /// Returns an animation that transitions `value` to `end_value` over
    /// `duration` using `Easing` for interpolation.
    pub fn new(value: Dynamic<T>, end_value: T, duration: Duration) -> Self {
        Self {
            value,
            end: end_value,
            duration,
            _easing: PhantomData,
        }
    }
}

impl<T, Easing> IntoAnimate for Animation<T, Easing>
where
    T: LinearInterpolate + Clone + Send + Sync + 'static,
    Easing: self::Easing,
{
    type Animate = RunningAnimation<T, Easing>;

    fn into_animate(self) -> Self::Animate {
        RunningAnimation {
            start: self.value.get(),
            animation: self,
            elapsed: Duration::ZERO,
        }
    }
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
    fn chain<Other: IntoAnimate>(self, other: Other) -> Chain<Self, Other> {
        Chain::new(self, other)
    }
}

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
    T: LinearInterpolate + Clone + Send + Sync,
    Easing: self::Easing,
{
    fn animate(&mut self, elapsed: Duration) -> ControlFlow<Duration> {
        self.elapsed = self.elapsed.checked_add(elapsed).unwrap_or(Duration::MAX);

        if let Some(remaining_elapsed) = self.elapsed.checked_sub(self.animation.duration) {
            self.animation.value.set(self.animation.end.clone());
            ControlFlow::Break(remaining_elapsed)
        } else {
            let progress = Easing::ease(ZeroToOne::new(
                self.elapsed.as_secs_f32() / self.animation.duration.as_secs_f32(),
            ));
            self.animation
                .value
                .set(self.start.lerp(&self.animation.end, progress));
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
    animation: Animation<T, Easing>,
    start: T,
    elapsed: Duration,
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

/// Performs a linear interpolation between two values.
pub trait LinearInterpolate {
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

    /// Returns the contained floating point value.
    #[must_use]
    pub fn into_f32(self) -> f32 {
        self.0
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

/// Performs easing for value interpolation.
pub trait Easing: Send + Sync + 'static {
    /// Returns a ratio between 0.0 and 1.0 of
    fn ease(progress: ZeroToOne) -> f32;
}

/// An [`Easing`] function that produces a steady, linear transition.
pub enum Linear {}

impl Easing for Linear {
    fn ease(progress: ZeroToOne) -> f32 {
        *progress
    }
}
