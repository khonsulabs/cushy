use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::Deref,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex, RwLock,
    },
};

use crate::{
    AnyChannels, AnySendSync, AnyTransmogrifier, AnyWidget, Frontend, TransmogrifierState, Widget,
    WidgetId,
};

type WidgetTypeId = TypeId;

/// A graphical user interface.
pub struct Gooey<F: Frontend> {
    transmogrifiers: HashMap<WidgetTypeId, <F as Frontend>::AnyTransmogrifier>,
    root: WidgetRegistration,
    storage: WidgetStorage,
    _phantom: PhantomData<F>,
}

impl<F: Frontend> Gooey<F> {
    /// Creates a user interface using `root`.
    pub fn with<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> W>(initializer: C) -> Self {
        let storage = WidgetStorage::default();
        let root = initializer(&storage);
        let root = storage.register(root);
        Self {
            root,
            transmogrifiers: HashMap::default(),
            storage,
            _phantom: PhantomData::default(),
        }
    }

    /// Returns the root widget.
    #[must_use]
    pub fn root_widget(&self) -> &WidgetRegistration {
        &self.root
    }

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
        if self.transmogrifiers.contains_key(&type_id) {
            return Err(transmogrifier);
        }

        self.transmogrifiers.insert(type_id, transmogrifier);

        Ok(())
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
        self.transmogrifiers
            .get(&widget.type_id)
            .map(|transmogrifier| {
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
        self.storage.for_each_widget(|widget_state| {
            let transmogrifier = self.transmogrifiers.get(&widget_state.id.type_id).unwrap();
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
    pub fn process_widget_messages(&self, frontend: &F) {
        let widgets_with_messages = {
            let mut widgets_with_messages = self.storage.data.widgets_with_messages.lock().unwrap();
            if widgets_with_messages.is_empty() {
                return;
            }

            std::mem::take(&mut *widgets_with_messages)
        };

        for widget_id in widgets_with_messages {
            self.with_transmogrifier(&widget_id, |transmogrifier, state, widget, channels| {
                transmogrifier.process_messages(state, widget, channels, frontend);
            });
        }
    }
}

impl<F: Frontend> Deref for Gooey<F> {
    type Target = WidgetStorage;

    fn deref(&self) -> &Self::Target {
        &self.storage
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
            let mut is_new = false;
            let mut state = self.data.state.write().unwrap();
            state.entry(next_id).or_insert_with(|| {
                is_new = true;
                WidgetState::new(widget.take().unwrap(), next_id)
            });
            if is_new {
                return WidgetRegistration::new::<W>(next_id, self);
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
    pub id: WidgetId,
    /// The widget.
    pub widget: Arc<Mutex<Box<dyn AnyWidget>>>,
    /// The transmogrifier state.
    pub state: Arc<Mutex<Option<TransmogrifierState>>>,
}

impl WidgetState {
    /// Initializes a new widget state with `widget`, `id`, and `None` for the
    /// transmogrifier state.
    pub fn new<W: AnyWidget>(widget: W, id: u32) -> Self {
        Self {
            id: WidgetId {
                id,
                type_id: widget.widget_type_id(),
            },
            widget: Arc::new(Mutex::new(Box::new(widget))),
            state: Arc::default(),
        }
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
    ) -> R {
        let mut state = self.state.lock().unwrap();
        let mut widget = self.widget.lock().unwrap();
        let state = state.get_or_insert_with(|| transmogrifier.default_state_for(self.id.clone()));

        callback(
            state.state.as_mut(),
            widget.as_mut(),
            state.channels.as_ref(),
        )
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
