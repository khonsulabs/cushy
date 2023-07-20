mod button;
pub mod flex;
mod label;

pub use button::Button;
pub use flex::Flex;
use gooey_core::style::Color;
pub use label::{Label, LabelExt};

use crate::button::ButtonTransmogrifier;
use crate::flex::FlexTransmogrifier;
use crate::label::LabelTransmogrifier;

#[cfg(feature = "web")]
pub fn web_widgets() -> gooey_core::Widgets<gooey_web::WebApp> {
    let _ = console_log::init();
    gooey_core::Widgets::default()
        .with::<ButtonTransmogrifier>()
        .with::<FlexTransmogrifier>()
        .with::<LabelTransmogrifier>()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn widgets() -> gooey_core::Widgets<gooey_web::WebApp> {
    web_widgets()
}

#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
pub fn raster_widgets<Surface>() -> gooey_core::Widgets<gooey_raster::RasterizedApp<Surface>>
where
    Surface: gooey_raster::Surface,
{
    gooey_core::Widgets::default()
        .with::<ButtonTransmogrifier>()
        .with::<FlexTransmogrifier>()
        .with::<LabelTransmogrifier>()
}

#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
pub fn widgets<Surface>() -> gooey_core::Widgets<gooey_raster::RasterizedApp<Surface>>
where
    Surface: gooey_raster::Surface,
{
    raster_widgets()
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
enum State {
    Normal,
    Hover,
    Active,
}

fn control_text_color(state: State) -> Color {
    match state {
        State::Normal => Color::rgba(0, 0, 0, 255),
        State::Hover => Color::rgba(20, 20, 20, 255),
        State::Active => Color::rgba(0, 0, 0, 255),
    }
}
