use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    collections::{HashMap, HashSet},
    convert::Infallible,
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, RwLock, Weak,
    },
};

use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use stylecs::{Style, StyleComponent};
use unic_langid::LanguageIdentifier;

use crate::{
    styles::style_sheet::StyleSheet, AnyChannels, AnyFrontend, AnySendSync, AnyTransmogrifier,
    AnyTransmogrifierContext, AnyWidget, Channels, Context, Frontend, ManagedCodeGuard,
    TransmogrifierState, Widget, WidgetId,
};

type WidgetTypeId = TypeId;

#[derive(Clone, Debug)]
/// A graphical user interface.
pub struct Gooey<F: Frontend> {
    data: Arc<GooeyData<F>>,
}

#[derive(Debug)]
struct GooeyData<F: Frontend> {
    transmogrifiers: Arc<Transmogrifiers<F>>,
    root: WidgetRegistration,
    storage: WidgetStorage,
    processing_messages_lock: Mutex<()>,
    inside_event_loop: AtomicU32,
}

impl<F: Frontend> Gooey<F> {
    /// Creates a new instance
    #[must_use]
    pub fn new(
        transmogrifiers: Arc<Transmogrifiers<F>>,
        root: WidgetRegistration,
        storage: WidgetStorage,
    ) -> Self {
        Self {
            data: Arc::new(GooeyData {
                transmogrifiers,
                storage,
                root,
                processing_messages_lock: Mutex::default(),
                inside_event_loop: AtomicU32::default(),
            }),
        }
    }

    /// Returns the root widget.
    pub fn root_widget(&self) -> &WidgetRegistration {
        &self.data.root
    }

