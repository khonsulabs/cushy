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

    /// Runs this application using the root widget provided by `initializer`.
    pub fn run<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> StyledWidget<W>>(
        self,
        initializer: C,
    ) {
        crate::main_with(self.transmogrifiers, initializer);
    }
}
