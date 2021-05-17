pub mod button;
pub mod container;

#[cfg(feature = "frontend-browser")]
fn window_document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

#[cfg(feature = "frontend-rasterizer")]
pub mod rasterized {
    use gooey_core::Gooey;
    use gooey_rasterizer::make_rasterized;

    use crate::{button::ButtonTransmogrifier, container::ContainerTransmogrifier};

    pub fn register_transmogrifiers<R: gooey_core::renderer::Renderer>(
        mut ui: Gooey<gooey_rasterizer::Rasterizer<R>>,
    ) -> Gooey<gooey_rasterizer::Rasterizer<R>> {
        drop(ui.register_transmogrifier(ButtonTransmogrifier));
        drop(ui.register_transmogrifier(ContainerTransmogrifier));

        ui
    }

    make_rasterized!(ButtonTransmogrifier);
    make_rasterized!(ContainerTransmogrifier);
}

#[cfg(feature = "frontend-browser")]
pub mod browser {
    use gooey_browser::{make_browser, WebSys};
    use gooey_core::Gooey;

    use crate::{button::ButtonTransmogrifier, container::ContainerTransmogrifier};

    pub fn register_transmogrifiers(mut ui: Gooey<WebSys>) -> Gooey<WebSys> {
        drop(ui.register_transmogrifier(ButtonTransmogrifier));
        drop(ui.register_transmogrifier(ContainerTransmogrifier));

        ui
    }

    make_browser!(ButtonTransmogrifier);
    make_browser!(ContainerTransmogrifier);
}
