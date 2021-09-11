use std::{
    any::{type_name, Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
    ops::Deref,
    sync::Arc,
    time::Duration,
};

use flume::{Receiver, Sender};
use stylecs::Style;

use crate::{
    styles::{style_sheet::Classes, BackgroundColor, ColorPair, TextColor},
    AnyFrontend, AppContext, Frontend, StyledWidget, UnscheduledTimer, WeakWidgetRegistration,
    WidgetRef, WidgetRegistration, WidgetStorage, Window,
};

mod transmogrifier_context;

pub use transmogrifier_context::{AnyTransmogrifierContext, TransmogrifierContext};

/// A graphical user interface element.
pub trait Widget: Debug + Send + Sync + Sized + 'static {
    /// Widgets may need to communicate with transmogrifier implementations.
    /// This type is the type that can be sent to a transmogrifier.
    type Command: Debug + Send + Sync;

    /// The type of the event that any [`Transmogrifier`] for this widget to
    /// use.
    type Event: Debug + Send + Sync;

    /// The unique class name for this widget. Must not conflict with any other
    /// widgets in use. Widget authors should prefix their widget names to
    /// ensure no conflicts. For example, the `gooey-widgets` crate prefixes all
    /// of the `CLASS` constants with `gooey-`.
    const CLASS: &'static str;

    /// Returns all classes that apply styles for this widget.
    #[must_use]
    fn classes() -> Classes {
        Classes::from(Self::CLASS)
    }

    /// When true, the control is able to receive focus through focus
    /// advancement, most commonly done when using the tab key.
    const FOCUSABLE: bool;

    /// Called when an `event` from the transmogrifier was received.
    #[allow(unused_variables)]
    fn receive_event(&mut self, event: Self::Event, context: &Context<Self>) {
        unimplemented!(
            "an event `{:?}` was sent by the transmogrifier but receive_event isn't implemented \
             by {}",
            event,
            type_name::<Self>()
        )
    }

    /// Returns the effective text color for the given style.
    #[must_use]
    fn text_color(style: &Style) -> Option<&ColorPair> {
        style.get_with_fallback::<TextColor>()
    }

    /// Returns the effective background color for the given style.
    #[must_use]
    fn background_color(style: &Style) -> Option<&ColorPair> {
        style.get_with_fallback::<BackgroundColor>()
    }

    /// Invokes `with_fn` with the widget `widget_id` and a `Context`. Returns the
    /// result.
    ///
    /// Returns None if `OW` does not match the type of the widget contained.
    fn map<OW: Widget, F: FnOnce(&OW, &Context<OW>) -> R, R>(
        widget_id: &WidgetId,
        context: &Context<Self>,
        with_fn: F,
    ) -> Option<R> {
        context.map_widget(widget_id, with_fn)
    }

    /// Invokes `with_fn` with the widget `widget_id` and a `Context`. Returns the
    /// result.
    ///
    /// Returns None if `OW` does not match the type of the widget contained.
    fn map_mut<OW: Widget, F: FnOnce(&mut OW, &Context<OW>) -> R, R>(
        widget_id: &WidgetId,
        context: &Context<Self>,
        with_fn: F,
    ) -> Option<R> {
        context.map_widget_mut(widget_id, with_fn)
    }
}

/// A widget that can be created with defaults.
pub trait DefaultWidget: Widget {
    /// Returns a default widget.
    fn default_for(storage: &WidgetStorage) -> StyledWidget<Self>;
}

impl<T: Widget + Default> DefaultWidget for T {
    fn default_for(_storage: &WidgetStorage) -> StyledWidget<Self> {
        StyledWidget::default()
    }
}

/// A unique ID of a widget, with information about the widget type.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct WidgetId {
    /// The unique id of the widget.
    pub id: u32,
    /// The [`TypeId`] of the [`Widget`] type.
    pub type_id: TypeId,
    /// The type name of the widget.
    pub type_name: &'static str,
}

