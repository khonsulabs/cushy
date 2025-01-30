//! Reactive data types for Cushy
use std::cell::Cell;
use std::collections::{hash_map, VecDeque};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{mpsc, Arc};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Instant;

use ahash::AHashMap;
use alot::{LotId, Lots};
use channel::ChannelCallbackHandle;
use kempt::{map, Map, Set};
use parking_lot::Mutex;
use tracing::warn;
use value::Dynamic;

use self::channel::{AnyChannel, ChannelCallbackFuture};
use self::value::{DeadlockError, DynamicLockData};
use crate::{Cushy, Lazy};

pub mod channel;
pub mod value;

/// Unwrap values contained in a dynamic source.
pub trait Unwrapped<T>: Sized {
    /// The value type provided to the for each functions.
    type Value<'a>;

    /// Returns a dynamic that is updated with the unwrapped contents of thie
    /// source.
    ///
    /// The initial value of this dynamic will be the result of
    /// `unwrap_or_default()` on the value currently contained in this source.
    fn unwrapped(self) -> Dynamic<T>
    where
        T: Default,
    {
        self.unwrapped_or_else(T::default)
    }

    /// Returns a dynamic that is updated with the unwrapped contents of thie
    /// source.
    ///
    /// The initial value of this dynamic will be the result of
    /// `unwrap_or_else(initial)` on the value currently contained in this
    /// source.
    fn unwrapped_or_else(self, initial: impl FnOnce() -> T) -> Dynamic<T>;

    /// Invokes `for_each` when `self` is updated with a value that can be
    /// unwrapped.
    ///
    /// Returning `Err(CallbackDisconnected)` will prevent the callback from
    /// being invoked again.
    fn for_each_unwrapped_try<ForEach>(self, for_each: ForEach) -> CallbackHandle
    where
        ForEach:
            for<'a> FnMut(Self::Value<'a>) -> Result<(), CallbackDisconnected> + Send + 'static;

    /// Invokes `for_each` when `self` is updated with a value that can be
    /// unwrapped.
    fn for_each_unwrapped<ForEach>(self, mut for_each: ForEach) -> CallbackHandle
    where
        ForEach: for<'a> FnMut(Self::Value<'a>) + Send + 'static,
    {
        self.for_each_unwrapped_try(move |value| {
            for_each(value);
            Ok(())
        })
    }
}

/// A type that can be converted into an `Option<T>`.
///
/// This trait exists to unify how [`Unwrapped`] abstracts implementations for
/// `Result` and `Option`. In the future, if the standard library implements
/// `Into<Option<T>>` for `Result<T,E>`, this trait can be removed.
pub trait IntoOption<T> {
    /// Returns `self` as an option.
    fn into_option(self) -> Option<T>;
}

impl<T> IntoOption<T> for Option<T> {
    fn into_option(self) -> Option<T> {
        self
    }
}

impl<T, E> IntoOption<T> for Result<T, E> {
    fn into_option(self) -> Option<T> {
        self.ok()
    }
}

impl<'a, T> IntoOption<&'a T> for &'a Option<T> {
    fn into_option(self) -> Option<&'a T> {
        self.as_ref()
    }
}

impl<'a, T, E> IntoOption<&'a T> for &'a Result<T, E> {
    fn into_option(self) -> Option<&'a T> {
        self.as_ref().ok()
    }
}

/// A callback function is no longer connected to its source.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CallbackDisconnected;

static CALLBACK_EXECUTORS: Mutex<Map<usize, Arc<DynamicLockData>>> = Mutex::new(Map::new());

fn execute_callbacks(
    lock: Arc<DynamicLockData>,
    callbacks: &mut CallbacksList,
) -> Result<usize, DeadlockError> {
    let mut executors = CALLBACK_EXECUTORS.lock();
    let key = Arc::as_ptr(&lock) as usize;
    match executors.entry(key) {
        map::Entry::Occupied(_) => return Err(DeadlockError),
        map::Entry::Vacant(entry) => {
            entry.insert(lock);
        }
    }
    drop(executors);

    // Invoke all callbacks, removing those that report an
    // error.
    let mut count = 0;
    callbacks.invoked_at = Instant::now();
    callbacks.callbacks.drain_filter(|callback| {
        count += 1;
        callback.changed().is_err()
    });

    let mut executors = CALLBACK_EXECUTORS.lock();
    executors.remove(&key);

    Ok(count)
}

