//! Types for storing and interacting with values in Widgets.

use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::{self, Debug, Display};
use std::future::Future;
use std::hash::{BuildHasher, Hash};
use std::ops::{Add, AddAssign, Deref, DerefMut, Not};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::task::{Poll, Waker};
use std::thread::ThreadId;
use std::time::Duration;

use ahash::{AHashMap, AHashSet};
use alot::{LotId, Lots};
use intentional::Assert;
use kempt::{Map, Sort};
use parking_lot::{Condvar, Mutex, MutexGuard};

use crate::animation::{AnimationHandle, DynamicTransition, IntoAnimate, LinearInterpolate, Spawn};
use crate::context::{self, Trackable, WidgetContext};
use crate::reactive::{
    defer_execute_callbacks, CallbackCollection, ChangeCallbacks, ChangeCallbacksData,
};
use crate::utils::WithClone;
use crate::widget::{
    MakeWidget, MakeWidgetWithTag, Notify, OnceCallback, WidgetId, WidgetInstance, WidgetList,
};
use crate::widgets::checkbox::CheckboxState;
use crate::widgets::{Checkbox, Radio, Select, Space, Switcher};
use crate::window::WindowHandle;

/// A source of one or more `T` values.
pub trait Source<T> {
    /// Maps the contents with read-only access, providing access to the value's
    /// [`Generation`].
    fn try_map_generational<R>(
        &self,
        map: impl FnOnce(DynamicGuard<'_, T, true>) -> R,
    ) -> Result<R, DeadlockError>;

    /// Maps the contents with read-only access, providing access to the value's
    /// [`Generation`].
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    fn map_generational<R>(&self, map: impl FnOnce(DynamicGuard<'_, T, true>) -> R) -> R {
        self.try_map_generational(map).expect("deadlocked")
    }

    /// Returns the current generation of the value.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    fn generation(&self) -> Generation {
        self.map_generational(|g| g.generation())
    }

    /// Maps the contents with read-only access.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    fn map_ref<R>(&self, map: impl FnOnce(&T) -> R) -> R {
        self.map_generational(|gen| map(&*gen))
    }

    /// Returns a clone of the currently contained value.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    fn get(&self) -> T
    where
        T: Clone,
    {
        self.map_ref(T::clone)
    }

    /// Maps the contents with read-only access.
    fn try_map_ref<R>(&self, map: impl FnOnce(&T) -> R) -> Result<R, DeadlockError> {
        self.try_map_generational(|gen| map(&*gen))
    }

