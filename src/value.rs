//! Types for storing and interacting with values in Widgets.

use std::collections::HashMap;
use std::fmt::{self, Debug, Display};
use std::future::Future;
use std::hash::{BuildHasher, Hash};
use std::ops::{Add, AddAssign, Deref, DerefMut, Not};
use std::str::FromStr;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, TryLockError, Weak};
use std::task::{Poll, Waker};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use ahash::AHashSet;
use alot::{LotId, Lots};
use intentional::Assert;
use kempt::{Map, Sort};

use crate::animation::{AnimationHandle, DynamicTransition, IntoAnimate, LinearInterpolate, Spawn};
use crate::context::{self, WidgetContext};
use crate::utils::{run_in_bg, IgnorePoison, WithClone};
use crate::widget::{
    Children, MakeWidget, MakeWidgetWithTag, OnceCallback, WidgetId, WidgetInstance,
};
use crate::widgets::{Radio, Select, Space, Switcher};
use crate::window::WindowHandle;

/// An instance of a value that provides APIs to observe and react to its
/// contents.
pub struct Dynamic<T>(Arc<DynamicData<T>>);

impl<T> Dynamic<T> {
    /// Creates a new instance wrapping `value`.
    pub fn new(value: T) -> Self {
        Self(Arc::new(DynamicData {
            state: Mutex::new(State {
                wrapped: GenerationalValue {
                    value,
                    generation: Generation::default(),
                },
                callbacks: Arc::default(),
                windows: AHashSet::new(),
                readers: 0,
                wakers: Vec::new(),
                widgets: AHashSet::new(),
                on_disconnect: Vec::new(),
                source_callback: CallbackHandle::default(),
            }),
            during_callback_state: Mutex::default(),
            sync: Condvar::default(),
        }))
    }

    /// Returns a weak reference to this dynamic.
    ///
    /// This is powered by [`Arc`]/[`Weak`] and follows the same semantics for
    /// reference counting.
    #[must_use]
    pub fn downgrade(&self) -> WeakDynamic<T> {
        WeakDynamic::from(self)
    }

    /// Returns a new dynamic that has its contents linked with `self` by the
    /// pair of mapping functions provided.
    ///
    /// When the returned dynamic is updated, `r_into_t` will be invoked. This
    /// function accepts `&R` and can return `T`, or `Option<T>`. If a value is
    /// produced, `self` will be updated with the new value.
    ///
    /// When `self` is updated, `t_into_r` will be invoked. This function
    /// accepts `&T` and can return `R` or `Option<R>`. If a value is produced,
    /// the returned dynamic will be updated with the new value.
    ///
    /// # Panics
    ///
    /// This function panics if calling `t_into_r` with the current contents of
    /// the Dynamic produces a `None` value. This requirement is only for the
    /// first invocation, and it is guaranteed to occur before this function
    /// returns.
    pub fn linked<R, TIntoR, TIntoRResult, RIntoT, RIntoTResult>(
        &self,
        mut t_into_r: TIntoR,
        mut r_into_t: RIntoT,
    ) -> Dynamic<R>
    where
        T: PartialEq + Send + 'static,
        R: PartialEq + Send + 'static,
        TIntoRResult: Into<Option<R>> + Send + 'static,
        RIntoTResult: Into<Option<T>> + Send + 'static,
        TIntoR: FnMut(&T) -> TIntoRResult + Send + 'static,
        RIntoT: FnMut(&R) -> RIntoTResult + Send + 'static,
    {
        let initial_r = self
            .map_ref(|v| t_into_r(v))
            .into()
            .expect("t_into_r must succeed with the current value");
        let r = Dynamic::new(initial_r);
        r.with_clone(move |r| {
            self.for_each(move |t| {
                if let Some(update) = t_into_r(t).into() {
                    let _result = r.replace(update);
                }
            })
            .persist();
        });

        self.with_clone(|t| {
            r.with_for_each(move |r| {
                if let Some(update) = r_into_t(r).into() {
                    let _result = t.replace(update);
                }
            })
        })
    }

    /// Creates a [linked](Self::linked) dynamic containing a `String`.
    ///
    /// When `self` is updated, [`ToString::to_string()`] will be called to
    /// produce a new string value to store in the returned dynamic.
    ///
    /// When the returned dynamic is updated, [`str::parse`](std::str) is called
    /// to produce a new `T`. If an error is returned, `self` will not be
    /// updated. Otherwise, `self` will be updated with the produced value.
    #[must_use]
    pub fn linked_string(&self) -> Dynamic<String>
    where
        T: ToString + FromStr + PartialEq + Send + 'static,
    {
        self.linked(ToString::to_string, |s: &String| s.parse().ok())
    }