trait CallbackCollection: Send + Sync + 'static {
    fn remove(&self, id: LotId);
}

#[derive(Default)]
struct ChangeCallbacksData {
    callbacks: Mutex<CallbacksList>,
    lock: Arc<DynamicLockData>,
}

impl CallbackCollection for ChangeCallbacksData {
    fn remove(&self, id: LotId) {
        if CallbackExecutor::is_current_thread() {
            let mut state = self.lock.state.lock();
            state.callbacks_to_remove.push(id);
        } else {
            let mut data = self.callbacks.lock();
            data.callbacks.remove(id);
        }
    }
}

struct CallbacksList {
    callbacks: Lots<Box<dyn ValueCallback>>,
    invoked_at: Instant,
}

impl Default for CallbacksList {
    fn default() -> Self {
        Self {
            callbacks: Lots::new(),
            invoked_at: Instant::now(),
        }
    }
}

struct ChangeCallbacks {
    data: Arc<ChangeCallbacksData>,
    changed_at: Instant,
}

impl ChangeCallbacks {
    fn new(data: Arc<ChangeCallbacksData>) -> Self {
        Self {
            data,
            changed_at: Instant::now(),
        }
    }

    fn execute(self) -> usize {
        // Invoke the callbacks
        let mut data = self.data.callbacks.lock();
        // If the callbacks have already been invoked by another
        // thread such that the callbacks observed the value our
        // thread wrote, we can skip the callbacks.
        let Some(Ok(count)) = (data.invoked_at < self.changed_at)
            .then(|| execute_callbacks(self.data.lock.clone(), &mut data))
        else {
            return 0;
        };

        // Clean up all callbacks that were disconnected while our callbacks
        // were locked.
        let mut state = self.data.lock.state.lock();
        for callback in state.callbacks_to_remove.drain(..) {
            data.callbacks.remove(callback);
        }
        drop(data);
        drop(state);
        self.data.lock.sync.notify_all();
        count
    }
}

trait ValueCallback: Send {
    fn changed(&mut self) -> Result<(), CallbackDisconnected>;
}

impl<F> ValueCallback for F
where
    F: for<'a> FnMut() -> Result<(), CallbackDisconnected> + Send + 'static,
{
    fn changed(&mut self) -> Result<(), CallbackDisconnected> {
        self()
    }
}

static THREAD_SENDER: Lazy<mpsc::SyncSender<BackgroundTask>> = Lazy::new(|| {
    let (sender, receiver) = mpsc::sync_channel(256);
    std::thread::spawn(move || CallbackExecutor::new(receiver).run());
    sender
});

fn defer_execute_callbacks(callbacks: ChangeCallbacks) {
    let _ = THREAD_SENDER.send(BackgroundTask::ExecuteCallbacks(callbacks));
}

enum BackgroundTask {
    ExecuteCallbacks(ChangeCallbacks),
    Channel(ChannelTask),
    Wake(usize),
}

enum ChannelTask {
    Register {
        id: usize,
        data: Arc<dyn AnyChannel>,
    },
    Notify {
        id: usize,
    },
    Unregister(usize),
}

struct RegisteredFuture {
    future: Option<PollChannelFuture>,
    waker: Waker,
}

struct FutureWaker {
    id: usize,
}

impl Wake for FutureWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let _ = THREAD_SENDER.send(BackgroundTask::Wake(self.id));
    }
}

#[derive(Default)]
struct Futures {
    registered: Vec<RegisteredFuture>,
    queue: VecDeque<usize>,
    available: Set<usize>,
}

impl Futures {
    fn spawn(&mut self, future: PollChannelFuture) -> usize {
        let id = self.push(future);
        self.queue.push_back(id);
        id
    }

