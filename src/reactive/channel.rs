//! Reactive channels for Cushy
//!
//! Channels ensure that every message sent is delivered to a receiver. Dynamics
//! contain values and can provide reactivity, but if a dynamic is updated more
//! quickly than the change callbacks are executed, it is possible for change
//! callbacks to not observe every value stored in the Dynamic. Channels allow
//! building data flows that ensure every value written is observed.
//!
//! Cushy supports two types of channels:
//!
//! - Multi-Producer, Single-Consumer (MPSC): One or more [`Sender<T>`]s send
//!   values to either a [`Receiver<T>`] or a callback function.
//!
//!   Created by:
//!
//!   - [`unbounded()`]
//!   - [`bounded()`]
//!   - [`build()`]`.finish()`/`build().bound(capacity).finish()`
//!
//! - Broadcast: A [`Broadcaster<T>`] or a [`BroadcastChannel<T>`] sends values
//!   to one or more callback functions. This type requires `T` to implement
//!   `Clone` as each callback receives its own clone of the value being
//!   broadcast.
//!
//!   Broadcast channels ensure every callback associated is completed for each
//!   value received before receiving the next value.
//!
//!   Created by:
//!   - [`BroadcastChannel::unbounded()`]
//!   - [`BroadcastChannel::bounded()`]
//!   - [`build()`]`broadcasting().finish()`/`build().bound(capacity).broadcasting().finish()`
//!
//! All channel types support being either unbounded or bounded. An unbounded
//! channel dynamically allocates its queue and grows as needed. It can cause
//! unexpected memory use or panics if the queues grow too large for the
//! available system memory. Bounded channels allocate a buffer of a known
//! capacity and can block on send or return errors when the queue is full.
//!
//! One of the features provided by Cushy's channels are the abilility to attach
//! callbacks to be executed when values are sent. Instead of needing to
//! manually spawn threads or async tasks, these callbacks are automatically
//! scheduled by Cushy, making channel reactivity feel similar to
//! [`Dynamic<T>`](crate::value::Dynamic) reactivity. However, channels
//! guarantee that the callbacks associated with them receive *every* value
//! written, while dynamics only guarantee that the latest written value will be
//! observed.
//!
//! # Blocking callbacks
//!
//! When a callback might block while waiting on another thread, a network task,
//! or some other operation that may take a long time or require synchronization
//! that could block (e.g., mutexes), it should be considered a *blocking*
//! callback. Each blocking callback is executed in a way that ensures it cannot
//! block any other operation while waiting for new values to be sent.
//!
//! These callbacks can be configured using:
//!
//! - [`Receiver::on_receive`]
//! - [`BroadcastChannel::on_receive`]
//! - [`Builder::on_receive`]
//!
//! # Non-blocking callbacks
//!
//! When a callback will never block for a significant amount of time or in a
//! way that depends on other threads or callbacks or external resources, a
//! non-blocking callback can be used. These callbacks are executed in a shared
//! execution environment that minimizes resource consumption compared to what
//! is required to execute blocking callbacks.
//!
//! These callbacks can be configured using:
//!
//! - [`Receiver::on_receive_nonblocking`]
//! - [`BroadcastChannel::on_receive_nonblocking`]
//! - [`Builder::on_receive_nonblocking`]
//!
//! # Async callbacks
//!
//! If a callback needs to `await` a future, an async callback can be used.
//! These callbacks are functions that take a value and return a future that can
//! be awaited to process the value. The future returned is awaited to
//! completion before the next value is received from the channel.
//!
//! These callbacks can be configured using:
//!
//! - [`Receiver::on_receive_async`]
//! - [`BroadcastChannel::on_receive_async`]
//! - [`Builder::on_receive_async`]
use std::collections::VecDeque;
use std::fmt::{self, Debug};
use std::future::Future;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{ready, Context, Poll, Waker};
use std::time::{Duration, Instant};

use builder::Builder;
use parking_lot::{Condvar, Mutex, MutexGuard};
use sealed::{AnyChannelCallback, AsyncCallbackFuture, ChannelCallbackError, ChannelCallbackKind};

use super::value::Dynamic;
use super::{
    enqueue_task, CallbackHandle, CallbackHandleInner, CallbackKind, IntoOption, Unwrapped,
};
use crate::reactive::{BackgroundTask, CallbackDisconnected, ChannelTask};

pub mod builder;

/// Returns multi-producer, single-consumer channel with no limit to the number
/// of values enqueued.
#[must_use]
pub fn unbounded<T>() -> (Sender<T>, Receiver<T>)
where
    T: Send + 'static,
{
    Builder::new().finish()
}

/// Returns multi-producer, single-consumer channel that limits queued values to
/// `capacity` items.
#[must_use]
pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>)
where
    T: Send + 'static,
{
    Builder::new().bounded(capacity).finish()
}

/// Returns a [`Builder`] for a Cushy channel.
pub fn build<T>() -> Builder<T> {
    Builder::default()
}

mod sealed {
    use std::future::Future;
    use std::pin::Pin;

    pub enum ChannelCallbackKind<T> {
        Blocking(Box<dyn FnMut(T) -> Result<(), super::CallbackDisconnected> + Send + 'static>),
        NonBlocking(Box<dyn AnyChannelCallback<T>>),
    }

    pub trait AnyChannelCallback<T>: Send + 'static {
        fn invoke(&mut self, value: T) -> Result<(), ChannelCallbackError>;
    }

    pub enum ChannelCallbackError {
        Async(AsyncCallbackFuture),
        Disconnected,
    }

    pub type AsyncCallbackFuture =
        Pin<Box<dyn Future<Output = Result<(), super::CallbackDisconnected>>>>;
}

/// An error occurred while trying to send a value to a channel.
pub enum TrySendError<T> {
    /// The channel was full.
    Full(T),
    /// The channel no longer has any associated behaviors or receivers.
    Disconnected(T),
}

/// A future that sends a value to a [channel](self).
#[must_use = "Futures must be awaited to be executed"]
pub struct SendAsync<'a, T> {
    value: Option<T>,
    channel: &'a Sender<T>,
}

impl<T> Future for SendAsync<'_, T>
where
    T: Unpin + Send + 'static,
{
    type Output = Result<(), T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(value) = self.value.take() else {
            return Poll::Ready(Ok(()));
        };
        match self
            .channel
            .data
            .try_send_inner(value, channel_id(&self.channel.data), |channel| {
                let will_wake = channel
                    .wakers
                    .iter()
                    .any(|waker| waker.will_wake(cx.waker()));
                if !will_wake {
                    channel.wakers.push(cx.waker().clone());
                }
                ControlFlow::Break(())
            }) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(TrySendError::Disconnected(value)) => Poll::Ready(Err(value)),
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
    fn disconnect(&self);
    fn persist_callback_handle(&self);
    fn drop_callback_handle(&self);
}