/// Transforms a Widget into whatever is needed for [`Frontend`] `F`.
pub trait Transmogrifier<F: Frontend>: Debug + Sized {
    /// The type of the widget being transmogrified.
    type Widget: Widget;
    /// The type the storage this transmogrifier uses for state.
    type State: Default + Debug + Any + Send + Sync;

    /// Called after a transmogrifier is initialized.
    #[allow(unused_variables)]
    fn initialize(
        &self,
        widget: &mut Self::Widget,
        reference: &WidgetRef<Self::Widget>,
        frontend: &F,
    ) -> Self::State {
        <Self::State as Default>::default()
    }

    /// Called when a command is received from the widget.
    #[allow(unused_variables)] // Keeps documentation clean
    fn receive_command(
        &self,
        command: <Self::Widget as Widget>::Command,
        context: &mut TransmogrifierContext<'_, Self, F>,
    ) {
        unimplemented!(
            "{} tried to send a command, but the transmogrifier wasn't expecting one",
            type_name::<Self>()
        )
    }

    /// Processes commands and events for this widget and transmogrifier.
    fn process_messages(&self, mut transmogrifier_context: TransmogrifierContext<'_, Self, F>) {
        // The frontend is initiating this call, so we should process events that the
        // Transmogrifier sends first.
        let context = Context::from(&transmogrifier_context);
        let mut received_one_message = true;
        while received_one_message {
            received_one_message = false;
            while let Ok(event) = transmogrifier_context.channels.event_receiver.try_recv() {
                received_one_message = true;
                transmogrifier_context.widget.receive_event(event, &context);
            }

            while let Ok(command) = transmogrifier_context.channels.command_receiver.try_recv() {
                received_one_message = true;
                self.receive_command(command, &mut transmogrifier_context);
            }
        }
    }

