use std::{any::TypeId, convert::TryFrom, fmt::Debug, time::Duration};

use stylecs::Style;
use url::Url;

use crate::{
    assets::{self, Asset, Image},
    styles::{style_sheet::State, SystemTheme},
    window_builder::AnyWindowBuilder,
    AnySendSync, AnyTransmogrifierContext, AnyWidget, AppContext, Callback, Gooey,
    LocalizationParameters, Timer, Transmogrifier, TransmogrifierContext, TransmogrifierState,
    WidgetId, WidgetRef, WidgetRegistration, WidgetStorage, Window,
};

/// A frontend is an implementation of widgets and layouts.
pub trait Frontend: Clone + Debug + Send + Sync + 'static {
    /// The generic-free type of the frontend-specific transmogrifier trait.
    type AnyTransmogrifier: AnyTransmogrifier<Self>;
    /// The context type provided to aide in transmogrifying.
    type Context;

    /// Returns the underlying [`Gooey`] instance.
    fn gooey(&self) -> &'_ Gooey<Self>;

    /// Returns the current system theme.
    fn theme(&self) -> SystemTheme;

    /// Returns the [`State`] for the widget. Not all frontends track UI state,
    /// and this function is primarily used when building frontends.
    #[allow(unused_variables)]
    fn ui_state_for(&self, widget_id: &WidgetId) -> State {
        State::default()
    }

    /// Notifies the frontend that a widget has messages. Frontends should
    /// ensure that `process_widget_messages` is called at some point after this
    /// method is called.
    fn set_widget_has_messages(&self, widget: WidgetId);

    /// Loads an image asynchronously, executing `completed` when loaded.
    fn load_image(&self, asset: &Image, completed: Callback<Image>, error: Callback<String>);

    /// Returns the full Url for the asset, if available.
    fn asset_url(&self, asset: &Asset) -> Option<Url> {
        let mut url = self
            .asset_configuration()
            .asset_base_url
            .clone()
            .unwrap_or_else(|| Url::parse("http://localhost:8080/assets/").unwrap());
        for part in asset.path() {
            url = url.join(part).expect("invalid asset path component");
        }
        Some(url)
    }

    /// Returns the asset configuration.
    fn asset_configuration(&self) -> &assets::Configuration;

    /// Executed when `Gooey` exits a managed code block.
    fn exit_managed_code(&self) {}

    /// Schedules a timer that invokes `callback` after `duration`, and repeats if `repeating` is true.
    fn schedule_timer(&self, callback: Callback, duration: Duration, repeating: bool) -> Timer;

    /// A widget is being initialized.
    #[allow(unused_variables)]
    fn widget_initialized(&self, widget: &WidgetId, style: &Style) {}

    /// Returns the window for this interface, if present.
    fn window(&self) -> Option<&dyn Window>;

    /// Opens a window. Returns false if unable to open the window.
    fn open(&self, window: Box<dyn AnyWindowBuilder>) -> bool;

    /// Localizes `key` with `parameters`.
    #[must_use]
    fn localize<'a>(
        &self,
        key: &str,
        parameters: impl Into<Option<LocalizationParameters<'a>>>,
    ) -> String {
        self.gooey().localize(key, parameters)
    }
}

/// An interface for Frontend that doesn't requier knowledge of associated
/// types.
pub trait AnyFrontend: AnySendSync {
    /// Clones the frontend, returning the clone in a box.
    #[must_use]
    fn cloned(&self) -> Box<dyn AnyFrontend>;
    /// Returns the widget storage.
    #[must_use]
    fn storage(&self) -> &'_ WidgetStorage;

    /// Returns the current application context.
    fn app(&self) -> &AppContext {
        self.storage().app()
    }

    /// Returns the window for this frontend instance.
    fn window(&self) -> Option<&dyn Window>;

    /// Returns the current system theme.
    fn theme(&self) -> SystemTheme;

