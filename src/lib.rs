use core::window::{NewWindow, Window, WindowBuilder};
use core::{Frontend, Widgets};
use std::sync::Arc;

use gooey_core::Context;
#[cfg(feature = "raster")]
pub use gooey_raster as raster;
#[cfg(feature = "web")]
pub use gooey_web as web;
pub use {gooey_core as core, gooey_widgets as widgets};

pub struct App<A>
where
    A: Frontend,
{
    widgets: Arc<gooey_core::Widgets<A>>,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
impl Default for App<gooey_web::WebApp> {
    fn default() -> Self {
        Self::new(gooey_widgets::widgets())
    }
}

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
impl Default for App<gooey_raster::RasterizedApp<gooey_kludgine::Kludgine>> {
    fn default() -> Self {
        Self::new(gooey_widgets::widgets())
    }
}

impl<A> App<A>
where
    A: Frontend,
{
    pub fn new(widgets: Widgets<A>) -> Self {
        Self {
            widgets: Arc::new(widgets),
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
impl App<gooey_web::WebApp> {
    pub fn run_with<Widget, Initializer>(self, init: Initializer) -> !
    where
        Initializer: FnOnce(WindowBuilder) -> NewWindow<Widget>,
        Widget: gooey_core::Widget,
    {
        gooey_web::attach_to_body(self.widgets, init(WindowBuilder::default()));

        wasm_bindgen::throw_str("This is not an actual error. Please ignore.");
    }

    pub fn run<Widget, Initializer>(self, init: Initializer) -> !
    where
        Initializer: FnOnce(&Context, &Window) -> Widget + std::panic::UnwindSafe + Send + 'static,
        Widget: gooey_core::Widget,
    {
        self.run_with(|builder| builder.create(init))
    }
}

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
impl App<gooey_raster::RasterizedApp<gooey_kludgine::Kludgine>> {
    pub fn run_with<Widget, Initializer>(self, init: Initializer) -> !
    where
        Initializer: FnOnce(WindowBuilder) -> NewWindow<Widget>,
        Widget: gooey_core::Widget,
    {
        gooey_kludgine::run(self.widgets, init(WindowBuilder::default()))
    }

    pub fn run<Widget, Initializer>(self, init: Initializer) -> !
    where
        Initializer: FnOnce(&Context, &Window) -> Widget + std::panic::UnwindSafe + Send + 'static,
        Widget: gooey_core::Widget,
    {
        self.run_with(|builder| builder.create(init))
    }
}
