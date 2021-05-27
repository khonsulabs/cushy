use std::{
    any::{type_name, Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
    ops::Deref,
};

use flume::{Receiver, Sender};

use crate::{
    AnyFrontend, Frontend, WeakWidgetRegistration, WidgetRef, WidgetRegistration, WidgetStorage,
};

/// A graphical user interface element.
pub trait Widget: Debug + Send + Sync + 'static {
    /// Widgets may need to receive instructions from other entities. This type
    /// is the type other widgets can use to communicate with this widget;
    type Command: Debug + Send + Sync;

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
        unimplemented!(
            "an event `{:?}` was sent by the transmogrifier but receive_event isn't implemented \
             by {}",
            event,
            type_name::<Self>()
        )
    }

    /// Called when an `event` from the transmogrifier was received.
    #[allow(unused_variables)]
    fn receive_command(&mut self, command: Self::Command, context: &Context<Self>)
    where
        Self: Sized,
    {
        unimplemented!(
            "a commmand `{:?}` was sent but receive_command isn't implemented by {}",
            command,
            type_name::<Self>()
        )
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

    /// Called after a transmogrifier is initialized.
    #[allow(unused_variables)]
    fn initialize(
        &self,
        widget: &Self::Widget,
        reference: &WidgetRef<Self::Widget>,
        frontend: &F,
    ) -> Self::State {
        <Self::State as Default>::default()
    }

    /// Called when a command is received from the widget.
    #[allow(unused_variables)] // Keeps documentation clean
    fn receive_command(
        &self,
        state: &mut Self::State,
        command: <Self::Widget as Widget>::TransmogrifierCommand,
        widget: &Self::Widget,
        frontend: &F,
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
        frontend: &F,
        channels: &Channels<Self::Widget>,
    ) {
        // The frontend is initiating this call, so we should process events that the
        // Transmogrifier sends first.
        let context = Context::new(channels, frontend);
        let mut received_one_message = true;
        while received_one_message {
            received_one_message = false;
            while let Ok(command) = channels.command_receiver.try_recv() {
                received_one_message = true;
                widget.receive_command(command, &context);
            }

            while let Ok(event) = channels.event_receiver.try_recv() {
                received_one_message = true;
                widget.receive_event(event, &context);
            }

            while let Ok(command) = channels.transmogrifier_command_receiver.try_recv() {
                received_one_message = true;
                self.receive_command(state, command, widget, frontend);
            }
        }
    }

    /// Returns an initialized state using `Self::State::default()`.
    fn default_state_for(
        &self,
        widget: &Self::Widget,
        reference: &WidgetRef<Self::Widget>,
        frontend: &F,
    ) -> TransmogrifierState {
        let state = self.initialize(widget, reference, frontend);
        TransmogrifierState {
            state: Box::new(state),
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

    /// Returns a `Sender` for this widget's [`Command`](crate::Widget::Command)
    /// type.
    #[must_use]
    fn command_sender(&self) -> Box<dyn AnySendSync>;

    /// Returns a `Sender` for this widget's
    /// [`TransmogrifierEvent`](crate::Widget::TransmogrifierEvent) type.
    #[must_use]
    fn event_sender(&self) -> &'_ dyn AnySendSync;
}

/// Generic storage for a transmogrifier.
#[derive(Debug)]
pub struct TransmogrifierState {
    /// The `State` type, stored without its type information.
    pub state: Box<dyn AnySendSync>,
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
    /// The frontend that created this context.
    pub frontend: Box<dyn AnyFrontend>,
    widget: WeakWidgetRegistration,
    command_sender: Sender<W::TransmogrifierCommand>,
    _widget: PhantomData<W>,
}

impl<W: Widget> Context<W> {
    /// Create a new `Context`.
    #[must_use]
    pub fn new<F: Frontend>(channels: &Channels<W>, frontend: &F) -> Self {
        Self {
            widget: channels.widget.clone(),
            command_sender: channels.transmogrifier_command_sender.clone(),
            frontend: Box::new(frontend.clone()),
            _widget: PhantomData::default(),
        }
    }

    /// Send `command` to the transmogrifier.
    pub fn send_command(&self, command: W::TransmogrifierCommand) {
        if let Some(widget) = self.widget.upgrade() {
            drop(self.command_sender.send(command));
            self.frontend
                .storage()
                .set_widget_has_messages(widget.id().clone());
        }
    }
}

impl<W: Widget> Deref for Context<W> {
    type Target = WidgetStorage;

    fn deref(&self) -> &Self::Target {
        self.frontend.storage()
    }
}

/// Communication channels used to communicate between [`Widget`]s and
/// [`Transmogrifier`](crate::Transmogrifier)s.
#[derive(Debug)]
pub struct Channels<W: Widget> {
    widget: WeakWidgetRegistration,
    command_sender: Sender<W::Command>,
    command_receiver: Receiver<W::Command>,
    transmogrifier_command_sender: Sender<W::TransmogrifierCommand>,
    transmogrifier_command_receiver: Receiver<W::TransmogrifierCommand>,
    event_sender: Sender<W::TransmogrifierEvent>,
    event_receiver: Receiver<W::TransmogrifierEvent>,
    _phantom: PhantomData<W>,
}

impl<W: Widget> Clone for Channels<W> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            command_sender: self.command_sender.clone(),
            command_receiver: self.command_receiver.clone(),
            transmogrifier_command_sender: self.transmogrifier_command_sender.clone(),
            transmogrifier_command_receiver: self.transmogrifier_command_receiver.clone(),
            event_sender: self.event_sender.clone(),
            event_receiver: self.event_receiver.clone(),
            _phantom: PhantomData::default(),
        }
    }
}

