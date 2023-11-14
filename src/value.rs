//! Types for storing and interacting with values in Widgets.

use std::cell::Cell;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, TryLockError};
use std::task::{Poll, Waker};
use std::thread::ThreadId;

use intentional::Assert;

use crate::animation::{DynamicTransition, LinearInterpolate};
use crate::context::{WidgetContext, WindowHandle};
use crate::utils::WithClone;

/// An instance of a value that provides APIs to observe and react to its
/// contents.
#[derive(Debug)]
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
                callbacks: Vec::new(),
                windows: Vec::new(),
                readers: 0,
                wakers: Vec::new(),
            }),
            during_callback_state: Mutex::default(),
            sync: AssertUnwindSafe(Condvar::new()),
        }))
    }

    /// Maps the contents with read-only access.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    pub fn map_ref<R>(&self, map: impl FnOnce(&T) -> R) -> R {
        let state = self.state().expect("deadlocked");
        map(&state.wrapped.value)
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

    /// Returns a new dynamic that is updated using `U::from(T.clone())` each
    /// time `self` is updated.
    #[must_use]
    pub fn map_each_into<U>(&self) -> Dynamic<U>
    where
        U: From<T> + Send + 'static,
        T: Clone,
    {
        self.map_each(|value| U::from(value.clone()))
    }

    /// Returns a new dynamic that is updated using `U::from(&T)` each
    /// time `self` is updated.
    #[must_use]
    pub fn map_each_to<U>(&self) -> Dynamic<U>
    where
        U: for<'a> From<&'a T> + Send + 'static,
        T: Clone,
    {
        self.map_each(|value| U::from(value))
    }

    /// Attaches `for_each` to this value so that it is invoked each time the
    /// value's contents are updated.
    pub fn for_each<F>(&self, mut for_each: F)
    where
        F: for<'a> FnMut(&'a T) + Send + 'static,
    {
        self.0.for_each(move |gen| for_each(&gen.value));
    }

    /// Attaches `for_each` to this value so that it is invoked each time the
    /// value's contents are updated. This function returns `self`.
    #[must_use]
    pub fn with_for_each<F>(self, mut for_each: F) -> Self
    where
        F: for<'a> FnMut(&'a T) + Send + 'static,
    {
        self.0.for_each(move |gen| for_each(&gen.value));
        self
    }

    /// Creates a new dynamic value that contains the result of invoking `map`
    /// each time this value is changed.
    pub fn map_each<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        F: for<'a> FnMut(&'a T) -> R + Send + 'static,
        R: Send + 'static,
    {
        self.0.map_each(move |gen| map(&gen.value))
    }

    /// Creates a new dynamic value that contains the result of invoking `map`
    /// each time this value is changed.
    ///
    /// This version of `map_each` uses [`Dynamic::try_update`] to prevent
    /// deadlocks and debounce dependent values.
    pub fn map_each_unique<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        F: for<'a> FnMut(&'a T) -> R + Send + 'static,
        R: Send + PartialEq + 'static,
    {
        self.0.map_each_unique(move |gen| map(&gen.value))
    }

    /// A helper function that invokes `with_clone` with a clone of self. This
    /// code may produce slightly more readable code.
    ///
    /// ```rust
    /// let value = gooey::value::Dynamic::new(1);
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
    pub fn get_tracked(&self, context: &WidgetContext<'_, '_>) -> T
    where
        T: Clone,
    {
        context.redraw_when_changed(self);
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
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    #[must_use]
    pub fn replace(&self, new_value: T) -> T {
        self.0
            .map_mut(|value, _| std::mem::replace(value, new_value))
            .expect("deadlocked")
    }

    /// Stores `new_value` in this dynamic. Before returning from this function,
    /// all observers will be notified that the contents have been updated.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    pub fn set(&self, new_value: T) {
        let _old = self.replace(new_value);
    }

    /// Updates this dynamic with `new_value`, but only if `new_value` is not
    /// equal to the currently stored value.
    ///
    /// Returns true if the value was updated.
    ///
    /// # Panics
    ///
    /// This function panics if this value is already locked by the current
    /// thread.
    pub fn update(&self, new_value: T) -> bool
    where
        T: PartialEq,
    {
        self.0
            .map_mut(|value, changed| {
                if *value == new_value {
                    *changed = false;
                    false
                } else {
                    *value = new_value;
                    true
                }
            })
            .expect("deadlocked")
    }

    /// Attempt to store `new_value` in `self`. If the value cannot be stored
    /// due to a deadlock, it is returned as an error.
    ///
    /// Returns true if the value was updated.
    pub fn try_update(&self, new_value: T) -> Result<bool, T>
    where
        T: PartialEq,
    {
        let cell = Cell::new(Some(new_value));
        self.0
            .map_mut(|value, changed| {
                let new_value = cell.take().assert("only one callback will be invoked");
                if *value == new_value {
                    *changed = false;
                    false
                } else {
                    *value = new_value;
                    true
                }
            })
            .map_err(|_| cell.take().assert("only one callback will be invoked"))
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
        let state = self.state().expect("deadlocked");
        if state.readers == 0 {
            drop(state);
            self.0.sync.notify_all();
        }
    }
}