    /// Maps the contents with read-only access.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    pub fn map_ref<R>(&self, map: impl FnOnce(&T) -> R) -> R {
        self.map_generational(|gen| map(&gen.value))
    }

    /// Maps the contents with read-only access, providing access to the value's
    /// [`Generation`].
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    pub fn map_generational<R>(&self, map: impl FnOnce(&GenerationalValue<T>) -> R) -> R {
        let state = self.state().expect("deadlocked");
        map(&state.wrapped)
    }

    /// Maps the contents with exclusive access. Before returning from this
    /// function, all observers will be notified that the contents have been
    /// updated.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    pub fn map_mut<R>(&self, map: impl FnOnce(&mut T) -> R) -> R {
        self.0.map_mut(|value, _| map(value)).expect("deadlocked")
    }

    /// Updates the value to the result of invoking [`Not`] on the current
    /// value. This function returns the new value.
    #[allow(clippy::must_use_candidate)]
    pub fn toggle(&self) -> T
    where
        T: Not<Output = T> + Clone,
    {
        self.map_mut(|value| {
            *value = !value.clone();
            value.clone()
        })
    }

    /// Sets the current `source` for this dynamic with `source`.
    ///
    /// A dynamic can have multiple source callbacks.
    ///
    /// This ensures that `source` stays active as long as any clones of `self`
    /// are alive.
    pub fn set_source(&self, source: CallbackHandle) {
        self.state().assert("deadlocked").source_callback += source;
    }

    /// Returns a new dynamic that contains the updated contents of this dynamic
    /// at most once every `period`.
    #[must_use]
    pub fn debounced_every(&self, period: Duration) -> Self
    where
        T: PartialEq + Clone + Send + Sync + 'static,
    {
        let debounced = Dynamic::new(self.get());
        let mut debounce = Debounce::new(debounced.clone(), period);
        let callback = self.for_each_cloned(move |value| debounce.update(value));
        debounced.set_source(callback);
        debounced
    }

    /// Returns a new dynamic that contains the updated contents of this dynamic
    /// delayed by `period`. Each time this value is updated, the delay is
    /// reset.
    #[must_use]
    pub fn debounced_with_delay(&self, period: Duration) -> Self
    where
        T: PartialEq + Clone + Send + Sync + 'static,
    {
        let debounced = Dynamic::new(self.get());
        let mut debounce = Debounce::new(debounced.clone(), period).extending();
        let callback = self.for_each_cloned(move |value| debounce.update(value));
        debounced.set_source(callback);
        debounced
    }

    /// Returns a new dynamic that is updated using `U::from(T.clone())` each
    /// time `self` is updated.
    #[must_use]
    pub fn map_each_into<U>(&self) -> Dynamic<U>
    where
        U: PartialEq + From<T> + Send + 'static,
        T: Clone + Send + 'static,
    {
        self.map_each(|value| U::from(value.clone()))
    }

    /// Returns a new dynamic that is updated using `U::from(&T)` each
    /// time `self` is updated.
    #[must_use]
    pub fn map_each_to<U>(&self) -> Dynamic<U>
    where
        U: PartialEq + for<'a> From<&'a T> + Send + 'static,
        T: Clone + Send + 'static,
    {
        self.map_each(|value| U::from(value))
    }

    /// Returns a new dynamic that contains a weak clone of `self`'s contents.
    ///
    /// The returned `Dynamic` does not use any strong references, ensuring the
    /// returned dynamic does not extend the lifetime of `self`.
    #[must_use]
    pub fn weak_clone(&self) -> Self
    where
        T: Clone + PartialEq + Send + 'static,
    {
        let weak_source = self.downgrade();
        let weak_out = Dynamic::new(self.get());
        weak_out.set_source(self.0.for_each({
            let weak_out = weak_out.clone();
            move || {
                if let Some(source) = weak_source.upgrade() {
                    weak_out.set(source.get());
                }
            }
        }));
        weak_out
    }

    /// Attaches `for_each` to this value so that it is invoked each time the
    /// value's contents are updated.
    pub fn for_each<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a T) + Send + 'static,
    {
        let this = self.clone();
        self.0.for_each(move || {
            this.map_ref(&mut for_each);
        })
    }

    /// Attaches `for_each` to this value and its [`Generation`] so that it is
    /// invoked each time the value's contents are updated.
    pub fn for_each_generational<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a GenerationalValue<T>) + Send + 'static,
    {
        let this = self.clone();
        self.0.for_each(move || {
            this.map_generational(&mut for_each);
        })
    }

    /// Attaches `for_each` to this value so that it is invoked each time the
    /// value's contents are updated.
    pub fn for_each_cloned<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Clone + Send + 'static,
        F: FnMut(T) + Send + 'static,
    {
        let this = self.clone();
        self.0.for_each(move || {
            for_each(this.get());
        })
    }

    /// Attaches `for_each` to this value so that it is invoked each time the
    /// value's contents are updated. This function returns `self`.
    #[must_use]
    pub fn with_for_each<F>(self, for_each: F) -> Self
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a T) + Send + 'static,
    {
        self.for_each(for_each).persist();
        self
    }

    /// Creates a new dynamic value that contains the result of invoking `map`
    /// each time this value is changed.
    pub fn map_each<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a T) -> R + Send + 'static,
        R: PartialEq + Send + 'static,
    {
        let this = self.clone();
        self.0.map_each(move || this.map_ref(&mut map))
    }

    /// Creates a new dynamic value that contains the result of invoking `map`
    /// each time this value is changed.
    pub fn map_each_generational<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a GenerationalValue<T>) -> R + Send + 'static,
        R: PartialEq + Send + 'static,
    {
        let this = self.clone();
        self.0.map_each(move || this.map_generational(&mut map))
    }

    /// Creates a new dynamic value that contains the result of invoking `map`
    /// each time this value is changed.
    pub fn map_each_cloned<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        T: Clone + Send + 'static,
        F: FnMut(T) -> R + Send + 'static,
        R: PartialEq + Send + 'static,
    {
        let this = self.clone();
        self.0.map_each(move || map(this.get()))
    }

    /// A helper function that invokes `with_clone` with a clone of self. This
    /// code may produce slightly more readable code.
    ///
    /// ```rust
    /// let value = cushy::value::Dynamic::new(1);
    ///
    /// // Using with_clone
    /// value.with_clone(|value| {
    ///     std::thread::spawn(move || {
    ///         println!("{}", value.get());
    ///     })
    /// });
    ///
    /// // Using an explicit clone
    /// std::thread::spawn({
    ///     let value = value.clone();
    ///     move || {
    ///         println!("{}", value.get());
    ///     }
    /// });
    ///
    /// println!("{}", value.get());
    /// ````
    pub fn with_clone<R>(&self, with_clone: impl FnOnce(Self) -> R) -> R {
        with_clone(self.clone())
    }

    pub(crate) fn redraw_when_changed(&self, window: WindowHandle) {
        self.0.redraw_when_changed(window);
    }

    pub(crate) fn invalidate_when_changed(&self, window: WindowHandle, widget: WidgetId) {
        self.0.invalidate_when_changed(window, widget);
    }

    /// Returns a clone of the currently contained value.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.0.get().expect("deadlocked").value
    }

    /// Returns a clone of the currently contained value.
    ///
    /// `context` will be invalidated when the value is updated.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn get_tracking_redraw(&self, context: &WidgetContext<'_, '_>) -> T
    where
        T: Clone,
    {
        context.redraw_when_changed(self);
        self.get()
    }

    /// Returns a clone of the currently contained value.
    ///
    /// `context` will be invalidated when the value is updated.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn get_tracking_invalidate(&self, context: &WidgetContext<'_, '_>) -> T
    where
        T: Clone,
    {
        context.invalidate_when_changed(self);
        self.get()
    }

    /// Returns the currently stored value, replacing the current contents with
    /// `T::default()`.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn take(&self) -> T
    where
        T: Default,
    {
        std::mem::take(&mut self.lock())
    }

    /// Checks if the currently stored value is different than `T::default()`,
    /// and if so, returns `Some(self.take())`.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn take_if_not_default(&self) -> Option<T>
    where
        T: Default + PartialEq,
    {
        let default = T::default();
        let mut guard = self.lock();
        if *guard == default {
            None
        } else {
            Some(std::mem::replace(&mut guard, default))
        }
    }

    /// Replaces the contents with `new_value`, returning the previous contents.
    /// Before returning from this function, all observers will be notified that
    /// the contents have been updated.
    ///
    /// If the calling thread has exclusive access to the contents of this
    /// dynamic, this call will return None and the value will not be updated.
    /// If detecting this is important, use [`Self::try_replace()`].
    pub fn replace(&self, new_value: T) -> Option<T>
    where
        T: PartialEq,
    {
        self.try_replace(new_value).ok()
    }

    /// Replaces the contents with `new_value` if `new_value` is different than
    /// the currently stored value. If the value is updated, the previous
    /// contents are returned.
    ///
    ///
    /// Before returning from this function, all observers will be notified that
    /// the contents have been updated.
    ///
    /// # Errors
    ///
    /// - [`ReplaceError::NoChange`]: Returned when `new_value` is equal to the
    /// currently stored value.
    /// - [`ReplaceError::Deadlock`]: Returned when the current thread already
    ///       has exclusive access to the contents of this dynamic.
    pub fn try_replace(&self, new_value: T) -> Result<T, ReplaceError<T>>
    where
        T: PartialEq,
    {
        match self.0.map_mut(|value, changed| {
            if *value == new_value {
                *changed = false;
                Err(ReplaceError::NoChange(new_value))
            } else {
                Ok(std::mem::replace(value, new_value))
            }
        }) {
            Ok(old) => old,
            Err(_) => Err(ReplaceError::Deadlock),
        }
    }

    /// Stores `new_value` in this dynamic. Before returning from this function,
    /// all observers will be notified that the contents have been updated.
    ///
    /// If the calling thread has exclusive access to the contents of this
    /// dynamic, this call will return None and the value will not be updated.
    /// If detecting this is important, use [`Self::try_replace()`].
    pub fn set(&self, new_value: T)
    where
        T: PartialEq,
    {
        let _old = self.replace(new_value);
    }

    /// Replaces the current value with `new_value` if the current value is
    /// equal to `expected_current`.
    ///
    /// Returns `Ok` with the overwritten value upon success.
    ///
    /// # Errors
    ///
    /// - [`TryCompareSwapError::Deadlock`]: This operation would result in a
    ///       thread deadlock.
    /// - [`TryCompareSwapError::CurrentValueMismatch`]: The current value did
    ///       not match `expected_current`. The `T` returned is a clone of the
    ///       currently stored value.
    pub fn try_compare_swap(
        &self,
        expected_current: &T,
        new_value: T,
    ) -> Result<T, TryCompareSwapError<T>>
    where
        T: Clone + PartialEq,
    {
        match self.0.map_mut(|value, changed| {
            if value == expected_current {
                Ok(std::mem::replace(value, new_value))
            } else {
                *changed = false;
                Err(TryCompareSwapError::CurrentValueMismatch(value.clone()))
            }
        }) {
            Ok(old) => old,
            Err(_) => Err(TryCompareSwapError::Deadlock),
        }
    }

    /// Replaces the current value with `new_value` if the current value is
    /// equal to `expected_current`.
    ///
    /// Returns `Ok` with the overwritten value upon success.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the currently stored value when `expected_current`
    /// does not match the currently stored value.
    pub fn compare_swap(&self, expected_current: &T, new_value: T) -> Result<T, T>
    where
        T: Clone + PartialEq,
    {
        match self.try_compare_swap(expected_current, new_value) {
            Ok(old) => Ok(old),
            Err(TryCompareSwapError::Deadlock) => unreachable!("deadlocked"),
            Err(TryCompareSwapError::CurrentValueMismatch(value)) => Err(value),
        }
    }

    /// Returns a new reference-based reader for this dynamic value.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn create_reader(&self) -> DynamicReader<T> {
        self.state().expect("deadlocked").readers += 1;
        DynamicReader {
            source: self.0.clone(),
            read_generation: self.0.state().expect("deadlocked").wrapped.generation,
        }
    }

    /// Converts this [`Dynamic`] into a reader.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn into_reader(self) -> DynamicReader<T> {
        self.create_reader()
    }

    /// Returns an exclusive reference to the contents of this dynamic.
    ///
    /// This call will block until all other guards for this dynamic have been
    /// dropped.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn lock(&self) -> DynamicGuard<'_, T> {
        DynamicGuard {
            guard: self.0.state().expect("deadlocked"),
            accessed_mut: false,
            prevent_notifications: false,
        }
    }

    fn state(&self) -> Result<DynamicMutexGuard<'_, T>, DeadlockError> {
        self.0.state()
    }

    /// Returns the current generation of the value.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn generation(&self) -> Generation {
        self.state().expect("deadlocked").wrapped.generation
    }

    /// Returns a pending transition for this value to `new_value`.
    pub fn transition_to(&self, new_value: T) -> DynamicTransition<T>
    where
        T: LinearInterpolate + Clone + Send + Sync,
    {
        DynamicTransition {
            dynamic: self.clone(),
            new_value,
        }
    }

    /// Returns a new [`Radio`] that updates this dynamic to `widget_value` when
    /// pressed. `label` is drawn next to the checkbox and is also clickable to
    /// select the radio.
    #[must_use]
    pub fn new_radio(&self, widget_value: T, label: impl MakeWidget) -> Radio<T>
    where
        Self: Clone,
        // Technically this trait bound isn't necessary, but it prevents trying
        // to call new_radio on unsupported types. The MakeWidget/Widget
        // implementations require these bounds (and more).
        T: Clone + Eq,
    {
        Radio::new(widget_value, self.clone(), label)
    }

    /// Returns a new [`Select`] that updates this dynamic to `widget_value`
    /// when pressed. `label` is drawn next to the checkbox and is also
    /// clickable to select the widget.
    #[must_use]
    pub fn new_select(&self, widget_value: T, label: impl MakeWidget) -> Select<T>
    where
        Self: Clone,
        // Technically this trait bound isn't necessary, but it prevents trying
        // to call new_select on unsupported types. The MakeWidget/Widget
        // implementations require these bounds (and more).
        T: Clone + Eq,
    {
        Select::new(widget_value, self.clone(), label)
    }

    /// Validates the contents of this dynamic using the `check` function,
    /// returning a dynamic that contains the validation status.
    #[must_use]
    pub fn validate_with<E, Valid>(&self, mut check: Valid) -> Dynamic<Validation>
    where
        T: Send + 'static,
        Valid: for<'a> FnMut(&'a T) -> Result<(), E> + Send + 'static,
        E: Display,
    {
        let validation = Dynamic::new(Validation::None);
        let callback = self.for_each({
            let validation = validation.clone();
            move |value| {
                validation.set(match check(value) {
                    Ok(()) => Validation::Valid,
                    Err(err) => Validation::Invalid(err.to_string()),
                });
            }
        });
        validation.set_source(callback);
        validation
    }
}

