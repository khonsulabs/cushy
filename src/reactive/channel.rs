//! A reactive multi-sender, single-consumer (mpsc) channel for Cushy.

use std::collections::VecDeque;
use std::fmt::{self, Debug};
use std::future::Future;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use parking_lot::{Condvar, Mutex, MutexGuard};

use crate::reactive::{enqueue_task, BackgroundTask, ChannelTask};
use crate::value::CallbackDisconnected;

/// An error occurred while trying to send a value.
pub enum TrySendError<T> {
    /// The recipient was full.
    Full(T),
    /// The recipient is no longer reachable.
    Disconnected(T),
}

/// A future that sends a message to a [`Channel<T>`].
#[must_use = "Futures must be awaited to be executed"]
pub struct SendAsync<'a, T> {
    value: Option<T>,
    channel: &'a Channel<T>,
}

impl<T> Future for SendAsync<'_, T>
where
    T: Unpin + Clone + Send + 'static,
{
    type Output = Option<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(value) = self.value.take() else {
            return Poll::Ready(None);
        };
        match self.channel.try_send_inner(value, |channel| {
            let will_wake = channel
                .wakers
                .iter()
                .any(|waker| waker.will_wake(cx.waker()));
            if !will_wake {
                channel.wakers.push(cx.waker().clone());
            }
            ControlFlow::Break(())
        }) {
            Ok(()) => Poll::Ready(None),
            Err(TrySendError::Disconnected(value)) => Poll::Ready(Some(value)),
            Err(TrySendError::Full(value)) => {
                self.value = Some(value);
                Poll::Pending
            }
        }
    }
}

pub(super) trait AnyChannel: Send + Sync + 'static {
    fn should_poll(&self) -> bool;
    fn poll(&self, futures: &mut Vec<ChannelCallbackFuture>) -> bool;
    fn disconnect_callback(&self);
}

impl<T> AnyChannel for ChannelData<T>
where
    T: Send + 'static,
{
    fn should_poll(&self) -> bool {
        let channel = self.synced.lock();
        !channel.queue.is_empty() && channel.callback.is_some()
    }

    fn poll(&self, futures: &mut Vec<ChannelCallbackFuture>) -> bool {
        let mut channel = self.synced.lock();

        let Some(value) = channel.queue.pop_front() else {
            return channel.instances > 0 && channel.callback.is_some();
        };

        self.condvar.notify_all();
        for waker in channel.wakers.drain(..) {
            waker.wake();
        }

        if let Some(callback) = &mut channel.callback {
            match callback.invoke(value) {
                Ok(()) => {}
                Err(ChannelCallbackError::Async(future)) => {
                    futures.push(ChannelCallbackFuture {
                        future: Pin::from(future),
                    });
                }
                Err(ChannelCallbackError::Disconnected) => {
                    channel.callback = None;
                    return false;
                }
            }
        }

        true
    }

    fn disconnect_callback(&self) {
        self.synced.lock().callback = None;
    }
}

pub(super) struct ChannelCallbackFuture {
    pub(super) future: Pin<Box<dyn Future<Output = Result<(), CallbackDisconnected>>>>,
}

/// A reactive multi-sender, single-consumer (mpsc) channel that executes code
/// in the background when values are received.
///
/// A [`Dynamic<T>`](crate::value::Dynamic) is a container for a `T` that can be
/// reacted against. Due to this design, it is possible to not observe *every*
/// value that passes through the container. For some use cases (such as the
/// [Command Pattern][command]), it is important that every value is observed.
///
/// This type ensures the associated code is executed for each value sent.
/// Additionally, unlike other channel types, this code is scheduled to be
/// executed by Cushy automatically instead of requiring additional threads or
/// an external async runtime.
///
/// [command]: https://en.wikipedia.org/wiki/Command_pattern
#[derive(Debug)]
pub struct Channel<T> {
    data: Arc<ChannelData<T>>,
}