    /// Returns a clone of the currently contained value.
    fn try_get(&self) -> Result<T, DeadlockError>
    where
        T: Clone,
    {
        self.try_map_generational(|gen| gen.clone())
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
    fn get_tracking_redraw(&self, context: &WidgetContext<'_>) -> T
    where
        T: Clone,
        Self: Trackable + Sized,
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
    fn get_tracking_invalidate(&self, context: &WidgetContext<'_>) -> T
    where
        T: Clone,
        Self: Trackable + Sized,
    {
        context.invalidate_when_changed(self);
        self.get()
    }

    /// Executes `on_change` when the contents of this dynamic are updated.
    ///
    /// Returning `Err(CallbackDisconnected)` will prevent the callback from
    /// being invoked again.
    fn on_change_try<F>(&self, on_change: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: FnMut() -> Result<(), CallbackDisconnected> + Send + 'static;

    /// Executes `on_change` when the contents of this dynamic are updated.
    fn on_change<F>(&self, mut on_change: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: FnMut() + Send + 'static,
    {
        self.on_change_try(move || {
            on_change();
            Ok(())
        })
    }

    /// Attaches `for_each` to this value so that it is invoked each time the
    /// source's contents are updated.
    ///
    /// `for_each` will not be invoked with the currently stored value.
    ///
    /// Returning `Err(CallbackDisconnected)` will prevent the callback from
    /// being invoked again.
    fn for_each_subsequent_generational_try<F>(&self, for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(DynamicGuard<'_, T, true>) -> Result<(), CallbackDisconnected>
            + Send
            + 'static;

    /// Attaches `for_each` to this value so that it is invoked each time the
    /// source's contents are updated.
    ///
    /// `for_each` will not be invoked with the currently stored value.
    fn for_each_subsequent_generational<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(DynamicGuard<'_, T, true>) + Send + 'static,
    {
        self.for_each_subsequent_generational_try(move |value| {
            for_each(value);
            Ok(())
        })
    }

    /// Attaches `for_each` to this value so that it is invoked each time the
    /// source's contents are updated.
    ///
    /// `for_each` will not be invoked with the currently stored value.
    ///
    /// Returning `Err(CallbackDisconnected)` will prevent the callback from
    /// being invoked again.
    fn for_each_subsequent_try<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.for_each_subsequent_generational_try(move |gen| for_each(&*gen))
    }

    /// Attaches `for_each` to this value so that it is invoked each time the
    /// source's contents are updated.
    ///
    /// `for_each` will not be invoked with the currently stored value.
    fn for_each_subsequent<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a T) + Send + 'static,
    {
        self.for_each_subsequent_try(move |value| {
            for_each(value);
            Ok(())
        })
    }

    /// Invokes `for_each` with the current contents and each time this source's
    /// contents are updated.
    ///
    /// Returning `Err(CallbackDisconnected)` will prevent the callback from
    /// being invoked again.
    fn for_each_generational_try<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(DynamicGuard<'_, T, true>) -> Result<(), CallbackDisconnected>
            + Send
            + 'static,
    {
        self.map_generational(&mut for_each)
            .expect("initial for_each invocation failed");
        self.for_each_subsequent_generational_try(for_each)
    }

    /// Invokes `for_each` with the current contents and each time this source's
    /// contents are updated.
    fn for_each_generational<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(DynamicGuard<'_, T, true>) + Send + 'static,
    {
        self.for_each_generational_try(move |value| {
            for_each(value);
            Ok(())
        })
    }

    /// Invokes `for_each` with the current contents and each time this source's
    /// contents are updated.
    ///
    /// Returning `Err(CallbackDisconnected)` will prevent the callback from
    /// being invoked again.
    fn for_each_try<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.for_each_generational_try(move |gen| for_each(&*gen))
    }

    /// Invokes `for_each` with the current contents and each time this source's
    /// contents are updated.
    fn for_each<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a T) + Send + 'static,
    {
        self.for_each_try(move |value| {
            for_each(value);
            Ok(())
        })
    }

    /// Invokes `for_each` with the current contents and each time this source's
    /// contents are updated.
    ///
    /// Returning `Err(CallbackDisconnected)` will prevent the callback from
    /// being invoked again.
    fn for_each_generational_cloned_try<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Clone + Send + 'static,
        F: FnMut(GenerationalValue<T>) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.map_generational(|value| for_each(GenerationalValue::from(&value)))
            .expect("initial for_each invocation failed");
        self.for_each_subsequent_generational_cloned_try(for_each)
    }

    /// Invokes `for_each` with the current contents and each time this source's
    /// contents are updated.
    ///
    /// Returning `Err(CallbackDisconnected)` will prevent the callback from
    /// being invoked again.
    fn for_each_subsequent_generational_cloned_try<F>(&self, for_each: F) -> CallbackHandle
    where
        T: Clone + Send + 'static,
        F: FnMut(GenerationalValue<T>) -> Result<(), CallbackDisconnected> + Send + 'static;

    /// Invokes `for_each` with the current contents and each time this source's
    /// contents are updated.
    ///
    /// Returning `Err(CallbackDisconnected)` will prevent the callback from
    /// being invoked again.
    fn for_each_cloned_try<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Clone + Send + 'static,
        F: FnMut(T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.for_each_generational_cloned_try(move |gen| for_each(gen.value))
    }

    /// Invokes `for_each` each time this source's contents are updated.
    ///
    /// Returning `Err(CallbackDisconnected)` will prevent the callback from
    /// being invoked again.
    fn for_each_subsequent_cloned_try<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Clone + Send + 'static,
        F: FnMut(T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.for_each_subsequent_generational_cloned_try(move |gen| for_each(gen.value))
    }

    /// Invokes `for_each` each time this source's contents are updated.
    fn for_each_subsequent_cloned<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Clone + Send + 'static,
        F: FnMut(T) + Send + 'static,
    {
        self.for_each_subsequent_cloned_try(move |value| {
            for_each(value);
            Ok(())
        })
    }

    /// Invokes `for_each` with the current contents and each time this source's
    /// contents are updated.
    fn for_each_cloned<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Clone + Send + 'static,
        F: FnMut(T) + Send + 'static,
    {
        self.for_each_cloned_try(move |value| {
            for_each(value);
            Ok(())
        })
    }

    /// Notifies `notify` with a clone of the  current contents each time this
    /// source's contents are updated.
    fn for_each_notify(&self, notify: impl Into<Notify<T>>) -> CallbackHandle
    where
        T: Unpin + Clone + Send + 'static,
    {
        let mut notify = notify.into();
        self.for_each_cloned(move |value| notify.notify(value))
    }

    /// Notifies `notify` with a clone of the  current contents each time this
    /// source's contents are updated, disconnecting the callback if the target
    /// is disconnected.
    fn for_each_try_notify(&self, notify: impl Into<Notify<T>>) -> CallbackHandle
    where
        T: Unpin + Clone + Send + 'static,
    {
        let mut notify = notify.into();
        self.for_each_cloned_try(move |value| {
            notify.try_notify(value).map_err(|_| CallbackDisconnected)
        })
    }

    /// Returns a new dynamic that contains the updated contents of this dynamic
    /// at most once every `period`.
    #[must_use]
    fn debounced_every(&self, period: Duration) -> Dynamic<T>
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
    fn debounced_with_delay(&self, period: Duration) -> Dynamic<T>
    where
        T: PartialEq + Clone + Send + Sync + 'static,
    {
        let debounced = Dynamic::new(self.get());
        let mut debounce = Debounce::new(debounced.clone(), period).extending();
        let callback = self.for_each_cloned(move |value| debounce.update(value));
        debounced.set_source(callback);
        debounced
    }

    /// Creates a new dynamic value that contains the result of invoking `map`
    /// each time this value is changed.
    fn map_each_generational<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        T: Send + 'static,
        F: for<'a> FnMut(DynamicGuard<'a, T, true>) -> R + Send + 'static,
        R: PartialEq + Send + 'static,
    {
        let mapped = Dynamic::new(self.map_generational(&mut map));
        let mapped_weak = mapped.downgrade();
        mapped.set_source(self.for_each_generational_try(move |value| {
            let mapped = mapped_weak.upgrade().ok_or(CallbackDisconnected)?;
            mapped.set(map(value));
            Ok(())
        }));
        mapped
    }

    /// Creates a new dynamic value that contains the result of invoking `map`
    /// each time this value is changed.
    fn map_each<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a T) -> R + Send + 'static,
        R: PartialEq + Send + 'static,
    {
        self.map_each_generational(move |gen| map(&*gen))
    }

    /// Creates a new dynamic value that contains the result of invoking `map`
    /// each time this value is changed.
    fn map_each_cloned<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        T: Clone + Send + 'static,
        F: FnMut(T) -> R + Send + 'static,
        R: PartialEq + Send + 'static,
    {
        let mapped = Dynamic::new(map(self.get()));
        let mapped_weak = mapped.downgrade();
        mapped.set_source(self.for_each_cloned_try(move |value| {
            let mapped = mapped_weak.upgrade().ok_or(CallbackDisconnected)?;
            mapped.set(map(value));
            Ok(())
        }));
        mapped
    }

    /// Returns a new [`Dynamic`] that contains a clone of each value from
    /// `self`.
    ///
    /// The returned dynamic does not hold a strong reference to `self`,
    /// ensuring that `self` can be cleaned up even if the returned dynamic
    /// still exists.
    fn weak_clone(&self) -> Dynamic<T>
    where
        T: Clone + Send + 'static,
    {
        let mapped = Dynamic::new(self.get());
        let mapped_weak = mapped.downgrade();

        mapped.set_source(
            self.for_each_cloned_try(move |value| {
                let mapped = mapped_weak.upgrade().ok_or(CallbackDisconnected)?;
                *mapped.lock() = value;
                Ok(())
            })
            .weak(),
        );
        mapped
    }

    /// Returns a new dynamic that is updated using `U::from(T.clone())` each
    /// time `self` is updated.
    #[must_use]
    fn map_each_into<U>(&self) -> Dynamic<U>
    where
        U: PartialEq + From<T> + Send + 'static,
        T: Clone + Send + 'static,
    {
        self.map_each(|value| U::from(value.clone()))
    }

    /// Returns a new dynamic that is updated using `U::from(&T)` each
    /// time `self` is updated.
    #[must_use]
    fn map_each_to<U>(&self) -> Dynamic<U>
    where
        U: PartialEq + for<'a> From<&'a T> + Send + 'static,
        T: Clone + Send + 'static,
    {
        self.map_each(|value| U::from(value))
    }
}

/// A destination for values of type `T`.
pub trait Destination<T> {
    /// Maps the contents with exclusive access. Before returning from this
    /// function, all observers will be notified that the contents have been
    /// updated.
    fn try_map_mut<R>(&self, map: impl FnOnce(Mutable<'_, T>) -> R) -> Result<R, DeadlockError>;

    /// Maps the contents with exclusive access. Before returning from this
    /// function, all observers will be notified that the contents have been
    /// updated.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    fn map_mut<R>(&self, map: impl FnOnce(Mutable<'_, T>) -> R) -> R {
        self.try_map_mut(map).expect("deadlocked")
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
    ///     currently stored value.
    /// - [`ReplaceError::Deadlock`]: Returned when the current thread already
    ///     has exclusive access to the contents of this dynamic.
    fn try_replace(&self, new_value: T) -> Result<T, ReplaceError<T>>
    where
        T: PartialEq,
    {
        match self.try_map_mut(|mut value| {
            if *value == new_value {
                Err(ReplaceError::NoChange(new_value))
            } else {
                Ok(std::mem::replace(&mut *value, new_value))
            }
        }) {
            Ok(old) => old,
            Err(DeadlockError) => Err(ReplaceError::Deadlock),
        }
    }

    /// Replaces the contents with `new_value`, returning the previous contents.
    /// Before returning from this function, all observers will be notified that
    /// the contents have been updated.
    ///
    /// If the calling thread has exclusive access to the contents of this
    /// dynamic, this call will return None and the value will not be updated.
    /// If detecting this is important, use [`Self::try_replace()`].
    ///
    /// # Replacing a new value without `PartialEq`
    ///
    /// This function requires that the contained type implements `PartialEq`.
    /// One common problem with reactive data graphs is that they can be very
    /// "noisy". Cushy attempts to minimize noise by only invoking callbacks
    /// when the value has changed, and it detects this by using `PartialEq`.
    ///
    /// However, not all types implement `PartialEq`.
    /// [`map_mut()`](Self::map_mut) does not require `PartialEq`, and can be
    /// used along with [`std::mem::replace()`] to perform the same operation
    /// without checking for equality.
    fn replace(&self, new_value: T) -> Option<T>
    where
        T: PartialEq,
    {
        self.try_replace(new_value).ok()
    }

    /// Stores `new_value` in this dynamic. Before returning from this function,
    /// all observers will be notified that the contents have been updated.
    ///
    /// # Setting a new value without `PartialEq`
    ///
    /// This function requires that the contained type implements `PartialEq`.
    /// One common problem with reactive data graphs is that they can be very
    /// "noisy". Cushy attempts to minimize noise by only invoking callbacks
    /// when the value has changed, and it detects this by using `PartialEq`.
    ///
    /// However, not all types implement `PartialEq`. See [`force_set()`](Self::force_set).
    fn set(&self, new_value: T)
    where
        T: PartialEq,
    {
        let _old = self.replace(new_value);
    }

    /// Stores `new_value` in this dynamic without checking for equality.
    ///
    /// Before returning from this function, all observers will be notified
    /// that the contents have been updated.
    fn force_set(&self, new_value: T) {
        self.map_mut(|mut old_value|{
            let _old_value = std::mem::replace(&mut *old_value, new_value);
        });
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
    fn try_compare_swap(
        &self,
        expected_current: &T,
        new_value: T,
    ) -> Result<T, TryCompareSwapError<T>>
    where
        T: Clone + PartialEq,
    {
        match self.try_map_mut(|mut value| {
            if &*value == expected_current {
                Ok(std::mem::replace(&mut *value, new_value))
            } else {
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
    fn compare_swap(&self, expected_current: &T, new_value: T) -> Result<T, T>
    where
        T: Clone + PartialEq,
    {
        match self.try_compare_swap(expected_current, new_value) {
            Ok(old) => Ok(old),
            Err(TryCompareSwapError::Deadlock) => unreachable!("deadlocked"),
            Err(TryCompareSwapError::CurrentValueMismatch(value)) => Err(value),
        }
    }

    /// Updates the value to the result of invoking [`Not`] on the current
    /// value. This function returns the new value.
    #[allow(clippy::must_use_candidate)]
    fn toggle(&self) -> T
    where
        T: Not<Output = T> + Clone,
    {
        self.map_mut(|mut value| {
            *value = !value.clone();
            value.clone()
        })
    }

    /// Returns the currently stored value, replacing the current contents with
    /// `T::default()`.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    fn take(&self) -> T
    where
        Self: Source<T>,
        T: Default,
    {
        self.map_mut(|mut value| std::mem::take(&mut *value))
    }

    /// Checks if the currently stored value is different than `T::default()`,
    /// and if so, returns `Some(self.take())`.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    fn take_if_not_default(&self) -> Option<T>
    where
        T: Default + PartialEq,
    {
        let default = T::default();
        self.map_mut(|mut value| {
            if *value == default {
                None
            } else {
                Some(std::mem::replace(&mut *value, default))
            }
        })
    }
}

impl<T> Source<T> for Arc<DynamicData<T>> {
    fn try_map_generational<R>(
        &self,
        map: impl FnOnce(DynamicGuard<'_, T, true>) -> R,
    ) -> Result<R, DeadlockError> {
        let state = self.state()?;
        Ok(map(DynamicGuard {
            guard: DynamicOrOwnedGuard::Dynamic(state),
            accessed_mut: false,
            prevent_notifications: false,
        }))
    }

    fn on_change_try<F>(&self, on_change: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: FnMut() -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        dynamic_for_each(self, on_change)
    }

    fn for_each_subsequent_generational_try<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(DynamicGuard<'a, T, true>) -> Result<(), CallbackDisconnected>
            + Send
            + 'static,
    {
        let this = WeakDynamic(Arc::downgrade(self));
        dynamic_for_each(self, move || {
            let this = this.upgrade().ok_or(CallbackDisconnected)?;
            this.map_generational(&mut for_each)?;
            Ok(())
        })
    }

    fn for_each_subsequent_generational_cloned_try<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Clone + Send + 'static,
        F: FnMut(GenerationalValue<T>) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        let this = WeakDynamic(Arc::downgrade(self));
        dynamic_for_each(self, move || {
            let this = this.upgrade().ok_or(CallbackDisconnected)?;

            if let Ok(value) = this.try_map_generational(|g| g.guard.clone()) {
                for_each(value)?;
            }

            Ok(())
        })
    }
}

impl<T> Source<T> for Dynamic<T> {
    fn try_map_generational<R>(
        &self,
        map: impl FnOnce(DynamicGuard<'_, T, true>) -> R,
    ) -> Result<R, DeadlockError> {
        self.0.try_map_generational(map)
    }

    fn on_change_try<F>(&self, on_change: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: FnMut() -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        dynamic_for_each(&self.0, on_change)
    }

    fn for_each_subsequent_generational_try<F>(&self, for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(DynamicGuard<'_, T, true>) -> Result<(), CallbackDisconnected>
            + Send
            + 'static,
    {
        self.0.for_each_subsequent_generational_try(for_each)
    }

    fn for_each_subsequent_generational_cloned_try<F>(&self, for_each: F) -> CallbackHandle
    where
        T: Clone + Send + 'static,
        F: FnMut(GenerationalValue<T>) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.0.for_each_subsequent_generational_cloned_try(for_each)
    }
}

impl<T> Source<T> for DynamicReader<T> {
    fn try_map_generational<R>(
        &self,
        map: impl FnOnce(DynamicGuard<'_, T, true>) -> R,
    ) -> Result<R, DeadlockError> {
        self.source.try_map_generational(|generational| {
            *self.read_generation.lock() = generational.generation();
            map(generational)
        })
    }

    fn on_change_try<F>(&self, on_change: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: FnMut() -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        dynamic_for_each(&self.source, on_change)
    }

    fn for_each_subsequent_generational_try<F>(&self, for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(DynamicGuard<'_, T, true>) -> Result<(), CallbackDisconnected>
            + Send
            + 'static,
    {
        self.source.for_each_subsequent_generational_try(for_each)
    }

    fn for_each_subsequent_generational_cloned_try<F>(&self, for_each: F) -> CallbackHandle
    where
        T: Clone + Send + 'static,
        F: FnMut(GenerationalValue<T>) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.source
            .for_each_subsequent_generational_cloned_try(for_each)
    }
}

impl<T> Destination<T> for Dynamic<T> {
    fn try_map_mut<R>(&self, map: impl FnOnce(Mutable<'_, T>) -> R) -> Result<R, DeadlockError> {
        self.0.map_mut(map)
    }
}

/// A `mut` reference to `T` that tracks whether the contents have been accessed
/// through `DerefMut`.
#[derive(Debug)]
pub struct Mutable<'a, T> {
    value: &'a mut T,
    mutated: Mutated<'a>,
}

#[derive(Debug)]
enum Mutated<'a> {
    External(&'a mut bool),
    Ignored,
}

impl Mutated<'_> {
    fn set(&mut self, mutated: bool) {
        match self {
            Self::External(value) => **value = mutated,
            Self::Ignored => {}
        }
    }
}

impl<T> Deref for Mutable<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> DerefMut for Mutable<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mutated.set(true);
        self.value
    }
}

impl<'a, T> Mutable<'a, T> {
    /// Creates a new wrapper that sets `mutated` to true when `DerefMut` is
    /// used to access `value`.
    #[must_use]
    pub fn new(value: &'a mut T, mutated: &'a mut bool) -> Self {
        *mutated = false;
        Self {
            value,
            mutated: Mutated::External(mutated),
        }
    }
}

impl<'a, T> From<&'a mut T> for Mutable<'a, T> {
    fn from(value: &'a mut T) -> Self {
        Self {
            value,
            mutated: Mutated::Ignored,
        }
    }
}

/// A unique, reactive value.
///
/// This type is useful for situations where a value is owned by exactly one
/// type but needs to have reactivity through [`Source`]/[`Destination`].
///
/// A [`Dynamic`] utilizes a [`Arc`] + [`Mutex`] to support updating its values
/// from multiple threads. This type utilizes a [`RefCell`], preventing it from
/// being shared between multiple threads.
#[derive(Default)]
pub struct Owned<T> {
    wrapped: RefCell<GenerationalValue<T>>,
    callbacks: Arc<OwnedCallbacks<T>>,
}

impl<T> Owned<T> {
    /// Returns a new reactive value.
    pub fn new(value: T) -> Self {
        Self {
            wrapped: RefCell::new(GenerationalValue {
                value,
                generation: Generation::default(),
            }),
            callbacks: Arc::default(),
        }
    }

    /// Borrows the contents of this value with read-only access.
    pub fn borrow(&self) -> OwnedRef<'_, T> {
        OwnedRef(self.wrapped.borrow())
    }

    /// Borrows the contents of this value with exclusive access.
    ///
    /// When the returned type is accessed through [`DerefMut`], all associated
    /// reactive callbacks will be invoked upon dropping the returned
    /// [`OwnedMut`].
    pub fn borrow_mut(&mut self) -> OwnedMut<'_, T> {
        OwnedMut {
            borrowed: self.wrapped.borrow_mut(),
            accessed_mut: false,
            owned: self,
        }
    }

    /// Returns the contained value.
    pub fn into_inner(self) -> T {
        self.wrapped.into_inner().value
    }
}

impl<T> Source<T> for Owned<T> {
    fn try_map_generational<R>(
        &self,
        map: impl FnOnce(DynamicGuard<'_, T, true>) -> R,
    ) -> Result<R, DeadlockError> {
        Ok(map(DynamicGuard {
            guard: DynamicOrOwnedGuard::Owned(self.wrapped.borrow_mut()),
            accessed_mut: false,
            prevent_notifications: false,
        }))
    }

    fn on_change_try<F>(&self, mut on_change: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: FnMut() -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        let mut callbacks = self.callbacks.active.lock();
        CallbackHandle(CallbackHandleInner::Single(CallbackHandleData {
            id: Some(
                callbacks.push(Box::new(move |g: DynamicGuard<'_, T, true>| {
                    drop(g);
                    on_change()
                })),
            ),
            owner: None,
            callbacks: self.callbacks.clone(),
        }))
    }

    fn for_each_subsequent_generational_try<F>(&self, for_each: F) -> CallbackHandle
    where
        T: Send + 'static,
        F: for<'a> FnMut(DynamicGuard<'a, T, true>) -> Result<(), CallbackDisconnected>
            + Send
            + 'static,
    {
        let mut callbacks = self.callbacks.active.lock();
        CallbackHandle(CallbackHandleInner::Single(CallbackHandleData {
            id: Some(callbacks.push(Box::new(for_each))),
            owner: None,
            callbacks: self.callbacks.clone(),
        }))
    }

    fn for_each_subsequent_generational_cloned_try<F>(&self, mut for_each: F) -> CallbackHandle
    where
        T: Clone + Send + 'static,
        F: FnMut(GenerationalValue<T>) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.for_each_generational_try(move |gen| for_each(GenerationalValue::from(&gen.guard)))
    }
}

impl<T> Destination<T> for Owned<T>
where
    T: 'static,
{
    fn try_map_mut<R>(&self, map: impl FnOnce(Mutable<'_, T>) -> R) -> Result<R, DeadlockError> {
        let mut updated = false;
        let result = map(Mutable::new(
            &mut self.wrapped.borrow_mut().value,
            &mut updated,
        ));
        if updated {
            self.callbacks.invoke(&mut &self.wrapped, |wrapped| {
                DynamicOrOwnedGuard::Owned(wrapped.borrow_mut())
            });
        }
        Ok(result)
    }
}

#[cfg(feature = "serde")]
impl<T> serde::Serialize for Owned<T>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.map_ref(|this| this.serialize(serializer))
    }
}

#[cfg(feature = "serde")]
impl<'de, T> serde::Deserialize<'de> for Owned<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::new)
    }
}

/// A read-only reference to the value in an [`Owned`].
pub struct OwnedRef<'a, T>(Ref<'a, GenerationalValue<T>>)
where
    T: 'static;

impl<T> Deref for OwnedRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An exclusive reference to the value contained in an [`Owned`].
///
/// This type tracks if the referenced value is accessed through [`DerefMut`].
/// If it is, reactive callbacks associated with the [`Owned`] value will be
/// invoked.
pub struct OwnedMut<'a, T>
where
    T: 'static,
{
    owned: &'a Owned<T>,
    borrowed: RefMut<'a, GenerationalValue<T>>,
    accessed_mut: bool,
}

impl<T> Deref for OwnedMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.borrowed.value
    }
}