/// An error returned from [`Dynamic::try_compare_swap`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TryCompareSwapError<T> {
    /// The dynamic is already locked for exclusive access by the current
    /// thread. This operation would result in a deadlock.
    Deadlock,
    /// The current value did not match the expected value. This variant's value
    /// is the value at the time of comparison.
    CurrentValueMismatch(T),
}

impl<T> Debug for Dynamic<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&DebugDynamicData(&self.0), f)
    }
}

impl Dynamic<WidgetInstance> {
    /// Returns a new [`Switcher`] widget whose contents is the value of this
    /// dynamic.
    #[must_use]
    pub fn into_switcher(self) -> Switcher {
        Switcher::new(self)
    }
}

impl MakeWidgetWithTag for Dynamic<WidgetInstance> {
    fn make_with_tag(self, id: crate::widget::WidgetTag) -> WidgetInstance {
        self.into_switcher().make_with_tag(id)
    }
}

impl MakeWidgetWithTag for Dynamic<Option<WidgetInstance>> {
    fn make_with_tag(self, id: crate::widget::WidgetTag) -> WidgetInstance {
        self.map_each(|widget| {
            widget
                .as_ref()
                .map_or_else(|| Space::clear().make_widget(), Clone::clone)
        })
        .make_with_tag(id)
    }
}

impl<T> context::sealed::Trackable for Dynamic<T> {
    fn redraw_when_changed(&self, handle: WindowHandle) {
        self.redraw_when_changed(handle);
    }

    fn invalidate_when_changed(&self, handle: WindowHandle, id: WidgetId) {
        self.invalidate_when_changed(handle, id);
    }
}

impl<T> Eq for Dynamic<T> {}

impl<T> PartialEq for Dynamic<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Default for Dynamic<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Clone for Dynamic<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Drop for Dynamic<T> {
    fn drop(&mut self) {
        // Ignoring deadlocks here allows complex flows to work properly, and
        // the only issue is that `on_disconnect` will not fire if during a map
        // callback on a `DynamicReader` the final reference to the source
        // `Dynamic`.
        if let Ok(mut state) = self.state() {
            if Arc::strong_count(&self.0) == state.readers + 1 {
                let on_disconnect = std::mem::take(&mut state.on_disconnect);
                drop(state);

                for on_disconnect in on_disconnect {
                    on_disconnect.invoke(());
                }

                self.0.sync.notify_all();
            }
        } else {
            // In the event that this is the rare edge case and a reader is
            // blocking, we want to signal that we've dropped the final
            // reference.
            self.0.sync.notify_all();
        }
    }
}

impl<T> From<Dynamic<T>> for DynamicReader<T> {
    fn from(value: Dynamic<T>) -> Self {
        value.create_reader()
    }
}

impl From<&str> for Dynamic<String> {
    fn from(value: &str) -> Self {
        Dynamic::from(value.to_string())
    }
}

impl From<String> for Dynamic<String> {
    fn from(value: String) -> Self {
        Dynamic::new(value)
    }
}

struct DynamicMutexGuard<'a, T> {
    dynamic: &'a DynamicData<T>,
    guard: MutexGuard<'a, State<T>>,
}

impl<T> Debug for DynamicMutexGuard<'_, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.guard.debug("DynamicMutexGuard", f)
    }
}

impl<'a, T> Drop for DynamicMutexGuard<'a, T> {
    fn drop(&mut self) {
        let mut during_state = self.dynamic.during_callback_state.lock().ignore_poison();
        *during_state = None;
        drop(during_state);
        self.dynamic.sync.notify_all();
    }
}

impl<'a, T> Deref for DynamicMutexGuard<'a, T> {
    type Target = State<T>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}
impl<'a, T> DerefMut for DynamicMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

#[derive(Debug)]
struct LockState {
    locked_thread: ThreadId,
}

struct DynamicData<T> {
    state: Mutex<State<T>>,
    during_callback_state: Mutex<Option<LockState>>,
    sync: Condvar,
}

impl<T> DynamicData<T> {
    fn state(&self) -> Result<DynamicMutexGuard<'_, T>, DeadlockError> {
        let mut during_sync = self.during_callback_state.lock().ignore_poison();

        let current_thread_id = std::thread::current().id();
        let guard = loop {
            match self.state.try_lock() {
                Ok(g) => break g,
                Err(TryLockError::Poisoned(poision)) => break poision.into_inner(),
                Err(TryLockError::WouldBlock) => loop {
                    match &*during_sync {
                        Some(state) if state.locked_thread == current_thread_id => {
                            return Err(DeadlockError)
                        }
                        Some(_) => {
                            during_sync = self.sync.wait(during_sync).ignore_poison();
                        }
                        None => break,
                    }
                },
            }
        };
        *during_sync = Some(LockState {
            locked_thread: current_thread_id,
        });
        Ok(DynamicMutexGuard {
            dynamic: self,
            guard,
        })
    }

    pub fn redraw_when_changed(&self, window: WindowHandle) {
        let mut state = self.state().expect("deadlocked");
        state.windows.insert(window);
    }

    pub fn invalidate_when_changed(&self, window: WindowHandle, widget: WidgetId) {
        let mut state = self.state().expect("deadlocked");
        state.widgets.insert((window, widget));
    }

    pub fn get(&self) -> Result<GenerationalValue<T>, DeadlockError>
    where
        T: Clone,
    {
        self.state().map(|state| state.wrapped.clone())
    }

    pub fn map_mut<R>(&self, map: impl FnOnce(&mut T, &mut bool) -> R) -> Result<R, DeadlockError> {
        let mut state = self.state()?;
        let (old, callbacks) = {
            let state = &mut *state;
            let mut changed = true;
            let result = map(&mut state.wrapped.value, &mut changed);
            let callbacks = changed.then(|| state.note_changed());

            (result, callbacks)
        };
        drop(state);
        drop(callbacks);

        self.sync.notify_all();

        Ok(old)
    }

    pub fn for_each<F>(&self, map: F) -> CallbackHandle
    where
        F: for<'a> FnMut() + Send + 'static,
    {
        let state = self.state().expect("deadlocked");
        let mut data = state.callbacks.callbacks.lock().ignore_poison();
        CallbackHandle(CallbackHandleInner::Single(CallbackHandleData {
            id: Some(data.callbacks.push(Box::new(map))),
            callbacks: state.callbacks.clone(),
        }))
    }

    pub fn map_each<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        F: for<'a> FnMut() -> R + Send + 'static,
        R: PartialEq + Send + 'static,
    {
        let initial_value = map();
        let mapped_value = Dynamic::new(initial_value);
        let returned = mapped_value.clone();

        let callback = self.for_each(move || {
            mapped_value.set(map());
        });

        returned.set_source(callback);

        returned
    }
}

struct DebugDynamicData<'a, T>(&'a Arc<DynamicData<T>>);

impl<T> Debug for DebugDynamicData<'_, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.state() {
            Ok(state) => state.debug("Dynamic", f),
            Err(_) => f.debug_tuple("Dynamic").field(&"<unable to lock>").finish(),
        }
    }
}

/// An error occurred while updating a value in a [`Dynamic`].
pub enum ReplaceError<T> {
    /// The value was already equal to the one set.
    NoChange(T),
    /// The current thread already has exclusive access to this dynamic.
    Deadlock,
}

/// A deadlock occurred accessing a [`Dynamic`].
///
/// Currently Cushy is only able to detect deadlocks where a single thread tries
/// to lock the same [`Dynamic`] multiple times.
#[derive(Debug)]
struct DeadlockError;

impl std::error::Error for DeadlockError {}

impl Display for DeadlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a deadlock was detected")
    }
}

/// A handle to a callback installed on a [`Dynamic`]. When dropped, the
/// callback will be uninstalled.
///
/// To prevent the callback from ever being uninstalled, use
/// [`Self::persist()`].
#[must_use]
pub struct CallbackHandle(CallbackHandleInner);

impl Default for CallbackHandle {
    fn default() -> Self {
        Self(CallbackHandleInner::None)
    }
}

enum CallbackHandleInner {
    None,
    Single(CallbackHandleData),
    Multi(Vec<CallbackHandleData>),
}

struct CallbackHandleData {
    id: Option<LotId>,
    callbacks: Arc<ChangeCallbacksData>,
}

impl Debug for CallbackHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tuple = f.debug_tuple("CallbackHandle");
        match &self.0 {
            CallbackHandleInner::None => {}
            CallbackHandleInner::Single(handle) => {
                tuple.field(&handle.id);
            }
            CallbackHandleInner::Multi(handles) => {
                for handle in handles {
                    tuple.field(&handle.id);
                }
            }
        }

        tuple.finish()
    }
}

impl CallbackHandle {
    /// Persists the callback so that it will always be invoked until the
    /// dynamic is freed.
    pub fn persist(self) {
        match self.0 {
            CallbackHandleInner::None => {}
            CallbackHandleInner::Single(mut handle) => {
                let _id = handle.id.take();
                drop(handle);
            }
            CallbackHandleInner::Multi(handles) => {
                for handle in handles {
                    handle.persist();
                }
            }
        }
    }
}

impl Eq for CallbackHandle {}