impl<T> Channel<T>
where
    T: Send + 'static,
{
    /// Returns a channel that executes `on_receive` for each value sent.
    ///
    /// The returned channel will never be considered full and will panic if a
    /// large enough queue cannot be allocated.
    #[must_use]
    pub fn unbounded<F>(mut on_receive: F) -> Self
    where
        F: FnMut(T) + Send + 'static,
    {
        Self::new(
            None,
            Box::new(move |value| {
                on_receive(value);
                Ok(())
            }),
        )
    }

    /// Returns a channel that executes `on_receive` for each value sent. The
    /// channel will be disconnected if the callback returns
    /// `Err(CallbackDisconnected)`.
    ///
    /// The returned channel will never be considered full and will panic if a
    /// large enough queue cannot be allocated.
    #[must_use]
    pub fn unbounded_try<F>(mut on_receive: F) -> Self
    where
        F: FnMut(T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        Self::new(
            None,
            Box::new(move |value| {
                on_receive(value).map_err(|_| ChannelCallbackError::Disconnected)
            }),
        )
    }

    /// Returns a channel that executes the future returned from `on_receive`
    /// for each value sent.
    ///
    /// The returned channel will never be considered full and will panic if a
    /// large enough queue cannot be allocated.
    #[must_use]
    pub fn unbounded_async<F, Fut>(mut on_receive: F) -> Self
    where
        F: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self::new(
            None,
            Box::new(move |value| {
                let future = on_receive(value);
                Err(ChannelCallbackError::Async(Box::new(async move {
                    future.await;
                    Ok(())
                })))
            }),
        )
    }

    /// Returns a channel that executes the future returned from `on_receive`
    /// for each value sent. The channel will be disconnected if the callback
    /// returns `Err(CallbackDisconnected)`.
    ///
    /// The returned channel will never be considered full and will panic if a
    /// large enough queue cannot be allocated.
    #[must_use]
    pub fn unbounded_async_try<F, Fut>(mut on_receive: F) -> Self
    where
        F: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), CallbackDisconnected>> + Send + 'static,
    {
        Self::new(
            None,
            Box::new(move |value| Err(ChannelCallbackError::Async(Box::new(on_receive(value))))),
        )
    }

    /// Returns a bounded channel that executes `on_receive` for each value
    /// sent.
    ///
    /// The returned channel will only allow `capacity` values to be queued at
    /// any moment in time. Each `send` function documents what happens when the
    /// channel is full.
    #[must_use]
    pub fn bounded<F>(capacity: usize, mut on_receive: F) -> Self
    where
        F: FnMut(T) + Send + 'static,
    {
        Self::new(
            Some(capacity),
            Box::new(move |value| {
                on_receive(value);
                Ok(())
            }),
        )
    }

    /// Returns a bounded channel that executes `on_receive` for each value
    /// sent. The channel will be disconnected if the callback returns
    /// `Err(CallbackDisconnected)`.
    ///
    /// The returned channel will only allow `capacity` values to be queued at
    /// any moment in time. Each `send` function documents what happens when the
    /// channel is full.
    #[must_use]
    pub fn bounded_try<F>(capacity: usize, mut on_receive: F) -> Self
    where
        F: FnMut(T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        Self::new(
            Some(capacity),
            Box::new(move |value| {
                on_receive(value).map_err(|_| ChannelCallbackError::Disconnected)
            }),
        )
    }

    /// Returns a bounded channel that executes the future returned from
    /// `on_receive` for each value sent.
    ///
    /// The returned channel will only allow `capacity` values to be queued at
    /// any moment in time. Each `send` function documents what happens when the
    /// channel is full.
    #[must_use]
    pub fn bounded_async<F, Fut>(capacity: usize, mut on_receive: F) -> Self
    where
        F: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self::new(
            Some(capacity),
            Box::new(move |value| {
                let future = on_receive(value);
                Err(ChannelCallbackError::Async(Box::new(async move {
                    future.await;
                    Ok(())
                })))
            }),
        )
    }

    /// Returns a bounded channel that executes the future returned from `on_receive`
    /// for each value sent. The channel will be disconnected if the callback
    /// returns `Err(CallbackDisconnected)`.
    ///
    /// The returned channel will only allow `capacity` values to be queued at
    /// any moment in time. Each `send` function documents what happens when the
    /// channel is full.
    #[must_use]
    pub fn bounded_async_try<F, Fut>(capacity: usize, mut on_receive: F) -> Self
    where
        F: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), CallbackDisconnected>> + Send + 'static,
    {
        Self::new(
            Some(capacity),
            Box::new(move |value| Err(ChannelCallbackError::Async(Box::new(on_receive(value))))),
        )
    }

    fn new(limit: Option<usize>, callback: Box<dyn AnyChannelCallback<T>>) -> Self {
        let (queue, limit) = match limit {
            Some(limit) => (VecDeque::with_capacity(limit), limit),
            None => (VecDeque::new(), usize::MAX),
        };
        let this = Self {
            data: Arc::new(ChannelData {
                condvar: Condvar::new(),
                synced: Mutex::new(SyncedChannelData {
                    queue,
                    limit,
                    instances: 1,
                    wakers: Vec::new(),
                    callback: Some(callback),
                }),
            }),
        };
        enqueue_task(BackgroundTask::Channel(ChannelTask::Register {
            id: this.id(),
            data: this.data.clone(),
        }));
        this
    }

    /// Sends `value` to this channel.
    ///
    /// Returns `Some(value)` if the channel is disconnected.
    ///
    /// If the channel is full, this function will block the current thread
    /// until space is made available. If another Channel's `on_receive` is
    /// sending a value to a bounded channel, that `on_receive` should be async
    /// and use [`send_async()`](Self::send_async) instead.
    #[allow(clippy::must_use_candidate)]
    pub fn send(&self, value: T) -> Option<T> {
        match self.try_send_inner(value, |channel| {
            self.data.condvar.wait(channel);
            ControlFlow::Continue(())
        }) {
            Ok(()) => None,
            Err(TrySendError::Disconnected(value) | TrySendError::Full(value)) => Some(value),
        }
    }

    /// Sends `value` to this channel asynchronously.
    ///
    /// The future returns `Some(value)` if the channel is disconnected.
    ///
    /// If the channel is full, this future will wait until space is made
    /// available before sending.
    pub fn send_async(&self, value: T) -> SendAsync<'_, T> {
        SendAsync {
            value: Some(value),
            channel: self,
        }
    }

    /// Tries to send `value` to this channel. Returns an error if unable to
    /// send.
    ///
    /// # Errors
    ///
    /// - When the channel is disconnected, [`TrySendError::Disconnected`] will
    ///   be returned.
    /// - When the channel is full, [`TrySendError::Full`] will
    ///   be returned.
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        self.try_send_inner(value, |_| ControlFlow::Break(()))
    }

    fn try_send_inner(
        &self,
        value: T,
        mut when_full: impl FnMut(&mut MutexGuard<'_, SyncedChannelData<T>>) -> ControlFlow<()>,
    ) -> Result<(), TrySendError<T>> {
        let mut channel = self.data.synced.lock();
        while channel.callback.is_some() {
            if channel.queue.len() >= channel.limit {
                match when_full(&mut channel) {
                    ControlFlow::Continue(()) => continue,
                    ControlFlow::Break(()) => return Err(TrySendError::Full(value)),
                }
            }
            let should_notify = channel.queue.is_empty();
            channel.queue.push_back(value);
            drop(channel);

            if should_notify {
                enqueue_task(BackgroundTask::Channel(ChannelTask::Notify {
                    id: self.id(),
                }));
            }

            return Ok(());
        }
        Err(TrySendError::Disconnected(value))
    }
}

