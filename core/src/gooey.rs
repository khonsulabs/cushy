use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::Deref,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex, RwLock, Weak,
    },
};

use crate::{
    AnyChannels, AnyFrontend, AnySendSync, AnyTransmogrifier, AnyWidget, Channels, Frontend,
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
    transmogrifiers: Transmogrifiers<F>,
    root: WidgetRegistration,
    storage: WidgetStorage,
    processing_messages_lock: Mutex<()>,
}

impl<F: Frontend> Gooey<F> {
    /// Creates a user interface using `root`.
    pub fn with<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> W>(
        transmogrifiers: Transmogrifiers<F>,
        initializer: C,
    ) -> Self {
        let storage = WidgetStorage::default();
        let root = initializer(&storage);
        let root = storage.register(root);
        Self {
            data: Arc::new(GooeyData {
                transmogrifiers,
                root,
                storage,
                processing_messages_lock: Mutex::default(),
            }),
        }
    }

    /// Returns the root widget.
    #[must_use]
    pub fn root_widget(&self) -> &WidgetRegistration {
        &self.data.root
    }

    /// Executes `callback` with the transmogrifier and transmogrifier state as
    /// parameters.
    #[allow(clippy::missing_panics_doc)] // unwrap is guranteed due to get_or_initialize
    pub fn with_transmogrifier<
        R,
        C: FnOnce(
            &'_ <F as Frontend>::AnyTransmogrifier,
            &mut dyn AnySendSync,
            &mut dyn AnyWidget,
        ) -> R,
    >(
        &self,
        widget: &WidgetId,
        frontend: &F,
        callback: C,
    ) -> Option<R> {
        self.data
            .transmogrifiers
            .map
            .get(&widget.type_id)
            .and_then(|transmogrifier| {
                let state = self
                    .widget_state(widget.id)
                    .expect("Missing widget state for root");
                state.with_state(transmogrifier, frontend, |state, widget| {
                    callback(transmogrifier, state, widget)
                })
            })
    }