impl<T, Behavior> AnyChannel for Arc<ChannelData<T, Behavior>>
where
    T: Send + 'static,
    Behavior: CallbackBehavior<T>,
{
    fn should_poll(&self) -> bool {
        let channel = self.synced.lock();
        !channel.queue.is_empty() && channel.behavior.should_poll()
    }

    fn poll(&self, futures: &mut Vec<ChannelCallbackFuture>) -> bool {
        let mut channel = self.synced.lock();
        while let Some(value) = channel.queue.pop_front() {
            notify(&mut channel, self);

            match channel.behavior.invoke(value, self) {
                Ok(()) => {}
                Err(ChannelCallbackError::Async(future)) => {
                    futures.push(ChannelCallbackFuture { future });
                    return true;
                }
                Err(ChannelCallbackError::Disconnected) => {
                    return false;
                }
            }
        }

        channel.senders > 0 && channel.behavior.connected()
    }

    fn disconnect(&self) {
        let mut data = self.synced.lock();
        data.behavior.disconnect();
        notify_dropping(data, self);
    }

    fn persist_callback_handle(&self) {
        let mut data = self.synced.lock();
        data.handle_status = CallbackHandleStatus::Persisted;
    }

    fn drop_callback_handle(&self) {
        let mut data = self.synced.lock();
        if !matches!(data.handle_status, CallbackHandleStatus::Persisted) {
            data.handle_status = CallbackHandleStatus::Dropped;
            notify_dropping(data, self);
        }
    }
}

pub(super) struct ChannelCallbackFuture {
    pub(super) future: Pin<Box<dyn Future<Output = Result<(), CallbackDisconnected>>>>,
}

/// A sender of values to a [channel](self).
#[derive(Debug)]
pub struct Sender<T> {
    data: Arc<ChannelData<T, SingleCallback<T>>>,
}

impl<T> Sender<T>
where
    T: Send + 'static,
{
    /// Sends `value` to this channel.
    ///
    /// If the channel is full, this function will block the current thread
    /// until space is made available. If one channel's `on_receive` is sending
    /// a value to a bounded channel, that `on_receive` should be
    /// `on_receive_async` instead and use [`send_async()`](Self::send_async).
    ///
    /// # Errors
    ///
    /// Returns `Err(value)` if the channel is disconnected.
    pub fn send(&self, value: T) -> Result<(), T> {
        match self
            .data
            .try_send_inner(value, channel_id(&self.data), |channel| {
                self.data.condvar.wait(channel);
                ControlFlow::Continue(())
            }) {
            Ok(()) => Ok(()),
            Err(TrySendError::Disconnected(value) | TrySendError::Full(value)) => Err(value),
        }
    }

    /// Sends `value` to this channel asynchronously.
    ///
    /// If the channel is full, this future will wait until space is made
    /// available before sending.
    ///
    /// # Errors
    ///
    /// The returned future will return `Err(value)` if the channel is
    /// disconnected.
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
        self.data
            .try_send_inner(value, channel_id(&self.data), |_| ControlFlow::Break(()))
    }

    /// Sends `value` to this channel, removing the oldest unread value if the
    /// channel is full.
    ///
    /// If the channel is full, the unread value will be returned in
    /// `Ok(Some(unread_value))`. If the channel has capacity, `Ok(None)` will
    /// be returned.
    ///
    /// # Errors
    ///
    /// Returns `value` if the channel is disconnected.
    pub fn force_send(&self, value: T) -> Result<Option<T>, T> {
        self.data.force_send_inner(value, channel_id(&self.data))
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut channel = self.data.synced.lock();
        channel.senders += 1;
        Self {
            data: self.data.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut channel = self.data.synced.lock();
        channel.senders -= 1;

        if channel.senders == 0 {
            match &channel.behavior {
                SingleCallback::Receiver => {
                    drop(channel);
                    self.data.condvar.notify_all();
                }
                SingleCallback::Callback(_) => {
                    drop(channel);
                    enqueue_task(BackgroundTask::Channel(ChannelTask::Unregister(
                        channel_id(&self.data),
                    )));
                }
                SingleCallback::Disconnected => {}
            }
        }
    }
}

impl<T> Default for Sender<T>
where
    T: Send + 'static,
{
    /// Returns a disconnected sender.
    fn default() -> Self {
        unbounded().0
    }
}

enum SingleCallback<T> {
    Receiver,
    Callback(Box<dyn AnyChannelCallback<T>>),
    Disconnected,
}

impl<T> CallbackBehavior<T> for SingleCallback<T>
where
    T: Send + 'static,
{
    fn connected(&self) -> bool {
        !matches!(self, Self::Disconnected)
    }

    fn should_poll(&self) -> bool {
        matches!(self, Self::Callback(_))
    }

    fn disconnect(&mut self) {
        *self = Self::Disconnected;
    }

    fn invoke(
        &mut self,
        value: T,
        _channel: &Arc<ChannelData<T, Self>>,
    ) -> Result<(), ChannelCallbackError> {
        let cb = match self {
            SingleCallback::Receiver => unreachable!("callback installed without callback"),
            SingleCallback::Callback(cb) => cb,
            SingleCallback::Disconnected => return Err(ChannelCallbackError::Disconnected),
        };

        match cb.invoke(value) {
            Err(ChannelCallbackError::Disconnected) => {
                *self = Self::Disconnected;
                Err(ChannelCallbackError::Disconnected)
            }
            other => other,
        }
    }
}

impl<T> Debug for SingleCallback<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SingleCallback::Receiver => f.write_str("0 callbacks"),
            SingleCallback::Callback(_) => f.write_str("1 callback"),
            SingleCallback::Disconnected => f.write_str("disconnected"),
        }
    }
}

enum BroadcastCallback<T> {
    Blocking {
        sender: Sender<(T, Autowaker)>,
        result: Receiver<()>,
    },
    NonBlocking(Box<dyn AnyChannelCallback<T>>),
}

impl<T> BroadcastCallback<T> {
    fn spawn_blocking(
        mut cb: Box<dyn FnMut(T) -> Result<(), super::CallbackDisconnected> + Send + 'static>,
    ) -> Self
    where
        T: Send + 'static,
    {
        let (value_sender, value_receiver) = bounded::<(T, Autowaker)>(1);
        let (result_sender, result_receiver) = bounded(1);
        std::thread::spawn(move || {
            while let Some((value, waker)) = value_receiver.receive() {
                if let Ok(()) = cb(value) {
                    if result_sender.send(()).is_err() {
                        return;
                    }
                    waker.wake();
                } else {
                    drop(result_sender);
                    waker.wake();
                    return;
                }
            }
        });
        Self::Blocking {
            sender: value_sender,
            result: result_receiver,
        }
    }
}

struct MultipleCallbacks<T>(Vec<BroadcastCallback<T>>);

