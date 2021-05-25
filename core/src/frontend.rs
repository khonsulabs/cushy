use std::{any::TypeId, fmt::Debug, sync::Arc};

use crate::{
    AnyChannels, AnySendSync, AnyWidget, Channels, Gooey, Transmogrifier, TransmogrifierState,
    WidgetRef, WidgetRegistration, WidgetStorage,
};

/// A frontend is an implementation of widgets and layouts.
pub trait Frontend: Clone + Debug + Send + Sync + 'static {
    /// The generic-free type of the frontend-specific transmogrifier trait.
    type AnyTransmogrifier: AnyTransmogrifier<Self>;
    /// The context type provided to aide in transmogrifying.
    type Context;

    /// Returns the underlying [`Gooey`] instance.
    fn gooey(&self) -> &'_ Gooey<Self>;

    /// Processes any pending messages for widgets and transmogrifiers.
    fn process_widget_messages(&self) {
        self.gooey().process_widget_messages(self);
    }
}

/// An interface for Frontend that doesn't requier knowledge of associated
/// types.
#[allow(clippy::module_name_repetitions)]
pub trait AnyFrontend: AnySendSync {
    /// Clones the frontend, returning the clone in a box.
    #[must_use]
    fn cloned(&self) -> Box<dyn AnyFrontend>;
    /// Returns the widget storage.
    #[must_use]
    fn storage(&self) -> &'_ WidgetStorage;

    /// Processes any pending messages for widgets and transmogrifiers.
    fn process_widget_messages(&self);
}

impl<T> AnyFrontend for T
where
    T: Frontend + AnySendSync,
{
    fn cloned(&self) -> Box<dyn AnyFrontend> {
        Box::new(self.clone())
    }

    fn storage(&self) -> &'_ WidgetStorage {
        self.gooey()
    }

    fn process_widget_messages(&self) {
        self.process_widget_messages()
    }
}

/// A Transmogrifier without any associated types.
pub trait AnyTransmogrifier<F: Frontend>: Debug {
    /// Returns the [`TypeId`] of the underlying [`Widget`](crate::Widget).
    fn widget_type_id(&self) -> TypeId;
    /// Initializes default state for a newly created widget.
    fn default_state_for(
        &self,
        widget: &mut dyn AnyWidget,
        registration: &Arc<WidgetRegistration>,
        frontend: &F,
    ) -> TransmogrifierState;

    /// Processes commands and events for this widget and transmogrifier.
    fn process_messages(
        &self,
        state: &mut dyn AnySendSync,
        widget: &mut dyn AnyWidget,
        channels: &dyn AnyChannels,
        frontend: &F,
    );
}

impl<F: Frontend, T> AnyTransmogrifier<F> for T
where
    T: Transmogrifier<F>,
{
    fn process_messages(
        &self,
        state: &mut dyn AnySendSync,
        widget: &mut dyn AnyWidget,
        channels: &dyn AnyChannels,
        frontend: &F,
    ) {
        let widget = widget
            .as_mut_any()
            .downcast_mut::<<Self as Transmogrifier<F>>::Widget>()
            .unwrap();
        let state = state
            .as_mut_any()
            .downcast_mut::<<Self as Transmogrifier<F>>::State>()
            .unwrap();
        let channels = channels
            .as_any()
            .downcast_ref::<Channels<<Self as Transmogrifier<F>>::Widget>>()
            .unwrap();
        <Self as Transmogrifier<F>>::process_messages(self, state, widget, frontend, channels);
    }

    fn widget_type_id(&self) -> TypeId {
        <Self as Transmogrifier<F>>::widget_type_id(self)
    }

    fn default_state_for(
        &self,
        widget: &mut dyn AnyWidget,
        registration: &Arc<WidgetRegistration>,
        frontend: &F,
    ) -> TransmogrifierState {
        let widget = widget
            .as_mut_any()
            .downcast_mut::<<Self as Transmogrifier<F>>::Widget>()
            .unwrap();
        let registration = WidgetRef::new(registration, frontend.clone()).unwrap();
        <Self as Transmogrifier<F>>::default_state_for(self, widget, &registration, frontend)
    }
}
