use gooey_core::{Frontend, StyledWidget, Transmogrifiers, Widget, WidgetStorage};

/// A cross-platform application.
#[derive(Default, Debug)]
pub struct App {
    transmogrifiers: Transmogrifiers<crate::ActiveFrontend>,
}

impl App {
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

    /// Runs this application using the root widget provided by `initializer`.
    pub fn run<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> StyledWidget<W>>(
        self,
        initializer: C,
    ) {
        crate::main_with(self.transmogrifiers, initializer)
    }
}