impl<T> CallbackBehavior<T> for MultipleCallbacks<T>
where
    T: Unpin + Clone + Send + 'static,
{
    fn connected(&self) -> bool {
        !self.0.is_empty()
    }

    fn should_poll(&self) -> bool {
        self.connected()
    }

    fn disconnect(&mut self) {
        self.0.clear();
    }

    fn invoke(
        &mut self,
        value: T,
        channel: &Arc<ChannelData<T, Self>>,
    ) -> Result<(), ChannelCallbackError> {
        let mut sent_one = false;

        let mut i = 0;
        let mut value = TakeN::new(value, self.0.len());
        while i < self.0.len() {
            match &mut self.0[i] {
                BroadcastCallback::Blocking { .. } => {
                    return Err(ChannelCallbackError::Async(Box::pin(BroadcastSend {
                        value,
                        sent_one,
                        data: channel.clone(),
                        current_recipient_future: None,
                        current_is_blocking: false,
                        next_recipient: i,
                    })))
                }
                BroadcastCallback::NonBlocking(cb) => {
                    match cb.invoke(value.next().expect("enough value clones")) {
                        Ok(()) => {
                            sent_one = true;
                        }
                        Err(ChannelCallbackError::Disconnected) => {
                            self.0.remove(i);
                            continue;
                        }
                        Err(ChannelCallbackError::Async(future)) => {
                            return Err(ChannelCallbackError::Async(Box::pin(BroadcastSend {
                                value,
                                sent_one,
                                data: channel.clone(),
                                current_recipient_future: Some(future),
                                current_is_blocking: false,
                                next_recipient: i + 1,
                            })))
                        }
                    }
                }
            }

            i += 1;
        }

        if sent_one {
            Ok(())
        } else {
            Err(ChannelCallbackError::Disconnected)
        }
    }
}

impl<T> Debug for MultipleCallbacks<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.len() == 1 {
            f.write_str("1 callback")
        } else {
            write!(f, "{} callbacks", self.0.len())
        }
    }
}

struct TakeN<T> {
    value: Option<T>,
    remaining: usize,
}

impl<T> TakeN<T> {
    fn new(value: T, count: usize) -> Self {
        Self {
            value: Some(value),
            remaining: count,
        }
    }
}

impl<T> Iterator for TakeN<T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.remaining = self.remaining.saturating_sub(1);
        if self.remaining > 0 {
            self.value.clone()
        } else {
            self.value.take()
        }
    }
}

struct BroadcastSend<T> {
    sent_one: bool,
    value: TakeN<T>,
    next_recipient: usize,
    data: Arc<ChannelData<T, MultipleCallbacks<T>>>,
    current_recipient_future: Option<AsyncCallbackFuture>,
    current_is_blocking: bool,
}

impl<T> BroadcastSend<T>
where
    T: Unpin + Clone + Send + 'static,
{
    fn poll_tasks(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(future) = &mut self.current_recipient_future {
            match ready!(future.as_mut().poll(cx)) {
                Ok(()) => {
                    self.current_recipient_future = None;
                    self.sent_one = true;
                }
                Err(CallbackDisconnected) => {
                    self.current_recipient_future = None;
                }
            }
        } else if self.current_is_blocking {
            let mut data = self.data.synced.lock();
            let BroadcastCallback::Blocking { result, .. } = &data.behavior.0[self.next_recipient]
            else {
                unreachable!("valid state");
            };
            match result.try_receive() {
                Ok(()) => {
                    self.next_recipient += 1;
                }
                Err(TryReceiveError::Empty) => return Poll::Pending,
                Err(TryReceiveError::Disconnected) => {
                    data.behavior.0.remove(self.next_recipient);
                }
            }
            self.current_is_blocking = false;
        }
        Poll::Ready(())
    }
}

impl<T> Future for BroadcastSend<T>
where
    T: Unpin + Clone + Send + 'static,
{
    type Output = Result<(), CallbackDisconnected>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        ready!(this.poll_tasks(cx));

        let mut data_mutex = this.data.synced.lock();
        loop {
            let data = &mut *data_mutex;
            if let Some(cb) = data.behavior.0.get_mut(this.next_recipient) {
                match cb {
                    BroadcastCallback::Blocking { sender, .. } => {
                        if let Ok(()) = sender.send((
                            this.value.next().expect("enough value clones"),
                            Autowaker(Some(cx.waker().clone())),
                        )) {
                            this.current_is_blocking = true;
                            drop(data_mutex);

                            ready!(this.poll_tasks(cx));

                            data_mutex = this.data.synced.lock();
                            continue;
                        }

                        data.behavior.0.remove(this.next_recipient);
                        continue;
                    }
                    BroadcastCallback::NonBlocking(cb) => {
                        match cb.invoke(this.value.next().expect("enough value clones")) {
                            Ok(()) => {
                                this.sent_one = true;
                            }
                            Err(ChannelCallbackError::Disconnected) => {
                                data.behavior.0.remove(this.next_recipient);
                                continue;
                            }
                            Err(ChannelCallbackError::Async(future)) => {
                                this.current_recipient_future = Some(future);
                                drop(data_mutex);

                                ready!(this.poll_tasks(cx));

                                data_mutex = this.data.synced.lock();
                            }
                        }
                    }
                }

                this.next_recipient += 1;
            } else if this.sent_one {
                return Poll::Ready(Ok(()));
            } else {
                for waker in data.wakers.drain(..) {
                    waker.wake();
                }
                drop(data_mutex);
                this.data.condvar.notify_all();
                return Poll::Ready(Err(CallbackDisconnected));
            }
        }
    }
}

trait CallbackBehavior<T>: Sized + Send + 'static {
    fn connected(&self) -> bool;
    fn should_poll(&self) -> bool;
    fn disconnect(&mut self);
    fn invoke(
        &mut self,
        value: T,
        channel: &Arc<ChannelData<T, Self>>,
    ) -> Result<(), ChannelCallbackError>;
}

#[derive(Debug)]
struct ChannelData<T, Callbacks> {
    condvar: Condvar,
    synced: Mutex<SyncedChannelData<T, Callbacks>>,
}