impl<T> From<Dynamic<T>> for DynamicReader<T> {
    fn from(value: Dynamic<T>) -> Self {
        value.create_reader()
    }
}

#[derive(Debug)]
struct DynamicMutexGuard<'a, T> {
    dynamic: &'a DynamicData<T>,
    guard: MutexGuard<'a, State<T>>,
}

impl<'a, T> Drop for DynamicMutexGuard<'a, T> {
    fn drop(&mut self) {
        let mut during_state = self
            .dynamic
            .during_callback_state
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g);
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

#[derive(Debug)]
struct DynamicData<T> {
    state: Mutex<State<T>>,
    during_callback_state: Mutex<Option<LockState>>,

    // The AssertUnwindSafe is only needed on Mac. For some reason on
    // Mac OS, Condvar isn't RefUnwindSafe.
    sync: AssertUnwindSafe<Condvar>,
}

impl<T> DynamicData<T> {
    fn state(&self) -> Result<DynamicMutexGuard<'_, T>, DeadlockError> {
        let mut during_sync = self
            .during_callback_state
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g);

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
                            during_sync = self
                                .sync
                                .wait(during_sync)
                                .map_or_else(PoisonError::into_inner, |g| g);
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
        state.windows.push(window);
    }

    pub fn get(&self) -> Result<GenerationalValue<T>, DeadlockError>
    where
        T: Clone,
    {
        self.state().map(|state| state.wrapped.clone())
    }

    pub fn map_mut<R>(&self, map: impl FnOnce(&mut T, &mut bool) -> R) -> Result<R, DeadlockError> {
        let mut state = self.state()?;
        let old = {
            let state = &mut *state;
            let mut changed = true;
            let result = map(&mut state.wrapped.value, &mut changed);
            if changed {
                state.note_changed();
            }

            result
        };
        drop(state);

        self.sync.notify_all();

        Ok(old)
    }

    pub fn for_each<F>(&self, map: F)
    where
        F: for<'a> FnMut(&'a GenerationalValue<T>) + Send + 'static,
    {
        let mut state = self.state().expect("deadlocked");
        state.callbacks.push(Box::new(map));
    }

    pub fn map_each<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        F: for<'a> FnMut(&'a GenerationalValue<T>) -> R + Send + 'static,
        R: Send + 'static,
    {
        let mut state = self.state().expect("deadlocked");
        let initial_value = map(&state.wrapped);
        let mapped_value = Dynamic::new(initial_value);
        let returned = mapped_value.clone();
        state
            .callbacks
            .push(Box::new(move |updated: &GenerationalValue<T>| {
                mapped_value.set(map(updated));
            }));

        returned
    }

    pub fn map_each_unique<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        F: for<'a> FnMut(&'a GenerationalValue<T>) -> R + Send + 'static,
        R: PartialEq + Send + 'static,
    {
        let mut state = self.state().expect("deadlocked");
        let initial_value = map(&state.wrapped);
        let mapped_value = Dynamic::new(initial_value);
        let returned = mapped_value.clone();
        state
            .callbacks
            .push(Box::new(move |updated: &GenerationalValue<T>| {
                let _deadlock = mapped_value.try_update(map(updated));
            }));

        returned
    }
}