impl<T> DerefMut for OwnedMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.accessed_mut = true;
        &mut self.borrowed.value
    }
}

impl<T> Drop for OwnedMut<'_, T>
where
    T: 'static,
{
    fn drop(&mut self) {
        if self.accessed_mut {
            self.owned.callbacks.invoke(&mut self.borrowed, |borrowed| {
                DynamicOrOwnedGuard::OwnedRef(&mut *borrowed)
            });
        }
    }
}

struct OwnedCallbacks<T> {
    active: Mutex<Lots<Box<dyn OwnedCallbackFn<T>>>>,
}

impl<T> Default for OwnedCallbacks<T> {
    fn default() -> Self {
        Self {
            active: Mutex::default(),
        }
    }
}

impl<T> OwnedCallbacks<T>
where
    T: 'static,
{
    pub fn invoke<'a, U>(
        &self,
        user: &'a mut U,
        value: impl for<'b> Fn(&'b mut U) -> DynamicOrOwnedGuard<'b, T, true>,
    ) {
        let mut callbacks = self.active.lock();
        callbacks.drain_filter(|callback| {
            callback
                .updated(DynamicGuard {
                    guard: value(user),
                    accessed_mut: false,
                    prevent_notifications: false,
                })
                .is_err()
        });
    }
}

impl<T> CallbackCollection for OwnedCallbacks<T>
where
    T: 'static,
{
    fn remove(&self, id: LotId) {
        self.active.lock().remove(id);
    }
}

trait OwnedCallbackFn<T>: Send + 'static {
    fn updated(&mut self, value: DynamicGuard<'_, T, true>) -> Result<(), CallbackDisconnected>;
}

impl<F, T> OwnedCallbackFn<T> for F
where
    F: for<'a> FnMut(DynamicGuard<'a, T, true>) -> Result<(), CallbackDisconnected>
        + Send
        + 'static,
{
    fn updated(&mut self, value: DynamicGuard<'_, T, true>) -> Result<(), CallbackDisconnected> {
        self(value)
    }
}

/// An instance of a value that provides APIs to observe and react to its
/// contents.
pub struct Dynamic<T>(Arc<DynamicData<T>>);

impl<T> Dynamic<T> {
    /// Creates a new instance wrapping `value`.
    pub fn new(value: T) -> Self {
        let state = State::new(value);
        let lock = state.callbacks.lock.clone();
        Self(Arc::new(DynamicData {
            state: Mutex::new(state),
            lock,
        }))
    }

    pub(crate) fn as_ptr(&self) -> *const () {
        Arc::as_ptr(&self.0).cast()
    }

    /// Returns a weak reference to this dynamic.
    ///
    /// This is powered by [`Arc`]/[`Weak`] and follows the same semantics for
    /// reference counting.
    #[must_use]
    pub fn downgrade(&self) -> WeakDynamic<T> {
        WeakDynamic::from(self)
    }