impl<T, Behavior> ChannelData<T, Behavior>
where
    T: Send + 'static,
    Behavior: CallbackBehavior<T>,
{
    fn new(
        limit: Option<usize>,
        behavior: Behavior,
        senders: usize,
        receivers: usize,
    ) -> Arc<ChannelData<T, Behavior>> {
        let (queue, limit) = match limit {
            Some(limit) => (VecDeque::with_capacity(limit), limit),
            None => (VecDeque::new(), usize::MAX),
        };
        let this = Arc::new(ChannelData {
            condvar: Condvar::new(),
            synced: Mutex::new(SyncedChannelData {
                queue,
                limit,
                senders,
                receivers,
                wakers: Vec::new(),
                handle_status: CallbackHandleStatus::None,
                behavior,
            }),
        });
        enqueue_task(BackgroundTask::Channel(ChannelTask::Register {
            id: channel_id(&this),
            data: Arc::new(this.clone()),
        }));
        this
    }

    fn force_send_inner(&self, value: T, id: usize) -> Result<Option<T>, T> {
        let mut overflowed = None;
        self.try_send_inner(value, id, |g| {
            overflowed = g.queue.pop_front();
            ControlFlow::Continue(())
        })
        .map_err(|err| match err {
            TrySendError::Full(value) | TrySendError::Disconnected(value) => value,
        })?;
        Ok(overflowed)
    }

    fn try_send_inner(
        &self,
        value: T,
        id: usize,
        mut when_full: impl FnMut(
            &mut MutexGuard<'_, SyncedChannelData<T, Behavior>>,
        ) -> ControlFlow<()>,
    ) -> Result<(), TrySendError<T>> {
        let mut channel = self.synced.lock();
        while !matches!(channel.handle_status, CallbackHandleStatus::Dropped)
            && (channel.receivers > 0 || channel.behavior.connected())
        {
            if channel.queue.len() >= channel.limit {
                match when_full(&mut channel) {
                    ControlFlow::Continue(()) => continue,
                    ControlFlow::Break(()) => return Err(TrySendError::Full(value)),
                }
            }
            let has_receiver = channel.receivers > 0;
            let should_notify = !has_receiver && channel.queue.is_empty();
            channel.queue.push_back(value);
            drop(channel);

            if should_notify {
                enqueue_task(BackgroundTask::Channel(ChannelTask::Notify { id }));
            } else if has_receiver {
                self.condvar.notify_all();
            }

            return Ok(());
        }
        Err(TrySendError::Disconnected(value))
    }
}

#[derive(Debug)]
enum CallbackHandleStatus {
    None,
    Held,
    Persisted,
    Dropped,
}

struct SyncedChannelData<T, Behavior> {
    queue: VecDeque<T>,
    limit: usize,
    senders: usize,
    receivers: usize,
    wakers: Vec<Waker>,
    handle_status: CallbackHandleStatus,

    behavior: Behavior,
}

impl<T, Behavior> Debug for SyncedChannelData<T, Behavior>
where
    T: Debug,
    Behavior: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyncedChannelData")
            .field("queue", &self.queue)
            .field("limit", &self.limit)
            .field("senders", &self.senders)
            .field("receiers", &self.receivers)
            .field("wakers", &self.wakers)
            .field("callback_handle", &self.handle_status)
            .field("behavior", &self.behavior)
            .finish()
    }
}

/// A channel that broadcasts values received to one or more callbacks.
///
/// This type represents both a sender and a receiver in terms of determining
/// whether a channel is "connected". This is because at any time additional
/// callbacks can be associated through this type while also allowing values to
/// be sent to already-installed callbacks.
///
/// Because of this ability to attach future callbacks, a broadcast channel can
/// be created with no associated callbacks. When a value is sent to a channel
/// that has a [`BroadcastChannel`] connected to it, the value will be queued
/// even if no callbacks are currently associated. To prevent this, use
/// [`create_broadcaster()`](Self::create_broadcaster)/[`into_broadcaster()`](Self::into_broadcaster)
/// to create a [`Broadcaster`] for this channel and drop all
/// [`BroadcastChannel`] instances after callbacks have been associated.
pub struct BroadcastChannel<T> {
    data: Arc<ChannelData<T, MultipleCallbacks<T>>>,
}