/// A deadlock occurred accessing a [`Dynamic`].
///
/// Currently Gooey is only able to detect deadlocks where a single thread tries
/// to lock the same [`Dynamic`] multiple times.
#[derive(Debug)]
pub struct DeadlockError;

impl std::error::Error for DeadlockError {}

impl Display for DeadlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a deadlock was detected")
    }
}

struct State<T> {
    wrapped: GenerationalValue<T>,
    callbacks: Vec<Box<dyn ValueCallback<T>>>,
    windows: Vec<WindowHandle>,
    wakers: Vec<Waker>,
    readers: usize,
}

impl<T> State<T> {
    fn note_changed(&mut self) {
        self.wrapped.generation = self.wrapped.generation.next();

        for callback in &mut self.callbacks {
            callback.update(&self.wrapped);
        }
        for window in self.windows.drain(..) {
            window.redraw();
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("wrapped", &self.wrapped)
            .field("readers", &self.readers)
            .finish_non_exhaustive()
    }
}

trait ValueCallback<T>: Send {
    fn update(&mut self, value: &GenerationalValue<T>);
}

impl<T, F> ValueCallback<T> for F
where
    F: for<'a> FnMut(&'a GenerationalValue<T>) + Send + 'static,
{
    fn update(&mut self, value: &GenerationalValue<T>) {
        self(value);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GenerationalValue<T> {
    pub value: T,
    pub generation: Generation,
}

/// An exclusive reference to the contents of a [`Dynamic`].
///
/// If the contents are accessed through [`DerefMut`], all obververs will be
/// notified of a change when this guard is dropped.
#[derive(Debug)]
pub struct DynamicGuard<'a, T> {
    guard: DynamicMutexGuard<'a, T>,
    accessed_mut: bool,
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
        if self.accessed_mut {
            self.guard.note_changed();
        }
    }
}

/// A reader that tracks the last generation accessed through this reader.
#[derive(Debug)]
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
        let mut deadlock_state = self
            .source
            .during_callback_state
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g);
        assert!(
            deadlock_state
                .as_ref()
                .map_or(true, |state| state.locked_thread
                    != std::thread::current().id()),
            "deadlocked"
        );
        loop {
            let state = self
                .source
                .state
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g);
            if state.wrapped.generation != self.read_generation {
                return true;
            } else if state.readers == Arc::strong_count(&self.source) {
                return false;
            }
            drop(state);

            // Wait for a notification of a change, which is synch
            deadlock_state = self
                .source
                .sync
                .wait(deadlock_state)
                .map_or_else(PoisonError::into_inner, |g| g);
        }
    }

    /// Suspends the current async task until the contained value has been
    /// updated or there are no remaining writers for the value.
    ///
    /// Returns true if a newly updated value was discovered.
    pub fn wait_until_updated(&mut self) -> BlockUntilUpdatedFuture<'_, T> {
        BlockUntilUpdatedFuture(self)
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
    T: Default,
{
    /// Returns [`Dynamic::default()`] with `self` installed as a for-each
    /// callback.
    fn into_dynamic(self) -> Dynamic<T> {
        Dynamic::default().with_for_each(self)
    }
}