    /// Returns the number [`Dynamic`]s that point to this same value.
    ///
    /// The returned count includes `self`.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn instances(&self) -> usize {
        Arc::strong_count(&self.0) - self.readers()
    }

    /// Returns the number of [`DynamicReader`]s for this value.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn readers(&self) -> usize {
        self.state::<true>().expect("deadlocked").readers
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
        let r = Dynamic::new(
            self.map_ref(|v| t_into_r(v))
                .into()
                .expect("t_into_r must succeed with the current value"),
        );
        let r_weak = r.downgrade();
        r.set_source(self.for_each_try(move |t| {
            let r = r_weak.upgrade().ok_or(CallbackDisconnected)?;
            if let Some(update) = t_into_r(t).into() {
                r.set(update);
            }
            Ok(())
        }));

        // The linked dynamic holds a reference to the original, since it's
        // being created from the original.
        let t = self.clone();
        self.set_source(r.for_each_try(move |r| {
            if let Some(update) = r_into_t(r).into() {
                let _result = t.replace(update);
            }
            Ok(())
        }));

        r
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

    /// Returns a dynamic that is synchronized with a borrowed value from
    /// `self`.
    ///
    /// When the returned dynamic is updated, `self` will be updated using
    /// `get_mut`.
    pub fn linked_accessor<U, Getter, Setter>(&self, get: Getter, get_mut: Setter) -> Dynamic<U>
    where
        T: Send + 'static,
        U: PartialEq + Clone + Send + 'static,
        Getter: Fn(&T) -> &U + Send + Clone + 'static,
        Setter: Fn(&mut T) -> &mut U + Send + 'static,
    {
        let ignore_changes = Arc::new(AtomicBool::new(false));

        let linked = Dynamic::new(self.map_ref(|source| get(source).clone()));
        let weak_linked = linked.downgrade();
        let weak_source = self.downgrade();

        linked.set_source(self.for_each_generational_try({
            let ignore_changes = ignore_changes.clone();
            let get = get.clone();
            move |source| {
                if ignore_changes.load(Ordering::Relaxed) {
                    return Ok(());
                }

                let linked = weak_linked.upgrade().ok_or(CallbackDisconnected)?;
                let new_value = get(&*source).clone();
                drop(source);

                if let Ok(mut linked) = linked.try_lock() {
                    if *linked != new_value {
                        ignore_changes.store(true, Ordering::Relaxed);
                        *linked = new_value;
                        drop(linked);
                        ignore_changes.store(false, Ordering::Relaxed);
                    }
                }
                Ok(())
            }
        }));

        linked
            .for_each_generational_try(move |linked| {
                if ignore_changes.load(Ordering::Relaxed) {
                    return Ok(());
                }

                let source = weak_source.upgrade().ok_or(CallbackDisconnected)?;
                let new_value = linked.clone();
                drop(linked);

                if let Ok(mut source) = source.try_lock() {
                    if get(&*source) != &new_value {
                        ignore_changes.store(true, Ordering::Relaxed);
                        *get_mut(&mut source) = new_value;
                        drop(source);
                        ignore_changes.store(false, Ordering::Relaxed);
                    }
                }
                Ok(())
            })
            .persist();

        linked
    }

    /// Sets the current `source` for this dynamic with `source`.
    ///
    /// A dynamic can have multiple source callbacks.
    ///
    /// This ensures that `source` stays active as long as any clones of `self`
    /// are alive.
    pub fn set_source(&self, source: CallbackHandle) {
        self.state::<true>().assert("deadlocked").source_callback += source;
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

    /// A helper function that invokes `with_clone` with a clone of self. This
    /// code may produce slightly more readable code.
    ///
    /// ```rust
    /// use cushy::value::{Dynamic, Source};
    ///
    /// let value = Dynamic::new(1);
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

    /// Returns a new reference-based reader for this dynamic value.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn create_reader(&self) -> DynamicReader<T> {
        let mut state = self.state::<true>().expect("deadlocked");
        state.readers += 1;
        DynamicReader {
            source: self.0.clone(),
            read_generation: Mutex::new(state.wrapped.generation),
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
        self.lock_inner()
    }

    /// Returns an exclusive reference to the contents of this dynamic.
    ///
    /// This call will block until all other guards for this dynamic have been
    /// dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if the current thread already holds a lock to this
    /// dynamic.
    pub fn try_lock(&self) -> Result<DynamicGuard<'_, T>, DeadlockError> {
        Ok(DynamicGuard {
            guard: DynamicOrOwnedGuard::Dynamic(self.0.state()?),
            accessed_mut: false,
            prevent_notifications: false,
        })
    }

    fn try_lock_nonblocking<const READONLY: bool>(
        &self,
    ) -> Result<DynamicGuard<'_, T, READONLY>, TryLockError> {
        Ok(DynamicGuard {
            guard: DynamicOrOwnedGuard::Dynamic(self.0.state_nonblocking()?),
            accessed_mut: false,
            prevent_notifications: false,
        })
    }

    fn lock_inner<const READONLY: bool>(&self) -> DynamicGuard<'_, T, READONLY> {
        let guard = self.0.state().expect("deadlocked");
        DynamicGuard {
            guard: DynamicOrOwnedGuard::Dynamic(guard),
            accessed_mut: false,
            prevent_notifications: false,
        }
    }

    fn state<const READONLY: bool>(
        &self,
    ) -> Result<DynamicMutexGuard<'_, T, READONLY>, DeadlockError> {
        self.0.state()
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
    /// pressed.
    #[must_use]
    pub fn new_radio(&self, widget_value: T) -> Radio<T>
    where
        Self: Clone,
        // Technically this trait bound isn't necessary, but it prevents trying
        // to call new_radio on unsupported types. The MakeWidget/Widget
        // implementations require these bounds (and more).
        T: Clone + PartialEq,
    {
        Radio::new(widget_value, self.clone())
    }

    /// Returns a new checkbox that updates `self` when clicked.
    #[must_use]
    pub fn new_checkbox(&self) -> Checkbox
    where
        Self: IntoDynamic<CheckboxState>,
    {
        Checkbox::new(self.clone())
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
        T: Clone + PartialEq,
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

#[cfg(feature = "serde")]
impl<T> serde::Serialize for Dynamic<T>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.map_ref(|this| this.serialize(serializer))
    }
}

#[cfg(feature = "serde")]
impl<'de, T> serde::Deserialize<'de> for Dynamic<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::new)
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
        self.into_reader().into_switcher()
    }

    /// Returns a new [`Switcher`] widget whose contents is the value of this
    /// dynamic.
    #[must_use]
    pub fn to_switcher(&self) -> Switcher {
        self.create_reader().into_switcher()
    }
}

impl DynamicReader<WidgetInstance> {
    /// Returns a new [`Switcher`] widget whose contents is the value of this
    /// dynamic reader.
    #[must_use]
    pub fn into_switcher(self) -> Switcher {
        Switcher::new(self)
    }

    /// Returns a new [`Switcher`] widget whose contents is the value of this
    /// dynamic reader.
    #[must_use]
    pub fn to_switcher(&self) -> Switcher {
        Switcher::new(self.clone())
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
    fn inner_redraw_when_changed(&self, handle: WindowHandle) {
        self.0.redraw_when_changed(handle);
    }

    fn inner_sync_when_changed(&self, handle: WindowHandle) {
        self.0.sync_when_changed(handle);
    }

    fn inner_invalidate_when_changed(&self, handle: WindowHandle, id: WidgetId) {
        self.0.invalidate_when_changed(handle, id);
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
        if let Ok(mut state) = self.state::<true>() {
            if Arc::strong_count(&self.0) == state.readers + 1 {
                let cleanup = state.cleanup();
                drop(state);
                drop(cleanup);

                self.0.lock.sync.notify_all();
            }
        } else {
            // In the event that this is the rare edge case and a reader is
            // blocking, we want to signal that we've dropped the final
            // reference.
            self.0.lock.sync.notify_all();
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

struct DynamicMutexGuard<'a, T, const READONLY: bool> {
    dynamic: &'a DynamicData<T>,
    guard: MutexGuard<'a, State<T>>,
    released_hold: bool,
}

impl<T, const READONLY: bool> Debug for DynamicMutexGuard<'_, T, READONLY>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.guard.debug("DynamicMutexGuard", f)
    }
}

impl<T, const READONLY: bool> DynamicMutexGuard<'_, T, READONLY> {
    fn unlocked<R>(&mut self, while_unlocked: impl FnOnce() -> R) -> R {
        MutexGuard::unlocked(&mut self.guard, || {
            let mut state = self.dynamic.lock.state.lock();
            let current_holder = state.lock_holder.take();
            drop(state);
            self.dynamic.lock.sync.notify_all();
            let result = while_unlocked();

            let mut state = self.dynamic.lock.state.lock();
            state.lock_holder = current_holder;
            result
        })
    }

    fn release_hold(&mut self) {
        self.released_hold = true;
        self.dynamic.lock.state.lock().lock_holder = None;
        self.dynamic.lock.sync.notify_all();
    }
}

impl<T, const READONLY: bool> Drop for DynamicMutexGuard<'_, T, READONLY> {
    fn drop(&mut self) {
        if !self.released_hold {
            self.release_hold();
        }
    }
}

impl<T, const READONLY: bool> Deref for DynamicMutexGuard<'_, T, READONLY> {
    type Target = State<T>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T, const READONLY: bool> DerefMut for DynamicMutexGuard<'_, T, READONLY> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

struct DynamicData<T> {
    state: Mutex<State<T>>,
    lock: Arc<DynamicLockData>,
}

impl<T> DynamicData<T> {
    fn state<const READONLY: bool>(
        &self,
    ) -> Result<DynamicMutexGuard<'_, T, READONLY>, DeadlockError> {
        self.state_inner::<_, _, READONLY, true>(|mut already_locked| {
            already_locked.block();
            Ok(already_locked)
        })
    }

    fn state_nonblocking<'a, const READONLY: bool>(
        &'a self,
    ) -> Result<DynamicMutexGuard<'a, T, READONLY>, TryLockError<'a>> {
        self.state_inner::<_, _, READONLY, false>(|state: AlreadyLocked<'a>| {
            Err(TryLockError::AlreadyLocked(state))
        })
    }

    fn state_inner<'a, E, F, const READONLY: bool, const BLOCKING: bool>(
        &'a self,
        mut when_locked: F,
    ) -> Result<DynamicMutexGuard<'a, T, READONLY>, E>
    where
        E: std::fmt::Debug + From<DeadlockError>,
        F: FnMut(AlreadyLocked<'a>) -> Result<AlreadyLocked<'a>, E>,
    {
        let current_thread_id = std::thread::current().id();
        let mut lock = self.lock.state.lock();
        loop {
            match lock.lock_holder {
                None => break,
                Some(holder) if holder == current_thread_id => return Err(DeadlockError.into()),
                Some(_) => {
                    let AlreadyLocked { state, .. } = when_locked(AlreadyLocked {
                        state: lock,
                        sync: &self.lock.sync,
                    })?;
                    lock = state;
                }
            }
        }

        lock.lock_holder = Some(current_thread_id);

        let guard = if BLOCKING {
            self.state.lock()
        } else {
            loop {
                if let Some(guard) = self.state.try_lock() {
                    break guard;
                }

                let AlreadyLocked { state, .. } = match when_locked(AlreadyLocked {
                    state: lock,
                    sync: &self.lock.sync,
                }) {
                    Ok(locked) => locked,
                    Err(other) => {
                        self.lock.state.lock().lock_holder = None;
                        return Err(other);
                    }
                };
                lock = state;
            }
        };
        drop(lock);

        Ok(DynamicMutexGuard {
            dynamic: self,
            guard,
            released_hold: false,
        })
    }

    pub fn redraw_when_changed(&self, window: WindowHandle) {
        let mut state = self.state::<true>().expect("deadlocked");
        state.invalidation.windows.insert(window, true);
    }

    pub fn sync_when_changed(&self, window: WindowHandle) {
        let mut state = self.state::<true>().expect("deadlocked");
        state.invalidation.windows.entry(window).or_insert(false);
    }

    pub fn invalidate_when_changed(&self, window: WindowHandle, widget: WidgetId) {
        let mut state = self.state::<true>().expect("deadlocked");
        state.invalidation.widgets.insert((window, widget));
    }

    pub fn map_mut<R>(&self, map: impl FnOnce(Mutable<T>) -> R) -> Result<R, DeadlockError> {
        let mut state_guard = self.state::<true>()?;
        let (old, callbacks) = {
            let state = &mut *state_guard;
            let mut changed = false;
            let result = map(Mutable::new(&mut state.wrapped.value, &mut changed));
            let callbacks = changed.then(|| state_guard.note_changed());

            (result, callbacks)
        };
        drop(state_guard);
        if let Some(callbacks) = callbacks {
            defer_execute_callbacks(callbacks);
        }

        Ok(old)
    }
}

