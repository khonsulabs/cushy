pub mod button;
pub mod component;
pub mod container;

pub const CONTROL_CLASS: &str = "gooey-control";

#[cfg(feature = "frontend-rasterizer")]
pub mod rasterized {
    use gooey_core::{renderer::Renderer, Transmogrifiers};
    use gooey_rasterizer::{make_rasterized, Rasterizer};

    use crate::{button::ButtonTransmogrifier, container::ContainerTransmogrifier};

    pub fn register_transmogrifiers<R: Renderer>(
        transmogrifiers: &mut Transmogrifiers<Rasterizer<R>>,
    ) {
        drop(transmogrifiers.register_transmogrifier(ButtonTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(ContainerTransmogrifier));
    }

    pub fn default_transmogrifiers<R: Renderer>() -> Transmogrifiers<Rasterizer<R>> {
        let mut transmogrifiers = Transmogrifiers::default();
        register_transmogrifiers(&mut transmogrifiers);
        transmogrifiers
    }

    make_rasterized!(ButtonTransmogrifier);
    make_rasterized!(ContainerTransmogrifier);
}

#[cfg(feature = "frontend-browser")]
pub mod browser {
    use gooey_browser::{make_browser, WebSys};
    use gooey_core::Transmogrifiers;

    use crate::{button::ButtonTransmogrifier, container::ContainerTransmogrifier};

    pub fn register_transmogrifiers(transmogrifiers: &mut Transmogrifiers<WebSys>) {
        drop(transmogrifiers.register_transmogrifier(ButtonTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(ContainerTransmogrifier));
    }

    pub fn default_transmogrifiers() -> Transmogrifiers<WebSys> {
        let mut transmogrifiers = Transmogrifiers::default();
        register_transmogrifiers(&mut transmogrifiers);
        transmogrifiers
    }

    make_browser!(ButtonTransmogrifier);
    make_browser!(ContainerTransmogrifier);
}