impl<T> BroadcastChannel<T>
where
    T: Unpin + Clone + Send + 'static,
{
    /// Returns broadcast channel with no limit to the number of values
    /// enqueued.
    #[must_use]
    pub fn unbounded() -> Self {
        Builder::new().broadcasting().finish()
    }

    /// Returns broadcast channel that limits queued values to `capacity` items.
    #[must_use]
    pub fn bounded(capacity: usize) -> Self {
        Builder::new().broadcasting().bounded(capacity).finish()
    }

    /// Returns a builder for a broadcast channel.
    pub fn build() -> Builder<T, builder::Broadcast<T>> {
        Builder::new().broadcasting()
    }

    /// Sends `value` to this channel.
    ///
    /// If the channel is full, this function will block the current thread
    /// until space is made available. If one channel's `on_receive` is sending
    /// a value to a bounded channel, that `on_receive` should be
    /// `on_receive_async` instead and use [`send_async()`](Self::send_async).
    ///
    /// # Errors
    ///
    /// Returns `Err(value)` if the channel is disconnected.
    pub fn send(&self, value: T) -> Result<(), T> {
        match self
            .data
            .try_send_inner(value, channel_id(&self.data), |channel| {
                self.data.condvar.wait(channel);
                ControlFlow::Continue(())
            }) {
            Ok(()) => Ok(()),
            Err(TrySendError::Disconnected(value) | TrySendError::Full(value)) => Err(value),
        }
    }

    /// Sends `value` to this channel asynchronously.
    ///
    /// If the channel is full, this future will wait until space is made
    /// available before sending.
    ///
    /// # Errors
    ///
    /// The returned future will return `Err(value)` if the channel is
    /// disconnected.
    pub fn send_async(&self, value: T) -> BroadcastAsync<'_, T> {
        BroadcastAsync {
            value: Some(value),
            channel: &self.data,
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
        self.data
            .try_send_inner(value, channel_id(&self.data), |_| ControlFlow::Break(()))
    }

    /// Sends `value` to this channel, removing the oldest unread value if the
    /// channel is full.
    ///
    /// If the channel is full, the unread value will be returned in
    /// `Ok(Some(unread_value))`. If the channel has capacity, `Ok(None)` will
    /// be returned.
    ///
    /// # Errors
    ///
    /// Returns `value` if the channel is disconnected.
    pub fn force_send(&self, value: T) -> Result<Option<T>, T> {
        self.data.force_send_inner(value, channel_id(&self.data))
    }

    /// Creates a new receiver for this channel.
    ///
    /// All receivers and callbacks must receive each value before the next
    /// value is able to be received.
    #[must_use]
    pub fn create_receiver(&self) -> Receiver<T> {
        let (sender, receiver) = bounded(1);
        self.on_receive_async_try(move |value| {
            let sender = sender.clone();
            async move {
                sender
                    .send_async(value)
                    .await
                    .map_err(|_| CallbackDisconnected)
            }
        })
        .persist();
        receiver
    }

    /// Invokes `on_receive` each time a value is sent to this channel.
    ///
    /// This function assumes `on_receive` may block while waiting on another
    /// thread, another process, another callback, a network request, a locking
    /// primitive, or any other number of ways that could impact other callbacks
    /// from executing.
    pub fn on_receive<Map>(&self, mut on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) + Send + 'static,
    {
        self.on_receive_try(move |value| {
            on_receive(value);
            Ok(())
        })
    }

    /// Invokes `on_receive` each time a value is sent to this channel. Once an
    /// error is returned, this callback will be removed from the channel.
    ///
    /// This function assumes `on_receive` may block while waiting on another
    /// thread, another process, another callback, a network request, a locking
    /// primitive, or any other number of ways that could impact other callbacks
    /// from executing.
    ///
    /// Once the last callback associated with a channel is removed, [`Sender`]s
    /// will begin returning disconnected errors.
    pub fn on_receive_try<Map>(&self, on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.on_receive_inner(ChannelCallbackKind::Blocking(Box::new(on_receive)))
    }

    /// Invokes `on_receive` each time a value is sent to this channel.
    ///
    /// This function assumes `on_receive` will not block while waiting on
    /// another thread, another process, another callback, a network request, a
    /// locking primitive, or any other number of ways that could impact other
    /// callbacks from executing in a shared environment.
    pub fn on_receive_nonblocking<Map>(&self, mut on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) + Send + 'static,
    {
        self.on_receive_nonblocking_try(move |value| {
            on_receive(value);
            Ok(())
        })
    }

    /// Invokes `on_receive` each time a value is sent to this channel. Once an
    /// error is returned, this callback will be removed from the channel.
    ///
    /// This function assumes `on_receive` will not block while waiting on
    /// another thread, another process, another callback, a network request, a
    /// locking primitive, or any other number of ways that could impact other
    /// callbacks from executing in a shared environment.
    ///
    /// Once the last callback associated with a channel is removed, [`Sender`]s
    /// will begin returning disconnected errors.
    pub fn on_receive_nonblocking_try<Map>(&self, mut on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.on_receive_inner(ChannelCallbackKind::NonBlocking(Box::new(move |value| {
            on_receive(value).map_err(|CallbackDisconnected| ChannelCallbackError::Disconnected)
        })))
    }

    /// Invokes `on_receive` each time a value is sent to this channel.
    pub fn on_receive_async<Map, Fut>(&self, mut on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_receive_async_try(move |value| {
            let future = on_receive(value);
            async move {
                future.await;
                Ok(())
            }
        })
    }

    /// Invokes `on_receive` each time a value is sent to this channel. The
    /// returned future will then be awaited. Once an error is returned, this
    /// callback will be removed from the channel.
    ///
    /// Once the last callback associated with a channel is removed, [`Sender`]s
    /// will begin returning disconnected errors.
    pub fn on_receive_async_try<Map, Fut>(&self, mut on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), CallbackDisconnected>> + Send + 'static,
    {
        self.on_receive_inner(ChannelCallbackKind::NonBlocking(Box::new(move |value| {
            let future = on_receive(value);
            Err(ChannelCallbackError::Async(Box::pin(future)))
        })))
    }

    fn on_receive_inner(&self, cb: ChannelCallbackKind<T>) -> CallbackHandle {
        let data_arc = Arc::new(self.data.clone());
        let mut data = self.data.synced.lock();
        data.handle_status = CallbackHandleStatus::Held;
        let should_register = data.behavior.0.is_empty();
        match cb {
            ChannelCallbackKind::Blocking(cb) => {
                data.behavior.0.push(BroadcastCallback::spawn_blocking(cb));
            }
            ChannelCallbackKind::NonBlocking(cb) => {
                data.behavior.0.push(BroadcastCallback::NonBlocking(cb));
            }
        }
        if should_register {
            drop(data);
            enqueue_task(BackgroundTask::Channel(ChannelTask::Register {
                id: channel_id(&self.data),
                data: data_arc.clone(),
            }));
        }
        CallbackHandle(CallbackHandleInner::Single(CallbackKind::Channel(
            ChannelCallbackHandle(data_arc),
        )))
    }
}

impl<T> BroadcastChannel<T> {
    /// Returns a [`Broadcaster`] that sends to this channel.
    #[must_use]
    pub fn create_broadcaster(&self) -> Broadcaster<T> {
        let mut data = self.data.synced.lock();
        data.senders += 1;
        Broadcaster {
            data: self.data.clone(),
        }
    }

    /// Returns this instance as a [`Broadcaster`] that sends to this channel.
    #[must_use]
    pub fn into_broadcaster(self) -> Broadcaster<T> {
        self.create_broadcaster()
    }
}

fn notify<T, Behavior>(
    synced: &mut SyncedChannelData<T, Behavior>,
    data: &ChannelData<T, Behavior>,
) {
    for waker in synced.wakers.drain(..) {
        waker.wake();
    }
    data.condvar.notify_all();
}

fn notify_dropping<T, Behavior>(
    mut guard: MutexGuard<'_, SyncedChannelData<T, Behavior>>,
    data: &ChannelData<T, Behavior>,
) {
    for waker in guard.wakers.drain(..) {
        waker.wake();
    }
    drop(guard);
    data.condvar.notify_all();
}

impl<T, U> Unwrapped<U> for &BroadcastChannel<T>
where
    T: IntoOption<U> + Clone + Unpin + Send + 'static,
    U: Send + 'static,
{
    type Value<'a> = U;

    fn unwrapped_or_else(self, initial: impl FnOnce() -> U) -> Dynamic<U> {
        let mut data = self.data.synced.lock();
        let initial = if data.receivers == 1 && data.behavior.0.is_empty() {
            if let Some(value) = data.queue.pop_front() {
                notify_dropping(data, &self.data);

                value.into_option().unwrap_or_else(initial)
            } else {
                initial()
            }
        } else {
            initial()
        };
        let mapped = Dynamic::new(initial);
        let weak_mapped = mapped.downgrade();
        mapped.set_source(self.on_receive_try(move |value| {
            let mapped = weak_mapped.upgrade().ok_or(CallbackDisconnected)?;
            if let Some(value) = value.into_option() {
                *mapped.lock() = value;
            }
            Ok(())
        }));
        mapped
    }

    fn for_each_unwrapped_try<ForEach>(self, mut for_each: ForEach) -> CallbackHandle
    where
        ForEach:
            for<'a> FnMut(Self::Value<'a>) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.on_receive_try(move |option| {
            if let Some(value) = option.into_option() {
                for_each(value)
            } else {
                Ok(())
            }
        })
    }
}

// impl<T> Unwrapped<T> for &BroadcastChannel<Option<T>>
// where
//     T: Unpin + Clone + Send + 'static,
// {
//     type Value<'a> = T;

//     fn unwrapped_or_else(self, initial: impl FnOnce() -> T) -> Dynamic<T> {
//         let mut data = self.data.synced.lock();
//         let initial = if data.receivers == 1 && data.behavior.0.is_empty() {
//             if let Some(value) = data.queue.pop_front() {
//                 notify_dropping(data, &self.data);