    /// Returns the transmogrifier fo the type id
    #[must_use]
    pub fn transmogrifier_for_type_id(
        &self,
        widget_type_id: TypeId,
    ) -> Option<&'_ <F as Frontend>::AnyTransmogrifier> {
        self.data.transmogrifiers.map.get(&widget_type_id)
    }

    /// Executes `callback` with the transmogrifier and transmogrifier state as
    /// parameters.
    #[allow(clippy::missing_panics_doc)]
    pub fn with_transmogrifier<
        R,
        C: FnOnce(&'_ <F as Frontend>::AnyTransmogrifier, AnyTransmogrifierContext<'_, F>) -> R,
    >(
        &self,
        widget_id: &WidgetId,
        frontend: &F,
        callback: C,
    ) -> Option<R> {
        let transmogrifier = match self.data.transmogrifiers.map.get(&widget_id.type_id) {
            Some(transmogrifer) => transmogrifer,
            None => panic!("No transmogrifier registered for {}", widget_id.type_name),
        };

        let widget_state = self.widget_state(widget_id)?;
        widget_state.with_state(transmogrifier, frontend, |state, widget| {
            let style = widget_state.style.lock();
            let channels = widget_state.channels.as_ref();
            callback(
                transmogrifier,
                AnyTransmogrifierContext::new(
                    // unwrap is guranteed because this block can't
                    // execute unless the widget registration is still
                    // alive.
                    widget_state.id.upgrade().unwrap(),
                    state,
                    frontend,
                    widget,
                    channels,
                    &style,
                    &frontend.ui_state_for(widget_id),
                ),
            )
        })
    }

    /// Executes `callback` with the transmogrifier and transmogrifier state as
    /// parameters.
    #[allow(clippy::missing_panics_doc)]
    pub fn for_each_widget<
        C: FnMut(&'_ <F as Frontend>::AnyTransmogrifier, &mut dyn AnySendSync, &dyn AnyWidget),
    >(
        &self,
        frontend: &F,
        mut callback: C,
    ) {
        self.data.storage.for_each_widget(|widget_state| {
            // unwrap is guaranteed here because each widget that has been
            // inserted already has interacted with the transmogrifier.
            let transmogrifier = self
                .data
                .transmogrifiers
                .map
                .get(&widget_state.registration().unwrap().id().type_id)
                .unwrap();
            widget_state.with_state(transmogrifier, frontend, |transmogrifier_state, widget| {
                callback(transmogrifier, transmogrifier_state, widget);
            });
        });
    }

    /// Loops over each widget processing its events first, then its commands.
    ///
    /// # Panics
    ///
    /// Panics upon internal locking errors.
    pub fn process_widget_messages(&self, frontend: &F) {
        // If a widget calls code that posts more messages, we don't want this
        // code to re-enter. Instead, the first process_widget_message will loop
        // to catch new widgets that have messages.
        let _guard = match self.data.processing_messages_lock.try_lock() {
            Some(guard) => guard,
            None => return,
        };

        let mut managed_code_guard = self.enter_managed_code(frontend);
        managed_code_guard.allow_process_messages = false;

        // This method will return early if there are no widgets with pending
        // messages. `chatter_limit` is put in place to limit cross-talk betwen
        // widgets. It's possible for one widget to invoke behavior that
        // triggers a message in another widget, which then returns the favor.
        // 10 is an arbitrary number.
        let mut chatter_limit = 10;
        while chatter_limit > 0 {
            chatter_limit -= 1;
            let widgets_with_messages = {
                let mut widgets_with_messages = self.data.storage.data.widgets_with_messages.lock();
                if widgets_with_messages.is_empty() {
                    break;
                }

                std::mem::take(&mut *widgets_with_messages)
            };

            for widget_id in widgets_with_messages {
                self.with_transmogrifier(&widget_id, frontend, |transmogrifier, context| {
                    transmogrifier.process_messages(context);
                });
            }
        }
    }

    pub(crate) fn post_transmogrifier_event<W: Widget>(
        &self,
        widget_id: &WidgetId,
        event: <W as Widget>::Event,
        frontend: &F,
    ) {
        if let Some(state) = self.widget_state(widget_id) {
            let channels = state.channels::<W>().unwrap();
            channels.post_event(event);
            frontend.set_widget_has_messages(widget_id.clone());
        }

        // Process any messages that may have been triggered onto other widgets.
        frontend.gooey().process_widget_messages(frontend);
    }

    pub(crate) fn post_command<W: Widget>(
        &self,
        widget_id: &WidgetId,
        event: <W as Widget>::Command,
        frontend: &F,
    ) {
        if let Some(state) = self.widget_state(widget_id) {
            let channels = state.channels::<W>().unwrap();
            channels.post_command(event);
            frontend.set_widget_has_messages(widget_id.clone());
        }

        // Process any messages that may have been triggered onto other widgets.
        frontend.gooey().process_widget_messages(frontend);
    }

    /// Returns the root widget.
    #[must_use]
    pub fn stylesheet(&self) -> &StyleSheet {
        self.app().stylesheet()
    }

    /// Enters a region of managed code. Automatically exits the region when the returned guard is dropped.
    ///
    /// When the last managed code region is exited, widget messages are processed before returning.
    #[must_use]
    pub fn enter_managed_code(&self, frontend: &F) -> ManagedCodeGuard {
        self.data.inside_event_loop.fetch_add(1, Ordering::SeqCst);
        ManagedCodeGuard {
            frontend: Box::new(frontend.clone()),
            allow_process_messages: true,
        }
    }

    pub(crate) fn exit_managed_code(&self, frontend: &F, allow_process_messages: bool) {
        let previous_value = self.data.inside_event_loop.fetch_sub(1, Ordering::SeqCst);
        if previous_value == 1 && allow_process_messages {
            self.process_widget_messages(frontend);
        }
    }

    /// Returns whether `Gooey` managed code is currently executing.
    #[must_use]
    pub fn is_managed_code(&self) -> bool {
        self.data.inside_event_loop.load(Ordering::SeqCst) > 0
    }

    /// Localizes `key` with `parameters`.
    #[must_use]
    pub fn localize<'a>(
        &self,
        key: &str,
        parameters: impl Into<Option<LocalizationParameters<'a>>>,
    ) -> String {
        self.data.storage.localize(key, parameters)
    }
}

impl<F: Frontend> Deref for Gooey<F> {
    type Target = WidgetStorage;

    fn deref(&self) -> &Self::Target {
        &self.data.storage
    }
}

