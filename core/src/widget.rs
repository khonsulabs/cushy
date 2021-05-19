use std::{
    any::{Any, TypeId},
    fmt::Debug,
};

use crate::{Frontend, TransmogrifierStorage, WidgetRef};

/// A graphical user interface element.
pub trait Widget: Send + Sync + 'static {
    /// The type of the event that any [`Transmogrifier`] for this widget to
    /// use.
    type TransmogrifierEvent: Send + Sync;
}

/// Transforms a Widget into whatever is needed for [`Frontend`] `F`.
pub trait Transmogrifier<F: Frontend> {
    /// The type of the widget being transmogrified.
    type Widget: Widget;
    /// The type the storage this transmogrifier uses for state.
    type State: Default + Debug + Any + Send + Sync;
}

/// A Widget without any associated types. Useful for implementing frontends.
#[allow(clippy::module_name_repetitions)]
pub trait AnyWidgetInstance: Send + Sync {
    /// Returns the widget as the [`Any`] type.
    #[must_use]
    fn as_any(&self) -> &'_ dyn Any;

    /// Returns the [`TypeId`] of the widget.
    #[must_use]
    fn widget_type_id(&self) -> TypeId;

    /// Returns the unique id of this widget instance.
    #[must_use]
    fn widget_ref(&self) -> &'_ WidgetRef;
}

impl<T> AnyWidgetInstance for WidgetInstance<T>
where
    T: Widget + Send + Sync + Any,
{
    fn as_any(&self) -> &'_ dyn Any {
        &self.widget
    }

    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn widget_ref(&self) -> &'_ WidgetRef {
        &self.id
    }
}

/// Generic storage for a transmogrifier.
#[derive(Debug)]
pub struct TransmogrifierState(pub Box<dyn AnySendSync>);

/// A value that can be used as [`Any`] that is threadsafe.
pub trait AnySendSync: Any + Debug + Send + Sync {
    /// Returns the underlying type as [`Any`].
    fn as_mut_any(&mut self) -> &'_ mut dyn Any;
}

impl<T> AnySendSync for T
where
    T: Any + Debug + Send + Sync,
{
    fn as_mut_any(&mut self) -> &'_ mut dyn Any {
        self
    }
}

/// An instance of a widget
#[allow(clippy::module_name_repetitions)]
pub struct WidgetInstance<W: Widget> {
    id: WidgetRef,
    /// The instantiated widget.
    pub widget: W,
}

impl<W: Widget> WidgetInstance<W> {
    /// Returns a new instance after reserving transmogrifier storage.
    pub fn new(widget: W, storage: &TransmogrifierStorage) -> Self {
        let id = storage.generate_widget_ref();
        Self { id, widget }
    }
}