    /// Executes `callback` with the transmogrifier and transmogrifier state as
    /// parameters.
    #[allow(clippy::missing_panics_doc)] // unwrap is guranteed due to get_or_initialize
    pub fn for_each_widget<
        C: FnMut(&'_ <F as Frontend>::AnyTransmogrifier, &mut dyn AnySendSync, &dyn AnyWidget),
    >(
        &self,
        frontend: &F,
        mut callback: C,
    ) {
        self.data.storage.for_each_widget(|widget_state| {
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
            Ok(guard) => guard,
            Err(_) => return,
        };
        // This method will return early if there are no widgets with pending
        // messages. `chatter_limit` is put in place to limit cross-talk betwen
        // widgets. It's possible for one widget to invoke behavior that
        // triggers a message in another widget, which then returns the favor.
        // 10 is an arbitrary number.
        let mut chatter_limit = 10;
        while chatter_limit > 0 {
            chatter_limit -= 1;
            let widgets_with_messages = {
                let mut widgets_with_messages =
                    self.data.storage.data.widgets_with_messages.lock().unwrap();
                if widgets_with_messages.is_empty() {
                    return;
                }

                std::mem::take(&mut *widgets_with_messages)
            };

            for widget_id in widgets_with_messages {
                self.with_transmogrifier(&widget_id, frontend, |transmogrifier, state, widget| {
                    let widget_state = self.data.storage.widget_state(widget_id.id).unwrap();
                    transmogrifier.process_messages(
                        state,
                        widget,
                        widget_state.channels.as_ref().as_ref(),
                        frontend,
                    );
                });
            }
        }
    }

    pub(crate) fn post_transmogrifier_event<W: Widget>(
        &self,
        widget_id: &WidgetId,
        event: <W as Widget>::TransmogrifierEvent,
        frontend: &F,
    ) {
        if let Some(state) = self.widget_state(widget_id.id) {
            let channels = state.channels::<W>().unwrap();
            channels.post_event(event);
            self.set_widget_has_messages(widget_id.clone());
        }

        // Process any messages that may have been triggered onto other widgets.
        frontend.process_widget_messages();
    }

    pub(crate) fn post_command<W: Widget>(
        &self,
        widget_id: &WidgetId,
        event: <W as Widget>::Command,
        frontend: &F,
    ) {
        if let Some(state) = self.widget_state(widget_id.id) {
            let channels = state.channels::<W>().unwrap();
            channels.post_command(event);
            self.set_widget_has_messages(widget_id.clone());
        }

        // Process any messages that may have been triggered onto other widgets.
        frontend.process_widget_messages();
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
#[derive(Clone, Default, Debug)]
pub struct WidgetStorage {
    data: Arc<WidgetStorageData>,
}

#[derive(Default, Debug)]
struct WidgetStorageData {
    widget_id_generator: AtomicU32,
    state: RwLock<HashMap<u32, WidgetState>>,
    widgets_with_messages: Mutex<HashSet<WidgetId>>,
}

impl WidgetStorage {
    /// Register a widget with storage.
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // The unwrap is unreachable
    pub fn register<W: Widget + AnyWidget>(&self, widget: W) -> WidgetRegistration {
        let mut widget = Some(widget);
        loop {
            let next_id = self.data.widget_id_generator.fetch_add(1, Ordering::AcqRel);
            // Insert None if the slot is free, which is most likely the case.
            // If it performs the insert, is_new is flagged and the id is
            // returned. If not, the loop repeats until a free entry is found.
            let mut widget_registration = None;
            let mut state = self.data.state.write().unwrap();
            state.entry(next_id).or_insert_with(|| {
                let reg = WidgetRegistration::new::<W>(next_id, self);
                let state = WidgetState::new(widget.take().unwrap(), &reg);
                widget_registration = Some(reg);
                state
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
        drop(removed_value)
    }

    /// Returns the state of the widget with id `widget_id`.
    ///
    /// # Panics
    ///
    /// Panics if internal lock handling results in an error.
    #[must_use]
    pub fn widget_state(&self, widget_id: u32) -> Option<WidgetState> {
        let state = self.data.state.read().unwrap();
        state.get(&widget_id).cloned()
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

        for widget in widgets {
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
        let mut statuses = self.data.widgets_with_messages.lock().unwrap();
        statuses.insert(widget);
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
    pub channels: Arc<Box<dyn AnyChannels>>,
}

impl WidgetState {
    /// Initializes a new widget state with `widget`, `id`, and `None` for the
    /// transmogrifier state.
    pub fn new<W: Widget + AnyWidget>(widget: W, id: &WidgetRegistration) -> Self {
        Self {
            id: WeakWidgetRegistration::from(id),
            widget: Arc::new(Mutex::new(Box::new(widget))),
            state: Arc::default(),
            channels: Arc::new(Box::new(Channels::<W>::new(id))),
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
        let mut state = self.state.lock().unwrap();
        let mut widget = self.widget.lock().unwrap();
        self.id.upgrade().map(|id| {
            let state = state.get_or_insert_with(|| {
                transmogrifier.default_state_for(widget.as_mut(), &id, frontend)
            });

            callback(state.state.as_mut(), widget.as_mut())
        })
    }

    #[must_use]
    pub(crate) fn any_channels(&self) -> &'_ dyn AnyChannels {
        self.channels.as_ref().as_ref()
    }

    /// Returns the channels used to communicate with this widget.
    #[must_use]
    pub fn channels<W: Widget>(&self) -> Option<&'_ Channels<W>> {
        self.any_channels().as_any().downcast_ref()
    }
}

/// References an initialized widget. On drop, frees the storage and id.
#[derive(Clone, Debug)]
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
        self.storage.unregister_id(self.id.id)
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
            Some(Self {
                registration: WeakWidgetRegistration::from(registration),
                frontend,
                _phantom: PhantomData::default(),
            })
        } else {
            None
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
    pub fn post_event<F: Frontend>(&self, event: W::TransmogrifierEvent) {
        let frontend = self.frontend.as_ref().as_any().downcast_ref::<F>().unwrap();
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
