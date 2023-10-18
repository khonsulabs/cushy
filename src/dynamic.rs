use std::fmt::Debug;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use kludgine::app::WindowHandle;

use crate::window::sealed::WindowCommand;

#[derive(Debug)]
pub struct Dynamic<T>(Arc<DynamicData<T>>);

impl<T> Dynamic<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(DynamicData {
            state: Mutex::new(State {
                wrapped: GenerationalValue {
                    value,
                    generation: 0,
                },
                callbacks: Vec::new(),
                windows: Vec::new(),
                readers: 0,
            }),
            sync: Condvar::new(),
        }))
    }

    pub fn map_ref<R>(&self, map: impl FnOnce(&T) -> R) -> R {
        let state = self.state();
        map(&state.wrapped.value)
    }

    pub fn map_mut<R>(&self, map: impl FnOnce(&mut T) -> R) -> R {
        let mut state = self.state();
        state.wrapped.generation = state.wrapped.generation.wrapping_add(1);
        let result = map(&mut state.wrapped.value);
        drop(state);
        self.0.sync.notify_all();
        result
    }

    pub fn for_each<F>(&self, mut map: F)
    where
        F: for<'a> FnMut(&'a T) + Send + 'static,
    {
        self.0.for_each(move |gen| map(&gen.value));
    }

    pub fn map_each<R, F>(&self, mut map: F) -> Dynamic<R>
    where
        F: for<'a> FnMut(&'a T) -> R + Send + 'static,
        R: Send + 'static,
    {
        self.0.map_each(move |gen| map(&gen.value))
    }

    pub fn with_clone<R>(&self, with_clone: impl FnOnce(Self) -> R) -> R {
        with_clone(self.clone())
    }

    pub fn redraw_when_changed(&self, window: WindowHandle<WindowCommand>) {
        self.0.redraw_when_changed(window);
    }

    #[must_use]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.0.get().value
    }

    #[must_use]
    pub fn replace(&self, new_value: T) -> T {
        self.0.replace(new_value).value
    }

    pub fn set(&self, new_value: T) {
        let _old = self.replace(new_value);
    }

    #[must_use]
    pub fn create_ref_reader(&self) -> DynamicRefReader<T> {
        self.state().readers += 1;
        DynamicRefReader {
            source: self.0.clone(),
            read_generation: self.0.state().wrapped.generation,
        }
    }

    fn state(&self) -> MutexGuard<'_, State<T>> {
        self.0.state()
    }

    #[must_use]
    pub fn generation(&self) -> usize {
        self.state().wrapped.generation
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

impl<T> From<Dynamic<T>> for DynamicRefReader<T> {
    fn from(value: Dynamic<T>) -> Self {
        value.create_ref_reader()
    }
}

#[derive(Debug)]
struct DynamicData<T> {
    state: Mutex<State<T>>,
    sync: Condvar,
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
    pub fn replace(&self, new_value: T) -> GenerationalValue<T> {
        let mut state = self.state();
        let old = {
            let state = &mut *state;
            let generation = state.wrapped.generation.wrapping_add(1);
            let old = std::mem::replace(
                &mut state.wrapped,
                GenerationalValue {
                    value: new_value,
                    generation,
                },
            );

            for callback in &mut state.callbacks {
                callback.update(&state.wrapped);
            }
            for window in state.windows.drain(..) {
                let _result = window.send(WindowCommand::Redraw);
            }
            old
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
pub struct GenerationalValue<T> {
    pub value: T,
    pub generation: usize,
}

#[derive(Debug)]
pub struct DynamicRefReader<T> {
    source: Arc<DynamicData<T>>,
    read_generation: usize,
}

impl<T> DynamicRefReader<T> {
    pub fn map_ref<R>(&mut self, map: impl FnOnce(&T) -> R) -> R {
        let state = self.source.state();
        self.read_generation = state.wrapped.generation;
        map(&state.wrapped.value)
    }

    #[must_use]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.source.get().value
    }

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

    pub fn redraw_if_changed(&mut self, window: WindowHandle<WindowCommand>) {
        self.source.redraw_when_changed(window);
    }
}

impl<T> Clone for DynamicRefReader<T> {
    fn clone(&self) -> Self {
        self.source.state().readers += 1;
        Self {
            source: self.source.clone(),
            read_generation: self.read_generation,
        }
    }
}

impl<T> Drop for DynamicRefReader<T> {
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
