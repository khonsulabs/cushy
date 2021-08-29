use std::{borrow::Cow, convert::TryFrom, marker::PhantomData};

use crate::{
    styles::{style_sheet::State, Style},
    AnyChannels, AnySendSync, AnyWidget, Channels, Frontend, Transmogrifier, WidgetRegistration,
};

/// A context passed into [`Transmogrifier`] functions with access to useful
/// data and types. This type is mostly used to avoid passing so many parameters
/// across all functions.
pub struct TransmogrifierContext<'a, T: Transmogrifier<F>, F: Frontend> {
    /// The widget's registration.
    pub registration: WidgetRegistration,
    /// The transmogrifier's state.
    pub state: &'a mut <T as Transmogrifier<F>>::State,
    /// The active frontend.
    pub frontend: &'a F,
    /// The widget.
    pub widget: &'a mut <T as Transmogrifier<F>>::Widget,
    /// The effective widget style.
    pub style: Cow<'a, Style>,
    /// The current user interface state for this widget, if applicable for the
    /// frontend and function in question.
    pub ui_state: &'a State,
    /// Communication channels to use message passing for communication.
    pub channels: &'a Channels<<T as Transmogrifier<F>>::Widget>,
    _transmogrifier: PhantomData<T>,
}

impl<'a, T: Transmogrifier<F>, F: Frontend> TransmogrifierContext<'a, T, F> {
    /// Returns a new context.
    pub fn new(
        registration: WidgetRegistration,
        state: &'a mut <T as Transmogrifier<F>>::State,
        frontend: &'a F,
        widget: &'a mut <T as Transmogrifier<F>>::Widget,
        channels: &'a Channels<<T as Transmogrifier<F>>::Widget>,
        style: &'a Style,
        ui_state: &'a State,
    ) -> Self {
        Self {
            registration,
            state,
            frontend,
            widget,
            style: Cow::Borrowed(style),
            ui_state,
            channels,
            _transmogrifier: PhantomData::default(),
        }
    }

    /// Returns `self` after swapping the style with the one provided.
    #[must_use]
    pub fn with_style(self, style: Style) -> Self {
        Self {
            registration: self.registration.clone(),
            state: self.state,
            frontend: self.frontend,
            widget: self.widget,
            style: Cow::Owned(style),
            ui_state: self.ui_state,
            channels: self.channels,
            _transmogrifier: PhantomData,
        }
    }

    /// Returns the style as a reference.
    #[must_use]
    pub fn style(&self) -> &Style {
        &self.style
    }
}

impl<'a, 'b, T: Transmogrifier<F>, F: Frontend> TryFrom<&'b mut AnyTransmogrifierContext<'a, F>>
    for TransmogrifierContext<'b, T, F>
{
    type Error = ();

    fn try_from(context: &'b mut AnyTransmogrifierContext<'a, F>) -> Result<Self, Self::Error> {
        let widget = context
            .widget
            .as_mut_any()
            .downcast_mut::<<T as Transmogrifier<F>>::Widget>()
            .ok_or(())?;
        let state = context
            .state
            .as_mut_any()
            .downcast_mut::<<T as Transmogrifier<F>>::State>()
            .ok_or(())?;
        let channels = context
            .channels
            .as_any()
            .downcast_ref::<Channels<<T as Transmogrifier<F>>::Widget>>()
            .unwrap();
        Ok(Self::new(
            context.registration.clone(),
            state,
            context.frontend,
            widget,
            channels,
            context.style,
            context.ui_state,
        ))
    }
}

/// A context used internally when the [`Transmogrifier`] type cannot be known.
#[allow(clippy::module_name_repetitions)]
pub struct AnyTransmogrifierContext<'a, F: Frontend> {
    /// The widget's registration.
    pub registration: WidgetRegistration,
    /// The transmogrifier's state.
    pub state: &'a mut dyn AnySendSync,
    /// The active frontend.
    pub frontend: &'a F,
    /// The widget.
    pub widget: &'a mut dyn AnyWidget,
    /// The effective widget style.
    pub style: &'a Style,
    /// The current user interface state for this widget, if applicable for the
    /// frontend and function in question.
    pub ui_state: &'a State,
    /// Communication channels to use message passing for communication.
    pub channels: &'a dyn AnyChannels,
}

impl<'a, F: Frontend> AnyTransmogrifierContext<'a, F> {
    /// Returns a new context.
    pub fn new(
        registration: WidgetRegistration,
        state: &'a mut dyn AnySendSync,
        frontend: &'a F,
        widget: &'a mut dyn AnyWidget,
        channels: &'a dyn AnyChannels,
        style: &'a Style,
        ui_state: &'a State,
    ) -> Self {
        Self {
            registration,
            state,
            frontend,
            widget,
            style,
            ui_state,
            channels,
        }
    }
}
