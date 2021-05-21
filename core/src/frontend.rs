use std::any::TypeId;

use crate::{
    AnyChannels, AnySendSync, AnyWidget, Channels, Gooey, Transmogrifier, TransmogrifierState,
    WidgetId,
};

/// A frontend is an implementation of widgets and layouts.
pub trait Frontend: Sized {
    /// The generic-free type of the frontend-specific transmogrifier trait.
    type AnyWidgetTransmogrifier: AnyFrontendTransmogrifier<Self>;
    /// The context type provided to aide in transmogrifying.
    type Context;

    /// Returns the underlying [`Gooey`] instance.
    fn gooey(&self) -> &'_ Gooey<Self>;
}

/// A Transmogrifier without any associated types.
pub trait AnyFrontendTransmogrifier<F: Frontend> {
    /// Returns the [`TypeId`] of the underlying [`Widget`](crate::Widget).
    fn widget_type_id(&self) -> TypeId;
    /// Initializes default state for a newly created widget.
    fn default_state_for(&self, widget_id: WidgetId) -> TransmogrifierState;

    /// Processes commands and events for this widget and transmogrifier.
    fn process_messages(
        &self,
        state: &mut dyn AnySendSync,
        widget: &mut dyn AnyWidget,
        channels: &dyn AnyChannels,
        frontend: &F,
    );
}

impl<F: Frontend, T> AnyFrontendTransmogrifier<F> for T
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

    fn default_state_for(&self, widget_id: WidgetId) -> TransmogrifierState {
        <Self as Transmogrifier<F>>::default_state_for(self, widget_id)
    }
}