    fn push(&mut self, future: PollChannelFuture) -> usize {
        let mut id = None;
        while !self.available.is_empty() {
            let available_id = self.available.remove_member(0);
            if self.registered[available_id].future.is_none() {
                id = Some(available_id);
                break;
            }
        }
        if let Some(id) = id {
            self.registered[id].future = Some(future);
            id
        } else {
            let id = self.registered.len();
            self.registered.push(RegisteredFuture {
                future: Some(future),
                waker: Waker::from(Arc::new(FutureWaker { id })),
            });
            id
        }
    }

    fn poll(&mut self) -> usize {
        // We want to make sure we yield to allow other change callbacks to
        // execute, so we only allow each future currently enqueued to be polled
        // once.
        let mut callbacks_executed = 0;
        for _ in 0..self.queue.len() {
            let Some(id) = self.queue.pop_front() else {
                break;
            };

            let registered = &mut self.registered[id];
            if let Some(future) = &mut registered.future {
                let mut ctx = Context::from_waker(&registered.waker);
                match Pin::new(future).poll(&mut ctx) {
                    Poll::Ready(()) => {
                        registered.future = None;
                        self.available.insert(id);
                        callbacks_executed += 1;
                    }
                    Poll::Pending => {}
                }
            } else {
                self.available.insert(id);
            }
        }
        callbacks_executed
    }

    fn wake(&mut self, id: usize) {
        self.queue.push_back(id);
    }
}

struct CallbackExecutor {
    receiver: mpsc::Receiver<BackgroundTask>,

    channels: WatchedChannels,
    futures: Futures,

    queue: VecDeque<ChangeCallbacks>,
}

impl CallbackExecutor {
    fn new(receiver: mpsc::Receiver<BackgroundTask>) -> Self {
        Self {
            receiver,
            queue: VecDeque::new(),
            futures: Futures::default(),
            channels: WatchedChannels::default(),
        }
    }

    fn enqueue_nonblocking(&mut self) {
        // Exhaust any pending callbacks without blocking.
        while let Ok(task) = self.receiver.try_recv() {
            self.enqueue(task);
        }
    }

    fn run(mut self) {
        IS_EXECUTOR_THREAD.set(true);
        let cushy = Cushy::current();
        let _runtime = cushy.enter_runtime();

        // Because this is stored in a static, this likely will never return an
        // error, but if it does, it's during program shutdown, and we can exit safely.
        while let Ok(task) = self.receiver.recv() {
            self.enqueue(task);

            while !self.futures.queue.is_empty() || !self.queue.is_empty() {
                self.enqueue_nonblocking();
                let mut callbacks_executed = 0;
                while let Some(enqueued) = self.queue.pop_front() {
                    callbacks_executed += enqueued.execute();
                }

                callbacks_executed += self.futures.poll();

                if callbacks_executed > 0 {
                    tracing::trace!("{callbacks_executed} callbacks executed");
                }
            }
        }
    }

    fn enqueue(&mut self, task: BackgroundTask) {
        match task {
            BackgroundTask::Channel(channel) => match channel {
                ChannelTask::Register { id, data } => {
                    self.channels.register(id, data, &mut self.futures);
                }
                ChannelTask::Notify { id } => {
                    self.channels.notify(id, &mut self.futures);
                }
                ChannelTask::Unregister(id) => {
                    if let Some(future_id) = self.channels.unregister(id) {
                        self.futures.wake(future_id);
                    }
                }
            },
            BackgroundTask::ExecuteCallbacks(callbacks) => {
                self.queue.push_back(callbacks);
            }
            BackgroundTask::Wake(future_id) => {
                self.futures.wake(future_id);
            }
        }
    }

    fn is_current_thread() -> bool {
        IS_EXECUTOR_THREAD.get()
    }
}

#[derive(Default)]
struct WatchedChannels {
    registry: Lots<WatchedChannel>,
    by_id: AHashMap<usize, LotId>,
}