/// A collection of transmogrifiers to use inside of a frontend.
#[derive(Debug)]
pub struct Transmogrifiers<F: Frontend> {
    map: HashMap<WidgetTypeId, <F as Frontend>::AnyTransmogrifier>,
    _phantom: PhantomData<F>,
}

impl<F: Frontend> Transmogrifiers<F> {
    /// Registers a transmogrifier.
    ///
    /// # Errors
    ///
    /// If an existing transmogrifier is already registered, the transmogrifier
    /// is returned in `Err()`.
    pub fn register_transmogrifier<T: Into<<F as Frontend>::AnyTransmogrifier>>(
        &mut self,
        transmogrifier: T,
    ) -> Result<(), <F as Frontend>::AnyTransmogrifier> {
        let transmogrifier = transmogrifier.into();
        let type_id = <<F as Frontend>::AnyTransmogrifier as AnyTransmogrifier<F>>::widget_type_id(
            &transmogrifier,
        );
        if self.map.contains_key(&type_id) {
            return Err(transmogrifier);
        }

        self.map.insert(type_id, transmogrifier);

        Ok(())
    }
}

impl<F: Frontend> Default for Transmogrifiers<F> {
    fn default() -> Self {
        Self {
            map: HashMap::default(),
            _phantom: PhantomData::default(),
        }
    }
}

/// Generic-type-less widget storage.
#[derive(Clone, Debug)]
pub struct WidgetStorage {
    data: Arc<WidgetStorageData>,
    app: AppContext,
}

#[derive(Default, Debug)]
struct WidgetStorageData {
    widget_id_generator: AtomicU32,
    state: RwLock<HashMap<u32, Option<WidgetState>>>,
    widgets_with_messages: Mutex<HashSet<WidgetId>>,
}

impl WidgetStorage {
    /// Returns a new instance for the window provided.
    #[must_use]
    pub fn new(window: AppContext) -> Self {
        Self {
            data: Arc::default(),
            app: window,
        }
    }

    /// Returns the application.
    #[must_use]
    pub fn app(&self) -> &AppContext {
        &self.app
    }

    /// Register a widget with storage.
    #[allow(clippy::missing_panics_doc)] // The unwrap is unreachable
    pub fn register<W: Widget + AnyWidget>(
        &self,
        styled_widget: StyledWidget<W>,
    ) -> WidgetRegistration {
        let StyledWidget {
            widget,
            style,
            registration,
        } = styled_widget;
        let registration = registration.unwrap_or_else(|| self.allocate::<W>());
        let mut state = self.data.state.write().unwrap();
        let state = state.entry(registration.id().id).or_default();
        assert!(state.is_none(), "widget id in use");

        *state = Some(WidgetState::new(widget, style, &registration));
        registration
    }

    /// Allocates a widget registration. This allows obtaining a registration
    /// before the widget has been created, allowing for a parent widget to pass
    /// its id to its children during widget construction. Attempting to
    /// interact with this widget before calling `register` with the created
    /// widget will panic.
    ///
    /// # Panics
    ///
    /// Panics on internal locking failures.
    pub fn allocate<W: Widget + AnyWidget>(&self) -> WidgetRegistration {
        loop {
            let next_id = self.data.widget_id_generator.fetch_add(1, Ordering::AcqRel);
            // Insert None if the slot is free, which is most likely the case.
            // If it performs the insert, is_new is flagged and the id is
            // returned. If not, the loop repeats until a free entry is found.
            let mut widget_registration = None;
            let mut state = self.data.state.write().unwrap();
            state.entry(next_id).or_insert_with(|| {
                let reg = WidgetRegistration::new::<W>(next_id, self);
                widget_registration = Some(reg);
                None
            });
            if let Some(widget_registration) = widget_registration {
                return widget_registration;
            }
        }
    }

    pub(crate) fn unregister_id(&self, widget_id: u32) {
        // To prevent a deadlock if a widget is being removed that contains
        // other widgets, we need to let drop happen outside of the lock
        // acquisition.
        let removed_value = {
            let mut state = self.data.state.write().unwrap();
            state.remove(&widget_id)
        };
        drop(removed_value);
    }

