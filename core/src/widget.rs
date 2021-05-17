use std::any::{Any, TypeId};

use crate::Frontend;

/// A graphical user interface element.
pub trait Widget: 'static {
    /// The type of the event that any [`Transmogrifier`] for this widget to
    /// use.
    type TransmogrifierEvent: Send + Sync;
}

/// Transforms a Widget into whatever is needed for [`Frontend`] `F`.
pub trait Transmogrifier<F: Frontend> {
    /// The type of the widget being transmogrified.
    type Widget: Widget;
}

/// A Widget without any associated types. Useful for implementing frontends.
#[allow(clippy::module_name_repetitions)]
pub trait AnyWidget: Send + Sync {
    /// Returns the widget as the [`Any`] type.
    #[must_use]
    fn as_any(&'_ self) -> &'_ dyn Any;
    /// Returns the [`TypeId`] of the widget.
    #[must_use]
    fn widget_type_id(&self) -> TypeId;
}

impl<T> AnyWidget for T
where
    T: Widget + Any + Send + Sync,
{
    fn as_any(&'_ self) -> &'_ dyn Any {
        self
    }

    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}
