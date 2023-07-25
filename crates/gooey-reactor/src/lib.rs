//! A platform-independent, `#![forbid(unsafe_code)]` reactive event system.
#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use std::any::Any;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::mem;
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::task::Waker;

use alot::{LotId, Lots};

#[cfg(feature = "async")]
mod stream;
#[cfg(feature = "async")]
pub use stream::ValueStream;

/// A handle to a running reactor.
///
/// Reactors contain a set of hierarchical
/// [`Scope`]s.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Reactor(LotId);

impl Default for Reactor {
    fn default() -> Self {
        let mut reactors = all_reactors();
        let reactor_id = reactors.push(ReactorData::default());
        Reactor(reactor_id)
    }
}

impl Reactor {
    /// Creates a new scope within the reactor.
    #[must_use]
    pub fn new_scope(&self) -> ScopeGuard {
        let mut reactors = all_reactors();
        let reactor = &mut reactors[self.0];

        ScopeGuard(Scope {
            id: ScopeId(reactor.scopes.push(ScopeData::default())),
            reactor: *self,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct ValueId(LotId);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct ScopeId(LotId);

/// A node within a [`Reactor`].
///
/// Each scope may contain:
///
/// - Child scopes.
/// - [`Dynamic`] values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Scope {
    id: ScopeId,
    reactor: Reactor,
}

impl Scope {
    /// Returns the reactor that this scope belongs to.
    #[must_use]
    pub const fn reactor(&self) -> Reactor {
        self.reactor
    }

    /// Creates a new scope within this scope.
    #[must_use]
    pub fn new_scope(&self) -> ScopeGuard {
        let mut reactors = all_reactors();
        let reactor = &mut reactors[self.reactor.0];
        let id = ScopeId(reactor.scopes.push(ScopeData {
            parent: Some(self.id),
            values: Lots::default(),
            children: HashSet::new(),
        }));
        reactor.scopes[self.id.0].children.insert(id);

        ScopeGuard(Scope {
            id,
            reactor: self.reactor,
        })
    }

    /// Creates a new dynamic value containing `initial_value`.
    #[must_use]
    pub fn new_dynamic<T>(&self, initial_value: T) -> Dynamic<T>
    where
        T: Any + Send + Sync + 'static,
    {
        let mut reactors = all_reactors();
        let reactor = &mut reactors[self.reactor.0];
        let scope = &mut reactor.scopes[self.id.0];
        let value = scope
            .values
            .push(Box::new(Arc::new(Mutex::new(ValueData::new(
                initial_value,
            )))));
        Dynamic {
            id: ValueRef {
                scope: *self,
                value: ValueId(value),
            },
            _phantom: PhantomData,
        }
    }

    /// Creates a new dynamic value initialized with `T::default()`.
    #[must_use]
    pub fn new_default_dynamic<T>(&self) -> Dynamic<T>
    where
        T: Default + Any + Send + Sync + 'static,
    {
        self.new_dynamic(T::default())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct ValueRef {
    scope: Scope,
    value: ValueId,
}

/// A handle to a dynamic value that can be updated and reacted to.
///
/// The `T` value is stored within the [`Scope`] that created the `Dynamic`. It
/// can be updated using one of these approaches:
///
/// - [`Dynamic::map_mut()`]
/// - [`Dynamic::set()`]
///
/// Changes to this type can be observed using one of these approaches:
///
/// - [`Dynamic::iter()`]
/// - [`Dynamic::for_each()`]
/// - [`Dynamic::map_each()`]
/// - [`Dynamic::into_stream()`]
#[derive(Debug, Eq, PartialEq)]
pub struct Dynamic<T>
where
    T: 'static,
{
    id: ValueRef,
    _phantom: PhantomData<&'static T>,
}

impl<T> Clone for Dynamic<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _phantom: PhantomData,
        }
    }
}

impl<T> Copy for Dynamic<T> {}

impl<T> Dynamic<T>
where
    T: 'static,
{
    /// Returns a clone of the currently contained value.
    ///
    /// `None` is returned if this value belonged to a scope that has been
    /// destructed.
    pub fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.map_ref(T::clone)
    }

    /// Calls `map` with the currently contained value.
    ///
    /// All other access to this value will be blocked while `map` executes.
    ///
    /// `None` is returned if this value belonged to a scope that has been
    /// destructed.
    pub fn map_ref<R>(&self, map: impl FnOnce(&T) -> R) -> Option<R> {
        let reactors = all_reactors();
        let value = reactors
            .get(self.id.scope.reactor.0)
            .and_then(|reactor| reactor.scopes.get(self.id.scope.id.0))
            .and_then(|scope| scope.values.get(self.id.value.0))
            .and_then(|value| {
                value
                    .as_ref()
                    .as_any()
                    .downcast_ref::<Arc<Mutex<ValueData<T>>>>()
                    .cloned()
            })?;
        drop(reactors);
        let value = value.lock().map_or_else(PoisonError::into_inner, |a| a);
        Some(map(&value.value))
    }

    /// Calls `map` with the currently contained value.
    ///
    /// All other access to this value will be blocked while `map` executes.
    ///
    /// `None` is returned if this value belonged to a scope that has been
    /// destructed.
    pub fn map_mut<R>(&self, map: impl FnOnce(&mut T) -> R) -> Option<R> {
        let reactors = all_reactors();
        let value = reactors
            .get(self.id.scope.reactor.0)
            .and_then(|reactor| reactor.scopes.get(self.id.scope.id.0))
            .and_then(|scope| scope.values.get(self.id.value.0))
            .and_then(|value| {
                value
                    .as_ref()
                    .as_any()
                    .downcast_ref::<Arc<Mutex<ValueData<T>>>>()
                    .cloned()
            })?;
        drop(reactors);
        let mut value = value.lock().map_or_else(PoisonError::into_inner, |a| a);
        let mapped = map(&mut value.value);
        value.notify_changed();
        Some(mapped)
    }

    /// Replaces the currently contained value with `new_value`, returning the
    /// existing value.
    ///
    /// `None` is returned if this value belonged to a scope that has been
    /// destructed.
    pub fn set(&self, new_value: T) -> Option<T> {
        let reactors = all_reactors();
        if let Some(reactor) = reactors.get(self.id.scope.reactor.0) {
            if let Some(data) = reactor
                .scopes
                .get(self.id.scope.id.0)
                .and_then(|scope| scope.values.get(self.id.value.0))
                .and_then(|value| {
                    value
                        .as_ref()
                        .as_any()
                        .downcast_ref::<Arc<Mutex<ValueData<T>>>>()
                        .cloned()
                })
            {
                drop(reactors);
                let mut data = data.lock().map_or_else(PoisonError::into_inner, |a| a);
                let old_value = mem::replace(&mut data.value, new_value);
                data.notify_changed();
                return Some(old_value);
            }
        }

        None
    }

    /// Returns an iterator to values contained in this dynamic.
    #[must_use]
    #[allow(clippy::iter_not_returning_iterator)] // It does, but only for Clone types.
    pub fn iter(&self) -> ValueIterator<T> {
        ValueIterator {
            value: *self,
            condvar: None,
            // Specifying a read generation of 1 prevents reading the initial value.
            read_generation: Some(NonZeroUsize::new(1).expect("not zero")),
        }
    }

    /// Returns an async [`futures_core::Stream`] returning values contained in
    /// this dynamic.
    #[cfg(feature = "async")]
    #[must_use]
    pub fn into_stream(self) -> ValueStream<T> {
        ValueStream {
            value: self,
            waker: None,
            // Specifying a read generation of 1 prevents reading the initial value.
            read_generation: Some(NonZeroUsize::new(1).expect("not zero")),
        }
    }

    /// Returns a new `Dynamic` that is updated automatically when this
    /// `Dynamic` is changed.
    ///
    /// Each time a value is updated in this `Dynamic`, `map` will be invoked
    /// and the result will be stored in the returned `Dynamic`.
    ///
    /// `None` is returned if this value belonged to a scope that has been
    /// destructed.
    #[must_use]
    pub fn map_each<R, F>(&self, mut map: F) -> Option<Dynamic<R>>
    where
        F: for<'a> FnMut(&'a T) -> R + Send + 'static,
        R: Send + Sync,
    {
        let initial_value = self.map_ref(&mut map)?;
        let mapped = self.id.scope.new_dynamic(initial_value);
        let reactors = all_reactors();
        let reactor = reactors.get(self.id.scope.reactor.0)?;
        let scope = reactor.scopes.get(self.id.scope.id.0)?;
        let value = scope.values.get(self.id.value.0)?;
        let data = value
            .as_ref()
            .as_any()
            .downcast_ref::<Arc<Mutex<ValueData<T>>>>()?;
        let mut data = data.lock().map_or_else(PoisonError::into_inner, |a| a);
        data.callbacks.push(Box::new(ValueMapping {
            map: Box::new(map),
            result: mapped,
        }));
        Some(mapped)
    }