impl PartialEq for CallbackHandle {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (CallbackHandleInner::None, CallbackHandleInner::None) => true,
            (CallbackHandleInner::Single(this), CallbackHandleInner::Single(other)) => {
                this == other
            }
            (CallbackHandleInner::Multi(this), CallbackHandleInner::Multi(other)) => this == other,
            _ => false,
        }
    }
}

impl CallbackHandleData {
    fn persist(mut self) {
        let _id = self.id.take();
        drop(self);
    }
}

impl Drop for CallbackHandleData {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            let mut data = self.callbacks.callbacks.lock().ignore_poison();
            data.callbacks.remove(id);
        }
    }
}
impl PartialEq for CallbackHandleData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && Arc::ptr_eq(&self.callbacks, &other.callbacks)
    }
}

impl Add for CallbackHandle {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for CallbackHandle {
    fn add_assign(&mut self, rhs: Self) {
        match (&mut self.0, rhs.0) {
            (_, CallbackHandleInner::None) => {}
            (CallbackHandleInner::None, other) => {
                self.0 = other;
            }
            (CallbackHandleInner::Single(_), CallbackHandleInner::Single(other)) => {
                let CallbackHandleInner::Single(single) =
                    std::mem::replace(&mut self.0, CallbackHandleInner::Multi(vec![other]))
                else {
                    unreachable!("just matched")
                };
                let CallbackHandleInner::Multi(multi) = &mut self.0 else {
                    unreachable!("just replaced")
                };
                multi.push(single);
            }
            (CallbackHandleInner::Single(_), CallbackHandleInner::Multi(multi)) => {
                let CallbackHandleInner::Single(single) =
                    std::mem::replace(&mut self.0, CallbackHandleInner::Multi(multi))
                else {
                    unreachable!("just matched")
                };
                let CallbackHandleInner::Multi(multi) = &mut self.0 else {
                    unreachable!("just replaced")
                };
                multi.push(single);
            }
            (CallbackHandleInner::Multi(this), CallbackHandleInner::Single(single)) => {
                this.push(single);
            }
            (CallbackHandleInner::Multi(this), CallbackHandleInner::Multi(mut other)) => {
                this.append(&mut other);
            }
        }
    }
}

struct State<T> {
    wrapped: GenerationalValue<T>,
    source_callback: CallbackHandle,
    callbacks: Arc<ChangeCallbacksData>,
    windows: AHashSet<WindowHandle>,
    widgets: AHashSet<(WindowHandle, WidgetId)>,
    wakers: Vec<Waker>,
    on_disconnect: Vec<OnceCallback>,
    readers: usize,
}

impl<T> State<T> {
    fn note_changed(&mut self) -> ChangeCallbacks {
        self.wrapped.generation = self.wrapped.generation.next();

        for (window, widget) in self.widgets.drain() {
            window.invalidate(widget);
        }
        for window in self.windows.drain() {
            window.redraw();
        }
        for waker in self.wakers.drain(..) {
            waker.wake();
        }

        ChangeCallbacks {
            data: self.callbacks.clone(),
            changed_at: Instant::now(),
        }
    }

    fn debug(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result
    where
        T: Debug,
    {
        f.debug_struct(name)
            .field("value", &self.wrapped.value)
            .field("generation", &self.wrapped.generation.0)
            .finish()
    }
}

impl<T> Debug for State<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("wrapped", &self.wrapped)
            .field("readers", &self.readers)
            .finish_non_exhaustive()
    }
}

#[derive(Default)]
struct ChangeCallbacksData {
    callbacks: Mutex<CallbacksList>,
    currently_executing: Mutex<Option<ThreadId>>,
    sync: Condvar,
}

struct CallbacksList {
    callbacks: Lots<Box<dyn ValueCallback>>,
    invoked_at: Instant,
}

impl Default for CallbacksList {
    fn default() -> Self {
        Self {
            callbacks: Lots::new(),
            invoked_at: Instant::now(),
        }
    }
}

struct ChangeCallbacks {
    data: Arc<ChangeCallbacksData>,
    changed_at: Instant,
}

impl Drop for ChangeCallbacks {
    fn drop(&mut self) {
        let mut currently_executing = self.data.currently_executing.lock().expect("lock poisoned");
        let current_thread = thread::current().id();
        loop {
            match &*currently_executing {
                None => {
                    // No other thread is executing these callbacks. Set this
                    // thread as the current executor so that we can prevent
                    // infinite cycles.
                    *currently_executing = Some(current_thread);
                    drop(currently_executing);

                    // Invoke the callbacks
                    let mut state = self.data.callbacks.lock().ignore_poison();
                    // If the callbacks have already been invoked by another
                    // thread such that the callbacks observed the value our
                    // thread wrote, we can skip the callbacks.
                    if state.invoked_at < self.changed_at {
                        state.invoked_at = Instant::now();
                        for callback in &mut state.callbacks {
                            callback.changed();
                        }
                    }
                    drop(state);

                    // Remove ourselves as the current executor, notifying any
                    // other threads that are waiting.
                    currently_executing =
                        self.data.currently_executing.lock().expect("lock poisoned");
                    *currently_executing = None;
                    drop(currently_executing);
                    self.data.sync.notify_all();

                    return;
                }
                Some(executing) if executing == &current_thread => {
                    tracing::warn!("Could not invoke dynamic callbacks because they are already running on this thread");

                    return;
                }
                Some(_) => {
                    currently_executing = self
                        .data
                        .sync
                        .wait(currently_executing)
                        .expect("lock poisoned");
                }
            }
        }
    }
}

trait ValueCallback: Send {
    fn changed(&mut self);
}

impl<F> ValueCallback for F
where
    F: for<'a> FnMut() + Send + 'static,
{
    fn changed(&mut self) {
        self();
    }
}

/// A value stored in a [`Dynamic`] with its [`Generation`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationalValue<T> {
    /// The stored value.
    pub value: T,
    generation: Generation,
}

impl<T> GenerationalValue<T> {
    /// Returns the generation of this value.
    ///
    /// Each time a [`Dynamic`] is updated, the generation is also updated. This
    /// value can be used to track whether a particular value has been observed.
    pub const fn generation(&self) -> Generation {
        self.generation
    }

    /// Returns a new instance containing the result of invoking `map` with
    /// `self.value`.
    ///
    /// The returned instance will have the same generation as this instance.
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> GenerationalValue<U> {
        GenerationalValue {
            value: map(self.value),
            generation: self.generation,
        }
    }

    /// Returns a new instance containing the result of invoking `map` with
    /// `&self.value`.
    ///
    /// The returned instance will have the same generation as this instance.
    pub fn map_ref<U>(&self, map: impl for<'a> FnOnce(&'a T) -> U) -> GenerationalValue<U> {
        GenerationalValue {
            value: map(&self.value),
            generation: self.generation,
        }
    }
}

impl<T> Deref for GenerationalValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for GenerationalValue<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

/// An exclusive reference to the contents of a [`Dynamic`].
///
/// If the contents are accessed through [`DerefMut`], all obververs will be
/// notified of a change when this guard is dropped.
#[derive(Debug)]
pub struct DynamicGuard<'a, T> {
    guard: DynamicMutexGuard<'a, T>,
    accessed_mut: bool,
    prevent_notifications: bool,
}

impl<T> DynamicGuard<'_, T> {
    /// Returns the generation of the value at the time of locking the dynamic.
    ///
    /// Even if this guard accesses the data through [`DerefMut`], this value
    /// will remain unchanged while the guard is held.
    #[must_use]
    pub fn generation(&self) -> Generation {
        self.guard.wrapped.generation
    }

    /// Prevent any access through [`DerefMut`] from triggering change
    /// notifications.
    pub fn prevent_notifications(&mut self) {
        self.prevent_notifications = true;
    }
}

impl<'a, T> Deref for DynamicGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard.wrapped.value
    }
}

impl<'a, T> DerefMut for DynamicGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.accessed_mut = true;
        &mut self.guard.wrapped.value
    }
}

impl<T> Drop for DynamicGuard<'_, T> {
    fn drop(&mut self) {
        if self.accessed_mut && !self.prevent_notifications {
            let mut callbacks = Some(self.guard.note_changed());
            run_in_bg(move || drop(callbacks.take()));
        }
    }
}

/// A weak reference to a [`Dynamic`].
///
/// This is powered by [`Arc`]/[`Weak`] and follows the same semantics for
/// reference counting.
pub struct WeakDynamic<T>(Weak<DynamicData<T>>);

impl<T> WeakDynamic<T> {
    /// Returns the [`Dynamic`] this weak reference points to, unless no
    /// remaining [`Dynamic`] instances exist for the underlying value.
    #[must_use]
    pub fn upgrade(&self) -> Option<Dynamic<T>> {
        self.0.upgrade().map(Dynamic)
    }
}
impl<T> Debug for WeakDynamic<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(strong) = self.upgrade() {
            Debug::fmt(&strong, f)
        } else {
            f.debug_tuple("WeakDynamic")
                .field(&"<pending drop>")
                .finish()
        }
    }
}

impl<'a, T> From<&'a Dynamic<T>> for WeakDynamic<T> {
    fn from(value: &'a Dynamic<T>) -> Self {
        Self(Arc::downgrade(&value.0))
    }
}

impl<T> From<Dynamic<T>> for WeakDynamic<T> {
    fn from(value: Dynamic<T>) -> Self {
        Self::from(&value)
    }
}

impl<T> Clone for WeakDynamic<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Eq for WeakDynamic<T> {}

impl<T> PartialEq for WeakDynamic<T> {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}