impl WatchedChannels {
    fn register(&mut self, id: usize, channel: Arc<dyn AnyChannel>, futures: &mut Futures) {
        let hash_map::Entry::Vacant(entry) = self.by_id.entry(id) else {
            return;
        };
        let future_id = channel.should_poll().then(|| {
            futures.spawn(PollChannelFuture {
                channel: channel.clone(),
                futures: Vec::new(),
            })
        });
        entry.insert(self.registry.push(WatchedChannel {
            data: channel,
            future_id,
        }));
    }

    fn notify(&mut self, id: usize, futures: &mut Futures) {
        let Some(channel) = self
            .by_id
            .get(&id)
            .and_then(|id| self.registry.get_mut(*id))
        else {
            return;
        };
        if channel.future_id.is_none() {
            channel.future_id = Some(futures.push(PollChannelFuture {
                channel: channel.data.clone(),
                futures: Vec::new(),
            }));
        }
        futures
            .queue
            .push_back(channel.future_id.expect("initialized above"));
    }

    fn unregister(&mut self, id: usize) -> Option<usize> {
        let id = self.by_id.remove(&id)?;
        self.registry
            .remove(id)
            .and_then(|removed| removed.future_id)
    }
}

struct WatchedChannel {
    data: Arc<dyn AnyChannel>,
    future_id: Option<usize>,
}

struct PollChannelFuture {
    channel: Arc<dyn AnyChannel>,
    futures: Vec<ChannelCallbackFuture>,
}

impl Future for PollChannelFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        if this.futures.is_empty() && !this.channel.poll(&mut this.futures) {
            this.channel.disconnect();
            return Poll::Ready(());
        }
        loop {
            let mut completed_one = false;
            let mut i = 0;
            while i < self.futures.len() {
                match self.futures[i].future.as_mut().poll(cx) {
                    Poll::Ready(result) => {
                        match result {
                            Ok(()) => {}
                            Err(CallbackDisconnected) => {
                                self.channel.disconnect();
                            }
                        }
                        completed_one = true;
                        self.futures.remove(i);
                    }
                    Poll::Pending => {
                        i += 1;
                    }
                }
            }

            if !completed_one {
                break;
            }
        }

        Poll::Pending
    }
}

thread_local! {
    static IS_EXECUTOR_THREAD: Cell<bool> = const { Cell::new(false) };
}

fn enqueue_task(task: BackgroundTask) {
    if THREAD_SENDER.send(task).is_err() {
        warn!("background task thread not running");
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
    Single(CallbackKind),
    Multi(Vec<CallbackKind>),
}

#[derive(Debug, PartialEq)]
enum CallbackKind {
    Channel(ChannelCallbackHandle),
    Value(CallbackHandleData),
}

impl CallbackKind {
    fn persist(self) {
        match self {
            Self::Channel(channel) => {
                channel.persist();
            }
            Self::Value(data) => {
                data.persist();
            }
        }
    }

    fn forget_owners(&mut self) {
        match self {
            CallbackKind::Channel(_) => {}
            CallbackKind::Value(handle) => {
                handle.owner = None;
            }
        }
    }
}

trait ReferencedDynamic: Sync + Send + 'static {}
impl<T> ReferencedDynamic for T where T: Sync + Send + 'static {}

struct CallbackHandleData {
    id: Option<LotId>,
    owner: Option<Arc<dyn ReferencedDynamic>>,
    callbacks: Arc<dyn CallbackCollection>,
}

impl fmt::Debug for CallbackHandleData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.id, f)
    }
}

impl fmt::Debug for CallbackHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tuple = f.debug_tuple("CallbackHandle");
        match &self.0 {
            CallbackHandleInner::None => {}
            CallbackHandleInner::Single(handle) => {
                tuple.field(handle);
            }
            CallbackHandleInner::Multi(handles) => {
                for handle in handles {
                    tuple.field(handle);
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
            CallbackHandleInner::Single(handle) => {
                handle.persist();
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
                handle.forget_owners();
            }
            CallbackHandleInner::Multi(handles) => {
                for handle in handles {
                    handle.forget_owners();
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

impl std::ops::Add for CallbackHandle {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl std::ops::AddAssign for CallbackHandle {
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
