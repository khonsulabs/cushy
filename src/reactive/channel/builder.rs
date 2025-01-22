//! Builder types for Cushy [`channel`](super)qs.
use std::future::Future;
use std::marker::PhantomData;

use super::sealed::{CallbackKind, ChannelCallbackError};
use super::{
    BroadcastCallback, BroadcastChannel, ChannelData, MultipleCallbacks, Receiver, Sender,
};
use crate::value::CallbackDisconnected;

/// Builds a Cushy channel.
///
/// This type can be used to create all types of channels supported by Cushy.
/// See the [`channel`](self) module documentation for an overview of the
/// channels provided.
#[must_use]
pub struct Builder<T, Mode = SingleConsumer> {
    mode: Mode,
    ty: PhantomData<T>,
    bound: Option<usize>,
}

impl<T> Default for Builder<T, SingleConsumer> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Builder<T, SingleConsumer> {
    /// Returns a builder for a Cushy channel.
    ///
    /// The default builder will create an unbounded, Multi-Producer,
    /// Single-Consumer channel. See the [`channel`](self) module documentation
    /// for an overview of the channels provided.
    pub const fn new() -> Self {
        Self {
            mode: SingleConsumer { _private: () },
            ty: PhantomData,
            bound: None,
        }
    }
}

impl<T, Mode> Builder<T, Mode>
where
    T: Send + 'static,
    Mode: ChannelMode<T> + sealed::ChannelMode<T, Next = <Mode as ChannelMode<T>>::Next>,
{
    /// Invokes `on_receive` each time a value is sent to this channel.
    ///
    /// This function assumes `on_receive` may block while waiting on another
    /// thread, another process, another callback, a network request, a locking
    /// primitive, or any other number of ways that could impact other callbacks
    /// from executing.
    pub fn on_receive<Map>(self, mut on_receive: Map) -> Builder<T, <Mode as ChannelMode<T>>::Next>
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
    pub fn on_receive_try<Map>(self, map: Map) -> Builder<T, <Mode as ChannelMode<T>>::Next>
    where
        Map: FnMut(T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        Builder {
            mode: self
                .mode
                .push_callback(CallbackKind::Blocking(Box::new(map))),
            bound: self.bound,
            ty: self.ty,
        }
    }

    /// Invokes `on_receive` each time a value is sent to this channel.
    ///
    /// This function assumes `on_receive` will not block while waiting on
    /// another thread, another process, another callback, a network request, a
    /// locking primitive, or any other number of ways that could impact other
    /// callbacks from executing in a shared environment.
    pub fn on_receive_nonblocking<Map>(
        self,
        mut on_receive: Map,
    ) -> Builder<T, <Mode as ChannelMode<T>>::Next>
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
    pub fn on_receive_nonblocking_try<Map>(
        self,
        mut map: Map,
    ) -> Builder<T, <Mode as ChannelMode<T>>::Next>
    where
        Map: FnMut(T) -> Result<(), CallbackDisconnected> + Send + 'static,
    {
        Builder {
            mode: self
                .mode
                .push_callback(CallbackKind::NonBlocking(Box::new(move |value| {
                    map(value).map_err(|CallbackDisconnected| ChannelCallbackError::Disconnected)
                }))),
            bound: self.bound,
            ty: self.ty,
        }
    }

    /// Invokes `on_receive` each time a value is sent to this channel.
    pub fn on_receive_async<Map, Fut>(
        self,
        mut on_receive: Map,
    ) -> Builder<T, <Mode as ChannelMode<T>>::Next>
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
    pub fn on_receive_async_try<Map, Fut>(
        self,
        mut on_receive: Map,
    ) -> Builder<T, <Mode as ChannelMode<T>>::Next>
    where
        Map: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), CallbackDisconnected>> + Send + 'static,
    {
        Builder {
            mode: self
                .mode
                .push_callback(CallbackKind::NonBlocking(Box::new(move |value| {
                    let future = on_receive(value);
                    Err(ChannelCallbackError::Async(Box::pin(future)))
                }))),
            bound: self.bound,
            ty: self.ty,
        }
    }

    /// Returns this builder reconfigured to create a [`BroadcastChannel`].
    ///
    /// See the [`channel`](self) module documentation for an overview of the
    /// channels provided.
    pub fn broadcasting(self) -> Builder<T, Broadcast<T>> {
        Builder {
            mode: self.mode.into(),
            ty: self.ty,
            bound: self.bound,
        }
    }

    /// Restricts this channel to `capacity` values queued.
    pub fn bounded(mut self, capacity: usize) -> Self {
        self.bound = Some(capacity);
        self
    }

    /// Returns the finished channel.
    pub fn finish(self) -> Mode::Channel {
        self.mode.finish(self.bound)
    }
}

