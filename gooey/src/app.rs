use std::future::Future;

use cfg_if::cfg_if;
use gooey_core::{Frontend, StyledWidget, Transmogrifiers, Widget, WidgetStorage};

#[derive(Default, Debug)]
pub struct App {
    transmogrifiers: Transmogrifiers<crate::ActiveFrontend>,
}

impl App {
    pub fn with<T: Into<<crate::ActiveFrontend as Frontend>::AnyTransmogrifier>>(
        mut self,
        transmogrifier: T,
    ) -> Self {
        self.transmogrifiers
            .register_transmogrifier(transmogrifier)
            .expect("a transmogrifier is already registered for this widget");
        self
    }

    #[cfg(feature = "async")]
    pub fn spawn<F: Future<Output = ()> + Send + 'static>(future: F) {
        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                todo!("wasm bindgen futures")
            } else if #[cfg(feature = "frontend-kludgine")] {
                gooey_kludgine::kludgine::prelude::Runtime::initialize();
                gooey_kludgine::kludgine::prelude::Runtime::spawn(future);
            } else {
                compile_error!("unsupported async configuration")
            }
        }
    }

    #[cfg(feature = "async")]
    pub fn block_on<F: Future<Output = R> + Send + 'static, R: Send + Sync>(future: F) -> R {
        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                todo!("wasm bindgen futures")
            } else if #[cfg(feature = "frontend-kludgine")] {
                gooey_kludgine::kludgine::prelude::Runtime::initialize();
                gooey_kludgine::kludgine::prelude::Runtime::block_on(future)
            } else {
                compile_error!("unsupported async configuration")
            }
        }
    }

    pub fn run<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> StyledWidget<W>>(
        self,
        initializer: C,
    ) {
        crate::main_with(self.transmogrifiers, initializer)
    }
}