impl<T> PartialEq<Dynamic<T>> for WeakDynamic<T> {
    fn eq(&self, other: &Dynamic<T>) -> bool {
        Weak::as_ptr(&self.0) == Arc::as_ptr(&other.0)
    }
}

impl<T> PartialEq<WeakDynamic<T>> for Dynamic<T> {
    fn eq(&self, other: &WeakDynamic<T>) -> bool {
        Arc::as_ptr(&self.0) == Weak::as_ptr(&other.0)
    }
}

/// A reader that tracks the last generation accessed through this reader.
pub struct DynamicReader<T> {
    source: Arc<DynamicData<T>>,
    read_generation: Generation,
}

impl<T> DynamicReader<T> {
    /// Maps the contents of the dynamic value and returns the result.
    ///
    /// This function marks the currently stored value as being read.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    pub fn map_ref<R>(&mut self, map: impl FnOnce(&T) -> R) -> R {
        let state = self.source.state().expect("deadlocked");
        self.read_generation = state.wrapped.generation;
        map(&state.wrapped.value)
    }

    /// Returns true if the dynamic has been modified since the last time the
    /// value was accessed through this reader.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn has_updated(&self) -> bool {
        self.source.state().expect("deadlocked").wrapped.generation != self.read_generation
    }

    /// Returns a clone of the currently contained value.
    ///
    /// This function marks the currently stored value as being read.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn get(&mut self) -> T
    where
        T: Clone,
    {
        let GenerationalValue { value, generation } = self.source.get().expect("deadlocked");
        self.read_generation = generation;
        value
    }

    /// Returns a clone of the currently contained value.
    ///
    /// This function marks the currently stored value as being read.
    ///
    /// `context` will be invalidated when the value is updated.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn get_tracking_redraw(&mut self, context: &WidgetContext<'_, '_>) -> T
    where
        T: Clone,
    {
        self.source.redraw_when_changed(context.handle());
        self.get()
    }

    /// Returns a clone of the currently contained value.
    ///
    /// This function marks the currently stored value as being read.
    ///
    /// `context` will be invalidated when the value is updated.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn get_tracking_invalidate(&mut self, context: &WidgetContext<'_, '_>) -> T
    where
        T: Clone,
    {
        self.source
            .invalidate_when_changed(context.handle(), context.widget().id());
        self.get()
    }

    /// Blocks the current thread until the contained value has been updated or
    /// there are no remaining writers for the value.
    ///
    /// Returns true if a newly updated value was discovered.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    pub fn block_until_updated(&mut self) -> bool {
        let mut deadlock_state = self.source.during_callback_state.lock().ignore_poison();
        assert!(
            deadlock_state
                .as_ref()
                .map_or(true, |state| state.locked_thread
                    != std::thread::current().id()),
            "deadlocked"
        );
        loop {
            let state = self.source.state.lock().ignore_poison();
            if state.wrapped.generation != self.read_generation {
                return true;
            } else if state.readers == Arc::strong_count(&self.source) {
                return false;
            }
            drop(state);

            // Wait for a notification of a change, which is synch
            deadlock_state = self.source.sync.wait(deadlock_state).ignore_poison();
        }
    }

    /// Returns true if this reader still has any writers connected to it.
    #[must_use]
    pub fn connected(&self) -> bool {
        self.source.state.lock().ignore_poison().readers < Arc::strong_count(&self.source)
    }

    /// Suspends the current async task until the contained value has been
    /// updated or there are no remaining writers for the value.
    ///
    /// Returns true if a newly updated value was discovered.
    pub fn wait_until_updated(&mut self) -> BlockUntilUpdatedFuture<'_, T> {
        BlockUntilUpdatedFuture(self)
    }

    /// Invokes `on_disconnect` when no instances of `Dynamic<T>` exist.
    ///
    /// This callback will be invoked even if this `DynamicReader` has been
    /// dropped.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    pub fn on_disconnect<OnDisconnect>(&self, on_disconnect: OnDisconnect)
    where
        OnDisconnect: FnOnce() + Send + 'static,
    {
        self.source
            .state()
            .expect("deadlocked")
            .on_disconnect
            .push(OnceCallback::new(|()| on_disconnect()));
    }
}

impl<T> context::sealed::Trackable for DynamicReader<T> {
    fn redraw_when_changed(&self, handle: WindowHandle) {
        self.source.redraw_when_changed(handle);
    }

    fn invalidate_when_changed(&self, handle: WindowHandle, id: WidgetId) {
        self.source.invalidate_when_changed(handle, id);
    }
}

impl<T> Debug for DynamicReader<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicReader")
            .field("source", &DebugDynamicData(&self.source))
            .field("read_generation", &self.read_generation.0)
            .finish()
    }
}

impl<T> Clone for DynamicReader<T> {
    fn clone(&self) -> Self {
        self.source.state().expect("deadlocked").readers += 1;
        Self {
            source: self.source.clone(),
            read_generation: self.read_generation,
        }
    }
}

impl<T> Drop for DynamicReader<T> {
    fn drop(&mut self) {
        let mut state = self.source.state().expect("deadlocked");
        state.readers -= 1;
    }
}

/// Suspends the current async task until the contained value has been
/// updated or there are no remaining writers for the value.
///
/// Yeilds true if a newly updated value was discovered.
#[derive(Debug)]
#[must_use = "futures must be .await'ed to be executed"]
pub struct BlockUntilUpdatedFuture<'a, T>(&'a mut DynamicReader<T>);

impl<'a, T> Future for BlockUntilUpdatedFuture<'a, T> {
    type Output = bool;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.source.state().expect("deadlocked");
        if state.wrapped.generation != self.0.read_generation {
            return Poll::Ready(true);
        } else if state.readers == Arc::strong_count(&self.0.source) {
            return Poll::Ready(false);
        }

        state.wakers.push(cx.waker().clone());
        Poll::Pending
    }
}

#[test]
fn disconnecting_reader_from_dynamic() {
    let value = Dynamic::new(1);
    let mut ref_reader = value.create_reader();
    drop(value);
    assert!(!ref_reader.block_until_updated());
}

#[test]
fn disconnecting_reader_threaded() {
    let a = Dynamic::new(1);
    let mut a_reader = a.create_reader();
    let b = Dynamic::new(1);
    let mut b_reader = b.create_reader();

    let thread = std::thread::spawn(move || {
        b.set(2);

        assert!(a_reader.block_until_updated());
        assert_eq!(a_reader.get(), 2);
        assert!(!a_reader.block_until_updated());
    });

    // Wait for the thread to set b to 2.
    assert!(b_reader.block_until_updated());
    assert_eq!(b_reader.get(), 2);

    // Set a to 2 and drop the handle.
    a.set(2);
    drop(a);

    thread.join().unwrap();
}

#[test]
fn disconnecting_reader_async() {
    let a = Dynamic::new(1);
    let mut a_reader = a.create_reader();
    let b = Dynamic::new(1);
    let mut b_reader = b.create_reader();

    let async_thread = std::thread::spawn(move || {
        pollster::block_on(async move {
            // Set b to 2, allowing the thread to execute its code.
            b.set(2);

            assert!(a_reader.wait_until_updated().await);
            assert_eq!(a_reader.get(), 2);
            assert!(!a_reader.wait_until_updated().await);
        });
    });

    // Wait for the pollster thread to set b to 2.
    assert!(b_reader.block_until_updated());
    assert_eq!(b_reader.get(), 2);

    // Set a to 2 and drop the handle.
    a.set(2);
    drop(a);

    async_thread.join().unwrap();
}

/// A tag that represents an individual revision of a [`Dynamic`] value.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct Generation(usize);

impl Generation {
    /// Returns the next tag.
    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

/// A type that can convert into a `Dynamic<T>`.
pub trait IntoDynamic<T> {
    /// Returns `self` as a dynamic.
    fn into_dynamic(self) -> Dynamic<T>;
}

impl<T> IntoDynamic<T> for Dynamic<T> {
    fn into_dynamic(self) -> Dynamic<T> {
        self
    }
}

impl<T, F> IntoDynamic<T> for F
where
    F: FnMut(&T) + Send + 'static,
    T: Default + Send + 'static,
{
    /// Returns [`Dynamic::default()`] with `self` installed as a for-each
    /// callback.
    fn into_dynamic(self) -> Dynamic<T> {
        Dynamic::default().with_for_each(self)
    }
}

/// A type that can be the source of a [`Switcher`] widget.
pub trait Switchable<T>: IntoDynamic<T> + Sized {
    /// Returns a new [`Switcher`] whose contents is the result of invoking
    /// `map` each time `self` is updated.
    fn switcher<F>(self, map: F) -> Switcher
    where
        F: FnMut(&T, &Dynamic<T>) -> WidgetInstance + Send + 'static,
        T: Send + 'static,
    {
        Switcher::mapping(self, map)
    }

    /// Returns a new [`Switcher`] whose contents switches between the values
    /// contained in `map` using the value in `self` as the key.
    fn switch_between<Collection>(self, map: Collection) -> Switcher
    where
        Collection: GetWidget<T> + Send + 'static,
        T: Send + 'static,
    {
        Switcher::mapping(self, move |key, _| {
            map.get(key)
                .map_or_else(|| Space::clear().make_widget(), Clone::clone)
        })
    }
}

/// A collection of widgets that can be queried by `Key`.
pub trait GetWidget<Key> {
    /// Returns the widget associated with `key`, if found.
    fn get<'a>(&'a self, key: &Key) -> Option<&'a WidgetInstance>;
}

impl<Key, State> GetWidget<Key> for HashMap<Key, WidgetInstance, State>
where
    Key: Hash + Eq,
    State: BuildHasher,
{
    fn get<'a>(&'a self, key: &Key) -> Option<&'a WidgetInstance> {
        HashMap::get(self, key)
    }
}

