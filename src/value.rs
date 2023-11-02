//! Types for storing and interacting with values in Widgets.

use std::fmt::Debug;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use kludgine::app::WindowHandle;

use crate::context::WidgetContext;
use crate::window::sealed::WindowCommand;

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
            }),
            sync: AssertUnwindSafe(Condvar::new()),
        }))
    }

    /// Maps the contents with read-only access.
    pub fn map_ref<R>(&self, map: impl FnOnce(&T) -> R) -> R {
        let state = self.state();
        map(&state.wrapped.value)
    }

    /// Maps the contents with exclusive access. Before returning from this
    /// function, all observers will be notified that the contents have been
    /// updated.
    pub fn map_mut<R>(&self, map: impl FnOnce(&mut T) -> R) -> R {
        self.0.map_mut(map)
    }

    /// Attaches `for_each` to this value so that it is invoked each time the
    /// value's contents are updated.
    pub fn for_each<F>(&self, mut for_each: F)
    where
        F: for<'a> FnMut(&'a T) + Send + 'static,
    {
        self.0.for_each(move |gen| for_each(&gen.value));
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

    /// A helper function that invokes `with_clone` with a clone of self. This
    /// code may produce slightly more readable code.
    ///
    /// ```rust
    /// let value = gooey::dynamic::Dynamic::new(1);
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

    pub(crate) fn redraw_when_changed(&self, window: WindowHandle<WindowCommand>) {
        self.0.redraw_when_changed(window);
    }

    /// Returns a clone of the currently contained value.
    #[must_use]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.0.get().value
    }

    /// Replaces the contents with `new_value`, returning the previous contents.
    /// Before returning from this function, all observers will be notified that
    /// the contents have been updated.
    #[must_use]
    pub fn replace(&self, new_value: T) -> T {
        self.0.map_mut(|value| std::mem::replace(value, new_value))
    }

    /// Stores `new_value` in this dynamic. Before returning from this function,
    /// all observers will be notified that the contents have been updated.
    pub fn set(&self, new_value: T) {
        let _old = self.replace(new_value);
    }

    /// Returns a new reference-based reader for this dynamic value.
    #[must_use]
    pub fn create_ref_reader(&self) -> DynamicReader<T> {
        self.state().readers += 1;
        DynamicReader {
            source: self.0.clone(),
            read_generation: self.0.state().wrapped.generation,
        }
    }

    fn state(&self) -> MutexGuard<'_, State<T>> {
        self.0.state()
    }

    /// Returns the current generation of the value.
    #[must_use]
    pub fn generation(&self) -> Generation {
        self.state().wrapped.generation
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
        let state = self.state();
        if state.readers == 0 {
            drop(state);
            self.0.sync.notify_all();
        }
    }
}

impl<T> From<Dynamic<T>> for DynamicReader<T> {
    fn from(value: Dynamic<T>) -> Self {
        value.create_ref_reader()
    }
}

#[derive(Debug)]
struct DynamicData<T> {
    state: Mutex<State<T>>,

    // The AssertUnwindSafe is only needed on Mac. For some reason on
    // Mac OS, Condvar isn't RefUnwindSafe.
    sync: AssertUnwindSafe<Condvar>,
}

impl<T> DynamicData<T> {
    fn state(&self) -> MutexGuard<'_, State<T>> {
        self.state
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
    }

    pub fn redraw_when_changed(&self, window: WindowHandle<WindowCommand>) {
        let mut state = self.state();
        state.windows.push(window);
    }

    #[must_use]
    pub fn get(&self) -> GenerationalValue<T>
    where
        T: Clone,
    {
        self.state().wrapped.clone()
    }

    #[must_use]
    pub fn map_mut<R>(&self, map: impl FnOnce(&mut T) -> R) -> R {
        let mut state = self.state();
        let old = {
            let state = &mut *state;
            let generation = state.wrapped.generation.next();
            let result = map(&mut state.wrapped.value);
            state.wrapped.generation = generation;

            for callback in &mut state.callbacks {
                callback.update(&state.wrapped);
            }
            for window in state.windows.drain(..) {
                let _result = window.send(WindowCommand::Redraw);
            }
            result
        };
        drop(state);

        self.sync.notify_all();

        old
    }

    pub fn for_each<F>(&self, map: F)
    where
        F: for<'a> FnMut(&'a GenerationalValue<T>) + Send + 'static,
    {
        let mut state = self.state();
        state.callbacks.push(Box::new(map));
    }

    pub fn map_each<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        F: for<'a> FnMut(&'a GenerationalValue<T>) -> R + Send + 'static,
        R: Send + 'static,
    {
        let mut state = self.state();
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
}
struct State<T> {
    wrapped: GenerationalValue<T>,
    callbacks: Vec<Box<dyn ValueCallback<T>>>,
    windows: Vec<WindowHandle<WindowCommand>>,
    readers: usize,
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
    pub fn map_ref<R>(&mut self, map: impl FnOnce(&T) -> R) -> R {
        let state = self.source.state();
        self.read_generation = state.wrapped.generation;
        map(&state.wrapped.value)
    }

    /// Returns a clone of the currently contained value.
    ///
    /// This function marks the currently stored value as being read.
    #[must_use]
    pub fn get(&mut self) -> T
    where
        T: Clone,
    {
        let GenerationalValue { value, generation } = self.source.get();
        self.read_generation = generation;
        value
    }

    /// Blocks the current thread until the contained value has been updated or
    /// there are no remaining writers for the value.
    ///
    /// Returns true if a newly updated value was discovered.
    pub fn block_until_updated(&mut self) -> bool {
        let mut state = self.source.state();
        loop {
            if state.wrapped.generation != self.read_generation {
                return true;
            } else if state.readers == Arc::strong_count(&self.source) {
                return false;
            }

            state = self
                .source
                .sync
                .wait(state)
                .map_or_else(PoisonError::into_inner, |g| g);
        }
    }
}

impl<T> Clone for DynamicReader<T> {
    fn clone(&self) -> Self {
        self.source.state().readers += 1;
        Self {
            source: self.source.clone(),
            read_generation: self.read_generation,
        }
    }
}

impl<T> Drop for DynamicReader<T> {
    fn drop(&mut self) {
        let mut state = self.source.state();
        state.readers -= 1;
    }
}

#[test]
fn disconnecting_reader_from_dynamic() {
    let value = Dynamic::new(1);
    let mut ref_reader = value.create_ref_reader();
    drop(value);
    assert!(!ref_reader.block_until_updated());
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
    pub fn map<R>(&mut self, map: impl FnOnce(&T) -> R) -> R {
        match self {
            Value::Constant(value) => map(value),
            Value::Dynamic(dynamic) => dynamic.map_ref(map),
        }
    }

    /// Maps the current contents with exclusive access and returns the result.
    pub fn map_mut<R>(&mut self, map: impl FnOnce(&mut T) -> R) -> R {
        match self {
            Value::Constant(value) => map(value),
            Value::Dynamic(dynamic) => dynamic.map_mut(map),
        }
    }

    /// Returns a clone of the currently stored value.
    pub fn get(&mut self) -> T
    where
        T: Clone,
    {
        self.map(Clone::clone)
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
