use std::any::TypeId;

use crate::TransmogrifierState;

/// A frontend is an implementation of widgets and layouts.
pub trait Frontend: Sized {
    /// The generic-free type of the frontend-specific transmogrifier trait.
    type AnyWidgetTransmogrifier: AnyTransmogrifier;
    /// The context type provided to aide in transmogrifying.
    type Context;
}

/// Methods needed for registering transmogrifies
pub trait AnyTransmogrifier {
    /// Returns the [`TypeId`] of the underlying [`Widget`](crate::Widget).
    fn widget_type_id(&self) -> TypeId;
    /// Initializes default state for a newly created widget.
    fn default_state(&self) -> TransmogrifierState;
}