impl<W: Widget> AnyChannels for Channels<W> {
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn command_sender(&self) -> Box<dyn AnySendSync> {
        Box::new(self.command_sender.clone())
    }

    fn event_sender(&self) -> &'_ dyn AnySendSync {
        &self.event_sender
    }
}

impl<W: Widget> Channels<W> {
    /// Creates a new set of channels for a widget and transmogrifier.
    #[must_use]
    pub fn new(widget: &WidgetRegistration) -> Self {
        let (command_sender, command_receiver) = flume::unbounded();
        let (transmogrifier_command_sender, transmogrifier_command_receiver) = flume::unbounded();
        let (event_sender, event_receiver) = flume::unbounded();

        Self {
            widget: WeakWidgetRegistration::from(widget),
            command_sender,
            command_receiver,
            transmogrifier_command_sender,
            transmogrifier_command_receiver,
            event_sender,
            event_receiver,
            _phantom: PhantomData::default(),
        }
    }

    /// Sends an event to the [`Widget`].
    pub fn post_event(&self, event: W::TransmogrifierEvent) {
        if let Some(registration) = self.widget() {
            drop(self.event_sender.send(event));
            registration.set_has_messages();
        }
    }

    /// Sends a `command` to the [`Widget`].
    pub fn post_command(&self, command: W::Command) {
        if let Some(registration) = self.widget() {
            drop(self.command_sender.send(command));
            registration.set_has_messages();
        }
    }

    /// Returns the widget registration. Returns none if the widget has been
    /// destroyed.
    #[must_use]
    pub fn widget(&self) -> Option<WidgetRegistration> {
        self.widget.upgrade()
    }

    /// Returns the widget reference for this widget. Returns none if the widget
    /// has been destroyed.
    #[must_use]
    pub fn widget_ref<F: Frontend>(&self, frontend: F) -> Option<WidgetRef<W>> {
        self.widget().and_then(|reg| WidgetRef::new(&reg, frontend))
    }
}

/// A callback that receives information `I`, and returns `R`.
pub struct Callback<I = (), R = ()> {
    callback: Option<Box<dyn CallbackFn<I, R>>>,
}

impl<I, R> Debug for Callback<I, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(type_name::<Self>())?;
        f.write_str(" { callback: ")?;
        if self.callback.is_some() {
            f.write_str("Some(_) }")
        } else {
            f.write_str("None }")
        }
    }
}

impl<I, R> Default for Callback<I, R> {
    fn default() -> Self {
        Self { callback: None }
    }
}

/// A callback implementation. Not typically directly implemented, as this trait
/// is auto-implemented for any `Fn(I) -> R` types.
pub trait CallbackFn<I, R>: Send + Sync {
    /// Invoke the callback with `info`.
    fn invoke(&self, info: I) -> R;
}

impl<I, R> Callback<I, R> {
    /// Create a new callback with the provided function.
    pub fn new<C: CallbackFn<I, R> + 'static>(callback: C) -> Self {
        Self {
            callback: Some(Box::new(callback)),
        }
    }

    /// Invoke the callback. If implemented, a the result is returned.
    pub fn invoke(&self, info: I) -> Option<R> {
        self.callback.as_ref().map(|cb| cb.invoke(info))
    }
}

impl<I, R, T> CallbackFn<I, R> for T
where
    T: Fn(I) -> R + Send + Sync,
{
    fn invoke(&self, info: I) -> R {
        self(info)
    }
}
