use gooey_core::{Frontend, StyledWidget, Transmogrifiers, Widget, WidgetStorage};
use gooey_widgets::{
    component::{Behavior, ComponentTransmogrifier},
    navigator::{DefaultBarBehavior, Location, NavigatorBehavior},
};

/// A cross-platform application.
#[must_use]
pub struct App {
    initializer: Box<dyn FnOnce(Transmogrifiers<crate::ActiveFrontend>) -> crate::ActiveFrontend>,
    transmogrifiers: Transmogrifiers<crate::ActiveFrontend>,
}

impl App {
    /// Returns a new application using `initializer` to create a root widget and any custom `transmogrifiers`.
    pub fn new<W: Widget, I: FnOnce(&WidgetStorage) -> StyledWidget<W> + 'static>(
        initializer: I,
        transmogrifiers: Transmogrifiers<crate::ActiveFrontend>,
    ) -> Self {
        Self {
            initializer: Box::new(move |transmogrifiers| crate::app(transmogrifiers, initializer)),
            transmogrifiers,
        }
    }

    /// Returns a new application using `initializer` to create a root widget with no transmogrifiers.
    pub fn from_root<W: Widget, I: FnOnce(&WidgetStorage) -> StyledWidget<W> + 'static>(
        initializer: I,
    ) -> Self {
        Self::new(initializer, Transmogrifiers::default())
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

    /// Runs this application using the root widget provided by `initializer`.
    pub fn run(self) {
        let initializer = self.initializer;
        let frontend = initializer(self.transmogrifiers);
        crate::run(frontend);
    }

    /// Returns a headless renderer for this app. Only supported with feature
    /// `frontend-kludgine` currently.
    #[cfg(all(feature = "frontend-kludgine", not(target_arch = "wasm32")))]
    pub fn headless(self) -> crate::Headless<crate::ActiveFrontend> {
        let initializer = self.initializer;
        let frontend = initializer(self.transmogrifiers);
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
