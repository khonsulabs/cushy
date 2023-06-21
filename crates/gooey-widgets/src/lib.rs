mod button;
mod flex;
mod label;

pub use button::Button;
pub use flex::Flex;
use gooey_web::WebApp;
pub use label::{Label, LabelExt};

use crate::button::ButtonTransmogrifier;
use crate::flex::FlexTransmogrifier;
use crate::label::LabelTransmogrifier;

#[cfg(feature = "web")]
pub fn web_widgets() -> gooey_core::Widgets<WebApp> {
    let _ = console_log::init();
    gooey_core::Widgets::default()
        .with::<ButtonTransmogrifier>()
        .with::<FlexTransmogrifier>()
        .with::<LabelTransmogrifier>()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn widgets() -> gooey_core::Widgets<WebApp> {
    web_widgets()
}

#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
pub fn raster_widgets() -> gooey_core::Widgets<gooey_raster::Rasterizer> {
    todo!("need rasterizer")
}

#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
pub fn widgets() -> gooey_core::Widgets<gooey_raster::Rasterizer> {
    raster_widgets()
}