impl<T> Channel<T> {
    fn id(&self) -> usize {
        Arc::as_ptr(&self.data) as usize
    }
}

impl<T> Clone for Channel<T> {
    fn clone(&self) -> Self {
        let mut channel = self.data.synced.lock();
        channel.instances += 1;
        Self {
            data: self.data.clone(),
        }
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        let mut channel = self.data.synced.lock();
        channel.instances -= 1;

        if channel.instances == 0 {
            drop(channel);
            enqueue_task(BackgroundTask::Channel(ChannelTask::Unregister(self.id())));
        }
    }
}

#[derive(Debug)]
struct ChannelData<T> {
    condvar: Condvar,
    synced: Mutex<SyncedChannelData<T>>,
}

struct SyncedChannelData<T> {
    queue: VecDeque<T>,
    limit: usize,
    instances: usize,
    wakers: Vec<Waker>,

    callback: Option<Box<dyn AnyChannelCallback<T>>>,
}

impl<T> Debug for SyncedChannelData<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyncedChannelData")
            .field("queue", &self.queue)
            .field("limit", &self.limit)
            .field("instances", &self.instances)
            .field("wakers", &self.wakers)
            .field(
                "callback",
                &if self.callback.is_some() {
                    "(Connected)"
                } else {
                    "(Disconnected)"
                },
            )
            .finish()
    }
}

trait AnyChannelCallback<T>: Send + 'static {
    fn invoke(&mut self, value: T) -> Result<(), ChannelCallbackError>;
}

impl<F, T> AnyChannelCallback<T> for F
where
    F: FnMut(T) -> Result<(), ChannelCallbackError> + Send + 'static,
{
    fn invoke(&mut self, value: T) -> Result<(), ChannelCallbackError> {
        self(value)
    }
}

enum ChannelCallbackError {
    Async(Box<dyn Future<Output = Result<(), CallbackDisconnected>>>),
    Disconnected,
}

#[test]
fn channel_basics() {
    use crate::value::{Destination, Dynamic, Source};
    let result = Dynamic::new(0);
    let result_reader = result.create_reader();
    let channel = Channel::<usize>::unbounded(move |value| result.set(dbg!(value)));
    assert!(!result_reader.has_updated());
    channel.send(1);
    result_reader.block_until_updated();
    assert_eq!(result_reader.get(), 1);
}

#[test]
fn async_channels() {
    use crate::value::{Destination, Dynamic, Source};

    let result = Dynamic::new(0);
    let result_reader = result.create_reader();
    let channel2 = Channel::<usize>::unbounded_async(move |value| {
        let result = result.clone();
        async move {
            result.set(dbg!(value));
        }
    });
    let channel1 = Channel::<usize>::unbounded_async(move |value| {
        let channel2 = channel2.clone();
        async move {
            channel2.send(dbg!(value));
        }
    });
    assert!(!result_reader.has_updated());
    channel1.send(1);
    result_reader.block_until_updated();
    assert_eq!(result_reader.get(), 1);
    channel1.send(2);
    result_reader.block_until_updated();
    assert_eq!(result_reader.get(), 2);
}