    /// Returns the state of the widget with id `widget_id`.
    ///
    /// # Panics
    ///
    /// Panics if internal lock handling results in an error.
    #[must_use]
    pub fn widget_state(&self, widget_id: &WidgetId) -> Option<WidgetState> {
        let state = self.data.state.read().unwrap();
        state.get(&widget_id.id).cloned().flatten()
    }

    /// Executes `callback` with the widget state parameters.
    ///
    /// # Panics
    ///
    /// Panics if internal lock handling results in an error.
    pub fn for_each_widget<C: FnMut(WidgetState)>(&self, mut callback: C) {
        let widgets = {
            let state = self.data.state.read().unwrap();
            state.values().cloned().collect::<Vec<_>>()
        };

        for widget in widgets.into_iter().flatten() {
            callback(widget);
        }
    }

    /// Marks a widget as having messages. If this isn't set, pending messages
    /// will not be received.
    ///
    /// # Panics
    ///
    /// Panics if internal lock handling results in an error.
    pub fn set_widget_has_messages(&self, widget: WidgetId) {
        let mut statuses = self.data.widgets_with_messages.lock();
        statuses.insert(widget);
    }

    /// Returns the application context for this interface.
    #[must_use]
    pub const fn app(&self) -> &AppContext {
        &self.app
    }

    /// Localizes `key` with `parameters`.
    #[must_use]
    pub fn localize<'a>(
        &self,
        key: &str,
        parameters: impl Into<Option<LocalizationParameters<'a>>>,
    ) -> String {
        self.app.localize(key, parameters)
    }
}

/// A type that registers widgets with an associated key.
pub trait KeyedStorage<K: Key>: Debug + Send + Sync + 'static {
    /// Register `styled_widget` with `key`.
    fn register<W: Widget + AnyWidget>(
        &mut self,
        key: impl Into<Option<K>>,
        styled_widget: StyledWidget<W>,
    ) -> WidgetRegistration;

    /// Returns the underlying widget storage.
    fn storage(&self) -> &WidgetStorage;

    /// If this storage is representing a component, this returns a weak
    /// registration that can be used to communicate with it.
    fn related_storage(&self) -> Option<Box<dyn RelatedStorage<K>>>;
}

/// A key for a widget.
pub trait Key: Clone + Hash + Debug + Eq + PartialEq + Send + Sync + 'static {}

impl<T> Key for T where T: Clone + Hash + Debug + Eq + PartialEq + Send + Sync + 'static {}

impl<K: Key> KeyedStorage<K> for WidgetStorage {
    fn register<W: Widget + AnyWidget>(
        &mut self,
        _key: impl Into<Option<K>>,
        styled_widget: StyledWidget<W>,
    ) -> WidgetRegistration {
        Self::register(self, styled_widget)
    }

    fn storage(&self) -> &WidgetStorage {
        self
    }

    fn related_storage(&self) -> Option<Box<dyn RelatedStorage<K>>> {
        None
    }
}

/// Related storage enables a widget to communicate in a limited way about
/// widgets being inserted or removed.
pub trait RelatedStorage<K: Key>: Debug + Send + Sync + 'static {
    /// Returns the registration of the widget that this is from.
    fn widget(&self) -> WeakWidgetRegistration;
    /// Removes the widget with `key` from this storage. Returns the removed registration if one was removed.
    fn remove(&self, key: &K) -> Option<WeakWidgetRegistration>;
    /// Registers `widget` with `key`.
    fn register(&self, key: K, widget: &WidgetRegistration);
}

/// A widget and its initial style information.
#[derive(Debug)]
#[must_use]
pub struct StyledWidget<W: Widget> {
    /// The widget.
    pub widget: W,
    /// The style information.
    pub style: Style,
    /// The pre-allocated registration, if any. See [`WidgetStorage::allocate()`] for more information.
    pub registration: Option<WidgetRegistration>,
}

impl<W: Widget + Default> Default for StyledWidget<W> {
    fn default() -> Self {
        Self {
            widget: W::default(),
            style: Style::default(),
            registration: None,
        }
    }
}

impl<W: Widget + From<WidgetRegistration>> From<WidgetRegistration> for StyledWidget<W> {
    fn from(widget: WidgetRegistration) -> Self {
        Self::from(W::from(widget))
    }
}