fn dynamic_for_each<T, F>(this: &Arc<DynamicData<T>>, map: F) -> CallbackHandle
where
    F: FnMut() -> Result<(), CallbackDisconnected> + Send + 'static,
    T: Send + 'static,
{
    let state = this.state::<true>().expect("deadlocked");
    let mut data = state.callbacks.callbacks.lock();
    CallbackHandle(CallbackHandleInner::Single(CallbackHandleData {
        id: Some(data.callbacks.push(Box::new(map))),
        owner: Some(this.clone()),
        callbacks: state.callbacks.clone(),
    }))
}

/// A callback function is no longer connected to its source.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CallbackDisconnected;

struct DebugDynamicData<'a, T>(&'a Arc<DynamicData<T>>);

impl<T> Debug for DebugDynamicData<'_, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.state::<true>() {
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
pub struct DeadlockError;

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
#[must_use = "Callbacks are disconnected once the associated CallbackHandle is dropped. Consider using `CallbackHandle::persist()` to prevent the callback from being disconnected."]
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

trait ReferencedDynamic: Sync + Send + 'static {}
impl<T> ReferencedDynamic for T where T: Sync + Send + 'static {}

struct CallbackHandleData {
    id: Option<LotId>,
    owner: Option<Arc<dyn ReferencedDynamic>>,
    callbacks: Arc<dyn CallbackCollection>,
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

    /// Drops any references to owning [`Dynamic`]s associated with this
    /// callback.
    ///
    /// This enables creating weak connections between callback graphs.
    pub fn forget_owners(&mut self) {
        match &mut self.0 {
            CallbackHandleInner::None => {}
            CallbackHandleInner::Single(handle) => {
                handle.owner = None;
            }
            CallbackHandleInner::Multi(handles) => {
                for handle in handles {
                    handle.owner = None;
                }
            }
        }
    }

    /// Drops any references to owning [`Dynamic`]s associated with this
    /// callback, and returns self.
    ///
    /// This uses [`Self::forget_owners()`].
    pub fn weak(mut self) -> Self {
        self.forget_owners();
        self
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
            self.callbacks.remove(id);
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

#[derive(Default)]
struct InvalidationState {
    windows: AHashMap<WindowHandle, bool>,
    widgets: AHashSet<(WindowHandle, WidgetId)>,
    wakers: Vec<Waker>,
}

impl InvalidationState {
    fn invoke(&mut self) {
        for (window, widget) in self.widgets.drain() {
            window.invalidate(widget);
        }
        for (window, redraw) in self.windows.drain() {
            if redraw {
                window.redraw();
            } else {
                window.sync();
            }
        }
        for waker in self.wakers.drain(..) {
            waker.wake();
        }
    }

    fn extend(&mut self, other: &mut InvalidationState) {
        self.widgets.extend(other.widgets.drain());
        self.windows.extend(other.windows.drain());

        for waker in other.wakers.drain(..) {
            if !self
                .wakers
                .iter()
                .any(|existing| existing.will_wake(&waker))
            {
                self.wakers.push(waker);
            }
        }
    }
}

struct State<T> {
    wrapped: GenerationalValue<T>,
    source_callback: CallbackHandle,
    callbacks: Arc<ChangeCallbacksData>,
    invalidation: InvalidationState,
    on_disconnect: Option<Vec<OnceCallback>>,
    readers: usize,
}

impl<T> State<T> {
    fn new(value: T) -> Self {
        Self {
            wrapped: GenerationalValue {
                value,
                generation: Generation::default(),
            },
            callbacks: Arc::default(),
            invalidation: InvalidationState {
                windows: AHashMap::new(),
                wakers: Vec::new(),
                widgets: AHashSet::new(),
            },
            readers: 0,
            on_disconnect: Some(Vec::new()),
            source_callback: CallbackHandle::default(),
        }
    }

    fn note_changed(&mut self) -> ChangeCallbacks {
        self.wrapped.generation = self.wrapped.generation.next();

        if !InvalidationBatch::take_invalidations(&mut self.invalidation) {
            self.invalidation.invoke();
        }

        ChangeCallbacks::new(self.callbacks.clone())
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

    #[must_use]
    fn cleanup(&mut self) -> StateCleanup {
        StateCleanup {
            on_disconnect: self.on_disconnect.take(),
            wakers: std::mem::take(&mut self.invalidation.wakers),
        }
    }
}

impl<T> Drop for State<T> {
    fn drop(&mut self) {
        // Ensure any disconnections that didn't fire due to deadlocking still
        // are invoked.
        drop(self.cleanup());
    }
}

struct StateCleanup {
    on_disconnect: Option<Vec<OnceCallback>>,
    wakers: Vec<Waker>,
}

impl Drop for StateCleanup {
    fn drop(&mut self) {
        for on_disconnect in self.on_disconnect.take().into_iter().flatten() {
            on_disconnect.invoke(());
        }

        for waker in self.wakers.drain(..) {
            waker.wake();
        }
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

#[derive(Default, Debug)]
pub(super) struct DynamicLockState {
    lock_holder: Option<ThreadId>,
    pub(super) callbacks_to_remove: Vec<LotId>,
}

#[derive(Default, Debug)]
pub(super) struct DynamicLockData {
    pub(super) state: Mutex<DynamicLockState>,
    pub(super) sync: Condvar,
}

/// A value stored in a [`Dynamic`] with its [`Generation`].
#[derive(Default, Clone, Debug, Eq, PartialEq)]
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

impl<T, const READONLY: bool> From<&DynamicGuard<'_, T, READONLY>> for GenerationalValue<T>
where
    T: Clone,
{
    fn from(value: &DynamicGuard<'_, T, READONLY>) -> Self {
        Self {
            value: (**value).clone(),
            generation: value.generation(),
        }
    }
}

impl<T, const READONLY: bool> From<&DynamicOrOwnedGuard<'_, T, READONLY>> for GenerationalValue<T>
where
    T: Clone,
{
    fn from(value: &DynamicOrOwnedGuard<'_, T, READONLY>) -> Self {
        (**value).clone()
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

#[derive(Debug)]
enum DynamicOrOwnedGuard<'a, T, const READONLY: bool> {
    Dynamic(DynamicMutexGuard<'a, T, READONLY>),
    Owned(RefMut<'a, GenerationalValue<T>>),
    OwnedRef(&'a mut GenerationalValue<T>),
}
impl<T, const READONLY: bool> DynamicOrOwnedGuard<'_, T, READONLY> {
    fn note_changed(&mut self) -> Option<ChangeCallbacks> {
        match self {
            Self::Dynamic(guard) => Some(guard.note_changed()),
            Self::Owned(_) | Self::OwnedRef(_) => None,
        }
    }

    fn unlocked<R>(&mut self, while_unlocked: impl FnOnce() -> R) -> R {
        match self {
            Self::Dynamic(guard) => guard.unlocked(while_unlocked),
            Self::Owned(_) | Self::OwnedRef(_) => while_unlocked(),
        }
    }
}

impl<T, const READONLY: bool> Deref for DynamicOrOwnedGuard<'_, T, READONLY> {
    type Target = GenerationalValue<T>;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Dynamic(guard) => &guard.wrapped,
            Self::Owned(r) => r,
            Self::OwnedRef(r) => r,
        }
    }
}

impl<T, const READONLY: bool> DerefMut for DynamicOrOwnedGuard<'_, T, READONLY> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Dynamic(guard) => &mut guard.wrapped,
            Self::Owned(r) => r,
            Self::OwnedRef(r) => r,
        }
    }
}

/// An exclusive reference to the contents of a [`Dynamic`].
///
/// If the contents are accessed through [`DerefMut`], all obververs will be
/// notified of a change when this guard is dropped.
#[derive(Debug)]
pub struct DynamicGuard<'a, T, const READONLY: bool = false> {
    guard: DynamicOrOwnedGuard<'a, T, READONLY>,
    accessed_mut: bool,
    prevent_notifications: bool,
}

impl<T, const READONLY: bool> DynamicGuard<'_, T, READONLY> {
    /// Returns the generation of the value at the time of locking the dynamic.
    ///
    /// Even if this guard accesses the data through [`DerefMut`], this value
    /// will remain unchanged while the guard is held.
    #[must_use]
    pub fn generation(&self) -> Generation {
        self.guard.generation
    }

    /// Prevent any access through [`DerefMut`] from triggering change
    /// notifications.
    pub fn prevent_notifications(&mut self) {
        self.prevent_notifications = true;
    }

    /// Executes `while_unlocked` while this guard is temporarily unlocked.
    pub fn unlocked<F, R>(&mut self, while_unlocked: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.guard.unlocked(while_unlocked)
    }
}

impl<T, const READONLY: bool> Deref for DynamicGuard<'_, T, READONLY> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard.value
    }
}

impl<T> DerefMut for DynamicGuard<'_, T, false> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.accessed_mut = true;
        &mut self.guard.value
    }
}

impl<T, const READONLY: bool> Drop for DynamicGuard<'_, T, READONLY> {
    fn drop(&mut self) {
        if self.accessed_mut && !self.prevent_notifications {
            let callbacks = self.guard.note_changed();
            if let Some(callbacks) = callbacks {
                defer_execute_callbacks(callbacks);
            }
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

/// A reader of a [`Dynamic<T>`] that tracks the last generation accessed.
pub struct DynamicReader<T> {
    source: Arc<DynamicData<T>>,
    read_generation: Mutex<Generation>,
}

impl<T> DynamicReader<T> {
    /// Returns an read-only, exclusive reference to the contents of this
    /// dynamic.
    ///
    /// This call will block until all other guards for this dynamic have been
    /// dropped.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn lock(&self) -> DynamicGuard<'_, T, true> {
        DynamicGuard {
            guard: DynamicOrOwnedGuard::Dynamic(self.source.state().expect("deadlocked")),
            accessed_mut: false,
            prevent_notifications: false,
        }
    }

    fn try_lock_nonblocking(&self) -> Result<DynamicGuard<'_, T, true>, TryLockError> {
        Ok(DynamicGuard {
            guard: DynamicOrOwnedGuard::Dynamic(self.source.state_nonblocking()?),
            accessed_mut: false,
            prevent_notifications: false,
        })
    }

    /// Returns the current generation that has been accessed through this
    /// reader.
    #[must_use]
    pub fn read_generation(&self) -> Generation {
        *self.read_generation.lock()
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
        self.source
            .state::<true>()
            .expect("deadlocked")
            .wrapped
            .generation
            != self.read_generation()
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
    pub fn block_until_updated(&self) -> bool {
        assert_ne!(
            self.source.lock.state.lock().lock_holder,
            Some(std::thread::current().id()),
            "deadlocked"
        );
        let mut state = self.source.state.lock();
        loop {
            if state.wrapped.generation != self.read_generation() {
                return true;
            } else if state.readers == Arc::strong_count(&self.source)
                || state.on_disconnect.is_none()
            {
                return false;
            }

            // Wait for a notification of a change, which is synch
            self.source.lock.sync.wait(&mut state);
        }
    }

    /// Returns true if this reader still has any writers connected to it.
    #[must_use]
    pub fn connected(&self) -> bool {
        let state = self.source.state.lock();
        state.readers < Arc::strong_count(&self.source) && state.on_disconnect.is_some()
    }

    /// Suspends the current async task until the contained value has been
    /// updated or there are no remaining writers for the value.
    ///
    /// Returns true if a newly updated value was discovered.
    pub fn wait_until_updated(&self) -> BlockUntilUpdatedFuture<'_, T> {
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
        let mut state = self.source.state::<true>().expect("deadlocked");

        if let Some(callbacks) = &mut state.on_disconnect {
            callbacks.push(OnceCallback::new(|()| on_disconnect()));
        }
    }
}

