use std::cell::Cell;
use std::collections::{hash_map, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{mpsc, Arc};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Instant;

use ahash::AHashMap;
use alot::{LotId, Lots};
use kempt::{map, Map, Set};
use parking_lot::Mutex;
use tracing::warn;

use self::channel::{AnyChannel, ChannelCallbackFuture};
use self::value::{CallbackDisconnected, DeadlockError, DynamicLockData};
use crate::Lazy;

pub mod channel;
pub mod value;

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

    fn id(&self) -> CallbacksId {
        CallbacksId(Arc::as_ptr(&self.data) as usize)
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
    let invoked_by = EXECUTING_CALLBACK_ROOT.get();
    let _ = THREAD_SENDER.send(BackgroundTask::ExecuteCallbacks {
        callbacks,
        invoked_by,
    });
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct CallbacksId(usize);

#[derive(Clone, Copy, Eq, PartialEq)]
struct CallbackInvocationId {
    node: LotId,
    invocation_root: CallbacksId,
}

struct InvocationTreeNode {
    id: CallbacksId,
    enqueued: Option<ChangeCallbacks>,
    parent: Option<LotId>,
    first_child: Option<LotId>,
    root_callbacks: CallbacksId,
    next: Option<LotId>,
}

#[derive(Default)]
struct InvocationTree {
    nodes: Lots<InvocationTreeNode>,
}

impl InvocationTree {
    fn new_root(&mut self, callbacks: ChangeCallbacks) -> CallbackInvocationId {
        let callbacks_id = callbacks.id();
        let node = self.nodes.push(InvocationTreeNode {
            id: callbacks_id,
            parent: None,
            first_child: None,
            root_callbacks: callbacks_id,
            enqueued: Some(callbacks),
            next: None,
        });
        CallbackInvocationId {
            node,
            invocation_root: callbacks_id,
        }
    }

    /// Pushes `invoked` into the list of callbacks executed by group of
    /// callbacks pointed to by `invoked_by`.
    ///
    /// # Errors
    ///
    /// Returns an error if `invoked` has already been executed by this group of
    /// callbacks.
    fn push(
        &mut self,
        callbacks: ChangeCallbacks,
        enqueued_while_executing: Option<CallbackInvocationId>,
    ) -> Option<CallbackInvocationId> {
        if let Some(enqueued_while_executing) = enqueued_while_executing {
            // Verify that `callbacks` wasn't executed in the chain leading to the node that was executing this node.
            let mut search = enqueued_while_executing.node;
            let callbacks_id = callbacks.id();
            loop {
                let node = &self.nodes[search];
                if node.id == callbacks_id {
                    // This set of callbacks has already been executed in this
                    // chain.
                    return None;
                }
                let Some(parent) = node.parent else {
                    break;
                };
                search = parent;
            }
            let root_invoked_by = enqueued_while_executing.invocation_root;
            let existing_first_child = self.nodes[enqueued_while_executing.node].first_child;
            if let Some(mut node_id) = existing_first_child {
                loop {
                    let node = &mut self.nodes[node_id];
                    if node.id == callbacks_id {
                        if let Some(enqueued) = &mut node.enqueued {
                            enqueued.changed_at = enqueued.changed_at.max(callbacks.changed_at);
                            return None;
                        }

                        node.enqueued = Some(callbacks);
                        return Some(CallbackInvocationId {
                            node: node_id,
                            invocation_root: root_invoked_by,
                        });
                    }

                    if let Some(next) = node.next {
                        // Continue traversing the list.
                        node_id = next;
                    } else {
                        break;
                    }
                }
            }

            // `callbacks` hasn't been executed by the list pointed at by
            // `enqueued_while_executing`.
            let id = self.nodes.push(InvocationTreeNode {
                id: callbacks.id(),
                enqueued: Some(callbacks),
                parent: Some(enqueued_while_executing.node),
                first_child: None,
                root_callbacks: root_invoked_by,
                next: existing_first_child,
            });
            self.nodes[enqueued_while_executing.node].first_child = Some(id);
            Some(CallbackInvocationId {
                node: id,
                invocation_root: root_invoked_by,
            })
        } else {
            // New root
            Some(self.new_root(callbacks))
        }
    }

    fn complete(&mut self, invocation: CallbackInvocationId) {
        self.remove_completed_recursive(invocation.node);
    }

    fn remove_completed_recursive(&mut self, mut node_id: LotId) {
        let mut node = &mut self.nodes[node_id];
        while node.enqueued.is_none() && node.first_child.is_none() {
            let after_removed = node.next;
            if let Some(parent_id) = node.parent {
                let parent = &mut self.nodes[parent_id];
                // Repair the linked list
                if parent.first_child == Some(node_id) {
                    parent.first_child = after_removed;
                } else {
                    let mut current = parent.first_child.expect("valid child");
                    while self.nodes[current].next != Some(node_id) {
                        current = self.nodes[current].next.expect("removed node to exist");
                    }
                    self.nodes[current].next = after_removed;
                }

                self.nodes.remove(node_id);

                // Attempt to remove the parent if this was its last node.
                node = &mut self.nodes[parent_id];
                node_id = parent_id;
            } else {
                self.nodes.remove(node_id);
                break;
            }
        }
    }
}

thread_local! {
    static EXECUTING_CALLBACK_ROOT: Cell<Option<CallbackInvocationId>> = const { Cell::new(None) };
}

struct EnqueuedCallbacks {
    node_id: CallbackInvocationId,
    callbacks: ChangeCallbacks,
}

enum BackgroundTask {
    ExecuteCallbacks {
        callbacks: ChangeCallbacks,
        invoked_by: Option<CallbackInvocationId>,
    },
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
                        self.registered.remove(id);
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

    invocations: InvocationTree,
    queue: VecDeque<LotId>,
}

impl CallbackExecutor {
    fn new(receiver: mpsc::Receiver<BackgroundTask>) -> Self {
        Self {
            receiver,
            invocations: InvocationTree::default(),
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

        // Because this is stored in a static, this likely will never return an
        // error, but if it does, it's during program shutdown, and we can exit safely.
        while let Ok(task) = self.receiver.recv() {
            self.enqueue(task);

            loop {
                let mut callbacks_executed = 0;
                loop {
                    let Some(enqueued) = self.pop_callbacks() else {
                        break;
                    };
                    EXECUTING_CALLBACK_ROOT.set(Some(enqueued.node_id));
                    callbacks_executed += enqueued.callbacks.execute();

                    // Enqueue any queued operations before we complete this
                    // invocation to ensure all related invocations are tracked.
                    self.enqueue_nonblocking();
                    self.invocations.complete(enqueued.node_id);
                }
                EXECUTING_CALLBACK_ROOT.set(None);

                // Once we've exited the loop, we can assume all callback invocation
                // chains have completed.
                assert!(self.invocations.nodes.is_empty());

                callbacks_executed += self.futures.poll();

                if callbacks_executed > 0 {
                    tracing::trace!("{callbacks_executed} callbacks executed");
                }

                if self.futures.queue.is_empty() {
                    break;
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
                    self.channels.unregister(id);
                }
            },
            BackgroundTask::ExecuteCallbacks {
                callbacks,
                invoked_by,
            } => {
                if let Some(pushed) = self.invocations.push(callbacks, invoked_by) {
                    self.queue.push_back(pushed.node);
                }
            }
            BackgroundTask::Wake(future_id) => {
                self.futures.wake(future_id);
            }
        }
    }

    fn pop_callbacks(&mut self) -> Option<EnqueuedCallbacks> {
        while let Some(id) = self.queue.pop_front() {
            if let Some(callbacks) = self.invocations.nodes[id].enqueued.take() {
                return Some(EnqueuedCallbacks {
                    callbacks,
                    node_id: CallbackInvocationId {
                        node: id,
                        invocation_root: self.invocations.nodes[id].root_callbacks,
                    },
                });
            }
        }

        None
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
            futures.push(PollChannelFuture {
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

    fn unregister(&mut self, id: usize) {
        let Some(id) = self.by_id.remove(&id) else {
            return;
        };
        self.registry.remove(id);
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
                                self.channel.disconnect_callback();
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
