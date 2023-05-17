use std::any::Any;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::mem;
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::task::Waker;

use alot::{LotId, Lots};
use once_cell::sync::OnceCell;

#[cfg(feature = "async")]
mod stream;
#[cfg(feature = "async")]
pub use stream::ValueStream;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Scope {
    id: ScopeId,
    reactor: Reactor,
}

impl Scope {
    pub const fn reactor(&self) -> &Reactor {
        &self.reactor
    }

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

    pub fn new_value<T>(&self, initial_value: T) -> Value<T>
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
        Value {
            id: ValueRef {
                scope: *self,
                value: ValueId(value),
            },
            _phantom: PhantomData,
        }
    }

    pub fn new_default_value<T>(&self) -> Value<T>
    where
        T: Default + Any + Send + Sync + 'static,
    {
        self.new_value(T::default())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct ValueRef {
    scope: Scope,
    value: ValueId,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Value<T>
where
    T: 'static,
{
    id: ValueRef,
    _phantom: PhantomData<&'static T>,
}

impl<T> Clone for Value<T>
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

impl<T> Copy for Value<T> {}

impl<T> Value<T>
where
    T: 'static,
{
    pub fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.map_ref(T::clone)
    }

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
        value.notify_changed();
        Some(map(&mut value.value))
    }

    pub fn replace(&self, new_value: T) -> Option<T> {
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

    pub fn iter(&self) -> ValueIterator<T> {
        ValueIterator {
            value: *self,
            condvar: None,
            // Specifying a read generation of 1 prevents reading the initial value.
            read_generation: Some(NonZeroUsize::new(1).expect("not zero")),
        }
    }

    #[cfg(feature = "async")]
    pub fn into_stream(self) -> ValueStream<T> {
        ValueStream {
            value: self,
            waker: None,
            // Specifying a read generation of 1 prevents reading the initial value.
            read_generation: Some(NonZeroUsize::new(1).expect("not zero")),
        }
    }
}

impl<T> IntoIterator for Value<T>
where
    T: Clone + 'static,
{
    type IntoIter = ValueIterator<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct ValueIterator<T>
where
    T: 'static,
{
    value: Value<T>,
    condvar: Option<Arc<Condvar>>,
    read_generation: Option<NonZeroUsize>,
}

impl<T> ValueIterator<T>
where
    T: Clone + 'static,
{
    pub fn get(&mut self) -> Option<T> {
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
                    } else {
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

static REACTORS: OnceCell<Mutex<Lots<ReactorData>>> = OnceCell::new();

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
}

impl<T> ValueData<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            generation: NonZeroUsize::new(1).expect("not zero"),
            waiters: Waiters::default(),
        }
    }

    pub fn notify_changed(&mut self) {
        self.generation = self
            .generation
            .checked_add(1)
            .unwrap_or_else(|| NonZeroUsize::new(1).expect("not zero"));

        self.waiters.notify();
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

#[derive(Debug)]
pub struct ScopeGuard(Scope);

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        let reactor = &mut all_reactors()[self.0.reactor.0];
        let Some(removed) = reactor.scopes.remove(self.0.id.0)
            else { unreachable!("scope already disposed") };
        assert!(
            removed.children.is_empty(),
            "ScopeGuard dropped while children guards are still active"
        );
        if let Some(parent) = removed.parent {
            assert!(
                reactor.scopes[parent.0].children.remove(&self.0.id),
                "child was not present in parent"
            );
        }
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
    pub const fn scope(&self) -> Scope {
        self.0
    }
}