    /// Notifies the frontend that a widget has messages. Frontends should
    /// ensure that `process_widget_messages` is called at some point after this
    /// method is called.
    fn set_widget_has_messages(&self, widget: WidgetId);

    /// Marks that managed code is being executed. Can be nested. Automatically exited when the returned guard is dropped.
    #[must_use]
    fn enter_managed_code(&self) -> ManagedCodeGuard;

    /// Loads an image asynchronously, executing `completed` when loaded.
    fn load_image(&self, asset: &Image, completed: Callback<Image>, error: Callback<String>);

    /// Returns the full Url for the asset, if available.
    fn asset_url(&self, asset: &Asset) -> Option<Url>;

    /// Schedules a timer that invokes `callback` after `duration`, and repeats if `repeating` is true.
    fn schedule_timer(&self, callback: Callback, duration: Duration, repeating: bool) -> Timer;

    /// Localizes `key` with `parameters`.
    #[must_use]
    fn localize<'a>(&self, key: &str, parameters: Option<LocalizationParameters<'a>>) -> String;

    /// Opens a window. Returns false if unable to open the window.
    fn open(&self, window: Box<dyn AnyWindowBuilder>) -> bool;

    /// Internal API used by `ManagedCodeGuard`. Do not call directly.
    #[doc(hidden)]
    fn _exit_managed_code(&self, allow_process_messages: bool);
}

/// A guard marking that Gooey-managed code is executing.
pub struct ManagedCodeGuard {
    pub(crate) frontend: Box<dyn AnyFrontend>,
    pub(crate) allow_process_messages: bool,
}

impl Drop for ManagedCodeGuard {
    fn drop(&mut self) {
        self.frontend
            ._exit_managed_code(self.allow_process_messages);
    }
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

    fn set_widget_has_messages(&self, widget: WidgetId) {
        self.set_widget_has_messages(widget);
    }

    fn enter_managed_code(&self) -> ManagedCodeGuard {
        self.gooey().enter_managed_code(self)
    }

    fn _exit_managed_code(&self, allow_process_messages: bool) {
        self.gooey().exit_managed_code(self, allow_process_messages);
        self.exit_managed_code();
    }

    fn theme(&self) -> SystemTheme {
        self.theme()
    }

    fn load_image(&self, asset: &Image, completed: Callback<Image>, error: Callback<String>) {
        self.load_image(asset, completed, error);
    }

    fn asset_url(&self, asset: &Asset) -> Option<Url> {
        self.asset_url(asset)
    }

    fn schedule_timer(&self, callback: Callback, duration: Duration, repeating: bool) -> Timer {
        self.schedule_timer(callback, duration, repeating)
    }

    fn localize<'a>(&self, key: &str, parameters: Option<LocalizationParameters<'a>>) -> String {
        self.localize(key, parameters)
    }

    fn window(&self) -> Option<&dyn Window> {
        self.window()
    }

    fn open(&self, window: Box<dyn AnyWindowBuilder>) -> bool {
        self.open(window)
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
        registration: &WidgetRegistration,
        frontend: &F,
    ) -> TransmogrifierState;

    /// Processes commands and events for this widget and transmogrifier.
    fn process_messages(&self, context: AnyTransmogrifierContext<'_, F>);
}

impl<F: Frontend, T> AnyTransmogrifier<F> for T
where
    T: Transmogrifier<F>,
{
    fn process_messages(&self, mut context: AnyTransmogrifierContext<'_, F>) {
        <Self as Transmogrifier<F>>::process_messages(
            self,
            TransmogrifierContext::try_from(&mut context).unwrap(),
        );
    }

    fn widget_type_id(&self) -> TypeId {
        <Self as Transmogrifier<F>>::widget_type_id(self)
    }

    fn default_state_for(
        &self,
        widget: &mut dyn AnyWidget,
        registration: &WidgetRegistration,
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