impl<T> context::sealed::Trackable for DynamicReader<T> {
    fn inner_redraw_when_changed(&self, handle: WindowHandle) {
        self.source.redraw_when_changed(handle);
    }

    fn inner_sync_when_changed(&self, handle: WindowHandle) {
        self.source.sync_when_changed(handle);
    }

    fn inner_invalidate_when_changed(&self, handle: WindowHandle, id: WidgetId) {
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
            .field("read_generation", &self.read_generation().0)
            .finish()
    }
}

impl<T> Clone for DynamicReader<T> {
    fn clone(&self) -> Self {
        self.source.state::<true>().expect("deadlocked").readers += 1;
        Self {
            source: self.source.clone(),
            read_generation: Mutex::new(self.read_generation()),
        }
    }
}

impl<T> Drop for DynamicReader<T> {
    fn drop(&mut self) {
        let mut state = self.source.state::<true>().expect("deadlocked");
        state.readers -= 1;
    }
}

/// Suspends the current async task until the contained value has been
/// updated or there are no remaining writers for the value.
///
/// Yeilds true if a newly updated value was discovered.
#[derive(Debug)]
#[must_use = "futures must be .await'ed to be executed"]
pub struct BlockUntilUpdatedFuture<'a, T>(&'a DynamicReader<T>);

impl<T> Future for BlockUntilUpdatedFuture<'_, T> {
    type Output = bool;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.source.state::<true>().expect("deadlocked");
        if state.wrapped.generation != self.0.read_generation() {
            return Poll::Ready(true);
        } else if state.readers == Arc::strong_count(&self.0.source)
            || state.on_disconnect.is_none()
        {
            return Poll::Ready(false);
        }

        state.invalidation.wakers.push(cx.waker().clone());
        Poll::Pending
    }
}

#[test]
fn disconnecting_reader_from_dynamic() {
    let value = Dynamic::new(1);
    let ref_reader = value.create_reader();
    drop(value);
    assert!(!ref_reader.block_until_updated());
}

#[test]
fn disconnecting_reader_threaded() {
    let a = Dynamic::new(1);
    let a_reader = a.create_reader();
    let b = Dynamic::new(1);
    let b_reader = b.create_reader();

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
    let a_reader = a.create_reader();
    let b = Dynamic::new(1);
    let b_reader = b.create_reader();

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

impl Add for Generation {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for Generation {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_add(rhs.0);
    }
}

/// A type that can convert into a `ReadOnly<T>`.
pub trait IntoReadOnly<T> {
    /// Returns `self` as a `ReadOnly`.
    fn into_read_only(self) -> ReadOnly<T>;
}

impl<T> IntoReadOnly<T> for T {
    fn into_read_only(self) -> ReadOnly<T> {
        ReadOnly::Constant(self)
    }
}

impl<T> IntoReadOnly<T> for ReadOnly<T> {
    fn into_read_only(self) -> ReadOnly<T> {
        self
    }
}

impl<T> IntoReadOnly<T> for Value<T> {
    fn into_read_only(self) -> ReadOnly<T> {
        match self {
            Value::Constant(value) => ReadOnly::Constant(value),
            Value::Dynamic(dynamic) => ReadOnly::Reader(dynamic.into_reader()),
        }
    }
}

impl<T> IntoReadOnly<T> for Dynamic<T> {
    fn into_read_only(self) -> ReadOnly<T> {
        self.create_reader().into_read_only()
    }
}

impl<T> IntoReadOnly<T> for DynamicReader<T> {
    fn into_read_only(self) -> ReadOnly<T> {
        ReadOnly::Reader(self)
    }
}

impl<T> IntoReadOnly<T> for Owned<T> {
    fn into_read_only(self) -> ReadOnly<T> {
        ReadOnly::Constant(self.into_inner())
    }
}

/// A type that can be converted into a [`DynamicReader<T>`].
pub trait IntoReader<T> {
    /// Returns this value as a reader.
    fn into_reader(self) -> DynamicReader<T>;
}

impl<T> IntoReader<T> for Dynamic<T> {
    fn into_reader(self) -> DynamicReader<T> {
        self.into_reader()
    }
}

impl<T> IntoReader<T> for DynamicReader<T> {
    fn into_reader(self) -> DynamicReader<T> {
        self
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

impl GetWidget<usize> for WidgetList {
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

/// A value that can only be read from.
pub enum ReadOnly<T> {
    /// A value that will not ever change externally.
    Constant(T),
    /// A value that is read from a dynamic.
    Reader(DynamicReader<T>),
}

impl<T> ReadOnly<T> {
    /// Returns a clone of the currently stored value.
    #[must_use]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        match self {
            Self::Constant(value) => value.clone(),
            Self::Reader(value) => value.get(),
        }
    }

    /// Returns the current generation of the data stored, if the contained
    /// value is [`Dynamic`].
    pub fn generation(&self) -> Option<Generation> {
        match self {
            Self::Constant(_) => None,
            Self::Reader(value) => Some(value.generation()),
        }
    }

    /// Maps the current contents to `map` and returns the result.
    pub fn map<R>(&self, map: impl FnOnce(&T) -> R) -> R {
        match self {
            Self::Constant(value) => map(value),
            Self::Reader(dynamic) => dynamic.map_ref(map),
        }
    }

    /// Returns a new value that is updated using `U::from(T.clone())` each time
    /// `self` is updated.
    #[must_use]
    pub fn map_each<R, F>(&self, mut map: F) -> ReadOnly<R>
    where
        T: Send + 'static,
        F: for<'a> FnMut(&'a T) -> R + Send + 'static,
        R: PartialEq + Send + 'static,
    {
        match self {
            Self::Constant(value) => ReadOnly::Constant(map(value)),
            Self::Reader(dynamic) => ReadOnly::Reader(dynamic.map_each(map).into_reader()),
        }
    }
}

impl<T> From<DynamicReader<T>> for ReadOnly<T> {
    fn from(value: DynamicReader<T>) -> Self {
        Self::Reader(value)
    }
}

impl<T> From<Dynamic<T>> for ReadOnly<T> {
    fn from(value: Dynamic<T>) -> Self {
        Self::from(value.into_reader())
    }
}

impl<T> From<Owned<T>> for ReadOnly<T> {
    fn from(value: Owned<T>) -> Self {
        Self::Constant(value.into_inner())
    }
}

impl<T> Debug for ReadOnly<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(arg0) => Debug::fmt(arg0, f),
            Self::Reader(arg0) => Debug::fmt(arg0, f),
        }
    }
}

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
        context: &WidgetContext<'_>,
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
        context: &WidgetContext<'_>,
        map: impl FnOnce(&T) -> R,
    ) -> R {
        match self {
            Value::Constant(value) => map(value),
            Value::Dynamic(dynamic) => {
                dynamic.invalidate_when_changed(context);
                dynamic.map_ref(map)
            }
        }
    }

    /// Maps the current contents with exclusive access and returns the result.
    pub fn map_mut<R>(&mut self, map: impl FnOnce(Mutable<'_, T>) -> R) -> R {
        match self {
            Value::Constant(value) => map(Mutable::from(value)),
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
    pub fn get_tracking_redraw(&self, context: &WidgetContext<'_>) -> T
    where
        T: Clone,
    {
        self.map_tracking_redraw(context, Clone::clone)
    }

    /// Returns a clone of the currently stored value.
    ///
    /// If `self` is a dynamic, `context` will be invalidated when the value is
    /// updated.
    pub fn get_tracking_invalidate(&self, context: &WidgetContext<'_>) -> T
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
}

impl<T> crate::context::sealed::Trackable for ReadOnly<T> {
    fn inner_invalidate_when_changed(&self, handle: WindowHandle, id: WidgetId) {
        if let ReadOnly::Reader(dynamic) = self {
            dynamic.inner_invalidate_when_changed(handle, id);
        }
    }

    fn inner_sync_when_changed(&self, handle: WindowHandle) {
        if let ReadOnly::Reader(dynamic) = self {
            dynamic.inner_sync_when_changed(handle);
        }
    }

    fn inner_redraw_when_changed(&self, handle: WindowHandle) {
        if let ReadOnly::Reader(dynamic) = self {
            dynamic.inner_redraw_when_changed(handle);
        }
    }
}

impl<T> crate::context::sealed::Trackable for Value<T> {
    fn inner_invalidate_when_changed(&self, handle: WindowHandle, id: WidgetId) {
        if let Value::Dynamic(dynamic) = self {
            dynamic.inner_invalidate_when_changed(handle, id);
        }
    }

    fn inner_sync_when_changed(&self, handle: WindowHandle) {
        if let Value::Dynamic(dynamic) = self {
            dynamic.inner_sync_when_changed(handle);
        }
    }

    fn inner_redraw_when_changed(&self, handle: WindowHandle) {
        if let Value::Dynamic(dynamic) = self {
            dynamic.inner_redraw_when_changed(handle);
        }
    }
}

impl<T> From<Dynamic<T>> for Value<T> {
    fn from(value: Dynamic<T>) -> Self {
        Self::Dynamic(value)
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

impl IntoValue<String> for &'_ str {
    fn into_value(self) -> Value<String> {
        Value::Constant(self.to_owned())
    }
}

impl IntoReadOnly<String> for &'_ str {
    fn into_read_only(self) -> ReadOnly<String> {
        ReadOnly::Constant(self.to_string())
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

    /// Invokes `for_each` with the current contents and each time this source's
    /// contents are updated.
    fn for_each<F>(&self, for_each: F) -> CallbackHandle
    where
        F: for<'a> FnMut(Self::Ref<'a>) + Send + 'static;

    /// Attaches `for_each` to this value so that it is invoked each time the
    /// source's contents are updated.
    ///
    /// `for_each` will not be invoked with the currently stored value.
    fn for_each_subsequent<F>(&self, for_each: F) -> CallbackHandle
    where
        F: for<'a> FnMut(Self::Ref<'a>) + Send + 'static;
}

macro_rules! impl_tuple_for_each {
    ($($type:ident $source:ident $field:tt $var:ident),+) => {
        impl<$($type,$source,)+> ForEach<($($type,)+)> for ($(&$source,)+)
        where
            $(
                $source: DynamicRead<$type> + Source<$type> + Clone + Send + 'static,
                $type: Send + 'static,
            )+
        {
            type Ref<'a> = ($(&'a $type,)+);

            #[allow(unused_mut)]
            fn for_each<F>(&self, mut for_each: F) -> CallbackHandle
            where
                F: for<'a> FnMut(Self::Ref<'a>) + Send + 'static,
            {
                {
                    $(let $var = self.$field.read();)+
                    for_each(($(&$var,)+));
                };
                self.for_each_subsequent(for_each)
            }

            #[allow(unused_mut)]
            fn for_each_subsequent<F>(&self, mut for_each: F) -> CallbackHandle
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
        $handles += $self.$field.for_each(move |field| $for_each((field,)));
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
        $handles += $var.on_change_try({
            let for_each = $for_each.clone();
            $(let $avar = $avar.clone();)+
            move || {
                loop {
                    let result = 'locks: {
                        $(let $avar = match $avar.read_nonblocking() {
                            Ok(guard) => guard,
                            Err($crate::value::TryLockError::WouldDeadlock) => panic!("Deadlocked"),
                            Err($crate::value::TryLockError::AlreadyLocked(locked)) => {
                                break 'locks Err(locked);
                            }
                        };)+

                        Ok(($($avar,)+))
                    };
                    match result {
                        Ok(($($avar,)+)) => {
                            let mut for_each = for_each.lock();
                            (for_each)(($(&$avar,)+));
                            return Ok(())
                        }
                        Err(mut already_locked) => {
                            already_locked.block();
                        }
                    }
                }
            }
        });
    };
}