/// Builder configuration for a single-consumer channel with no associated
/// callback.
pub struct SingleConsumer {
    _private: (),
}

impl<T> ChannelMode<T> for SingleConsumer
where
    T: Send + 'static,
{
    type Channel = (Sender<T>, Receiver<T>);
    type Next = SingleCallback<T>;

    fn finish(self, limit: Option<usize>) -> Self::Channel {
        let data = ChannelData::new(limit, super::SingleCallback::Receiver, 1, 1);

        (Sender { data: data.clone() }, Receiver { data })
    }
}

impl<T> sealed::ChannelMode<T> for SingleConsumer {
    type Next = SingleCallback<T>;

    fn push_callback(self, cb: CallbackKind<T>) -> Self::Next {
        SingleCallback { cb }
    }
}

impl<T> From<SingleConsumer> for Broadcast<T> {
    fn from(_: SingleConsumer) -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }
}

/// Builder configuration for a single-consumer channel with an associated
/// callback.
pub struct SingleCallback<T> {
    cb: CallbackKind<T>,
}

impl<T> ChannelMode<T> for SingleCallback<T>
where
    T: Send + 'static,
{
    type Channel = Sender<T>;
    type Next = Broadcast<T>;

    fn finish(self, limit: Option<usize>) -> Self::Channel {
        let data = match self.cb {
            CallbackKind::Blocking(cb) => {
                let data = ChannelData::new(limit, super::SingleCallback::Receiver, 1, 1);
                let receiver = Receiver { data: data.clone() };
                receiver.spawn_thread(cb);
                data
            }
            CallbackKind::NonBlocking(cb) => {
                ChannelData::new(limit, super::SingleCallback::Callback(cb), 1, 0)
            }
        };
        Sender { data }
    }
}

impl<T> sealed::ChannelMode<T> for SingleCallback<T> {
    type Next = Broadcast<T>;

    fn push_callback(self, cb: CallbackKind<T>) -> Self::Next {
        Broadcast {
            callbacks: vec![cb],
        }
    }
}

impl<T> From<SingleCallback<T>> for Broadcast<T> {
    fn from(single: SingleCallback<T>) -> Self {
        Self {
            callbacks: vec![single.cb],
        }
    }
}

/// Builder configuration for a [`BroadcastChannel`].
pub struct Broadcast<T> {
    callbacks: Vec<CallbackKind<T>>,
}

impl<T> ChannelMode<T> for Broadcast<T>
where
    T: Unpin + Clone + Send + 'static,
{
    type Channel = BroadcastChannel<T>;
    type Next = Self;

    fn finish(self, limit: Option<usize>) -> Self::Channel {
        let callbacks = self
            .callbacks
            .into_iter()
            .map(|cb| match cb {
                CallbackKind::Blocking(cb) => BroadcastCallback::spawn_blocking(cb),
                CallbackKind::NonBlocking(cb) => BroadcastCallback::NonBlocking(cb),
            })
            .collect();
        let data = ChannelData::new(limit, MultipleCallbacks(callbacks), 1, 1);
        BroadcastChannel { data }
    }
}

impl<T> sealed::ChannelMode<T> for Broadcast<T> {
    type Next = Self;

    fn push_callback(mut self, cb: CallbackKind<T>) -> Self::Next {
        self.callbacks.push(cb);
        self
    }
}

/// A channel configuration.
pub trait ChannelMode<T>: Into<Broadcast<T>> {
    /// The next configuration when a new callback is associated with this
    /// builder.
    type Next;
    /// The resulting channel type created from this configuration.
    type Channel;

    /// Returns the built channel.
    fn finish(self, limit: Option<usize>) -> Self::Channel;
}

mod sealed {
    use crate::channel::sealed::CallbackKind;

    pub trait ChannelMode<T> {
        type Next;

        fn push_callback(self, callback: CallbackKind<T>) -> Self::Next;
    }
}