    /// Returns an initialized state using `Self::State::default()`.
    fn default_state_for(
        &self,
        widget: &mut Self::Widget,
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
    /// [`Event`](crate::Widget::Event) type.
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
#[derive(Debug)]
pub struct Context<W: Widget> {
    /// The frontend that created this context.
    frontend: Box<dyn AnyFrontend>,
    widget: WeakWidgetRegistration,
    command_sender: Sender<W::Command>,
    _widget: PhantomData<W>,
}

impl<W: Widget> Clone for Context<W> {
    fn clone(&self) -> Self {
        Self {
            frontend: self.frontend.cloned(),
            widget: self.widget.clone(),
            command_sender: self.command_sender.clone(),
            _widget: PhantomData::default(),
        }
    }
}

impl<W: Widget> Context<W> {
    /// Create a new `Context`.
    #[must_use]
    pub fn new(channels: &Channels<W>, frontend: &dyn AnyFrontend) -> Self {
        Self {
            widget: channels.widget.clone(),
            command_sender: channels.command_sender.clone(),
            frontend: frontend.cloned(),
            _widget: PhantomData::default(),
        }
    }

    /// Returns the current application context.
    #[must_use]
    pub fn app(&self) -> &AppContext {
        self.frontend.storage().app()
    }

    /// Returns the window for this context.
    #[must_use]
    pub fn window(&self) -> Option<&dyn Window> {
        self.frontend.window()
    }

    /// Send `command` to the transmogrifier.
    pub fn send_command(&self, command: W::Command) {
        if let Some(widget) = self.widget.upgrade() {
            drop(self.command_sender.send(command));
            self.frontend.set_widget_has_messages(widget.id().clone());
        }
    }

    /// Send `command` to the widget.
    pub fn send_command_to<OW: Widget>(&self, widget: &WidgetId, command: OW::Command) {
        if let Some(state) = self.widget_state(widget) {
            let channels = state.channels::<OW>().expect("incorrect widget type");
            channels.post_command(command);
            self.frontend.set_widget_has_messages(widget.clone());
        }
    }

    /// Returns the registration of the widget that this context is for.
    #[must_use]
    pub fn widget(&self) -> WidgetRef<W> {
        WidgetRef::from_weak_registration(self.widget.clone(), self.frontend.cloned())
    }

    /// Returns the registration of the widget that this context is for.
    #[must_use]
    pub fn registration(&self) -> &'_ WeakWidgetRegistration {
        &self.widget
    }

    /// Maps the widget of this context.
    pub fn map<F: FnOnce(&W, &Self) -> R, R>(&self, map: F) -> Option<R> {
        self.registration()
            .upgrade()
            .and_then(|registration| self.map_widget(registration.id(), map))
    }

    /// Maps the widget of this context with mutability.
    pub fn map_mut<F: FnOnce(&mut W, &Self) -> R, R>(&self, map: F) -> Option<R> {
        self.registration()
            .upgrade()
            .and_then(|registration| self.map_widget_mut(registration.id(), map))
    }

    /// Invokes `with_fn` with the widget `widget_id` and a `Context`. Returns the
    /// result.
    ///
    /// Returns None if `OW` does not match the type of the widget contained.
    pub fn map_widget<OW: Widget, F: FnOnce(&OW, &Context<OW>) -> R, R>(
        &self,
        widget: &WidgetId,
        with_fn: F,
    ) -> Option<R> {
        self.widget_state(widget)
            .and_then(|state| state.with_widget(self.frontend.as_ref(), with_fn))
    }

    /// Invokes `with_fn` with the widget `widget_id` and a `Context`. Returns the
    /// result.
    ///
    /// Returns None if `OW` does not match the type of the widget contained.
    pub fn map_widget_mut<OW: Widget, F: FnOnce(&mut OW, &Context<OW>) -> R, R>(
        &self,
        widget_id: &WidgetId,
        with_fn: F,
    ) -> Option<R> {
        self.widget_state(widget_id)
            .and_then(|state| state.with_widget_mut(self.frontend.as_ref(), with_fn))
    }

    /// Returns an unscheduled timer that will invoke `callback` once after `period` elapses.
    pub fn timer(&self, period: Duration, callback: Callback) -> UnscheduledTimer<'_> {
        UnscheduledTimer::new(period, callback, self.frontend.as_ref())
    }

    /// Returns the frontend that created this context.
    #[must_use]
    pub fn frontend(&self) -> &dyn AnyFrontend {
        self.frontend.as_ref()
    }
}

impl<'a, 't, W: Widget, T: Transmogrifier<F, Widget = W>, F: Frontend>
    From<&'a TransmogrifierContext<'t, T, F>> for Context<W>
{
    fn from(context: &'a TransmogrifierContext<'t, T, F>) -> Self {
        Self::new(context.channels, context.frontend)
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
    event_sender: Sender<W::Event>,
    event_receiver: Receiver<W::Event>,
    _phantom: PhantomData<W>,
}

impl<W: Widget> Clone for Channels<W> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            command_sender: self.command_sender.clone(),
            command_receiver: self.command_receiver.clone(),
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
        let (event_sender, event_receiver) = flume::unbounded();

        Self {
            widget: WeakWidgetRegistration::from(widget),
            command_sender,
            command_receiver,
            event_sender,
            event_receiver,
            _phantom: PhantomData::default(),
        }
    }

    /// Sends an event to the [`Widget`].
    pub fn post_event(&self, event: W::Event) {
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
    callback: Option<Arc<dyn CallbackFn<I, R>>>,
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

impl<I, R> Clone for Callback<I, R> {
    fn clone(&self) -> Self {
        Self {
            callback: self.callback.clone(),
        }
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
            callback: Some(Arc::new(callback)),
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

/// Provides the standard interface for all builder-pattern builders in Gooey.
pub trait Builder {
    /// The built type.
    type Output;
    /// Finish building and return the built result.
    fn finish(self) -> Self::Output;
}
