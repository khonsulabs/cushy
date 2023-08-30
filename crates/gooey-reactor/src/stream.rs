use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::Poll;

use alot::LotId;
use futures_core::{Future, Stream};

use crate::{all_reactors, Dynamic, ValueData};

/// An async-compatble [`Stream`] of values contained in a [`Dynamic`].
///
/// This type keeps track of the [generation](Dynamic::generation) of values it
/// reads from a [`Dynamic`] and provides methods for `await`ing new values.
pub struct ValueStream<T>
where
    T: 'static,
{
    pub(crate) value: Dynamic<T>,
    pub(crate) waker: Option<LotId>,
    pub(crate) read_generation: Option<NonZeroUsize>,
}

impl<T> ValueStream<T>
where
    T: 'static,
{
    /// Returns a future that waits for the contents of the [`Dynamic`] to be
    /// updated.
    ///
    /// The future will return false if the [`Dynamic`] has been destroyed.
    pub fn wait_next(&mut self) -> WaitNext<'_, T> {
        WaitNext(self)
    }
}

impl<T> std::ops::Deref for ValueStream<T> {
    type Target = Dynamic<T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> std::ops::DerefMut for ValueStream<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

pub struct WaitNext<'a, T>(&'a mut ValueStream<T>)
where
    T: 'static;

impl<'a, T> Future for WaitNext<'a, T> {
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if let Some(runtime) = all_reactors().get_mut(self.0.value.id.scope.reactor.0) {
            if let Some(data) = runtime
                .scopes
                .get_mut(self.0.value.id.scope.id.0)
                .and_then(|scope| scope.values.get_mut(self.0.value.id.value.0))
                .and_then(|value| {
                    value
                        .as_mut()
                        .as_mut_any()
                        .downcast_mut::<Arc<Mutex<ValueData<T>>>>()
                })
            {
                let mut data = data.lock().map_or_else(PoisonError::into_inner, |a| a);
                if self.0.read_generation != Some(data.generation) {
                    self.0.read_generation = Some(data.generation);
                    return Poll::Ready(true);
                }

                match self.0.waker {
                    Some(existing_slot) => {
                        // Only update the waker if the stored one doesn't
                        // point to the same task.
                        match data.waiters.wakers.get_mut(existing_slot) {
                            Some(waker) if waker.will_wake(cx.waker()) => {}
                            Some(waker) => {
                                *waker = cx.waker().clone();
                            }
                            None => {
                                data.waiters.wakers.push(cx.waker().clone());
                            }
                        }
                    }
                    None => {
                        self.0.waker = Some(data.waiters.wakers.push(cx.waker().clone()));
                    }
                }
                return Poll::Pending;
            }
        }
        Poll::Ready(false)
    }
}

impl<T> Stream for ValueStream<T>
where
    T: Clone + 'static,
{
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut reactors = all_reactors();
        if let Some(runtime) = reactors.get_mut(self.value.id.scope.reactor.0) {
            if let Some(data) = runtime
                .scopes
                .get_mut(self.value.id.scope.id.0)
                .and_then(|scope| scope.values.get_mut(self.value.id.value.0))
                .and_then(|value| {
                    value
                        .as_mut()
                        .as_mut_any()
                        .downcast_mut::<Arc<Mutex<ValueData<T>>>>()
                })
            {
                let mut data = data.lock().map_or_else(PoisonError::into_inner, |a| a);
                if self.read_generation != Some(data.generation) {
                    self.read_generation = Some(data.generation);
                    return Poll::Ready(Some(data.value.clone()));
                }

                match self.waker {
                    Some(existing_slot) => {
                        // Only update the waker if the stored one doesn't
                        // point to the same task.
                        match data.waiters.wakers.get_mut(existing_slot) {
                            Some(waker) if waker.will_wake(cx.waker()) => {}
                            Some(waker) => {
                                *waker = cx.waker().clone();
                            }
                            None => {
                                data.waiters.wakers.push(cx.waker().clone());
                            }
                        }
                    }
                    None => {
                        self.waker = Some(data.waiters.wakers.push(cx.waker().clone()));
                    }
                }
                return Poll::Pending;
            }
        }
        Poll::Ready(None)
    }
}
