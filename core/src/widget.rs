use std::any::{Any, TypeId};

use euclid::Size2D;
use stylecs::Points;

use crate::Frontend;

/// A graphical user interface element.
pub trait Widget: Send + Sync + 'static {
    /// The type of the event that any [`Transmogrifier`] for this widget to
    /// use.
    type TransmogrifierEvent: Send + Sync;
}

/// Transforms a Widget into whatever is needed for [`Frontend`] `F`.
pub trait Transmogrifier<F: Frontend>: Send + Sync {
    /// The type of the widget being transmogrified.
    type Widget: Widget;
    /// The frontend-specific context type provided to aide in transmogrifying.
    type Context: Send + Sync;

    /// Calculate the content-size needed for this `widget`, trying to stay
    /// within `constraints`.
    fn content_size(
        &self,
        widget: &Self::Widget,
        constraints: Size2D<Option<f32>, Points>,
        context: &Self::Context,
    ) -> Size2D<f32, Points>;
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
    T: Widget + Any,
{
    fn as_any(&'_ self) -> &'_ dyn Any {
        self
    }

    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}