impl<Key> GetWidget<Key> for Map<Key, WidgetInstance>
where
    Key: Sort,
{
    fn get<'a>(&'a self, key: &Key) -> Option<&'a WidgetInstance> {
        Map::get(self, key)
    }
}

impl GetWidget<usize> for Children {
    fn get<'a>(&'a self, key: &usize) -> Option<&'a WidgetInstance> {
        (**self).get(*key)
    }
}

impl GetWidget<usize> for Vec<WidgetInstance> {
    fn get<'a>(&'a self, key: &usize) -> Option<&'a WidgetInstance> {
        (**self).get(*key)
    }
}

impl<T, W> Switchable<T> for W where W: IntoDynamic<T> {}

/// A value that may be either constant or dynamic.
pub enum Value<T> {
    /// A value that will not ever change externally.
    Constant(T),
    /// A value that may be updated externally.
    Dynamic(Dynamic<T>),
}

impl<T> Value<T> {
    /// Returns a [`Value::Dynamic`] containing `value`.
    pub fn dynamic(value: T) -> Self {
        Self::Dynamic(Dynamic::new(value))
    }

    /// Maps the current contents to `map` and returns the result.
    pub fn map<R>(&self, map: impl FnOnce(&T) -> R) -> R {
        match self {
            Value::Constant(value) => map(value),
            Value::Dynamic(dynamic) => dynamic.map_ref(map),
        }
    }

    /// Maps the current contents to `map` and returns the result.
    ///
    /// If `self` is a dynamic, `context` will be invalidated when the value is
    /// updated.
    pub fn map_tracking_redraw<R>(
        &self,
        context: &WidgetContext<'_, '_>,
        map: impl FnOnce(&T) -> R,
    ) -> R {
        match self {
            Value::Constant(value) => map(value),
            Value::Dynamic(dynamic) => {
                context.redraw_when_changed(dynamic);
                dynamic.map_ref(map)
            }
        }
    }

    /// Maps the current contents to `map` and returns the result.
    ///
    /// If `self` is a dynamic, `context` will be invalidated when the value is
    /// updated.
    pub fn map_tracking_invalidate<R>(
        &self,
        context: &WidgetContext<'_, '_>,
        map: impl FnOnce(&T) -> R,
    ) -> R {
        match self {
            Value::Constant(value) => map(value),
            Value::Dynamic(dynamic) => {
                context.invalidate_when_changed(dynamic);
                dynamic.map_ref(map)
            }
        }
    }

    /// Maps the current contents with exclusive access and returns the result.
    pub fn map_mut<R>(&mut self, map: impl FnOnce(&mut T) -> R) -> R {
        match self {
            Value::Constant(value) => map(value),
            Value::Dynamic(dynamic) => dynamic.map_mut(map),
        }
    }

    /// Returns a new value that is updated using `U::from(T.clone())` each time
    /// `self` is updated.
    #[must_use]
    pub fn map_each<R, F>(&self, mut map: F) -> Value<R>
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a T) -> R + Send + 'static,
        R: PartialEq + Send + 'static,
    {
        match self {
            Value::Constant(value) => Value::Constant(map(value)),
            Value::Dynamic(dynamic) => Value::Dynamic(dynamic.map_each(map)),
        }
    }

    /// Returns a clone of the currently stored value.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.map(Clone::clone)
    }

    /// Returns a clone of the currently stored value.
    ///
    /// If `self` is a dynamic, `context` will be refreshed when the value is
    /// updated.
    pub fn get_tracking_redraw(&self, context: &WidgetContext<'_, '_>) -> T
    where
        T: Clone,
    {
        self.map_tracking_redraw(context, Clone::clone)
    }

    /// Returns a clone of the currently stored value.
    ///
    /// If `self` is a dynamic, `context` will be invalidated when the value is
    /// updated.
    pub fn get_tracking_invalidate(&self, context: &WidgetContext<'_, '_>) -> T
    where
        T: Clone,
    {
        self.map_tracking_invalidate(context, Clone::clone)
    }

    /// Returns the current generation of the data stored, if the contained
    /// value is [`Dynamic`].
    pub fn generation(&self) -> Option<Generation> {
        match self {
            Value::Constant(_) => None,
            Value::Dynamic(value) => Some(value.generation()),
        }
    }

    /// Marks the widget for redraw when this value is updated.
    ///
    /// This function has no effect if the value is constant.
    pub fn redraw_when_changed(&self, context: &WidgetContext<'_, '_>) {
        if let Value::Dynamic(dynamic) = self {
            context.redraw_when_changed(dynamic);
        }
    }

    /// Marks the widget for redraw when this value is updated.
    ///
    /// This function has no effect if the value is constant.
    pub fn invalidate_when_changed(&self, context: &WidgetContext<'_, '_>) {
        if let Value::Dynamic(dynamic) = self {
            context.invalidate_when_changed(dynamic);
        }
    }
}

impl<T> IntoDynamic<T> for Value<T> {
    fn into_dynamic(self) -> Dynamic<T> {
        match self {
            Value::Constant(value) => Dynamic::new(value),
            Value::Dynamic(value) => value,
        }
    }
}

impl<T> Debug for Value<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(arg0) => Debug::fmt(arg0, f),
            Self::Dynamic(arg0) => Debug::fmt(arg0, f),
        }
    }
}

impl<T> Clone for Value<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Constant(arg0) => Self::Constant(arg0.clone()),
            Self::Dynamic(arg0) => Self::Dynamic(arg0.clone()),
        }
    }
}

impl<T> Default for Value<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::Constant(T::default())
    }
}

/// A type that can be converted into a [`Value`].
pub trait IntoValue<T> {
    /// Returns this type as a [`Value`].
    fn into_value(self) -> Value<T>;
}

impl<T> IntoValue<T> for T {
    fn into_value(self) -> Value<T> {
        Value::Constant(self)
    }
}

impl<'a> IntoValue<String> for &'a str {
    fn into_value(self) -> Value<String> {
        Value::Constant(self.to_owned())
    }
}

impl IntoValue<String> for Dynamic<&'static str> {
    fn into_value(self) -> Value<String> {
        self.map_each(ToString::to_string).into_value()
    }
}

impl<T> IntoValue<T> for Dynamic<T> {
    fn into_value(self) -> Value<T> {
        Value::Dynamic(self)
    }
}

impl<T> IntoValue<T> for &'_ Dynamic<T> {
    fn into_value(self) -> Value<T> {
        Value::Dynamic(self.clone())
    }
}

impl<T> IntoValue<T> for Value<T> {
    fn into_value(self) -> Value<T> {
        self
    }
}

impl<T> IntoValue<Option<T>> for T {
    fn into_value(self) -> Value<Option<T>> {
        Value::Constant(Some(self))
    }
}

/// A type that can have a `for_each` operation applied to it.
pub trait ForEach<T> {
    /// The borrowed representation of T to pass into the `for_each` function.
    type Ref<'a>;

    /// Apply `for_each` to each value contained within `self`.
    fn for_each<F>(&self, for_each: F) -> CallbackHandle
    where
        F: for<'a> FnMut(Self::Ref<'a>) + Send + 'static;
}

macro_rules! impl_tuple_for_each {
    ($($type:ident $field:tt $var:ident),+) => {
        impl<$($type,)+> ForEach<($($type,)+)> for ($(&Dynamic<$type>,)+)
        where
            $($type: Send + 'static,)+
        {
            type Ref<'a> = ($(&'a $type,)+);

            #[allow(unused_mut)]
            fn for_each<F>(&self, mut for_each: F) -> CallbackHandle
            where
                F: for<'a> FnMut(Self::Ref<'a>) + Send + 'static,
            {
                let mut handles = CallbackHandle::default();
                impl_tuple_for_each!(self for_each handles [] [$($type $field $var),+]);
                handles
            }
        }
    };
    ($self:ident $for_each:ident $handles:ident [] [$type:ident $field:tt $var:ident]) => {
        $handles += $self.$field.for_each(move |field: &$type| $for_each((field,)));
    };
    ($self:ident $for_each:ident $handles:ident [] [$($type:ident $field:tt $var:ident),+]) => {
        let $for_each = Arc::new(Mutex::new($for_each));
        $(let $var = $self.$field.clone();)*


        impl_tuple_for_each!(invoke $self $for_each $handles [] [$($type $field $var),+]);
    };
    (
        invoke
        // Identifiers used from the outer method
        $self:ident $for_each:ident $handles:ident
        // List of all tuple fields that have already been positioned as the focused call
        [$($ltype:ident $lfield:tt $lvar:ident),*]
        //
        [$type:ident $field:tt $var:ident, $($rtype:ident $rfield:tt $rvar:ident),+]
    ) => {
        impl_tuple_for_each!(
            invoke
            $self $for_each $handles
            $type $field $var
            [$($ltype $lfield $lvar,)* $type $field $var, $($rtype $rfield $rvar),+]
            [$($ltype $lfield $lvar,)* $($rtype $rfield $rvar),+]
        );
        impl_tuple_for_each!(
            invoke
            $self $for_each $handles
            [$($ltype $lfield $lvar,)* $type $field $var]
            [$($rtype $rfield $rvar),+]
        );
    };
    (
        invoke
        // Identifiers used from the outer method
        $self:ident $for_each:ident $handles:ident
        // List of all tuple fields that have already been positioned as the focused call
        [$($ltype:ident $lfield:tt $lvar:ident),+]
        //
        [$type:ident $field:tt $var:ident]
    ) => {
        impl_tuple_for_each!(
            invoke
            $self $for_each $handles
            $type $field $var
            [$($ltype $lfield $lvar,)+ $type $field $var]
            [$($ltype $lfield $lvar),+]
        );
    };
    (
        invoke
        // Identifiers used from the outer method
        $self:ident $for_each:ident $handles:ident
        // Tuple field that for_each is being invoked on
        $type:ident $field:tt $var:ident
        // The list of all tuple fields in this invocation, in the correct order.
        [$($atype:ident $afield:tt $avar:ident),+]
        // The list of tuple fields excluding the one being invoked.
        [$($rtype:ident $rfield:tt $rvar:ident),+]
    ) => {
        $handles += $var.for_each((&$for_each, $(&$rvar,)+).with_clone(|(for_each, $($rvar,)+)| {
            move |$var: &$type| {
                $(let $rvar = $rvar.lock();)+
                let mut for_each =
                    for_each.lock().ignore_poison();
                (for_each)(($(&$avar,)+));
            }
        }));
    };
}