//                 value.unwrap_or_else(initial)
//             } else {
//                 initial()
//             }
//         } else {
//             initial()
//         };
//         let mapped = Dynamic::new(initial);
//         let weak_mapped = mapped.downgrade();
//         mapped.set_source(self.on_receive_try(move |value| {
//             let mapped = weak_mapped.upgrade().ok_or(CallbackDisconnected)?;
//             if let Some(value) = value {
//                 *mapped.lock() = value;
//             }
//             Ok(())
//         }));
//         mapped
//     }

//     fn for_each_unwrapped_try<ForEach>(self, mut for_each: ForEach) -> CallbackHandle
//     where
//         ForEach:
//             for<'a> FnMut(Self::Value<'a>) -> Result<(), CallbackDisconnected> + Send + 'static,
//     {
//         self.on_receive_try(move |option| {
//             if let Some(value) = option {
//                 for_each(value)
//             } else {
//                 Ok(())
//             }
//         })
//     }
// }

impl<T> Clone for BroadcastChannel<T> {
    fn clone(&self) -> Self {
        let mut data = self.data.synced.lock();
        data.senders += 1;
        data.receivers += 1;
        drop(data);
        Self {
            data: self.data.clone(),
        }
    }
}

impl<T> Default for BroadcastChannel<T>
where
    T: Unpin + Clone + Send + 'static,
{
    /// Returns an unbounded broadcast channel.
    fn default() -> Self {
        Self::unbounded()
    }
}

impl<T> Drop for BroadcastChannel<T> {
    fn drop(&mut self) {
        let mut data = self.data.synced.lock();
        data.senders -= 1;
        data.receivers -= 1;

        let notify_disconnected = data.senders == 0 || data.behavior.0.is_empty();
        if notify_disconnected {
            notify_dropping(data, &self.data);
        }
        if notify_disconnected {
            self.data.condvar.notify_all();
            enqueue_task(BackgroundTask::Channel(ChannelTask::Unregister(
                channel_id(&self.data),
            )));
        }
    }
}

/// Sends values to a [`BroadcastChannel`].
#[derive(Debug)]
pub struct Broadcaster<T> {
    data: Arc<ChannelData<T, MultipleCallbacks<T>>>,
}

impl<T> Broadcaster<T>
where
    T: Unpin + Clone + Send + 'static,
{
    /// Sends `value` to this channel.
    ///
    /// If the channel is full, this function will block the current thread
    /// until space is made available. If one channel's `on_receive` is sending
    /// a value to a bounded channel, that `on_receive` should be
    /// `on_receive_async` instead and use [`send_async()`](Self::send_async).
    ///
    /// # Errors
    ///
    /// Returns `Err(value)` if the channel is disconnected.
    pub fn send(&self, value: T) -> Result<(), T> {
        match self
            .data
            .try_send_inner(value, channel_id(&self.data), |channel| {
                self.data.condvar.wait(channel);
                ControlFlow::Continue(())
            }) {
            Ok(()) => Ok(()),
            Err(TrySendError::Disconnected(value) | TrySendError::Full(value)) => Err(value),
        }
    }

    /// Sends `value` to this channel asynchronously.
    ///
    /// If the channel is full, this future will wait until space is made
    /// available before sending.
    ///
    /// # Errors
    ///
    /// The returned future will return `Err(value)` if the channel is
    /// disconnected.
    pub fn send_async(&self, value: T) -> BroadcastAsync<'_, T> {
        BroadcastAsync {
            value: Some(value),
            channel: &self.data,
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
        self.data
            .try_send_inner(value, channel_id(&self.data), |_| ControlFlow::Break(()))
    }

    /// Sends `value` to this channel, removing the oldest unread value if the
    /// channel is full.
    ///
    /// If the channel is full, the unread value will be returned in
    /// `Ok(Some(unread_value))`. If the channel has capacity, `Ok(None)` will
    /// be returned.
    ///
    /// # Errors
    ///
    /// Returns `value` if the channel is disconnected.
    pub fn force_send(&self, value: T) -> Result<Option<T>, T> {
        self.data.force_send_inner(value, channel_id(&self.data))
    }
}

impl<T> Clone for Broadcaster<T> {
    fn clone(&self) -> Self {
        let mut data = self.data.synced.lock();
        data.senders += 1;
        drop(data);
        Self {
            data: self.data.clone(),
        }
    }
}

impl<T> Default for Broadcaster<T>
where
    T: Unpin + Clone + Send + 'static,
{
    /// Returns a disconnected broadcaster.
    fn default() -> Self {
        BroadcastChannel::unbounded().into_broadcaster()
    }
}

impl<T> Drop for Broadcaster<T> {
    fn drop(&mut self) {
        let mut data = self.data.synced.lock();
        data.senders -= 1;

        let notify_disconnected = data.senders == 0;
        if notify_disconnected {
            notify_dropping(data, &self.data);
        }
        if notify_disconnected {
            enqueue_task(BackgroundTask::Channel(ChannelTask::Unregister(
                channel_id(&self.data),
            )));
        }
    }
}

/// A future that broadcasts a value to a [`BroadcastChannel<T>`].
#[must_use = "Futures must be awaited to be executed"]
pub struct BroadcastAsync<'a, T> {
    value: Option<T>,
    channel: &'a Arc<ChannelData<T, MultipleCallbacks<T>>>,
}