    /// Attaches `for_each` to this dynamic such that it is executed each time
    /// this dynamic changes.
    pub fn for_each<F>(&self, mut for_each: F)
    where
        F: for<'a> FnMut(&'a T) + Send + 'static,
    {
        self.map_ref(&mut for_each);
        let mut reactors = all_reactors();
        let Some(data) = reactors
            .get_mut(self.id.scope.reactor.0)
            .and_then(|reactor| reactor.scopes.get_mut(self.id.scope.id.0))
            .and_then(|scope| scope.values.get(self.id.value.0))
            .and_then(|value|
                value
                .as_ref()
                .as_any()
                .downcast_ref::<Arc<Mutex<ValueData<T>>>>()) else { return };

        let mut data = data.lock().map_or_else(PoisonError::into_inner, |a| a);
        data.callbacks.push(Box::new(ValueForEach {
            map: Box::new(for_each),
        }));
    }

    /// Returns the current *generation* of this dynamic.
    ///
    /// A *generation* is a counter that is updated each time the value is
    /// accessed mutably.
    pub fn generation(&self) -> Option<NonZeroUsize> {
        let reactors = all_reactors();
        if let Some(data) = reactors
            .get(self.id.scope.reactor.0)
            .and_then(|reactor| reactor.scopes.get(self.id.scope.id.0))
            .and_then(|scope| scope.values.get(self.id.value.0))
            .and_then(|value| {
                value
                    .as_ref()
                    .as_any()
                    .downcast_ref::<Arc<Mutex<ValueData<T>>>>()
            })
        {
            let data = data.lock().map_or_else(PoisonError::into_inner, |a| a);
            Some(data.generation)
        } else {
            None
        }
    }
}

