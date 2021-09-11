use std::{str::FromStr, sync::Arc};

use gooey_core::{
    styles::style_sheet::StyleSheet, unic_langid::LanguageIdentifier, AnyWindowBuilder, AppContext,
    Frontend, Localizer, StyledWidget, Transmogrifiers, Widget, WidgetStorage, WindowBuilder,
};
use gooey_widgets::{
    component::{Behavior, ComponentTransmogrifier},
    form::{Form, Model},
    navigator::{DefaultBarBehavior, Location, NavigatorBehavior},
};
use sys_locale::get_locale;

use crate::style::default_stylesheet;

type InitializerFn = dyn FnOnce(
    Transmogrifiers<crate::ActiveFrontend>,
    AppContext,
    &mut dyn AnyWindowBuilder,
) -> crate::ActiveFrontend;

/// A cross-platform application.
#[must_use]
pub struct App {
    initial_window: Box<dyn AnyWindowBuilder>,
    initializer: Box<InitializerFn>,
    stylesheet: Option<StyleSheet>,
    language: LanguageIdentifier,
    localizer: Arc<dyn Localizer>,
    transmogrifiers: Transmogrifiers<crate::ActiveFrontend>,
}

impl App {
    /// Returns a new application using `initializer` to create a root widget and any custom `transmogrifiers`.
    // Panic is only going to happen if unic-langid can't parse "en-US".
    #[allow(clippy::missing_panics_doc)]
    pub fn new<W: Widget>(
        initial_window: WindowBuilder<W>,
        transmogrifiers: Transmogrifiers<crate::ActiveFrontend>,
    ) -> Self {
        let language = locale_from_environment()
            .or_else(|| {
                get_locale().and_then(|identifier| match identifier.parse() {
                    Ok(language) => Some(language),
                    Err(err) => {
                        log::error!(
                            "Detected locale {}, but encountered error parsing: {:?}. Defaulting \
                             to en-US.",
                            identifier,
                            err
                        );
                        None
                    }
                })
            })
            .unwrap_or_else(|| "en-US".parse().unwrap());
        Self {
            initializer: Box::new(move |transmogrifiers, context, window| {
                let builder = window
                    .as_mut_any()
                    .downcast_mut::<WindowBuilder<W>>()
                    .unwrap();
                crate::app(transmogrifiers, builder, context)
            }),
            initial_window: Box::new(initial_window),
            localizer: Arc::new(()),
            language,
            transmogrifiers,
            stylesheet: None,
        }
    }