impl<T> Future for BroadcastAsync<'_, T>
where
    T: Unpin + Clone + Send + 'static,
{
    type Output = Option<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(value) = self.value.take() else {
            return Poll::Ready(None);
        };
        match self
            .channel
            .try_send_inner(value, channel_id(self.channel), |channel| {
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

impl<F, T> AnyChannelCallback<T> for F
where
    F: FnMut(T) -> Result<(), ChannelCallbackError> + Send + 'static,
{
    fn invoke(&mut self, value: T) -> Result<(), ChannelCallbackError> {
        self(value)
    }
}

fn channel_id<T, Behavior>(data: &Arc<ChannelData<T, Behavior>>) -> usize {
    Arc::as_ptr(data) as usize
}

/// A receiver for values sent by a [`Sender`].
pub struct Receiver<T> {
    data: Arc<ChannelData<T, SingleCallback<T>>>,
}

impl<T> Receiver<T>
where
    T: Send + 'static,
{
    /// Returns the next value, blocking the current thread until one is
    /// available.
    ///
    /// Returns `None` if there are no [`Sender`]s still connected to this
    /// channel.
    #[must_use]
    pub fn receive(&self) -> Option<T> {
        self.try_receive_inner(|guard| {
            self.data.condvar.wait(guard);
            ControlFlow::Continue(())
        })
        .ok()
    }

    /// Returns the next value if it can be retrieved within `timeout`.
    ///
    /// # Errors
    ///
    /// - [`TryReceiveError::Disconnected`] is returned if no senders are
    ///   connected to this receiver.
    /// - [`TryReceiveError::Empty`] is returned if `timeout` elapses before a
    ///   value is received.
    pub fn receive_timeout(&self, timeout: Duration) -> Result<T, TryReceiveError> {
        self.receive_until(Instant::now() + timeout)
    }

    /// Returns the next value if it can be retrieved before `instant`.
    ///
    /// If a value is already available, it will be returned even if `instant`
    /// is in the past when this function is invoked. The timeout logic only is
    /// applied when the queue is empty.
    ///
    /// # Errors
    ///
    /// - [`TryReceiveError::Disconnected`] is returned if no senders are
    ///   connected to this receiver.
    /// - [`TryReceiveError::Empty`] is returned if `timeout` elapses before a
    ///   value is received.
    pub fn receive_until(&self, instant: Instant) -> Result<T, TryReceiveError> {
        let mut timed_out = false;
        self.try_receive_inner(|guard| {
            if self.data.condvar.wait_until(guard, instant).timed_out() {
                timed_out = true;
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
    }

    /// Returns the next value if possible, otherwise returning an error
    /// describing why a value was unable to be received.
    ///
    /// This function will not block the current thread.
    ///
    /// # Errors
    ///
    /// - [`TryReceiveError::Disconnected`] is returned if no senders are
    ///   connected to this receiver.
    /// - [`TryReceiveError::Empty`] is returned if no value is available in
    ///   this channel.
    pub fn try_receive(&self) -> Result<T, TryReceiveError> {
        self.try_receive_inner(|_guard| ControlFlow::Break(()))
    }

    fn try_receive_inner(
        &self,
        mut when_empty: impl FnMut(
            &mut MutexGuard<'_, SyncedChannelData<T, SingleCallback<T>>>,
        ) -> ControlFlow<()>,
    ) -> Result<T, TryReceiveError> {
        let mut data = self.data.synced.lock();
        loop {
            if matches!(data.handle_status, CallbackHandleStatus::Dropped) {
                return Err(TryReceiveError::Disconnected);
            }

            if let Some(value) = data.queue.pop_front() {
                for waker in data.wakers.drain(..) {
                    waker.wake();
                }
                drop(data);
                self.data.condvar.notify_all();
                return Ok(value);
            }

            if data.senders == 0 {
                return Err(TryReceiveError::Disconnected);
            }

            if when_empty(&mut data).is_break() {
                return Err(TryReceiveError::Empty);
            }
        }
    }

    /// Invokes `on_receive` each time a value is sent to this channel.
    ///
    /// This function assumes `on_receive` may block while waiting on another
    /// thread, another process, another callback, a network request, a locking
    /// primitive, or any other number of ways that could impact other callbacks
    /// from executing.
    pub fn on_receive<Map>(self, mut on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) + Send + 'static,
    {
        self.on_receive_try(move |value| {
            on_receive(value);
            Ok(())
        })
    }

    /// Invokes `on_receive` each time a value is sent to this channel. Once an
    /// error is returned, this callback will be removed from the channel.
    ///
    /// This function assumes `on_receive` may block while waiting on another
    /// thread, another process, another callback, a network request, a locking
    /// primitive, or any other number of ways that could impact other callbacks
    /// from executing.
    ///
    /// Once the last callback associated with a channel is removed, [`Sender`]s
    /// will begin returning disconnected errors.
    pub fn on_receive_try<Map>(self, on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.on_receive_inner(ChannelCallbackKind::Blocking(Box::new(on_receive)))
    }

    /// Invokes `on_receive` each time a value is sent to this channel.
    ///
    /// This function assumes `on_receive` will not block while waiting on
    /// another thread, another process, another callback, a network request, a
    /// locking primitive, or any other number of ways that could impact other
    /// callbacks from executing in a shared environment.
    pub fn on_receive_nonblocking<Map>(self, mut on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) + Send + 'static,
    {
        self.on_receive_nonblocking_try(move |value| {
            on_receive(value);
            Ok(())
        })
    }

    /// Invokes `on_receive` each time a value is sent to this channel. Once an
    /// error is returned, this callback will be removed from the channel.
    ///
    /// This function assumes `on_receive` will not block while waiting on
    /// another thread, another process, another callback, a network request, a
    /// locking primitive, or any other number of ways that could impact other
    /// callbacks from executing in a shared environment.
    ///
    /// Once the last callback associated with a channel is removed, [`Sender`]s
    /// will begin returning disconnected errors.
    pub fn on_receive_nonblocking_try<Map>(self, mut on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.on_receive_inner(ChannelCallbackKind::NonBlocking(Box::new(move |value| {
            on_receive(value).map_err(|CallbackDisconnected| ChannelCallbackError::Disconnected)
        })))
    }

    /// Invokes `on_receive` each time a value is sent to this channel.
    pub fn on_receive_async<Map, Fut>(self, mut on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_receive_async_try(move |value| {
            let future = on_receive(value);
            async move {
                future.await;
                Ok(())
            }
        })
    }

    /// Invokes `on_receive` each time a value is sent to this channel. The
    /// returned future will then be awaited. Once an error is returned, this
    /// callback will be removed from the channel.
    ///
    /// Once the last callback associated with a channel is removed, [`Sender`]s
    /// will begin returning disconnected errors.
    pub fn on_receive_async_try<Map, Fut>(self, mut on_receive: Map) -> CallbackHandle
    where
        Map: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), CallbackDisconnected>> + Send + 'static,
    {
        self.on_receive_inner(ChannelCallbackKind::NonBlocking(Box::new(move |value| {
            let future = on_receive(value);
            Err(ChannelCallbackError::Async(Box::pin(future)))
        })))
    }

    fn on_receive_inner(self, cb: ChannelCallbackKind<T>) -> CallbackHandle {
        let data_arc = Arc::new(self.data.clone());
        let mut data = self.data.synced.lock();
        data.handle_status = CallbackHandleStatus::Held;
        match cb {
            ChannelCallbackKind::Blocking(fn_mut) => {
                drop(data);
                self.spawn_thread(fn_mut);
            }
            ChannelCallbackKind::NonBlocking(cb) => {
                data.behavior = SingleCallback::Callback(cb);
                drop(data);
                enqueue_task(BackgroundTask::Channel(ChannelTask::Register {
                    id: channel_id(&self.data),
                    data: data_arc.clone(),
                }));
            }
        }
        CallbackHandle(CallbackHandleInner::Single(CallbackKind::Channel(
            ChannelCallbackHandle(data_arc),
        )))
    }

    fn spawn_thread(
        self,
        mut cb: Box<dyn FnMut(T) -> Result<(), super::CallbackDisconnected> + Send + 'static>,
    ) {
        std::thread::spawn(move || {
            while let Some(value) = self.receive() {
                if let Err(CallbackDisconnected) = cb(value) {
                    return;
                }
            }
        });
    }
}

impl<T> Future for &Receiver<T>
where
    T: Unpin + Send + 'static,
{
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.try_receive_inner(|guard| {
            let will_wake = guard.wakers.iter().any(|w| w.will_wake(cx.waker()));
            if !will_wake {
                guard.wakers.push(cx.waker().clone());
            }
            ControlFlow::Break(())
        }) {
            Ok(value) => Poll::Ready(Some(value)),
            Err(TryReceiveError::Disconnected) => Poll::Ready(None),
            Err(TryReceiveError::Empty) => Poll::Pending,
        }
    }
}

impl<T> Unwrapped<T> for Receiver<Option<T>>
where
    T: Send + 'static,
{
    type Value<'a> = T;

    fn unwrapped_or_else(self, initial: impl FnOnce() -> T) -> Dynamic<T> {
        let initial = self.try_receive().ok().flatten().unwrap_or_else(initial);
        let mapped = Dynamic::new(initial);
        let weak_mapped = mapped.downgrade();
        mapped.set_source(self.on_receive_try(move |value| {
            let mapped = weak_mapped.upgrade().ok_or(CallbackDisconnected)?;
            if let Some(value) = value {
                *mapped.lock() = value;
            }
            Ok(())
        }));
        mapped
    }

    fn for_each_unwrapped_try<ForEach>(self, mut for_each: ForEach) -> CallbackHandle
    where
        ForEach:
            for<'a> FnMut(Self::Value<'a>) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.on_receive_try(move |option| {
            if let Some(value) = option {
                for_each(value)
            } else {
                Ok(())
            }
        })
    }
}

impl<T, E> Unwrapped<T> for Receiver<Result<T, E>>
where
    T: Send + 'static,
    E: Send + 'static,
{
    type Value<'a> = T;

    fn unwrapped_or_else(self, initial: impl FnOnce() -> T) -> Dynamic<T> {
        let initial = self
            .try_receive()
            .ok()
            .and_then(Result::ok)
            .unwrap_or_else(initial);
        let mapped = Dynamic::new(initial);
        let weak_mapped = mapped.downgrade();
        mapped.set_source(self.on_receive_try(move |value| {
            let mapped = weak_mapped.upgrade().ok_or(CallbackDisconnected)?;
            if let Ok(value) = value {
                *mapped.lock() = value;
            }
            Ok(())
        }));
        mapped
    }

    fn for_each_unwrapped_try<ForEach>(self, mut for_each: ForEach) -> CallbackHandle
    where
        ForEach:
            for<'a> FnMut(Self::Value<'a>) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        self.on_receive_try(move |option| {
            if let Ok(value) = option {
                for_each(value)
            } else {
                Ok(())
            }
        })
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let mut data = self.data.synced.lock();
        data.receivers -= 1;
        if matches!(data.behavior, SingleCallback::Receiver) {
            data.behavior = SingleCallback::Disconnected;
            notify_dropping(data, &self.data);
        }
    }
}

