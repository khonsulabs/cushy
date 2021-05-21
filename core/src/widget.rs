use std::{
    any::{Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
    sync::{Arc, Weak},
};

use flume::{Receiver, Sender};

use crate::{Frontend, WidgetRef, WidgetRegistration, WidgetStorage};

/// A graphical user interface element.
pub trait Widget: Debug + Send + Sync + 'static {
    /// Widgets may need to communicate with transmogrifier implementations.
    /// This type is the type that can be sent to a transmogrifier.
    type TransmogrifierCommand: Debug + Send + Sync;

    /// The type of the event that any [`Transmogrifier`] for this widget to
    /// use.
    type TransmogrifierEvent: Debug + Send + Sync;

    /// Called when an `event` from the transmogrifier was received.
    #[allow(unused_variables)]
    fn receive_event(&mut self, event: Self::TransmogrifierEvent, context: &Context<Self>)
    where
        Self: Sized,
    {
        unimplemented!("an event was sent by the transmogrifier but receive_event isn't implemnted")
    }
}

/// A unique ID of a widget, with information about the widget type.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[allow(clippy::module_name_repetitions)]
pub struct WidgetId {
    /// The unique id of the widget.
    pub id: u32,
    /// The [`TypeId`] of the [`Widget`] type.
    pub type_id: TypeId,
}

/// Transforms a Widget into whatever is needed for [`Frontend`] `F`.
pub trait Transmogrifier<F: Frontend>: Debug {
    /// The type of the widget being transmogrified.
    type Widget: Widget;
    /// The type the storage this transmogrifier uses for state.
    type State: Default + Debug + Any + Send + Sync;

    /// Called when a command is received from the widget.
    #[allow(unused_variables)] // Keeps documentation clean
    fn receive_command(
        &self,
        state: &mut Self::State,
        command: <Self::Widget as Widget>::TransmogrifierCommand,
        widget: &Self::Widget,
        storage: &WidgetStorage,
    ) {
        unimplemented!(
            "widget tried to send a command, but the transmogrifier wasn't expecting one"
        )
    }

    /// Processes commands and events for this widget and transmogrifier.
    fn process_messages(
        &self,
        state: &mut Self::State,
        widget: &mut Self::Widget,
        storage: &WidgetStorage,
        channels: &Channels<Self::Widget>,
    ) {
        // The frontend is initiating this call, so we should process events that the
        // Transmogrifier sends first.
        while let Ok(event) = channels.event_receiver.try_recv() {
            let context = Context::new(channels, storage);
            widget.receive_event(event, &context);
        }

        while let Ok(command) = channels.command_receiver.try_recv() {
            self.receive_command(state, command, widget, storage);
        }
    }

    /// Returns an initialized state using `Self::State::default()`.
    fn default_state_for(&self, widget: &Arc<WidgetRegistration>) -> TransmogrifierState {
        TransmogrifierState {
            state: Box::new(<Self::State as Default>::default()),
            channels: Box::new(Channels::<Self::Widget>::new(widget)),
        }
    }

    /// Returns the [`TypeId`] of [`Self::Widget`].
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<Self::Widget>()
    }
}

/// A Widget without any associated types. Useful for implementing frontends.
#[allow(clippy::module_name_repetitions)]
pub trait AnyWidget: AnySendSync + 'static {
    /// Returns the [`TypeId`] of the widget.
    #[must_use]
    fn widget_type_id(&self) -> TypeId;
}

impl<T> AnyWidget for T
where
    T: Widget + Debug + Send + Sync + Any,
{
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

/// A generic-type-less trait for [`Channels`]
pub trait AnyChannels: AnySendSync {
    /// Returns the [`TypeId`] of the widget these channels are for.
    #[must_use]
    fn widget_type_id(&self) -> TypeId;
}

/// Generic storage for a transmogrifier.
#[derive(Debug)]
pub struct TransmogrifierState {
    /// The `State` type, stored without its type information.
    pub state: Box<dyn AnySendSync>,
    /// The `Channels<Widget>` type, stored without its type information.
    pub channels: Box<dyn AnyChannels>,
}

/// A value that can be used as [`Any`] that is threadsafe.
pub trait AnySendSync: Any + Debug + Send + Sync {
    /// Returns the underlying type as [`Any`].
    fn as_any(&self) -> &'_ dyn Any;
    /// Returns the underlying type as [`Any`].
    fn as_mut_any(&mut self) -> &'_ mut dyn Any;
}

impl<T> AnySendSync for T
where
    T: Any + Debug + Send + Sync,
{
    fn as_mut_any(&mut self) -> &'_ mut dyn Any {
        self
    }

    fn as_any(&self) -> &'_ dyn Any {
        self
    }
}

/// Enables [`Widget`]s to send commands to the
/// [`Transmogrifier`](crate::Transmogrifier).
pub struct Context<W: Widget> {
    widget: Weak<WidgetRegistration>,
    command_sender: Sender<W::TransmogrifierCommand>,
    storage: WidgetStorage,
    _widget: PhantomData<W>,
}

impl<W: Widget> Context<W> {
    /// Create a new `Context`.
    #[must_use]
    pub fn new(channels: &Channels<W>, storage: &WidgetStorage) -> Self {
        Self {
            widget: channels.widget.clone(),
            command_sender: channels.command_sender.clone(),
            storage: storage.clone(),
            _widget: PhantomData::default(),
        }
    }

    /// Send `command` to the transmogrifier.
    pub fn send_command(&self, command: W::TransmogrifierCommand) {
        if let Some(widget) = self.widget.upgrade() {
            drop(self.command_sender.send(command));
            self.storage.set_widget_has_messages(widget.id().clone());
        }
    }
}

/// Communication channels used to communicate between [`Widget`]s and
/// [`Transmogrifier`](crate::Transmogrifier)s.
#[derive(Debug)]
pub struct Channels<W: Widget> {
    widget: Weak<WidgetRegistration>,
    command_sender: Sender<W::TransmogrifierCommand>,
    command_receiver: Receiver<W::TransmogrifierCommand>,
    event_sender: Sender<W::TransmogrifierEvent>,
    event_receiver: Receiver<W::TransmogrifierEvent>,
    _phantom: PhantomData<W>,
}
impl<W: Widget> AnyChannels for Channels<W> {
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<W>()
    }
}

impl<W: Widget> Channels<W> {
    /// Creates a new set of channels for a widget and transmogrifier.
    #[must_use]
    fn new(widget: &Arc<WidgetRegistration>) -> Self {
        let (command_sender, command_receiver) = flume::unbounded();
        let (event_sender, event_receiver) = flume::unbounded();

        Self {
            widget: Arc::downgrade(widget),
            command_sender,
            command_receiver,
            event_sender,
            event_receiver,
            _phantom: PhantomData::default(),
        }
    }

    /// Sends an event to the [`Widget`].
    pub fn post_event(&self, event: W::TransmogrifierEvent) {
        drop(self.event_sender.send(event))
    }

    /// Returns the widget registration. Returns none if the widget has been
    /// destroyed.
    #[must_use]
    pub fn widget(&self) -> Option<Arc<WidgetRegistration>> {
        self.widget.upgrade()
    }

    /// Returns the widget reference for this widget. Returns none if the widget
    /// has been destroyed.
    #[must_use]
    pub fn widget_ref<F: Frontend>(&self, frontend: F) -> Option<WidgetRef<W, F>> {
        self.widget().and_then(|reg| WidgetRef::new(&reg, frontend))
    }
}