    /// Returns a new application using `initializer` to create a root widget with no transmogrifiers.
    pub fn from_root<W: Widget, I: FnOnce(&WidgetStorage) -> StyledWidget<W> + 'static>(
        initializer: I,
    ) -> Self {
        Self::new(WindowBuilder::new(initializer), Transmogrifiers::default())
    }

    /// Registers a [`Transmogrifier`](gooey_core::Transmogrifier). This will
    /// allow `T::Widget` to be used in this application.
    pub fn with<T: Into<<crate::ActiveFrontend as Frontend>::AnyTransmogrifier>>(
        mut self,
        transmogrifier: T,
    ) -> Self {
        self.transmogrifiers
            .register_transmogrifier(transmogrifier)
            .expect("a transmogrifier is already registered for this widget");
        self
    }

    /// Registers the transmogrifier for a component with behavior `B`.
    pub fn with_component<B: Behavior>(mut self) -> Self {
        self.transmogrifiers
            .register_transmogrifier(ComponentTransmogrifier::<B>::default())
            .expect("a transmogrifier is already registered for this widget");
        self
    }

    /// Registers the transmogrifier for a navigator with behavior `B`.
    pub fn with_navigator<Loc: Location>(mut self) -> Self {
        self.transmogrifiers
            .register_transmogrifier(ComponentTransmogrifier::<NavigatorBehavior<Loc>>::default())
            .expect("a transmogrifier is already registered for this widget");
        self.transmogrifiers
            .register_transmogrifier(ComponentTransmogrifier::<DefaultBarBehavior<Loc>>::default())
            .expect("a transmogrifier is already registered for this widget");
        self
    }

    /// Sets the stylesheet for this application. If not specified,
    /// [`default_stylesheet()`] is used.
    pub fn stylesheet(mut self, stylesheet: StyleSheet) -> Self {
        self.stylesheet = Some(stylesheet);
        self
    }

    /// Enables localization through the provided localizer.
    pub fn localizer<L: Localizer>(mut self, localizer: L) -> Self {
        self.localizer = Arc::new(localizer);
        self
    }

    /// Sets the initial language for localization. By default, the language is
    /// auto-detected using a combination of
    /// [`sys-locale`](https://crates.io/crates/sys-locale) and environment
    /// variables, so this should only be used in situations where you want to
    /// ensure the user is presented a specific language during your
    /// application's startup.
    pub fn initial_language(mut self, language: LanguageIdentifier) -> Self {
        self.language = language;
        self
    }

    /// Registers the transmogrifier for a form with model `M`.
    pub fn with_form<M: Model>(mut self) -> Self {
        self.transmogrifiers
            .register_transmogrifier(ComponentTransmogrifier::<Form<M>>::default())
            .expect("a transmogrifier is already registered for this widget");
        self
    }

    /// Runs this application using the root widget provided by `initializer`.
    pub fn run(self) {
        let initializer = self.initializer;
        let mut initial_window = self.initial_window;
        let frontend = initializer(
            self.transmogrifiers,
            AppContext::new(
                self.stylesheet.unwrap_or_else(default_stylesheet),
                self.language,
                self.localizer,
            ),
            initial_window.as_mut(),
        );
        crate::run(frontend, initial_window.configuration());
    }

    /// Returns a headless renderer for this app. Only supported with feature
    /// `frontend-kludgine` currently.
    #[cfg(all(feature = "frontend-kludgine", not(target_arch = "wasm32")))]
    pub fn headless(self) -> crate::Headless<crate::ActiveFrontend> {
        let initializer = self.initializer;
        let mut initial_window = self.initial_window;
        let frontend = initializer(
            self.transmogrifiers,
            AppContext::new(
                self.stylesheet.unwrap_or_else(default_stylesheet),
                self.language,
                self.localizer,
            ),
            initial_window.as_mut(),
        );
        crate::Headless::new(frontend)
    }

    /// Spawns an asynchronous task using the runtime that the `App` uses.
    /// Requires feature `async` to be enabled.
    #[cfg(feature = "async")]
    pub fn spawn<F: std::future::Future<Output = ()> + Send + 'static>(future: F) {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                gooey_browser::WebSys::initialize();
                let _promise = wasm_bindgen_futures::future_to_promise(async move { future.await; Ok(wasm_bindgen::JsValue::UNDEFINED) });
            } else if #[cfg(feature = "frontend-kludgine")] {
                gooey_kludgine::kludgine::prelude::Runtime::initialize();
                gooey_kludgine::kludgine::prelude::Runtime::spawn(future);
            } else {
                compile_error!("unsupported async configuration")
            }
        }
    }

    /// Sleeps asynchronously for `duration`.
    #[cfg(feature = "async")]
    pub async fn sleep_for<D: Into<std::time::Duration> + Send>(duration: D) {
        let duration = duration.into();
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let (sender, receiver) = flume::bounded(1);
                {
                    use wasm_bindgen::{JsCast, closure::Closure};
                    let window = web_sys::window().unwrap();
                    window.set_timeout_with_callback_and_timeout_and_arguments_0(Closure::once_into_js(move || {
                        let _ = sender.send(());
                    }).as_ref().unchecked_ref(), duration.as_millis() as i32).unwrap();
                }
                let _ = receiver.recv_async().await;
            } else if #[cfg(feature = "frontend-kludgine")] {
                tokio::time::sleep(duration).await;
            } else {
                compile_error!("unsupported async configuration")
            }
        }
    }
}

impl<W: Widget> From<WindowBuilder<W>> for App {
    fn from(window: WindowBuilder<W>) -> Self {
        Self::new(window, Transmogrifiers::default())
    }
}

fn locale_from_environment() -> Option<LanguageIdentifier> {
    std::env::var("LANG")
        .ok()
        .and_then(|lang| LanguageIdentifier::from_str(&lang).ok())
}