impl_all_tuples!(impl_tuple_for_each);

/// A type that can create a `Dynamic<U>` from a `T` passed into a mapping
/// function.
pub trait MapEach<T, U> {
    /// The borrowed representation of `T` passed into the mapping function.
    type Ref<'a>;

    /// Apply `map_each` to each value in `self`, storing the result in the
    /// returned dynamic.
    fn map_each<F>(&self, map_each: F) -> Dynamic<U>
    where
        F: for<'a> FnMut(Self::Ref<'a>) -> U + Send + 'static;
}

macro_rules! impl_tuple_map_each {
    ($($type:ident $field:tt $var:ident),+) => {
        impl<U, $($type),+> MapEach<($($type,)+), U> for ($(&Dynamic<$type>,)+)
        where
            U: PartialEq + Send + 'static,
            $($type: Send + 'static),+
        {
            type Ref<'a> = ($(&'a $type,)+);

            fn map_each<F>(&self, mut map_each: F) -> Dynamic<U>
            where
                F: for<'a> FnMut(Self::Ref<'a>) -> U + Send + 'static,
            {
                let dynamic = {
                    $(let $var = self.$field.lock();)+

                    Dynamic::new(map_each(($(&$var,)+)))
                };
                dynamic.set_source(self.for_each({
                    let dynamic = dynamic.clone();

                    move |tuple| {
                        dynamic.set(map_each(tuple));
                    }
                }));
                dynamic
            }
        }
    };
}

impl_all_tuples!(impl_tuple_map_each);

/// A type that can have a `for_each` operation applied to it.
pub trait ForEachCloned<T> {
    /// Apply `for_each` to each value contained within `self`.
    fn for_each_cloned<F>(&self, for_each: F) -> CallbackHandle
    where
        F: for<'a> FnMut(T) + Send + 'static;
}

macro_rules! impl_tuple_for_each_cloned {
    ($($type:ident $field:tt $var:ident),+) => {
        impl<$($type,)+> ForEachCloned<($($type,)+)> for ($(&Dynamic<$type>,)+)
        where
            $($type: Clone + Send + 'static,)+
        {

            #[allow(unused_mut)]
            fn for_each_cloned<F>(&self, mut for_each: F) -> CallbackHandle
            where
                F: for<'a> FnMut(($($type,)+)) + Send + 'static,
            {
                let mut handles = CallbackHandle::default();
                impl_tuple_for_each_cloned!(self for_each handles [] [$($type $field $var),+]);
                handles
            }
        }
    };
    ($self:ident $for_each:ident $handles:ident [] [$type:ident $field:tt $var:ident]) => {
        $handles += $self.$field.for_each(move |field: &$type| $for_each((field.clone(),)));
    };
    ($self:ident $for_each:ident $handles:ident [] [$($type:ident $field:tt $var:ident),+]) => {
        let $for_each = Arc::new(Mutex::new($for_each));
        $(let $var = $self.$field.clone();)*


        impl_tuple_for_each_cloned!(invoke $self $for_each $handles [] [$($type $field $var),+]);
    };
    (
        invoke
        // Identifiers used from the outer method
        $self:ident $for_each:ident $handles:ident
        // List of all tuple fields that have already been positioned as the focused call
        [$($ltype:ident $lfield:tt $lvar:ident),*]
        //
        [$type:ident $field:tt $var:ident, $($rtype:ident $rfield:tt $rvar:ident),+]
    ) => {
        impl_tuple_for_each_cloned!(
            invoke
            $self $for_each $handles
            $type $field $var
            [$($ltype $lfield $lvar,)* $type $field $var, $($rtype $rfield $rvar),+]
            [$($ltype $lfield $lvar,)* $($rtype $rfield $rvar),+]
        );
        impl_tuple_for_each_cloned!(
            invoke
            $self $for_each $handles
            [$($ltype $lfield $lvar,)* $type $field $var]
            [$($rtype $rfield $rvar),+]
        );
    };
    (
        invoke
        // Identifiers used from the outer method
        $self:ident $for_each:ident $handles:ident
        // List of all tuple fields that have already been positioned as the focused call
        [$($ltype:ident $lfield:tt $lvar:ident),+]
        //
        [$type:ident $field:tt $var:ident]
    ) => {
        impl_tuple_for_each_cloned!(
            invoke
            $self $for_each $handles
            $type $field $var
            [$($ltype $lfield $lvar,)+ $type $field $var]
            [$($ltype $lfield $lvar),+]
        );
    };
    (
        invoke
        // Identifiers used from the outer method
        $self:ident $for_each:ident $handles:ident
        // Tuple field that for_each is being invoked on
        $type:ident $field:tt $var:ident
        // The list of all tuple fields in this invocation, in the correct order.
        [$($atype:ident $afield:tt $avar:ident),+]
        // The list of tuple fields excluding the one being invoked.
        [$($rtype:ident $rfield:tt $rvar:ident),+]
    ) => {
        $handles += $var.for_each_cloned((&$for_each, $(&$rvar,)+).with_clone(|(for_each, $($rvar,)+)| {
            move |$var: $type| {
                $(let $rvar = $rvar.get();)+
                if let Ok(mut for_each) =
                    for_each.try_lock() {
                (for_each)(($($avar,)+));
                    }
            }
        }));
    };
}

impl_all_tuples!(impl_tuple_for_each_cloned);

/// A type that can create a `Dynamic<U>` from a `T` passed into a mapping
/// function.
pub trait MapEachCloned<T, U> {
    /// Apply `map_each` to each value in `self`, storing the result in the
    /// returned dynamic.
    fn map_each_cloned<F>(&self, map_each: F) -> Dynamic<U>
    where
        F: for<'a> FnMut(T) -> U + Send + 'static;
}

macro_rules! impl_tuple_map_each_cloned {
    ($($type:ident $field:tt $var:ident),+) => {
        impl<U, $($type),+> MapEachCloned<($($type,)+), U> for ($(&Dynamic<$type>,)+)
        where
            U: PartialEq + Send + 'static,
            $($type: Clone + Send + 'static),+
        {

            fn map_each_cloned<F>(&self, mut map_each: F) -> Dynamic<U>
            where
                F: for<'a> FnMut(($($type,)+)) -> U + Send + 'static,
            {
                let dynamic = {
                    $(let $var = self.$field.get();)+

                    Dynamic::new(map_each(($($var,)+)))
                };
                dynamic.set_source(self.for_each_cloned({
                    let dynamic = dynamic.clone();

                    move |tuple| {
                        dynamic.set(map_each(tuple));
                    }
                }));
                dynamic
            }
        }
    };
}

impl_all_tuples!(impl_tuple_map_each_cloned);

/// The status of validating data.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum Validation {
    /// No validation has been performed yet.
    ///
    /// This status represents that the data is still in its initial state, so
    /// errors should be delayed until it is changed.
    #[default]
    None,
    /// The data is valid.
    Valid,
    /// The data is invalid. The string contains a human-readable message.
    Invalid(String),
}

impl Validation {
    /// Returns the effective text to display along side the field.
    ///
    /// When there is a validation error, it is returned, otherwise the hint is
    /// returned.
    #[must_use]
    pub fn message<'a>(&'a self, hint: &'a str) -> &'a str {
        match self {
            Validation::None | Validation::Valid => hint,
            Validation::Invalid(err) => err,
        }
    }

    /// Returns true if there is a validation error.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Invalid(_))
    }

    /// Returns the result of merging both validations.
    #[must_use]
    pub fn and(&self, other: &Self) -> Self {
        match (self, other) {
            (Validation::Valid, Validation::Valid) => Validation::Valid,
            (Validation::Invalid(error), _) | (_, Validation::Invalid(error)) => {
                Validation::Invalid(error.clone())
            }
            (Validation::None, _) | (_, Validation::None) => Validation::None,
        }
    }
}

impl<T, E> IntoDynamic<Validation> for Dynamic<Result<T, E>>
where
    T: Send + 'static,
    E: Display + Send + 'static,
{
    fn into_dynamic(self) -> Dynamic<Validation> {
        self.map_each(|result| match result {
            Ok(_) => Validation::Valid,
            Err(err) => Validation::Invalid(err.to_string()),
        })
    }
}

/// A grouping of validations that can be checked simultaneously.
#[derive(Debug, Default, Clone)]
pub struct Validations {
    state: Dynamic<ValidationsState>,
    invalid: Dynamic<usize>,
}

#[derive(Default, Debug, Eq, PartialEq, Clone)]
enum ValidationsState {
    #[default]
    Initial,
    Resetting,
    Checked,
    Disabled,
}