impl<W: Widget> From<W> for StyledWidget<W> {
    fn from(widget: W) -> Self {
        Self {
            widget,
            style: Style::default(),
            registration: None,
        }
    }
}

impl<W: Widget> StyledWidget<W> {
    /// Returns a new instance.
    pub fn new(widget: W, style: Style, registration: Option<WidgetRegistration>) -> Self {
        Self {
            widget,
            style,
            registration,
        }
    }

    /// Adds `component` to `style` and returns self.
    pub fn with<C: StyleComponent + Clone>(mut self, component: C) -> Self {
        self.style.push(component);
        self
    }
}

/// Generic, clone-able storage for a widget's transmogrifier.
#[derive(Clone, Debug)]
pub struct WidgetState {
    /// The id of the widget.
    pub id: WeakWidgetRegistration,
    /// The widget.
    pub widget: Arc<Mutex<Box<dyn AnyWidget>>>,
    /// The transmogrifier state.
    pub state: Arc<Mutex<Option<TransmogrifierState>>>,

    /// The channels to communicate with the widget.
    pub channels: Arc<dyn AnyChannels>,

    /// The widget's style.
    pub style: Arc<Mutex<Style>>,
}

impl WidgetState {
    /// Initializes a new widget state with `widget`, `id`, and `None` for the
    /// transmogrifier state.
    pub fn new<W: Widget + AnyWidget>(widget: W, style: Style, id: &WidgetRegistration) -> Self {
        Self {
            id: WeakWidgetRegistration::from(id),
            widget: Arc::new(Mutex::new(Box::new(widget))),
            style: Arc::new(Mutex::new(style)),
            state: Arc::default(),
            channels: Arc::new(Channels::<W>::new(id)),
        }
    }

    /// Gets the registration for this widget. Returns None if the widget has
    /// been destroyed.
    #[must_use]
    pub fn registration(&self) -> Option<WidgetRegistration> {
        self.id.upgrade()
    }

    /// Returns the state for this widget. If this is the first call, the state
    /// is initialized with the `Default::default()` implementation for the
    /// `State` associated type on [`Transmogrifier`](crate::Transmogrifier).
    ///
    /// # Panics
    ///
    /// Panics if internal lock poisoning occurs.
    pub fn with_state<
        R,
        T: AnyTransmogrifier<F>,
        F: Frontend,
        C: FnOnce(&mut dyn AnySendSync, &mut dyn AnyWidget) -> R,
    >(
        &self,
        transmogrifier: &T,
        frontend: &F,
        callback: C,
    ) -> Option<R> {
        let mut state = self.state.lock();
        let mut widget = self.widget.lock();
        self.id.upgrade().map(|id| {
            let state = state.get_or_insert_with(|| {
                {
                    let style = self.style.lock();
                    frontend.widget_initialized(id.id(), &style);
                }

                transmogrifier.default_state_for(widget.as_mut(), &id, frontend)
            });

            callback(state.state.as_mut(), widget.as_mut())
        })
    }