/// An error trying to receive a value from a channel.
pub enum TryReceiveError {
    /// The channel was empty.
    Empty,
    /// The channel has no senders connected.
    Disconnected,
}

struct Autowaker(Option<Waker>);

impl Autowaker {
    fn wake_by_ref(&mut self) {
        let Some(waker) = self.0.take() else {
            return;
        };
        waker.wake();
    }

    fn wake(mut self) {
        self.wake_by_ref();
    }
}

impl Drop for Autowaker {
    fn drop(&mut self) {
        self.wake_by_ref();
    }
}

pub(super) struct ChannelCallbackHandle(Arc<dyn AnyChannel>);

impl ChannelCallbackHandle {
    pub fn persist(self) {
        self.0.persist_callback_handle();
    }
}

impl fmt::Debug for ChannelCallbackHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Channel")
    }
}

impl PartialEq for ChannelCallbackHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Drop for ChannelCallbackHandle {
    fn drop(&mut self) {
        self.0.drop_callback_handle();
    }
}

#[test]
fn channel_basics() {
    let (result_sender, result_receiver) = unbounded();

    let sender = Builder::new()
        .on_receive_nonblocking(move |value| result_sender.send(dbg!(value)).unwrap())
        .finish();
    sender.send(1).unwrap();

    assert_eq!(result_receiver.receive().unwrap(), 1);
    drop(sender);
    assert_eq!(result_receiver.receive(), None);
}

#[test]
fn send_then_spawn() {
    let (result_sender, result_receiver) = unbounded();

    let (sender, receiver) = Builder::new().finish();
    sender.send(1).unwrap();
    receiver
        .on_receive_nonblocking(move |value| result_sender.send(dbg!(value)).unwrap())
        .persist();

    assert_eq!(result_receiver.receive().unwrap(), 1);
    drop(sender);
    assert_eq!(result_receiver.receive(), None);
}

#[test]
fn disconnected_send() {
    let (sender, receiver) = Builder::new().finish();
    // Sending is allowed while a receiver could theoretically receive it.
    sender.send(1).unwrap();
    drop(receiver);
    assert_eq!(sender.send(2), Err(2));
}

#[test]
fn broadcast_basic() {
    let (result_sender, result_receiver) = unbounded();

    let channel = Builder::new()
        .broadcasting()
        .on_receive_nonblocking({
            let result_sender = result_sender.clone();
            move |value| {
                result_sender.send(value).unwrap();
            }
        })
        .on_receive_nonblocking({
            move |value| {
                result_sender.send(value).unwrap();
            }
        })
        .finish();
    channel.send(1).unwrap();

    assert_eq!(result_receiver.receive(), Some(1));
    assert_eq!(result_receiver.receive(), Some(1));
    drop(channel);
    assert_eq!(result_receiver.receive(), None);
}

#[test]
fn async_channels() {
    let (a_sender, a_receiver) = bounded(1);
    let (b_sender, b_receiver) = bounded(1);

    a_receiver
        .on_receive_async(move |value| {
            let b_sender = b_sender.clone();
            async move {
                for i in 0..value {
                    b_sender.send_async(dbg!(i)).await.unwrap();
                }
            }
        })
        .persist();
    a_sender.send(5).unwrap();
    for i in 0..5 {
        println!("Reading {i}");
        assert_eq!(b_receiver.receive(), Some(i));
    }
    drop(a_sender);
    assert_eq!(b_receiver.receive(), None);
}

#[test]
fn callback_disconnection() {
    let (a_sender, a_receiver) = bounded(1);
    let (b_sender, b_receiver) = bounded(1);

    let handle = a_receiver.on_receive(move |value| b_sender.send(value).unwrap());
    a_sender.send(1_usize).unwrap();
    assert_eq!(b_receiver.receive(), Some(1));

    drop(handle);

    assert_eq!(a_sender.send(2_usize), Err(2));
    assert_eq!(b_receiver.receive(), None);
}
