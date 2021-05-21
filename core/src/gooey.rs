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
    AnyChannels, AnySendSync, AnyTransmogrifier, AnyWidget, Channels, Frontend,
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
    root: Arc<WidgetRegistration>,
    storage: WidgetStorage,
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
            }),
        }
    }

    /// Returns the root widget.
    #[must_use]
    pub fn root_widget(&self) -> &Arc<WidgetRegistration> {
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
            &dyn AnyChannels,
        ) -> R,
    >(
        &self,
        widget: &WidgetId,
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
                state.with_state(transmogrifier, |state, widget, channels| {
                    callback(transmogrifier, state, widget, channels)
                })
            })
    }

    /// Executes `callback` with the transmogrifier and transmogrifier state as
    /// parameters.
    #[allow(clippy::missing_panics_doc)] // unwrap is guranteed due to get_or_initialize
    pub fn for_each_widget<
        C: FnMut(
            &'_ <F as Frontend>::AnyTransmogrifier,
            &mut dyn AnySendSync,
            &dyn AnyWidget,
            &dyn AnyChannels,
        ),
    >(
        &self,
        mut callback: C,
    ) {
        self.data.storage.for_each_widget(|widget_state| {
            let transmogrifier = self
                .data
                .transmogrifiers
                .map
                .get(&widget_state.registration().unwrap().id.type_id)
                .unwrap();
            widget_state.with_state(transmogrifier, |transmogrifier_state, widget, channels| {
                callback(transmogrifier, transmogrifier_state, widget, channels);
            });
        });
    }

    /// Loops over each widget processing its events first, then its commands.
    ///
    /// # Panics
    ///
    /// Panics upon internal locking errors.
    pub fn process_widget_messages(&self) {
        let widgets_with_messages = {
            let mut widgets_with_messages =
                self.data.storage.data.widgets_with_messages.lock().unwrap();
            if widgets_with_messages.is_empty() {
                return;
            }

            std::mem::take(&mut *widgets_with_messages)
        };

        for widget_id in widgets_with_messages {
            self.with_transmogrifier(&widget_id, |transmogrifier, state, widget, channels| {
                transmogrifier.process_messages(state, widget, channels, &self.data.storage);
            });
        }
    }

    pub(crate) fn post_transmogrifier_event<W: Widget>(
        &self,
        widget: &WidgetId,
        event: <W as Widget>::TransmogrifierEvent,
    ) {
        self.with_transmogrifier(widget, |transmogrifier, state, widget, channels| {
            let channels = channels.as_any().downcast_ref::<Channels<W>>().unwrap();

            channels.post_event(event);
            transmogrifier.process_messages(state, widget, channels, self);
        });

        // Process any messages that may have been triggered onto other widgets.
        self.process_widget_messages();
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
    pub fn register<W: Widget + AnyWidget>(&self, widget: W) -> Arc<WidgetRegistration> {
        let mut widget = Some(widget);
        loop {
            let next_id = self.data.widget_id_generator.fetch_add(1, Ordering::AcqRel);
            // Insert None if the slot is free, which is most likely the case.
            // If it performs the insert, is_new is flagged and the id is
            // returned. If not, the loop repeats until a free entry is found.
            let mut widget_registration = None;
            let mut state = self.data.state.write().unwrap();
            state.entry(next_id).or_insert_with(|| {
                let reg = Arc::new(WidgetRegistration::new::<W>(next_id, self));
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

    #[must_use]
    pub(crate) fn widget_state(&self, widget_id: u32) -> Option<WidgetState> {
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

    pub(crate) fn set_widget_has_messages(&self, widget: WidgetId) {
        let mut statuses = self.data.widgets_with_messages.lock().unwrap();
        statuses.insert(widget);
    }
}

/// Generic, clone-able storage for a widget's transmogrifier.
#[derive(Clone, Debug)]
pub struct WidgetState {
    /// The id of the widget.
    pub id: Weak<WidgetRegistration>,
    /// The widget.
    pub widget: Arc<Mutex<Box<dyn AnyWidget>>>,
    /// The transmogrifier state.
    pub state: Arc<Mutex<Option<TransmogrifierState>>>,
}

impl WidgetState {
    /// Initializes a new widget state with `widget`, `id`, and `None` for the
    /// transmogrifier state.
    pub fn new<W: AnyWidget>(widget: W, id: &Arc<WidgetRegistration>) -> Self {
        Self {
            id: Arc::downgrade(id),
            widget: Arc::new(Mutex::new(Box::new(widget))),
            state: Arc::default(),
        }
    }

    /// Gets the registration for this widget. Returns None if the widget has
    /// been destroyed.
    #[must_use]
    pub fn registration(&self) -> Option<Arc<WidgetRegistration>> {
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
        C: FnOnce(&mut dyn AnySendSync, &mut dyn AnyWidget, &dyn AnyChannels) -> R,
    >(
        &self,
        transmogrifier: &T,
        callback: C,
    ) -> Option<R> {
        let mut state = self.state.lock().unwrap();
        let mut widget = self.widget.lock().unwrap();
        self.id.upgrade().map(|id| {
            let state = state.get_or_insert_with(|| transmogrifier.default_state_for(&id));

            callback(
                state.state.as_mut(),
                widget.as_mut(),
                state.channels.as_ref(),
            )
        })
    }
}

/// References an initialized widget. On drop, frees the storage and id.
#[derive(Debug)]
pub struct WidgetRegistration {
    id: WidgetId,
    storage: WidgetStorage,
}

impl WidgetRegistration {
    pub(crate) fn new<W: Widget>(id: u32, storage: &WidgetStorage) -> Self {
        Self {
            id: WidgetId {
                id,
                type_id: TypeId::of::<W>(),
            },
            storage: storage.clone(),
        }
    }

    /// Returns the unique ID of this widget. IDs are unique per `Gooey`
    /// instance, not across the entire executable.
    #[must_use]
    pub const fn id(&self) -> &'_ WidgetId {
        &self.id
    }
}

impl Drop for WidgetRegistration {
    fn drop(&mut self) {
        self.storage.unregister_id(self.id.id)
    }
}

/// A widget reference. Does not prevent a widget from being destroyed if
/// removed from an interface.
#[derive(Debug)]
pub struct WidgetRef<W: Widget, F: Frontend> {
    registration: Weak<WidgetRegistration>,
    frontend: F,
    _phantom: PhantomData<(W, F)>,
}

impl<W: Widget, F: Frontend> Clone for WidgetRef<W, F> {
    fn clone(&self) -> Self {
        Self {
            registration: self.registration.clone(),
            frontend: self.frontend.clone(),
            _phantom: PhantomData::default(),
        }
    }
}

impl<W: Widget, F: Frontend> WidgetRef<W, F> {
    /// Creates a new reference from a [`WidgetRegistration`]. Returns None if
    /// the `W` type doesn't match the type of the widget.
    #[must_use]
    pub fn new(registration: &Arc<WidgetRegistration>, frontend: F) -> Option<Self> {
        if registration.id.type_id == TypeId::of::<W>() {
            Some(Self {
                registration: Arc::downgrade(registration),
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
    pub fn registration(&self) -> Option<Arc<WidgetRegistration>> {
        self.registration.upgrade()
    }

    /// Posts `event` to a transmogrifier.
    pub fn post_event(&self, event: W::TransmogrifierEvent) {
        if let Some(registration) = self.registration() {
            self.frontend
                .gooey()
                .post_transmogrifier_event::<W>(&registration.id, event);
        }
    }
}