/// A value that may be either constant or dynamic.
#[derive(Debug)]
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
    pub fn map_tracked<R>(&self, context: &WidgetContext<'_, '_>, map: impl FnOnce(&T) -> R) -> R {
        match self {
            Value::Constant(value) => map(value),
            Value::Dynamic(dynamic) => {
                context.redraw_when_changed(dynamic);
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
        F: for<'a> FnMut(&'a T) -> R + Send + 'static,
        R: Send + 'static,
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
    /// If `self` is a dynamic, `context` will be invalidated when the value is
    /// updated.
    pub fn get_tracked(&self, context: &WidgetContext<'_, '_>) -> T
    where
        T: Clone,
    {
        self.map_tracked(context, Clone::clone)
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

impl<T> IntoValue<T> for Dynamic<T> {
    fn into_value(self) -> Value<T> {
        Value::Dynamic(self)
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
    fn for_each<F>(&self, for_each: F)
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
            fn for_each<F>(&self, mut for_each: F)
            where
                F: for<'a> FnMut(Self::Ref<'a>) + Send + 'static,
            {
                impl_tuple_for_each!(self for_each [] [$($type $field $var),+]);
            }
        }
    };
    ($self:ident $for_each:ident [] [$type:ident $field:tt $var:ident]) => {
        $self.$field.for_each(move |field: &$type| $for_each((field,)));
    };
    ($self:ident $for_each:ident [] [$($type:ident $field:tt $var:ident),+]) => {
        let $for_each = Arc::new(Mutex::new($for_each));
        $(let $var = $self.$field.clone();)*


        impl_tuple_for_each!(invoke $self $for_each [] [$($type $field $var),+]);
    };
    (
        invoke
        // Identifiers used from the outer method
        $self:ident $for_each:ident
        // List of all tuple fields that have already been positioned as the focused call
        [$($ltype:ident $lfield:tt $lvar:ident),*]
        //
        [$type:ident $field:tt $var:ident, $($rtype:ident $rfield:tt $rvar:ident),+]
    ) => {
        impl_tuple_for_each!(
            invoke
            $self $for_each
            $type $field $var
            [$($ltype $lfield $lvar,)* $type $field $var, $($rtype $rfield $rvar),+]
            [$($ltype $lfield $lvar,)* $($rtype $rfield $rvar),+]
        );
        impl_tuple_for_each!(
            invoke
            $self $for_each
            [$($ltype $lfield $lvar,)* $type $field $var]
            [$($rtype $rfield $rvar),+]
        );
    };
    (
        invoke
        // Identifiers used from the outer method
        $self:ident $for_each:ident
        // List of all tuple fields that have already been positioned as the focused call
        [$($ltype:ident $lfield:tt $lvar:ident),+]
        //
        [$type:ident $field:tt $var:ident]
    ) => {
        impl_tuple_for_each!(
            invoke
            $self $for_each
            $type $field $var
            [$($ltype $lfield $lvar,)+ $type $field $var]
            [$($ltype $lfield $lvar),+]
        );
    };
    (
        invoke
        // Identifiers used from the outer method
        $self:ident $for_each:ident
        // Tuple field that for_each is being invoked on
        $type:ident $field:tt $var:ident
        // The list of all tuple fields in this invocation, in the correct order.
        [$($atype:ident $afield:tt $avar:ident),+]
        // The list of tuple fields excluding the one being invoked.
        [$($rtype:ident $rfield:tt $rvar:ident),+]
    ) => {
        $var.for_each((&$for_each, $(&$rvar,)+).with_clone(|(for_each, $($rvar,)+)| {
            move |$var: &$type| {
                $(let $rvar = $rvar.lock();)+
                let mut for_each =
                    for_each.lock().map_or_else(PoisonError::into_inner, |g| g);
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
            U: Send + 'static,
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
                self.for_each({
                    let dynamic = dynamic.clone();

                    move |tuple| {
                        dynamic.set(map_each(tuple));
                    }
                });
                dynamic
            }
        }
    };
}

impl_all_tuples!(impl_tuple_map_each);