    #[must_use]
    pub(crate) fn any_channels(&self) -> &'_ dyn AnyChannels {
        self.channels.as_ref()
    }

    /// Returns the channels used to communicate with this widget.
    #[must_use]
    pub fn channels<W: Widget>(&self) -> Option<&'_ Channels<W>> {
        self.any_channels().as_any().downcast_ref()
    }

    /// Invokes `with_fn` with this `Widget` and a `Context`. Returns the
    /// result.
    ///
    /// Returns None if `W` does not match the type of the widget contained.
    pub fn with_widget<W: Widget, F: FnOnce(&W, &Context<W>) -> R, R>(
        &self,
        frontend: &dyn AnyFrontend,
        with_fn: F,
    ) -> Option<R> {
        let _guard = frontend.enter_managed_code();
        let result = {
            let widget = self.widget.lock();
            let channels = self.channels::<W>()?;
            let context = Context::new(channels, frontend);
            Some(with_fn(widget.as_ref().as_any().downcast_ref()?, &context))
        };
        result
    }

    /// Invokes `with_fn` with this `Widget` and a `Context`. Returns the
    /// result.
    ///
    /// Returns None if `W` does not match the type of the widget contained.
    pub fn with_widget_mut<OW: Widget, F: FnOnce(&mut OW, &Context<OW>) -> R, R>(
        &self,
        frontend: &dyn AnyFrontend,
        with_fn: F,
    ) -> Option<R> {
        let _guard = frontend.enter_managed_code();
        let result = {
            let mut widget = self.widget.lock();
            let channels = self.channels::<OW>()?;
            let context = Context::new(channels, frontend);
            Some(with_fn(
                widget.as_mut().as_mut_any().downcast_mut()?,
                &context,
            ))
        };
        result
    }

    /// Returns a [`WidgetGuard`] for this widget. Returns `None` if `OW` is the wrong type.
    pub fn lock<'a, OW: Widget>(
        &'a self,
        frontend: &dyn AnyFrontend,
    ) -> Option<WidgetGuard<'a, OW>> {
        let widget = self.widget.lock();
        let channels = self.channels::<OW>()?;
        let context = Context::new(channels, frontend);
        let widget =
            MutexGuard::try_map(widget, |widget| widget.as_mut().as_mut_any().downcast_mut())
                .ok()?;
        Some(WidgetGuard::new(widget, context))
    }
}

/// A locked widget reference. No other threads can operate on the widget while
/// this value is alive.
pub struct WidgetGuard<'a, W: Widget> {
    /// The locked widget.
    pub widget: MappedMutexGuard<'a, W>,
    /// The context that can be used to call methods on `widget`.
    pub context: Context<W>,

    _managed_code_guard: ManagedCodeGuard,
}

impl<'a, W: Widget> WidgetGuard<'a, W> {
    pub(crate) fn new(widget: MappedMutexGuard<'a, W>, context: Context<W>) -> Self {
        // While the guard is active, we're considered in managed code.
        let managed_code_guard = context.frontend().enter_managed_code();
        Self {
            widget,
            context,
            _managed_code_guard: managed_code_guard,
        }
    }
}

/// References an initialized widget. On drop, frees the storage and id.
#[derive(Clone, Debug)]
#[must_use]
pub struct WidgetRegistration {
    data: Arc<WidgetRegistrationData>,
}

/// References an initialized widget. These references will not keep a widget
/// from being removed.
#[derive(Clone, Debug)]
pub struct WeakWidgetRegistration {
    data: Weak<WidgetRegistrationData>,
}

impl WeakWidgetRegistration {
    /// Attempt to convert this weak widget registration into a strong one.
    #[must_use]
    pub fn upgrade(&self) -> Option<WidgetRegistration> {
        self.data.upgrade().map(|data| WidgetRegistration { data })
    }
}

impl From<&WidgetRegistration> for WeakWidgetRegistration {
    fn from(reg: &WidgetRegistration) -> Self {
        Self {
            data: Arc::downgrade(&reg.data),
        }
    }
}

#[derive(Debug)]
struct WidgetRegistrationData {
    id: WidgetId,
    storage: WidgetStorage,
}

impl WidgetRegistration {
    pub(crate) fn new<W: Widget>(id: u32, storage: &WidgetStorage) -> Self {
        Self {
            data: Arc::new(WidgetRegistrationData {
                id: WidgetId {
                    id,
                    type_id: TypeId::of::<W>(),
                    type_name: type_name::<W>(),
                },
                storage: storage.clone(),
            }),
        }
    }

    /// Returns the unique ID of this widget. IDs are unique per `Gooey`
    /// instance, not across the entire executable.
    #[must_use]
    pub fn id(&self) -> &'_ WidgetId {
        &self.data.id
    }

    /// Sets that this widget has messages. Should not be necessary in normal
    /// usage patterns. This is only needed if you're directly calling send on a
    /// widget's channels.
    pub fn set_has_messages(&self) {
        self.data
            .storage
            .set_widget_has_messages(self.data.id.clone());
    }
}

impl Drop for WidgetRegistrationData {
    fn drop(&mut self) {
        self.storage.unregister_id(self.id.id);
    }
}