/// A lock was unable to be acquired.
#[derive(Debug)]
pub enum TryLockError<'guard> {
    /// Attempting to acquire this lock would have resulted in a deadlock.
    WouldDeadlock,
    /// The lock is currently acquired.
    ///
    /// The returned structure can be used to block the current thread until
    /// locking can be retried.
    AlreadyLocked(AlreadyLocked<'guard>),
}

impl From<DeadlockError> for TryLockError<'_> {
    fn from(_value: DeadlockError) -> Self {
        Self::WouldDeadlock
    }
}

/// A lock could not be aquired without blocking.
#[derive(Debug)]
pub struct AlreadyLocked<'guard> {
    state: MutexGuard<'guard, DynamicLockState>,
    sync: &'guard Condvar,
}

impl AlreadyLocked<'_> {
    /// Blocks the current thread until the lock state has changed.
    pub fn block(&mut self) {
        self.sync.wait(&mut self.state);
    }
}

/// Read access to a value stored in a [`Dynamic`].
pub trait DynamicRead<T> {
    /// Returns a guard that provides exclusive, read-only access to the value
    /// contained wihtin this dynamic.
    fn read(&self) -> DynamicGuard<'_, T, true>;

    /// Attempts to obtain a guard that provides exclusive, read-only access to
    /// the value contained wihtin this dynamic.
    ///
    /// # Errors
    ///
    /// Returns an error if blocking would be required to lock this dynamic.
    fn read_nonblocking(&self) -> Result<DynamicGuard<'_, T, true>, TryLockError>;
}

impl<T> DynamicRead<T> for Dynamic<T> {
    fn read(&self) -> DynamicGuard<'_, T, true> {
        self.lock_inner()
    }

    fn read_nonblocking(&self) -> Result<DynamicGuard<'_, T, true>, TryLockError> {
        self.try_lock_nonblocking()
    }
}

impl<T> DynamicRead<T> for DynamicReader<T> {
    fn read(&self) -> DynamicGuard<'_, T, true> {
        self.lock()
    }

    fn read_nonblocking(&self) -> Result<DynamicGuard<'_, T, true>, TryLockError> {
        self.try_lock_nonblocking()
    }
}

impl_all_tuples!(impl_tuple_for_each, 2);

/// A type that can create a `Dynamic<U>` from a `T` passed into a mapping
/// function.
pub trait MapEach<T, U> {
    /// The borrowed representation of `T` passed into the mapping function.
    type Ref<'a>
    where
        T: 'a;

    /// Apply `map_each` to each value in `self`, storing the result in the
    /// returned dynamic.
    fn map_each<F>(&self, map_each: F) -> Dynamic<U>
    where
        F: for<'a> FnMut(Self::Ref<'a>) -> U + Send + 'static;
}

macro_rules! impl_tuple_map_each {
    ($($type:ident $source:ident $field:tt $var:ident),+) => {
        impl<U, $($type,$source),+> MapEach<($($type,)+), U> for ($(&$source,)+)
        where
            U: PartialEq + Send + 'static,
            $(
                $type: Send + 'static,
                $source: DynamicRead<$type> + Source<$type> + Clone + Send + 'static,
            )+
        {
            type Ref<'a> = ($(&'a $type,)+);

            fn map_each<F>(&self, mut map_each: F) -> Dynamic<U>
            where
                F: for<'a> FnMut(Self::Ref<'a>) -> U + Send + 'static,
            {
                let dynamic = {
                    $(let $var = self.$field.read();)+

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

impl_all_tuples!(impl_tuple_map_each, 2);

/// A type that can have a `for_each` operation applied to it.
pub trait ForEachCloned<T> {
    /// Apply `for_each` to each value contained within `self`.
    fn for_each_cloned<F>(&self, for_each: F) -> CallbackHandle
    where
        F: for<'a> FnMut(T) + Send + 'static;
}

macro_rules! impl_tuple_for_each_cloned {
    ($($type:ident $source:ident $field:tt $var:ident),+) => {
        impl<$($type,$source,)+> ForEachCloned<($($type,)+)> for ($(&$source,)+)
        where
            $(
                $type: Clone + Send + 'static,
                $source: Source<$type> + Clone + Send + 'static,
            )+
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
        $handles += $self.$field.for_each_cloned(move |field| $for_each((field,)));
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
                if let Some(mut for_each) =
                    for_each.try_lock() {
                (for_each)(($($avar,)+));
                    }
            }
        }));
    };
}

impl_all_tuples!(impl_tuple_for_each_cloned, 2);

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
    ($($type:ident $source:ident $field:tt $var:ident),+) => {
        impl<U, $($type,$source),+> MapEachCloned<($($type,)+), U> for ($(&$source,)+)
        where
            U: PartialEq + Send + 'static,
            $(
                $type: Clone + Send + 'static,
                $source: Source<$type> + Clone + Send + 'static,
            )+
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

impl_all_tuples!(impl_tuple_map_each_cloned, 2);

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
        let error_message = dynamic.map_each_generational(move |gen| message_mapping(&gen.guard));

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
        self.invalid.map_mut(|mut invalid| *invalid += 1);

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
                    invalid_count.map_mut(|mut invalid| *invalid += 1);
                } else {
                    invalid_count.map_mut(|mut invalid| *invalid -= 1);
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
            dynamic.map_each_generational(move |generational| map_to_message(&generational.guard));
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

/// A batch of invalidations across one or more windows.
///
/// This type helps background tasks synchronize when to invalidate or redraw a
/// widget. Without this type, if a tracked dynamic is changed, the window is
/// immediately sent a request to redraw itself. These requests are batched to
/// ensure efficiency, but if a background task is updating several dynamics
/// independent of one another, it may desire that those updates only trigger
/// one redraw per "step".
///
/// The closure invoked by [`InvalidationBatch::batch`] will gather all
/// invalidations into a single batch that can be executed by the handle
/// provided or automatically when the closure returns.
pub struct InvalidationBatch<'a>(&'a RefCell<InvalidationBatchGuard>);

#[derive(Default)]
struct InvalidationBatchGuard {
    nesting: usize,
    state: InvalidationState,
}

thread_local! {
    static GUARD: RefCell<InvalidationBatchGuard> = RefCell::default();
}

impl InvalidationBatch<'_> {
    /// Executes `batched` gathering all tracked invalidations into a shared
    /// batch.
    ///
    /// The closure accepts an `&InvalidationBatch<'_>` parameter which can be
    /// used to [`invoke()`](Self::invoke) the batch on-demand while during
    /// `batched`.
    ///
    /// This function supports nested invocation. When nested, only the
    /// outermost batch can manually invoke. When the outermost batch's callback
    /// ends, any pending invalidations are invoked automatically.
    pub fn batch(batched: impl FnOnce(&InvalidationBatch<'_>)) {
        GUARD.with(|guard| {
            let mut batch = guard.borrow_mut();
            batch.nesting += 1;
            drop(batch);

            batched(&InvalidationBatch(guard));

            let mut batch = guard.borrow_mut();
            batch.nesting -= 1;
            if batch.nesting == 0 {
                batch.state.invoke();
            }
        });
    }

    /// Invokes all pending invalidations.
    ///
    /// This function is a no-op if `self` is a nested batch. Only the root
    /// batch of each thread can trigger invalidations manually.
    pub fn invoke(&self) {
        let mut batch = self.0.borrow_mut();
        if batch.nesting == 1 {
            batch.state.invoke();
        }
    }

    #[must_use]
    fn take_invalidations(state: &mut InvalidationState) -> bool {
        GUARD.with(|guard| {
            let mut batch = guard.borrow_mut();
            if batch.nesting > 0 {
                // A batch is active on this thread
                batch.state.extend(state);
                true
            } else {
                false
            }
        })
    }
}

/// Watches one or more [`Source`]s and invokes associated callbacks when
/// changed.
///
/// This type is useful when needing to ensure logic is executed or a value is
/// regenerated each time one of many sources are changed.
#[derive(Debug, Clone, Default)]
pub struct Watcher(Dynamic<usize>);

impl Watcher {
    /// Notifies any observers of this watcher and invokes all associated
    /// callbacks.
    pub fn notify(&self) {
        let mut counter = self.0.lock();
        *counter = counter.wrapping_add(1);
    }

    /// Ensures all callbacks attached to this watcher are invoked when `other`
    /// is changed.
    pub fn watch<T>(&self, other: &impl Source<T>)
    where
        T: Send + 'static,
    {
        let counter = self.clone();
        self.0
            .set_source(other.for_each_subsequent_generational(move |guard| {
                // We want to drop our guard before changing the counter to
                // ensure all callbacks associated with our counter are executed
                // without this type holding any source locks.
                drop(guard);
                counter.notify();
            }));
    }

    /// Returns a new dynamic populated by invoking `when_changed` each time any
    /// watched source is updated.
    pub fn map_changed<F, T>(&self, mut when_changed: F) -> Dynamic<T>
    where
        F: FnMut() -> T + Send + 'static,
        T: PartialEq + Send + 'static,
    {
        self.0.map_each(move |_| when_changed())
    }

    /// Invokes `when_changed` each time any watched source is updated.
    pub fn when_changed<F>(&self, mut when_changed: F) -> CallbackHandle
    where
        F: FnMut() + Send + 'static,
    {
        self.0.for_each(move |_| when_changed())
    }
}