impl Validations {
    /// Validates `dynamic`'s contents using `check`, returning a dynamic
    /// containing the validation status.
    ///
    /// The validation is linked with `self` such that checking `self`'s
    /// validation status will include this validation.
    #[must_use]
    pub fn validate<T, E, Valid>(
        &self,
        dynamic: &Dynamic<T>,
        mut check: Valid,
    ) -> Dynamic<Validation>
    where
        T: Send + 'static,
        Valid: for<'a> FnMut(&'a T) -> Result<(), E> + Send + 'static,
        E: Display,
    {
        let validation = Dynamic::new(Validation::None);
        let mut message_mapping = Self::map_to_message(move |value| check(value));
        let error_message = dynamic.map_each_generational(move |value| message_mapping(value));

        validation.set_source((&self.state, &error_message).for_each_cloned({
            let mut f = self.generate_validation(dynamic);
            let validation = validation.clone();

            move |(current_state, message)| {
                validation.set(f(current_state, message));
            }
        }));

        validation
    }

    /// Returns a dynamic validation status that is created by transforming the
    /// `Err` variant of `result` using [`Display`].
    ///
    /// The validation is linked with `self` such that checking `self`'s
    /// validation status will include this validation.
    #[must_use]
    pub fn validate_result<T, E>(
        &self,
        result: impl IntoDynamic<Result<T, E>>,
    ) -> Dynamic<Validation>
    where
        T: Send + 'static,
        E: Display + Send + 'static,
    {
        let result = result.into_dynamic();
        let error_message = result.map_each(move |value| match value {
            Ok(_) => None,
            Err(err) => Some(err.to_string()),
        });

        self.validate(&error_message, |error_message| match error_message {
            None => Ok(()),
            Some(message) => Err(message.clone()),
        })
    }

    fn map_to_message<T, E, Valid>(
        mut check: Valid,
    ) -> impl for<'a> FnMut(&'a GenerationalValue<T>) -> GenerationalValue<Option<String>> + Send + 'static
    where
        T: Send + 'static,
        Valid: for<'a> FnMut(&'a T) -> Result<(), E> + Send + 'static,
        E: Display,
    {
        move |value| {
            value.map_ref(|value| match check(value) {
                Ok(()) => None,
                Err(err) => Some(err.to_string()),
            })
        }
    }

    fn generate_validation<T>(
        &self,
        dynamic: &Dynamic<T>,
    ) -> impl FnMut(ValidationsState, GenerationalValue<Option<String>>) -> Validation
    where
        T: Send + 'static,
    {
        self.invalid.map_mut(|invalid| *invalid += 1);

        let invalid_count = self.invalid.clone();
        let dynamic = dynamic.clone();
        let mut initial_generation = dynamic.generation();
        let mut invalid = true;

        move |current_state, generational| {
            let new_invalid = match (&current_state, &generational.value) {
                (ValidationsState::Disabled, _) | (_, None) => false,
                (_, Some(_)) => true,
            };
            if invalid != new_invalid {
                if new_invalid {
                    invalid_count.map_mut(|invalid| *invalid += 1);
                } else {
                    invalid_count.map_mut(|invalid| *invalid -= 1);
                }
                invalid = new_invalid;
            }
            let new_status = if let Some(err) = generational.value {
                Validation::Invalid(err.to_string())
            } else {
                Validation::Valid
            };
            match current_state {
                ValidationsState::Resetting => {
                    initial_generation = dynamic.generation();
                    Validation::None
                }
                ValidationsState::Initial if initial_generation == dynamic.generation() => {
                    Validation::None
                }
                _ => new_status,
            }
        }
    }

    /// Returns a builder that can be used to create validations that only run
    /// when `condition` is true.
    pub fn when(&self, condition: impl IntoDynamic<bool>) -> WhenValidation<'_> {
        WhenValidation {
            validations: self,
            condition: condition.into_dynamic(),
            not: false,
        }
    }

    /// Returns a builder that can be used to create validations that only run
    /// when `condition` is false.
    pub fn when_not(&self, condition: impl IntoDynamic<bool>) -> WhenValidation<'_> {
        WhenValidation {
            validations: self,
            condition: condition.into_dynamic(),
            not: true,
        }
    }

    /// Returns true if this set of validations are all valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.invoke_callback((), &mut |()| true)
    }

    fn invoke_callback<T, R, F>(&self, t: T, handler: &mut F) -> R
    where
        F: FnMut(T) -> R + Send + 'static,
        R: Default,
    {
        let _result = self
            .state
            .compare_swap(&ValidationsState::Initial, ValidationsState::Checked);
        if self.invalid.get() == 0 {
            handler(t)
        } else {
            R::default()
        }
    }

    /// Returns a function that invokes `handler` only when all tracked
    /// validations are valid.
    ///
    /// The returned function can be use in a
    /// [`Callback`](crate::widget::Callback).
    ///
    /// When the contents are invalid, `R::default()` is returned.
    pub fn when_valid<T, R, F>(self, mut handler: F) -> impl FnMut(T) -> R + Send + 'static
    where
        F: FnMut(T) -> R + Send + 'static,
        R: Default,
    {
        move |t: T| self.invoke_callback(t, &mut handler)
    }

    /// Resets the validation status for all related validations.
    pub fn reset(&self) {
        self.state.set(ValidationsState::Resetting);
        self.state.set(ValidationsState::Initial);
    }
}

/// A builder for validations that only run when a precondition is met.
pub struct WhenValidation<'a> {
    validations: &'a Validations,
    condition: Dynamic<bool>,
    not: bool,
}

impl WhenValidation<'_> {
    /// Validates `dynamic`'s contents using `check`, returning a dynamic
    /// containing the validation status.
    ///
    /// The validation is linked with `self` such that checking `self`'s
    /// validation status will include this validation.
    ///
    /// Each change to `dynamic` is validated, but the result of the validation
    /// will be ignored if the required prerequisite isn't met.
    #[must_use]
    pub fn validate<T, E, Valid>(
        &self,
        dynamic: &Dynamic<T>,
        mut check: Valid,
    ) -> Dynamic<Validation>
    where
        T: Send + 'static,
        Valid: for<'a> FnMut(&'a T) -> Result<(), E> + Send + 'static,
        E: Display,
    {
        let validation = Dynamic::new(Validation::None);
        let mut map_to_message = Validations::map_to_message(move |value| check(value));
        let error_message =
            dynamic.map_each_generational(move |generational| map_to_message(generational));
        let mut f = self.validations.generate_validation(dynamic);
        let not = self.not;

        (&self.condition, &self.validations.state, &error_message).map_each_cloned({
            let validation = validation.clone();
            move |(condition, state, message)| {
                let enabled = if not { !condition } else { condition };
                let state = if enabled {
                    state
                } else {
                    ValidationsState::Disabled
                };
                let result = f(state, message);
                if enabled {
                    validation.set(result);
                } else {
                    validation.set(Validation::None);
                }
            }
        });

        validation
    }

    /// Returns a dynamic validation status that is created by transforming the
    /// `Err` variant of `result` using [`Display`].
    ///
    /// The validation is linked with `self` such that checking `self`'s
    /// validation status will include this validation.
    #[must_use]
    pub fn validate_result<T, E>(
        &self,
        result: impl IntoDynamic<Result<T, E>>,
    ) -> Dynamic<Validation>
    where
        T: Send + 'static,
        E: Display + Send + 'static,
    {
        let result = result.into_dynamic();
        let error_message = result.map_each(move |value| match value {
            Ok(_) => None,
            Err(err) => Some(err.to_string()),
        });

        self.validate(&error_message, |error_message| match error_message {
            None => Ok(()),
            Some(message) => Err(message.clone()),
        })
    }
}

struct Debounce<T> {
    destination: Dynamic<T>,
    period: Duration,
    delay: Option<AnimationHandle>,
    buffer: Dynamic<T>,
    extend: bool,
    _callback: Option<CallbackHandle>,
}

impl<T> Debounce<T>
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    pub fn new(destination: Dynamic<T>, period: Duration) -> Self {
        Self {
            buffer: Dynamic::new(destination.get()),
            destination,
            period,
            delay: None,
            extend: false,
            _callback: None,
        }
    }

    pub fn extending(mut self) -> Self {
        self.extend = true;
        self
    }

    pub fn update(&mut self, value: T) {
        if self.buffer.replace(value).is_some() {
            let create_delay = if self.extend {
                true
            } else {
                self.delay
                    .as_ref()
                    .map_or(true, AnimationHandle::is_complete)
            };

            if create_delay {
                let destination = self.destination.clone();
                let buffer = self.buffer.clone();
                self.delay = Some(
                    self.period
                        .on_complete(move || {
                            destination.set(buffer.get());
                        })
                        .spawn(),
                );
            }
        }
    }
}

#[test]
fn map_cycle_is_finite() {
    crate::initialize_tracing();
    let a = Dynamic::new(0_usize);

    // This callback updates a each time a is updated with a + 1, causing an
    // infinite cycle if not broken by Cushy.
    a.for_each_cloned({
        let a = a.clone();
        move |current| {
            a.set(current + 1);
        }
    })
    .persist();

    // Cushy will invoke the callback for the first set call, but the set call
    // within the callback will not cause the callback to be invoked again.
    // Thus, we expect setting the value to 1 to result in `a` containing 2.
    a.set(1);
    assert_eq!(a.get(), 2);
}

#[test]
fn compare_swap() {
    let dynamic = Dynamic::new(1);
    assert_eq!(dynamic.compare_swap(&1, 2), Ok(1));
    assert_eq!(dynamic.compare_swap(&1, 0), Err(2));
    assert_eq!(dynamic.compare_swap(&2, 0), Ok(2));
    assert_eq!(dynamic.get(), 0);
}
