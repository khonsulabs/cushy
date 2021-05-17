use std::{any::TypeId, collections::HashMap, marker::PhantomData};

use crate::{AnyTransmogrifier, AnyWidget, Frontend, Widget};

type WidgetTypeId = TypeId;

/// A graphical user interface.
pub struct Gooey<F: Frontend> {
    /// The available widget transmogrifiers.
    pub transmogrifiers: HashMap<WidgetTypeId, <F as Frontend>::AnyWidgetTransmogrifier>,
    root: Box<dyn AnyWidget>,
    _phantom: PhantomData<F>,
}

impl<F: Frontend> Gooey<F> {
    /// Creates a user interface using `root`.
    pub fn new<W: Widget + Send + Sync>(root: W) -> Self {
        Self {
            root: Box::new(root),
            transmogrifiers: HashMap::default(),
            _phantom: PhantomData::default(),
        }
    }

    /// Returns the root widget.
    #[must_use]
    pub fn root_widget(&self) -> &dyn AnyWidget {
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

    /// Returns the registered transmogrifier for the widget type id specified.
    #[must_use]
    pub fn transmogrifier(
        &self,
        widget_type_id: TypeId,
    ) -> Option<&'_ <F as Frontend>::AnyWidgetTransmogrifier> {
        self.transmogrifiers.get(&widget_type_id)
    }

    /// Returns the registered transmogrifier for the root widget.
    #[must_use]
    pub fn root_transmogrifier(&'_ self) -> Option<&'_ <F as Frontend>::AnyWidgetTransmogrifier> {
        self.transmogrifier(self.root_widget().widget_type_id())
    }
}
