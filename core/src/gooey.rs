use std::{any::TypeId, collections::HashMap, marker::PhantomData};

use crate::{AnyWidget, Frontend, Widget};

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
}