/// A widget reference. Does not prevent a widget from being destroyed if
/// removed from an interface.
#[derive(Debug)]
pub struct WidgetRef<W: Widget> {
    registration: WeakWidgetRegistration,
    frontend: Box<dyn AnyFrontend>,
    _phantom: PhantomData<W>,
}

impl<W: Widget> Clone for WidgetRef<W> {
    fn clone(&self) -> Self {
        Self {
            registration: self.registration.clone(),
            frontend: self.frontend.cloned(),
            _phantom: PhantomData::default(),
        }
    }
}

impl<W: Widget> WidgetRef<W> {
    /// Creates a new reference from a [`WidgetRegistration`]. Returns None if
    /// the `W` type doesn't match the type of the widget.
    #[must_use]
    pub fn new<F: Frontend>(registration: &WidgetRegistration, frontend: F) -> Option<Self> {
        if registration.id().type_id == TypeId::of::<W>() {
            Some(Self {
                registration: WeakWidgetRegistration::from(registration),
                frontend: Box::new(frontend),
                _phantom: PhantomData::default(),
            })
        } else {
            None
        }
    }

    /// Creates a new reference from a [`WidgetRegistration`]. Returns None if
    /// the `W` type doesn't match the type of the widget.
    #[must_use]
    pub fn new_with_any_frontend(
        registration: &WidgetRegistration,
        frontend: Box<dyn AnyFrontend>,
    ) -> Option<Self> {
        if registration.id().type_id == TypeId::of::<W>() {
            Some(Self::from_weak_registration(
                WeakWidgetRegistration::from(registration),
                frontend,
            ))
        } else {
            None
        }
    }

    /// Creates a new reference from a [`WidgetRegistration`]. Returns None if
    /// the `W` type doesn't match the type of the widget.
    #[must_use]
    pub fn from_weak_registration(
        registration: WeakWidgetRegistration,
        frontend: Box<dyn AnyFrontend>,
    ) -> Self {
        Self {
            registration,
            frontend,
            _phantom: PhantomData::default(),
        }
    }

    /// Returns the registration record. Returns None if the widget has been
    /// removed from the interface.
    #[must_use]
    pub fn registration(&self) -> Option<WidgetRegistration> {
        self.registration.upgrade()
    }

    /// Posts `event` to a transmogrifier.
    ///
    /// # Panics
    ///
    /// Panics if `F` is not the type of the frontend in use.
    pub fn post_event<F: Frontend>(&self, event: W::Event) {
        let frontend = self.frontend.as_ref().as_any().downcast_ref::<F>().unwrap();
        let _guard = frontend.enter_managed_code();
        if let Some(registration) = self.registration() {
            frontend
                .gooey()
                .post_transmogrifier_event::<W>(registration.id(), event, frontend);
        }
    }

    /// Posts `event` to a transmogrifier.
    ///
    /// # Panics
    ///
    /// Panics if `F` is not the type of the frontend in use.
    pub fn post_command<F: Frontend>(&self, command: W::Command) {
        let frontend = self.frontend.as_ref().as_any().downcast_ref::<F>().unwrap();
        if let Some(registration) = self.registration() {
            frontend
                .gooey()
                .post_command::<W>(registration.id(), command, frontend);
        }
    }
}

/// A type that provides localization (multi-lingual representations of text).
pub trait Localizer: Send + Sync + Debug + 'static {
    /// Localizes `key` with `parameters` and returns a string in the user's
    /// preferred locale.
    fn localize<'a>(
        &self,
        key: &str,
        parameters: Option<LocalizationParameters<'a>>,
        language: &LanguageIdentifier,
    ) -> String;
}

impl Localizer for () {
    /// Returns `key.to_string()`, ignoring the remaining parameters.
    fn localize<'a>(
        &self,
        key: &str,
        _parameters: Option<LocalizationParameters<'a>>,
        _language: &LanguageIdentifier,
    ) -> String {
        key.to_string()
    }
}

/// A context used during initialization of a window or application.
#[derive(Debug, Clone)]
pub struct AppContext {
    stylesheet: Arc<StyleSheet>,
    localizer: Arc<dyn Localizer>,
    language: Arc<RwLock<LanguageIdentifier>>,
}