impl Source<usize> for Watcher {
    fn try_map_generational<R>(
        &self,
        map: impl FnOnce(DynamicGuard<'_, usize, true>) -> R,
    ) -> Result<R, DeadlockError> {
        self.0.try_map_generational(map)
    }

    fn on_change_try<F>(&self, on_change: F) -> CallbackHandle
    where
        F: FnMut() -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.0.on_change_try(on_change)
    }

    fn for_each_subsequent_generational_try<F>(&self, for_each: F) -> CallbackHandle
    where
        F: for<'a> FnMut(DynamicGuard<'_, usize, true>) -> Result<(), CallbackDisconnected>
            + Send
            + 'static,
    {
        self.0.for_each_subsequent_generational_try(for_each)
    }

    fn for_each_subsequent_generational_cloned_try<F>(&self, for_each: F) -> CallbackHandle
    where
        F: FnMut(GenerationalValue<usize>) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.0.for_each_subsequent_generational_cloned_try(for_each)
    }
}

impl crate::context::sealed::Trackable for Watcher {
    fn inner_invalidate_when_changed(&self, handle: WindowHandle, id: WidgetId) {
        self.0.inner_invalidate_when_changed(handle, id);
    }

    fn inner_redraw_when_changed(&self, handle: WindowHandle) {
        self.0.inner_redraw_when_changed(handle);
    }

    fn inner_sync_when_changed(&self, handle: WindowHandle) {
        self.0.inner_sync_when_changed(handle);
    }
}

/// A value that has its read and updated states tracked.
pub struct Tracked<Source>
where
    Source: TrackedSource,
{
    current: Source::Value,
    source: Source,
    current_generation: Generation,
    unread: bool,
}

impl<Source> Tracked<Source>
where
    Source: TrackedSource,
{
    /// Returns a new tracked instance.
    pub fn new(source: Source) -> Self {
        let (current, current_generation) = source.read();
        Self {
            current,
            current_generation,
            source,
            unread: true,
        }
    }

    /// Marks the initial value as read and returns self.
    #[must_use]
    pub fn ignoring_first(mut self) -> Self {
        self.unread = false;
        self
    }

    /// Marks the initial value as read if `test` returns true. Returns self.
    #[must_use]
    pub fn ignoring_first_if(mut self, test: impl FnOnce(&Source::Value) -> bool) -> Self {
        if test(&self.current) {
            self.unread = false;
        }
        self
    }

    /// Updates this tracked instance's cached value from the source.
    ///
    /// Returns true if a value hasn't been read yet.
    pub fn update(&mut self) -> bool {
        if let Some((updated, updated_generation)) =
            self.source.read_if_needed(self.current_generation)
        {
            self.current = updated;
            self.current_generation = updated_generation;
            self.unread = true;
            true
        } else {
            self.unread
        }
    }

    /// Returns the current value from the source, if it hasn't been read
    /// before.
    pub fn updated(&mut self) -> Option<&Source::Value> {
        if self.update() {
            self.unread = false;
            Some(&self.current)
        } else {
            None
        }
    }

    /// Returns true if the currently cached value hasn't been read.
    pub const fn unread(&self) -> bool {
        self.unread
    }

    /// Reads the current value from the source.
    pub fn read(&mut self) -> &Source::Value {
        self.update();
        self.read_cached()
    }

    /// Reads the current cached value.
    ///
    /// This function does not check if an updated value exists in the source.
    pub fn read_cached(&mut self) -> &Source::Value {
        self.unread = false;
        self.peek()
    }

    /// Returns the current cached value without changing the unread state.
    pub const fn peek(&self) -> &Source::Value {
        &self.current
    }

    /// Returns the source being tracked.
    pub const fn source(&self) -> &Source {
        &self.source
    }

    /// Marks the current value in the source as being read.
    pub fn mark_read(&mut self) {
        self.unread = false;
        self.current_generation = self.source.generation();
    }

    /// Updates the value stored in the source, and marks it as being read.
    pub fn set_and_read(&mut self, new_value: Source::Value)
    where
        Source::Value: PartialEq + Clone,
    {
        self.current = new_value;
        self.unread = false;
        if self.source.set(self.current.clone()) {
            self.current_generation = self.source.generation();
        }
    }

    /// Updates the value stored in the source.
    pub fn set(&mut self, new_value: Source::Value)
    where
        Source::Value: PartialEq + Clone,
    {
        self.current = new_value;
        if self.source.set(self.current.clone()) {
            self.current_generation = self.source.generation();
        }
    }
}

/// A [`Source`] that can be used in a [`Tracked`] instance.
pub trait TrackedSource: sealed::TrackedSource {}

mod sealed {
    use super::Generation;
    pub trait TrackedSource {
        type Value;
        fn read(&self) -> (Self::Value, Generation);
        fn read_if_needed(&self, read_generation: Generation) -> Option<(Self::Value, Generation)>;
        fn generation(&self) -> Generation;
        fn set(&self, new_value: Self::Value) -> bool;
    }
}

impl<T> TrackedSource for Dynamic<T> where T: Clone + PartialEq {}

impl<T> sealed::TrackedSource for Dynamic<T>
where
    T: Clone + PartialEq,
{
    type Value = T;

    fn read(&self) -> (Self::Value, Generation) {
        self.map_generational(|g| (g.clone(), g.generation()))
    }

    fn read_if_needed(&self, read_generation: Generation) -> Option<(Self::Value, Generation)> {
        self.map_generational(|g| {
            if g.generation() == read_generation {
                None
            } else {
                Some((g.clone(), g.generation()))
            }
        })
    }

    fn generation(&self) -> Generation {
        Source::generation(self)
    }

    fn set(&self, new_value: Self::Value) -> bool {
        let mut value = self.lock();
        if *value == new_value {
            false
        } else {
            *value = new_value;
            true
        }
    }
}

impl<T> TrackedSource for Value<T> where T: Clone + PartialEq {}

impl<T> sealed::TrackedSource for Value<T>
where
    T: Clone + PartialEq,
{
    type Value = T;

    fn read(&self) -> (Self::Value, Generation) {
        match self {
            Value::Constant(value) => (value.clone(), Generation::default()),
            Value::Dynamic(value) => sealed::TrackedSource::read(value),
        }
    }

    fn read_if_needed(&self, read_generation: Generation) -> Option<(Self::Value, Generation)> {
        match self {
            Value::Constant(_) => None,
            Value::Dynamic(value) => sealed::TrackedSource::read_if_needed(value, read_generation),
        }
    }

    fn generation(&self) -> Generation {
        match self {
            Value::Constant(_) => Generation::default(),
            Value::Dynamic(value) => Source::generation(value),
        }
    }

    fn set(&self, new_value: Self::Value) -> bool {
        match self {
            Value::Constant(_) => false,
            Value::Dynamic(value) => sealed::TrackedSource::set(value, new_value),
        }
    }
}

impl<S> Destination<S::Value> for Tracked<S>
where
    S: TrackedSource + Destination<S::Value>,
{
    fn try_map_mut<R>(
        &self,
        map: impl FnOnce(Mutable<'_, S::Value>) -> R,
    ) -> Result<R, DeadlockError> {
        self.source.try_map_mut(map)
    }
}

impl<T> From<Dynamic<T>> for Tracked<Dynamic<T>>
where
    T: Clone + PartialEq,
{
    fn from(source: Dynamic<T>) -> Self {
        Self::new(source)
    }
}

impl<T> From<Value<T>> for Tracked<Value<T>>
where
    T: Clone + PartialEq,
{
    fn from(source: Value<T>) -> Self {
        Self::new(source)
    }
}

impl<Source> context::sealed::Trackable for Tracked<Source>
where
    Source: Trackable + TrackedSource,
{
    fn inner_invalidate_when_changed(&self, handle: WindowHandle, id: WidgetId) {
        self.source.inner_invalidate_when_changed(handle, id);
    }

    fn inner_sync_when_changed(&self, handle: WindowHandle) {
        self.source.inner_sync_when_changed(handle);
    }

    fn inner_redraw_when_changed(&self, handle: WindowHandle) {
        self.source.inner_redraw_when_changed(handle);
    }
}

#[test]
fn compare_swap() {
    let dynamic = Dynamic::new(1);
    assert_eq!(dynamic.compare_swap(&1, 2), Ok(1));
    assert_eq!(dynamic.compare_swap(&1, 0), Err(2));
    assert_eq!(dynamic.compare_swap(&2, 0), Ok(2));
    assert_eq!(dynamic.get(), 0);
}

#[test]
fn ref_counts() {
    let dynamic = Dynamic::new(1);
    assert_eq!(dynamic.instances(), 1);

    let second = dynamic.clone();
    assert_eq!(dynamic.instances(), 2);

    assert_eq!(dynamic.readers(), 0);
    let reader = second.into_reader();
    assert_eq!(dynamic.instances(), 1);
    assert_eq!(dynamic.readers(), 1);

    // Test that once the last instance is dropped that the reader is no longer
    // connected and that on_disconnect gets invoked.
    assert!(reader.connected());
    let invoked = Dynamic::new(false);
    reader.on_disconnect({
        let invoked = invoked.clone();
        move || {
            invoked.set(true);
        }
    });
    drop(dynamic);

    assert!(invoked.get());
    assert!(!reader.connected());
}

#[test]
fn linked_short_circuit() {
    let usize = Dynamic::new(0_usize);
    let usize_reader = usize.create_reader();
    let string = usize.linked_string();

    string.map_ref(|s| assert_eq!(s, "0"));
    string.set(String::from("1"));
    usize_reader.block_until_updated();
    assert_eq!(usize.get(), 1);

    let string_reader = string.create_reader();
    usize.set(2);
    string_reader.block_until_updated();
    string.map_ref(|s| assert_eq!(s, "2"));
}

#[test]
fn graph_shortcircuit() {
    let a = Dynamic::new(0_usize);
    let doubled = a.map_each_cloned(|a| dbg!(a) * 2);
    let doubled_reader = doubled.create_reader();
    let quadrupled = doubled.map_each_cloned(|doubled| dbg!(doubled) * 2);
    let invocation_count = Dynamic::new(0_usize);
    a.set_source(quadrupled.for_each_cloned({
        let a = a.clone();
        let invocation_count = invocation_count.clone();
        move |quad| {
            *invocation_count.lock() += 1;
            a.set(dbg!(quad) / 4);
        }
    }));
    let invocation_count = invocation_count.into_reader();

    assert_eq!(a.get(), 0);
    assert_eq!(quadrupled.get(), 0);
    a.set(1);

    // We expect two invocations at this point:
    // - Once by using quadrupled.for_each_cloned.
    // - Once by the callback chain invoked by setting a to 1.
    while invocation_count.get() < 2 {
        invocation_count.block_until_updated();
    }

    assert_eq!(doubled_reader.get(), 2);
    assert_eq!(quadrupled.get(), 4);
    quadrupled.set(16);
    doubled_reader.block_until_updated();
    assert_eq!(a.get(), 4);
    assert_eq!(doubled_reader.get(), 8);
}
