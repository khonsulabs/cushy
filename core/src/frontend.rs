/// A frontend is an implementation of widgets and layouts.
pub trait Frontend: Sized {
    /// The generic-free type of the frontend-specific transmogrifier trait.
    type AnyWidgetTransmogrifier;
}