struct ValueMapping<V, R>
where
    R: 'static,
{
    map: Box<dyn MapFunction<V, R>>,
    result: Dynamic<R>,
}

struct ValueForEach<V> {
    map: Box<dyn MapFunction<V, ()>>,
}

trait MapFunction<T, R>: Send {
    fn map(&mut self, value: &T) -> R;
}

impl<T, R, F> MapFunction<T, R> for F
where
    F: for<'a> FnMut(&'a T) -> R + Send,
{
    fn map(&mut self, value: &T) -> R {
        self(value)
    }
}

trait AnyCallback<T>: Send {
    fn invoke(&mut self, value: &T);
}

impl<T, R> AnyCallback<T> for ValueMapping<T, R>
where
    R: Send + Sync,
{
    fn invoke(&mut self, value: &T) {
        let mapped = self.map.map(value);
        self.result.set(mapped);
    }
}

impl<T> AnyCallback<T> for ValueForEach<T> {
    fn invoke(&mut self, value: &T) {
        self.map.map(value);
    }
}

impl<T> IntoIterator for Dynamic<T>
where
    T: Clone + 'static,
{
    type IntoIter = ValueIterator<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A blocking iterator over values contained in a [`Dynamic`].
///
/// This type keeps track of the [generation](Dynamic::generation) of values it
/// reads from a [`Dynamic`] and provides methods for blocking until new values
/// are observed.
pub struct ValueIterator<T>
where
    T: 'static,
{
    value: Dynamic<T>,
    condvar: Option<Arc<Condvar>>,
    read_generation: Option<NonZeroUsize>,
}

impl<T> ValueIterator<T>
where
    T: 'static,
{
    /// Returns a clone of the currently contained value.
    ///
    /// Returns `None` if the [`Dynamic`] has been destroyed.
    pub fn get(&mut self) -> Option<T>
    where
        T: Clone,
    {
        let reactors = all_reactors();
        if let Some(data) = reactors
            .get(self.value.id.scope.reactor.0)
            .and_then(|reactor| reactor.scopes.get(self.value.id.scope.id.0))
            .and_then(|scope| scope.values.get(self.value.id.value.0))
            .and_then(|value| {
                value
                    .as_ref()
                    .as_any()
                    .downcast_ref::<Arc<Mutex<ValueData<T>>>>()
            })
        {
            let data = data.lock().map_or_else(PoisonError::into_inner, |a| a);
            self.read_generation = Some(data.generation);
            Some(data.value.clone())
        } else {
            None
        }
    }

    /// Waits until the [`Dynamic`] has had its value updated.
    ///
    /// Returns false if the [`Dynamic`] has been destroyed.
    pub fn wait_next(&mut self) -> bool {
        self.block_until_next_value(|_| {}).is_some()
    }

    fn block_until_next_value<R>(&mut self, map: impl FnOnce(&T) -> R) -> Option<R> {
        let mut reactors = all_reactors();
        loop {
            if let Some(reactor) = reactors.get_mut(self.value.id.scope.reactor.0) {
                if let Some(data) = reactor
                    .scopes
                    .get_mut(self.value.id.scope.id.0)
                    .and_then(|scope| scope.values.get_mut(self.value.id.value.0))
                    .and_then(|value| {
                        value
                            .as_mut()
                            .as_mut_any()
                            .downcast_mut::<Arc<Mutex<ValueData<T>>>>()
                            .cloned()
                    })
                {
                    let mut data = data.lock().map_or_else(PoisonError::into_inner, |a| a);

                    if self.read_generation != Some(data.generation) {
                        self.read_generation = Some(data.generation);
                        drop(reactors);

                        return Some(map(&data.value));
                    }

                    if self.condvar.is_none() {
                        self.condvar = Some(data.waiters.condvar().clone());
                    }

                    reactors = self
                        .condvar
                        .as_ref()
                        .expect("always initialized above")
                        .wait(reactors)
                        .map_or_else(PoisonError::into_inner, |g| g);
                    continue;
                }
            }

            break None;
        }
    }
}

impl<T> Iterator for ValueIterator<T>
where
    T: Clone + 'static,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.block_until_next_value(T::clone)
    }
}