impl AppContext {
    /// Returns a new context with the language and localizer provided.
    #[must_use]
    pub fn new(
        stylesheet: StyleSheet,
        initial_language: LanguageIdentifier,
        localizer: Arc<dyn Localizer>,
    ) -> Self {
        Self {
            stylesheet: Arc::new(stylesheet),
            language: Arc::new(RwLock::new(initial_language)),
            localizer,
        }
    }

    /// Localizes `key` with `parameters`.
    #[must_use]
    // For this usage of RwLock, panics should not be possible.
    #[allow(clippy::missing_panics_doc)]
    pub fn localize<'a>(
        &self,
        key: &str,
        parameters: impl Into<Option<LocalizationParameters<'a>>>,
    ) -> String {
        let language = self.language.read().unwrap();
        self.localizer.localize(key, parameters.into(), &language)
    }

    /// Returns the stylesheet for the application.
    #[must_use]
    pub fn stylesheet(&self) -> &StyleSheet {
        &self.stylesheet
    }
}

/// A parameter used in localization.
#[derive(Debug, Clone)]
pub enum LocalizationParameter<'a> {
    /// A string value.
    String(Cow<'a, str>),
    /// A numeric value.
    Numeric(f64),
}

impl<'a> From<f64> for LocalizationParameter<'a> {
    fn from(value: f64) -> Self {
        Self::Numeric(value)
    }
}

impl<'a> From<&'a str> for LocalizationParameter<'a> {
    fn from(value: &'a str) -> Self {
        Self::String(Cow::Borrowed(value))
    }
}

impl<'a> From<String> for LocalizationParameter<'a> {
    fn from(value: String) -> Self {
        Self::String(Cow::Owned(value))
    }
}

/// Parameters used in localization strings.
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct LocalizationParameters<'a>(HashMap<String, LocalizationParameter<'a>>);

impl<'a> From<HashMap<String, LocalizationParameter<'a>>> for LocalizationParameters<'a> {
    fn from(parameters: HashMap<String, LocalizationParameter<'a>>) -> Self {
        Self(parameters)
    }
}

impl<'a> LocalizationParameters<'a> {
    /// Returns an empty set of parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder-style method for adding a new parameter.
    pub fn with(
        mut self,
        name: impl Into<String>,
        parameter: impl Into<LocalizationParameter<'a>>,
    ) -> Self {
        self.insert(name.into(), parameter.into());
        self
    }
}

impl<'a> Deref for LocalizationParameters<'a> {
    type Target = HashMap<String, LocalizationParameter<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for LocalizationParameters<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> IntoIterator for LocalizationParameters<'a> {
    type IntoIter = std::collections::hash_map::IntoIter<String, LocalizationParameter<'a>>;
    type Item = (String, LocalizationParameter<'a>);

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// An error that can be localized.
pub trait LocalizableError: std::error::Error + 'static {
    /// Returns the localized, human-readable version of this error.
    fn localize(&self, context: &AppContext) -> String;
}

impl LocalizableError for Infallible {
    fn localize(&self, _context: &AppContext) -> String {
        unreachable!()
    }
}

/// Enables using `Display` to convert an error to a string. This macro is
/// provided to make it easy to implement on types from other crates. For your
/// own types, it might be preferred to use `impl NonLocalizedError for MyError
/// {}`.
#[macro_export]
macro_rules! use_display_to_localize_error {
    ($err:ty) => {
        impl LocalizableError for $err {
            fn localize(&self, _context: &AppContext) -> String {
                self.to_string()
            }
        }
    };
}

/// A trait that uses `Display` to convert the error to a String, avoiding any
/// localization.
pub trait NonLocalizedError: std::error::Error + 'static {}

impl<T> LocalizableError for T
where
    T: NonLocalizedError,
{
    fn localize(&self, _context: &AppContext) -> String {
        self.to_string()
    }
}

impl NonLocalizedError for std::num::ParseIntError {}
impl NonLocalizedError for std::num::ParseFloatError {}
impl NonLocalizedError for std::net::AddrParseError {}
impl NonLocalizedError for std::str::ParseBoolError {}
