use std::{
    any::TypeId,
    collections::HashMap,
    marker::PhantomData,
    ops::Deref,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex, MutexGuard, RwLock,
    },
};

use crate::{
    AnySendSync, AnyTransmogrifier, AnyWidgetInstance, Frontend, TransmogrifierState, Widget,
    WidgetInstance,
};

type WidgetTypeId = TypeId;

/// A graphical user interface.
pub struct Gooey<F: Frontend> {
    /// The available widget transmogrifiers.
    pub transmogrifiers: HashMap<WidgetTypeId, <F as Frontend>::AnyWidgetTransmogrifier>,
    root: Box<dyn AnyWidgetInstance>,
    storage: TransmogrifierStorage,
    _phantom: PhantomData<F>,
}

/// Generic storage for a transmogrifier.
#[derive(Clone, Default, Debug)]
pub struct TransmogrifierStorage {
    data: Arc<TransmogrifierStorageData>,
}

#[derive(Default, Debug)]
struct TransmogrifierStorageData {
    widget_id_generator: AtomicU32,
    state: RwLock<HashMap<u32, WidgetState>>,
}

impl<F: Frontend> Gooey<F> {
    /// Creates a user interface using `root`.
    pub fn with<W: Widget + Send + Sync, C: FnOnce(&TransmogrifierStorage) -> W>(
        initializer: C,
    ) -> Self {
        let storage = TransmogrifierStorage::default();
        let root = initializer(&storage);
        let root = WidgetInstance::new(root, &storage);
        Self {
            root: Box::new(root),
            transmogrifiers: HashMap::default(),
            storage,
            _phantom: PhantomData::default(),
        }
    }

    /// Returns the root widget.
    #[must_use]
    pub fn root_widget(&self) -> &dyn AnyWidgetInstance {
        self.root.as_ref()
    }

    /// Registers a transmogrifier.
    ///
    /// # Errors
    ///
    /// If an existing transmogrifier is already registered, the transmogrifier
    /// is returned in `Err()`.
    pub fn register_transmogrifier<T: Into<<F as Frontend>::AnyWidgetTransmogrifier>>(
        &mut self,
        transmogrifier: T,
    ) -> Result<(), <F as Frontend>::AnyWidgetTransmogrifier> {
        let transmogrifier = transmogrifier.into();
        let type_id = transmogrifier.widget_type_id();
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
        C: FnOnce(&'_ <F as Frontend>::AnyWidgetTransmogrifier, &mut dyn AnySendSync) -> R,
    >(
        &self,
        widget: &dyn AnyWidgetInstance,
        callback: C,
    ) -> Option<R> {
        self.transmogrifiers
            .get(&widget.widget_type_id())
            .map(|transmogrifier| {
                let state = self
                    .widget_state(widget.widget_ref())
                    .expect("Missing widget state for root");
                let mut state = state.get_or_initialize(transmogrifier);
                let state = state.as_mut().map(|state| state.0.as_mut()).unwrap();
                callback(transmogrifier, state)
            })
    }
}

impl<F: Frontend> Deref for Gooey<F> {
    type Target = TransmogrifierStorage;

    fn deref(&self) -> &Self::Target {
        &self.storage
    }
}

impl TransmogrifierStorage {
    #[must_use]
    pub(crate) fn generate_widget_ref(&self) -> WidgetRef {
        loop {
            let next_id = self.data.widget_id_generator.fetch_add(1, Ordering::AcqRel);
            // Insert None if the slot is free, which is most likely the case.
            // If it performs the insert, is_new is flagged and the id is
            // returned. If not, the loop repeats until a free entry is found.
            let mut is_new = false;
            let mut state = self.data.state.write().unwrap();
            state.entry(next_id).or_insert_with(|| {
                is_new = true;
                WidgetState::default()
            });
            if is_new {
                return WidgetRef::new(next_id, self);
            }
        }
    }

    pub(crate) fn unregister_id(&self, id: u32) {
        let mut state = self.data.state.write().unwrap();
        state.remove(&id);
    }

    #[must_use]
    pub(crate) fn widget_state(&self, widget: &WidgetRef) -> Option<WidgetState> {
        let state = self.data.state.read().unwrap();
        state.get(&widget.id()).cloned()
    }
}

/// Generic, clone-able storage for a widget's transmogrifier.
#[derive(Clone, Debug, Default)]
pub struct WidgetState(Arc<Mutex<Option<TransmogrifierState>>>);

impl WidgetState {
    /// Returns the state for this widget. If this is the first call, the state
    /// is initialized with the `Default::default()` implementation for the
    /// `State` associated type on [`Transmogrifier`](crate::Transmogrifier).
    ///
    /// # Panics
    ///
    /// Panics if internal lock poisoning occurs.
    pub fn get_or_initialize<'a, T: AnyTransmogrifier>(
        &'a self,
        transmogrifier: &T,
    ) -> MutexGuard<'a, Option<TransmogrifierState>> {
        let mut guard = self.0.lock().unwrap();
        if guard.is_none() {
            *guard = Some(transmogrifier.default_state());
        }
        guard
    }
}

/// References an initialized widget. On drop, frees the storage and id.
#[derive(Clone, Debug)]
pub struct WidgetRef(u32, TransmogrifierStorage);

impl WidgetRef {
    pub(crate) fn new(id: u32, storage: &TransmogrifierStorage) -> Self {
        Self(id, storage.clone())
    }

    /// Returns the unique ID of this widget. IDs are unique per `Gooey`
    /// instance, not across the entire executable.
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.0
    }
}

impl Drop for WidgetRef {
    fn drop(&mut self) {
        self.1.unregister_id(self.0)
    }
}
