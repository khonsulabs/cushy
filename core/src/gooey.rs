use crate::{AnyWidget, Widget};

/// A graphical user interface.
pub struct Gooey {
    root: Box<dyn AnyWidget>,
}

impl Gooey {
    /// Creates a user interface using `root`.
    pub fn new<W: Widget>(root: W) -> Self {
        Self {
            root: Box::new(root),
        }
    }

    /// Returns the root widget.
    #[must_use]
    pub fn root_widget(&self) -> &dyn AnyWidget {
        self.root.as_ref()
    }
}