#[derive(Default)]
struct ReactorData {
    scopes: Lots<ScopeData>,
}

static REACTORS: OnceLock<Mutex<Lots<ReactorData>>> = OnceLock::new();

fn all_reactors() -> MutexGuard<'static, Lots<ReactorData>> {
    REACTORS
        .get_or_init(Mutex::default)
        .lock()
        .map_or_else(PoisonError::into_inner, |g| g)
}

impl ReactorData {}

#[derive(Default)]
struct ScopeData {
    parent: Option<ScopeId>,
    values: Lots<Box<dyn AnyValue>>,
    children: HashSet<ScopeId>,
}

trait AnyValue: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
}

struct ValueData<T> {
    value: T,
    generation: NonZeroUsize,
    waiters: Waiters,
    callbacks: Vec<Box<dyn AnyCallback<T>>>,
}

impl<T> ValueData<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            generation: NonZeroUsize::new(1).expect("not zero"),
            waiters: Waiters::default(),
            callbacks: Vec::new(),
        }
    }

    pub fn notify_changed(&mut self) {
        self.generation = self
            .generation
            .checked_add(1)
            .unwrap_or_else(|| NonZeroUsize::new(1).expect("not zero"));

        self.waiters.notify();
        for cb in &mut self.callbacks {
            cb.invoke(&self.value);
        }
    }
}

impl<T> AnyValue for Arc<Mutex<ValueData<T>>>
where
    T: Any + Send + Sync + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Default)]
struct Waiters {
    condvar: Option<Arc<Condvar>>,
    wakers: Lots<Waker>,
}

impl Waiters {
    pub fn condvar(&mut self) -> &Arc<Condvar> {
        if self.condvar.is_none() {
            self.condvar = Some(Arc::new(Condvar::new()));
        }
        self.condvar.as_ref().expect("always initialized above")
    }

    pub fn notify(&mut self) {
        if let Some(condvar) = &self.condvar {
            condvar.notify_all();
        }

        for waker in self.wakers.drain() {
            waker.wake();
        }
    }
}

/// A handle that guards a [`Scope`]. When dropped, the [`Scope`] is freed.
#[derive(Debug)]
pub struct ScopeGuard(Scope);

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        let mut reactors = all_reactors();
        let reactor = &mut reactors[self.0.reactor.0];
        let Some(removed) = reactor.scopes.remove(self.0.id.0)
            else { unreachable!("scope already disposed") };

        // Detach any children scopes
        for child in removed.children {
            reactor.scopes[child.0].parent = None;
        }

        // Remove this from the parent.
        if let Some(parent) = removed.parent {
            assert!(
                reactor.scopes[parent.0].children.remove(&self.0.id),
                "child was not present in parent"
            );
        }
        // Stop holding the lock on the reactor before dropping the removed
        // child.
        drop(reactors);
    }
}

impl Deref for ScopeGuard {
    type Target = Scope;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a ScopeGuard> for Scope {
    fn from(guard: &'a ScopeGuard) -> Self {
        guard.0
    }
}

impl ScopeGuard {
    /// Returns the scope of this guard.
    #[must_use]
    pub const fn scope(&self) -> Scope {
        self.0
    }
}

#[test]
fn map_each_test() {
    let reactor = Reactor::default();
    let scope = reactor.new_scope();
    let first_value = scope.new_dynamic(1);
    let mapped_value = first_value.map_each(|updated| updated * 2).unwrap();
    assert_eq!(mapped_value.get(), Some(2));
    first_value.set(2);
    assert_eq!(mapped_value.get(), Some(4));
}
